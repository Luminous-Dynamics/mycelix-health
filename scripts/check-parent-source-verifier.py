#!/usr/bin/env python3
"""Adversarial self-test for independent parent-source receipt verification v1."""
from __future__ import annotations

import hashlib
import json
import pathlib
import subprocess
import tempfile
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
MATERIALIZER = ROOT / "scripts/materialize-parent-mycelix.py"
VERIFIER = ROOT / "scripts/verify-parent-source-materialization.py"
REMOTE = "https://github.com/Luminous-Dynamics/mycelix.git"
REPOSITORY = "Luminous-Dynamics/mycelix"


def command(*args: str, cwd: pathlib.Path | None = None, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, cwd=cwd, check=check, capture_output=True, text=True)


def git(repo: pathlib.Path, *args: str) -> str:
    return command("git", *args, cwd=repo).stdout.strip()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_parent(path: pathlib.Path) -> tuple[str, str]:
    path.mkdir(parents=True)
    git(path, "init")
    git(path, "config", "user.email", "ci-verifier-test@example.invalid")
    git(path, "config", "user.name", "CI Verifier Test")
    git(path, "remote", "add", "origin", REMOTE)
    files = {
        "crates/mycelix-bridge-common/Cargo.toml": "[package]\nname='mycelix-bridge-common'\nversion='0.0.0'\n",
        "crates/mycelix-leptos-core/Cargo.toml": "[package]\nname='mycelix-leptos-core'\nversion='0.0.0'\n",
        "crates/mycelix-zkp-core/Cargo.toml": "[package]\nname='mycelix-zkp-core'\nversion='0.0.0'\n",
        "mycelix-core/libs/mycelix-fl/Cargo.toml": "[package]\nname='mycelix-fl'\nversion='0.0.0'\n",
    }
    for relative, content in files.items():
        target = path / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
    git(path, "add", ".")
    git(path, "commit", "-m", "fixture")
    return git(path, "rev-parse", "HEAD"), git(path, "rev-parse", "HEAD^{tree}")


def source_profile(commit: str, tree: str) -> dict[str, Any]:
    return {
        "schema": "mycelix-health-parent-source-profile/v1",
        "profile_id": "verifier-fixture-v1",
        "authority": "DeclaredDependencySourceIdentityOnly",
        "repository": REPOSITORY,
        "commit_sha": commit,
        "tree_sha": tree,
        "exports": [
            {
                "source": "crates",
                "destination": "../crates",
                "required_manifests": [
                    "mycelix-bridge-common/Cargo.toml",
                    "mycelix-leptos-core/Cargo.toml",
                    "mycelix-zkp-core/Cargo.toml",
                ],
            },
            {
                "source": "mycelix-core",
                "destination": "../mycelix-core",
                "required_manifests": ["libs/mycelix-fl/Cargo.toml"],
            },
            {
                "source": "mycelix-core",
                "destination": "../mycelix-workspace/mycelix-core",
                "required_manifests": ["libs/mycelix-fl/Cargo.toml"],
            },
        ],
        "non_claims": ["fixture only"],
    }


def setup(root: pathlib.Path, name: str):
    case = root / name
    parent = case / "parent"
    health = case / "health"
    health.mkdir(parents=True)
    commit, tree = write_parent(parent)
    profile = case / "profile.json"
    profile.write_text(json.dumps(source_profile(commit, tree), indent=2) + "\n", encoding="utf-8")
    receipt = case / "receipt.json"
    verification = case / "verification.json"
    return case, parent, health, profile, receipt, verification


def run_materializer(
    parent: pathlib.Path,
    health: pathlib.Path,
    profile: pathlib.Path,
    receipt: pathlib.Path,
    *,
    mode: str = "pinned",
) -> subprocess.CompletedProcess[str]:
    return command(
        "python3",
        str(MATERIALIZER),
        "--checkout",
        str(parent),
        "--workspace-root",
        str(health),
        "--profile",
        str(profile),
        "--mode",
        mode,
        "--replace-existing",
        "--receipt",
        str(receipt),
        check=False,
    )


def run_verifier(
    parent: pathlib.Path,
    health: pathlib.Path,
    profile: pathlib.Path,
    receipt: pathlib.Path,
    verification: pathlib.Path,
) -> subprocess.CompletedProcess[str]:
    return command(
        "python3",
        str(VERIFIER),
        "--checkout",
        str(parent),
        "--workspace-root",
        str(health),
        "--profile",
        str(profile),
        "--receipt",
        str(receipt),
        "--verification",
        str(verification),
        check=False,
    )


def expect_success(result: subprocess.CompletedProcess[str]) -> None:
    if result.returncode != 0:
        raise SystemExit(result.stdout + result.stderr)


