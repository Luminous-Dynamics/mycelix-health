#!/usr/bin/env python3
"""Seal an edited lock-movement review draft into canonical create-only evidence.

The sealer verifies that every mechanically observed dependency movement is
represented exactly once and explicitly dispositioned with a rationale. The
sealed statement is bound to the exact independent `VerifiedRepairCandidateOnly`
receipt. It does not authenticate the reviewer or authorize committing the lock.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
from typing import Any, NoReturn

TEMPLATE_SCHEMA = "mycelix-health-lock-movement-review/v1"
STATEMENT_SCHEMA = "mycelix-health-lock-movement-review-statement/v1"
ALLOWED = frozenset({"expected", "necessary", "accepted", "rejected"})
AUDIT_CATEGORIES = (
    "added_local_packages",
    "removed_local_packages",
    "added_registry_packages",
    "removed_registry_packages",
    "changed_registry_versions",
    "added_git_packages",
    "removed_git_packages",
)


def fail(message: str) -> NoReturn:
    raise SystemExit(f"lock movement review sealing failed: {message}")


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def load_object(path: pathlib.Path, *, label: str, canonical: bool = False) -> tuple[dict[str, Any], bytes]:
    try:
        raw = path.read_bytes()
        value = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot load {label} {path}: {error}")
    if not isinstance(value, dict):
        fail(f"{label} must contain one JSON object")
    if canonical and raw != canonical_bytes(value):
        fail(f"{label} is not canonical JSON")
    return value, raw


def read_sidecar(path: pathlib.Path) -> str:
    sidecar = pathlib.Path(str(path) + ".sha256")
    try:
        value = sidecar.read_text(encoding="utf-8").strip()
    except OSError as error:
        fail(f"cannot read digest sidecar {sidecar}: {error}")
    if len(value) != 64 or any(ch not in "0123456789abcdefABCDEF" for ch in value):
        fail(f"invalid SHA-256 sidecar: {sidecar}")
    return value.lower()


def require_sidecar(path: pathlib.Path, raw: bytes, *, label: str) -> None:
    if read_sidecar(path) != sha256_bytes(raw):
        fail(f"{label} sidecar digest mismatch")


def movement_values(audit: dict[str, Any]) -> list[tuple[str, Any]]:
    values: list[tuple[str, Any]] = []
    for category in AUDIT_CATEGORIES:
        raw = audit.get(category)
        if category == "changed_registry_versions":
            if not isinstance(raw, dict):
                fail(f"audit category {category} must be an object")
            for name in sorted(raw):
                item = raw[name]
                if not isinstance(item, dict):
                    fail(f"audit category {category}[{name!r}] must be an object")
                values.append((category, {"name": name, "before": item.get("before"), "after": item.get("after")}))
        else:
            if not isinstance(raw, list):
                fail(f"audit category {category} must be an array")
            values.extend((category, item) for item in raw)
    return values


def movement_id(category: str, value: Any) -> str:
    material = canonical_bytes({"category": category, "value": value})
    return sha256_bytes(b"mycelix-health-lock-movement-review-item/v1\0" + material)


def write_create_only(path: pathlib.Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() or path.is_symlink():
        fail(f"refusing to overwrite sealed review artifact: {path}")
    try:
        with path.open("xb") as handle:
            handle.write(data)
    except FileExistsError:
        fail(f"refusing to overwrite sealed review artifact: {path}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-dir", type=pathlib.Path, required=True)
    parser.add_argument("--draft", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()

    candidate_dir = args.candidate_dir.resolve()
    draft_path = args.draft.resolve()
    output = args.output.resolve()
    candidate_path = candidate_dir / "lock-candidate.json"
    verification_path = candidate_dir / "lock-candidate-verification.json"
    audit_path = candidate_dir / "lock-audit.json"

    candidate, candidate_raw = load_object(candidate_path, label="candidate report", canonical=True)
    verification, verification_raw = load_object(verification_path, label="candidate verification", canonical=True)
    audit, audit_raw = load_object(audit_path, label="lock audit", canonical=True)
    draft, _ = load_object(draft_path, label="review draft")
    require_sidecar(candidate_path, candidate_raw, label="candidate report")
    require_sidecar(verification_path, verification_raw, label="candidate verification")

    if candidate.get("authority") != "RepairCandidateOnly":
        fail("candidate authority must be RepairCandidateOnly")
    if verification.get("schema") != "mycelix-health-cargo-lock-candidate-verification/v1":
        fail("unsupported candidate verification schema")
    if verification.get("authority") != "VerifiedRepairCandidateOnly" or verification.get("result") != "pass":
        fail("candidate verification must be a passing VerifiedRepairCandidateOnly receipt")
    if verification.get("candidate_report_sha256") != sha256_bytes(candidate_raw):
        fail("candidate verification does not bind supplied candidate report")
    if candidate.get("audit") != audit or verification.get("audit") != audit:
        fail("candidate/verification audit differs from lock-audit.json")
    if draft.get("schema") != TEMPLATE_SCHEMA or draft.get("authority") != "ReviewTemplateOnly":
        fail("unsupported review draft schema or authority")

    candidate_lock = (candidate_dir / "Cargo.lock.candidate").read_bytes()
    candidate_lock_sha = sha256_bytes(candidate_lock)
    if candidate.get("locks", {}).get("candidate_sha256") != candidate_lock_sha:
        fail("candidate report does not bind candidate lock bytes")
    if verification.get("locks", {}).get("candidate_sha256") != candidate_lock_sha:
        fail("candidate verification does not bind candidate lock bytes")
    if verification.get("subject") != candidate.get("subject"):
        fail("candidate verification subject differs from candidate report")

    bindings = {
        "candidate_report_sha256": sha256_bytes(candidate_raw),
        "candidate_verification_sha256": sha256_bytes(verification_raw),
        "candidate_lock_sha256": candidate_lock_sha,
        "audit_sha256": sha256_bytes(audit_raw),
    }
    for key, expected in bindings.items():
        if draft.get(key) != expected:
            fail(f"review draft {key} does not match candidate evidence")
    if draft.get("subject") != candidate.get("subject") or draft.get("harness") != candidate.get("harness"):
        fail("review draft subject/harness identity differs from candidate")

    expected: dict[str, tuple[str, Any]] = {}
    for category, value in movement_values(audit):
        identifier = movement_id(category, value)
        if identifier in expected:
            fail(f"duplicate mechanically derived movement ID: {identifier}")
        expected[identifier] = (category, value)

    items = draft.get("items")
    if not isinstance(items, list):
        fail("review draft items must be an array")
    observed: dict[str, dict[str, Any]] = {}
    for index, raw_item in enumerate(items):
        if not isinstance(raw_item, dict):
            fail(f"review draft item {index} must be an object")
        identifier = raw_item.get("movement_id")
        if not isinstance(identifier, str) or identifier in observed:
            fail(f"review draft item {index} has missing/duplicate movement_id")
        if identifier not in expected:
            fail(f"review draft item {index} references unknown movement_id {identifier}")
        category, value = expected[identifier]
        if raw_item.get("category") != category or raw_item.get("value") != value:
            fail(f"review draft item {identifier} differs from mechanically derived movement")
        disposition = raw_item.get("disposition")
        if disposition not in ALLOWED:
            fail(f"review draft item {identifier} must be explicitly dispositioned; got {disposition!r}")
        rationale = raw_item.get("rationale")
        reference = raw_item.get("review_reference")
        if not isinstance(rationale, str) or not rationale.strip():
            fail(f"review draft item {identifier} requires non-empty rationale")
        if not isinstance(reference, str):
            fail(f"review draft item {identifier} review_reference must be a string")
        observed[identifier] = {
            "movement_id": identifier,
            "category": category,
            "value": value,
            "disposition": disposition,
            "rationale": rationale.strip(),
            "review_reference": reference.strip(),
        }

    missing = sorted(set(expected) - set(observed))
    if missing:
        fail(f"review draft omits {len(missing)} movement(s)")
    if len(observed) != len(expected):
        fail("review draft movement count differs from audit")

    sealed = {
        "schema": STATEMENT_SCHEMA,
        "authority": "ReviewStatementOnly",
        **bindings,
        "subject": candidate.get("subject"),
        "harness": candidate.get("harness"),
        "items": sorted(observed.values(), key=lambda item: (item["category"], item["movement_id"])),
        "review_reference_authentication": "UnverifiedContextOnly",
        "non_claims": [
            "ReviewStatementOnly depends on the exact independently verified repair candidate but does not upgrade its authority.",
            "ReviewStatementOnly does not cryptographically authenticate a reviewer or review_reference.",
            "A sealed statement is not permission to copy, commit, merge, or qualify Cargo.lock.candidate.",
            "This statement is dependency-review context, not repository or clinical qualification.",
        ],
    }
    raw = canonical_bytes(sealed)
    write_create_only(output, raw)
    write_create_only(pathlib.Path(str(output) + ".sha256"), (sha256_bytes(raw) + "\n").encode())
    print(json.dumps(sealed, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
