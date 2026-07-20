#!/usr/bin/env python3
"""Perform a zero-mutation preflight of a clinical promotion input set."""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
import pathlib
import shutil
import stat
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]
DIAGNOSTICS_PATH = ROOT / "scripts/clinical-promotion-diagnostics.py"
SPEC = importlib.util.spec_from_file_location("health_promotion_diagnostics", DIAGNOSTICS_PATH)
assert SPEC and SPEC.loader
D = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(D)


class PreflightError(ValueError):
    pass


def fail(message: str) -> NoReturn:
    raise PreflightError(message)


def contains_forbidden(value: Any, forbidden: set[str]) -> bool:
    if isinstance(value, str):
        return value in forbidden
    if isinstance(value, dict):
        return any(contains_forbidden(item, forbidden) for item in value.values())
    if isinstance(value, list):
        return any(contains_forbidden(item, forbidden) for item in value)
    return False


def safe_mode(path: pathlib.Path) -> bool:
    mode = stat.S_IMODE(path.lstat().st_mode)
    return mode & 0o022 == 0


def inspect_path(
    policy: dict[str, Any],
    role: str,
    path: pathlib.Path,
    expected_kind: str,
    *,
    check_forbidden: bool = False,
) -> tuple[list[dict[str, Any]], dict[str, Any] | None]:
    reasons: list[dict[str, Any]] = []
    if not path.exists():
        reasons.append(D.reason(policy, "INPUT_MISSING", f"{role} is missing", stage="preflight", details={"role": role, "name": path.name}))
        return reasons, None
    if path.is_symlink():
        reasons.append(D.reason(policy, "INPUT_UNSAFE", f"{role} must not be a symbolic link", stage="preflight", details={"role": role, "name": path.name}))
        return reasons, None
    if expected_kind == "file" and not path.is_file():
        reasons.append(D.reason(policy, "INPUT_UNSAFE", f"{role} must be a regular file", stage="preflight", details={"role": role, "name": path.name}))
        return reasons, None
    if expected_kind == "directory" and not path.is_dir():
        reasons.append(D.reason(policy, "INPUT_UNSAFE", f"{role} must be a directory", stage="preflight", details={"role": role, "name": path.name}))
        return reasons, None
    if not safe_mode(path):
        reasons.append(D.reason(policy, "INPUT_UNSAFE", f"{role} is group/world writable", stage="preflight", details={"role": role, "name": path.name, "mode": oct(stat.S_IMODE(path.lstat().st_mode))}))
    if expected_kind == "file":
        try:
            value = D.load_json(path)
        except D.DiagnosticError as error:
            reasons.append(D.reason(policy, "INPUT_INVALID_JSON", str(error), stage="preflight", details={"role": role, "name": path.name}))
            return reasons, None
        if check_forbidden and contains_forbidden(value, set(policy.get("forbidden_values", []))):
            reasons.append(D.reason(policy, "UNRESOLVED_PLACEHOLDER", f"{role} contains a forbidden source-checkout placeholder", stage="preflight", details={"role": role, "name": path.name}))
        return reasons, value
    return reasons, None


def input_paths(args: argparse.Namespace) -> dict[str, pathlib.Path]:
    return {
        "promotion_policy": args.policy,
        "release_bundle": args.release_bundle,
        "compatibility_report": args.compatibility_report,
        "supply_chain_report": args.supply_chain_report,
        "reproducibility_report": args.reproducibility_report,
        "reproducibility_provenance": args.reproducibility_provenance,
        "attestation_subjects": args.attestation_subjects,
        "attestation_report": args.attestation_report,
        "suite_root": args.suite_root,
    }


