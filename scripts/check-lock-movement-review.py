#!/usr/bin/env python3
"""Adversarial self-test for Cargo.lock movement review coverage v1."""
from __future__ import annotations

import hashlib
import json
import pathlib
import subprocess
import tempfile
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
PREPARE = ROOT / "scripts/prepare-lock-movement-review.py"
SEAL = ROOT / "scripts/seal-lock-movement-review.py"
VERIFY = ROOT / "scripts/verify-lock-movement-review.py"


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_canonical(path: pathlib.Path, value: dict[str, Any], *, sidecar: bool = False) -> None:
    raw = canonical_bytes(value)
    path.write_bytes(raw)
    if sidecar:
        pathlib.Path(str(path) + ".sha256").write_text(sha256_bytes(raw) + "\n", encoding="utf-8")


def run(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, capture_output=True, text=True, check=False)


def expect_success(result: subprocess.CompletedProcess[str]) -> None:
    if result.returncode != 0:
        raise SystemExit(result.stdout + result.stderr)


def expect_failure(result: subprocess.CompletedProcess[str], fragment: str) -> None:
    if result.returncode == 0:
        raise SystemExit(f"expected failure containing {fragment!r}")
    text = result.stdout + result.stderr
    if fragment not in text:
        raise SystemExit(f"expected {fragment!r}, got:\n{text}")


def fixture(root: pathlib.Path) -> pathlib.Path:
    candidate_dir = root / "candidate"
    candidate_dir.mkdir(parents=True)
    lock = b"version = 4\n"
    (candidate_dir / "Cargo.lock.candidate").write_bytes(lock)
    audit = {
        "added_local_packages": [["mycelix-new", "0.1.0"]],
        "removed_local_packages": [],
        "added_registry_packages": [["dep-a", "2.0.0", "registry+fixture"]],
        "removed_registry_packages": [["dep-a", "1.0.0", "registry+fixture"]],
        "changed_registry_versions": {"dep-a": {"before": ["1.0.0"], "after": ["2.0.0"]}},
        "added_git_packages": [],
        "removed_git_packages": [],
    }
    write_canonical(candidate_dir / "lock-audit.json", audit)
    subject = {"sha": "1" * 40, "tree_sha": "2" * 40}
    harness = {"sha": "3" * 40, "tree_sha": "4" * 40}
    candidate = {
        "schema": "mycelix-health-cargo-lock-candidate/v1",
        "authority": "RepairCandidateOnly",
        "subject": subject,
        "harness": harness,
        "locks": {"candidate_sha256": sha256_bytes(lock), "before_sha256": "5" * 64, "changed": True},
        "audit": audit,
    }
    candidate_path = candidate_dir / "lock-candidate.json"
    write_canonical(candidate_path, candidate, sidecar=True)
    candidate_raw = candidate_path.read_bytes()
    verification = {
        "schema": "mycelix-health-cargo-lock-candidate-verification/v1",
        "authority": "VerifiedRepairCandidateOnly",
        "result": "pass",
        "candidate_report_sha256": sha256_bytes(candidate_raw),
        "subject": subject,
        "harness": {"sha": harness["sha"], "tree_sha": harness["tree_sha"], "verifier_sha256": "6" * 64},
        "source_verification_sha256": "7" * 64,
        "locks": {"before_sha256": "5" * 64, "candidate_sha256": sha256_bytes(lock)},
        "environment": {},
        "required_packages": [],
        "audit": audit,
        "non_claims": [],
    }
    write_canonical(candidate_dir / "lock-candidate-verification.json", verification, sidecar=True)
    return candidate_dir


def prepare(candidate: pathlib.Path, output: pathlib.Path) -> subprocess.CompletedProcess[str]:
    return run("python3", str(PREPARE), "--candidate-dir", str(candidate), "--output", str(output))


def seal(candidate: pathlib.Path, draft: pathlib.Path, output: pathlib.Path) -> subprocess.CompletedProcess[str]:
    return run("python3", str(SEAL), "--candidate-dir", str(candidate), "--draft", str(draft), "--output", str(output))


def verify(candidate: pathlib.Path, review: pathlib.Path, output: pathlib.Path, *extra: str) -> subprocess.CompletedProcess[str]:
    return run(
        "python3", str(VERIFY),
        "--candidate-dir", str(candidate),
        "--review", str(review),
        "--verification", str(output),
        *extra,
    )


