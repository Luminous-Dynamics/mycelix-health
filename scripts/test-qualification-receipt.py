#!/usr/bin/env python3
"""Adversarial executable tests for scripts/qualification-receipt.py."""
from __future__ import annotations

import copy
import importlib.util
import json
import pathlib
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("qualification_receipt", ROOT / "scripts/qualification-receipt.py")
assert SPEC and SPEC.loader
Q = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(Q)


def hx(value: int) -> str:
    return bytes([value]) * 32 .hex() if False else (bytes([value]) * 32).hex()


def generate_keypair(root: pathlib.Path, name: str) -> tuple[pathlib.Path, pathlib.Path, str]:
    private = root / f"{name}.private.pem"
    public = root / f"{name}.public.pem"
    subprocess.run(["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(private)], check=True, capture_output=True)
    subprocess.run(["openssl", "pkey", "-in", str(private), "-pubout", "-out", str(public)], check=True, capture_output=True)
    return private, public, Q.public_raw_hex(public)


class QualificationReceiptTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="qualification-receipt-test-")
        self.root = pathlib.Path(self.temp.name)
        self.alice_private, _, alice_public = generate_keypair(self.root, "alice")
        self.bob_private, _, bob_public = generate_keypair(self.root, "bob")
        self.policy = {
            "schema_version": 1,
            "policy_id": "mycelix-health-qualification-governance-v1",
            "trust_store_id": "qualification-test-trust-v1",
            "rules": [
                {
                    "receipt_kind": "CompositionCommitmentQualification",
                    "allowed_scopes": ["Cutover"],
                    "minimum_approvers": 2,
                    "minimum_organizations": 2,
                    "required_roles": ["privacy", "safety"],
                    "max_statement_lifetime_micros": 10_000_000,
                    "max_attestation_age_micros": 5_000_000,
                    "deployment_evidence_required": True
                }
            ]
        }
        self.trust = {
            "schema_version": 1,
            "trust_store_id": "qualification-test-trust-v1",
            "approvers": [
                {
                    "approver_id": "alice",
                    "signer_key_id": "alice-ed25519-v1",
                    "ed25519_public_key": alice_public,
                    "organization": "org-a",
                    "roles": ["privacy"],
                    "status": "active",
                    "valid_from_micros": 0,
                    "valid_until_micros": 20_000_000,
                    "revoked_at_micros": None,
                    "compromised_at_micros": None
                },
                {
                    "approver_id": "bob",
                    "signer_key_id": "bob-ed25519-v1",
                    "ed25519_public_key": bob_public,
                    "organization": "org-b",
                    "roles": ["safety"],
                    "status": "active",
                    "valid_from_micros": 0,
                    "valid_until_micros": 20_000_000,
                    "revoked_at_micros": None,
                    "compromised_at_micros": None
                }
            ]
        }
        self.statement = self.make_statement(self.policy, self.trust)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def make_statement(self, policy: dict, trust: dict, **overrides: object) -> dict:
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
            "governance_policy_digest": Q.object_digest(policy),
            "trust_store_digest": Q.object_digest(trust),
            "admission_scope": "Cutover",
            "claim_profile_digest": hx(9)
        }
        value.update(overrides)
        return value

    def signatures(self, statement: dict | None = None, attested_at: int = 2_000_000) -> list[dict]:
        statement = statement or self.statement
        return [
            Q.sign_statement(statement, self.alice_private, "alice", "alice-ed25519-v1", attested_at),
            Q.sign_statement(statement, self.bob_private, "bob", "bob-ed25519-v1", attested_at),
        ]

    def build(self, statement: dict | None = None, policy: dict | None = None, trust: dict | None = None, signatures: list[dict] | None = None, verification_time: int = 3_000_000) -> dict:
        statement = statement or self.statement
        policy = policy or self.policy
        trust = trust or self.trust
        signatures = signatures or self.signatures(statement)
        return Q.build_bundle(statement, signatures, policy, trust, verification_time)

    def assert_rejects(self, fn, contains: str | None = None) -> None:
        with self.assertRaises(Q.QualificationError) as caught:
            fn()
        if contains is not None:
            self.assertIn(contains, str(caught.exception))

    def test_rust_python_golden_transcript(self) -> None:
        vector = json.loads((ROOT / "release/qualification-receipt-golden-v1.json").read_text())
        transcript = Q.qualification_transcript(vector["statement"])
        self.assertEqual(len(transcript), vector["transcript_length_bytes"])
        self.assertEqual(Q.sha256_bytes(transcript), vector["transcript_sha256"])

    def test_exact_valid_two_party_bundle_and_independent_rebuild(self) -> None:
        bundle = self.build()
        rebuilt = Q.verify_bundle(bundle, self.policy, self.trust, 3_000_000)
        self.assertEqual(bundle, rebuilt)
        self.assertEqual(len(bundle["approvals"]), 2)
        self.assertEqual(bundle["verified_organizations"], ["org-a", "org-b"])
        self.assertEqual(bundle["verified_roles"], ["privacy", "safety"])
        self.assertTrue(bundle["claims"]["underlying_theorem_truth_not_established"])

    def test_statement_mutation_after_signing_rejected(self) -> None:
        signatures = self.signatures()
        mutated = copy.deepcopy(self.statement)
        mutated["subject_digest"] = hx(99)
        self.assert_rejects(lambda: self.build(mutated, signatures=signatures), "another receipt")

    def test_attestation_time_mutation_after_signing_rejected_cryptographically(self) -> None:
        signatures = self.signatures()
        signatures[0] = dict(signatures[0])
        signatures[0]["attested_at_micros"] = 2_000_001
        self.assert_rejects(lambda: self.build(signatures=signatures), "signature verification failed")

    def test_wrong_scope_is_governance_rejected_even_when_freshly_signed(self) -> None:
        statement = self.make_statement(self.policy, self.trust, admission_scope="Reference")
        signatures = self.signatures(statement)
        self.assert_rejects(lambda: self.build(statement, signatures=signatures), "scope")

    def test_wrong_policy_or_trust_digest_rejected(self) -> None:
        wrong_policy = dict(self.statement)
        wrong_policy["governance_policy_digest"] = hx(77)
        self.assert_rejects(lambda: self.build(wrong_policy, signatures=self.signatures(wrong_policy)), "policy digest")
        wrong_trust = dict(self.statement)
        wrong_trust["trust_store_digest"] = hx(78)
        self.assert_rejects(lambda: self.build(wrong_trust, signatures=self.signatures(wrong_trust)), "trust-store digest")

    def test_unknown_signer_rejected(self) -> None:
        signatures = self.signatures()
        signatures[0] = dict(signatures[0])
        signatures[0]["approver_id"] = "mallory"
        self.assert_rejects(lambda: self.build(signatures=signatures), "unknown qualification approver")

    def test_revoked_and_compromised_approvers_rejected(self) -> None:
        revoked = copy.deepcopy(self.trust)
        revoked["approvers"][0]["status"] = "revoked"
        statement = self.make_statement(self.policy, revoked)
        self.assert_rejects(lambda: self.build(statement, trust=revoked, signatures=self.signatures(statement)), "not eligible")

        compromised = copy.deepcopy(self.trust)
        compromised["approvers"][1]["compromised_at_micros"] = 1_500_000
        statement = self.make_statement(self.policy, compromised)
        self.assert_rejects(lambda: self.build(statement, trust=compromised, signatures=self.signatures(statement)), "not eligible")

    def test_duplicate_approver_and_signer_rejected(self) -> None:
        signature = self.signatures()[0]
        self.assert_rejects(lambda: self.build(signatures=[signature, signature]), "distinct approvers")

        trust = copy.deepcopy(self.trust)
        trust["approvers"][1]["signer_key_id"] = trust["approvers"][0]["signer_key_id"]
        self.assert_rejects(lambda: Q.validate_trust_store(trust, self.policy), "signer-key IDs")

    def test_organization_diversity_and_required_role_enforced(self) -> None:
        one_org = copy.deepcopy(self.trust)
        one_org["approvers"][1]["organization"] = "org-a"
        statement = self.make_statement(self.policy, one_org)
        self.assert_rejects(lambda: self.build(statement, trust=one_org, signatures=self.signatures(statement)), "organization diversity")

        missing_role = copy.deepcopy(self.trust)
        missing_role["approvers"][1]["roles"] = ["privacy"]
        statement = self.make_statement(self.policy, missing_role)
        self.assert_rejects(lambda: self.build(statement, trust=missing_role, signatures=self.signatures(statement)), "required roles")

    def test_statement_and_attestation_freshness_enforced(self) -> None:
        self.assert_rejects(lambda: self.build(verification_time=999_999), "not current")
        self.assert_rejects(lambda: self.build(verification_time=9_000_000), "not current")
        stale = self.signatures(attested_at=2_000_000)
        self.assert_rejects(lambda: self.build(signatures=stale, verification_time=8_000_001), "stale")

    def test_future_attestation_rejected(self) -> None:
        signatures = self.signatures(attested_at=4_000_000)
        self.assert_rejects(lambda: self.build(signatures=signatures, verification_time=3_000_000), "future")

    def test_bundle_tamper_fails_independent_reconstruction(self) -> None:
        bundle = self.build()
        tampered = copy.deepcopy(bundle)
        tampered["verified_roles"] = ["privacy", "safety", "clinical"]
        self.assert_rejects(lambda: Q.verify_bundle(tampered, self.policy, self.trust, 3_000_000), "independent reconstruction")

    def test_statement_shape_fails_closed(self) -> None:
        bad = dict(self.statement)
        bad["receipt_kind"] = "UnknownQualification"
        self.assert_rejects(lambda: Q.qualification_transcript(bad), "unsupported receipt kind")
        bad = dict(self.statement)
        bad["sequence"] = 2
        self.assert_rejects(lambda: Q.qualification_transcript(bad), "predecessor shape")
        bad = dict(self.statement)
        bad["deployment_evidence_digest"] = None
        signatures = self.signatures(bad)
        self.assert_rejects(lambda: self.build(bad, signatures=signatures), "requires deployment evidence")


if __name__ == "__main__":
    unittest.main(verbosity=2)
