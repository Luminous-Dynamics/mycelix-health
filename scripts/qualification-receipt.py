#!/usr/bin/env python3
"""Prepare, sign, assemble, and independently verify governed qualification receipts."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import struct
import subprocess
import tempfile
from typing import Any, NoReturn

RECEIPT_DOMAIN = b"MYCELIX-HEALTH-QUALIFICATION-RECEIPT-V1\0"
ATTESTATION_DOMAIN = b"MYCELIX-HEALTH-QUALIFICATION-ATTESTATION-V1\0"
ED25519_SPKI_PREFIX = bytes.fromhex("302a300506032b6570032100")

RECEIPT_KIND = {
    "ReferenceQualification": 1,
    "DeploymentQualification": 2,
    "MigrationResultQualification": 3,
    "CompositionCommitmentQualification": 4,
    "DurablePrepareQualification": 5,
    "DurableActivationQualification": 6,
    "AuditCheckpointQualification": 7,
    "MonitoringAssessmentQualification": 8,
    "P0ClosureQualification": 9,
}
OUTCOME = {"Qualified": 1, "Failed": 2, "Indeterminate": 3, "Superseded": 4}
ADMISSION_SCOPE = {
    "Reference": 1,
    "Deployment": 2,
    "Cutover": 3,
    "Audit": 4,
    "Monitoring": 5,
    "ProductActivation": 6,
}
ALLOWED_ROLES = {"clinical", "privacy", "compliance", "safety"}


class QualificationError(ValueError):
    pass


def fail(message: str) -> NoReturn:
    raise QualificationError(message)


def load_json(path: pathlib.Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        fail(f"expected JSON object: {path}")
    return value


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def object_digest(value: dict[str, Any]) -> str:
    return sha256_bytes(canonical_json_bytes(value))


def hex32(value: Any, label: str) -> bytes:
    if not isinstance(value, str) or len(value) != 64:
        fail(f"{label} must be 32-byte lowercase hexadecimal")
    try:
        raw = bytes.fromhex(value)
    except ValueError as error:
        raise QualificationError(f"{label} is not hexadecimal") from error
    if len(raw) != 32 or raw == bytes(32) or value.lower() != value:
        fail(f"{label} must be nonzero 32-byte lowercase hexadecimal")
    return raw


def enum_value(table: dict[str, int], value: Any, label: str) -> int:
    if not isinstance(value, str) or value not in table:
        fail(f"unsupported {label}: {value!r}")
    return table[value]


def u16(value: Any, label: str) -> bytes:
    if not isinstance(value, int) or isinstance(value, bool) or not 0 <= value <= 0xFFFF:
        fail(f"{label} must fit u16")
    return struct.pack(">H", value)


def u64(value: Any, label: str) -> bytes:
    if not isinstance(value, int) or isinstance(value, bool) or not 0 <= value <= 0xFFFFFFFFFFFFFFFF:
        fail(f"{label} must fit u64")
    return struct.pack(">Q", value)


def i64(value: Any, label: str) -> bytes:
    if not isinstance(value, int) or isinstance(value, bool) or not -(1 << 63) <= value < (1 << 63):
        fail(f"{label} must fit i64")
    return struct.pack(">q", value)


def optional_digest(value: Any, label: str) -> bytes:
    if value is None:
        return b"\x00"
    return b"\x01" + hex32(value, label)


def qualification_transcript(statement: dict[str, Any]) -> bytes:
    """Reproduce #210 QualificationStatement::transcript_bytes() exactly."""
    if statement.get("schema_version") != 1:
        fail("qualification statement schema_version must be 1")
    kind = enum_value(RECEIPT_KIND, statement.get("receipt_kind"), "receipt kind")
    outcome = enum_value(OUTCOME, statement.get("outcome"), "outcome")
    scope = enum_value(ADMISSION_SCOPE, statement.get("admission_scope"), "admission scope")
    contract_version = statement.get("contract_version")
    if not isinstance(contract_version, int) or contract_version <= 0:
        fail("contract_version must be positive")
    issued = statement.get("issued_at_micros")
    expires = statement.get("expires_at_micros")
    if not isinstance(issued, int) or isinstance(issued, bool) or not isinstance(expires, int) or isinstance(expires, bool) or expires <= issued:
        fail("qualification statement time window is invalid")
    sequence = statement.get("sequence")
    if not isinstance(sequence, int) or isinstance(sequence, bool) or sequence <= 0:
        fail("sequence must be positive")
    predecessor = statement.get("previous_receipt")
    if (sequence == 1) != (predecessor is None):
        fail("qualification predecessor shape is invalid")

    out = bytearray(RECEIPT_DOMAIN)
    out += u16(1, "schema version")
    out.append(kind)
    out += hex32(statement.get("contract_digest"), "contract_digest")
    out += u16(contract_version, "contract_version")
    out += hex32(statement.get("qualification_lineage"), "qualification_lineage")
    out += hex32(statement.get("subject_digest"), "subject_digest")
    out += hex32(statement.get("dependency_set_digest"), "dependency_set_digest")
    out += optional_digest(statement.get("deployment_evidence_digest"), "deployment_evidence_digest")
    out += optional_digest(statement.get("context_digest"), "context_digest")
    out.append(outcome)
    out += i64(issued, "issued_at_micros")
    out += i64(expires, "expires_at_micros")
    out += hex32(statement.get("nonce"), "nonce")
    out += u64(sequence, "sequence")
    out += optional_digest(predecessor, "previous_receipt")
    out += hex32(statement.get("governance_policy_digest"), "governance_policy_digest")
    out += hex32(statement.get("trust_store_digest"), "trust_store_digest")
    out.append(scope)
    out += hex32(statement.get("claim_profile_digest"), "claim_profile_digest")
    return bytes(out)


