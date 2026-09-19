#!/usr/bin/env python3
"""Adversarial self-test for canonical repository CI evidence receipts v1."""
from __future__ import annotations

import hashlib
import json
import os
import pathlib
import subprocess
import tempfile
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
BUILDER = ROOT / "scripts/build-ci-evidence-receipt.py"
FINAL_VERIFIER = ROOT / "scripts/verify-ci-evidence-receipt.py"
PIN = "1" * 40
REPOSITORY = "Luminous-Dynamics/mycelix-health"
WORKFLOW = ".github/workflows/test.yml"


def command(*args: str, cwd: pathlib.Path | None = None, env: dict[str, str] | None = None, check: bool = True):
    return subprocess.run(args, cwd=cwd, env=env, check=check, capture_output=True, text=True)


def git(repo: pathlib.Path, *args: str) -> str:
    return command("git", *args, cwd=repo).stdout.strip()


def init_repo(repo: pathlib.Path, *, name: str) -> None:
    repo.mkdir(parents=True)
    git(repo, "init")
    git(repo, "config", "user.email", "ci-receipt-test@example.invalid")
    git(repo, "config", "user.name", name)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    return sha256_bytes(path.read_bytes())


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def workflow_fixture(*, checkout_pin: str = PIN) -> str:
    return f"""name: fixture
on: workflow_dispatch
jobs:
  fixture:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@{checkout_pin}
      - uses: cachix/install-nix-action@{PIN}
      - uses: actions/upload-artifact@{PIN}
"""


def setup_case(root: pathlib.Path, name: str, *, source_authority: str, checkout_pin: str = PIN):
    case = root / name
    workspace = case / "workspace"
    harness = case / "harness"
    evidence = case / "evidence"
    evidence.mkdir(parents=True)
    init_repo(workspace, name="CI Product Fixture")
    init_repo(harness, name="CI Harness Fixture")

    product_files = {
        "Cargo.lock": "# frozen lock\n",
        "flake.lock": "{}\n",
        "rust-toolchain.toml": "[toolchain]\nchannel='1.96.0'\n",
        "release/ci-parent-source-profile-v1.json": "{}\n",
    }
    for relative, content in product_files.items():
        target = workspace / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
    git(workspace, "add", ".")
    git(workspace, "commit", "-m", "fixture product subject")
    subject = git(workspace, "rev-parse", "HEAD")
    subject_tree = git(workspace, "rev-parse", "HEAD^{tree}")

    harness_files = {
        WORKFLOW: workflow_fixture(checkout_pin=checkout_pin),
        "scripts/materialize-parent-mycelix.py": "# materializer fixture\n",
        "scripts/verify-parent-source-materialization.py": "# source verifier fixture\n",
        "scripts/build-ci-evidence-receipt.py": BUILDER.read_text(encoding="utf-8"),
        "scripts/verify-ci-evidence-receipt.py": FINAL_VERIFIER.read_text(encoding="utf-8"),
    }
    for relative, content in harness_files.items():
        target = harness / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
    git(harness, "add", ".")
    git(harness, "commit", "-m", "fixture CI harness")
    harness_sha = git(harness, "rev-parse", "HEAD")
    harness_tree = git(harness, "rev-parse", "HEAD^{tree}")

    profile = workspace / "release/ci-parent-source-profile-v1.json"
    materializer = harness / "scripts/materialize-parent-mycelix.py"
    source_verifier = harness / "scripts/verify-parent-source-materialization.py"
    source_verification = {
        "schema": "mycelix-health-parent-source-verification/v1",
        "authority": source_authority,
        "mode": "pinned" if source_authority.startswith("VerifiedPinned") else "compatibility",
        "source_receipt_sha256": "2" * 64,
        "profile_sha256": sha256_file(profile),
        "materializer_sha256": sha256_file(materializer),
        "verifier_sha256": sha256_file(source_verifier),
        "observed_source": {
            "repository": "Luminous-Dynamics/mycelix",
            "commit_sha": "6" * 40,
            "tree_sha": "7" * 40,
            "clean_checkout": True,
        },
        "exports": [{"source": "crates", "destination": "../crates", "content_sha256": "8" * 64}],
        "non_claims": [],
    }
    raw = canonical_bytes(source_verification)
    source_path = evidence / "parent-source-verification.json"
    source_path.write_bytes(raw)
    pathlib.Path(str(source_path) + ".sha256").write_text(sha256_bytes(raw) + "\n", encoding="utf-8")

    for filename, data in (
        ("cargo-metadata.json", "{}\n"),
        ("flake-metadata.json", "{}\n"),
        ("ci-dev-shell.drv", "/nix/store/fixture-ci-shell.drv\n"),
    ):
        (evidence / filename).write_text(data, encoding="utf-8")
    for filename, sidecar in (
        (workspace / "Cargo.lock", evidence / "Cargo.lock.sha256"),
        (workspace / "flake.lock", evidence / "flake.lock.sha256"),
        (workspace / "rust-toolchain.toml", evidence / "rust-toolchain.toml.sha256"),
    ):
        sidecar.write_text(sha256_file(filename) + "  " + filename.name + "\n", encoding="utf-8")

    return workspace, harness, evidence, subject, subject_tree, harness_sha, harness_tree


