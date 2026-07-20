#!/usr/bin/env python3
"""Gate clinical-promotion preflight, rehearsal, refusal, and explanation semantics."""
from __future__ import annotations

import importlib.util
import json
import pathlib
import py_compile
import subprocess
import sys
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]
POLICY = ROOT / "release/clinical-promotion-diagnostics-policy.json"
CASES = ROOT / "tests/clinical-promotion/refusal-cases.json"


def fail(message: str) -> NoReturn:
    raise SystemExit(f"clinical promotion diagnostics contract failed: {message}")


def load(path: pathlib.Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        fail(f"{path} must contain an object")
    return value


def module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def run(*args: str) -> None:
    result = subprocess.run(args, cwd=ROOT, text=True, capture_output=True)
    if result.returncode != 0:
        fail(f"{' '.join(args)} returned {result.returncode}: {result.stderr or result.stdout}")


def main() -> None:
    policy = load(POLICY)
    stages = policy.get("stage_order")
    dependencies = policy.get("stage_dependencies")
    if policy.get("schema_version") != 1 or policy.get("release_id") != "health-v1":
        fail("diagnostics policy identity drifted")
    if not isinstance(stages, list) or stages[-1:] != ["promotion"]:
        fail("promotion must remain the final diagnostic stage")
    if not isinstance(dependencies, dict) or set(dependencies.get("promotion", [])) != {
        "release_evidence",
        "compatibility",
        "supply_chain",
        "reproducibility",
        "attestation",
        "empirical_suite",
        "source_coherence",
    }:
        fail("promotion dependency graph drifted")
    if set(policy.get("statuses", [])) != {"verified", "refused", "unavailable", "skipped"}:
        fail("diagnostic status vocabulary drifted")
    required_codes = {
        "PREREQUISITE_TOOL_MISSING",
        "INPUT_MISSING",
        "INPUT_UNSAFE",
        "INPUT_INVALID_JSON",
        "UNRESOLVED_PLACEHOLDER",
        "REPORT_NOT_VERIFIED",
        "SOURCE_REVISION_MISMATCH",
        "CURRENT_MATERIAL_DIGEST_MISMATCH",
        "ARTIFACT_DIGEST_MISMATCH",
        "EMPIRICAL_SUITE_INELIGIBLE",
        "ATTESTATION_UNAVAILABLE",
        "ONLINE_AUDIT_UNAVAILABLE",
        "PROMOTION_OUTPUT_EXISTS",
        "PROMOTION_POLICY_REFUSAL",
        "INTERNAL_ERROR",
    }
    if not required_codes.issubset(policy.get("reason_codes", {})):
        fail("required refusal taxonomy is incomplete")

    scripts = [
        ROOT / "scripts/clinical-promotion-diagnostics.py",
        ROOT / "scripts/preflight-clinical-promotion.py",
        ROOT / "scripts/rehearse-clinical-promotion.py",
        ROOT / "scripts/promote-clinical-release.py",
        ROOT / "scripts/explain-clinical-promotion.py",
        ROOT / "scripts/check-clinical-promotion-diagnostics.py",
    ]
    for path in scripts:
        py_compile.compile(str(path), doraise=True)

    promotion = module("health_clinical_promotion", ROOT / "scripts/promote-clinical-release.py")
    cases = load(CASES)
    if cases.get("schema_version") != 1 or len(cases.get("cases", [])) < 10:
        fail("refusal corpus is too small or has the wrong version")
    for case in cases["cases"]:
        code, stage = promotion.classify_failure(case["message"])
        if code != case["expected_code"] or stage != case["expected_stage"]:
            fail(f"refusal classification drifted for {case['message']!r}: {(code, stage)}")

    run(sys.executable, "scripts/clinical-promotion-diagnostics.py", "--self-test")
    run(sys.executable, "scripts/explain-clinical-promotion.py", "--self-test")

    promotion_text = (ROOT / "scripts/promote-clinical-release.py").read_text(encoding="utf-8")
    required_fragments = [
        "--refusal-output",
        "promotion-refusal",
        "promotion_output_created",
        "write_create_only",
        "refusal output must be outside the promotion output directory",
    ]
    for fragment in required_fragments:
        if fragment not in promotion_text:
            fail(f"promotion refusal contract lacks {fragment!r}")
    if "os.environ" in promotion_text:
        fail("promotion refusal report must not enumerate environment values")

    preflight_text = (ROOT / "scripts/preflight-clinical-promotion.py").read_text(encoding="utf-8")
    for fragment in ("is_symlink", "group/world writable", "--require-canonical-tools", "UNRESOLVED_PLACEHOLDER"):
        if fragment not in preflight_text:
            fail(f"preflight lacks {fragment!r}")

    explanation_text = (ROOT / "scripts/explain-clinical-promotion.py").read_text(encoding="utf-8")
    if "root_cause_stages" not in explanation_text or "blocked_downstream_stages_are_not_misreported" not in explanation_text:
        fail("root-cause explanation boundary drifted")

    print("clinical promotion diagnostics contract: ok")


if __name__ == "__main__":
    main()
