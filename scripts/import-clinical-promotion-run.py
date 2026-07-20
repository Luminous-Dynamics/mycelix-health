#!/usr/bin/env python3
"""Verify and normalize a canonical-runner clinical-promotion evidence package."""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
import pathlib
import re
import stat
import tarfile
import tempfile
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]
RUNNER_POLICY = ROOT / "release/clinical-promotion-runner-evidence-policy.json"


def load_module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


D = load_module("health_promotion_diagnostics", ROOT / "scripts/clinical-promotion-diagnostics.py")
PKG = load_module("health_promotion_runner_package", ROOT / "scripts/package-clinical-promotion-run.py")
EXPLAIN = load_module("health_promotion_explanation", ROOT / "scripts/explain-clinical-promotion.py")
HEX64 = re.compile(r"^[0-9a-f]{64}$")


class RunnerImportError(ValueError):
    pass


def fail(message: str) -> NoReturn:
    raise RunnerImportError(message)


def safe_file(path: pathlib.Path, maximum_bytes: int) -> None:
    if not path.exists() or path.is_symlink() or not path.is_file():
        fail(f"runner package member is not a regular non-symlink file: {path.name}")
    mode = stat.S_IMODE(path.lstat().st_mode)
    if mode & 0o077:
        fail(f"runner package member is not owner-only: {path.name}")
    if path.stat().st_size > maximum_bytes:
        fail(f"runner package member exceeds byte limit: {path.name}")


def validate_context(context: dict[str, Any], policy: dict[str, Any]) -> None:
    if context.get("schema_version") != 1:
        fail("runner context schema_version must be 1")
    required = set(policy["required_context_fields"])
    if required - set(context):
        fail("runner context is incomplete")
    if D.HEX40.fullmatch(str(context.get("source_revision", ""))) is None:
        fail("runner context source revision is invalid")
    if context.get("source_ref") not in policy["allowed_source_refs"]:
        fail("runner context source ref is not allowed")
    if D.HEX40.fullmatch(str(context.get("source_ref_head", ""))) is None:
        fail("runner context source ref head is invalid")
    if context.get("source_ref_ancestry_verified") is not True:
        fail("runner context source-ref ancestry was not verified")
    if context.get("workflow") not in policy["allowed_workflows"]:
        fail("runner context workflow is not allowed")
    if context.get("event_name") not in policy["allowed_events"]:
        fail("runner context event is not allowed")
    if not isinstance(context.get("run_id"), int) or context["run_id"] <= 0:
        fail("runner context run_id is invalid")
    if not isinstance(context.get("run_attempt"), int) or context["run_attempt"] <= 0:
        fail("runner context run_attempt is invalid")
    identity = {key: context[key] for key in sorted(context) if key not in {"schema_version", "context_digest_sha256"}}
    expected = D.sha256_bytes(D.canonical_bytes(identity))
    if context.get("context_digest_sha256") != expected:
        fail("runner context digest mismatch")


