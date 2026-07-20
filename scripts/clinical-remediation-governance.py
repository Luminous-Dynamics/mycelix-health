#!/usr/bin/env python3
"""Shared deterministic contracts for remediation approvals and candidate lineage."""
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import pathlib
import re
import subprocess
from datetime import datetime, timezone
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]
POLICY = ROOT / "release/clinical-remediation-governance-policy.json"
TRUST_STORE = ROOT / "release/trusted-remediation-approvers.json"
HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")


def module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


D = module("health_promotion_diagnostics", ROOT / "scripts/clinical-promotion-diagnostics.py")
PLAN = module("health_promotion_remediation_plan", ROOT / "scripts/plan-clinical-promotion-remediation.py")


class GovernanceError(ValueError):
    pass


def fail(message: str) -> NoReturn:
    raise GovernanceError(message)


def load_json(path: pathlib.Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        fail(f"expected a JSON object: {path}")
    return value


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_create_only(path: pathlib.Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    fd = os.open(path, flags, 0o600)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(value, handle, indent=2, sort_keys=True)
            handle.write("\n")
    except Exception:
        path.unlink(missing_ok=True)
        raise


def require_hex(value: Any, size: int, label: str) -> str:
    pattern = HEX40 if size == 40 else HEX64
    if not isinstance(value, str) or not pattern.fullmatch(value):
        fail(f"{label} must be {size} lowercase hexadecimal characters")
    return value


def parse_utc(value: Any, label: str) -> datetime:
    if not isinstance(value, str) or not value.endswith("Z"):
        fail(f"{label} must be an RFC3339 UTC timestamp")
    try:
        parsed = datetime.fromisoformat(value[:-1] + "+00:00")
    except ValueError as error:
        raise GovernanceError(f"{label} is not a valid timestamp") from error
    if parsed.tzinfo is None:
        fail(f"{label} must include UTC timezone")
    return parsed.astimezone(timezone.utc)


def load_policy(path: pathlib.Path = POLICY) -> dict[str, Any]:
    value = load_json(path)
    if value.get("schema_version") != 1 or value.get("release_id") != "health-v1":
        fail("remediation governance policy identity drifted")
    if value.get("policy_id") != "mycelix-health-clinical-remediation-governance-v1":
        fail("unexpected remediation governance policy id")
    approval = value.get("approval")
    change = value.get("change_manifest")
    lineage = value.get("lineage")
    claims = value.get("claims")
    if not isinstance(approval, dict) or not isinstance(change, dict) or not isinstance(lineage, dict):
        fail("remediation governance policy sections are missing")
    if not isinstance(claims, dict) or not claims or any(item is not True for item in claims.values()):
        fail("remediation governance claims must all be true")
    roles = approval.get("allowed_roles")
    required_roles = approval.get("required_roles")
    if not isinstance(roles, list) or len(roles) != len(set(roles)) or not roles:
        fail("approval roles are invalid")
    if not isinstance(required_roles, list) or not set(required_roles) <= set(roles):
        fail("required approval roles are invalid")
    if approval.get("minimum_distinct_approvers", 0) < 2 or approval.get("minimum_distinct_organizations", 0) < 2:
        fail("remediation approval must require independent reviewers")
    if not isinstance(value.get("protected_paths"), list) or len(value["protected_paths"]) != len(set(value["protected_paths"])):
        fail("protected remediation paths are invalid")
    value["_policy_path"] = str(path.resolve())
    return value


def load_trust_store(path: pathlib.Path = TRUST_STORE, policy: dict[str, Any] | None = None) -> dict[str, Any]:
    policy = policy or load_policy()
    value = load_json(path)
    if value.get("schema_version") != 1 or value.get("release_id") != policy["release_id"]:
        fail("remediation approver trust store identity drifted")
    if value.get("trust_store_id") != policy["trust_store_id"]:
        fail("remediation approver trust store id drifted")
    approvers = value.get("approvers")
    if not isinstance(approvers, list):
        fail("remediation approver trust store lacks approvers")
    ids: set[str] = set()
    for item in approvers:
        if not isinstance(item, dict):
            fail("remediation approver record is not an object")
        approver_id = item.get("approver_id")
        if not isinstance(approver_id, str) or not approver_id or approver_id in ids:
            fail("remediation approver ids must be unique and nonempty")
        ids.add(approver_id)
        if item.get("status") not in {"active", "revoked"}:
            fail(f"invalid remediation approver status: {approver_id}")
        roles = item.get("roles")
        if not isinstance(roles, list) or not roles or not set(roles) <= set(policy["approval"]["allowed_roles"]):
            fail(f"invalid remediation approver roles: {approver_id}")
        if not isinstance(item.get("organization"), str) or not item["organization"]:
            fail(f"missing remediation approver organization: {approver_id}")
        require_hex(item.get("ed25519_public_key"), 64, f"public key for {approver_id}")
        parse_utc(item.get("valid_from_utc"), f"valid_from_utc for {approver_id}")
        if item.get("valid_until_utc") is not None:
            parse_utc(item["valid_until_utc"], f"valid_until_utc for {approver_id}")
        if item.get("revoked_at_utc") is not None:
            parse_utc(item["revoked_at_utc"], f"revoked_at_utc for {approver_id}")
    value["_trust_store_path"] = str(path.resolve())
    return value


def git(repo: pathlib.Path, *args: str, text: bool = True) -> str | bytes:
    result = subprocess.run(["git", "-C", str(repo), *args], check=False, capture_output=True, text=text)
    if result.returncode != 0:
        stderr = result.stderr.strip() if text else result.stderr.decode("utf-8", "replace").strip()
        fail(f"git command failed ({' '.join(args)}): {stderr}")
    return result.stdout.strip() if text else result.stdout


def resolve_revision(repo: pathlib.Path, revision: str, label: str) -> str:
    require_hex(revision, 40, label)
    resolved = str(git(repo, "rev-parse", f"{revision}^{{commit}}"))
    if resolved != revision:
        fail(f"{label} does not resolve exactly")
    return revision


def blob_sha256(repo: pathlib.Path, revision: str, path: str) -> str | None:
    exists = subprocess.run(
        ["git", "-C", str(repo), "cat-file", "-e", f"{revision}:{path}"],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    if exists.returncode != 0:
        return None
    raw = git(repo, "show", f"{revision}:{path}", text=False)
    assert isinstance(raw, bytes)
    return sha256_bytes(raw)


def path_allowed(path: str, policy: dict[str, Any]) -> bool:
    if not path or path.startswith("/") or ".." in pathlib.PurePosixPath(path).parts or "\\" in path:
        return False
    if len(path.encode("utf-8")) > policy["change_manifest"]["maximum_path_bytes"]:
        return False
    if path in policy["protected_paths"]:
        return False
    return path in policy["change_manifest"]["allowed_source_root_files"] or any(
        path.startswith(prefix) for prefix in policy["change_manifest"]["allowed_source_prefixes"]
    )


def build_change_manifest(
    repo: pathlib.Path,
    base_revision: str,
    target_revision: str,
    remediation_plan_path: pathlib.Path,
    policy: dict[str, Any],
) -> dict[str, Any]:
    resolve_revision(repo, base_revision, "base source revision")
    resolve_revision(repo, target_revision, "target source revision")
    plan = load_json(remediation_plan_path)
    remediation_policy = PLAN.load_policy()
    PLAN.verify_plan(plan, remediation_policy)
    if plan.get("source_revision") != base_revision:
        fail("remediation plan is stale for the selected base revision")
    output = str(git(repo, "diff", "--name-status", "--no-renames", base_revision, target_revision, "--"))
    changed: list[dict[str, Any]] = []
    if output:
        for line in output.splitlines():
            parts = line.split("\t")
            if len(parts) != 2 or parts[0] not in {"A", "M", "D", "T"}:
                fail(f"unsupported Git change record: {line}")
            status, path = parts
            if not path_allowed(path, policy):
                fail(f"changed path is outside the reviewed remediation boundary: {path}")
            changed.append(
                {
                    "path": path,
                    "status": status,
                    "before_sha256": blob_sha256(repo, base_revision, path),
                    "after_sha256": blob_sha256(repo, target_revision, path),
                }
            )
    changed.sort(key=lambda item: item["path"])
    if len(changed) > policy["change_manifest"]["maximum_changed_files"]:
        fail("remediation change manifest exceeds the file-count bound")
    action_types = sorted({item["action_type"] for item in plan.get("actions", [])})
    source_change_required = "repair_deterministic_tooling" in action_types
    if source_change_required and not changed:
        fail("engineering remediation requires a reviewed source change")
    if not source_change_required and changed:
        fail("the remediation plan does not authorize source changes")
    if changed and base_revision == target_revision:
        fail("changed remediation manifest cannot target the base revision")
    identity = {
        "release_id": policy["release_id"],
        "governance_policy_sha256": sha256_file(pathlib.Path(policy["_policy_path"])),
        "remediation_plan_id": plan["plan_id"],
        "remediation_plan_digest_sha256": plan["plan_digest_sha256"],
        "base_source_revision": base_revision,
        "target_source_revision": target_revision,
        "action_types": action_types,
        "changed_files": changed,
    }
    digest = sha256_bytes(canonical_bytes(identity))
    return {
        "schema_version": 1,
        "report_kind": "clinical-remediation-change-manifest",
        "manifest_id": f"health-remediation-change-{digest[:24]}",
        **identity,
        "change_manifest_digest_sha256": digest,
        "claims": {
            "changed_files_are_content_addressed": True,
            "protected_paths_are_unchanged": True,
            "manifest_executes_no_changes": True,
        },
    }


def verify_change_manifest(value: dict[str, Any], plan: dict[str, Any], policy: dict[str, Any]) -> dict[str, Any]:
    if value.get("schema_version") != 1 or value.get("report_kind") != "clinical-remediation-change-manifest":
        fail("remediation change manifest schema or kind is invalid")
    if value.get("release_id") != policy["release_id"]:
        fail("remediation change manifest release id drifted")
    require_hex(value.get("base_source_revision"), 40, "base source revision")
    require_hex(value.get("target_source_revision"), 40, "target source revision")
    if value.get("remediation_plan_id") != plan.get("plan_id") or value.get("remediation_plan_digest_sha256") != plan.get("plan_digest_sha256"):
        fail("remediation change manifest targets a different plan")
    if value.get("base_source_revision") != plan.get("source_revision"):
        fail("remediation change manifest uses a stale plan")
    if value.get("governance_policy_sha256") != sha256_file(pathlib.Path(policy["_policy_path"])):
        fail("remediation change manifest governance policy digest drifted")
    changed = value.get("changed_files")
    if not isinstance(changed, list) or len(changed) > policy["change_manifest"]["maximum_changed_files"]:
        fail("remediation change manifest file list is invalid")
    paths: list[str] = []
    for item in changed:
        if not isinstance(item, dict) or item.get("status") not in {"A", "M", "D", "T"}:
            fail("remediation change manifest contains an invalid change")
        path = item.get("path")
        if not isinstance(path, str) or not path_allowed(path, policy):
            fail("remediation change manifest contains a forbidden path")
        paths.append(path)
        for field in ("before_sha256", "after_sha256"):
            if item.get(field) is not None:
                require_hex(item[field], 64, f"{field} for {path}")
    if paths != sorted(paths) or len(paths) != len(set(paths)):
        fail("remediation change manifest paths must be unique and sorted")
    action_types = sorted({item["action_type"] for item in plan.get("actions", [])})
    if value.get("action_types") != action_types:
        fail("remediation change manifest action types drifted")
    if ("repair_deterministic_tooling" in action_types) != bool(changed):
        fail("remediation change manifest source-change classification drifted")
    identity = {
        "release_id": value.get("release_id"),
        "governance_policy_sha256": value.get("governance_policy_sha256"),
        "remediation_plan_id": value.get("remediation_plan_id"),
        "remediation_plan_digest_sha256": value.get("remediation_plan_digest_sha256"),
        "base_source_revision": value.get("base_source_revision"),
        "target_source_revision": value.get("target_source_revision"),
        "action_types": value.get("action_types"),
        "changed_files": changed,
    }
    digest = sha256_bytes(canonical_bytes(identity))
    if value.get("change_manifest_digest_sha256") != digest or value.get("manifest_id") != f"health-remediation-change-{digest[:24]}":
        fail("remediation change manifest identity mismatch")
    claims = value.get("claims")
    if not isinstance(claims, dict) or any(item is not True for item in claims.values()):
        fail("remediation change manifest claims are invalid")
    return value

APPROVAL_DOMAIN = b"MYCELIX-HEALTH-REMEDIATION-APPROVAL-V1\x00"
ED25519_SPKI_PREFIX = bytes.fromhex("302a300506032b6570032100")


def approval_identity(statement: dict[str, Any]) -> dict[str, Any]:
    return {
        "release_id": statement.get("release_id"),
        "governance_policy_sha256": statement.get("governance_policy_sha256"),
        "remediation_policy_sha256": statement.get("remediation_policy_sha256"),
        "remediation_plan_id": statement.get("remediation_plan_id"),
        "remediation_plan_digest_sha256": statement.get("remediation_plan_digest_sha256"),
        "change_manifest_id": statement.get("change_manifest_id"),
        "change_manifest_digest_sha256": statement.get("change_manifest_digest_sha256"),
        "base_source_revision": statement.get("base_source_revision"),
        "target_source_revision": statement.get("target_source_revision"),
        "issued_at_utc": statement.get("issued_at_utc"),
        "expires_at_utc": statement.get("expires_at_utc"),
        "nonce": statement.get("nonce"),
    }


def approval_signable_bytes(statement: dict[str, Any]) -> bytes:
    return APPROVAL_DOMAIN + canonical_bytes(approval_identity(statement))


def build_approval_statement(
    plan: dict[str, Any],
    manifest: dict[str, Any],
    issued_at_utc: str,
    expires_at_utc: str,
    nonce: str,
    policy: dict[str, Any],
) -> dict[str, Any]:
    remediation_policy = PLAN.load_policy()
    PLAN.verify_plan(plan, remediation_policy)
    verify_change_manifest(manifest, plan, policy)
    issued = parse_utc(issued_at_utc, "issued_at_utc")
    expires = parse_utc(expires_at_utc, "expires_at_utc")
    validity = int((expires - issued).total_seconds())
    if validity <= 0 or validity > policy["approval"]["maximum_validity_seconds"]:
        fail("remediation approval validity window is invalid")
    if not isinstance(nonce, str) or len(nonce.encode("utf-8")) < 16 or len(nonce.encode("utf-8")) > 128:
        fail("remediation approval nonce must contain 16 to 128 bytes")
    statement: dict[str, Any] = {
        "schema_version": 1,
        "approval_kind": "clinical-remediation-plan-approval",
        "release_id": policy["release_id"],
        "governance_policy_sha256": sha256_file(pathlib.Path(policy["_policy_path"])),
        "remediation_policy_sha256": manifest.get("governance_policy_sha256") and sha256_file(ROOT / "release/clinical-promotion-remediation-policy.json"),
        "remediation_plan_id": plan["plan_id"],
        "remediation_plan_digest_sha256": plan["plan_digest_sha256"],
        "change_manifest_id": manifest["manifest_id"],
        "change_manifest_digest_sha256": manifest["change_manifest_digest_sha256"],
        "base_source_revision": manifest["base_source_revision"],
        "target_source_revision": manifest["target_source_revision"],
        "issued_at_utc": issued_at_utc,
        "expires_at_utc": expires_at_utc,
        "nonce": nonce,
    }
    digest = sha256_bytes(approval_signable_bytes(statement))
    statement["approval_statement_id"] = f"health-remediation-approval-{digest[:24]}"
    statement["approval_statement_digest_sha256"] = digest
    return statement


def verify_approval_statement(
    statement: dict[str, Any],
    plan: dict[str, Any],
    manifest: dict[str, Any],
    policy: dict[str, Any],
) -> dict[str, Any]:
    if statement.get("schema_version") != 1 or statement.get("approval_kind") != "clinical-remediation-plan-approval":
        fail("remediation approval statement schema or kind is invalid")
    if statement.get("release_id") != policy["release_id"]:
        fail("remediation approval release id drifted")
    verify_change_manifest(manifest, plan, policy)
    expected = {
        "governance_policy_sha256": sha256_file(pathlib.Path(policy["_policy_path"])),
        "remediation_policy_sha256": sha256_file(ROOT / "release/clinical-promotion-remediation-policy.json"),
        "remediation_plan_id": plan.get("plan_id"),
        "remediation_plan_digest_sha256": plan.get("plan_digest_sha256"),
        "change_manifest_id": manifest.get("manifest_id"),
        "change_manifest_digest_sha256": manifest.get("change_manifest_digest_sha256"),
        "base_source_revision": manifest.get("base_source_revision"),
        "target_source_revision": manifest.get("target_source_revision"),
    }
    for field, value in expected.items():
        if statement.get(field) != value:
            fail(f"remediation approval statement {field} drifted")
    issued = parse_utc(statement.get("issued_at_utc"), "issued_at_utc")
    expires = parse_utc(statement.get("expires_at_utc"), "expires_at_utc")
    validity = int((expires - issued).total_seconds())
    if validity <= 0 or validity > policy["approval"]["maximum_validity_seconds"]:
        fail("remediation approval validity window is invalid")
    nonce = statement.get("nonce")
    if not isinstance(nonce, str) or not 16 <= len(nonce.encode("utf-8")) <= 128:
        fail("remediation approval nonce is invalid")
    digest = sha256_bytes(approval_signable_bytes(statement))
    if statement.get("approval_statement_digest_sha256") != digest:
        fail("remediation approval statement digest mismatch")
    if statement.get("approval_statement_id") != f"health-remediation-approval-{digest[:24]}":
        fail("remediation approval statement id mismatch")
    return statement


def openssl_run(*args: str, input_bytes: bytes | None = None) -> bytes:
    result = subprocess.run(["openssl", *args], input=input_bytes, check=False, capture_output=True)
    if result.returncode != 0:
        fail((result.stderr or result.stdout).decode("utf-8", "replace").strip() or "OpenSSL operation failed")
    return result.stdout


def sign_approval(statement: dict[str, Any], private_key: pathlib.Path, approver_id: str) -> dict[str, Any]:
    if not approver_id or len(approver_id.encode("utf-8")) > 256:
        fail("invalid remediation approver id")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="health-remediation-sign-") as raw:
        root = pathlib.Path(raw)
        message = root / "message.bin"
        signature = root / "signature.bin"
        message.write_bytes(approval_signable_bytes(statement))
        result = subprocess.run(
            ["openssl", "pkeyutl", "-sign", "-rawin", "-inkey", str(private_key), "-in", str(message), "-out", str(signature)],
            check=False,
            capture_output=True,
        )
        if result.returncode != 0:
            fail(result.stderr.decode("utf-8", "replace").strip() or "OpenSSL signing failed")
        raw_signature = signature.read_bytes()
    if len(raw_signature) != 64:
        fail("remediation approval signature must contain 64 bytes")
    return {
        "approver_id": approver_id,
        "approval_statement_id": statement["approval_statement_id"],
        "signature_ed25519": raw_signature.hex(),
    }


def verify_ed25519(raw_public_key_hex: str, payload: bytes, signature_hex: str) -> None:
    import tempfile

    require_hex(raw_public_key_hex, 64, "Ed25519 public key")
    require_hex(signature_hex, 128, "Ed25519 signature") if False else None
    try:
        signature = bytes.fromhex(signature_hex)
    except (TypeError, ValueError) as error:
        raise GovernanceError("Ed25519 signature is not valid hex") from error
    if len(signature) != 64:
        fail("Ed25519 signature must contain exactly 64 bytes")
    with tempfile.TemporaryDirectory(prefix="health-remediation-verify-") as raw:
        root = pathlib.Path(raw)
        public_der = root / "public.der"
        message = root / "message.bin"
        sig = root / "signature.bin"
        public_der.write_bytes(ED25519_SPKI_PREFIX + bytes.fromhex(raw_public_key_hex))
        message.write_bytes(payload)
        sig.write_bytes(signature)
        result = subprocess.run(
            [
                "openssl",
                "pkeyutl",
                "-verify",
                "-rawin",
                "-pubin",
                "-keyform",
                "DER",
                "-inkey",
                str(public_der),
                "-in",
                str(message),
                "-sigfile",
                str(sig),
            ],
            check=False,
            capture_output=True,
        )
        if result.returncode != 0:
            fail("remediation approval signature verification failed")


def approver_eligible(record: dict[str, Any], at: datetime) -> bool:
    if record.get("status") != "active":
        return False
    if at < parse_utc(record.get("valid_from_utc"), "approver valid_from_utc"):
        return False
    valid_until = record.get("valid_until_utc")
    if valid_until is not None and at >= parse_utc(valid_until, "approver valid_until_utc"):
        return False
    revoked_at = record.get("revoked_at_utc")
    if revoked_at is not None and at >= parse_utc(revoked_at, "approver revoked_at_utc"):
        return False
    return True


def build_approval_bundle(
    statement: dict[str, Any],
    signatures: list[dict[str, Any]],
    plan: dict[str, Any],
    manifest: dict[str, Any],
    policy: dict[str, Any],
    trust_store: dict[str, Any],
    verification_time_utc: str,
) -> dict[str, Any]:
    verify_approval_statement(statement, plan, manifest, policy)
    verification_time = parse_utc(verification_time_utc, "verification_time_utc")
    issued = parse_utc(statement["issued_at_utc"], "issued_at_utc")
    expires = parse_utc(statement["expires_at_utc"], "expires_at_utc")
    if verification_time < issued or verification_time >= expires:
        fail("remediation approval is not current at verification time")
    records = {item["approver_id"]: item for item in trust_store["approvers"]}
    approvals: list[dict[str, Any]] = []
    seen: set[str] = set()
    organizations: set[str] = set()
    roles: set[str] = set()
    for signature in sorted(signatures, key=lambda item: str(item.get("approver_id", ""))):
        approver_id = signature.get("approver_id")
        if not isinstance(approver_id, str) or approver_id in seen:
            fail("remediation approval signatures must use distinct approvers")
        seen.add(approver_id)
        if signature.get("approval_statement_id") != statement["approval_statement_id"]:
            fail("remediation approval signature targets a different statement")
        record = records.get(approver_id)
        if not isinstance(record, dict) or not approver_eligible(record, issued):
            fail(f"remediation approver was not eligible at issuance: {approver_id}")
        verify_ed25519(record["ed25519_public_key"], approval_signable_bytes(statement), signature.get("signature_ed25519"))
        organizations.add(record["organization"])
        roles.update(record["roles"])
        approvals.append(
            {
                "approver_id": approver_id,
                "organization": record["organization"],
                "roles": sorted(record["roles"]),
                "signature_ed25519": signature["signature_ed25519"],
            }
        )
    requirements = policy["approval"]
    if len(approvals) < requirements["minimum_distinct_approvers"]:
        fail("remediation approval quorum has too few approvers")
    if len(organizations) < requirements["minimum_distinct_organizations"]:
        fail("remediation approval quorum lacks organization diversity")
    if not set(requirements["required_roles"]) <= roles:
        fail("remediation approval quorum lacks required roles")
    identity = {
        "statement": statement,
        "approvals": approvals,
        "trust_store_sha256": sha256_file(pathlib.Path(trust_store["_trust_store_path"])),
        "verified_roles": sorted(roles),
        "verified_organizations": sorted(organizations),
    }
    digest = sha256_bytes(canonical_bytes(identity))
    return {
        "schema_version": 1,
        "report_kind": "clinical-remediation-approval-bundle",
        "bundle_id": f"health-remediation-approval-bundle-{digest[:24]}",
        **identity,
        "approval_bundle_digest_sha256": digest,
        "claims": {
            "approval_is_current": True,
            "approval_is_plan_and_manifest_specific": True,
            "approval_quorum_is_independent": True,
            "approval_does_not_promote_release": True,
        },
    }


def verify_approval_bundle(
    bundle: dict[str, Any],
    plan: dict[str, Any],
    manifest: dict[str, Any],
    policy: dict[str, Any],
    trust_store: dict[str, Any],
    verification_time_utc: str,
) -> dict[str, Any]:
    if bundle.get("schema_version") != 1 or bundle.get("report_kind") != "clinical-remediation-approval-bundle":
        fail("remediation approval bundle schema or kind is invalid")
    signatures = [
        {
            "approver_id": item.get("approver_id"),
            "approval_statement_id": bundle.get("statement", {}).get("approval_statement_id"),
            "signature_ed25519": item.get("signature_ed25519"),
        }
        for item in bundle.get("approvals", [])
        if isinstance(item, dict)
    ]
    rebuilt = build_approval_bundle(
        bundle.get("statement", {}), signatures, plan, manifest, policy, trust_store, verification_time_utc
    )
    if bundle != rebuilt:
        fail("remediation approval bundle does not match independent verification")
    return bundle

IMPORT = module("health_promotion_run_import_for_lineage", ROOT / "scripts/import-clinical-promotion-run.py")


def lineage_record_identity(record: dict[str, Any]) -> dict[str, Any]:
    return {key: record.get(key) for key in sorted(record) if key not in {"record_id", "record_digest_sha256"}}


def finalize_lineage_record(record: dict[str, Any]) -> dict[str, Any]:
    digest = sha256_bytes(canonical_bytes(lineage_record_identity(record)))
    record = dict(record)
    record["record_id"] = f"health-release-candidate-record-{digest[:24]}"
    record["record_digest_sha256"] = digest
    return record


def verify_lineage_record(record: dict[str, Any], policy: dict[str, Any], expected_depth: int, predecessor: str | None) -> dict[str, Any]:
    if record.get("schema_version") != 1 or record.get("release_id") != policy["release_id"]:
        fail("release-candidate lineage record identity drifted")
    if record.get("record_type") not in {"candidate-authorization", "rehearsal-observation"}:
        fail("release-candidate lineage record type is invalid")
    if record.get("state") not in policy["lineage"]["allowed_states"]:
        fail("release-candidate lineage state is invalid")
    if record.get("depth") != expected_depth or record.get("predecessor_record_digest_sha256") != predecessor:
        fail("release-candidate lineage predecessor or depth mismatch")
    require_hex(record.get("source_revision"), 40, "lineage source revision")
    if record.get("record_type") == "candidate-authorization":
        require_hex(record.get("base_source_revision"), 40, "lineage base source revision")
        if record.get("state") != "authorized-for-rehearsal":
            fail("candidate authorization has an invalid state")
        for field in (
            "remediation_plan_digest_sha256",
            "change_manifest_digest_sha256",
            "approval_bundle_digest_sha256",
        ):
            require_hex(record.get(field), 64, field)
    else:
        if record.get("state") not in {"promotion-eligible", "refused"}:
            fail("rehearsal observation has an invalid state")
        require_hex(record.get("runner_import_digest_sha256"), 64, "runner import digest")
        if not isinstance(record.get("run_id"), int) or record["run_id"] <= 0:
            fail("rehearsal observation run_id is invalid")
        if not isinstance(record.get("run_attempt"), int) or record["run_attempt"] <= 0:
            fail("rehearsal observation run_attempt is invalid")
        roots = record.get("root_causes")
        if not isinstance(roots, list):
            fail("rehearsal observation root causes are invalid")
        if (record["state"] == "promotion-eligible") != (not roots):
            fail("rehearsal observation state disagrees with root causes")
    digest = sha256_bytes(canonical_bytes(lineage_record_identity(record)))
    if record.get("record_digest_sha256") != digest or record.get("record_id") != f"health-release-candidate-record-{digest[:24]}":
        fail("release-candidate lineage record digest mismatch")
    return record


def finalize_lineage(records: list[dict[str, Any]], policy: dict[str, Any]) -> dict[str, Any]:
    if not records or len(records) > policy["lineage"]["maximum_depth"]:
        fail("release-candidate lineage length is invalid")
    identity = {
        "release_id": policy["release_id"],
        "governance_policy_sha256": sha256_file(pathlib.Path(policy["_policy_path"])),
        "records": records,
    }
    digest = sha256_bytes(canonical_bytes(identity))
    return {
        "schema_version": 1,
        "report_kind": "clinical-release-candidate-lineage",
        "lineage_id": f"health-release-candidate-lineage-{digest[:24]}",
        **identity,
        "tip_record_id": records[-1]["record_id"],
        "tip_record_digest_sha256": records[-1]["record_digest_sha256"],
        "tip_state": records[-1]["state"],
        "tip_source_revision": records[-1]["source_revision"],
        "lineage_digest_sha256": digest,
        "claims": {
            "lineage_is_append_only": True,
            "predecessors_are_content_addressed": True,
            "stale_plans_and_approvals_are_rejected": True,
            "lineage_alone_does_not_promote_release": True,
        },
    }


def verify_lineage(lineage: dict[str, Any], policy: dict[str, Any], trust_store: dict[str, Any] | None = None) -> dict[str, Any]:
    if lineage.get("schema_version") != 1 or lineage.get("report_kind") != "clinical-release-candidate-lineage":
        fail("release-candidate lineage schema or kind is invalid")
    if lineage.get("release_id") != policy["release_id"]:
        fail("release-candidate lineage release id drifted")
    if lineage.get("governance_policy_sha256") != sha256_file(pathlib.Path(policy["_policy_path"])):
        fail("release-candidate lineage governance policy drifted")
    records = lineage.get("records")
    if not isinstance(records, list) or not records or len(records) > policy["lineage"]["maximum_depth"]:
        fail("release-candidate lineage records are invalid")
    predecessor: str | None = None
    active_authorization: dict[str, Any] | None = None
    last_run: tuple[int, int] | None = None
    for depth, record in enumerate(records, start=1):
        if not isinstance(record, dict):
            fail("release-candidate lineage record is invalid")
        verify_lineage_record(record, policy, depth, predecessor)
        if record["record_type"] == "candidate-authorization":
            if active_authorization is not None and records[depth - 2]["record_type"] == "candidate-authorization":
                fail("candidate authorization must be followed by a rehearsal observation")
            if predecessor is not None and record["base_source_revision"] != records[depth - 2]["source_revision"]:
                fail("candidate authorization does not continue the predecessor source revision")
            if record["source_revision"] != record["target_source_revision"]:
                fail("candidate authorization target revision mismatch")
            embedded_plan = record.get("remediation_plan")
            embedded_manifest = record.get("change_manifest")
            embedded_approval = record.get("approval_bundle")
            if not isinstance(embedded_plan, dict) or not isinstance(embedded_manifest, dict) or not isinstance(embedded_approval, dict):
                fail("candidate authorization lacks embedded governance evidence")
            PLAN.verify_plan(embedded_plan, PLAN.load_policy())
            verify_change_manifest(embedded_manifest, embedded_plan, policy)
            if embedded_plan.get("plan_digest_sha256") != record.get("remediation_plan_digest_sha256"):
                fail("candidate authorization embedded plan digest mismatch")
            if embedded_manifest.get("change_manifest_digest_sha256") != record.get("change_manifest_digest_sha256"):
                fail("candidate authorization embedded change manifest digest mismatch")
            if embedded_approval.get("approval_bundle_digest_sha256") != record.get("approval_bundle_digest_sha256"):
                fail("candidate authorization embedded approval digest mismatch")
            if trust_store is not None:
                verify_approval_bundle(
                    embedded_approval,
                    embedded_plan,
                    embedded_manifest,
                    policy,
                    trust_store,
                    record.get("authorization_verified_at_utc"),
                )
            active_authorization = record
        else:
            if active_authorization is None or record["candidate_authorization_record_digest_sha256"] != active_authorization["record_digest_sha256"]:
                fail("rehearsal observation does not target the active authorization")
            if record["source_revision"] != active_authorization["source_revision"]:
                fail("rehearsal observation source revision differs from authorization")
            embedded_import = record.get("runner_import")
            if not isinstance(embedded_import, dict):
                fail("rehearsal observation lacks embedded runner import")
            IMPORT.verify_import_report(embedded_import)
            if embedded_import.get("report_digest_sha256") != record.get("runner_import_digest_sha256"):
                fail("rehearsal observation embedded import digest mismatch")
            if embedded_import.get("source_revision") != record.get("source_revision"):
                fail("rehearsal observation embedded import revision mismatch")
            current_run = (record["run_id"], record["run_attempt"])
            if last_run is not None and current_run <= last_run:
                fail("rehearsal observations are not strictly ordered")
            last_run = current_run
            active_authorization = None
        predecessor = record["record_digest_sha256"]
    identity = {
        "release_id": lineage["release_id"],
        "governance_policy_sha256": lineage["governance_policy_sha256"],
        "records": records,
    }
    digest = sha256_bytes(canonical_bytes(identity))
    if lineage.get("lineage_digest_sha256") != digest or lineage.get("lineage_id") != f"health-release-candidate-lineage-{digest[:24]}":
        fail("release-candidate lineage digest mismatch")
    tip = records[-1]
    if lineage.get("tip_record_id") != tip["record_id"] or lineage.get("tip_record_digest_sha256") != tip["record_digest_sha256"]:
        fail("release-candidate lineage tip identity mismatch")
    if lineage.get("tip_state") != tip["state"] or lineage.get("tip_source_revision") != tip["source_revision"]:
        fail("release-candidate lineage tip summary mismatch")
    claims = lineage.get("claims")
    if not isinstance(claims, dict) or any(item is not True for item in claims.values()):
        fail("release-candidate lineage claims are invalid")
    return lineage


def authorize_candidate(
    prior_lineage: dict[str, Any] | None,
    plan: dict[str, Any],
    manifest: dict[str, Any],
    approval_bundle: dict[str, Any],
    policy: dict[str, Any],
    trust_store: dict[str, Any],
    verification_time_utc: str,
) -> dict[str, Any]:
    PLAN.verify_plan(plan, PLAN.load_policy())
    verify_change_manifest(manifest, plan, policy)
    verify_approval_bundle(approval_bundle, plan, manifest, policy, trust_store, verification_time_utc)
    records: list[dict[str, Any]] = []
    predecessor: str | None = None
    if prior_lineage is not None:
        verify_lineage(prior_lineage, policy, trust_store)
        records = [dict(item) for item in prior_lineage["records"]]
        if records[-1]["record_type"] == "candidate-authorization":
            fail("cannot append a new candidate before observing the authorized rehearsal")
        predecessor = records[-1]["record_digest_sha256"]
        if manifest["base_source_revision"] != records[-1]["source_revision"]:
            fail("new candidate does not continue the previous source revision")
    record = finalize_lineage_record(
        {
            "schema_version": 1,
            "record_type": "candidate-authorization",
            "state": "authorized-for-rehearsal",
            "release_id": policy["release_id"],
            "depth": len(records) + 1,
            "predecessor_record_digest_sha256": predecessor,
            "source_revision": manifest["target_source_revision"],
            "base_source_revision": manifest["base_source_revision"],
            "target_source_revision": manifest["target_source_revision"],
            "remediation_plan_id": plan["plan_id"],
            "remediation_plan_digest_sha256": plan["plan_digest_sha256"],
            "change_manifest_id": manifest["manifest_id"],
            "change_manifest_digest_sha256": manifest["change_manifest_digest_sha256"],
            "remediation_plan": plan,
            "change_manifest": manifest,
            "approval_bundle": approval_bundle,
            "approval_bundle_id": approval_bundle["bundle_id"],
            "approval_bundle_digest_sha256": approval_bundle["approval_bundle_digest_sha256"],
            "authorization_verified_at_utc": verification_time_utc,
            "approval_expires_at_utc": approval_bundle["statement"]["expires_at_utc"],
        }
    )
    records.append(record)
    result = finalize_lineage(records, policy)
    verify_lineage(result, policy, trust_store)
    return result


def observe_candidate(
    lineage: dict[str, Any], imported: dict[str, Any], policy: dict[str, Any]
) -> dict[str, Any]:
    verify_lineage(lineage, policy)
    IMPORT.verify_import_report(imported)
    records = [dict(item) for item in lineage["records"]]
    authorization = records[-1]
    if authorization["record_type"] != "candidate-authorization":
        fail("lineage tip is not awaiting a rehearsal observation")
    if imported["source_revision"] != authorization["source_revision"]:
        fail("runner import source revision differs from candidate authorization")
    roots = [
        {"stage": item["stage"], "reason_code": item["code"]}
        for item in imported["explanation"]["root_causes"]
    ]
    state = "promotion-eligible" if not roots else "refused"
    record = finalize_lineage_record(
        {
            "schema_version": 1,
            "record_type": "rehearsal-observation",
            "state": state,
            "release_id": policy["release_id"],
            "depth": len(records) + 1,
            "predecessor_record_digest_sha256": authorization["record_digest_sha256"],
            "source_revision": imported["source_revision"],
            "candidate_authorization_record_digest_sha256": authorization["record_digest_sha256"],
            "runner_import": imported,
            "runner_import_id": imported["report_id"],
            "runner_import_digest_sha256": imported["report_digest_sha256"],
            "run_id": imported["run_id"],
            "run_attempt": imported["run_attempt"],
            "root_causes": roots,
        }
    )
    records.append(record)
    result = finalize_lineage(records, policy)
    verify_lineage(result, policy)
    return result
