#!/usr/bin/env python3
"""Strict public admission facade for qualification-lineage checkpoints.

The underlying checkpoint module is the implementation engine. Product/runtime
adapters should call this module so authority-bearing JSON is exact-schema,
not merely a superset whose unknown fields happen to be ignored.
"""
from __future__ import annotations

import importlib.util
import json
import pathlib
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]


def _load(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


C = _load("qualification_lineage_checkpoint", ROOT / "scripts/qualification-lineage-checkpoint.py")


class AdmissionSchemaError(ValueError):
    pass


def fail(message: str) -> NoReturn:
    raise AdmissionSchemaError(message)


def exact_keys(value: Any, required: set[str], optional: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail(f"{label} must be an object")
    keys = set(value)
    missing = required - keys
    unknown = keys - required - optional
    if missing:
        fail(f"{label} missing fields: {sorted(missing)}")
    if unknown:
        fail(f"{label} contains unknown fields: {sorted(unknown)}")
    return value


BINDING_REQUIRED = {
    "contract_digest",
    "contract_version",
    "qualification_lineage",
    "subject_digest",
    "context_digest",
    "admission_scope",
    "claim_profile_digest",
}
RECEIPT_STATE_REQUIRED = {"receipt_digest_sha256", "sequence", "previous_receipt", "nonce"}
DISPOSITION_REQUIRED = {
    "target_receipt_digest_sha256",
    "disposition",
    "governance_evidence_digest",
    "disposition_epoch",
}
LINEAGE_STATE_REQUIRED = {
    "schema_version",
    "lineage_binding",
    "receipts",
    "dispositions",
    "latest_ledger_time_micros",
}
PROFILE_MATCH_REQUIRED = {
    "schema_version",
    "report_kind",
    "profile_match_id",
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
    "profile_match_digest_sha256",
    "claims",
}
PROFILE_MATCH_CLAIMS = {
    "governed_bundle_independently_verified",
    "exact_adapter_profile_fields_match",
    "domain_separated_profile_match_commitment",
    "receipt_current_under_profile_age",
    "current_lineage_head_not_established",
    "product_seam_authority_not_established",
}
CHECKPOINT_POLICY_REQUIRED = {
    "schema_version",
    "policy_id",
    "trust_store_id",
    "minimum_approvers",
    "minimum_organizations",
    "required_roles",
    "max_statement_lifetime_micros",
    "max_attestation_age_micros",
    "deployment_evidence_required",
    "required_profile_seam",
}
TRUST_REQUIRED = {"schema_version", "trust_store_id", "approvers"}
APPROVER_REQUIRED = {
    "approver_id",
    "signer_key_id",
    "ed25519_public_key",
    "organization",
    "roles",
    "status",
    "valid_from_micros",
    "valid_until_micros",
    "revoked_at_micros",
    "compromised_at_micros",
}
CHECKPOINT_STATEMENT_REQUIRED = {
    "schema_version",
    "checkpoint_kind",
    "lineage_binding",
    "lineage_binding_digest_sha256",
    "lineage_state",
    "current_head_receipt_digest_sha256",
    "current_head_sequence",
    "current_head_status",
    "current_head_predecessor_receipt_digest_sha256",
    "lineage_state_commitment_sha256",
    "nonce_state_commitment_sha256",
    "disposition_state_commitment_sha256",
    "latest_disposition_epoch",
    "latest_ledger_time_micros",
    "profile_match_commitment_sha256",
    "governance_policy_digest_sha256",
    "trust_store_digest_sha256",
    "deployment_evidence_digest",
    "checkpoint_epoch",
    "previous_checkpoint_digest_sha256",
    "issued_at_micros",
    "expires_at_micros",
    "checkpoint_nonce",
}
CHECKPOINT_APPROVAL_REQUIRED = {
    "approver_id",
    "signer_key_id",
    "organization",
    "roles",
    "attested_at_micros",
    "signature_ed25519",
}
CHECKPOINT_BUNDLE_REQUIRED = {
    "schema_version",
    "report_kind",
    "bundle_id",
    "statement",
    "checkpoint_digest_sha256",
    "governance_policy_digest_sha256",
    "trust_store_digest_sha256",
    "approvals",
    "verified_roles",
    "verified_organizations",
    "verified_at_micros",
    "bundle_digest_sha256",
    "claims",
}
CHECKPOINT_BUNDLE_CLAIMS = {
    "checkpoint_signatures_verified",
    "checkpoint_quorum_verified",
    "lineage_state_commitment_bound",
    "profile_match_commitment_bound",
    "current_head_authority_eligible",
    "durable_admission_not_established",
}
STORE_REQUIRED = {
    "schema_version",
    "store_kind",
    "lineage_binding_digest_sha256",
    "accepted_checkpoints",
    "latest_checkpoint_epoch",
    "latest_checkpoint_digest_sha256",
    "latest_time_micros",
    "used_checkpoint_nonces",
    "fork_locked",
}
STORE_ENTRY_REQUIRED = {
    "checkpoint_digest_sha256",
    "checkpoint_epoch",
    "previous_checkpoint_digest_sha256",
    "checkpoint_nonce",
    "lineage_state",
    "current_head_receipt_digest_sha256",
    "current_head_sequence",
    "profile_match_commitment_sha256",
    "lineage_state_commitment_sha256",
    "issued_at_micros",
    "expires_at_micros",
    "bundle_digest_sha256",
    "bundle",
    "policy_snapshot",
    "trust_store_snapshot",
}


def validate_binding(value: Any) -> dict[str, Any]:
    return exact_keys(value, BINDING_REQUIRED, set(), "lineage binding")


def validate_lineage_state_shape(value: Any) -> dict[str, Any]:
    state = exact_keys(value, LINEAGE_STATE_REQUIRED, set(), "lineage state")
    validate_binding(state["lineage_binding"])
    if not isinstance(state["receipts"], list):
        fail("lineage receipts must be a list")
    for item in state["receipts"]:
        exact_keys(item, RECEIPT_STATE_REQUIRED, set(), "receipt state record")
    if not isinstance(state["dispositions"], list):
        fail("lineage dispositions must be a list")
    for item in state["dispositions"]:
        exact_keys(item, DISPOSITION_REQUIRED, set(), "disposition record")
    return state


def validate_profile_match_shape(value: Any) -> dict[str, Any]:
    report = exact_keys(value, PROFILE_MATCH_REQUIRED, set(), "profile-match report")
    exact_keys(report["claims"], PROFILE_MATCH_CLAIMS, set(), "profile-match claims")
    return report


def validate_policy_shape(value: Any) -> dict[str, Any]:
    return exact_keys(value, CHECKPOINT_POLICY_REQUIRED, set(), "checkpoint policy")


def validate_trust_store_shape(value: Any) -> dict[str, Any]:
    trust = exact_keys(value, TRUST_REQUIRED, set(), "checkpoint trust store")
    if not isinstance(trust["approvers"], list):
        fail("checkpoint trust-store approvers must be list")
    for item in trust["approvers"]:
        exact_keys(item, APPROVER_REQUIRED, set(), "checkpoint approver")
    return trust


def validate_statement_shape(value: Any) -> dict[str, Any]:
    statement = exact_keys(value, CHECKPOINT_STATEMENT_REQUIRED, set(), "checkpoint statement")
    validate_binding(statement["lineage_binding"])
    return statement


def validate_bundle_shape(value: Any) -> dict[str, Any]:
    bundle = exact_keys(value, CHECKPOINT_BUNDLE_REQUIRED, set(), "checkpoint bundle")
    validate_statement_shape(bundle["statement"])
    if not isinstance(bundle["approvals"], list):
        fail("checkpoint approvals must be list")
    for item in bundle["approvals"]:
        exact_keys(item, CHECKPOINT_APPROVAL_REQUIRED, set(), "checkpoint approval")
    exact_keys(bundle["claims"], CHECKPOINT_BUNDLE_CLAIMS, set(), "checkpoint bundle claims")
    return bundle


def validate_store_shape(value: Any) -> dict[str, Any]:
    store = exact_keys(value, STORE_REQUIRED, set(), "checkpoint durable store")
    if not isinstance(store["accepted_checkpoints"], list):
        fail("accepted_checkpoints must be list")
    for entry in store["accepted_checkpoints"]:
        exact_keys(entry, STORE_ENTRY_REQUIRED, set(), "stored checkpoint entry")
        validate_bundle_shape(entry["bundle"])
        validate_policy_shape(entry["policy_snapshot"])
        validate_trust_store_shape(entry["trust_store_snapshot"])
    return store


def prepare_checkpoint(
    lineage_state: dict[str, Any],
    profile_match: dict[str, Any],
    policy: dict[str, Any],
    trust_store: dict[str, Any],
    **kwargs: Any,
) -> dict[str, Any]:
    validate_lineage_state_shape(lineage_state)
    validate_profile_match_shape(profile_match)
    validate_policy_shape(policy)
    validate_trust_store_shape(trust_store)
    statement = C.build_checkpoint_statement(
        lineage_state, profile_match, policy, trust_store, **kwargs
    )
    validate_statement_shape(statement)
    return statement


def verify_bundle(
    bundle: dict[str, Any],
    policy: dict[str, Any],
    trust_store: dict[str, Any],
    now_micros: int,
) -> dict[str, Any]:
    validate_bundle_shape(bundle)
    validate_policy_shape(policy)
    validate_trust_store_shape(trust_store)
    return C.verify_checkpoint_bundle(bundle, policy, trust_store, now_micros)


def _validate_store_file(path: pathlib.Path) -> None:
    if not path.exists():
        return
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise AdmissionSchemaError("checkpoint durable store is unreadable") from error
    validate_store_shape(value)


def admit(
    store_path: pathlib.Path,
    bundle: dict[str, Any],
    policy: dict[str, Any],
    trust_store: dict[str, Any],
    now_micros: int,
) -> str:
    validate_bundle_shape(bundle)
    validate_policy_shape(policy)
    validate_trust_store_shape(trust_store)
    _validate_store_file(store_path)
    outcome = C.admit_checkpoint(store_path, bundle, policy, trust_store, now_micros)
    _validate_store_file(store_path)
    return outcome


def emit_head(
    store_path: pathlib.Path,
    bundle: dict[str, Any],
    policy: dict[str, Any],
    trust_store: dict[str, Any],
    now_micros: int,
) -> dict[str, Any]:
    validate_bundle_shape(bundle)
    validate_policy_shape(policy)
    validate_trust_store_shape(trust_store)
    _validate_store_file(store_path)
    head = C.emit_verified_head(store_path, bundle, policy, trust_store, now_micros)
    _validate_store_file(store_path)
    return head
