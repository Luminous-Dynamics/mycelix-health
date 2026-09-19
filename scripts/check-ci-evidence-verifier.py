#!/usr/bin/env python3
"""Adversarial self-test for independent final CI evidence verification v1."""
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
VERIFIER = ROOT / "scripts/verify-ci-evidence-receipt.py"
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
    git(repo, "config", "user.email", "ci-verifier-test@example.invalid")
    git(repo, "config", "user.name", name)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    return sha256_bytes(path.read_bytes())


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def workflow_fixture() -> str:
    return f"""name: fixture
on: workflow_dispatch
jobs:
  fixture:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@{PIN}
      - uses: cachix/install-nix-action@{PIN}
      - uses: actions/upload-artifact@{PIN}
"""


def github_env(harness_sha: str) -> dict[str, str]:
    env = os.environ.copy()
    env.update(
        {
            "GITHUB_ACTIONS": "true",
            "GITHUB_REPOSITORY": REPOSITORY,
            "GITHUB_EVENT_NAME": "workflow_dispatch",
            "GITHUB_WORKFLOW_SHA": harness_sha,
            "GITHUB_WORKFLOW_REF": f"{REPOSITORY}/{WORKFLOW}@refs/heads/harness",
            "GITHUB_RUN_ID": "9988",
            "GITHUB_RUN_ATTEMPT": "1",
            "RUNNER_OS": "Linux",
            "RUNNER_ARCH": "X64",
            "ImageOS": "ubuntu24",
            "ImageVersion": "fixture",
        }
    )
    return env


def setup_case(root: pathlib.Path, name: str, *, mode: str):
    case = root / name
    workspace = case / "workspace"
    harness = case / "harness"
    evidence = case / "evidence"
    evidence.mkdir(parents=True)
    init_repo(workspace, name="Verifier Product Fixture")
    init_repo(harness, name="Verifier Harness Fixture")

    for relative, content in {
        "Cargo.lock": "# frozen lock\n",
        "flake.lock": "{}\n",
        "rust-toolchain.toml": "[toolchain]\nchannel='1.96.0'\n",
        "release/ci-parent-source-profile-v1.json": "{}\n",
    }.items():
        target = workspace / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
    git(workspace, "add", ".")
    git(workspace, "commit", "-m", "product")
    subject = git(workspace, "rev-parse", "HEAD")

    for relative, content in {
        WORKFLOW: workflow_fixture(),
        "scripts/materialize-parent-mycelix.py": "# materializer fixture\n",
        "scripts/verify-parent-source-materialization.py": "# source verifier fixture\n",
        "scripts/build-ci-evidence-receipt.py": BUILDER.read_text(encoding="utf-8"),
        "scripts/verify-ci-evidence-receipt.py": VERIFIER.read_text(encoding="utf-8"),
    }.items():
        target = harness / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
    git(harness, "add", ".")
    git(harness, "commit", "-m", "harness")
    harness_sha = git(harness, "rev-parse", "HEAD")

    source_authority = (
        "VerifiedPinnedDependencySourceMaterialization"
        if mode == "pinned"
        else "VerifiedCompatibilityObservation"
    )
    source_report = {
        "schema": "mycelix-health-parent-source-verification/v1",
        "authority": source_authority,
        "mode": mode,
        "source_receipt_sha256": "2" * 64,
        "profile_sha256": sha256_file(workspace / "release/ci-parent-source-profile-v1.json"),
        "materializer_sha256": sha256_file(harness / "scripts/materialize-parent-mycelix.py"),
        "verifier_sha256": sha256_file(harness / "scripts/verify-parent-source-materialization.py"),
        "observed_source": {
            "repository": "Luminous-Dynamics/mycelix",
            "commit_sha": "6" * 40,
            "tree_sha": "7" * 40,
            "clean_checkout": True,
        },
        "exports": [{"source": "crates", "destination": "../crates", "content_sha256": "8" * 64}],
        "non_claims": [],
    }
    source_raw = canonical_bytes(source_report)
    source_path = evidence / "parent-source-verification.json"
    source_path.write_bytes(source_raw)
    pathlib.Path(str(source_path) + ".sha256").write_text(sha256_bytes(source_raw) + "\n", encoding="utf-8")

    for filename, data in (
        ("cargo-metadata.json", "{}\n"),
        ("flake-metadata.json", "{}\n"),
        ("ci-dev-shell.drv", "/nix/store/fixture-ci-shell.drv\n"),
    ):
        (evidence / filename).write_text(data, encoding="utf-8")
    for source, sidecar in (
        (workspace / "Cargo.lock", evidence / "Cargo.lock.sha256"),
        (workspace / "flake.lock", evidence / "flake.lock.sha256"),
        (workspace / "rust-toolchain.toml", evidence / "rust-toolchain.toml.sha256"),
    ):
        sidecar.write_text(sha256_file(source) + "  " + source.name + "\n", encoding="utf-8")

    env = github_env(harness_sha)
    built = command(
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
        PIN,
        "--install-nix-action-sha",
        PIN,
        "--upload-artifact-action-sha",
        PIN,
        "--output",
        str(evidence / "ci-evidence.json"),
        env=env,
        check=False,
    )
    if built.returncode != 0:
        raise SystemExit(built.stdout + built.stderr)
    return workspace, harness, evidence, subject, harness_sha, env