def expect_failure(result: subprocess.CompletedProcess[str], fragment: str) -> None:
    if result.returncode == 0:
        raise SystemExit(f"expected verifier failure containing {fragment!r}")
    combined = result.stdout + result.stderr
    if fragment not in combined:
        raise SystemExit(f"expected verifier failure containing {fragment!r}, got:\n{combined}")


def rewrite_receipt(receipt: pathlib.Path, value: dict[str, Any], *, canonical: bool = True) -> None:
    if canonical:
        raw = (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()
    else:
        raw = (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()
    receipt.write_bytes(raw)
    pathlib.Path(str(receipt) + ".sha256").write_text(sha256_bytes(raw) + "\n", encoding="utf-8")


def main() -> None:
    with tempfile.TemporaryDirectory() as raw:
        root = pathlib.Path(raw)

        # Positive pinned evidence is independently reconstructed and verified.
        case, parent, health, profile, receipt, verification = setup(root, "positive")
        expect_success(run_materializer(parent, health, profile, receipt))
        expect_success(run_verifier(parent, health, profile, receipt, verification))
        verified = json.loads(verification.read_text())
        assert verified["authority"] == "VerifiedPinnedDependencySourceMaterialization"
        assert pathlib.Path(str(verification) + ".sha256").is_file()

        # Non-canonical receipt bytes fail even if an attacker recomputes the sidecar.
        case, parent, health, profile, receipt, verification = setup(root, "noncanonical")
        expect_success(run_materializer(parent, health, profile, receipt))
        value = json.loads(receipt.read_text())
        rewrite_receipt(receipt, value, canonical=False)
        expect_failure(run_verifier(parent, health, profile, receipt, verification), "not in canonical JSON")

        # Semantic receipt tampering fails even with a recomputed canonical sidecar.
        case, parent, health, profile, receipt, verification = setup(root, "authority-tamper")
        expect_success(run_materializer(parent, health, profile, receipt))
        value = json.loads(receipt.read_text())
        value["authority"] = "CompatibilityOnly"
        rewrite_receipt(receipt, value)
        expect_failure(run_verifier(parent, health, profile, receipt, verification), "pinned receipt has incorrect authority")

        # Post-materialization byte substitution is detected by independent digest reconstruction.
        case, parent, health, profile, receipt, verification = setup(root, "destination-tamper")
        expect_success(run_materializer(parent, health, profile, receipt))
        target = health.parent / "crates" / "mycelix-bridge-common" / "Cargo.toml"
        target.write_text(target.read_text() + "# tampered\n")
        expect_failure(run_verifier(parent, health, profile, receipt, verification), "source/destination export digest mismatch")

        # The profile itself is bound by digest; prospective changes cannot reinterpret an old receipt.
        case, parent, health, profile, receipt, verification = setup(root, "profile-tamper")
        expect_success(run_materializer(parent, health, profile, receipt))
        value = json.loads(profile.read_text())
        value["non_claims"].append("prospective profile change")
        profile.write_text(json.dumps(value, indent=2) + "\n")
        expect_failure(run_verifier(parent, health, profile, receipt, verification), "profile digest")

        # The parent checkout must still be the clean source described by the receipt.
        case, parent, health, profile, receipt, verification = setup(root, "dirty-parent")
        expect_success(run_materializer(parent, health, profile, receipt))
        (parent / "dirty.txt").write_text("not committed\n")
        expect_failure(run_verifier(parent, health, profile, receipt, verification), "checkout is dirty")

        # Verification artifacts are create-only and cannot silently replace prior evidence.
        case, parent, health, profile, receipt, verification = setup(root, "verification-create-only")
        expect_success(run_materializer(parent, health, profile, receipt))
        expect_success(run_verifier(parent, health, profile, receipt, verification))
        expect_failure(run_verifier(parent, health, profile, receipt, verification), "refusing to overwrite verification")

        # A clean successor remains explicitly compatibility-only after independent verification.
        case, parent, health, profile, receipt, verification = setup(root, "compatibility")
        (parent / "successor.txt").write_text("new parent source\n")
        git(parent, "add", ".")
        git(parent, "commit", "-m", "successor")
        expect_success(run_materializer(parent, health, profile, receipt, mode="compatibility"))
        expect_success(run_verifier(parent, health, profile, receipt, verification))
        verified = json.loads(verification.read_text())
        assert verified["authority"] == "VerifiedCompatibilityObservation"
        assert verified["observed_source"]["commit_sha"] != json.loads(profile.read_text())["commit_sha"]

    print("parent source receipt verifier adversarial self-test: PASS")


if __name__ == "__main__":
    main()
