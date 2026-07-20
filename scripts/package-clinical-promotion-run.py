#!/usr/bin/env python3
"""Package verified clinical-promotion reports from a canonical runner.

Only structured diagnostic reports are retained. Raw logs, command output, environment
values, credentials, and promotion inputs are intentionally excluded.
"""
from __future__ import annotations

import argparse
import gzip
import importlib.util
import json
import os
import pathlib
import re
import shutil
import stat
import tarfile
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]
RUNNER_POLICY = ROOT / "release/clinical-promotion-runner-evidence-policy.json"
SPEC = importlib.util.spec_from_file_location(
    "health_promotion_diagnostics", ROOT / "scripts/clinical-promotion-diagnostics.py"
)
assert SPEC and SPEC.loader
D = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(D)
REPOSITORY = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
SAFE_TEXT = re.compile(r"^[A-Za-z0-9_.:/@+-]+$")


class RunnerPackageError(ValueError):
    pass


def fail(message: str) -> NoReturn:
    raise RunnerPackageError(message)


def load_policy(path: pathlib.Path = RUNNER_POLICY) -> dict[str, Any]:
    policy = D.load_json(path)
    if policy.get("schema_version") != 1 or policy.get("release_id") != "health-v1":
        fail("runner evidence policy identity drifted")
    for field in ("allowed_source_refs", "allowed_workflows", "allowed_events", "allowed_report_kinds"):
        value = policy.get(field)
        if not isinstance(value, list) or not value or len(value) != len(set(value)):
            fail(f"runner evidence policy {field} must be a unique non-empty list")
    limits = policy.get("package_limits")
    if not isinstance(limits, dict) or any(not isinstance(value, int) or value <= 0 for value in limits.values()):
        fail("runner evidence package limits are invalid")
    claims = policy.get("required_claims")
    if not isinstance(claims, dict) or not claims or any(value is not True for value in claims.values()):
        fail("runner evidence required claims must all be true")
    policy["_policy_path"] = str(path.resolve())
    return policy


def validate_text(value: Any, label: str, *, maximum: int = 256) -> str:
    if not isinstance(value, str) or not value or len(value) > maximum or SAFE_TEXT.fullmatch(value) is None:
        fail(f"runner context {label} is invalid")
    return value


def build_context(args: argparse.Namespace, policy: dict[str, Any]) -> dict[str, Any]:
    repository = validate_text(args.repository, "repository")
    if REPOSITORY.fullmatch(repository) is None:
        fail("runner context repository must be owner/name")
    if D.HEX40.fullmatch(args.source_revision or "") is None:
        fail("runner context source_revision must be a lowercase 40-character Git commit")
    if args.source_ref not in policy["allowed_source_refs"]:
        fail("runner context source_ref is not allowed")
    if D.HEX40.fullmatch(args.source_ref_head or "") is None:
        fail("runner context source_ref_head must be a lowercase 40-character Git commit")
    if args.source_ref_ancestry_verified is not True:
        fail("runner context must affirm verified source-ref ancestry")
    if args.workflow not in policy["allowed_workflows"]:
        fail("runner context workflow is not allowed")
    if args.event_name not in policy["allowed_events"]:
        fail("runner context event_name is not allowed")
    if args.run_id <= 0 or args.run_attempt <= 0:
        fail("runner context run identifiers must be positive")
    context = {
        "schema_version": 1,
        "repository": repository,
        "source_revision": args.source_revision,
        "source_ref": args.source_ref,
        "source_ref_head": args.source_ref_head,
        "source_ref_ancestry_verified": True,
        "workflow": args.workflow,
        "event_name": args.event_name,
        "run_id": args.run_id,
        "run_attempt": args.run_attempt,
        "runner_os": validate_text(args.runner_os, "runner_os", maximum=64),
        "runner_arch": validate_text(args.runner_arch, "runner_arch", maximum=64),
    }
    if set(policy["required_context_fields"]) - set(context):
        fail("runner context is missing policy-required fields")
    identity = {key: context[key] for key in sorted(context) if key != "schema_version"}
    context["context_digest_sha256"] = D.sha256_bytes(D.canonical_bytes(identity))
    return context


