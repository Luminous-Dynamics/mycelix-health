#!/usr/bin/env python3
"""Gate canonical-runner evidence import and bounded remediation semantics."""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
import pathlib
import py_compile
import shutil
import tempfile
import tarfile
from types import SimpleNamespace
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]
RUNNER_POLICY = ROOT / "release/clinical-promotion-runner-evidence-policy.json"
REMEDIATION_POLICY = ROOT / "release/clinical-promotion-remediation-policy.json"
CASES = ROOT / "tests/clinical-promotion/remediation-cases.json"
SOURCE_REVISION = "1" * 40
REPOSITORY = "luminous-dynamics/mycelix-health"


def fail(message: str) -> NoReturn:
    raise SystemExit(f"clinical promotion runner evidence contract failed: {message}")


def module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


D = module("health_promotion_diagnostics", ROOT / "scripts/clinical-promotion-diagnostics.py")
PKG = module("health_promotion_runner_package", ROOT / "scripts/package-clinical-promotion-run.py")
IMPORT = module("health_promotion_run_import", ROOT / "scripts/import-clinical-promotion-run.py")
PLAN = module("health_promotion_remediation", ROOT / "scripts/plan-clinical-promotion-remediation.py")
COMPARE = module("health_promotion_run_comparison", ROOT / "scripts/compare-clinical-promotion-runs.py")