def statement_digest(statement: dict[str, Any]) -> str:
    return sha256_bytes(qualification_transcript(statement))


def lp_text(value: Any, label: str) -> bytes:
    if not isinstance(value, str) or not value or len(value.encode("utf-8")) > 256:
        fail(f"invalid {label}")
    raw = value.encode("utf-8")
    return struct.pack(">I", len(raw)) + raw


def attestation_transcript(statement: dict[str, Any], approver_id: str, signer_key_id: str, attested_at_micros: int) -> bytes:
    transcript = qualification_transcript(statement)
    digest = hashlib.sha256(transcript).digest()
    return b"".join(
        [
            ATTESTATION_DOMAIN,
            digest,
            lp_text(approver_id, "approver_id"),
            lp_text(signer_key_id, "signer_key_id"),
            i64(attested_at_micros, "attested_at_micros"),
            transcript,
        ]
    )


def openssl(*args: str, input_bytes: bytes | None = None) -> bytes:
    result = subprocess.run(["openssl", *args], input=input_bytes, capture_output=True, check=False)
    if result.returncode != 0:
        fail((result.stderr or result.stdout).decode("utf-8", "replace").strip() or "OpenSSL operation failed")
    return result.stdout


def sign_bytes(private_key: pathlib.Path, payload: bytes) -> bytes:
    with tempfile.TemporaryDirectory(prefix="qualification-sign-") as raw:
        root = pathlib.Path(raw)
        message = root / "message.bin"
        signature = root / "signature.bin"
        message.write_bytes(payload)
        result = subprocess.run(
            ["openssl", "pkeyutl", "-sign", "-rawin", "-inkey", str(private_key), "-in", str(message), "-out", str(signature)],
            capture_output=True,
            check=False,
        )
        if result.returncode != 0:
            fail(result.stderr.decode("utf-8", "replace").strip() or "OpenSSL signing failed")
        value = signature.read_bytes()
    if len(value) != 64:
        fail("Ed25519 signature must contain exactly 64 bytes")
    return value