def verify_package(
    root: pathlib.Path,
    *,
    expected_repository: str,
    expected_source_revision: str,
    runner_policy_path: pathlib.Path = RUNNER_POLICY,
    diagnostics_policy_path: pathlib.Path = D.DEFAULT_POLICY,
) -> tuple[dict[str, Any], list[tuple[pathlib.Path, dict[str, Any]]]]:
    if not root.exists() or root.is_symlink() or not root.is_dir():
        fail("runner evidence package must be a regular directory")
    if stat.S_IMODE(root.lstat().st_mode) & 0o077:
        fail("runner evidence package directory must be owner-only")
    policy = PKG.load_policy(runner_policy_path)
    diagnostics_policy = D.load_policy(diagnostics_policy_path)
    limits = policy["package_limits"]
    members = sorted(path for path in root.iterdir())
    if len(members) > limits["maximum_file_count"]:
        fail("runner evidence package has too many files")
    if any(path.is_dir() for path in members):
        fail("runner evidence package must be flat")
    for path in members:
        safe_file(path, limits["maximum_file_bytes"])
    total_bytes = sum(path.stat().st_size for path in members)
    if total_bytes > limits["maximum_total_bytes"]:
        fail("runner evidence package exceeds total byte limit")
    manifest_path = root / "RUNNER-MANIFEST.json"
    context_path = root / "RUNNER-CONTEXT.json"
    safe_file(manifest_path, limits["maximum_file_bytes"])
    safe_file(context_path, limits["maximum_file_bytes"])
    manifest = D.load_json(manifest_path)
    context = D.load_json(context_path)
    validate_context(context, policy)
    if context.get("repository") != expected_repository:
        fail("runner package repository differs from expected repository")
    if context.get("source_revision") != expected_source_revision:
        fail("runner package source revision differs from expected revision")
    if manifest.get("schema_version") != 1 or manifest.get("status") != "verified":
        fail("runner package manifest is not verified schema v1")
    if manifest.get("release_id") != policy["release_id"]:
        fail("runner package release_id drifted")
    if manifest.get("source_revision") != expected_source_revision or manifest.get("repository") != expected_repository:
        fail("runner package manifest identity differs from context")
    if manifest.get("context") != context:
        fail("runner package manifest embeds a different context")
    if manifest.get("runner_policy_sha256") != D.sha256_file(runner_policy_path):
        fail("runner package policy digest drifted")
    if manifest.get("diagnostics_policy_sha256") != D.sha256_file(diagnostics_policy_path):
        fail("runner package diagnostics policy digest drifted")
    if manifest.get("claims") != policy["required_claims"]:
        fail("runner package claims drifted")
    report_entries = manifest.get("reports")
    if not isinstance(report_entries, list) or not report_entries:
        fail("runner package manifest has no reports")
    if len(report_entries) > limits["maximum_report_count"]:
        fail("runner package report count exceeds policy")
    reports: list[tuple[pathlib.Path, dict[str, Any]]] = []
    expected_names = {"RUNNER-CONTEXT.json", "RUNNER-MANIFEST.json"}
    seen_ids: set[str] = set()
    canonical_entries: list[dict[str, Any]] = []
    for entry in report_entries:
        if not isinstance(entry, dict):
            fail("runner package report entry is invalid")
        name = entry.get("name")
        if not isinstance(name, str) or pathlib.Path(name).name != name or name in expected_names:
            fail("runner package report name is unsafe or duplicated")
        expected_names.add(name)
        path = root / name
        safe_file(path, limits["maximum_file_bytes"])
        if entry.get("sha256") != D.sha256_file(path) or entry.get("size_bytes") != path.stat().st_size:
            fail(f"runner package report bytes differ from manifest: {name}")
        report = D.load_json(path)
        D.verify_report(report, diagnostics_policy)
        if report.get("report_id") != entry.get("report_id") or report.get("report_digest_sha256") != entry.get("report_digest_sha256"):
            fail(f"runner package report identity differs from manifest: {name}")
        if report.get("report_kind") != entry.get("report_kind") or report.get("status") != entry.get("status"):
            fail(f"runner package report summary differs from manifest: {name}")
        if report.get("source_revision") not in {None, expected_source_revision}:
            fail(f"runner package report revision differs: {name}")
        if report["report_id"] in seen_ids:
            fail("runner package report IDs are duplicated")
        seen_ids.add(report["report_id"])
        canonical_entries.append(entry)
        reports.append((path, report))
    actual_names = {path.name for path in members}
    if actual_names != expected_names:
        fail(f"runner package contains unmanifested files: {sorted(actual_names - expected_names)}")
    identity = {
        "runner_policy_sha256": manifest["runner_policy_sha256"],
        "diagnostics_policy_sha256": manifest["diagnostics_policy_sha256"],
        "context": context,
        "reports": sorted(canonical_entries, key=lambda item: item["name"]),
        "claims": manifest["claims"],
    }
    digest = D.sha256_bytes(D.canonical_bytes(identity))
    if manifest.get("package_digest_sha256") != digest:
        fail("runner package digest mismatch")
    if manifest.get("package_id") != f"health-promotion-run-{digest[:24]}":
        fail("runner package id mismatch")
    return manifest, reports



