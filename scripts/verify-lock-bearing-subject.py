#!/usr/bin/env python3
"""Verify a human-created lock-bearing replacement commit against a prepared plan.

This verifier proves Git/content identity only. It independently rebinds the plan
to the exact portable repair and review artifacts, then proves that the replacement
commit has exactly the frozen subject as its sole parent, exactly the precomputed
replacement tree, and exactly the reviewed Cargo.lock bytes as its only change.

Terminal authority: VerifiedReplacementSubjectIdentityOnly.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import subprocess
from typing import Any, NoReturn

SCHEMA = "mycelix-health-lock-bearing-subject-verification/v1"
HEX = frozenset("0123456789abcdefABCDEF")


def fail(message: str) -> NoReturn:
    raise SystemExit(f"replacement subject verification failed: {message}")


def run(*args: str, cwd: pathlib.Path | None = None) -> str:
    try:
        result = subprocess.run(args, cwd=cwd, check=True, capture_output=True, text=True)
    except (OSError, subprocess.CalledProcessError) as error:
        detail = ""
        if isinstance(error, subprocess.CalledProcessError):
            detail = f"\nstdout:\n{error.stdout or ''}\nstderr:\n{error.stderr or ''}"
        fail(f"command failed: {' '.join(args)}{detail}")
    return result.stdout.strip()


def run_bytes(*args: str, cwd: pathlib.Path | None = None) -> bytes:
    try:
        result = subprocess.run(args, cwd=cwd, check=True, capture_output=True)
    except (OSError, subprocess.CalledProcessError) as error:
        detail = ""
        if isinstance(error, subprocess.CalledProcessError):
            detail = f"\nstdout:\n{(error.stdout or b'').decode(errors='replace')}\nstderr:\n{(error.stderr or b'').decode(errors='replace')}"
        fail(f"command failed: {' '.join(args)}{detail}")
    return result.stdout


def git(repo: pathlib.Path, *args: str) -> str:
    return run("git", "-C", str(repo), *args)


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def full_sha(value: str, *, label: str) -> str:
    value = value.strip().lower()
    if len(value) != 40 or any(char not in HEX for char in value):
        fail(f"{label} must be a full 40-character hexadecimal Git SHA")
    return value


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
    if len(value) != 64 or any(char not in "0123456789abcdef" for char in value):
        fail(f"invalid SHA-256 sidecar: {sidecar}")
    return value


def require_sidecar(path: pathlib.Path, raw: bytes, *, label: str) -> None:
    if read_sidecar(path) != sha256_bytes(raw):
        fail(f"{label} sidecar mismatch")


def write_create_only(path: pathlib.Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() or path.is_symlink():
        fail(f"refusing to overwrite verification artifact: {path}")
    try:
        with path.open("xb") as handle:
            handle.write(data)
    except FileExistsError:
        fail(f"refusing to overwrite verification artifact: {path}")


def harness_identity(value: Any, *, label: str) -> tuple[str, str]:
    if not isinstance(value, dict):
        fail(f"{label} harness identity is malformed")
    return (
        full_sha(str(value.get("sha", "")), label=f"{label} harness SHA"),
        full_sha(str(value.get("tree_sha", "")), label=f"{label} harness tree"),
    )


def upstream_evidence(candidate_dir: pathlib.Path) -> dict[str, Any]:
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
        fail("unsupported candidate report schema or authority")
    if candidate_verification.get("schema") != "mycelix-health-cargo-lock-candidate-verification/v1":
        fail("unsupported candidate verification schema")
    if candidate_verification.get("authority") != "VerifiedRepairCandidateOnly" or candidate_verification.get("result") != "pass":
        fail("candidate verification must be successful VerifiedRepairCandidateOnly")
    if review_statement.get("schema") != "mycelix-health-lock-movement-review-statement/v1" or review_statement.get("authority") != "ReviewStatementOnly":
        fail("unsupported review statement schema or authority")
    if review_verification.get("schema") != "mycelix-health-lock-movement-review-verification/v1" or review_verification.get("authority") != "MechanicalReviewCoverageComplete":
        fail("unsupported review verification schema or authority")

    candidate_sha = sha256_bytes(candidate_raw)
    candidate_verification_sha = sha256_bytes(candidate_verification_raw)
    review_statement_sha = sha256_bytes(review_statement_raw)
    if candidate_verification.get("candidate_report_sha256") != candidate_sha:
        fail("candidate verification does not bind candidate report")
    if review_statement.get("candidate_report_sha256") != candidate_sha:
        fail("review statement does not bind candidate report")
    if review_statement.get("candidate_verification_sha256") != candidate_verification_sha:
        fail("review statement does not bind candidate verification")
    if review_verification.get("candidate_report_sha256") != candidate_sha:
        fail("review verification does not bind candidate report")
    if review_verification.get("candidate_verification_sha256") != candidate_verification_sha:
        fail("review verification does not bind candidate verification")
    if review_verification.get("review_statement_sha256") != review_statement_sha:
        fail("review verification does not bind review statement")
    if review_verification.get("complete") is not True or review_verification.get("has_rejections") is not False or review_verification.get("all_non_rejected") is not True:
        fail("upstream review is incomplete or contains rejected movement")

    candidate_locks = candidate.get("locks")
    candidate_verification_locks = candidate_verification.get("locks")
    if not isinstance(candidate_locks, dict) or not isinstance(candidate_verification_locks, dict):
        fail("candidate lock identities are malformed")
    before_lock_sha = candidate_locks.get("before_sha256")
    if candidate_verification_locks.get("before_sha256") != before_lock_sha:
        fail("candidate verification before-lock digest differs from candidate report")

    lock_path = candidate_dir / "Cargo.lock.candidate"
    try:
        lock_bytes = lock_path.read_bytes()
    except OSError as error:
        fail(f"cannot read candidate lock: {error}")
    lock_sha = sha256_bytes(lock_bytes)
    for value, label in (
        (candidate_locks.get("candidate_sha256"), "candidate report"),
        (candidate_verification_locks.get("candidate_sha256"), "candidate verification"),
        (review_statement.get("candidate_lock_sha256"), "review statement"),
        (review_verification.get("candidate_lock_sha256"), "review verification"),
    ):
        if value != lock_sha:
            fail(f"{label} does not bind candidate lock")

    subject = candidate.get("subject")
    if not isinstance(subject, dict):
        fail("candidate subject identity is malformed")
    for value, label in (
        (candidate_verification.get("subject"), "candidate verification"),
        (review_statement.get("subject"), "review statement"),
        (review_verification.get("subject"), "review verification"),
    ):
        if value != subject:
            fail(f"{label} subject identity differs from candidate")

    candidate_harness = harness_identity(candidate.get("harness"), label="candidate")
    candidate_verification_harness = harness_identity(candidate_verification.get("harness"), label="candidate verification")
    if candidate_verification_harness != candidate_harness:
        fail("candidate verification harness SHA/tree differs from candidate report")
    if review_statement.get("harness") != candidate.get("harness"):
        fail("review statement harness identity differs from candidate")
    if review_verification.get("harness") != candidate.get("harness"):
        fail("review verification harness identity differs from candidate")

    return {
        "subject": subject,
        "candidate_lock_sha256": lock_sha,
        "before_lock_sha256": before_lock_sha,
        "digests": {
            "candidate_report_sha256": candidate_sha,
            "candidate_verification_sha256": candidate_verification_sha,
            "review_statement_sha256": review_statement_sha,
            "review_verification_sha256": sha256_bytes(review_verification_raw),
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=pathlib.Path, required=True)
    parser.add_argument("--replacement-sha", required=True)
    parser.add_argument("--plan", type=pathlib.Path, required=True)
    parser.add_argument("--candidate-dir", type=pathlib.Path, required=True)
    parser.add_argument("--verification", type=pathlib.Path, required=True)
    args = parser.parse_args()

    repository = args.repository.resolve()
    replacement_sha = full_sha(args.replacement_sha, label="replacement commit SHA")
    plan_path = args.plan.resolve()
    candidate_dir = args.candidate_dir.resolve()
    output = args.verification.resolve()

    if not repository.is_dir():
        fail(f"repository does not exist: {repository}")
    if not candidate_dir.is_dir():
        fail(f"candidate directory does not exist: {candidate_dir}")

    upstream = upstream_evidence(candidate_dir)
    plan, plan_raw = load_canonical(plan_path, label="replacement subject plan")
    if read_sidecar(plan_path) != sha256_bytes(plan_raw):
        fail("replacement subject plan sidecar mismatch")
    if plan.get("schema") != "mycelix-health-lock-bearing-subject-plan/v1":
        fail("unsupported replacement subject plan schema")
    if plan.get("authority") != "PreparedReplacementSubjectOnly":
        fail("replacement subject plan authority must be PreparedReplacementSubjectOnly")
    if plan.get("evidence") != upstream["digests"]:
        fail("replacement subject plan evidence bindings differ from upstream evidence")
    if plan.get("subject") != upstream["subject"]:
        fail("replacement subject plan subject differs from upstream evidence")

    subject = plan.get("subject")
    proposed = plan.get("proposed_replacement")
    locks = plan.get("locks")
    if not isinstance(subject, dict) or not isinstance(proposed, dict) or not isinstance(locks, dict):
        fail("replacement subject plan is malformed")
    subject_sha = full_sha(str(subject.get("sha", "")), label="planned subject SHA")
    subject_tree = full_sha(str(subject.get("tree_sha", "")), label="planned subject tree")
    proposed_tree = full_sha(str(proposed.get("tree_sha", "")), label="planned replacement tree")
    if locks.get("candidate_sha256") != upstream["candidate_lock_sha256"] or locks.get("before_sha256") != upstream["before_lock_sha256"]:
        fail("replacement subject plan lock bindings differ from upstream evidence")
    if proposed.get("changed_paths") != ["Cargo.lock"] or proposed.get("parent_must_equal_subject_sha") is not True:
        fail("replacement subject plan does not require the expected Cargo.lock-only parent theorem")
    if proposed.get("commit_created") is not False:
        fail("replacement subject plan incorrectly claims a commit already existed")

    git(repository, "cat-file", "-e", f"{replacement_sha}^{{commit}}")
    actual_tree = full_sha(git(repository, "rev-parse", f"{replacement_sha}^{{tree}}"), label="replacement tree")
    if actual_tree != proposed_tree:
        fail("replacement commit tree differs from prepared replacement tree")

    parents = git(repository, "show", "-s", "--format=%P", replacement_sha).split()
    if parents != [subject_sha]:
        fail(f"replacement commit must have exactly the frozen subject as its sole parent; got {parents}")
    actual_parent_tree = full_sha(git(repository, "rev-parse", f"{subject_sha}^{{tree}}"), label="actual subject tree")
    if actual_parent_tree != subject_tree:
        fail("repository subject tree differs from prepared plan")

    changed = git(repository, "diff-tree", "--no-commit-id", "--name-only", "-r", subject_sha, replacement_sha)
    changed_paths = [line for line in changed.splitlines() if line]
    if changed_paths != ["Cargo.lock"]:
        fail(f"replacement commit changes paths other than Cargo.lock: {changed_paths}")

    lock_blob = full_sha(git(repository, "rev-parse", f"{replacement_sha}:Cargo.lock"), label="replacement Cargo.lock blob")
    lock_bytes = run_bytes("git", "-C", str(repository), "cat-file", "blob", lock_blob)
    lock_sha = sha256_bytes(lock_bytes)
    if lock_sha != locks.get("candidate_sha256"):
        fail("replacement Cargo.lock bytes differ from reviewed candidate")

    before_blob = full_sha(git(repository, "rev-parse", f"{subject_sha}:Cargo.lock"), label="subject Cargo.lock blob")
    before_bytes = run_bytes("git", "-C", str(repository), "cat-file", "blob", before_blob)
    if sha256_bytes(before_bytes) != locks.get("before_sha256"):
        fail("frozen subject Cargo.lock bytes differ from prepared plan")

    verification = {
        "schema": SCHEMA,
        "authority": "VerifiedReplacementSubjectIdentityOnly",
        "result": "pass",
        "plan_sha256": sha256_bytes(plan_raw),
        "subject": {"sha": subject_sha, "tree_sha": subject_tree},
        "replacement": {
            "sha": replacement_sha,
            "tree_sha": actual_tree,
            "parents": parents,
            "changed_paths": changed_paths,
        },
        "locks": {
            "before_sha256": sha256_bytes(before_bytes),
            "candidate_sha256": lock_sha,
        },
        "evidence": upstream["digests"],
        "non_claims": [
            "VerifiedReplacementSubjectIdentityOnly establishes Git/content identity, not software qualification.",
            "This verification does not authenticate or judge the human acceptance decision recorded upstream.",
            "Fresh exact-head --locked qualification is still required for the replacement commit.",
        ],
    }
    raw = canonical_bytes(verification)
    write_create_only(output, raw)
    write_create_only(pathlib.Path(str(output) + ".sha256"), (sha256_bytes(raw) + "\n").encode())
    print(json.dumps(verification, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
