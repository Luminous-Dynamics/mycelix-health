#!/usr/bin/env python3
"""Governed durable current-head checkpoints for qualification lineages."""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import pathlib
import struct
import tempfile
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]


def _load_module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


Q = _load_module("qualification_receipt", ROOT / "scripts/qualification-receipt.py")
P = _load_module("qualification_receipt_profile", ROOT / "scripts/qualification-receipt-profile.py")

CHECKPOINT_DOMAIN = b"MYCELIX-HEALTH-QUALIFICATION-LINEAGE-CHECKPOINT-V1\x00"
CHECKPOINT_ATTESTATION_DOMAIN = b"MYCELIX-HEALTH-QUALIFICATION-LINEAGE-CHECKPOINT-ATTESTATION-V1\x00"
CHECKPOINT_KIND = "qualification-lineage-checkpoint"
CHECKPOINT_POLICY_ID = "mycelix-health-qualification-lineage-checkpoint-governance-v1"
HEAD_REPORT_KIND = "mycelix-health-verified-qualification-head-checkpoint-v1"
STORE_KIND = "mycelix-health-qualification-lineage-checkpoint-store-v1"
STATUS_VALUES = {"Active", "Superseded", "Abandoned"}
DISPOSITION_VALUES = {"Supersede": "Superseded", "Abandon": "Abandoned"}


class CheckpointError(ValueError):
    pass


def fail(message: str) -> NoReturn:
    raise CheckpointError(message)


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def hex32(value: Any, label: str) -> str:
    Q.hex32(value, label)
    return value


def nonempty_text(value: Any, label: str, maximum: int = 256) -> str:
    if not isinstance(value, str) or not value or len(value.encode("utf-8")) > maximum:
        fail(f"invalid {label}")
    return value


def int_value(value: Any, label: str, minimum: int | None = None) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        fail(f"{label} must be integer")
    if minimum is not None and value < minimum:
        fail(f"{label} must be >= {minimum}")
    return value