def verify_import_report(value: dict[str, Any]) -> dict[str, Any]:
    if value.get("schema_version") != 1 or value.get("report_kind") != "canonical-runner-import":
        fail("canonical runner import schema or kind is invalid")
    if value.get("status") != "verified" or value.get("release_id") != "health-v1":
        fail("canonical runner import identity drifted")
    if not isinstance(value.get("repository"), str) or not isinstance(value.get("source_revision"), str):
        fail("canonical runner import lacks repository or source revision")
    if D.HEX40.fullmatch(value["source_revision"]) is None:
        fail("canonical runner import source revision is invalid")
    if not isinstance(value.get("run_id"), int) or value["run_id"] <= 0:
        fail("canonical runner import run_id is invalid")
    if not isinstance(value.get("run_attempt"), int) or value["run_attempt"] <= 0:
        fail("canonical runner import run_attempt is invalid")
    for field in ("runner_policy_sha256", "diagnostics_policy_sha256", "report_digest_sha256"):
        if not isinstance(value.get(field), str) or HEX64.fullmatch(value[field]) is None:
            fail(f"canonical runner import {field} is invalid")
    package = value.get("source_package")
    reports = value.get("reports")
    explanation = value.get("explanation")
    if not isinstance(package, dict) or not isinstance(reports, list) or not reports or not isinstance(explanation, dict):
        fail("canonical runner import package, reports, or explanation is invalid")
    for field in ("package_digest_sha256", "manifest_sha256"):
        if not isinstance(package.get(field), str) or HEX64.fullmatch(package[field]) is None:
            fail(f"canonical runner import source package {field} is invalid")
    for report in reports:
        if not isinstance(report, dict):
            fail("canonical runner import report reference is invalid")
        for field in ("sha256", "report_digest_sha256"):
            if not isinstance(report.get(field), str) or HEX64.fullmatch(report[field]) is None:
                fail(f"canonical runner import report {field} is invalid")
    roots = explanation.get("root_causes")
    if not isinstance(roots, list):
        fail("canonical runner import root causes are invalid")
    diagnostics_policy = D.load_policy()
    for index, item in enumerate(roots, start=1):
        if not isinstance(item, dict) or item.get("priority") != index:
            fail("canonical runner import root-cause ordering is invalid")
        D.verify_reason(diagnostics_policy, item)
    identity = {
        "runner_policy_sha256": value["runner_policy_sha256"],
        "diagnostics_policy_sha256": value["diagnostics_policy_sha256"],
        "source_package": package,
        "repository": value["repository"],
        "source_revision": value["source_revision"],
        "source_ref": value.get("source_ref"),
        "source_ref_head": value.get("source_ref_head"),
        "source_ref_ancestry_verified": value.get("source_ref_ancestry_verified"),
        "workflow": value.get("workflow"),
        "event_name": value.get("event_name"),
        "run_id": value["run_id"],
        "run_attempt": value["run_attempt"],
        "runner_os": value.get("runner_os"),
        "runner_arch": value.get("runner_arch"),
        "reports": reports,
        "explanation_report_digest_sha256": explanation.get("report_digest_sha256"),
        "root_causes": roots,
    }
    digest = D.sha256_bytes(D.canonical_bytes(identity))
    if value.get("report_digest_sha256") != digest:
        fail("canonical runner import digest mismatch")
    if value.get("report_id") != f"health-promotion-run-import-{digest[:24]}":
        fail("canonical runner import report id mismatch")
    return value




def extract_archive(archive_path: pathlib.Path, destination: pathlib.Path, policy: dict[str, Any]) -> pathlib.Path:
    archive_path = archive_path.resolve()
    if not archive_path.exists() or archive_path.is_symlink() or not archive_path.is_file():
        fail("runner evidence archive must be a regular non-symlink file")
    limits = policy["package_limits"]
    if archive_path.stat().st_size > limits["maximum_total_bytes"] + 1024 * 1024:
        fail("runner evidence archive exceeds compressed byte limit")
    destination.mkdir(parents=True, mode=0o700)
    os.chmod(destination, 0o700)
    seen: set[str] = set()
    total = 0
    try:
        with tarfile.open(archive_path, mode="r:gz") as archive:
            members = archive.getmembers()
            if not members or len(members) > limits["maximum_file_count"]:
                fail("runner evidence archive member count is invalid")
            for member in members:
                name = member.name
                if not member.isfile() or pathlib.PurePosixPath(name).name != name or name in seen:
                    fail("runner evidence archive contains an unsafe or duplicate member")
                if member.size < 0 or member.size > limits["maximum_file_bytes"]:
                    fail(f"runner evidence archive member exceeds byte limit: {name}")
                total += member.size
                if total > limits["maximum_total_bytes"]:
                    fail("runner evidence archive exceeds total uncompressed byte limit")
                source = archive.extractfile(member)
                if source is None:
                    fail(f"runner evidence archive member cannot be read: {name}")
                payload = source.read(limits["maximum_file_bytes"] + 1)
                if len(payload) != member.size:
                    fail(f"runner evidence archive member size differs: {name}")
                target = destination / name
                with target.open("xb") as handle:
                    handle.write(payload)
                os.chmod(target, 0o600)
                seen.add(name)
    except (tarfile.TarError, EOFError) as error:
        fail(f"runner evidence archive is malformed: {error}")
    return destination


def import_from_args(args: argparse.Namespace) -> dict[str, Any]:
    archive = getattr(args, "input_archive", None)
    directory = getattr(args, "input_dir", None)
    if (archive is None) == (directory is None):
        fail("exactly one runner evidence directory or archive is required")
    if directory is not None:
        return import_package(args)
    policy = PKG.load_policy(args.runner_policy.resolve())
    with tempfile.TemporaryDirectory() as raw:
        extracted = extract_archive(archive.resolve(), pathlib.Path(raw) / "package", policy)
        normalized = argparse.Namespace(**vars(args))
        normalized.input_dir = extracted
        normalized.input_archive = None
        return import_package(normalized)