def save_draft(path: pathlib.Path, value: dict[str, Any]) -> None:
    # Human edits may be non-canonical; sealing creates canonical evidence.
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> None:
    with tempfile.TemporaryDirectory() as raw_root:
        root = pathlib.Path(raw_root)
        candidate = fixture(root / "case")
        draft = root / "review-draft.json"
        expect_success(prepare(candidate, draft))
        template = json.loads(draft.read_text(encoding="utf-8"))
        assert template["authority"] == "ReviewTemplateOnly"
        assert len(template["items"]) == 4
        assert "candidate_verification_sha256" in template

        expect_failure(seal(candidate, draft, root / "incomplete-statement.json"), "must be explicitly dispositioned")

        completed = json.loads(draft.read_text(encoding="utf-8"))
        for item in completed["items"]:
            item["disposition"] = "expected"
            item["rationale"] = "fixture movement reviewed explicitly"
            item["review_reference"] = "fixture-review"
        save_draft(draft, completed)

        statement = root / "review-statement.json"
        expect_success(seal(candidate, draft, statement))
        sealed = json.loads(statement.read_text(encoding="utf-8"))
        assert sealed["authority"] == "ReviewStatementOnly"
        complete_out = root / "complete.json"
        expect_success(verify(candidate, statement, complete_out, "--require-no-rejections"))
        result = json.loads(complete_out.read_text(encoding="utf-8"))
        assert result["authority"] == "MechanicalReviewCoverageComplete"
        assert result["all_non_rejected"] is True
        assert "candidate_verification_sha256" in result

        rejected_draft = json.loads(draft.read_text(encoding="utf-8"))
        rejected_draft["items"][0]["disposition"] = "rejected"
        rejected_draft["items"][0]["rationale"] = "fixture rejects this movement"
        rejected_path = root / "rejected-draft.json"
        save_draft(rejected_path, rejected_draft)
        rejected_statement = root / "rejected-statement.json"
        expect_success(seal(candidate, rejected_path, rejected_statement))
        reject_out = root / "reject-covered.json"
        expect_success(verify(candidate, rejected_statement, reject_out))
        result = json.loads(reject_out.read_text(encoding="utf-8"))
        assert result["complete"] is True and result["has_rejections"] is True
        expect_failure(
            verify(candidate, rejected_statement, root / "reject-fail.json", "--require-no-rejections"),
            "review contains rejected movements",
        )

        missing = json.loads(draft.read_text(encoding="utf-8"))
        missing["items"] = missing["items"][:-1]
        missing_path = root / "missing-draft.json"
        save_draft(missing_path, missing)
        expect_failure(seal(candidate, missing_path, root / "missing-statement.json"), "review draft omits")

        substituted = json.loads(draft.read_text(encoding="utf-8"))
        substituted["items"][0]["value"] = ["substituted", "9.9.9"]
        substituted_path = root / "substituted-draft.json"
        save_draft(substituted_path, substituted)
        expect_failure(
            seal(candidate, substituted_path, root / "substituted-statement.json"),
            "differs from mechanically derived movement",
        )

        # Candidate-verification substitution with a recomputed sidecar must break
        # the review binding even though the altered receipt remains canonical.
        verification_path = candidate / "lock-candidate-verification.json"
        verification = json.loads(verification_path.read_text(encoding="utf-8"))
        verification["candidate_report_sha256"] = "0" * 64
        write_canonical(verification_path, verification, sidecar=True)
        expect_failure(
            prepare(candidate, root / "verification-substitution-draft.json"),
            "does not bind the supplied candidate report",
        )

        # Restore the fixture and prove post-seal statement substitution is detected.
        candidate = fixture(root / "case-restored")
        draft = root / "review-restored.json"
        expect_success(prepare(candidate, draft))
        completed = json.loads(draft.read_text(encoding="utf-8"))
        for item in completed["items"]:
            item["disposition"] = "expected"
            item["rationale"] = "fixture movement reviewed explicitly"
            item["review_reference"] = "fixture-review"
        save_draft(draft, completed)
        statement = root / "statement-restored.json"
        expect_success(seal(candidate, draft, statement))
        tampered = json.loads(statement.read_text(encoding="utf-8"))
        tampered["items"][0]["value"] = ["post-seal-substitution", "8.8.8"]
        write_canonical(statement, tampered, sidecar=True)
        expect_failure(
            verify(candidate, statement, root / "tampered-verification.json"),
            "differs from mechanically derived movement",
        )

    print("lock movement review adversarial checks: PASS")


if __name__ == "__main__":
    main()