def github_env(harness_sha: str, *, workflow: str = WORKFLOW) -> dict[str, str]:
    env = os.environ.copy()
    env.update(
        {
            "GITHUB_ACTIONS": "true",
            "GITHUB_REPOSITORY": REPOSITORY,
            "GITHUB_EVENT_NAME": "workflow_dispatch",
            "GITHUB_WORKFLOW_SHA": harness_sha,
            "GITHUB_WORKFLOW_REF": f"{REPOSITORY}/{workflow}@refs/heads/harness",
            "GITHUB_RUN_ID": "12345",
            "GITHUB_RUN_ATTEMPT": "2",
            "RUNNER_OS": "Linux",
            "RUNNER_ARCH": "X64",
            "ImageOS": "ubuntu24",
            "ImageVersion": "fixture",
        }
    )
    return env


def run_builder(
    workspace: pathlib.Path,
    harness: pathlib.Path,
    evidence: pathlib.Path,
    subject: str,
    harness_sha: str,
    *,
    mode: str,
    checkout_pin: str = PIN,
    environment: dict[str, str] | None = None,
):
    return command(
        "python3",
        str(BUILDER),
        "--mode",
        mode,
        "--subject-sha",
        subject,
        "--harness-sha",
        harness_sha,
        "--workspace",
        str(workspace),
        "--harness-root",
        str(harness),
        "--evidence-dir",
        str(evidence),
        "--workflow",
        WORKFLOW,
        "--checkout-action-sha",
        checkout_pin,
        "--install-nix-action-sha",
        PIN,
        "--upload-artifact-action-sha",
        PIN,
        "--output",
        str(evidence / "ci-evidence.json"),
        env=environment or github_env(harness_sha),
        check=False,
    )


def expect_success(result: subprocess.CompletedProcess[str]) -> None:
    if result.returncode != 0:
        raise SystemExit(result.stdout + result.stderr)


def expect_failure(result: subprocess.CompletedProcess[str], fragment: str) -> None:
    if result.returncode == 0:
        raise SystemExit(f"expected failure containing {fragment!r}")
    combined = result.stdout + result.stderr
    if fragment not in combined:
        raise SystemExit(f"expected failure containing {fragment!r}, got:\n{combined}")