def safe_report_file(path: pathlib.Path, maximum_bytes: int) -> None:
    if not path.exists() or path.is_symlink() or not path.is_file():
        fail(f"runner report must be a regular non-symlink file: {path.name}")
    if stat.S_IMODE(path.lstat().st_mode) & 0o022:
        fail(f"runner report is group/world writable: {path.name}")
    if path.stat().st_size > maximum_bytes:
        fail(f"runner report exceeds package limit: {path.name}")


def write_json(path: pathlib.Path, value: dict[str, Any]) -> None:
    with path.open("x", encoding="utf-8") as handle:
        json.dump(value, handle, indent=2, sort_keys=True)
        handle.write("\n")
    os.chmod(path, 0o600)


def package(args: argparse.Namespace) -> dict[str, Any]:
    policy = load_policy(args.runner_policy.resolve())
    diagnostics_policy = D.load_policy(args.diagnostics_policy.resolve())
    context = build_context(args, policy)
    limits = policy["package_limits"]
    if not args.report:
        fail("at least one diagnostic report is required")
    if len(args.report) > limits["maximum_report_count"]:
        fail("runner report count exceeds package limit")
    reports: list[tuple[pathlib.Path, dict[str, Any]]] = []
    seen_ids: set[str] = set()
    seen_names: set[str] = set()
    total_bytes = 0
    for raw_path in args.report:
        path = raw_path.resolve()
        safe_report_file(path, limits["maximum_file_bytes"])
        report = D.load_json(path)
        D.verify_report(report, diagnostics_policy)
        if report.get("report_kind") not in policy["allowed_report_kinds"]:
            fail(f"runner report kind is not allowed: {report.get('report_kind')}")
        if report.get("release_id") != policy["release_id"]:
            fail(f"runner report release differs: {path.name}")
        report_revision = report.get("source_revision")
        if report_revision is not None and report_revision != context["source_revision"]:
            fail(f"runner report source revision differs: {path.name}")
        report_id = report.get("report_id")
        if not isinstance(report_id, str) or report_id in seen_ids:
            fail("runner reports contain a missing or duplicate report_id")
        if path.name in seen_names:
            fail("runner reports contain duplicate file names")
        seen_ids.add(report_id)
        seen_names.add(path.name)
        total_bytes += path.stat().st_size
        reports.append((path, report))
    if len(reports) + 2 > limits["maximum_file_count"] or total_bytes > limits["maximum_total_bytes"]:
        fail("runner evidence package exceeds bounded file or byte limits")
    output = args.output_dir.resolve()
    if output.exists():
        fail("refusing to overwrite runner evidence package")
    output.mkdir(parents=True, mode=0o700)
    os.chmod(output, 0o700)
    report_entries: list[dict[str, Any]] = []
    try:
        context_path = output / "RUNNER-CONTEXT.json"
        write_json(context_path, context)
        for source, report in reports:
            destination = output / source.name
            write_json(destination, report)
            report_entries.append(
                {
                    "name": destination.name,
                    "sha256": D.sha256_file(destination),
                    "size_bytes": destination.stat().st_size,
                    "report_id": report["report_id"],
                    "report_kind": report["report_kind"],
                    "status": report["status"],
                    "source_revision": report.get("source_revision"),
                    "report_digest_sha256": report["report_digest_sha256"],
                }
            )
        manifest_identity = {
            "runner_policy_sha256": D.sha256_file(args.runner_policy.resolve()),
            "diagnostics_policy_sha256": D.sha256_file(args.diagnostics_policy.resolve()),
            "context": context,
            "reports": sorted(report_entries, key=lambda item: item["name"]),
            "claims": policy["required_claims"],
        }
        package_digest = D.sha256_bytes(D.canonical_bytes(manifest_identity))
        manifest = {
            "schema_version": 1,
            "package_id": f"health-promotion-run-{package_digest[:24]}",
            "status": "verified",
            "release_id": policy["release_id"],
            "source_revision": context["source_revision"],
            "repository": context["repository"],
            "run_id": context["run_id"],
            "run_attempt": context["run_attempt"],
            "runner_policy_sha256": manifest_identity["runner_policy_sha256"],
            "diagnostics_policy_sha256": manifest_identity["diagnostics_policy_sha256"],
            "context": context,
            "reports": manifest_identity["reports"],
            "claims": policy["required_claims"],
            "package_digest_sha256": package_digest,
        }
        write_json(output / "RUNNER-MANIFEST.json", manifest)
    except Exception:
        shutil.rmtree(output, ignore_errors=True)
        raise
    return manifest