def run_preflight(args: argparse.Namespace) -> tuple[list[dict[str, Any]], str | None, dict[str, pathlib.Path]]:
    policy = D.load_policy(args.diagnostics_policy)
    inputs = input_paths(args)
    reasons: list[dict[str, Any]] = []
    missing_tools: list[str] = []
    tool_groups = ["core"] + (["canonical_release"] if args.require_canonical_tools else [])
    seen_tools: set[str] = set()
    for group in tool_groups:
        for tool in policy.get("tool_requirements", {}).get(group, []):
            if tool in seen_tools:
                continue
            seen_tools.add(tool)
            if shutil.which(tool) is None:
                missing_tools.append(tool)
    for tool in missing_tools:
        reasons.append(D.reason(policy, "PREREQUISITE_TOOL_MISSING", f"required tool is unavailable: {tool}", stage="preflight", details={"tool": tool}))

    parsed: dict[str, dict[str, Any]] = {}
    kinds = {
        "promotion_policy": "file",
        "release_bundle": "directory",
        "compatibility_report": "file",
        "supply_chain_report": "file",
        "reproducibility_report": "file",
        "reproducibility_provenance": "file",
        "attestation_subjects": "file",
        "attestation_report": "file",
        "suite_root": "directory",
    }
    for role, path in inputs.items():
        local_reasons, value = inspect_path(policy, role, path, kinds[role])
        reasons.extend(local_reasons)
        if value is not None:
            parsed[role] = value

    release_bundle = args.release_bundle
    if release_bundle.is_dir() and not release_bundle.is_symlink():
        for role, filename in (
            ("signed_release_evidence", "health-v1.signed-evidence.json"),
            ("trusted_release_signers", "trusted-release-signers.json"),
        ):
            path = release_bundle / filename
            local_reasons, value = inspect_path(policy, role, path, "file", check_forbidden=(role == "signed_release_evidence"))
            reasons.extend(local_reasons)
            if value is not None:
                parsed[role] = value

    suite_root = args.suite_root
    if suite_root.is_dir() and not suite_root.is_symlink():
        for role, filename in (
            ("suite_ledger", "SUITE-LEDGER.json"),
            ("suite_manifest", "SUITE-MANIFEST.json"),
        ):
            path = suite_root / filename
            local_reasons, value = inspect_path(policy, role, path, "file")
            reasons.extend(local_reasons)
            if value is not None:
                parsed[role] = value

    source_revision: str | None = None
    signed = parsed.get("signed_release_evidence", {})
    evidence = signed.get("evidence") if isinstance(signed, dict) else None
    if isinstance(evidence, dict):
        candidate = evidence.get("source_revision")
        if isinstance(candidate, str) and D.HEX40.fullmatch(candidate):
            source_revision = candidate
        elif candidate is not None:
            reasons.append(D.reason(policy, "SOURCE_REVISION_MISMATCH", "signed release evidence has an invalid source revision", stage="preflight"))

    refused_codes = {item["code"] for item in reasons if item["code"] != "PREREQUISITE_TOOL_MISSING"}
    if refused_codes:
        status = "refused"
    elif missing_tools:
        status = "unavailable"
    else:
        status = "verified"
    results = [D.stage_result(policy, "preflight", status, reasons=reasons)]
    results.extend(D.stage_result(policy, stage, "skipped") for stage in policy["stage_order"] if stage != "preflight")
    return results, source_revision, inputs


def add_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--diagnostics-policy", type=pathlib.Path, default=D.DEFAULT_POLICY)
    parser.add_argument("--policy", type=pathlib.Path, default=ROOT / "release/clinical-promotion-policy.json")
    parser.add_argument("--release-bundle", type=pathlib.Path, required=True)
    parser.add_argument("--compatibility-report", type=pathlib.Path, required=True)
    parser.add_argument("--supply-chain-report", type=pathlib.Path, required=True)
    parser.add_argument("--reproducibility-report", type=pathlib.Path, required=True)
    parser.add_argument("--reproducibility-provenance", type=pathlib.Path, required=True)
    parser.add_argument("--attestation-subjects", type=pathlib.Path, required=True)
    parser.add_argument("--attestation-report", type=pathlib.Path, required=True)
    parser.add_argument("--suite-root", type=pathlib.Path, required=True)
    parser.add_argument("--require-canonical-tools", action="store_true")


def main() -> int:
    parser = argparse.ArgumentParser()
    add_arguments(parser)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    results, source_revision, inputs = run_preflight(args)
    policy = D.load_policy(args.diagnostics_policy)
    report = D.build_report(
        policy,
        results,
        source_revision=source_revision,
        inputs=inputs,
        report_kind="preflight",
    )
    D.write_create_only(args.output.resolve(), report)
    print(args.output.resolve())
    return 0 if report["status"] == "verified" else (4 if report["status"] == "unavailable" else 3)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (PreflightError, D.DiagnosticError, OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        print(f"clinical promotion preflight error: {error}")
        raise SystemExit(1)