def main() -> None:
    with tempfile.TemporaryDirectory() as raw:
        root = pathlib.Path(raw)

        workspace, harness, evidence, subject, subject_tree, harness_sha, harness_tree = setup_case(
            root, "pinned-success", source_authority="VerifiedPinnedDependencySourceMaterialization"
        )
        expect_success(run_builder(workspace, harness, evidence, subject, harness_sha, mode="pinned"))
        value = json.loads((evidence / "ci-evidence.json").read_text())
        assert value["authority"] == "ReproducibleRepositoryCIEvidence"
        assert value["subject"] == {"sha": subject, "tree_sha": subject_tree}
        assert value["harness"]["sha"] == harness_sha
        assert value["harness"]["tree_sha"] == harness_tree
        assert value["harness"]["receipt_verifier_sha256"] == sha256_file(
            harness / "scripts/verify-ci-evidence-receipt.py"
        )

        workspace, harness, evidence, subject, _, harness_sha, _ = setup_case(
            root, "compatibility-success", source_authority="VerifiedCompatibilityObservation"
        )
        expect_success(run_builder(workspace, harness, evidence, subject, harness_sha, mode="compatibility"))
        value = json.loads((evidence / "ci-evidence.json").read_text())
        assert value["authority"] == "CompatibilityOnly"

        workspace, harness, evidence, subject, _, harness_sha, _ = setup_case(
            root, "subject-mismatch", source_authority="VerifiedPinnedDependencySourceMaterialization"
        )
        expect_failure(run_builder(workspace, harness, evidence, "0" * 40, harness_sha, mode="pinned"), "Health HEAD")

        workspace, harness, evidence, subject, _, harness_sha, _ = setup_case(
            root, "harness-mismatch", source_authority="VerifiedPinnedDependencySourceMaterialization"
        )
        expect_failure(run_builder(workspace, harness, evidence, subject, "0" * 40, mode="pinned"), "harness HEAD")

        workspace, harness, evidence, subject, _, harness_sha, _ = setup_case(
            root, "product-drift", source_authority="VerifiedPinnedDependencySourceMaterialization"
        )
        (workspace / "Cargo.lock").write_text("# drift\n", encoding="utf-8")
        expect_failure(run_builder(workspace, harness, evidence, subject, harness_sha, mode="pinned"), "tracked/index drift")

        workspace, harness, evidence, subject, _, harness_sha, _ = setup_case(
            root, "harness-dirty", source_authority="VerifiedPinnedDependencySourceMaterialization"
        )
        (harness / "untracked.txt").write_text("dirty\n", encoding="utf-8")
        expect_failure(run_builder(workspace, harness, evidence, subject, harness_sha, mode="pinned"), "not fully clean")

        workspace, harness, evidence, subject, _, harness_sha, _ = setup_case(
            root, "workflow-sha-mismatch", source_authority="VerifiedPinnedDependencySourceMaterialization"
        )
        expect_failure(
            run_builder(workspace, harness, evidence, subject, harness_sha, mode="pinned", environment=github_env("0" * 40)),
            "workflow SHA",
        )

        workspace, harness, evidence, subject, _, harness_sha, _ = setup_case(
            root, "workflow-ref-mismatch", source_authority="VerifiedPinnedDependencySourceMaterialization"
        )
        expect_failure(
            run_builder(
                workspace,
                harness,
                evidence,
                subject,
                harness_sha,
                mode="pinned",
                environment=github_env(harness_sha, workflow=".github/workflows/other.yml"),
            ),
            "workflow ref",
        )

        workspace, harness, evidence, subject, _, harness_sha, _ = setup_case(
            root, "authority-mismatch", source_authority="VerifiedCompatibilityObservation"
        )
        expect_failure(run_builder(workspace, harness, evidence, subject, harness_sha, mode="pinned"), "authority mismatch")

        workspace, harness, evidence, subject, _, harness_sha, _ = setup_case(
            root, "source-profile-mismatch", source_authority="VerifiedPinnedDependencySourceMaterialization"
        )
        source = evidence / "parent-source-verification.json"
        source_value = json.loads(source.read_text())
        source_value["profile_sha256"] = "0" * 64
        raw_source = canonical_bytes(source_value)
        source.write_bytes(raw_source)
        pathlib.Path(str(source) + ".sha256").write_text(sha256_bytes(raw_source) + "\n", encoding="utf-8")
        expect_failure(run_builder(workspace, harness, evidence, subject, harness_sha, mode="pinned"), "profile digest")

        workspace, harness, evidence, subject, _, harness_sha, _ = setup_case(
            root, "source-sidecar-tamper", source_authority="VerifiedPinnedDependencySourceMaterialization"
        )
        pathlib.Path(str(evidence / "parent-source-verification.json") + ".sha256").write_text("0" * 64 + "\n")
        expect_failure(run_builder(workspace, harness, evidence, subject, harness_sha, mode="pinned"), "sidecar mismatch")

        workspace, harness, evidence, subject, _, harness_sha, _ = setup_case(
            root, "invalid-action-pin", source_authority="VerifiedPinnedDependencySourceMaterialization"
        )
        expect_failure(
            run_builder(workspace, harness, evidence, subject, harness_sha, mode="pinned", checkout_pin="not-a-sha"),
            "checkout action SHA",
        )

        workspace, harness, evidence, subject, _, harness_sha, _ = setup_case(
            root, "workflow-action-mismatch", source_authority="VerifiedPinnedDependencySourceMaterialization", checkout_pin="2" * 40
        )
        expect_failure(
            run_builder(workspace, harness, evidence, subject, harness_sha, mode="pinned", checkout_pin=PIN),
            "not present in exact workflow bytes",
        )

        workspace, harness, _, subject, _, harness_sha, _ = setup_case(
            root, "evidence-inside-product", source_authority="VerifiedPinnedDependencySourceMaterialization"
        )
        inside = workspace / "evidence"
        inside.mkdir()
        expect_failure(run_builder(workspace, harness, inside, subject, harness_sha, mode="pinned"), "must be outside")

        workspace, harness, evidence, subject, _, harness_sha, _ = setup_case(
            root, "create-only", source_authority="VerifiedPinnedDependencySourceMaterialization"
        )
        expect_success(run_builder(workspace, harness, evidence, subject, harness_sha, mode="pinned"))
        expect_failure(run_builder(workspace, harness, evidence, subject, harness_sha, mode="pinned"), "refusing to overwrite")

    print("canonical CI evidence receipt adversarial self-test: PASS")


if __name__ == "__main__":
    main()
