#!/usr/bin/env python3
"""Mechanically verify coverage of a sealed Cargo.lock movement review statement.

This verifier proves only that every mechanically observed dependency movement is
represented exactly once and explicitly dispositioned. It binds the statement to
the exact independent `VerifiedRepairCandidateOnly` receipt, but does not
authenticate a reviewer or authorize committing a lockfile.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
from typing import Any, NoReturn

REVIEW_SCHEMA = "mycelix-health-lock-movement-review-statement/v1"
VERIFICATION_SCHEMA = "mycelix-health-lock-movement-review-verification/v1"
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
    raise SystemExit(f"lock movement review verification failed: {message}")


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def load_json(path: pathlib.Path, *, label: str) -> tuple[dict[str, Any], bytes]:
    try:
        raw = path.read_bytes()
        value = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot load {label} {path}: {error}")
    if not isinstance(value, dict):
        fail(f"{label} must contain one JSON object")
    if raw != canonical_bytes(value):
        fail(f"{label} is not canonical JSON")
    return value, raw


def read_sidecar(path: pathlib.Path) -> str:
    sidecar = pathlib.Path(str(path) + ".sha256")
    try:
        value = sidecar.read_text(encoding="utf-8").strip()
    except OSError as error:
        fail(f"cannot read sidecar {sidecar}: {error}")
    if len(value) != 64 or any(ch not in "0123456789abcdefABCDEF" for ch in value):
        fail(f"invalid SHA-256 sidecar: {sidecar}")
    return value.lower()


def require_sidecar(path: pathlib.Path, raw: bytes, *, label: str) -> None:
    if read_sidecar(path) != sha256_bytes(raw):
        fail(f"{label} sidecar mismatch")


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
        fail(f"refusing to overwrite verification artifact: {path}")
    try:
        with path.open("xb") as handle:
            handle.write(data)
    except FileExistsError:
        fail(f"refusing to overwrite verification artifact: {path}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-dir", type=pathlib.Path, required=True)
    parser.add_argument("--review", type=pathlib.Path, required=True)
    parser.add_argument("--verification", type=pathlib.Path, required=True)
    parser.add_argument("--require-no-rejections", action="store_true")
    args = parser.parse_args()

    candidate_dir = args.candidate_dir.resolve()
    review_path = args.review.resolve()
    output_path = args.verification.resolve()
    candidate_path = candidate_dir / "lock-candidate.json"
    candidate_verification_path = candidate_dir / "lock-candidate-verification.json"
    audit_path = candidate_dir / "lock-audit.json"

    candidate, candidate_raw = load_json(candidate_path, label="candidate report")
    candidate_verification, candidate_verification_raw = load_json(
        candidate_verification_path, label="candidate verification"
    )
    audit, audit_raw = load_json(audit_path, label="lock audit")
    review, review_raw = load_json(review_path, label="sealed review statement")
    require_sidecar(candidate_path, candidate_raw, label="candidate report")
    require_sidecar(candidate_verification_path, candidate_verification_raw, label="candidate verification")
    require_sidecar(review_path, review_raw, label="review statement")

    if candidate.get("authority") != "RepairCandidateOnly":
        fail("candidate authority must be RepairCandidateOnly")
    if candidate_verification.get("schema") != "mycelix-health-cargo-lock-candidate-verification/v1":
        fail("unsupported candidate verification schema")
    if candidate_verification.get("authority") != "VerifiedRepairCandidateOnly" or candidate_verification.get("result") != "pass":
        fail("candidate verification must be a passing VerifiedRepairCandidateOnly receipt")
    if candidate_verification.get("candidate_report_sha256") != sha256_bytes(candidate_raw):
        fail("candidate verification does not bind supplied candidate report")
    if candidate.get("audit") != audit or candidate_verification.get("audit") != audit:
        fail("candidate/verification audit differs from lock-audit.json")
    if review.get("schema") != REVIEW_SCHEMA or review.get("authority") != "ReviewStatementOnly":
        fail("unsupported sealed review schema or authority")

    candidate_lock = (candidate_dir / "Cargo.lock.candidate").read_bytes()
    candidate_lock_sha = sha256_bytes(candidate_lock)
    if candidate.get("locks", {}).get("candidate_sha256") != candidate_lock_sha:
        fail("candidate report does not bind candidate lock bytes")
    if candidate_verification.get("locks", {}).get("candidate_sha256") != candidate_lock_sha:
        fail("candidate verification does not bind candidate lock bytes")
    if candidate_verification.get("subject") != candidate.get("subject"):
        fail("candidate verification subject differs from candidate report")

    expected_bindings = {
        "candidate_report_sha256": sha256_bytes(candidate_raw),
        "candidate_verification_sha256": sha256_bytes(candidate_verification_raw),
        "candidate_lock_sha256": candidate_lock_sha,
        "audit_sha256": sha256_bytes(audit_raw),
    }
    for key, expected in expected_bindings.items():
        if review.get(key) != expected:
            fail(f"review {key} does not match candidate evidence")
    if review.get("subject") != candidate.get("subject"):
        fail("review subject identity differs from candidate")
    if review.get("harness") != candidate.get("harness"):
        fail("review harness identity differs from candidate")

    expected: dict[str, tuple[str, Any]] = {}
    for category, value in movement_values(audit):
        identifier = movement_id(category, value)
        if identifier in expected:
            fail(f"duplicate mechanically derived movement ID: {identifier}")
        expected[identifier] = (category, value)

    raw_items = review.get("items")
    if not isinstance(raw_items, list):
        fail("review items must be an array")
    observed: dict[str, dict[str, Any]] = {}
    counts = {name: 0 for name in ALLOWED}
    for index, item in enumerate(raw_items):
        if not isinstance(item, dict):
            fail(f"review item {index} must be an object")
        identifier = item.get("movement_id")
        if not isinstance(identifier, str) or identifier in observed:
            fail(f"review item {index} has missing/duplicate movement_id")
        if identifier not in expected:
            fail(f"review item {index} references unknown movement_id {identifier}")
        expected_category, expected_value = expected[identifier]
        if item.get("category") != expected_category or item.get("value") != expected_value:
            fail(f"review item {identifier} differs from mechanically derived movement")
        disposition = item.get("disposition")
        if disposition not in ALLOWED:
            fail(f"review item {identifier} has invalid disposition {disposition!r}")
        rationale = item.get("rationale")
        reference = item.get("review_reference")
        if not isinstance(rationale, str) or not rationale.strip() or not isinstance(reference, str):
            fail(f"review item {identifier} requires rationale and string review_reference")
        counts[disposition] += 1
        observed[identifier] = item

    missing = sorted(set(expected) - set(observed))
    if missing:
        fail(f"review statement omits {len(missing)} movement(s)")
    if len(observed) != len(expected):
        fail("review statement movement count differs from audit")

    has_rejections = counts["rejected"] > 0
    if args.require_no_rejections and has_rejections:
        fail("review contains rejected movements")

    result = {
        "schema": VERIFICATION_SCHEMA,
        "authority": "MechanicalReviewCoverageComplete",
        "candidate_report_sha256": sha256_bytes(candidate_raw),
        "candidate_verification_sha256": sha256_bytes(candidate_verification_raw),
        "candidate_lock_sha256": candidate_lock_sha,
        "audit_sha256": sha256_bytes(audit_raw),
        "review_statement_sha256": sha256_bytes(review_raw),
        "subject": candidate.get("subject"),
        "harness": candidate.get("harness"),
        "movement_count": len(expected),
        "disposition_counts": dict(sorted(counts.items())),
        "complete": True,
        "has_rejections": has_rejections,
        "all_non_rejected": not has_rejections,
        "review_reference_authentication": "UnverifiedContextOnly",
        "non_claims": [
            "MechanicalReviewCoverageComplete is bound to the exact VerifiedRepairCandidateOnly receipt but does not upgrade its authority.",
            "MechanicalReviewCoverageComplete does not authenticate the human reviewer or review_reference strings.",
            "Complete coverage is not approval to copy, commit, merge, or qualify Cargo.lock.candidate.",
            "This verification does not establish repository or clinical qualification.",
        ],
    }
    raw = canonical_bytes(result)
    write_create_only(output_path, raw)
    write_create_only(pathlib.Path(str(output_path) + ".sha256"), (sha256_bytes(raw) + "\n").encode())
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
