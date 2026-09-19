#!/usr/bin/env python3
"""Create a review-only manifest covering every Cargo.lock dependency movement.

This program does not approve a lockfile. It converts the mechanically derived
lock audit into a canonical create-only checklist whose every item starts as
`unreviewed`. The template is bound to both the repair candidate and the exact
independent `VerifiedRepairCandidateOnly` receipt.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
from typing import Any, NoReturn

SCHEMA = "mycelix-health-lock-movement-review/v1"
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
    raise SystemExit(f"lock movement review template failed: {message}")


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def load_canonical(path: pathlib.Path, *, label: str) -> tuple[dict[str, Any], bytes]:
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
        fail(f"cannot read digest sidecar {sidecar}: {error}")
    if len(value) != 64 or any(ch not in "0123456789abcdefABCDEF" for ch in value):
        fail(f"invalid SHA-256 sidecar: {sidecar}")
    return value.lower()


def require_sidecar(path: pathlib.Path, raw: bytes, *, label: str) -> None:
    if read_sidecar(path) != sha256_bytes(raw):
        fail(f"{label} sidecar digest mismatch")


def write_create_only(path: pathlib.Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() or path.is_symlink():
        fail(f"refusing to overwrite review artifact: {path}")
    try:
        with path.open("xb") as handle:
            handle.write(data)
    except FileExistsError:
        fail(f"refusing to overwrite review artifact: {path}")


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


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-dir", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()

    candidate_dir = args.candidate_dir.resolve()
    output = args.output.resolve()
    candidate_path = candidate_dir / "lock-candidate.json"
    verification_path = candidate_dir / "lock-candidate-verification.json"
    audit_path = candidate_dir / "lock-audit.json"
    candidate, candidate_raw = load_canonical(candidate_path, label="candidate report")
    verification, verification_raw = load_canonical(verification_path, label="candidate verification")
    audit, audit_raw = load_canonical(audit_path, label="lock audit")
    require_sidecar(candidate_path, candidate_raw, label="candidate report")
    require_sidecar(verification_path, verification_raw, label="candidate verification")

    if candidate.get("schema") != "mycelix-health-cargo-lock-candidate/v1":
        fail("unsupported candidate schema")
    if candidate.get("authority") != "RepairCandidateOnly":
        fail("candidate authority must be RepairCandidateOnly")
    if verification.get("schema") != "mycelix-health-cargo-lock-candidate-verification/v1":
        fail("unsupported candidate verification schema")
    if verification.get("authority") != "VerifiedRepairCandidateOnly" or verification.get("result") != "pass":
        fail("candidate verification must be a passing VerifiedRepairCandidateOnly receipt")
    if verification.get("candidate_report_sha256") != sha256_bytes(candidate_raw):
        fail("candidate verification does not bind the supplied candidate report")

    try:
        candidate_lock = (candidate_dir / "Cargo.lock.candidate").read_bytes()
    except OSError as error:
        fail(f"cannot read candidate lock: {error}")
    candidate_lock_sha = sha256_bytes(candidate_lock)
    if candidate.get("locks", {}).get("candidate_sha256") != candidate_lock_sha:
        fail("candidate lock digest does not match candidate report")
    if verification.get("locks", {}).get("candidate_sha256") != candidate_lock_sha:
        fail("candidate verification does not bind candidate lock bytes")
    if candidate.get("audit") != audit or verification.get("audit") != audit:
        fail("candidate/verification audit differs from lock-audit.json")
    if verification.get("subject") != candidate.get("subject"):
        fail("candidate verification subject differs from candidate report")

    items = []
    for category, value in movement_values(audit):
        items.append(
            {
                "movement_id": movement_id(category, value),
                "category": category,
                "value": value,
                "disposition": "unreviewed",
                "rationale": "",
                "review_reference": "",
            }
        )
    items.sort(key=lambda item: (item["category"], item["movement_id"]))

    review = {
        "schema": SCHEMA,
        "authority": "ReviewTemplateOnly",
        "candidate_report_sha256": sha256_bytes(candidate_raw),
        "candidate_verification_sha256": sha256_bytes(verification_raw),
        "candidate_lock_sha256": candidate_lock_sha,
        "audit_sha256": sha256_bytes(audit_raw),
        "subject": candidate.get("subject"),
        "harness": candidate.get("harness"),
        "items": items,
        "instructions": {
            "allowed_dispositions": ["unreviewed", "expected", "necessary", "accepted", "rejected"],
            "rationale_required_when_reviewed": True,
            "review_reference_is_unverified_context_only": True,
        },
        "non_claims": [
            "This template is not evidence that any movement was reviewed.",
            "The independent candidate verification remains repair evidence only, not dependency approval.",
            "A completed manifest does not cryptographically authenticate the reviewer.",
            "Mechanical review coverage is not permission to commit, merge, or qualify a lockfile.",
        ],
    }
    raw = canonical_bytes(review)
    write_create_only(output, raw)
    write_create_only(pathlib.Path(str(output) + ".sha256"), (sha256_bytes(raw) + "\n").encode())
    print(json.dumps(review, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
