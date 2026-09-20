#!/usr/bin/env python3
"""Strict-schema and persisted-history tests for qualification checkpoints."""
from __future__ import annotations

import copy
import importlib.util
import json
import pathlib
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


def load(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


T = load(
    "checkpoint_tests",
    ROOT / "scripts/test-qualification-lineage-checkpoint.py",
)
A = load(
    "checkpoint_admission",
    ROOT / "scripts/qualification-lineage-checkpoint-admission.py",
)
C = load(
    "checkpoint_core",
    ROOT / "scripts/qualification-lineage-checkpoint.py",
)


class StrictAdmissionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="qualification-checkpoint-admission-")
        self.root = pathlib.Path(self.temp.name)
        self.h = T.Harness(self.root)

    def tearDown(self):
        self.temp.cleanup()

    def statement(self):
        return A.prepare_checkpoint(
            self.h.lineage_state,
            self.h.profile_match,
            self.h.checkpoint_policy,
            self.h.trust,
            deployment_evidence_digest=T.h("5"),
            checkpoint_epoch=1,
            previous_checkpoint_digest=None,
            issued_at_micros=210,
            expires_at_micros=900,
            checkpoint_nonce=T.h("b"),
        )

    def bundle(self):
        statement = self.statement()
        return self.h.checkpoint_bundle(statement)

    def test_unknown_lineage_state_field_is_rejected(self):
        state = copy.deepcopy(self.h.lineage_state)
        state["future_authority"] = True
        with self.assertRaises(A.AdmissionSchemaError):
            A.prepare_checkpoint(
                state,
                self.h.profile_match,
                self.h.checkpoint_policy,
                self.h.trust,
                deployment_evidence_digest=T.h("5"),
                checkpoint_epoch=1,
                previous_checkpoint_digest=None,
                issued_at_micros=210,
                expires_at_micros=900,
                checkpoint_nonce=T.h("b"),
            )

    def test_unknown_statement_and_bundle_fields_are_rejected(self):
        bundle = self.bundle()
        bundle["statement"]["authority_hint"] = "do-not-interpret"
        with self.assertRaises(A.AdmissionSchemaError):
            A.verify_bundle(bundle, self.h.checkpoint_policy, self.h.trust, 250)

        bundle = self.bundle()
        bundle["future_claims"] = {"active": True}
        with self.assertRaises(A.AdmissionSchemaError):
            A.verify_bundle(bundle, self.h.checkpoint_policy, self.h.trust, 250)

    def test_unknown_approval_policy_trust_and_profile_fields_are_rejected(self):
        bundle = self.bundle()
        bundle["approvals"][0]["unscoped_role"] = "admin"
        with self.assertRaises(A.AdmissionSchemaError):
            A.verify_bundle(bundle, self.h.checkpoint_policy, self.h.trust, 250)

        policy = copy.deepcopy(self.h.checkpoint_policy)
        policy["implicit_allow"] = True
        with self.assertRaises(A.AdmissionSchemaError):
            A.verify_bundle(self.bundle(), policy, self.h.trust, 250)

        trust = copy.deepcopy(self.h.trust)
        trust["approvers"][0]["superuser"] = True
        with self.assertRaises(A.AdmissionSchemaError):
            A.verify_bundle(self.bundle(), self.h.checkpoint_policy, trust, 250)

        profile = copy.deepcopy(self.h.profile_match)
        profile["product_authority"] = True
        with self.assertRaises(A.AdmissionSchemaError):
            A.prepare_checkpoint(
                self.h.lineage_state,
                profile,
                self.h.checkpoint_policy,
                self.h.trust,
                deployment_evidence_digest=T.h("5"),
                checkpoint_epoch=1,
                previous_checkpoint_digest=None,
                issued_at_micros=210,
                expires_at_micros=900,
                checkpoint_nonce=T.h("b"),
            )

    def test_unknown_durable_store_or_entry_fields_fail_closed(self):
        bundle = self.bundle()
        store = self.root / "store.json"
        self.assertEqual(
            A.admit(store, bundle, self.h.checkpoint_policy, self.h.trust, 250),
            "Added",
        )
        value = json.loads(store.read_text())
        value["ambient_authority"] = True
        store.write_text(json.dumps(value), encoding="utf-8")
        with self.assertRaises(A.AdmissionSchemaError):
            A.emit_head(store, bundle, self.h.checkpoint_policy, self.h.trust, 260)

        # Restore clean store, then smuggle into the history entry.
        store.unlink()
        A.admit(store, bundle, self.h.checkpoint_policy, self.h.trust, 250)
        value = json.loads(store.read_text())
        value["accepted_checkpoints"][0]["authority_override"] = True
        store.write_text(json.dumps(value), encoding="utf-8")
        with self.assertRaises(A.AdmissionSchemaError):
            A.emit_head(store, bundle, self.h.checkpoint_policy, self.h.trust, 260)

    def test_tampered_historical_signed_bundle_is_detected_after_restart(self):
        bundle = self.bundle()
        store = self.root / "store.json"
        A.admit(store, bundle, self.h.checkpoint_policy, self.h.trust, 250)
        value = json.loads(store.read_text())
        value["accepted_checkpoints"][0]["bundle"]["verified_roles"].append("clinical")
        store.write_text(json.dumps(value), encoding="utf-8")
        with self.assertRaises((A.AdmissionSchemaError, C.CheckpointError)):
            A.emit_head(store, bundle, self.h.checkpoint_policy, self.h.trust, 260)

    def test_tampered_historical_signed_statement_is_detected_after_restart(self):
        bundle = self.bundle()
        store = self.root / "store.json"
        A.admit(store, bundle, self.h.checkpoint_policy, self.h.trust, 250)
        value = json.loads(store.read_text())
        value["accepted_checkpoints"][0]["bundle"]["statement"][
            "latest_ledger_time_micros"
        ] += 1
        store.write_text(json.dumps(value), encoding="utf-8")
        with self.assertRaises((A.AdmissionSchemaError, C.CheckpointError)):
            A.emit_head(store, bundle, self.h.checkpoint_policy, self.h.trust, 260)


if __name__ == "__main__":
    unittest.main(verbosity=2)
