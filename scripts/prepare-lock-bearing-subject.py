#!/usr/bin/env python3
"""Prepare a deterministic lock-bearing replacement subject without committing it.

The program consumes the complete portable lock-repair, explicit-review, and signed
acceptance evidence chain, reconstructs the exact frozen product subject in a
disposable worktree, installs only the reviewed Cargo.lock candidate, and computes
the exact proposed Git tree with a temporary index. It never creates a commit,
moves a branch, or grants qualification authority.

Terminal authority: PreparedReplacementSubjectOnly.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCHEMA = "mycelix-health-lock-bearing-subject-plan/v1"
HEX = frozenset("0123456789abcdefABCDEF")


def fail(message: str) -> NoReturn:
    raise SystemExit(f"replacement subject preparation failed: {message}")


def run(*args: str, cwd: pathlib.Path | None = None, env: dict[str, str] | None = None) -> str:
    try:
        result = subprocess.run(args, cwd=cwd, env=env, check=True, capture_output=True, text=True)
    except (OSError, subprocess.CalledProcessError) as error:
        detail = ""
        if isinstance(error, subprocess.CalledProcessError):
            detail = f"\nstdout:\n{error.stdout or ''}\nstderr:\n{error.stderr or ''}"
        fail(f"command failed: {' '.join(args)}{detail}")
    return result.stdout.strip()


def git(repo: pathlib.Path, *args: str, env: dict[str, str] | None = None) -> str:
    return run("git", "-C", str(repo), *args, env=env)


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    try:
        return sha256_bytes(path.read_bytes())
    except OSError as error:
        fail(f"cannot hash {path}: {error}")


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
        fail(f"refusing to overwrite artifact: {path}")
    try:
        with path.open("xb") as handle:
            handle.write(data)
    except FileExistsError:
        fail(f"refusing to overwrite artifact: {path}")


def harness_identity(value: Any, *, label: str) -> tuple[str, str]:
    if not isinstance(value, dict):
        fail(f"{label} harness identity is malformed")
    return (
        full_sha(str(value.get("sha", "")), label=f"{label} harness SHA"),
        full_sha(str(value.get("tree_sha", "")), label=f"{label} harness tree"),
    )


def verify_evidence_chain(candidate_dir: pathlib.Path) -> dict[str, Any]:
    candidate_path = candidate_dir / "lock-candidate.json"
    candidate_verification_path = candidate_dir / "lock-candidate-verification.json"
    review_statement_path = candidate_dir / "lock-movement-review-statement.json"
    review_verification_path = candidate_dir / "lock-movement-review-verification.json"
    acceptance_statement_path = candidate_dir / "lock-acceptance-statement.json"
    acceptance_verification_path = candidate_dir / "lock-acceptance-verification.json"

    candidate, candidate_raw = load_canonical(candidate_path, label="candidate report")
    candidate_verification, candidate_verification_raw = load_canonical(
        candidate_verification_path, label="candidate verification"
    )
    review_statement, review_statement_raw = load_canonical(review_statement_path, label="review statement")
    review_verification, review_verification_raw = load_canonical(
        review_verification_path, label="review verification"
    )
    acceptance_statement, acceptance_statement_raw = load_canonical(
        acceptance_statement_path, label="acceptance statement"
    )
    acceptance_verification, acceptance_verification_raw = load_canonical(
        acceptance_verification_path, label="acceptance verification"
    )
    for path, raw, label in (
        (candidate_path, candidate_raw, "candidate report"),
        (candidate_verification_path, candidate_verification_raw, "candidate verification"),
        (review_statement_path, review_statement_raw, "review statement"),
        (review_verification_path, review_verification_raw, "review verification"),
        (acceptance_statement_path, acceptance_statement_raw, "acceptance statement"),
        (acceptance_verification_path, acceptance_verification_raw, "acceptance verification"),
    ):
        require_sidecar(path, raw, label=label)

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
    if acceptance_statement.get("schema") != "mycelix-health-lock-acceptance-statement/v1" or acceptance_statement.get("authority") != "AcceptanceStatementToSignOnly":
        fail("unsupported acceptance statement schema or authority")
    if acceptance_verification.get("schema") != "mycelix-health-lock-acceptance-verification/v1":
        fail("unsupported acceptance verification schema")
    if acceptance_verification.get("authority") != "VerifiedHumanAcceptanceAttestationOnly" or acceptance_verification.get("result") != "pass":
        fail("acceptance verification must be successful VerifiedHumanAcceptanceAttestationOnly")

    candidate_sha = sha256_bytes(candidate_raw)
    candidate_verification_sha = sha256_bytes(candidate_verification_raw)
    review_statement_sha = sha256_bytes(review_statement_raw)
    review_verification_sha = sha256_bytes(review_verification_raw)
    acceptance_statement_sha = sha256_bytes(acceptance_statement_raw)

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
    if review_verification.get("complete") is not True:
        fail("review verification is not complete")
    if review_verification.get("has_rejections") is not False or review_verification.get("all_non_rejected") is not True:
        fail("review contains one or more rejected dependency movements")

    expected_acceptance_evidence = {
        "candidate_report_sha256": candidate_sha,
        "candidate_verification_sha256": candidate_verification_sha,
        "review_statement_sha256": review_statement_sha,
        "review_verification_sha256": review_verification_sha,
    }
    if acceptance_statement.get("evidence") != expected_acceptance_evidence:
        fail("acceptance statement evidence bindings differ from reviewed dependency evidence")
    if acceptance_verification.get("statement_sha256") != acceptance_statement_sha:
        fail("acceptance verification does not bind acceptance statement")
    if acceptance_verification.get("evidence") != expected_acceptance_evidence:
        fail("acceptance verification evidence bindings differ from acceptance statement")
    if acceptance_verification.get("principal") != acceptance_statement.get("principal"):
        fail("acceptance principal differs between signed statement and verification")
    if acceptance_verification.get("signature_namespace") != "mycelix-health-lock-acceptance-v1":
        fail("acceptance verification namespace mismatch")

    candidate_locks = candidate.get("locks")
    candidate_verification_locks = candidate_verification.get("locks")
    if not isinstance(candidate_locks, dict) or not isinstance(candidate_verification_locks, dict):
        fail("candidate lock identities are malformed")
    before_lock_sha = candidate_locks.get("before_sha256")
    if candidate_verification_locks.get("before_sha256") != before_lock_sha:
        fail("candidate verification before-lock digest differs from candidate report")

    candidate_lock = candidate_dir / "Cargo.lock.candidate"
    if not candidate_lock.is_file():
        fail("candidate directory is missing Cargo.lock.candidate")
    candidate_lock_sha = sha256_file(candidate_lock)
    for value, label in (
        (candidate_locks.get("candidate_sha256"), "candidate report"),
        (candidate_verification_locks.get("candidate_sha256"), "candidate verification"),
        (review_statement.get("candidate_lock_sha256"), "review statement"),
        (review_verification.get("candidate_lock_sha256"), "review verification"),
        ((acceptance_statement.get("locks") or {}).get("candidate_sha256"), "acceptance statement"),
        ((acceptance_verification.get("locks") or {}).get("candidate_sha256"), "acceptance verification"),
    ):
        if value != candidate_lock_sha:
            fail(f"{label} does not bind Cargo.lock.candidate")
    for value, label in (
        ((acceptance_statement.get("locks") or {}).get("before_sha256"), "acceptance statement"),
        ((acceptance_verification.get("locks") or {}).get("before_sha256"), "acceptance verification"),
    ):
        if value != before_lock_sha:
            fail(f"{label} before-lock digest differs from candidate evidence")

    subject = candidate.get("subject")
    if not isinstance(subject, dict):
        fail("candidate subject identity is malformed")
    subject_sha = full_sha(str(subject.get("sha", "")), label="subject SHA")
    subject_tree = full_sha(str(subject.get("tree_sha", "")), label="subject tree")
    for value, label in (
        (candidate_verification.get("subject"), "candidate verification"),
        (review_statement.get("subject"), "review statement"),
        (review_verification.get("subject"), "review verification"),
        (acceptance_statement.get("subject"), "acceptance statement"),
        (acceptance_verification.get("subject"), "acceptance verification"),
    ):
        if value != subject:
            fail(f"{label} subject identity differs from candidate")

    candidate_harness = harness_identity(candidate.get("harness"), label="candidate")
    candidate_verification_harness = harness_identity(candidate_verification.get("harness"), label="candidate verification")
    if candidate_verification_harness != candidate_harness:
        fail("candidate verification harness SHA/tree differs from candidate report")
    for value, label in (
        (review_statement.get("harness"), "review statement"),
        (review_verification.get("harness"), "review verification"),
        (acceptance_statement.get("harness"), "acceptance statement"),
        (acceptance_verification.get("harness"), "acceptance verification"),
    ):
        if value != candidate.get("harness"):
            fail(f"{label} harness identity differs from candidate")

    return {
        "candidate": candidate,
        "candidate_raw": candidate_raw,
        "candidate_verification_raw": candidate_verification_raw,
        "review_statement_raw": review_statement_raw,
        "review_verification_raw": review_verification_raw,
        "acceptance_statement_raw": acceptance_statement_raw,
        "acceptance_verification_raw": acceptance_verification_raw,
        "subject_sha": subject_sha,
        "subject_tree": subject_tree,
        "candidate_lock": candidate_lock,
        "candidate_lock_sha256": candidate_lock_sha,
        "before_lock_sha256": before_lock_sha,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--subject-checkout", type=pathlib.Path, required=True)
    parser.add_argument("--candidate-dir", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()

    subject_checkout = args.subject_checkout.resolve()
    candidate_dir = args.candidate_dir.resolve()
    output = args.output.resolve()
    harness = ROOT.resolve()

    if not subject_checkout.is_dir():
        fail(f"subject checkout does not exist: {subject_checkout}")
    if not candidate_dir.is_dir():
        fail(f"candidate directory does not exist: {candidate_dir}")
    if output.exists() or output.is_symlink():
        fail(f"output already exists: {output}")
    if git(harness, "status", "--porcelain=v1", "--untracked-files=all"):
        fail("replacement-subject preparation harness checkout must be fully clean")

    chain = verify_evidence_chain(candidate_dir)
    subject_sha = chain["subject_sha"]
    subject_tree = chain["subject_tree"]
    git(subject_checkout, "cat-file", "-e", f"{subject_sha}^{{commit}}")
    actual_subject_tree = full_sha(git(subject_checkout, "rev-parse", f"{subject_sha}^{{tree}}"), label="actual subject tree")
    if actual_subject_tree != subject_tree:
        fail("candidate subject tree differs from repository object identity")

    harness_sha = full_sha(git(harness, "rev-parse", "HEAD"), label="preparation harness SHA")
    harness_tree = full_sha(git(harness, "rev-parse", "HEAD^{tree}"), label="preparation harness tree")

    temporary_root = pathlib.Path(tempfile.mkdtemp(prefix="mycelix-health-lock-bearing-subject-"))
    worktree = temporary_root / "health"
    index_path = temporary_root / "replacement.index"
    worktree_registered = False
    try:
        run("git", "-C", str(subject_checkout), "worktree", "add", "--detach", str(worktree), subject_sha)
        worktree_registered = True
        if git(worktree, "rev-parse", "HEAD") != subject_sha:
            fail("detached worktree HEAD differs from reviewed subject")
        if git(worktree, "rev-parse", "HEAD^{tree}") != subject_tree:
            fail("detached worktree tree differs from reviewed subject")
        if git(worktree, "status", "--porcelain=v1", "--untracked-files=all"):
            fail("detached subject worktree is not clean before candidate installation")

        target_lock = worktree / "Cargo.lock"
        before_sha = sha256_file(target_lock)
        if before_sha != chain["before_lock_sha256"]:
            fail("frozen subject Cargo.lock differs from reviewed before-lock digest")
        shutil.copyfile(chain["candidate_lock"], target_lock)
        if sha256_file(target_lock) != chain["candidate_lock_sha256"]:
            fail("installed candidate Cargo.lock bytes differ after copy")

        status = git(worktree, "status", "--porcelain=v1", "--untracked-files=all")
        status_lines = [line for line in status.splitlines() if line]
        if status_lines != [" M Cargo.lock"]:
            fail(f"prepared subject changed paths other than Cargo.lock: {status_lines}")
        diff = git(worktree, "diff", "--", "Cargo.lock")
        if not diff:
            fail("candidate Cargo.lock does not change the frozen subject")

        env = os.environ.copy()
        env["GIT_INDEX_FILE"] = str(index_path)
        run("git", "-C", str(worktree), "read-tree", subject_tree, env=env)
        run("git", "-C", str(worktree), "add", "--", "Cargo.lock", env=env)
        proposed_tree = full_sha(run("git", "-C", str(worktree), "write-tree", env=env), label="proposed replacement tree")
        changed_names = run("git", "-C", str(worktree), "diff-tree", "--no-commit-id", "--name-only", "-r", subject_tree, proposed_tree)
        names = [line for line in changed_names.splitlines() if line]
        if names != ["Cargo.lock"]:
            fail(f"proposed replacement tree changes unexpected paths: {names}")

        receipt = {
            "schema": SCHEMA,
            "authority": "PreparedReplacementSubjectOnly",
            "subject": {"sha": subject_sha, "tree_sha": subject_tree},
            "proposed_replacement": {
                "tree_sha": proposed_tree,
                "changed_paths": ["Cargo.lock"],
                "parent_must_equal_subject_sha": True,
                "commit_created": False,
            },
            "locks": {
                "before_sha256": before_sha,
                "candidate_sha256": chain["candidate_lock_sha256"],
            },
            "evidence": {
                "candidate_report_sha256": sha256_bytes(chain["candidate_raw"]),
                "candidate_verification_sha256": sha256_bytes(chain["candidate_verification_raw"]),
                "review_statement_sha256": sha256_bytes(chain["review_statement_raw"]),
                "review_verification_sha256": sha256_bytes(chain["review_verification_raw"]),
                "acceptance_statement_sha256": sha256_bytes(chain["acceptance_statement_raw"]),
                "acceptance_verification_sha256": sha256_bytes(chain["acceptance_verification_raw"]),
            },
            "acceptance": {
                "principal": json.loads(chain["acceptance_verification_raw"])["principal"],
                "signature_namespace": "mycelix-health-lock-acceptance-v1",
            },
            "preparation_harness": {
                "sha": harness_sha,
                "tree_sha": harness_tree,
                "program_sha256": sha256_file(pathlib.Path(__file__).resolve()),
            },
            "non_claims": [
                "PreparedReplacementSubjectOnly is not a commit and does not grant repository qualification.",
                "The verified human acceptance attestation authenticates only the configured OpenSSH principal under the supplied allowed_signers policy.",
                "The proposed replacement tree is not scientific or clinical evidence.",
                "A human-created replacement commit must be identity-verified and receive fresh exact-head --locked qualification.",
            ],
        }
        raw = canonical_bytes(receipt)
        write_create_only(output, raw)
        write_create_only(pathlib.Path(str(output) + ".sha256"), (sha256_bytes(raw) + "\n").encode())
        print(json.dumps(receipt, indent=2, sort_keys=True))
    finally:
        if worktree_registered:
            subprocess.run(
                ["git", "-C", str(subject_checkout), "worktree", "remove", "--force", str(worktree)],
                check=False,
                capture_output=True,
                text=True,
            )
            subprocess.run(
                ["git", "-C", str(subject_checkout), "worktree", "prune"],
                check=False,
                capture_output=True,
                text=True,
            )
        shutil.rmtree(temporary_root, ignore_errors=True)


if __name__ == "__main__":
    main()