def binding_identity(binding: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(binding, dict):
        fail("lineage_binding must be object")
    contract_version = int_value(binding.get("contract_version"), "contract_version", 1)
    scope = binding.get("admission_scope")
    if scope not in Q.ADMISSION_SCOPE:
        fail("unsupported lineage admission_scope")
    context = binding.get("context_digest")
    if context is not None:
        hex32(context, "context_digest")
    return {
        "contract_digest": hex32(binding.get("contract_digest"), "contract_digest"),
        "contract_version": contract_version,
        "qualification_lineage": hex32(binding.get("qualification_lineage"), "qualification_lineage"),
        "subject_digest": hex32(binding.get("subject_digest"), "subject_digest"),
        "context_digest": context,
        "admission_scope": scope,
        "claim_profile_digest": hex32(binding.get("claim_profile_digest"), "claim_profile_digest"),
    }


def binding_digest(binding: dict[str, Any]) -> str:
    return sha256_bytes(
        b"MYCELIX-HEALTH-QUALIFICATION-LINEAGE-BINDING-V1\x00"
        + canonical_json_bytes(binding_identity(binding))
    )


def validate_lineage_state(state: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(state, dict) or state.get("schema_version") != 1:
        fail("lineage state schema_version must be 1")
    binding = binding_identity(state.get("lineage_binding"))
    receipts = state.get("receipts")
    dispositions = state.get("dispositions")
    if not isinstance(receipts, list) or not receipts:
        fail("lineage state requires receipts")
    if not isinstance(dispositions, list):
        fail("lineage state dispositions must be list")
    latest_time = int_value(state.get("latest_ledger_time_micros"), "latest_ledger_time_micros")

    parsed: list[dict[str, Any]] = []
    digest_map: dict[str, dict[str, Any]] = {}
    nonces: set[str] = set()
    for raw in receipts:
        if not isinstance(raw, dict):
            fail("receipt state record must be object")
        digest = hex32(raw.get("receipt_digest_sha256"), "receipt_digest_sha256")
        if digest in digest_map:
            fail("duplicate receipt digest in lineage state")
        seq = int_value(raw.get("sequence"), "receipt sequence", 1)
        nonce = hex32(raw.get("nonce"), "receipt nonce")
        if nonce in nonces:
            fail("receipt nonce replay in lineage state")
        nonces.add(nonce)
        prev = raw.get("previous_receipt")
        if prev is not None:
            hex32(prev, "previous_receipt")
        if (seq == 1) != (prev is None):
            fail("receipt predecessor shape is invalid")
        record = {
            "receipt_digest_sha256": digest,
            "sequence": seq,
            "previous_receipt": prev,
            "nonce": nonce,
        }
        parsed.append(record)
        digest_map[digest] = record

    parsed.sort(key=lambda item: (item["sequence"], item["receipt_digest_sha256"]))
    for record in parsed:
        if record["sequence"] > 1:
            predecessor = digest_map.get(record["previous_receipt"])
            if predecessor is None:
                fail("receipt predecessor is missing from durable state")
            if predecessor["sequence"] + 1 != record["sequence"]:
                fail("receipt predecessor sequence is inconsistent")

    statuses = {item["receipt_digest_sha256"]: "Active" for item in parsed}
    disposition_records: list[dict[str, Any]] = []
    expected_epoch = 1
    for raw in sorted(
        dispositions,
        key=lambda item: item.get("disposition_epoch", 0) if isinstance(item, dict) else -1,
    ):
        if not isinstance(raw, dict):
            fail("disposition state record must be object")
        epoch = int_value(raw.get("disposition_epoch"), "disposition_epoch", 1)
        if epoch != expected_epoch:
            fail("disposition epochs must be contiguous from 1")
        expected_epoch += 1
        target = hex32(raw.get("target_receipt_digest_sha256"), "disposition target")
        if target not in statuses:
            fail("disposition targets unknown receipt")
        disposition = raw.get("disposition")
        if disposition not in DISPOSITION_VALUES:
            fail("unsupported disposition")
        if statuses[target] != "Active":
            fail("receipt disposition conflicts with existing state")
        statuses[target] = DISPOSITION_VALUES[disposition]
        disposition_records.append(
            {
                "target_receipt_digest_sha256": target,
                "disposition": disposition,
                "governance_evidence_digest": hex32(
                    raw.get("governance_evidence_digest"), "governance_evidence_digest"
                ),
                "disposition_epoch": epoch,
            }
        )

    children: dict[str, list[str]] = {item["receipt_digest_sha256"]: [] for item in parsed}
    roots: list[str] = []
    for record in parsed:
        digest = record["receipt_digest_sha256"]
        prev = record["previous_receipt"]
        if prev is None:
            roots.append(digest)
        else:
            children[prev].append(digest)
    forked = len(roots) != 1 or any(len(values) > 1 for values in children.values())
    leaves = [digest for digest, values in children.items() if not values]
    if len(leaves) != 1:
        forked = True

    visited: set[str] = set()
    if len(roots) == 1 and not forked:
        cursor = roots[0]
        while True:
            if cursor in visited:
                fail("cycle in lineage receipt state")
            visited.add(cursor)
            next_children = children[cursor]
            if not next_children:
                break
            cursor = next_children[0]
        if len(visited) != len(parsed):
            forked = True

    head_digest = None if forked or len(leaves) != 1 else leaves[0]
    head_record = None if head_digest is None else digest_map[head_digest]
    head_status = None if head_digest is None else statuses[head_digest]
    lineage_state = "Forked" if forked else ("ActiveHead" if head_status == "Active" else "InactiveHead")

    receipt_state = [
        {**record, "status": statuses[record["receipt_digest_sha256"]]} for record in parsed
    ]
    nonce_material = [
        {
            "receipt_digest_sha256": item["receipt_digest_sha256"],
            "sequence": item["sequence"],
            "nonce": item["nonce"],
        }
        for item in receipt_state
    ]
    state_identity = {
        "lineage_binding": binding,
        "receipts": receipt_state,
        "dispositions": disposition_records,
        "latest_ledger_time_micros": latest_time,
    }
    return {
        "lineage_binding": binding,
        "lineage_binding_digest_sha256": binding_digest(binding),
        "receipts": receipt_state,
        "dispositions": disposition_records,
        "latest_ledger_time_micros": latest_time,
        "lineage_state": lineage_state,
        "head_receipt_digest_sha256": head_digest,
        "head_sequence": None if head_record is None else head_record["sequence"],
        "head_predecessor_receipt_digest_sha256": None
        if head_record is None
        else head_record["previous_receipt"],
        "head_status": head_status,
        "lineage_state_commitment_sha256": sha256_bytes(
            b"MYCELIX-HEALTH-QUALIFICATION-LINEAGE-STATE-V1\x00"
            + canonical_json_bytes(state_identity)
        ),
        "nonce_state_commitment_sha256": sha256_bytes(
            b"MYCELIX-HEALTH-QUALIFICATION-NONCE-STATE-V1\x00"
            + canonical_json_bytes(nonce_material)
        ),
        "disposition_state_commitment_sha256": sha256_bytes(
            b"MYCELIX-HEALTH-QUALIFICATION-DISPOSITION-STATE-V1\x00"
            + canonical_json_bytes(disposition_records)
        ),
        "latest_disposition_epoch": len(disposition_records),
    }


def validate_checkpoint_policy(policy: dict[str, Any]) -> dict[str, Any]:
    if (
        not isinstance(policy, dict)
        or policy.get("schema_version") != 1
        or policy.get("policy_id") != CHECKPOINT_POLICY_ID
    ):
        fail("checkpoint policy identity is invalid")
    nonempty_text(policy.get("trust_store_id"), "trust_store_id")
    minimum = int_value(policy.get("minimum_approvers"), "minimum_approvers", 1)
    organizations = int_value(policy.get("minimum_organizations"), "minimum_organizations", 1)
    if organizations > minimum:
        fail("minimum_organizations cannot exceed minimum_approvers")
    roles = policy.get("required_roles")
    if (
        not isinstance(roles, list)
        or not roles
        or len(roles) != len(set(roles))
        or not set(roles) <= Q.ALLOWED_ROLES
    ):
        fail("checkpoint policy required_roles are invalid")
    for field in ("max_statement_lifetime_micros", "max_attestation_age_micros"):
        int_value(policy.get(field), field, 1)
    if not isinstance(policy.get("deployment_evidence_required"), bool):
        fail("deployment_evidence_required must be boolean")
    if policy.get("required_profile_seam") != "Patient208CompositionCommitment":
        fail("checkpoint policy must target Patient208CompositionCommitment in v1")
    return policy


def profile_match_commitment(report: dict[str, Any]) -> str:
    if (
        not isinstance(report, dict)
        or report.get("schema_version") != 1
        or report.get("report_kind") != "mycelix-health-qualification-profile-match-v1"
    ):
        fail("profile-match report identity is invalid")
    claims = report.get("claims")
    if not isinstance(claims, dict):
        fail("profile-match report claims missing")
    required = {
        "governed_bundle_independently_verified": True,
        "exact_adapter_profile_fields_match": True,
        "domain_separated_profile_match_commitment": True,
        "receipt_current_under_profile_age": True,
        "current_lineage_head_not_established": True,
        "product_seam_authority_not_established": True,
    }
    for key, value in required.items():
        if claims.get(key) is not value:
            fail(f"profile-match report claim drift: {key}")
    commitment = report.get("profile_match_digest_sha256")
    hex32(commitment, "profile_match_digest_sha256")
    identity = {
        key: report.get(key)
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
    rebuilt = Q.sha256_bytes(P.profile_match_transcript(identity))
    if rebuilt != commitment:
        fail("profile-match commitment does not independently reconstruct")
    return commitment


def build_checkpoint_statement(
    lineage_state: dict[str, Any],
    profile_match: dict[str, Any],
    policy: dict[str, Any],
    trust_store: dict[str, Any],
    deployment_evidence_digest: str | None,
    checkpoint_epoch: int,
    previous_checkpoint_digest: str | None,
    issued_at_micros: int,
    expires_at_micros: int,
    checkpoint_nonce: str,
) -> dict[str, Any]:
    state = validate_lineage_state(lineage_state)
    validate_checkpoint_policy(policy)
    Q.validate_trust_store(trust_store, policy)
    profile_commitment = profile_match_commitment(profile_match)
    if (
        state["lineage_state"] == "ActiveHead"
        and profile_match.get("receipt_digest_sha256") != state["head_receipt_digest_sha256"]
    ):
        fail("profile match does not target the exact active lineage head")
    if profile_match.get("qualification_lineage") != state["lineage_binding"]["qualification_lineage"]:
        fail("profile match lineage differs from checkpoint lineage")
    if policy["deployment_evidence_required"] and deployment_evidence_digest is None:
        fail("checkpoint policy requires deployment evidence")
    if deployment_evidence_digest is not None:
        hex32(deployment_evidence_digest, "deployment_evidence_digest")
    checkpoint_epoch = int_value(checkpoint_epoch, "checkpoint_epoch", 1)
    if (checkpoint_epoch == 1) != (previous_checkpoint_digest is None):
        fail("checkpoint predecessor shape is invalid")
    if previous_checkpoint_digest is not None:
        hex32(previous_checkpoint_digest, "previous_checkpoint_digest")
    issued_at_micros = int_value(issued_at_micros, "issued_at_micros")
    expires_at_micros = int_value(expires_at_micros, "expires_at_micros")
    if expires_at_micros <= issued_at_micros:
        fail("checkpoint time window is invalid")
    if expires_at_micros - issued_at_micros > policy["max_statement_lifetime_micros"]:
        fail("checkpoint lifetime exceeds policy")
    hex32(checkpoint_nonce, "checkpoint_nonce")

    return {
        "schema_version": 1,
        "checkpoint_kind": CHECKPOINT_KIND,
        "lineage_binding": state["lineage_binding"],
        "lineage_binding_digest_sha256": state["lineage_binding_digest_sha256"],
        "lineage_state": state["lineage_state"],
        "current_head_receipt_digest_sha256": state["head_receipt_digest_sha256"],
        "current_head_sequence": state["head_sequence"],
        "current_head_status": state["head_status"],
        "current_head_predecessor_receipt_digest_sha256": state[
            "head_predecessor_receipt_digest_sha256"
        ],
        "lineage_state_commitment_sha256": state["lineage_state_commitment_sha256"],
        "nonce_state_commitment_sha256": state["nonce_state_commitment_sha256"],
        "disposition_state_commitment_sha256": state[
            "disposition_state_commitment_sha256"
        ],
        "latest_disposition_epoch": state["latest_disposition_epoch"],
        "latest_ledger_time_micros": state["latest_ledger_time_micros"],
        "profile_match_commitment_sha256": profile_commitment,
        "governance_policy_digest_sha256": Q.object_digest(policy),
        "trust_store_digest_sha256": Q.object_digest(trust_store),
        "deployment_evidence_digest": deployment_evidence_digest,
        "checkpoint_epoch": checkpoint_epoch,
        "previous_checkpoint_digest_sha256": previous_checkpoint_digest,
        "issued_at_micros": issued_at_micros,
        "expires_at_micros": expires_at_micros,
        "checkpoint_nonce": checkpoint_nonce,
    }


def checkpoint_transcript(statement: dict[str, Any]) -> bytes:
    if (
        not isinstance(statement, dict)
        or statement.get("schema_version") != 1
        or statement.get("checkpoint_kind") != CHECKPOINT_KIND
    ):
        fail("checkpoint statement schema/kind invalid")
    binding = binding_identity(statement.get("lineage_binding"))
    if binding_digest(binding) != statement.get("lineage_binding_digest_sha256"):
        fail("checkpoint lineage binding digest mismatch")
    state = statement.get("lineage_state")
    if state not in {"ActiveHead", "InactiveHead", "Forked"}:
        fail("unsupported checkpoint lineage_state")
    head = statement.get("current_head_receipt_digest_sha256")
    head_seq = statement.get("current_head_sequence")
    head_status = statement.get("current_head_status")
    head_prev = statement.get("current_head_predecessor_receipt_digest_sha256")
    if state == "Forked":
        if any(value is not None for value in (head, head_seq, head_status, head_prev)):
            fail("forked checkpoint cannot claim a current head")
    else:
        hex32(head, "current_head_receipt_digest_sha256")
        int_value(head_seq, "current_head_sequence", 1)
        if head_status not in STATUS_VALUES:
            fail("invalid current_head_status")
        if head_seq == 1:
            if head_prev is not None:
                fail("genesis head cannot have predecessor")
        else:
            hex32(head_prev, "current_head_predecessor_receipt_digest_sha256")
    for field in (
        "lineage_state_commitment_sha256",
        "nonce_state_commitment_sha256",
        "disposition_state_commitment_sha256",
        "profile_match_commitment_sha256",
        "governance_policy_digest_sha256",
        "trust_store_digest_sha256",
        "checkpoint_nonce",
    ):
        hex32(statement.get(field), field)
    deployment = statement.get("deployment_evidence_digest")
    if deployment is not None:
        hex32(deployment, "deployment_evidence_digest")
    int_value(statement.get("latest_disposition_epoch"), "latest_disposition_epoch", 0)
    int_value(statement.get("latest_ledger_time_micros"), "latest_ledger_time_micros")
    epoch = int_value(statement.get("checkpoint_epoch"), "checkpoint_epoch", 1)
    previous = statement.get("previous_checkpoint_digest_sha256")
    if (epoch == 1) != (previous is None):
        fail("checkpoint predecessor shape invalid")
    if previous is not None:
        hex32(previous, "previous_checkpoint_digest_sha256")
    issued = int_value(statement.get("issued_at_micros"), "issued_at_micros")
    expires = int_value(statement.get("expires_at_micros"), "expires_at_micros")
    if expires <= issued:
        fail("checkpoint statement time window invalid")
    identity = {
        key: statement.get(key)
        for key in (
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
        )
    }
    return CHECKPOINT_DOMAIN + canonical_json_bytes(identity)


def checkpoint_digest(statement: dict[str, Any]) -> str:
    return sha256_bytes(checkpoint_transcript(statement))


def _lp_text(value: str, label: str) -> bytes:
    raw = nonempty_text(value, label).encode("utf-8")
    return struct.pack(">I", len(raw)) + raw


def checkpoint_attestation_transcript(
    statement: dict[str, Any], approver_id: str, signer_key_id: str, attested_at_micros: int
) -> bytes:
    transcript = checkpoint_transcript(statement)
    digest = hashlib.sha256(transcript).digest()
    return b"".join(
        [
            CHECKPOINT_ATTESTATION_DOMAIN,
            digest,
            _lp_text(approver_id, "approver_id"),
            _lp_text(signer_key_id, "signer_key_id"),
            Q.i64(attested_at_micros, "attested_at_micros"),
            transcript,
        ]
    )


def sign_checkpoint(
    statement: dict[str, Any],
    private_key: pathlib.Path,
    approver_id: str,
    signer_key_id: str,
    attested_at_micros: int,
) -> dict[str, Any]:
    payload = checkpoint_attestation_transcript(
        statement, approver_id, signer_key_id, attested_at_micros
    )
    signature = Q.sign_bytes(private_key, payload)
    return {
        "approver_id": approver_id,
        "signer_key_id": signer_key_id,
        "attested_at_micros": attested_at_micros,
        "checkpoint_digest_sha256": checkpoint_digest(statement),
        "signature_ed25519": signature.hex(),
    }


def assemble_checkpoint_bundle(
    statement: dict[str, Any],
    attestations: list[dict[str, Any]],
    policy: dict[str, Any],
    trust_store: dict[str, Any],
    verification_time_micros: int,
) -> dict[str, Any]:
    validate_checkpoint_policy(policy)
    Q.validate_trust_store(trust_store, policy)
    digest = checkpoint_digest(statement)
    if statement["governance_policy_digest_sha256"] != Q.object_digest(policy):
        fail("checkpoint governance policy digest mismatch")
    if statement["trust_store_digest_sha256"] != Q.object_digest(trust_store):
        fail("checkpoint trust-store digest mismatch")
    if policy["deployment_evidence_required"] and statement.get("deployment_evidence_digest") is None:
        fail("checkpoint deployment evidence is required")
    issued = statement["issued_at_micros"]
    expires = statement["expires_at_micros"]
    if verification_time_micros < issued or verification_time_micros >= expires:
        fail("checkpoint is not current at verification time")
    if expires - issued > policy["max_statement_lifetime_micros"]:
        fail("checkpoint lifetime exceeds policy")
    if not attestations:
        fail("checkpoint requires attestations")

    records = {item["approver_id"]: item for item in trust_store["approvers"]}
    seen_approvers: set[str] = set()
    seen_keys: set[str] = set()
    organizations: set[str] = set()
    roles: set[str] = set()
    verified: list[dict[str, Any]] = []
    for attestation in sorted(attestations, key=lambda item: str(item.get("approver_id", ""))):
        if not isinstance(attestation, dict):
            fail("checkpoint attestation must be object")
        approver_id = nonempty_text(attestation.get("approver_id"), "approver_id")
        signer_key_id = nonempty_text(attestation.get("signer_key_id"), "signer_key_id")
        if approver_id in seen_approvers or signer_key_id in seen_keys:
            fail("checkpoint attestations require distinct approvers and signer keys")
        seen_approvers.add(approver_id)
        seen_keys.add(signer_key_id)
        if attestation.get("checkpoint_digest_sha256") != digest:
            fail("checkpoint attestation targets another checkpoint")
        attested_at = int_value(attestation.get("attested_at_micros"), "attested_at_micros")
        if attested_at < issued or attested_at >= expires:
            fail("checkpoint attestation outside statement window")
        if attested_at > verification_time_micros:
            fail("checkpoint attestation is in the future")
        if verification_time_micros - attested_at > policy["max_attestation_age_micros"]:
            fail("checkpoint attestation is stale")
        record = records.get(approver_id)
        if not isinstance(record, dict) or record.get("signer_key_id") != signer_key_id:
            fail("checkpoint approver/key is unknown")
        if not Q.approver_eligible(record, attested_at):
            fail("checkpoint approver is inactive/revoked/compromised")
        payload = checkpoint_attestation_transcript(
            statement, approver_id, signer_key_id, attested_at
        )
        Q.verify_ed25519(
            record["ed25519_public_key"], payload, attestation.get("signature_ed25519")
        )
        organizations.add(record["organization"])
        roles.update(record["roles"])
        verified.append(
            {
                "approver_id": approver_id,
                "signer_key_id": signer_key_id,
                "organization": record["organization"],
                "roles": sorted(record["roles"]),
                "attested_at_micros": attested_at,
                "signature_ed25519": attestation["signature_ed25519"],
            }
        )
    if len(verified) < policy["minimum_approvers"]:
        fail("checkpoint quorum has too few approvers")
    if len(organizations) < policy["minimum_organizations"]:
        fail("checkpoint quorum lacks organization diversity")
    if not set(policy["required_roles"]) <= roles:
        fail("checkpoint quorum lacks required roles")

    identity = {
        "statement": statement,
        "checkpoint_digest_sha256": digest,
        "governance_policy_digest_sha256": Q.object_digest(policy),
        "trust_store_digest_sha256": Q.object_digest(trust_store),
        "approvals": verified,
        "verified_roles": sorted(roles),
        "verified_organizations": sorted(organizations),
        "verified_at_micros": verification_time_micros,
    }
    bundle_digest = sha256_bytes(
        b"MYCELIX-HEALTH-QUALIFICATION-LINEAGE-CHECKPOINT-BUNDLE-V1\x00"
        + canonical_json_bytes(identity)
    )
    consumable = (
        statement["lineage_state"] == "ActiveHead"
        and statement["current_head_status"] == "Active"
    )
    return {
        "schema_version": 1,
        "report_kind": "mycelix-health-qualification-lineage-checkpoint-bundle-v1",
        "bundle_id": f"health-qualification-checkpoint-bundle-{bundle_digest[:24]}",
        **identity,
        "bundle_digest_sha256": bundle_digest,
        "claims": {
            "checkpoint_signatures_verified": True,
            "checkpoint_quorum_verified": True,
            "lineage_state_commitment_bound": True,
            "profile_match_commitment_bound": True,
            "current_head_authority_eligible": consumable,
            "durable_admission_not_established": True,
        },
    }


def verify_checkpoint_bundle(
    bundle: dict[str, Any],
    policy: dict[str, Any],
    trust_store: dict[str, Any],
    verification_time_micros: int,
) -> dict[str, Any]:
    if (
        not isinstance(bundle, dict)
        or bundle.get("schema_version") != 1
        or bundle.get("report_kind")
        != "mycelix-health-qualification-lineage-checkpoint-bundle-v1"
    ):
        fail("checkpoint bundle schema/kind invalid")
    original_verified_at = int_value(
        bundle.get("verified_at_micros"), "bundle verified_at_micros"
    )
    if verification_time_micros < original_verified_at:
        fail("checkpoint verification clock rollback")
    statement = bundle.get("statement", {})
    expires = int_value(statement.get("expires_at_micros"), "checkpoint expires_at_micros")
    if verification_time_micros >= expires:
        fail("checkpoint bundle is expired at current verification time")
    signatures = [
        {
            "approver_id": item.get("approver_id"),
            "signer_key_id": item.get("signer_key_id"),
            "attested_at_micros": item.get("attested_at_micros"),
            "checkpoint_digest_sha256": bundle.get("checkpoint_digest_sha256"),
            "signature_ed25519": item.get("signature_ed25519"),
        }
        for item in bundle.get("approvals", [])
        if isinstance(item, dict)
    ]
    rebuilt = assemble_checkpoint_bundle(
        statement, signatures, policy, trust_store, original_verified_at
    )
    if rebuilt != bundle:
        fail("checkpoint bundle does not match independent reconstruction")
    return bundle


def empty_store(binding: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "store_kind": STORE_KIND,
        "lineage_binding_digest_sha256": binding_digest(binding),
        "accepted_checkpoints": [],
        "latest_checkpoint_epoch": 0,
        "latest_checkpoint_digest_sha256": None,
        "latest_time_micros": None,
        "used_checkpoint_nonces": [],
        "fork_locked": False,
    }


def _load_store(path: pathlib.Path, binding: dict[str, Any]) -> dict[str, Any]:
    if not path.exists():
        return empty_store(binding)
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CheckpointError("durable checkpoint store is unreadable/invalid") from error
    if (
        not isinstance(value, dict)
        or value.get("schema_version") != 1
        or value.get("store_kind") != STORE_KIND
    ):
        fail("durable checkpoint store identity invalid")
    if value.get("lineage_binding_digest_sha256") != binding_digest(binding):
        fail("durable checkpoint store belongs to another lineage")
    entries = value.get("accepted_checkpoints")
    nonces = value.get("used_checkpoint_nonces")
    if not isinstance(entries, list) or not isinstance(nonces, list):
        fail("durable checkpoint store shape invalid")
    if len(nonces) != len(set(nonces)):
        fail("durable checkpoint store contains duplicate nonces")
    seen_digests: set[str] = set()
    expected_previous = None
    expected_epoch = 1
    derived_nonces: list[str] = []
    derived_fork_lock = False
    for entry in entries:
        if not isinstance(entry, dict):
            fail("durable checkpoint store entry must be object")
        digest = hex32(entry.get("checkpoint_digest_sha256"), "stored checkpoint digest")
        if digest in seen_digests:
            fail("durable checkpoint store contains duplicate checkpoint digest")
        seen_digests.add(digest)
        epoch = int_value(entry.get("checkpoint_epoch"), "stored checkpoint epoch", 1)
        if epoch != expected_epoch:
            fail("durable checkpoint store epoch chain is not contiguous")
        expected_epoch += 1
        if entry.get("previous_checkpoint_digest_sha256") != expected_previous:
            fail("durable checkpoint store predecessor chain is inconsistent")
        nonce = hex32(entry.get("checkpoint_nonce"), "stored checkpoint nonce")
        derived_nonces.append(nonce)
        expected_previous = digest
        state = entry.get("lineage_state")
        if state not in {"ActiveHead", "InactiveHead", "Forked"}:
            fail("durable checkpoint store contains invalid lineage state")
        derived_fork_lock = derived_fork_lock or state == "Forked"

        stored_bundle = entry.get("bundle")
        stored_policy = entry.get("policy_snapshot")
        stored_trust = entry.get("trust_store_snapshot")
        if (
            not isinstance(stored_bundle, dict)
            or not isinstance(stored_policy, dict)
            or not isinstance(stored_trust, dict)
        ):
            fail("durable checkpoint store is missing signed verification material")
        original_verified_at = int_value(
            stored_bundle.get("verified_at_micros"), "stored bundle verified_at_micros"
        )
        verify_checkpoint_bundle(
            stored_bundle, stored_policy, stored_trust, original_verified_at
        )
        signed_statement = stored_bundle["statement"]
        expected_summary = {
            "checkpoint_digest_sha256": stored_bundle["checkpoint_digest_sha256"],
            "checkpoint_epoch": signed_statement["checkpoint_epoch"],
            "previous_checkpoint_digest_sha256": signed_statement[
                "previous_checkpoint_digest_sha256"
            ],
            "checkpoint_nonce": signed_statement["checkpoint_nonce"],
            "lineage_state": signed_statement["lineage_state"],
            "current_head_receipt_digest_sha256": signed_statement[
                "current_head_receipt_digest_sha256"
            ],
            "current_head_sequence": signed_statement["current_head_sequence"],
            "profile_match_commitment_sha256": signed_statement[
                "profile_match_commitment_sha256"
            ],
            "lineage_state_commitment_sha256": signed_statement[
                "lineage_state_commitment_sha256"
            ],
            "issued_at_micros": signed_statement["issued_at_micros"],
            "expires_at_micros": signed_statement["expires_at_micros"],
            "bundle_digest_sha256": stored_bundle["bundle_digest_sha256"],
        }
        for field, expected in expected_summary.items():
            if entry.get(field) != expected:
                fail(f"durable checkpoint store summary drifted: {field}")
    if nonces != derived_nonces:
        fail("durable checkpoint store nonce index drifted")
    expected_latest_epoch = 0 if not entries else entries[-1]["checkpoint_epoch"]
    expected_latest_digest = None if not entries else entries[-1]["checkpoint_digest_sha256"]
    if value.get("latest_checkpoint_epoch") != expected_latest_epoch:
        fail("durable checkpoint store latest epoch drifted")
    if value.get("latest_checkpoint_digest_sha256") != expected_latest_digest:
        fail("durable checkpoint store latest digest drifted")
    if bool(value.get("fork_locked")) != derived_fork_lock:
        fail("durable checkpoint store fork-lock state drifted")
    latest_time = value.get("latest_time_micros")
    if entries and not isinstance(latest_time, int):
        fail("durable checkpoint store missing latest time")
    if not entries and latest_time is not None:
        fail("empty durable checkpoint store cannot carry latest time")
    return value


def _durable_replace_json(path: pathlib.Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, sort_keys=True, indent=2) + "\n"
    fd, raw = tempfile.mkstemp(prefix=f".{path.name}.", suffix=".tmp", dir=path.parent)
    tmp = pathlib.Path(raw)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(tmp, path)
        dirfd = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(dirfd)
        finally:
            os.close(dirfd)
    finally:
        if tmp.exists():
            tmp.unlink()


def admit_checkpoint(
    store_path: pathlib.Path,
    bundle: dict[str, Any],
    policy: dict[str, Any],
    trust_store: dict[str, Any],
    verification_time_micros: int,
) -> str:
    verify_checkpoint_bundle(bundle, policy, trust_store, verification_time_micros)
    statement = bundle["statement"]
    binding = statement["lineage_binding"]
    store = _load_store(store_path, binding)
    previous_time = store.get("latest_time_micros")
    if previous_time is not None and verification_time_micros < previous_time:
        fail("checkpoint store clock rollback")
    digest = bundle["checkpoint_digest_sha256"]
    for existing in store["accepted_checkpoints"]:
        if existing.get("checkpoint_digest_sha256") == digest:
            if (
                existing.get("checkpoint_epoch") == statement["checkpoint_epoch"]
                and existing.get("checkpoint_nonce") == statement["checkpoint_nonce"]
            ):
                if verification_time_micros > previous_time:
                    replay_store = dict(store)
                    replay_store["latest_time_micros"] = verification_time_micros
                    _durable_replace_json(store_path, replay_store)
                return "IdempotentReplay"
            fail("conflicting checkpoint replay")
    if statement["checkpoint_nonce"] in set(store["used_checkpoint_nonces"]):
        fail("checkpoint nonce replay")
    expected_epoch = store["latest_checkpoint_epoch"] + 1
    if statement["checkpoint_epoch"] != expected_epoch:
        fail("checkpoint epoch is stale or gapped")
    expected_previous = store["latest_checkpoint_digest_sha256"]
    if statement["previous_checkpoint_digest_sha256"] != expected_previous:
        fail("checkpoint predecessor mismatch")
    if store.get("fork_locked") and statement["lineage_state"] == "ActiveHead":
        fail("unresolved forked lineage cannot regain active head authority in v1")
    record = {
        "checkpoint_digest_sha256": digest,
        "checkpoint_epoch": statement["checkpoint_epoch"],
        "previous_checkpoint_digest_sha256": statement["previous_checkpoint_digest_sha256"],
        "checkpoint_nonce": statement["checkpoint_nonce"],
        "lineage_state": statement["lineage_state"],
        "current_head_receipt_digest_sha256": statement["current_head_receipt_digest_sha256"],
        "current_head_sequence": statement["current_head_sequence"],
        "profile_match_commitment_sha256": statement["profile_match_commitment_sha256"],
        "lineage_state_commitment_sha256": statement["lineage_state_commitment_sha256"],
        "issued_at_micros": statement["issued_at_micros"],
        "expires_at_micros": statement["expires_at_micros"],
        "bundle_digest_sha256": bundle["bundle_digest_sha256"],
        "bundle": bundle,
        "policy_snapshot": policy,
        "trust_store_snapshot": trust_store,
    }
    next_store = dict(store)
    next_store["accepted_checkpoints"] = [*store["accepted_checkpoints"], record]
    next_store["latest_checkpoint_epoch"] = statement["checkpoint_epoch"]
    next_store["latest_checkpoint_digest_sha256"] = digest
    next_store["latest_time_micros"] = verification_time_micros
    next_store["used_checkpoint_nonces"] = [
        *store["used_checkpoint_nonces"],
        statement["checkpoint_nonce"],
    ]
    next_store["fork_locked"] = bool(
        store.get("fork_locked") or statement["lineage_state"] == "Forked"
    )
    _durable_replace_json(store_path, next_store)
    return "Added"


def emit_verified_head(
    store_path: pathlib.Path,
    bundle: dict[str, Any],
    policy: dict[str, Any],
    trust_store: dict[str, Any],
    now_micros: int,
) -> dict[str, Any]:
    verify_checkpoint_bundle(bundle, policy, trust_store, now_micros)
    statement = bundle["statement"]
    store = _load_store(store_path, statement["lineage_binding"])
    previous_time = store.get("latest_time_micros")
    if previous_time is not None and now_micros < previous_time:
        fail("checkpoint store clock rollback")
    if store["latest_checkpoint_digest_sha256"] != bundle["checkpoint_digest_sha256"]:
        fail("checkpoint is not the durable current checkpoint")
    if store.get("fork_locked"):
        fail("durable lineage is fork-locked")
    if statement["lineage_state"] != "ActiveHead" or statement["current_head_status"] != "Active":
        fail("checkpoint does not establish an active current head")
    if now_micros < statement["issued_at_micros"] or now_micros >= statement["expires_at_micros"]:
        fail("checkpoint is not current")
    record = store["accepted_checkpoints"][-1]
    if record["profile_match_commitment_sha256"] != statement["profile_match_commitment_sha256"]:
        fail("durable checkpoint/profile-match commitment mismatch")
    if previous_time is None or now_micros > previous_time:
        observed_store = dict(store)
        observed_store["latest_time_micros"] = now_micros
        _durable_replace_json(store_path, observed_store)
    identity = {
        "lineage_binding": statement["lineage_binding"],
        "lineage_binding_digest_sha256": statement["lineage_binding_digest_sha256"],
        "head_receipt_digest_sha256": statement["current_head_receipt_digest_sha256"],
        "head_sequence": statement["current_head_sequence"],
        "checkpoint_digest_sha256": bundle["checkpoint_digest_sha256"],
        "checkpoint_bundle_digest_sha256": bundle["bundle_digest_sha256"],
        "checkpoint_epoch": statement["checkpoint_epoch"],
        "profile_match_commitment_sha256": statement["profile_match_commitment_sha256"],
        "lineage_state_commitment_sha256": statement["lineage_state_commitment_sha256"],
        "governance_policy_digest_sha256": statement["governance_policy_digest_sha256"],
        "trust_store_digest_sha256": statement["trust_store_digest_sha256"],
        "deployment_evidence_digest": statement["deployment_evidence_digest"],
        "verified_at_micros": now_micros,
        "expires_at_micros": statement["expires_at_micros"],
    }
    digest = sha256_bytes(
        b"MYCELIX-HEALTH-VERIFIED-QUALIFICATION-HEAD-V1\x00" + canonical_json_bytes(identity)
    )
    return {
        "schema_version": 1,
        "report_kind": HEAD_REPORT_KIND,
        "verified_head_id": f"health-qualification-head-{digest[:24]}",
        **identity,
        "verified_head_digest_sha256": digest,
        "claims": {
            "checkpoint_bundle_independently_verified": True,
            "checkpoint_is_durable_current_checkpoint": True,
            "lineage_is_not_forked": True,
            "head_receipt_is_active": True,
            "profile_match_commitment_bound": True,
            "product_seam_conversion_not_performed": True,
        },
    }


def write_json_create_only(path: pathlib.Path, value: dict[str, Any]) -> None:
    Q.write_json_create_only(path, value)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    prepare = sub.add_parser("prepare")
    prepare.add_argument("lineage_state", type=pathlib.Path)
    prepare.add_argument("profile_match", type=pathlib.Path)
    prepare.add_argument("--policy", type=pathlib.Path, required=True)
    prepare.add_argument("--trust-store", type=pathlib.Path, required=True)
    prepare.add_argument("--deployment-evidence-digest")
    prepare.add_argument("--checkpoint-epoch", type=int, required=True)
    prepare.add_argument("--previous-checkpoint-digest")
    prepare.add_argument("--issued-at-micros", type=int, required=True)
    prepare.add_argument("--expires-at-micros", type=int, required=True)
    prepare.add_argument("--checkpoint-nonce", required=True)
    prepare.add_argument("--output", type=pathlib.Path, required=True)

    sign = sub.add_parser("sign")
    sign.add_argument("statement", type=pathlib.Path)
    sign.add_argument("--private-key", type=pathlib.Path, required=True)
    sign.add_argument("--approver-id", required=True)
    sign.add_argument("--signer-key-id", required=True)
    sign.add_argument("--attested-at-micros", type=int, required=True)
    sign.add_argument("--output", type=pathlib.Path, required=True)

    assemble = sub.add_parser("assemble")
    assemble.add_argument("statement", type=pathlib.Path)
    assemble.add_argument("attestations", nargs="+", type=pathlib.Path)
    assemble.add_argument("--policy", type=pathlib.Path, required=True)
    assemble.add_argument("--trust-store", type=pathlib.Path, required=True)
    assemble.add_argument("--verification-time-micros", type=int, required=True)
    assemble.add_argument("--output", type=pathlib.Path, required=True)

    verify = sub.add_parser("verify")
    verify.add_argument("bundle", type=pathlib.Path)
    verify.add_argument("--policy", type=pathlib.Path, required=True)
    verify.add_argument("--trust-store", type=pathlib.Path, required=True)
    verify.add_argument("--verification-time-micros", type=int, required=True)

    admit = sub.add_parser("admit")
    admit.add_argument("bundle", type=pathlib.Path)
    admit.add_argument("--policy", type=pathlib.Path, required=True)
    admit.add_argument("--trust-store", type=pathlib.Path, required=True)
    admit.add_argument("--verification-time-micros", type=int, required=True)
    admit.add_argument("--store", type=pathlib.Path, required=True)

    head = sub.add_parser("emit-head")
    head.add_argument("bundle", type=pathlib.Path)
    head.add_argument("--policy", type=pathlib.Path, required=True)
    head.add_argument("--trust-store", type=pathlib.Path, required=True)
    head.add_argument("--now-micros", type=int, required=True)
    head.add_argument("--store", type=pathlib.Path, required=True)
    head.add_argument("--output", type=pathlib.Path, required=True)
    return parser


def main() -> int:
    args = _parser().parse_args()
    if args.command == "prepare":
        statement = build_checkpoint_statement(
            Q.load_json(args.lineage_state),
            Q.load_json(args.profile_match),
            Q.load_json(args.policy),
            Q.load_json(args.trust_store),
            args.deployment_evidence_digest,
            args.checkpoint_epoch,
            args.previous_checkpoint_digest,
            args.issued_at_micros,
            args.expires_at_micros,
            args.checkpoint_nonce,
        )
        write_json_create_only(args.output, statement)
        print(checkpoint_digest(statement))
    elif args.command == "sign":
        value = sign_checkpoint(
            Q.load_json(args.statement),
            args.private_key,
            args.approver_id,
            args.signer_key_id,
            args.attested_at_micros,
        )
        write_json_create_only(args.output, value)
        print(value["checkpoint_digest_sha256"])
    elif args.command == "assemble":
        statement = Q.load_json(args.statement)
        attestations = [Q.load_json(path) for path in args.attestations]
        bundle = assemble_checkpoint_bundle(
            statement,
            attestations,
            Q.load_json(args.policy),
            Q.load_json(args.trust_store),
            args.verification_time_micros,
        )
        write_json_create_only(args.output, bundle)
        print(bundle["bundle_id"])
    elif args.command == "verify":
        verify_checkpoint_bundle(
            Q.load_json(args.bundle),
            Q.load_json(args.policy),
            Q.load_json(args.trust_store),
            args.verification_time_micros,
        )
        print("OK")
    elif args.command == "admit":
        outcome = admit_checkpoint(
            args.store,
            Q.load_json(args.bundle),
            Q.load_json(args.policy),
            Q.load_json(args.trust_store),
            args.verification_time_micros,
        )
        print(outcome)
    elif args.command == "emit-head":
        value = emit_verified_head(
            args.store,
            Q.load_json(args.bundle),
            Q.load_json(args.policy),
            Q.load_json(args.trust_store),
            args.now_micros,
        )
        write_json_create_only(args.output, value)
        print(value["verified_head_id"])
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        CheckpointError,
        Q.QualificationError,
        OSError,
        ValueError,
        TypeError,
        KeyError,
        json.JSONDecodeError,
    ) as error:
        print(f"qualification lineage checkpoint failed: {error}")
        raise SystemExit(1)
