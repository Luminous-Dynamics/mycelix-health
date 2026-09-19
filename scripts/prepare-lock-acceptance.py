#!/usr/bin/env python3
"""Prepare a canonical dependency-graph acceptance statement for human signing.

This program does not sign or approve anything. It binds the exact independently
verified repair candidate and the exact mechanically complete movement review into
bytes intended for an authorized human to sign with OpenSSH.

Authority: AcceptanceStatementToSignOnly.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
from typing import Any, NoReturn

SCHEMA = "mycelix-health-lock-acceptance-statement/v1"
NAMESPACE = "mycelix-health-lock-acceptance-v1"


def fail(message: str) -> NoReturn:
    raise SystemExit(f"lock acceptance preparation failed: {message}")


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
        value = sidecar.read_text(encoding="utf-8").strip().lower()
    except OSError as error:
        fail(f"cannot read sidecar {sidecar}: {error}")
    if len(value) != 64 or any(ch not in "0123456789abcdef" for ch in value):
        fail(f"invalid SHA-256 sidecar: {sidecar}")
    return value


def require_sidecar(path: pathlib.Path, raw: bytes, *, label: str) -> None:
    if read_sidecar(path) != sha256_bytes(raw):
        fail(f"{label} sidecar mismatch")


def write_create_only(path: pathlib.Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() or path.is_symlink():
        fail(f"refusing to overwrite acceptance artifact: {path}")
    try:
        with path.open("xb") as handle:
            handle.write(data)
    except FileExistsError:
        fail(f"refusing to overwrite acceptance artifact: {path}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-dir", type=pathlib.Path, required=True)
    parser.add_argument("--principal", required=True, help="OpenSSH allowed_signers principal expected to sign")
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()

    candidate_dir = args.candidate_dir.resolve()
    output = args.output.resolve()
    principal = args.principal.strip()
    if not principal or any(ch.isspace() for ch in principal):
        fail("principal must be one non-empty whitespace-free identifier")

    paths = {
        "candidate_report": candidate_dir / "lock-candidate.json",
        "candidate_verification": candidate_dir / "lock-candidate-verification.json",
        "review_statement": candidate_dir / "lock-movement-review-statement.json",
        "review_verification": candidate_dir / "lock-movement-review-verification.json",
    }
    loaded: dict[str, tuple[dict[str, Any], bytes]] = {}
    for label, path in paths.items():
        value, raw = load_canonical(path, label=label.replace("_", " "))
        require_sidecar(path, raw, label=label.replace("_", " "))
        loaded[label] = (value, raw)

    candidate, candidate_raw = loaded["candidate_report"]
    candidate_verification, candidate_verification_raw = loaded["candidate_verification"]
    review_statement, review_statement_raw = loaded["review_statement"]
    review_verification, review_verification_raw = loaded["review_verification"]

    if candidate.get("schema") != "mycelix-health-cargo-lock-candidate/v1" or candidate.get("authority") != "RepairCandidateOnly":
        fail("unsupported repair candidate")
    if candidate_verification.get("schema") != "mycelix-health-cargo-lock-candidate-verification/v1" or candidate_verification.get("authority") != "VerifiedRepairCandidateOnly" or candidate_verification.get("result") != "pass":
        fail("candidate verification must be successful VerifiedRepairCandidateOnly")
    if candidate_verification.get("candidate_report_sha256") != sha256_bytes(candidate_raw):
        fail("candidate verification does not bind candidate report")
    if review_statement.get("schema") != "mycelix-health-lock-movement-review-statement/v1" or review_statement.get("authority") != "ReviewStatementOnly":
        fail("unsupported review statement")
    if review_statement.get("candidate_verification_sha256") != sha256_bytes(candidate_verification_raw):
        fail("review statement does not bind candidate verification")
    if review_verification.get("schema") != "mycelix-health-lock-movement-review-verification/v1" or review_verification.get("authority") != "MechanicalReviewCoverageComplete":
        fail("unsupported movement-review verification")
    if review_verification.get("review_statement_sha256") != sha256_bytes(review_statement_raw):
        fail("review verification does not bind review statement")
    if review_verification.get("candidate_verification_sha256") != sha256_bytes(candidate_verification_raw):
        fail("review verification does not bind candidate verification")
    if review_verification.get("complete") is not True or review_verification.get("has_rejections") is not False or review_verification.get("all_non_rejected") is not True:
        fail("dependency movement review is incomplete or contains rejected movement")

    lock_path = candidate_dir / "Cargo.lock.candidate"
    try:
        lock_bytes = lock_path.read_bytes()
    except OSError as error:
        fail(f"cannot read candidate lock: {error}")
    lock_sha = sha256_bytes(lock_bytes)
    for value, label in (
        (candidate.get("locks", {}).get("candidate_sha256"), "candidate report"),
        (candidate_verification.get("locks", {}).get("candidate_sha256"), "candidate verification"),
        (review_statement.get("candidate_lock_sha256"), "review statement"),
        (review_verification.get("candidate_lock_sha256"), "review verification"),
    ):
        if value != lock_sha:
            fail(f"{label} does not bind candidate lock")

    subject = candidate.get("subject")
    harness = candidate.get("harness")
    for value, label in (
        (candidate_verification.get("subject"), "candidate verification"),
        (review_statement.get("subject"), "review statement"),
        (review_verification.get("subject"), "review verification"),
    ):
        if value != subject:
            fail(f"{label} subject differs from candidate")
    if candidate_verification.get("harness", {}).get("sha") != (harness or {}).get("sha") or candidate_verification.get("harness", {}).get("tree_sha") != (harness or {}).get("tree_sha"):
        fail("candidate verification harness differs from candidate")

    statement = {
        "schema": SCHEMA,
        "authority": "AcceptanceStatementToSignOnly",
        "signature_namespace": NAMESPACE,
        "principal": principal,
        "decision": "accept_exact_candidate_lock_for_replacement_subject_preparation",
        "subject": subject,
        "harness": harness,
        "locks": {
            "before_sha256": candidate.get("locks", {}).get("before_sha256"),
            "candidate_sha256": lock_sha,
        },
        "evidence": {
            "candidate_report_sha256": sha256_bytes(candidate_raw),
            "candidate_verification_sha256": sha256_bytes(candidate_verification_raw),
            "review_statement_sha256": sha256_bytes(review_statement_raw),
            "review_verification_sha256": sha256_bytes(review_verification_raw),
        },
        "non_claims": [
            "This unsigned statement is not acceptance evidence until its detached OpenSSH signature is independently verified.",
            "Acceptance of the dependency graph is not repository qualification or clinical evidence.",
            "A replacement lock-bearing commit must still be separately constructed, identity-verified, and qualified with --locked execution.",
        ],
    }
    raw = canonical_bytes(statement)
    write_create_only(output, raw)
    write_create_only(pathlib.Path(str(output) + ".sha256"), (sha256_bytes(raw) + "\n").encode())
    print(json.dumps(statement, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
