#!/usr/bin/env python3
"""Evaluate the clinical promotion boundary without creating a promotion decision."""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import pathlib
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]


def load_module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


D = load_module("health_promotion_diagnostics", ROOT / "scripts/clinical-promotion-diagnostics.py")
PF = load_module("health_promotion_preflight", ROOT / "scripts/preflight-clinical-promotion.py")
P = load_module("health_clinical_promotion", ROOT / "scripts/promote-clinical-release.py")


class RehearsalError(ValueError):
    pass


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def reason_for_exception(policy: dict[str, Any], stage: str, error: Exception) -> dict[str, Any]:
    code = getattr(error, "code", None)
    expected = isinstance(error, (P.PromotionError, RehearsalError, D.DiagnosticError, OSError, ValueError, TypeError, KeyError, json.JSONDecodeError, P.release_evidence.EvidenceError))
    if code not in policy.get("reason_codes", {}) and not expected:
        code = "INTERNAL_ERROR"
    if code not in policy.get("reason_codes", {}):
        message = str(error).lower()
        if "placeholder" in message or "unresolved" in message:
            code = "UNRESOLVED_PLACEHOLDER"
        elif "source revision" in message:
            code = "SOURCE_REVISION_MISMATCH"
        elif "empirical" in message or "fault matrix" in message:
            code = "EMPIRICAL_SUITE_INELIGIBLE"
        elif "attestation" in message and "unavailable" in message:
            code = "ATTESTATION_UNAVAILABLE"
        elif "not verified" in message:
            code = "REPORT_NOT_VERIFIED"
        elif "supply-chain report does not bind" in message or "policy" in message and "digest" in message:
            code = "CURRENT_MATERIAL_DIGEST_MISMATCH"
        elif "digest differs" in message or "non-identical" in message or "artifact" in message and "differs" in message:
            code = "ARTIFACT_DIGEST_MISMATCH"
        else:
            code = "PROMOTION_POLICY_REFUSAL"
    return D.reason(policy, code, str(error), stage=stage)


def verified_or_reason(
    policy: dict[str, Any],
    stage: str,
    function,
) -> dict[str, Any]:
    try:
        evidence = function()
        return D.stage_result(policy, stage, "verified", evidence=evidence or None)
    except (Exception,) as error:  # bounded below by sanitized reason artifacts
        return D.stage_result(policy, stage, "refused", reasons=[reason_for_exception(policy, stage, error)])


def report_status(value: dict[str, Any], label: str) -> dict[str, Any]:
    if value.get("status") != "verified":
        raise RehearsalError(f"{label} is not verified")
    return {"status": "verified"}