def verify_ed25519(public_key_hex: str, payload: bytes, signature_hex: str) -> None:
    public = hex32(public_key_hex, "Ed25519 public key")
    if not isinstance(signature_hex, str):
        fail("Ed25519 signature must be hexadecimal")
    try:
        signature = bytes.fromhex(signature_hex)
    except ValueError as error:
        raise QualificationError("Ed25519 signature is not hexadecimal") from error
    if len(signature) != 64:
        fail("Ed25519 signature must contain exactly 64 bytes")
    with tempfile.TemporaryDirectory(prefix="qualification-verify-") as raw:
        root = pathlib.Path(raw)
        key = root / "public.der"
        message = root / "message.bin"
        sig = root / "signature.bin"
        key.write_bytes(ED25519_SPKI_PREFIX + public)
        message.write_bytes(payload)
        sig.write_bytes(signature)
        result = subprocess.run(
            [
                "openssl", "pkeyutl", "-verify", "-rawin", "-pubin", "-keyform", "DER",
                "-inkey", str(key), "-in", str(message), "-sigfile", str(sig),
            ],
            capture_output=True,
            check=False,
        )
        if result.returncode != 0:
            fail("qualification attestation signature verification failed")


def public_raw_hex(public_key: pathlib.Path) -> str:
    der = openssl("pkey", "-pubin", "-in", str(public_key), "-outform", "DER")
    if not der.startswith(ED25519_SPKI_PREFIX) or len(der) != len(ED25519_SPKI_PREFIX) + 32:
        fail("public key is not canonical Ed25519")
    return der[-32:].hex()


def validate_policy(policy: dict[str, Any]) -> dict[str, Any]:
    if policy.get("schema_version") != 1 or policy.get("policy_id") != "mycelix-health-qualification-governance-v1":
        fail("qualification governance policy identity is invalid")
    if not isinstance(policy.get("trust_store_id"), str) or not policy["trust_store_id"]:
        fail("qualification governance policy lacks trust_store_id")
    rules = policy.get("rules")
    if not isinstance(rules, list) or not rules:
        fail("qualification governance policy requires rules")
    seen: set[str] = set()
    for rule in rules:
        if not isinstance(rule, dict):
            fail("qualification governance rule must be an object")
        kind = rule.get("receipt_kind")
        if kind not in RECEIPT_KIND or kind in seen:
            fail("qualification governance receipt kind is unknown or duplicated")
        seen.add(kind)
        scopes = rule.get("allowed_scopes")
        if not isinstance(scopes, list) or not scopes or len(scopes) != len(set(scopes)) or any(scope not in ADMISSION_SCOPE for scope in scopes):
            fail(f"invalid allowed scopes for {kind}")
        minimum = rule.get("minimum_approvers")
        organizations = rule.get("minimum_organizations")
        if not isinstance(minimum, int) or minimum < 1 or not isinstance(organizations, int) or organizations < 1 or organizations > minimum:
            fail(f"invalid approval threshold for {kind}")
        roles = rule.get("required_roles")
        if not isinstance(roles, list) or not roles or len(roles) != len(set(roles)) or not set(roles) <= ALLOWED_ROLES:
            fail(f"invalid required roles for {kind}")
        for field in ("max_statement_lifetime_micros", "max_attestation_age_micros"):
            if not isinstance(rule.get(field), int) or rule[field] <= 0:
                fail(f"invalid {field} for {kind}")
        required = rule.get("deployment_evidence_required")
        if not isinstance(required, bool):
            fail(f"deployment_evidence_required must be boolean for {kind}")
    return policy


