#!/usr/bin/env python3
"""Adversarial self-test for deterministic lock-bearing replacement subjects."""
from __future__ import annotations

import hashlib
import json
import pathlib
import subprocess
import tempfile
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
PREPARE = ROOT / "scripts/prepare-lock-bearing-subject.py"
VERIFY = ROOT / "scripts/verify-lock-bearing-subject.py"


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_canonical(path: pathlib.Path, value: dict[str, Any], *, sidecar: bool = True) -> None:
    raw = canonical_bytes(value)
    path.write_bytes(raw)
    if sidecar:
        pathlib.Path(str(path) + ".sha256").write_text(sha256_bytes(raw) + "\n", encoding="utf-8")


def run(*args: str, cwd: pathlib.Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, cwd=cwd, capture_output=True, text=True, check=False)


def must(*args: str, cwd: pathlib.Path | None = None) -> str:
    result = run(*args, cwd=cwd)
    if result.returncode != 0:
        raise SystemExit(result.stdout + result.stderr)
    return result.stdout.strip()


def expect_success(result: subprocess.CompletedProcess[str]) -> None:
    if result.returncode != 0:
        raise SystemExit(result.stdout + result.stderr)


def expect_failure(result: subprocess.CompletedProcess[str], fragment: str) -> None:
    if result.returncode == 0:
        raise SystemExit(f"expected failure containing {fragment!r}")
    text = result.stdout + result.stderr
    if fragment not in text:
        raise SystemExit(f"expected {fragment!r}, got:\n{text}")


def git(repo: pathlib.Path, *args: str) -> str:
    return must("git", "-C", str(repo), *args)


def build_fixture(root: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path, str, bytes]:
    repo = root / "repo"
    repo.mkdir(parents=True)
    must("git", "init", "-q", str(repo))
    git(repo, "config", "user.name", "Fixture")
    git(repo, "config", "user.email", "fixture@example.invalid")
    before_lock = b"version = 4\n# frozen fixture lock\n"
    candidate_lock = b"version = 4\n# reviewed replacement fixture lock\n"
    (repo / "Cargo.lock").write_bytes(before_lock)
    (repo / "README.md").write_text("fixture\n", encoding="utf-8")
    git(repo, "add", "Cargo.lock", "README.md")
    git(repo, "commit", "-q", "-m", "fixture subject")
    subject_sha = git(repo, "rev-parse", "HEAD")
    subject_tree = git(repo, "rev-parse", "HEAD^{tree}")

    candidate_dir = root / "candidate"
    candidate_dir.mkdir()
    (candidate_dir / "Cargo.lock.candidate").write_bytes(candidate_lock)

    subject = {"sha": subject_sha, "tree_sha": subject_tree}
    candidate = {
        "schema": "mycelix-health-cargo-lock-candidate/v1",
        "authority": "RepairCandidateOnly",
        "subject": subject,
        "harness": {"sha": "1" * 40, "tree_sha": "2" * 40},
        "locks": {
            "before_sha256": sha256_bytes(before_lock),
            "candidate_sha256": sha256_bytes(candidate_lock),
            "changed": True,
        },
        "audit": {
            "added_local_packages": [],
            "removed_local_packages": [],
            "added_registry_packages": [],
            "removed_registry_packages": [],
            "changed_registry_versions": {},
            "added_git_packages": [],
            "removed_git_packages": [],
        },
    }
    candidate_raw = canonical_bytes(candidate)
    write_canonical(candidate_dir / "lock-candidate.json", candidate)

    candidate_verification = {
        "schema": "mycelix-health-cargo-lock-candidate-verification/v1",
        "authority": "VerifiedRepairCandidateOnly",
        "result": "pass",
        "candidate_report_sha256": sha256_bytes(candidate_raw),
        "subject": subject,
        "harness": {"sha": "1" * 40, "tree_sha": "2" * 40},
        "locks": {"before_sha256": sha256_bytes(before_lock), "candidate_sha256": sha256_bytes(candidate_lock)},
        "required_packages": ["fixture-package"],
        "audit": candidate["audit"],
    }
    candidate_verification_raw = canonical_bytes(candidate_verification)
    write_canonical(candidate_dir / "lock-candidate-verification.json", candidate_verification)

    review_statement = {
        "schema": "mycelix-health-lock-movement-review-statement/v1",
        "authority": "ReviewStatementOnly",
        "candidate_report_sha256": sha256_bytes(candidate_raw),
        "candidate_verification_sha256": sha256_bytes(candidate_verification_raw),
        "candidate_lock_sha256": sha256_bytes(candidate_lock),
        "audit_sha256": sha256_bytes(canonical_bytes(candidate["audit"])),
        "subject": subject,
        "harness": candidate["harness"],
        "items": [],
        "review_reference_authentication": "UnverifiedContextOnly",
    }
    review_statement_raw = canonical_bytes(review_statement)
    write_canonical(candidate_dir / "lock-movement-review-statement.json", review_statement)

    review_verification = {
        "schema": "mycelix-health-lock-movement-review-verification/v1",
        "authority": "MechanicalReviewCoverageComplete",
        "candidate_report_sha256": sha256_bytes(candidate_raw),
        "candidate_verification_sha256": sha256_bytes(candidate_verification_raw),
        "candidate_lock_sha256": sha256_bytes(candidate_lock),
        "audit_sha256": review_statement["audit_sha256"],
        "review_statement_sha256": sha256_bytes(review_statement_raw),
        "subject": subject,
        "harness": candidate["harness"],
        "movement_count": 0,
        "disposition_counts": {"accepted": 0, "expected": 0, "necessary": 0, "rejected": 0},
        "complete": True,
        "has_rejections": False,
        "all_non_rejected": True,
    }
    write_canonical(candidate_dir / "lock-movement-review-verification.json", review_verification)
    return repo, candidate_dir, subject_sha, candidate_lock