def create_archive(package_root: pathlib.Path, archive_path: pathlib.Path, policy: dict[str, Any]) -> dict[str, Any]:
    package_root = package_root.resolve()
    archive_path = archive_path.resolve()
    if archive_path.exists():
        fail("refusing to overwrite runner evidence archive")
    if archive_path == package_root or package_root in archive_path.parents:
        fail("runner evidence archive must be outside the package directory")
    files = sorted(path for path in package_root.iterdir())
    limits = policy["package_limits"]
    if not files or len(files) > limits["maximum_file_count"]:
        fail("runner evidence archive file count is invalid")
    if any(path.is_symlink() or not path.is_file() for path in files):
        fail("runner evidence archive accepts only regular files")
    total = sum(path.stat().st_size for path in files)
    if total > limits["maximum_total_bytes"]:
        fail("runner evidence archive exceeds total byte limit")
    archive_path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    try:
        with archive_path.open("xb") as raw:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0, compresslevel=9) as compressed:
                with tarfile.open(fileobj=compressed, mode="w", format=tarfile.USTAR_FORMAT) as archive:
                    for path in files:
                        info = tarfile.TarInfo(path.name)
                        info.size = path.stat().st_size
                        info.mode = 0o600
                        info.uid = 0
                        info.gid = 0
                        info.uname = ""
                        info.gname = ""
                        info.mtime = 0
                        with path.open("rb") as handle:
                            archive.addfile(info, handle)
        os.chmod(archive_path, 0o600)
    except Exception:
        archive_path.unlink(missing_ok=True)
        raise
    return {
        "name": archive_path.name,
        "sha256": D.sha256_file(archive_path),
        "size_bytes": archive_path.stat().st_size,
        "file_count": len(files),
        "uncompressed_bytes": total,
    }

def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runner-policy", type=pathlib.Path, default=RUNNER_POLICY)
    parser.add_argument("--diagnostics-policy", type=pathlib.Path, default=D.DEFAULT_POLICY)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--source-ref", required=True)
    parser.add_argument("--source-ref-head", required=True)
    parser.add_argument("--source-ref-ancestry-verified", action="store_true")
    parser.add_argument("--workflow", required=True)
    parser.add_argument("--event-name", required=True)
    parser.add_argument("--run-id", type=int, required=True)
    parser.add_argument("--run-attempt", type=int, required=True)
    parser.add_argument("--runner-os", required=True)
    parser.add_argument("--runner-arch", required=True)
    parser.add_argument("--report", type=pathlib.Path, action="append", default=[])
    parser.add_argument("--output-dir", type=pathlib.Path, required=True)
    parser.add_argument("--archive", type=pathlib.Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    manifest = package(args)
    if args.archive is not None:
        archive = create_archive(args.output_dir.resolve(), args.archive.resolve(), load_policy(args.runner_policy.resolve()))
        print(args.archive.resolve())
        print(archive["sha256"])
    else:
        print(manifest["package_id"])
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RunnerPackageError, D.DiagnosticError, OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        print(f"clinical promotion runner packaging error: {error}")
        raise SystemExit(1)