def run_verifier(
    workspace: pathlib.Path,
    harness: pathlib.Path,
    evidence: pathlib.Path,
    subject: str,
    harness_sha: str,
    env: dict[str, str],
):
    return command(
        "python3",
        str(VERIFIER),
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
        "--receipt",
        str(evidence / "ci-evidence.json"),
        "--workflow",
        WORKFLOW,
        "--verification",
        str(evidence / "ci-evidence-verification.json"),
        env=env,
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


def rewrite_receipt(evidence: pathlib.Path, mutate) -> None:
    path = evidence / "ci-evidence.json"
    value = json.loads(path.read_text(encoding="utf-8"))
    mutate(value)
    raw = canonical_bytes(value)
    path.write_bytes(raw)
    pathlib.Path(str(path) + ".sha256").write_text(sha256_bytes(raw) + "\n", encoding="utf-8")


def main() -> None:
    with tempfile.TemporaryDirectory() as raw:
        root = pathlib.Path(raw)

        workspace, harness, evidence, subject, harness_sha, env = setup_case(root, "pinned-success", mode="pinned")
        expect_success(run_verifier(workspace, harness, evidence, subject, harness_sha, env))
        verified = json.loads((evidence / "ci-evidence-verification.json").read_text())
        assert verified["authority"] == "VerifiedReproducibleRepositoryCIEvidence"

        workspace, harness, evidence, subject, harness_sha, env = setup_case(root, "compatibility-success", mode="compatibility")
        expect_success(run_verifier(workspace, harness, evidence, subject, harness_sha, env))
        verified = json.loads((evidence / "ci-evidence-verification.json").read_text())
        assert verified["authority"] == "VerifiedCompatibilityOnlyObservation"

        workspace, harness, evidence, subject, harness_sha, env = setup_case(root, "receipt-tamper", mode="pinned")
        rewrite_receipt(evidence, lambda value: value["subject"].__setitem__("tree_sha", "0" * 40))
        expect_failure(run_verifier(workspace, harness, evidence, subject, harness_sha, env), "product tree SHA differs")

        workspace, harness, evidence, subject, harness_sha, env = setup_case(root, "tool-hash-tamper", mode="pinned")
        rewrite_receipt(evidence, lambda value: value["harness"].__setitem__("receipt_builder_sha256", "0" * 64))
        expect_failure(run_verifier(workspace, harness, evidence, subject, harness_sha, env), "receipt_builder_sha256")

        workspace, harness, evidence, subject, harness_sha, env = setup_case(root, "source-report-tamper", mode="pinned")
        source = evidence / "parent-source-verification.json"
        source_value = json.loads(source.read_text(encoding="utf-8"))
        source_value["observed_source"]["commit_sha"] = "9" * 40
        source_raw = canonical_bytes(source_value)
        source.write_bytes(source_raw)
        pathlib.Path(str(source) + ".sha256").write_text(sha256_bytes(source_raw) + "\n", encoding="utf-8")
        expect_failure(run_verifier(workspace, harness, evidence, subject, harness_sha, env), "source verification digest differs")

        workspace, harness, evidence, subject, harness_sha, env = setup_case(root, "product-drift", mode="pinned")
        (workspace / "Cargo.lock").write_text("# changed\n", encoding="utf-8")
        expect_failure(run_verifier(workspace, harness, evidence, subject, harness_sha, env), "tracked/index drift")

        workspace, harness, evidence, subject, harness_sha, env = setup_case(root, "harness-dirty", mode="pinned")
        (harness / "untracked.txt").write_text("dirty\n", encoding="utf-8")
        expect_failure(run_verifier(workspace, harness, evidence, subject, harness_sha, env), "not fully clean")

        workspace, harness, evidence, subject, harness_sha, env = setup_case(root, "execution-mismatch", mode="pinned")
        env = dict(env)
        env["GITHUB_RUN_ATTEMPT"] = "2"
        expect_failure(run_verifier(workspace, harness, evidence, subject, harness_sha, env), "execution identity differs")

        workspace, harness, evidence, subject, harness_sha, env = setup_case(root, "create-only", mode="pinned")
        expect_success(run_verifier(workspace, harness, evidence, subject, harness_sha, env))
        expect_failure(run_verifier(workspace, harness, evidence, subject, harness_sha, env), "refusing to overwrite")

    print("independent CI evidence verifier adversarial self-test: PASS")


if __name__ == "__main__":
    main()