def prepare(repo: pathlib.Path, candidate_dir: pathlib.Path, plan: pathlib.Path) -> subprocess.CompletedProcess[str]:
    return run(
        "python3", str(PREPARE),
        "--subject-checkout", str(repo),
        "--candidate-dir", str(candidate_dir),
        "--output", str(plan),
    )


def verify(repo: pathlib.Path, candidate_dir: pathlib.Path, replacement: str, plan: pathlib.Path, output: pathlib.Path) -> subprocess.CompletedProcess[str]:
    return run(
        "python3", str(VERIFY),
        "--repository", str(repo),
        "--replacement-sha", replacement,
        "--plan", str(plan),
        "--candidate-dir", str(candidate_dir),
        "--verification", str(output),
    )


def main() -> None:
    with tempfile.TemporaryDirectory() as raw_root:
        root = pathlib.Path(raw_root)
        repo, candidate_dir, subject_sha, candidate_lock = build_fixture(root)
        plan = root / "replacement-plan.json"
        expect_success(prepare(repo, candidate_dir, plan))
        plan_value = json.loads(plan.read_text(encoding="utf-8"))
        assert plan_value["authority"] == "PreparedReplacementSubjectOnly"
        assert plan_value["proposed_replacement"]["commit_created"] is False
        proposed_tree = plan_value["proposed_replacement"]["tree_sha"]

        # Human creates exactly the reviewed Cargo.lock-only child commit.
        (repo / "Cargo.lock").write_bytes(candidate_lock)
        git(repo, "add", "Cargo.lock")
        git(repo, "commit", "-q", "-m", "fixture reviewed lock")
        replacement_sha = git(repo, "rev-parse", "HEAD")
        good_out = root / "replacement-verification.json"
        expect_success(verify(repo, candidate_dir, replacement_sha, plan, good_out))
        good = json.loads(good_out.read_text(encoding="utf-8"))
        assert good["authority"] == "VerifiedReplacementSubjectIdentityOnly"
        assert good["replacement"]["tree_sha"] == proposed_tree
        assert good["replacement"]["parents"] == [subject_sha]
        assert good["replacement"]["changed_paths"] == ["Cargo.lock"]

        # Same reviewed tree but wrong/no parent must fail.
        wrong_parent = must("git", "-C", str(repo), "commit-tree", proposed_tree, "-m", "wrong parent")
        expect_failure(
            verify(repo, candidate_dir, wrong_parent, plan, root / "wrong-parent.json"),
            "sole parent",
        )

        # Extra file movement changes the tree and is rejected before qualification.
        git(repo, "checkout", "-q", "--detach", subject_sha)
        (repo / "Cargo.lock").write_bytes(candidate_lock)
        (repo / "README.md").write_text("unexpected change\n", encoding="utf-8")
        git(repo, "add", "Cargo.lock", "README.md")
        git(repo, "commit", "-q", "-m", "extra path")
        extra_sha = git(repo, "rev-parse", "HEAD")
        expect_failure(
            verify(repo, candidate_dir, extra_sha, plan, root / "extra-path.json"),
            "tree differs",
        )

        # Wrong Cargo.lock bytes also produce the wrong prepared tree.
        git(repo, "checkout", "-q", "--detach", subject_sha)
        (repo / "Cargo.lock").write_text("version = 4\n# wrong bytes\n", encoding="utf-8")
        git(repo, "add", "Cargo.lock")
        git(repo, "commit", "-q", "-m", "wrong lock")
        wrong_lock_sha = git(repo, "rev-parse", "HEAD")
        expect_failure(
            verify(repo, candidate_dir, wrong_lock_sha, plan, root / "wrong-lock.json"),
            "tree differs",
        )

        # A plan whose upstream evidence digest is substituted fails even with a fresh sidecar.
        tampered_plan = json.loads(plan.read_text(encoding="utf-8"))
        tampered_plan["evidence"]["candidate_verification_sha256"] = "0" * 64
        write_canonical(plan, tampered_plan)
        expect_failure(
            verify(repo, candidate_dir, replacement_sha, plan, root / "tampered-plan.json"),
            "evidence bindings differ",
        )

        # Restore plan and substitute the independent candidate verification itself.
        expect_success(prepare(repo, candidate_dir, root / "fresh-plan.json"))
        fresh_plan = root / "fresh-plan.json"
        verification_path = candidate_dir / "lock-candidate-verification.json"
        verification = json.loads(verification_path.read_text(encoding="utf-8"))
        verification["candidate_report_sha256"] = "f" * 64
        write_canonical(verification_path, verification)
        expect_failure(
            verify(repo, candidate_dir, replacement_sha, fresh_plan, root / "substituted-upstream.json"),
            "does not bind candidate report",
        )

    print("lock-bearing replacement subject adversarial checks: PASS")


if __name__ == "__main__":
    main()