def validate_trust_store(trust: dict[str, Any], policy: dict[str, Any]) -> dict[str, Any]:
    if trust.get("schema_version") != 1 or trust.get("trust_store_id") != policy["trust_store_id"]:
        fail("qualification trust-store identity mismatch")
    records = trust.get("approvers")
    if not isinstance(records, list):
        fail("qualification trust store lacks approvers")
    approvers: set[str] = set()
    keys: set[str] = set()
    for record in records:
        if not isinstance(record, dict):
            fail("qualification approver record must be object")
        approver = record.get("approver_id")
        key_id = record.get("signer_key_id")
        if not isinstance(approver, str) or not approver or approver in approvers:
            fail("qualification approver IDs must be unique/nonempty")
        if not isinstance(key_id, str) or not key_id or key_id in keys:
            fail("qualification signer-key IDs must be unique/nonempty")
        approvers.add(approver)
        keys.add(key_id)
        hex32(record.get("ed25519_public_key"), f"public key for {approver}")
        if not isinstance(record.get("organization"), str) or not record["organization"]:
            fail(f"missing organization for {approver}")
        roles = record.get("roles")
        if not isinstance(roles, list) or not roles or len(roles) != len(set(roles)) or not set(roles) <= ALLOWED_ROLES:
            fail(f"invalid roles for {approver}")
        if record.get("status") not in {"active", "revoked"}:
            fail(f"invalid status for {approver}")
        start = record.get("valid_from_micros")
        end = record.get("valid_until_micros")
        if not isinstance(start, int) or isinstance(start, bool):
            fail(f"invalid valid_from_micros for {approver}")
        if end is not None and (not isinstance(end, int) or isinstance(end, bool) or end <= start):
            fail(f"invalid valid_until_micros for {approver}")
        for field in ("revoked_at_micros", "compromised_at_micros"):
            value = record.get(field)
            if value is not None and (not isinstance(value, int) or isinstance(value, bool)):
                fail(f"invalid {field} for {approver}")
    return trust


def policy_rule(policy: dict[str, Any], kind: str) -> dict[str, Any]:
    matches = [rule for rule in policy["rules"] if rule["receipt_kind"] == kind]
    if len(matches) != 1:
        fail(f"qualification policy lacks exact rule for {kind}")
    return matches[0]


def approver_eligible(record: dict[str, Any], at_micros: int) -> bool:
    if record["status"] != "active" or at_micros < record["valid_from_micros"]:
        return False
    if record.get("valid_until_micros") is not None and at_micros >= record["valid_until_micros"]:
        return False
    if record.get("revoked_at_micros") is not None and at_micros >= record["revoked_at_micros"]:
        return False
    if record.get("compromised_at_micros") is not None and at_micros >= record["compromised_at_micros"]:
        return False
    return True


def validate_statement_against_governance(statement: dict[str, Any], policy: dict[str, Any], trust: dict[str, Any], verification_time: int) -> dict[str, Any]:
    transcript = qualification_transcript(statement)
    rule = policy_rule(policy, statement["receipt_kind"])
    if statement["admission_scope"] not in rule["allowed_scopes"]:
        fail("qualification admission scope is not authorized by policy")
    if rule["deployment_evidence_required"] and statement.get("deployment_evidence_digest") is None:
        fail("qualification policy requires deployment evidence")
    if statement.get("governance_policy_digest") != object_digest(policy):
        fail("qualification statement governance-policy digest mismatch")
    if statement.get("trust_store_digest") != object_digest(trust):
        fail("qualification statement trust-store digest mismatch")
    issued = statement["issued_at_micros"]
    expires = statement["expires_at_micros"]
    if expires - issued > rule["max_statement_lifetime_micros"]:
        fail("qualification statement lifetime exceeds policy")
    if verification_time < issued or verification_time >= expires:
        fail("qualification statement is not current at verification time")
    if statement["outcome"] != "Qualified":
        fail("qualification statement outcome is not Qualified")
    if len(transcript) == 0:
        fail("qualification transcript unexpectedly empty")
    return rule


def sign_statement(statement: dict[str, Any], private_key: pathlib.Path, approver_id: str, signer_key_id: str, attested_at_micros: int) -> dict[str, Any]:
    payload = attestation_transcript(statement, approver_id, signer_key_id, attested_at_micros)
    signature = sign_bytes(private_key, payload)
    return {
        "schema_version": 1,
        "attestation_kind": "mycelix-health-qualification-ed25519-v1",
        "approver_id": approver_id,
        "signer_key_id": signer_key_id,
        "receipt_digest_sha256": statement_digest(statement),
        "attested_at_micros": attested_at_micros,
        "signature_ed25519": signature.hex(),
    }


