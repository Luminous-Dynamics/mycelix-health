#!/usr/bin/env python3
"""Exact product-profile matching for independently verified qualification bundles.

This module deliberately does not mint a #212 seam token. It proves only that a
cryptographically/governance-verified qualification bundle matches one exact
adapter profile. Current-lineage-head and product-consumption time semantics
remain in QUAL-EVID-002.
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import pathlib
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("qualification_receipt", ROOT / "scripts/qualification-receipt.py")
assert SPEC and SPEC.loader
Q = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(Q)


class ProfileMatchError(ValueError):
    pass


def fail(message: str) -> None:
    raise ProfileMatchError(message)


def validate_profile(profile: dict[str, Any]) -> dict[str, Any]:
    if profile.get("schema_version") != 1:
        fail("qualification adapter profile schema_version must be 1")
    if profile.get("seam_id") != "Patient208CompositionCommitment":
        fail("unsupported qualification adapter seam")
    expected = {
        "receipt_kind": "CompositionCommitmentQualification",
        "admission_scope": "Cutover",
    }
    for field, value in expected.items():
        if profile.get(field) != value:
            fail(f"qualification adapter profile {field} is invalid")
    if not isinstance(profile.get("contract_version"), int) or profile["contract_version"] <= 0:
        fail("qualification adapter profile contract_version must be positive")
    for field in (
        "contract_digest",
        "qualification_lineage",
        "subject_digest",
        "context_digest",
        "claim_profile_digest",
        "governance_policy_digest",
        "trust_store_digest",
        "deployment_evidence_digest",
    ):
        Q.hex32(profile.get(field), f"profile {field}")
    maximum = profile.get("max_consumption_age_micros")
    if not isinstance(maximum, int) or isinstance(maximum, bool) or maximum <= 0:
        fail("qualification adapter profile max_consumption_age_micros must be positive")
    return profile


def match_verified_bundle(
    bundle: dict[str, Any],
    policy: dict[str, Any],
    trust: dict[str, Any],
    profile: dict[str, Any],
    verification_time_micros: int,
) -> dict[str, Any]:
    Q.verify_bundle(bundle, policy, trust, verification_time_micros)
    validate_profile(profile)
    statement = bundle["statement"]

    exact_fields = (
        "receipt_kind",
        "contract_digest",
        "contract_version",
        "qualification_lineage",
        "subject_digest",
        "context_digest",
        "admission_scope",
        "claim_profile_digest",
        "governance_policy_digest",
        "trust_store_digest",
        "deployment_evidence_digest",
    )
    for field in exact_fields:
        if statement.get(field) != profile.get(field):
            fail(f"verified qualification does not match profile field: {field}")

    issued = statement["issued_at_micros"]
    expires = statement["expires_at_micros"]
    if verification_time_micros < issued:
        fail("qualification profile match is before receipt issuance")
    if verification_time_micros >= expires:
        fail("qualification profile match uses expired receipt")
    if verification_time_micros - issued > profile["max_consumption_age_micros"]:
        fail("qualification receipt exceeds profile consumption age")

    identity = {
        "seam_id": profile["seam_id"],
        "profile_digest_sha256": Q.object_digest(profile),
        "qualification_bundle_digest_sha256": bundle["bundle_digest_sha256"],
        "receipt_digest_sha256": bundle["receipt_digest_sha256"],
        "contract_digest": statement["contract_digest"],
        "contract_version": statement["contract_version"],
        "qualification_lineage": statement["qualification_lineage"],
        "subject_digest": statement["subject_digest"],
        "context_digest": statement["context_digest"],
        "claim_profile_digest": statement["claim_profile_digest"],
        "governance_policy_digest": statement["governance_policy_digest"],
        "trust_store_digest": statement["trust_store_digest"],
        "deployment_evidence_digest": statement["deployment_evidence_digest"],
        "matched_at_micros": verification_time_micros,
    }
    digest = Q.sha256_bytes(Q.canonical_json_bytes(identity))
    return {
        "schema_version": 1,
        "report_kind": "mycelix-health-qualification-profile-match-v1",
        "profile_match_id": f"health-qualification-profile-{digest[:24]}",
        **identity,
        "profile_match_digest_sha256": digest,
        "claims": {
            "governed_bundle_independently_verified": True,
            "exact_adapter_profile_fields_match": True,
            "receipt_current_under_profile_age": True,
            "current_lineage_head_not_established": True,
            "product_seam_authority_not_established": True,
        },
    }


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser()
    value.add_argument("bundle", type=pathlib.Path)
    value.add_argument("--policy", type=pathlib.Path, required=True)
    value.add_argument("--trust-store", type=pathlib.Path, required=True)
    value.add_argument("--profile", type=pathlib.Path, required=True)
    value.add_argument("--verification-time-micros", type=int, required=True)
    value.add_argument("--output", type=pathlib.Path, required=True)
    return value


def main() -> int:
    args = parser().parse_args()
    bundle = Q.load_json(args.bundle)
    policy = Q.load_json(args.policy)
    trust = Q.load_json(args.trust_store)
    profile = Q.load_json(args.profile)
    result = match_verified_bundle(bundle, policy, trust, profile, args.verification_time_micros)
    Q.write_json_create_only(args.output, result)
    print(result["profile_match_id"])
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (Q.QualificationError, ProfileMatchError, OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        print(f"qualification profile match failed: {error}")
        raise SystemExit(1)
