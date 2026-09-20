#!/usr/bin/env python3
"""Executable tests for exact #208 profile matching over governed qualification bundles."""
from __future__ import annotations

import copy
import importlib.util
import pathlib
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
Q_SPEC = importlib.util.spec_from_file_location("qualification_receipt", ROOT / "scripts/qualification-receipt.py")
assert Q_SPEC and Q_SPEC.loader
Q = importlib.util.module_from_spec(Q_SPEC)
Q_SPEC.loader.exec_module(Q)
P_SPEC = importlib.util.spec_from_file_location("qualification_receipt_profile", ROOT / "scripts/qualification-receipt-profile.py")
assert P_SPEC and P_SPEC.loader
P = importlib.util.module_from_spec(P_SPEC)
P_SPEC.loader.exec_module(P)


def hx(value: int) -> str:
    return (bytes([value]) * 32).hex()


def keypair(root: pathlib.Path, name: str) -> tuple[pathlib.Path, str]:
    private = root / f"{name}.pem"
    public = root / f"{name}.pub.pem"
    subprocess.run(["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(private)], check=True, capture_output=True)
    subprocess.run(["openssl", "pkey", "-in", str(private), "-pubout", "-out", str(public)], check=True, capture_output=True)
    return private, Q.public_raw_hex(public)


class ProfileMatchTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="qualification-profile-test-")
        root = pathlib.Path(self.temp.name)
        self.alice_private, alice_public = keypair(root, "alice")
        self.bob_private, bob_public = keypair(root, "bob")
        self.policy = {
            "schema_version": 1,
            "policy_id": "mycelix-health-qualification-governance-v1",
            "trust_store_id": "profile-test-trust-v1",
            "rules": [{
                "receipt_kind": "CompositionCommitmentQualification",
                "allowed_scopes": ["Cutover"],
                "minimum_approvers": 2,
                "minimum_organizations": 2,
                "required_roles": ["privacy", "safety"],
                "max_statement_lifetime_micros": 10_000_000,
                "max_attestation_age_micros": 5_000_000,
                "deployment_evidence_required": True
            }]
        }
        self.trust = {
            "schema_version": 1,
            "trust_store_id": "profile-test-trust-v1",
            "approvers": [
                {"approver_id": "alice", "signer_key_id": "alice-v1", "ed25519_public_key": alice_public, "organization": "org-a", "roles": ["privacy"], "status": "active", "valid_from_micros": 0, "valid_until_micros": 20_000_000, "revoked_at_micros": None, "compromised_at_micros": None},
                {"approver_id": "bob", "signer_key_id": "bob-v1", "ed25519_public_key": bob_public, "organization": "org-b", "roles": ["safety"], "status": "active", "valid_from_micros": 0, "valid_until_micros": 20_000_000, "revoked_at_micros": None, "compromised_at_micros": None}
            ]
        }
        self.statement = self.make_statement()
        self.profile = self.make_profile()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def make_statement(self, **overrides: object) -> dict:
        value = {
            "schema_version": 1,
            "receipt_kind": "CompositionCommitmentQualification",
            "contract_digest": hx(1),
            "contract_version": 1,
            "qualification_lineage": hx(2),
            "subject_digest": hx(3),
            "dependency_set_digest": hx(4),
            "deployment_evidence_digest": hx(5),
            "context_digest": hx(6),
            "outcome": "Qualified",
            "issued_at_micros": 1_000_000,
            "expires_at_micros": 9_000_000,
            "nonce": hx(10),
            "sequence": 1,
            "previous_receipt": None,
            "governance_policy_digest": Q.object_digest(self.policy),
            "trust_store_digest": Q.object_digest(self.trust),
            "admission_scope": "Cutover",
            "claim_profile_digest": hx(9)
        }
        value.update(overrides)
        return value

    def make_profile(self, **overrides: object) -> dict:
        value = {
            "schema_version": 1,
            "seam_id": "Patient208CompositionCommitment",
            "receipt_kind": "CompositionCommitmentQualification",
            "contract_digest": hx(1),
            "contract_version": 1,
            "qualification_lineage": hx(2),
            "subject_digest": hx(3),
            "context_digest": hx(6),
            "admission_scope": "Cutover",
            "claim_profile_digest": hx(9),
            "governance_policy_digest": Q.object_digest(self.policy),
            "trust_store_digest": Q.object_digest(self.trust),
            "deployment_evidence_digest": hx(5),
            "max_consumption_age_micros": 5_000_000
        }
        value.update(overrides)
        return value

    def bundle(self, statement: dict) -> dict:
        signatures = [
            Q.sign_statement(statement, self.alice_private, "alice", "alice-v1", 2_000_000),
            Q.sign_statement(statement, self.bob_private, "bob", "bob-v1", 2_000_000),
        ]
        return Q.build_bundle(statement, signatures, self.policy, self.trust, 3_000_000)

    def assert_profile_rejects(self, bundle: dict, profile: dict, text: str) -> None:
        with self.assertRaises(P.ProfileMatchError) as caught:
            P.match_verified_bundle(bundle, self.policy, self.trust, profile, 3_000_000)
        self.assertIn(text, str(caught.exception))

    def test_exact_signed_bundle_matches_profile_but_does_not_claim_seam_authority(self) -> None:
        result = P.match_verified_bundle(self.bundle(self.statement), self.policy, self.trust, self.profile, 3_000_000)
        self.assertEqual(result["seam_id"], "Patient208CompositionCommitment")
        self.assertTrue(result["claims"]["exact_adapter_profile_fields_match"])
        self.assertTrue(result["claims"]["current_lineage_head_not_established"])
        self.assertTrue(result["claims"]["product_seam_authority_not_established"])

    def test_freshly_signed_wrong_contract_is_valid_generic_evidence_but_not_profile_authority(self) -> None:
        statement = self.make_statement(contract_digest=hx(99))
        bundle = self.bundle(statement)
        self.assertEqual(Q.verify_bundle(bundle, self.policy, self.trust, 3_000_000), bundle)
        self.assert_profile_rejects(bundle, self.profile, "contract_digest")

    def test_freshly_signed_wrong_subject_context_or_claim_profile_is_rejected(self) -> None:
        for field, value in (("subject_digest", hx(90)), ("context_digest", hx(91)), ("claim_profile_digest", hx(92))):
            statement = self.make_statement(**{field: value})
            self.assert_profile_rejects(self.bundle(statement), self.profile, field)

    def test_wrong_profile_governance_or_deployment_identity_rejected(self) -> None:
        for field, value in (("governance_policy_digest", hx(70)), ("trust_store_digest", hx(71)), ("deployment_evidence_digest", hx(72))):
            profile = self.make_profile(**{field: value})
            self.assert_profile_rejects(self.bundle(self.statement), profile, field)

    def test_profile_age_can_be_stricter_than_generic_receipt_lifetime(self) -> None:
        profile = self.make_profile(max_consumption_age_micros=1_000_000)
        bundle = self.bundle(self.statement)
        with self.assertRaises(P.ProfileMatchError) as caught:
            P.match_verified_bundle(bundle, self.policy, self.trust, profile, 3_000_000)
        self.assertIn("consumption age", str(caught.exception))

    def test_bundle_tamper_is_rejected_before_profile_matching(self) -> None:
        bundle = self.bundle(self.statement)
        tampered = copy.deepcopy(bundle)
        tampered["receipt_digest_sha256"] = hx(99)
        with self.assertRaises(Q.QualificationError):
            P.match_verified_bundle(tampered, self.policy, self.trust, self.profile, 3_000_000)


if __name__ == "__main__":
    unittest.main(verbosity=2)