def build_bundle(statement: dict[str, Any], signatures: list[dict[str, Any]], policy: dict[str, Any], trust: dict[str, Any], verification_time: int) -> dict[str, Any]:
    validate_policy(policy)
    validate_trust_store(trust, policy)
    rule = validate_statement_against_governance(statement, policy, trust, verification_time)
    records = {record["approver_id"]: record for record in trust["approvers"]}
    receipt_digest = statement_digest(statement)
    seen_approvers: set[str] = set()
    seen_keys: set[str] = set()
    organizations: set[str] = set()
    roles: set[str] = set()
    approvals: list[dict[str, Any]] = []

    for item in sorted(signatures, key=lambda value: str(value.get("approver_id", ""))):
        if not isinstance(item, dict) or item.get("schema_version") != 1 or item.get("attestation_kind") != "mycelix-health-qualification-ed25519-v1":
            fail("invalid qualification attestation schema/kind")
        approver = item.get("approver_id")
        key_id = item.get("signer_key_id")
        if not isinstance(approver, str) or approver in seen_approvers:
            fail("qualification attestations require distinct approvers")
        if not isinstance(key_id, str) or key_id in seen_keys:
            fail("qualification attestations require distinct signer keys")
        seen_approvers.add(approver)
        seen_keys.add(key_id)
        if item.get("receipt_digest_sha256") != receipt_digest:
            fail("qualification attestation targets another receipt")
        attested = item.get("attested_at_micros")
        if not isinstance(attested, int) or isinstance(attested, bool):
            fail("qualification attestation time is invalid")
        if attested < statement["issued_at_micros"] or attested >= statement["expires_at_micros"]:
            fail("qualification attestation is outside statement window")
        if attested > verification_time:
            fail("qualification attestation is in the future")
        if verification_time - attested > rule["max_attestation_age_micros"]:
            fail("qualification attestation is stale")
        record = records.get(approver)
        if not isinstance(record, dict) or record["signer_key_id"] != key_id:
            fail(f"unknown qualification approver/key: {approver}")
        if not approver_eligible(record, attested):
            fail(f"qualification approver was not eligible at attestation time: {approver}")
        payload = attestation_transcript(statement, approver, key_id, attested)
        verify_ed25519(record["ed25519_public_key"], payload, item.get("signature_ed25519"))
        organizations.add(record["organization"])
        roles.update(record["roles"])
        approvals.append(
            {
                "approver_id": approver,
                "signer_key_id": key_id,
                "organization": record["organization"],
                "roles": sorted(record["roles"]),
                "attested_at_micros": attested,
                "signature_ed25519": item["signature_ed25519"],
            }
        )

    if len(approvals) < rule["minimum_approvers"]:
        fail("qualification quorum has too few approvers")
    if len(organizations) < rule["minimum_organizations"]:
        fail("qualification quorum lacks organization diversity")
    if not set(rule["required_roles"]) <= roles:
        fail("qualification quorum lacks required roles")

    identity = {
        "statement": statement,
        "receipt_digest_sha256": receipt_digest,
        "policy_digest_sha256": object_digest(policy),
        "trust_store_digest_sha256": object_digest(trust),
        "approvals": approvals,
        "verified_roles": sorted(roles),
        "verified_organizations": sorted(organizations),
        "verification_time_micros": verification_time,
    }
    digest = sha256_bytes(canonical_json_bytes(identity))
    return {
        "schema_version": 1,
        "report_kind": "mycelix-health-qualification-bundle-v1",
        "bundle_id": f"health-qualification-bundle-{digest[:24]}",
        **identity,
        "bundle_digest_sha256": digest,
        "claims": {
            "canonical_statement_verified": True,
            "ed25519_attestations_verified": True,
            "governance_quorum_verified": True,
            "statement_current_at_verification": True,
            "underlying_theorem_truth_not_established": True,
        },
    }