def evaluate(args: argparse.Namespace) -> dict[str, Any]:
    policy = D.load_policy(args.diagnostics_policy)
    preflight_results, source_revision, inputs = PF.run_preflight(args)
    by_stage: dict[str, dict[str, Any]] = {"preflight": preflight_results[0]}
    if by_stage["preflight"]["status"] != "verified":
        for stage in policy["stage_order"]:
            if stage != "preflight":
                by_stage[stage] = D.stage_result(policy, stage, "skipped")
        return D.build_report(
            policy,
            [by_stage[stage] for stage in policy["stage_order"]],
            source_revision=source_revision,
            inputs=inputs,
            report_kind="rehearsal",
        )

    promotion_policy = P.load(args.policy.resolve())
    release_state: dict[str, Any] = {}

    def release_stage() -> dict[str, Any]:
        evidence, hashes = P.verify_release_bundle(args.release_bundle.resolve(), promotion_policy)
        release_state["evidence"] = evidence
        release_state["hashes"] = hashes
        return {
            "source_revision": evidence.get("source_revision"),
            "dna_hash": evidence.get("dna_hash"),
            "signed_evidence_sha256": hashes["signed_evidence_sha256"],
        }

    by_stage["release_evidence"] = verified_or_reason(policy, "release_evidence", release_stage)

    def compatibility_stage() -> dict[str, Any]:
        value = P.load(args.compatibility_report.resolve())
        report_status(value, "compatibility report")
        expected = release_state.get("hashes", {}).get("signed_evidence_sha256")
        if expected and value.get("signed_evidence_sha256") != expected:
            raise RehearsalError("compatibility report targets different signed evidence")
        return {"status": "verified", "report_sha256": sha256(args.compatibility_report.resolve())}

    by_stage["compatibility"] = (
        verified_or_reason(policy, "compatibility", compatibility_stage)
        if by_stage["release_evidence"]["status"] == "verified"
        else D.stage_result(policy, "compatibility", "skipped")
    )

    def supply_stage() -> dict[str, Any]:
        value = P.load(args.supply_chain_report.resolve())
        report_status(value, "supply-chain report")
        current = {
            "supply_chain_policy": sha256(ROOT / "release/supply-chain-policy.json"),
            "sbom": sha256(ROOT / "release/health-v1.sbom.cdx.json"),
            "github_actions_lock": sha256(ROOT / "release/github-actions-lock.json"),
        }
        for name, digest in current.items():
            if value.get("materials", {}).get(name, {}).get("sha256") != digest:
                raise RehearsalError(f"supply-chain report does not bind current {name}")
        return {"status": "verified", "source_revision": value.get("source_revision")}

    by_stage["supply_chain"] = verified_or_reason(policy, "supply_chain", supply_stage)

    repro_state: dict[str, Any] = {}

    def reproducibility_stage() -> dict[str, Any]:
        report = P.load(args.reproducibility_report.resolve())
        provenance = P.load(args.reproducibility_provenance.resolve())
        report_status(report, "reproducibility report")
        report_status(provenance, "reproducibility provenance")
        hashes = P.comparisons(report)
        repro_state["hashes"] = hashes
        return {
            "status": "verified",
            "artifact_count": len(hashes),
            "source_revision": provenance.get("source_revision"),
        }

    by_stage["reproducibility"] = verified_or_reason(policy, "reproducibility", reproducibility_stage)

    def attestation_stage() -> dict[str, Any]:
        subjects = P.load(args.attestation_subjects.resolve())
        report = P.load(args.attestation_report.resolve())
        report_status(report, "GitHub attestation report")
        names = {item.get("name") for item in subjects.get("subjects", [])}
        verified_names = {item.get("name") for item in report.get("subjects", [])}
        if names != {"health-happ", "health-sdk"} or verified_names != names:
            raise RehearsalError("attested subject set is incomplete")
        for item in subjects.get("subjects", []):
            match = next(entry for entry in report["subjects"] if entry.get("name") == item.get("name"))
            if match.get("sha256") != item.get("sha256") or match.get("size_bytes") != item.get("size_bytes"):
                raise RehearsalError(f"attestation report differs for {item.get('name')}")
        return {"status": "verified", "source_revision": report.get("source_revision"), "subject_names": sorted(names)}

    by_stage["attestation"] = (
        verified_or_reason(policy, "attestation", attestation_stage)
        if by_stage["reproducibility"]["status"] == "verified"
        else D.stage_result(policy, "attestation", "skipped")
    )

    def empirical_stage() -> dict[str, Any]:
        ledger = P.load(args.suite_root.resolve() / "SUITE-LEDGER.json")
        manifest = P.load(args.suite_root.resolve() / "SUITE-MANIFEST.json")
        if ledger.get("clinical_release_eligible") is not True or manifest.get("clinical_release_eligible") is not True:
            raise RehearsalError("empirical suite is not clinically release eligible")
        if promotion_policy["requirements"].get("require_fault_matrix_complete") and ledger.get("fault_matrix_complete") is not True:
            raise RehearsalError("fault matrix is incomplete")
        return {
            "clinical_release_eligible": True,
            "fault_matrix_complete": ledger.get("fault_matrix_complete") is True,
            "source_revision": ledger.get("source_revision"),
        }

    by_stage["empirical_suite"] = verified_or_reason(policy, "empirical_suite", empirical_stage)

    def coherence_stage() -> dict[str, Any]:
        evidence = release_state.get("evidence")
        if not isinstance(evidence, dict):
            raise RehearsalError("signed release evidence was not verified")
        revision = evidence.get("source_revision")
        sources = {
            "supply_chain": P.load(args.supply_chain_report.resolve()).get("source_revision"),
            "reproducibility": P.load(args.reproducibility_provenance.resolve()).get("source_revision"),
            "attestation_subjects": P.load(args.attestation_subjects.resolve()).get("source_revision"),
            "attestation_report": P.load(args.attestation_report.resolve()).get("source_revision"),
            "empirical_suite": P.load(args.suite_root.resolve() / "SUITE-LEDGER.json").get("source_revision"),
        }
        mismatches = sorted(name for name, value in sources.items() if value != revision)
        if mismatches:
            raise RehearsalError(f"promotion inputs do not share one source revision: {', '.join(mismatches)}")
        return {"source_revision": revision, "input_count": len(sources) + 1}

    prerequisites = ["release_evidence", "supply_chain", "reproducibility", "attestation", "empirical_suite"]
    by_stage["source_coherence"] = (
        verified_or_reason(policy, "source_coherence", coherence_stage)
        if all(by_stage[stage]["status"] == "verified" for stage in prerequisites)
        else D.stage_result(policy, "source_coherence", "skipped")
    )

    def promotion_stage() -> dict[str, Any]:
        decision = P.promote(args)
        return {
            "decision_id": decision.get("decision_id"),
            "decision_digest_sha256": decision.get("decision_digest_sha256"),
            "would_promote": True,
        }

    promotion_dependencies = policy["stage_dependencies"]["promotion"]
    by_stage["promotion"] = (
        verified_or_reason(policy, "promotion", promotion_stage)
        if all(by_stage[stage]["status"] == "verified" for stage in promotion_dependencies)
        else D.stage_result(policy, "promotion", "skipped")
    )

    if isinstance(release_state.get("evidence"), dict):
        source_revision = release_state["evidence"].get("source_revision")
    return D.build_report(
        policy,
        [by_stage[stage] for stage in policy["stage_order"]],
        source_revision=source_revision,
        inputs=inputs,
        report_kind="rehearsal",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    PF.add_arguments(parser)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    report = evaluate(args)
    D.write_create_only(args.output.resolve(), report)
    print(args.output.resolve())
    return 0 if report["status"] == "verified" else (4 if report["status"] == "unavailable" else 3)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RehearsalError, D.DiagnosticError, OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        print(f"clinical promotion rehearsal error: {error}")
        raise SystemExit(1)
