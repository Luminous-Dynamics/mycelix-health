#!/usr/bin/env python3
"""Adversarial tests for governed durable qualification-lineage checkpoints."""
from __future__ import annotations

import copy
import importlib.util
import json
import pathlib
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


def load(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


Q = load("qualification_receipt", ROOT / "scripts/qualification-receipt.py")
P = load("qualification_receipt_profile", ROOT / "scripts/qualification-receipt-profile.py")
C = load(
    "qualification_lineage_checkpoint",
    ROOT / "scripts/qualification-lineage-checkpoint.py",
)


def h(byte: str) -> str:
    return byte * 64


class Harness:
    def __init__(self, root: pathlib.Path):
        self.root = root
        self.keys = []
        for name in ("privacy", "safety", "other"):
            private = root / f"{name}.pem"
            public = root / f"{name}.pub.pem"
            subprocess.run(
                ["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(private)],
                check=True,
                capture_output=True,
            )
            subprocess.run(
                ["openssl", "pkey", "-in", str(private), "-pubout", "-out", str(public)],
                check=True,
                capture_output=True,
            )
            self.keys.append((private, public))
        self.receipt_policy = {
            "schema_version": 1,
            "policy_id": "mycelix-health-qualification-governance-v1",
            "trust_store_id": "qualification-test-trust",
            "rules": [
                {
                    "receipt_kind": "CompositionCommitmentQualification",
                    "allowed_scopes": ["Cutover"],
                    "minimum_approvers": 2,
                    "minimum_organizations": 2,
                    "required_roles": ["privacy", "safety"],
                    "max_statement_lifetime_micros": 5_000,
                    "max_attestation_age_micros": 1_000,
                    "deployment_evidence_required": True,
                }
            ],
        }
        self.trust = {
            "schema_version": 1,
            "trust_store_id": "qualification-test-trust",
            "approvers": [
                {
                    "approver_id": "privacy-reviewer",
                    "signer_key_id": "privacy-key-v1",
                    "ed25519_public_key": Q.public_raw_hex(self.keys[0][1]),
                    "organization": "org-privacy",
                    "roles": ["privacy"],
                    "status": "active",
                    "valid_from_micros": 0,
                    "valid_until_micros": None,
                    "revoked_at_micros": None,
                    "compromised_at_micros": None,
                },
                {
                    "approver_id": "safety-reviewer",
                    "signer_key_id": "safety-key-v1",
                    "ed25519_public_key": Q.public_raw_hex(self.keys[1][1]),
                    "organization": "org-safety",
                    "roles": ["safety"],
                    "status": "active",
                    "valid_from_micros": 0,
                    "valid_until_micros": None,
                    "revoked_at_micros": None,
                    "compromised_at_micros": None,
                },
                {
                    "approver_id": "other-reviewer",
                    "signer_key_id": "other-key-v1",
                    "ed25519_public_key": Q.public_raw_hex(self.keys[2][1]),
                    "organization": "org-other",
                    "roles": ["privacy", "safety"],
                    "status": "active",
                    "valid_from_micros": 0,
                    "valid_until_micros": None,
                    "revoked_at_micros": None,
                    "compromised_at_micros": None,
                },
            ],
        }
        self.statement = {
            "schema_version": 1,
            "receipt_kind": "CompositionCommitmentQualification",
            "contract_digest": h("1"),
            "contract_version": 1,
            "qualification_lineage": h("2"),
            "subject_digest": h("3"),
            "dependency_set_digest": h("4"),
            "deployment_evidence_digest": h("5"),
            "context_digest": h("6"),
            "outcome": "Qualified",
            "issued_at_micros": 10,
            "expires_at_micros": 1_000,
            "nonce": h("a"),
            "sequence": 1,
            "previous_receipt": None,
            "governance_policy_digest": Q.object_digest(self.receipt_policy),
            "trust_store_digest": Q.object_digest(self.trust),
            "admission_scope": "Cutover",
            "claim_profile_digest": h("9"),
        }
        self.profile = {
            "schema_version": 1,
            "seam_id": "Patient208CompositionCommitment",
            "receipt_kind": "CompositionCommitmentQualification",
            "contract_digest": h("1"),
            "contract_version": 1,
            "qualification_lineage": h("2"),
            "subject_digest": h("3"),
            "context_digest": h("6"),
            "admission_scope": "Cutover",
            "claim_profile_digest": h("9"),
            "governance_policy_digest": self.statement["governance_policy_digest"],
            "trust_store_digest": self.statement["trust_store_digest"],
            "deployment_evidence_digest": h("5"),
            "max_consumption_age_micros": 500,
        }
        self.bundle = self._receipt_bundle(self.statement)
        self.profile_match = P.match_verified_bundle(
            self.bundle, self.receipt_policy, self.trust, self.profile, 200
        )
        self.lineage_state = {
            "schema_version": 1,
            "lineage_binding": {
                "contract_digest": h("1"),
                "contract_version": 1,
                "qualification_lineage": h("2"),
                "subject_digest": h("3"),
                "context_digest": h("6"),
                "admission_scope": "Cutover",
                "claim_profile_digest": h("9"),
            },
            "receipts": [
                {
                    "receipt_digest_sha256": self.bundle["receipt_digest_sha256"],
                    "sequence": 1,
                    "previous_receipt": None,
                    "nonce": self.statement["nonce"],
                }
            ],
            "dispositions": [],
            "latest_ledger_time_micros": 200,
        }
        self.checkpoint_policy = {
            "schema_version": 1,
            "policy_id": C.CHECKPOINT_POLICY_ID,
            "trust_store_id": self.trust["trust_store_id"],
            "minimum_approvers": 2,
            "minimum_organizations": 2,
            "required_roles": ["privacy", "safety"],
            "max_statement_lifetime_micros": 2_000,
            "max_attestation_age_micros": 1_000,
            "deployment_evidence_required": True,
            "required_profile_seam": "Patient208CompositionCommitment",
        }

    def _receipt_bundle(self, statement: dict) -> dict:
        signatures = [
            Q.sign_statement(
                statement,
                self.keys[0][0],
                "privacy-reviewer",
                "privacy-key-v1",
                100,
            ),
            Q.sign_statement(
                statement,
                self.keys[1][0],
                "safety-reviewer",
                "safety-key-v1",
                100,
            ),
        ]
        return Q.build_bundle(statement, signatures, self.receipt_policy, self.trust, 200)

    def checkpoint_statement(
        self,
        *,
        state: dict | None = None,
        profile_match: dict | None = None,
        epoch: int = 1,
        previous: str | None = None,
        nonce: str | None = None,
        issued: int = 210,
        expires: int = 900,
    ) -> dict:
        return C.build_checkpoint_statement(
            self.lineage_state if state is None else state,
            self.profile_match if profile_match is None else profile_match,
            self.checkpoint_policy,
            self.trust,
            h("5"),
            epoch,
            previous,
            issued,
            expires,
            h("b") if nonce is None else nonce,
        )

    def checkpoint_bundle(
        self,
        statement: dict,
        *,
        privacy=True,
        safety=True,
        trust=None,
        attested=220,
        verify_at=250,
    ) -> dict:
        signatures = []
        if privacy:
            signatures.append(
                C.sign_checkpoint(
                    statement,
                    self.keys[0][0],
                    "privacy-reviewer",
                    "privacy-key-v1",
                    attested,
                )
            )
        if safety:
            signatures.append(
                C.sign_checkpoint(
                    statement,
                    self.keys[1][0],
                    "safety-reviewer",
                    "safety-key-v1",
                    attested,
                )
            )
        return C.assemble_checkpoint_bundle(
            statement,
            signatures,
            self.checkpoint_policy,
            self.trust if trust is None else trust,
            verify_at,
        )


class CheckpointTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="qualification-checkpoint-test-")
        self.root = pathlib.Path(self.temp.name)
        self.h = Harness(self.root)

    def tearDown(self):
        self.temp.cleanup()

    def test_exact_linear_checkpoint_survives_restart_and_emits_typed_head(self):
        statement = self.h.checkpoint_statement()
        bundle = self.h.checkpoint_bundle(statement)
        store = self.root / "checkpoint-store.json"
        self.assertEqual(
            C.admit_checkpoint(store, bundle, self.h.checkpoint_policy, self.h.trust, 250),
            "Added",
        )
        reloaded = json.loads(store.read_text())
        self.assertEqual(
            reloaded["latest_checkpoint_digest_sha256"], bundle["checkpoint_digest_sha256"]
        )
        head = C.emit_verified_head(
            store, bundle, self.h.checkpoint_policy, self.h.trust, 260
        )
        self.assertEqual(
            head["head_receipt_digest_sha256"], self.h.bundle["receipt_digest_sha256"]
        )
        self.assertEqual(
            head["profile_match_commitment_sha256"],
            self.h.profile_match["profile_match_digest_sha256"],
        )
        self.assertTrue(head["claims"]["checkpoint_is_durable_current_checkpoint"])
        self.assertTrue(head["claims"]["product_seam_conversion_not_performed"])
        self.assertEqual(json.loads(store.read_text())["latest_time_micros"], 260)

    def test_fresh_process_can_reload_store_and_emit_same_head(self):
        statement = self.h.checkpoint_statement()
        bundle = self.h.checkpoint_bundle(statement)
        store = self.root / "checkpoint-store.json"
        C.admit_checkpoint(store, bundle, self.h.checkpoint_policy, self.h.trust, 250)
        bundle_path = self.root / "bundle.json"
        policy_path = self.root / "checkpoint-policy.json"
        trust_path = self.root / "trust.json"
        output = self.root / "head.json"
        bundle_path.write_text(json.dumps(bundle), encoding="utf-8")
        policy_path.write_text(json.dumps(self.h.checkpoint_policy), encoding="utf-8")
        trust_path.write_text(json.dumps(self.h.trust), encoding="utf-8")
        result = subprocess.run(
            [
                "python3",
                str(ROOT / "scripts/qualification-lineage-checkpoint.py"),
                "emit-head",
                str(bundle_path),
                "--policy",
                str(policy_path),
                "--trust-store",
                str(trust_path),
                "--now-micros",
                "260",
                "--store",
                str(store),
                "--output",
                str(output),
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        head = json.loads(output.read_text())
        self.assertEqual(
            head["head_receipt_digest_sha256"], self.h.bundle["receipt_digest_sha256"]
        )

    def test_exact_replay_is_idempotent(self):
        statement = self.h.checkpoint_statement()
        bundle = self.h.checkpoint_bundle(statement)
        store = self.root / "checkpoint-store.json"
        self.assertEqual(
            C.admit_checkpoint(store, bundle, self.h.checkpoint_policy, self.h.trust, 250),
            "Added",
        )
        self.assertEqual(
            C.admit_checkpoint(store, bundle, self.h.checkpoint_policy, self.h.trust, 400),
            "IdempotentReplay",
        )
        self.assertEqual(json.loads(store.read_text())["latest_time_micros"], 400)
        later_statement = self.h.checkpoint_statement(
            epoch=2,
            previous=bundle["checkpoint_digest_sha256"],
            nonce=h("c"),
            issued=300,
            expires=850,
        )
        later_bundle = self.h.checkpoint_bundle(
            later_statement, attested=320, verify_at=350
        )
        with self.assertRaises(C.CheckpointError):
            C.admit_checkpoint(
                store, later_bundle, self.h.checkpoint_policy, self.h.trust, 350
            )

    def test_forked_receipt_state_never_emits_head_authority(self):
        root_digest = self.h.bundle["receipt_digest_sha256"]
        forked = copy.deepcopy(self.h.lineage_state)
        forked["receipts"].extend(
            [
                {
                    "receipt_digest_sha256": h("c"),
                    "sequence": 2,
                    "previous_receipt": root_digest,
                    "nonce": h("d"),
                },
                {
                    "receipt_digest_sha256": h("e"),
                    "sequence": 2,
                    "previous_receipt": root_digest,
                    "nonce": h("f"),
                },
            ]
        )
        statement = self.h.checkpoint_statement(state=forked)
        self.assertEqual(statement["lineage_state"], "Forked")
        self.assertIsNone(statement["current_head_receipt_digest_sha256"])
        bundle = self.h.checkpoint_bundle(statement)
        self.assertFalse(bundle["claims"]["current_head_authority_eligible"])
        store = self.root / "checkpoint-store.json"
        C.admit_checkpoint(store, bundle, self.h.checkpoint_policy, self.h.trust, 250)
        with self.assertRaises(C.CheckpointError):
            C.emit_verified_head(store, bundle, self.h.checkpoint_policy, self.h.trust, 260)

    def test_inactive_head_cannot_emit_authority(self):
        inactive = copy.deepcopy(self.h.lineage_state)
        inactive["dispositions"] = [
            {
                "target_receipt_digest_sha256": self.h.bundle["receipt_digest_sha256"],
                "disposition": "Abandon",
                "governance_evidence_digest": h("7"),
                "disposition_epoch": 1,
            }
        ]
        statement = self.h.checkpoint_statement(state=inactive)
        self.assertEqual(statement["lineage_state"], "InactiveHead")
        bundle = self.h.checkpoint_bundle(statement)
        store = self.root / "checkpoint-store.json"
        C.admit_checkpoint(store, bundle, self.h.checkpoint_policy, self.h.trust, 250)
        with self.assertRaises(C.CheckpointError):
            C.emit_verified_head(store, bundle, self.h.checkpoint_policy, self.h.trust, 260)

    def test_profile_match_commitment_tamper_is_rejected(self):
        tampered = copy.deepcopy(self.h.profile_match)
        tampered["profile_match_digest_sha256"] = h("8")
        with self.assertRaises(C.CheckpointError):
            self.h.checkpoint_statement(profile_match=tampered)

    def test_profile_match_must_target_exact_active_head(self):
        other = copy.deepcopy(self.h.profile_match)
        identity = {
            key: other.get(key)
            for key in (
                "seam_id",
                "profile_digest_sha256",
                "qualification_bundle_digest_sha256",
                "receipt_digest_sha256",
                "contract_digest",
                "contract_version",
                "qualification_lineage",
                "subject_digest",
                "context_digest",
                "claim_profile_digest",
                "governance_policy_digest",
                "trust_store_digest",
                "deployment_evidence_digest",
                "matched_at_micros",
            )
        }
        identity["receipt_digest_sha256"] = h("8")
        other.update(identity)
        other["profile_match_digest_sha256"] = Q.sha256_bytes(
            P.profile_match_transcript(identity)
        )
        with self.assertRaises(C.CheckpointError):
            self.h.checkpoint_statement(profile_match=other)

    def test_attestation_timestamp_and_statement_mutation_invalidate_signature(self):
        statement = self.h.checkpoint_statement()
        first = C.sign_checkpoint(
            statement,
            self.h.keys[0][0],
            "privacy-reviewer",
            "privacy-key-v1",
            220,
        )
        second = C.sign_checkpoint(
            statement,
            self.h.keys[1][0],
            "safety-reviewer",
            "safety-key-v1",
            220,
        )
        changed_time = copy.deepcopy(first)
        changed_time["attested_at_micros"] = 221
        with self.assertRaises(Q.QualificationError):
            C.assemble_checkpoint_bundle(
                statement,
                [changed_time, second],
                self.h.checkpoint_policy,
                self.h.trust,
                250,
            )
        changed_statement = copy.deepcopy(statement)
        changed_statement["latest_ledger_time_micros"] += 1
        with self.assertRaises(C.CheckpointError):
            C.assemble_checkpoint_bundle(
                changed_statement,
                [first, second],
                self.h.checkpoint_policy,
                self.h.trust,
                250,
            )

    def test_revoked_or_compromised_signer_and_bad_quorum_reject(self):
        statement = self.h.checkpoint_statement()
        first = C.sign_checkpoint(
            statement,
            self.h.keys[0][0],
            "privacy-reviewer",
            "privacy-key-v1",
            220,
        )
        second = C.sign_checkpoint(
            statement,
            self.h.keys[1][0],
            "safety-reviewer",
            "safety-key-v1",
            220,
        )
        revoked = copy.deepcopy(self.h.trust)
        revoked["approvers"][0]["revoked_at_micros"] = 200
        with self.assertRaises(C.CheckpointError):
            C.assemble_checkpoint_bundle(
                statement,
                [first, second],
                self.h.checkpoint_policy,
                revoked,
                250,
            )
        with self.assertRaises(C.CheckpointError):
            C.assemble_checkpoint_bundle(
                statement,
                [first],
                self.h.checkpoint_policy,
                self.h.trust,
                250,
            )

    def test_checkpoint_epoch_predecessor_nonce_and_clock_are_monotonic(self):
        first_statement = self.h.checkpoint_statement()
        first_bundle = self.h.checkpoint_bundle(first_statement)
        store = self.root / "checkpoint-store.json"
        C.admit_checkpoint(store, first_bundle, self.h.checkpoint_policy, self.h.trust, 250)

        second_statement = self.h.checkpoint_statement(
            epoch=2,
            previous=first_bundle["checkpoint_digest_sha256"],
            nonce=h("c"),
            issued=260,
            expires=950,
        )
        second_bundle = self.h.checkpoint_bundle(
            second_statement, attested=270, verify_at=300
        )
        self.assertEqual(
            C.admit_checkpoint(
                store, second_bundle, self.h.checkpoint_policy, self.h.trust, 300
            ),
            "Added",
        )
        with self.assertRaises(C.CheckpointError):
            C.emit_verified_head(
                store, first_bundle, self.h.checkpoint_policy, self.h.trust, 300
            )

        stale_epoch = self.h.checkpoint_statement(
            epoch=2,
            previous=second_bundle["checkpoint_digest_sha256"],
            nonce=h("d"),
            issued=310,
            expires=980,
        )
        stale_bundle = self.h.checkpoint_bundle(
            stale_epoch, attested=320, verify_at=330
        )
        with self.assertRaises(C.CheckpointError):
            C.admit_checkpoint(
                store, stale_bundle, self.h.checkpoint_policy, self.h.trust, 330
            )

        reused_nonce = self.h.checkpoint_statement(
            epoch=3,
            previous=second_bundle["checkpoint_digest_sha256"],
            nonce=h("c"),
            issued=310,
            expires=980,
        )
        reused_bundle = self.h.checkpoint_bundle(
            reused_nonce, attested=320, verify_at=330
        )
        with self.assertRaises(C.CheckpointError):
            C.admit_checkpoint(
                store, reused_bundle, self.h.checkpoint_policy, self.h.trust, 330
            )

        rollback_statement = self.h.checkpoint_statement(
            epoch=3,
            previous=second_bundle["checkpoint_digest_sha256"],
            nonce=h("d"),
            issued=280,
            expires=980,
        )
        rollback_bundle = self.h.checkpoint_bundle(
            rollback_statement, attested=290, verify_at=299
        )
        with self.assertRaises(C.CheckpointError):
            C.admit_checkpoint(
                store, rollback_bundle, self.h.checkpoint_policy, self.h.trust, 299
            )

    def test_fork_lock_survives_restart_and_denies_linear_revival(self):
        root_digest = self.h.bundle["receipt_digest_sha256"]
        forked = copy.deepcopy(self.h.lineage_state)
        forked["receipts"].extend(
            [
                {
                    "receipt_digest_sha256": h("c"),
                    "sequence": 2,
                    "previous_receipt": root_digest,
                    "nonce": h("d"),
                },
                {
                    "receipt_digest_sha256": h("e"),
                    "sequence": 2,
                    "previous_receipt": root_digest,
                    "nonce": h("f"),
                },
            ]
        )
        first_statement = self.h.checkpoint_statement(state=forked)
        first_bundle = self.h.checkpoint_bundle(first_statement)
        store = self.root / "checkpoint-store.json"
        C.admit_checkpoint(store, first_bundle, self.h.checkpoint_policy, self.h.trust, 250)
        self.assertTrue(json.loads(store.read_text())["fork_locked"])

        linear_statement = self.h.checkpoint_statement(
            epoch=2,
            previous=first_bundle["checkpoint_digest_sha256"],
            nonce=h("c"),
            issued=260,
            expires=950,
        )
        linear_bundle = self.h.checkpoint_bundle(
            linear_statement, attested=270, verify_at=300
        )
        with self.assertRaises(C.CheckpointError):
            C.admit_checkpoint(
                store, linear_bundle, self.h.checkpoint_policy, self.h.trust, 300
            )

    def test_corrupt_durable_store_fails_closed(self):
        statement = self.h.checkpoint_statement()
        bundle = self.h.checkpoint_bundle(statement)
        store = self.root / "checkpoint-store.json"
        store.write_text("{not json", encoding="utf-8")
        with self.assertRaises(C.CheckpointError):
            C.admit_checkpoint(
                store, bundle, self.h.checkpoint_policy, self.h.trust, 250
            )

    def test_parseable_but_internally_inconsistent_store_fails_closed(self):
        statement = self.h.checkpoint_statement()
        bundle = self.h.checkpoint_bundle(statement)
        store = self.root / "checkpoint-store.json"
        C.admit_checkpoint(store, bundle, self.h.checkpoint_policy, self.h.trust, 250)
        value = json.loads(store.read_text())
        value["latest_checkpoint_epoch"] = 99
        store.write_text(json.dumps(value), encoding="utf-8")
        with self.assertRaises(C.CheckpointError):
            C.emit_verified_head(
                store, bundle, self.h.checkpoint_policy, self.h.trust, 260
            )

    def test_bundle_tampering_fails_independent_reconstruction(self):
        statement = self.h.checkpoint_statement()
        bundle = self.h.checkpoint_bundle(statement)
        tampered = copy.deepcopy(bundle)
        tampered["verified_roles"].append("clinical")
        with self.assertRaises(C.CheckpointError):
            C.verify_checkpoint_bundle(
                tampered, self.h.checkpoint_policy, self.h.trust, 250
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