def load(path: pathlib.Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        fail(f"{path.name} must contain an object")
    return value


def stages_with_root(code: str | None = None, stage: str = "preflight") -> list[dict[str, Any]]:
    policy = D.load_policy()
    required: set[str] = set()

    def include_dependencies(name: str) -> None:
        for dependency in policy["stage_dependencies"][name]:
            if dependency not in required:
                required.add(dependency)
                include_dependencies(dependency)

    if code is not None:
        include_dependencies(stage)
    results = []
    for name in policy["stage_order"]:
        if code is not None and name == stage:
            status = "unavailable" if code in {"PREREQUISITE_TOOL_MISSING", "ATTESTATION_UNAVAILABLE", "ONLINE_AUDIT_UNAVAILABLE"} else "refused"
            results.append(
                D.stage_result(
                    policy,
                    name,
                    status,
                    reasons=[D.reason(policy, code, f"synthetic {code.lower()} refusal", stage=name)],
                )
            )
        elif code is None or name in required:
            results.append(D.stage_result(policy, name, "verified"))
        else:
            results.append(D.stage_result(policy, name, "skipped"))
    return results


def report(path: pathlib.Path, *, code: str | None = None, stage: str = "preflight") -> dict[str, Any]:
    value = D.build_report(
        D.load_policy(),
        stages_with_root(code, stage),
        source_revision=SOURCE_REVISION,
        inputs={},
        report_kind="rehearsal",
    )
    D.write_create_only(path, value)
    D.verify_report(value, D.load_policy())
    return value


def package_args(output: pathlib.Path, report_path: pathlib.Path, attempt: int) -> SimpleNamespace:
    return SimpleNamespace(
        runner_policy=RUNNER_POLICY,
        diagnostics_policy=D.DEFAULT_POLICY,
        repository=REPOSITORY,
        source_revision=SOURCE_REVISION,
        source_ref="refs/heads/main",
        source_ref_head="2" * 40,
        source_ref_ancestry_verified=True,
        workflow=".github/workflows/clinical-promotion-rehearsal.yml",
        event_name="workflow_dispatch",
        run_id=7001,
        run_attempt=attempt,
        runner_os="Linux",
        runner_arch="X64",
        report=[report_path],
        output_dir=output,
    )


def import_args(package: pathlib.Path) -> SimpleNamespace:
    return SimpleNamespace(
        input_dir=package,
        expected_repository=REPOSITORY,
        expected_source_revision=SOURCE_REVISION,
        runner_policy=RUNNER_POLICY,
        diagnostics_policy=D.DEFAULT_POLICY,
    )


def expect_failure(function, fragment: str) -> None:
    try:
        function()
    except Exception as error:
        if fragment not in str(error):
            fail(f"expected failure containing {fragment!r}, got {error!r}")
        return
    fail(f"expected failure containing {fragment!r}")


def self_test() -> None:
    runner_policy = PKG.load_policy(RUNNER_POLICY)
    remediation_policy = PLAN.load_policy(REMEDIATION_POLICY)
    with tempfile.TemporaryDirectory() as raw:
        root = pathlib.Path(raw)
        first_report = root / "first-rehearsal.json"
        first_value = report(first_report, code="PREREQUISITE_TOOL_MISSING")
        explain_module = module("health_promotion_explanation_self_test", ROOT / "scripts/explain-clinical-promotion.py")
        first_explanation = explain_module.explain([(first_report, first_value)], D.load_policy())
        first_explanation_path = root / "first-explanation.json"
        D.write_create_only(first_explanation_path, first_explanation)
        first_package = root / "first-package"
        first_args = package_args(first_package, first_report, 1)
        first_args.report.append(first_explanation_path)
        first_manifest = PKG.package(first_args)
        if first_manifest["status"] != "verified":
            fail("synthetic first runner package was not verified")
        first_archive = root / "first-package.tar.gz"
        archive_summary = PKG.create_archive(first_package, first_archive, runner_policy)
        if archive_summary["sha256"] != D.sha256_file(first_archive):
            fail("deterministic runner archive digest drifted")
        archive_args = import_args(first_package)
        archive_args.input_dir = None
        archive_args.input_archive = first_archive
        first_import = IMPORT.import_from_args(archive_args)
        IMPORT.verify_import_report(first_import)
        mismatched_explanation = root / "mismatched-explanation-package"
        shutil.copytree(first_package, mismatched_explanation)
        explanation_path = mismatched_explanation / "first-explanation.json"
        altered = json.loads(explanation_path.read_text())
        altered["owner_queue"] = {"forged": [1]}
        identity = {
            "policy_sha256": altered["policy_sha256"],
            "reports": altered["source_reports"],
            "root_causes": altered["root_causes"],
        }
        # Owner queue is outside the explanation digest, so importer must compare the full report too.
        explanation_path.write_text(json.dumps(altered, indent=2, sort_keys=True) + "\n")
        os.chmod(explanation_path, 0o600)
        manifest_path = mismatched_explanation / "RUNNER-MANIFEST.json"
        manifest = json.loads(manifest_path.read_text())
        for entry in manifest["reports"]:
            if entry["name"] == explanation_path.name:
                entry["sha256"] = D.sha256_file(explanation_path)
                entry["size_bytes"] = explanation_path.stat().st_size
        manifest_identity = {
            "runner_policy_sha256": manifest["runner_policy_sha256"],
            "diagnostics_policy_sha256": manifest["diagnostics_policy_sha256"],
            "context": manifest["context"],
            "reports": sorted(manifest["reports"], key=lambda item: item["name"]),
            "claims": manifest["claims"],
        }
        manifest["package_digest_sha256"] = D.sha256_bytes(D.canonical_bytes(manifest_identity))
        manifest["package_id"] = f"health-promotion-run-{manifest['package_digest_sha256'][:24]}"
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
        os.chmod(manifest_path, 0o600)
        expect_failure(lambda: IMPORT.import_package(import_args(mismatched_explanation)), "differs from recomputed explanation")
        first_import_path = root / "first-import.json"
        D.write_create_only(first_import_path, first_import)
        first_plan = PLAN.build_plan(first_import_path, first_import, remediation_policy)
        PLAN.verify_plan(first_plan, remediation_policy)
        if first_plan["status"] != "infrastructure-action-required":
            fail("missing prerequisite did not produce infrastructure action")
        if [item["action_type"] for item in first_plan["actions"]] != ["restore_prerequisite"]:
            fail("missing prerequisite remediation action drifted")
        if any(item["executes_commands"] for item in first_plan["actions"]):
            fail("remediation plan became executable")

        tampered = root / "tampered-package"
        shutil.copytree(first_package, tampered)
        report_path = tampered / "first-rehearsal.json"
        os.chmod(report_path, 0o600)
        value = json.loads(report_path.read_text())
        value["status"] = "verified"
        report_path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
        os.chmod(report_path, 0o600)
        expect_failure(lambda: IMPORT.verify_package(tampered, expected_repository=REPOSITORY, expected_source_revision=SOURCE_REVISION), "bytes differ")

        extra = root / "extra-package"
        shutil.copytree(first_package, extra)
        (extra / "raw.log").write_text("raw output must not be retained\n")
        os.chmod(extra / "raw.log", 0o600)
        expect_failure(lambda: IMPORT.verify_package(extra, expected_repository=REPOSITORY, expected_source_revision=SOURCE_REVISION), "unmanifested files")
        expect_failure(lambda: IMPORT.verify_package(first_package, expected_repository="other/repo", expected_source_revision=SOURCE_REVISION), "repository differs")
        expect_failure(lambda: IMPORT.verify_package(first_package, expected_repository=REPOSITORY, expected_source_revision="2" * 40), "source revision differs")
        duplicate_archive = root / "first-package-duplicate.tar.gz"
        PKG.create_archive(first_package, duplicate_archive, runner_policy)
        if first_archive.read_bytes() != duplicate_archive.read_bytes():
            fail("runner evidence archive is not byte-for-byte deterministic")
        malicious = root / "malicious.tar.gz"
        with tarfile.open(malicious, "w:gz") as archive:
            info = tarfile.TarInfo("../escape.json")
            payload = b"{}\n"
            info.size = len(payload)
            archive.addfile(info, __import__("io").BytesIO(payload))
        expect_failure(lambda: IMPORT.extract_archive(malicious, root / "malicious-out", runner_policy), "unsafe")

        second_report = root / "second-rehearsal.json"
        report(second_report)
        second_package = root / "second-package"
        PKG.package(package_args(second_package, second_report, 2))
        second_import = IMPORT.import_package(import_args(second_package))
        second_import_path = root / "second-import.json"
        D.write_create_only(second_import_path, second_import)
        comparison = COMPARE.compare(first_import_path, second_import_path)
        if comparison["status"] != "cleared" or comparison["introduced_root_causes"]:
            fail("cleared runner refusal was not classified correctly")

        third_report = root / "third-rehearsal.json"
        report(third_report, code="CURRENT_MATERIAL_DIGEST_MISMATCH", stage="supply_chain")
        third_package = root / "third-package"
        third_args = package_args(third_package, third_report, 3)
        PKG.package(third_args)
        third_import = IMPORT.import_package(import_args(third_package))
        third_import_path = root / "third-import.json"
        D.write_create_only(third_import_path, third_import)
        regression = COMPARE.compare(second_import_path, third_import_path)
        if regression["status"] != "regressed":
            fail("introduced root cause was not classified as a regression")
        expected = [{"stage": "supply_chain", "reason_code": "CURRENT_MATERIAL_DIGEST_MISMATCH"}]
        if regression["introduced_root_causes"] != expected:
            fail("introduced root-cause identity drifted")

        # A digest-preserving edit is impossible because every report identity is recomputed.
        forged = dict(first_import)
        forged["explanation"] = dict(forged["explanation"])
        forged["explanation"]["root_causes"] = []
        expect_failure(lambda: IMPORT.verify_import_report(forged), "digest mismatch")
        forged_context = dict(first_import)
        forged_context["source_ref_head"] = "3" * 40
        expect_failure(lambda: IMPORT.verify_import_report(forged_context), "digest mismatch")
        forged_workflow = dict(first_import)
        forged_workflow["workflow"] = ".github/workflows/ci.yml"
        expect_failure(lambda: IMPORT.verify_import_report(forged_workflow), "digest mismatch")
    print("clinical promotion runner evidence self-test: ok")


def main() -> None:
    runner = load(RUNNER_POLICY)
    remediation = load(REMEDIATION_POLICY)
    cases = load(CASES)
    if runner.get("schema_version") != 1 or runner.get("release_id") != "health-v1":
        fail("runner evidence policy identity drifted")
    if remediation.get("schema_version") != 1 or remediation.get("release_id") != "health-v1":
        fail("remediation policy identity drifted")
    if remediation.get("claims", {}).get("plan_executes_no_commands") is not True:
        fail("remediation policy no-command claim drifted")
    if remediation.get("claims", {}).get("plan_mutates_no_evidence") is not True:
        fail("remediation policy no-evidence-mutation claim drifted")
    if not isinstance(cases.get("cases"), list) or len(cases["cases"]) != len(remediation["reason_action_map"]):
        fail("remediation corpus does not cover every reason")
    observed = {item["reason_code"]: item["expected_action_type"] for item in cases["cases"]}
    if observed != remediation["reason_action_map"]:
        fail("remediation corpus differs from policy map")
    protected = set(remediation.get("protected_paths", []))
    required_protected = {
        "release/clinical-promotion-policy.json",
        "release/clinical-promotion-diagnostics-policy.json",
        "release/supply-chain-policy.json",
        "release/health-v1.sbom.cdx.json",
        "release/github-actions-lock.json",
    }
    if not required_protected.issubset(protected):
        fail("remediation protected-path set is incomplete")

    scripts = [
        ROOT / "scripts/clinical-promotion-diagnostics.py",
        ROOT / "scripts/package-clinical-promotion-run.py",
        ROOT / "scripts/import-clinical-promotion-run.py",
        ROOT / "scripts/plan-clinical-promotion-remediation.py",
        ROOT / "scripts/compare-clinical-promotion-runs.py",
        ROOT / "scripts/check-clinical-promotion-runner-evidence.py",
    ]
    for path in scripts:
        py_compile.compile(str(path), doraise=True)
    planner = (ROOT / "scripts/plan-clinical-promotion-remediation.py").read_text(encoding="utf-8")
    for forbidden in ("subprocess", "os.system", "shutil.copy", "write_text(", "unlink(", "rename("):
        if forbidden in planner:
            fail(f"remediation planner contains forbidden mutation or execution primitive: {forbidden}")
    importer = (ROOT / "scripts/import-clinical-promotion-run.py").read_text(encoding="utf-8")
    for required in ("unmanifested files", "verify_report", "expected_repository", "expected_source_revision"):
        if required not in importer:
            fail(f"runner importer lacks {required!r}")

    workflow_path = ROOT / ".github/workflows/clinical-promotion-rehearsal.yml"
    workflow = workflow_path.read_text(encoding="utf-8")
    for required in (
        "gh run download",
        "--require-canonical-tools",
        "package-clinical-promotion-run.py",
        "import-clinical-promotion-run.py",
        "plan-clinical-promotion-remediation.py",
        "clinical-promotion-runner-evidence",
        "persist-credentials: false",
        "git merge-base --is-ancestor",
        "--source-ref-head",
        "--source-ref-ancestry-verified",
        "--archive",
        "--input-archive",
    ):
        if required not in workflow:
            fail(f"canonical rehearsal workflow lacks {required!r}")
    upload_block = workflow.split("- name: Upload verified runner evidence and advisory plan", 1)[-1]
    if "target/clinical-promotion-inputs" in upload_block or "target/clinical-promotion-reports" in upload_block:
        fail("canonical rehearsal workflow uploads raw inputs or intermediate reports")
    if "continue-on-error" in workflow:
        fail("canonical rehearsal workflow must handle refusal exit codes explicitly")

    ci = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    justfile = (ROOT / "justfile").read_text(encoding="utf-8")
    supply_gate = (ROOT / "scripts/check-supply-chain-release.py").read_text(encoding="utf-8")
    for text, label in ((ci, "CI"), (justfile, "justfile"), (supply_gate, "supply-chain gate")):
        if "check-clinical-promotion-runner-evidence.py" not in text:
            fail(f"{label} does not execute the runner-evidence gate")
    documentation = ROOT / "docs/release/CANONICAL_PROMOTION_RUNNER_EVIDENCE.md"
    if not documentation.exists() or "authorize clinical promotion" not in documentation.read_text(encoding="utf-8"):
        fail("canonical runner evidence documentation is missing its non-authorization boundary")
    self_test()
    print("clinical promotion runner evidence contract: ok")


if __name__ == "__main__":
    main()