def import_package(args: argparse.Namespace) -> dict[str, Any]:
    root = args.input_dir.resolve()
    manifest, reports = verify_package(
        root,
        expected_repository=args.expected_repository,
        expected_source_revision=args.expected_source_revision,
        runner_policy_path=args.runner_policy.resolve(),
        diagnostics_policy_path=args.diagnostics_policy.resolve(),
    )
    diagnostics_policy = D.load_policy(args.diagnostics_policy.resolve())
    primary_reports = [(path, report) for path, report in reports if report.get("report_kind") != "promotion-explanation"]
    packaged_explanations = [(path, report) for path, report in reports if report.get("report_kind") == "promotion-explanation"]
    if not primary_reports:
        fail("runner package has no primary diagnostic reports")
    if len(packaged_explanations) > 1:
        fail("runner package contains multiple promotion explanations")
    explanation = EXPLAIN.explain(primary_reports, diagnostics_policy)
    D.verify_report(explanation, diagnostics_policy)
    if packaged_explanations:
        packaged_path, packaged = packaged_explanations[0]
        if packaged != explanation:
            fail(f"packaged promotion explanation differs from recomputed explanation: {packaged_path.name}")
    report_refs = [
        {
            "name": path.name,
            "sha256": D.sha256_file(path),
            "report_id": report["report_id"],
            "report_kind": report["report_kind"],
            "status": report["status"],
            "report_digest_sha256": report["report_digest_sha256"],
        }
        for path, report in reports
    ]
    source_package = {
        "package_id": manifest["package_id"],
        "package_digest_sha256": manifest["package_digest_sha256"],
        "manifest_sha256": D.sha256_file(root / "RUNNER-MANIFEST.json"),
    }
    identity = {
        "runner_policy_sha256": manifest["runner_policy_sha256"],
        "diagnostics_policy_sha256": manifest["diagnostics_policy_sha256"],
        "source_package": source_package,
        "repository": manifest["repository"],
        "source_revision": manifest["source_revision"],
        "source_ref": manifest["context"]["source_ref"],
        "source_ref_head": manifest["context"]["source_ref_head"],
        "source_ref_ancestry_verified": True,
        "workflow": manifest["context"]["workflow"],
        "event_name": manifest["context"]["event_name"],
        "run_id": manifest["run_id"],
        "run_attempt": manifest["run_attempt"],
        "runner_os": manifest["context"]["runner_os"],
        "runner_arch": manifest["context"]["runner_arch"],
        "reports": report_refs,
        "explanation_report_digest_sha256": explanation["report_digest_sha256"],
        "root_causes": explanation["root_causes"],
    }
    digest = D.sha256_bytes(D.canonical_bytes(identity))
    return {
        "schema_version": 1,
        "report_kind": "canonical-runner-import",
        "report_id": f"health-promotion-run-import-{digest[:24]}",
        "status": "verified",
        "release_id": manifest["release_id"],
        "repository": manifest["repository"],
        "source_revision": manifest["source_revision"],
        "source_ref": manifest["context"]["source_ref"],
        "source_ref_head": manifest["context"]["source_ref_head"],
        "source_ref_ancestry_verified": True,
        "workflow": manifest["context"]["workflow"],
        "event_name": manifest["context"]["event_name"],
        "run_id": manifest["run_id"],
        "run_attempt": manifest["run_attempt"],
        "runner_os": manifest["context"]["runner_os"],
        "runner_arch": manifest["context"]["runner_arch"],
        "runner_policy_sha256": manifest["runner_policy_sha256"],
        "diagnostics_policy_sha256": manifest["diagnostics_policy_sha256"],
        "source_package": source_package,
        "reports": report_refs,
        "explanation": {
            "report_id": explanation["report_id"],
            "status": explanation["status"],
            "report_digest_sha256": explanation["report_digest_sha256"],
            "root_causes": explanation["root_causes"],
            "owner_queue": explanation["owner_queue"],
        },
        "claims": {
            "package_manifest_verified": True,
            "all_report_digests_recomputed": True,
            "all_reason_graphs_recomputed": True,
            "repository_and_revision_matched_expectation": True,
            "unmanifested_files_rejected": True,
            "raw_command_output_not_imported": True,
            "promotion_is_not_authorized_by_import": True,
        },
        "report_digest_sha256": digest,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runner-policy", type=pathlib.Path, default=RUNNER_POLICY)
    parser.add_argument("--diagnostics-policy", type=pathlib.Path, default=D.DEFAULT_POLICY)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--input-dir", type=pathlib.Path)
    source.add_argument("--input-archive", type=pathlib.Path)
    parser.add_argument("--expected-repository", required=True)
    parser.add_argument("--expected-source-revision", required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    result = import_from_args(args)
    D.write_create_only(args.output.resolve(), result)
    print(args.output.resolve())
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RunnerImportError, PKG.RunnerPackageError, D.DiagnosticError, EXPLAIN.ExplanationError, OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        print(f"clinical promotion runner import error: {error}")
        raise SystemExit(1)