def verify_bundle(bundle: dict[str, Any], policy: dict[str, Any], trust: dict[str, Any], verification_time: int) -> dict[str, Any]:
    if bundle.get("schema_version") != 1 or bundle.get("report_kind") != "mycelix-health-qualification-bundle-v1":
        fail("qualification bundle schema/kind is invalid")
    signatures = [
        {
            "schema_version": 1,
            "attestation_kind": "mycelix-health-qualification-ed25519-v1",
            "approver_id": approval.get("approver_id"),
            "signer_key_id": approval.get("signer_key_id"),
            "receipt_digest_sha256": bundle.get("receipt_digest_sha256"),
            "attested_at_micros": approval.get("attested_at_micros"),
            "signature_ed25519": approval.get("signature_ed25519"),
        }
        for approval in bundle.get("approvals", [])
        if isinstance(approval, dict)
    ]
    rebuilt = build_bundle(bundle.get("statement", {}), signatures, policy, trust, verification_time)
    if bundle != rebuilt:
        fail("qualification bundle does not match independent reconstruction")
    return bundle


def write_create_only(path: pathlib.Path, value: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(value)
    except Exception:
        path.unlink(missing_ok=True)
        raise


def write_json_create_only(path: pathlib.Path, value: dict[str, Any]) -> None:
    write_create_only(path, json.dumps(value, sort_keys=True, indent=2).encode("utf-8") + b"\n")


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser()
    subs = value.add_subparsers(dest="command", required=True)

    prepare = subs.add_parser("prepare")
    prepare.add_argument("statement", type=pathlib.Path)
    prepare.add_argument("--output", type=pathlib.Path, required=True)

    sign = subs.add_parser("sign")
    sign.add_argument("statement", type=pathlib.Path)
    sign.add_argument("--private-key", type=pathlib.Path, required=True)
    sign.add_argument("--approver-id", required=True)
    sign.add_argument("--signer-key-id", required=True)
    sign.add_argument("--attested-at-micros", type=int, required=True)
    sign.add_argument("--output", type=pathlib.Path, required=True)

    assemble = subs.add_parser("assemble")
    assemble.add_argument("statement", type=pathlib.Path)
    assemble.add_argument("--signature", type=pathlib.Path, action="append", required=True)
    assemble.add_argument("--policy", type=pathlib.Path, required=True)
    assemble.add_argument("--trust-store", type=pathlib.Path, required=True)
    assemble.add_argument("--verification-time-micros", type=int, required=True)
    assemble.add_argument("--output", type=pathlib.Path, required=True)

    verify = subs.add_parser("verify")
    verify.add_argument("bundle", type=pathlib.Path)
    verify.add_argument("--policy", type=pathlib.Path, required=True)
    verify.add_argument("--trust-store", type=pathlib.Path, required=True)
    verify.add_argument("--verification-time-micros", type=int, required=True)
    return value


def main() -> int:
    args = parser().parse_args()
    if args.command == "prepare":
        statement = load_json(args.statement)
        transcript = qualification_transcript(statement)
        write_create_only(args.output, transcript)
        print(sha256_bytes(transcript))
    elif args.command == "sign":
        statement = load_json(args.statement)
        value = sign_statement(statement, args.private_key, args.approver_id, args.signer_key_id, args.attested_at_micros)
        write_json_create_only(args.output, value)
        print(value["receipt_digest_sha256"])
    elif args.command == "assemble":
        statement = load_json(args.statement)
        signatures = [load_json(path) for path in args.signature]
        policy = load_json(args.policy)
        trust = load_json(args.trust_store)
        value = build_bundle(statement, signatures, policy, trust, args.verification_time_micros)
        write_json_create_only(args.output, value)
        print(value["bundle_id"])
    else:
        bundle = load_json(args.bundle)
        policy = load_json(args.policy)
        trust = load_json(args.trust_store)
        value = verify_bundle(bundle, policy, trust, args.verification_time_micros)
        print(value["bundle_id"])
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (QualificationError, OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        print(f"qualification receipt failed: {error}")
        raise SystemExit(1)
