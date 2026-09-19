#!/usr/bin/env python3
"""Adversarial self-test for parent-Mycelix source materialization v1."""
from __future__ import annotations

import json
import os
import pathlib
import subprocess
import tempfile
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/materialize-parent-mycelix.py"
REPOSITORY = "Luminous-Dynamics/mycelix"
REMOTE = "https://github.com/Luminous-Dynamics/mycelix.git"


def command(*args: str, cwd: pathlib.Path | None = None, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, cwd=cwd, check=check, capture_output=True, text=True)


def git(repo: pathlib.Path, *args: str) -> str:
    return command("git", *args, cwd=repo).stdout.strip()


def write_parent(
    path: pathlib.Path,
    *,
    remote: str = REMOTE,
    escaping_symlink: bool = False,
) -> tuple[str, str]:
    path.mkdir(parents=True)
    git(path, "init")
    git(path, "config", "user.email", "ci-source-test@example.invalid")
    git(path, "config", "user.name", "CI Source Test")
    git(path, "remote", "add", "origin", remote)
    files = {
        "crates/mycelix-bridge-common/Cargo.toml": "[package]\nname='mycelix-bridge-common'\nversion='0.0.0'\n",
        "crates/mycelix-leptos-core/Cargo.toml": "[package]\nname='mycelix-leptos-core'\nversion='0.0.0'\n",
        "crates/mycelix-zkp-core/Cargo.toml": "[package]\nname='mycelix-zkp-core'\nversion='0.0.0'\n",
        "mycelix-core/libs/mycelix-fl/Cargo.toml": "[package]\nname='mycelix-fl'\nversion='0.0.0'\n",
        "mycelix-core/README.md": "fixture\n",
    }
    for relative, content in files.items():
        target = path / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
    if escaping_symlink:
        (path / "secret.txt").write_text("outside export\n", encoding="utf-8")
        os.symlink("../secret.txt", path / "crates" / "escape")
    git(path, "add", ".")
    git(path, "commit", "-m", "fixture parent")
    return git(path, "rev-parse", "HEAD"), git(path, "rev-parse", "HEAD^{tree}")


def profile(commit: str, tree: str) -> dict[str, Any]:
    return {
        "schema": "mycelix-health-parent-source-profile/v1",
        "profile_id": "fixture-parent-v1",
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


def setup_case(
    root: pathlib.Path,
    name: str,
    *,
    remote: str = REMOTE,
    escaping_symlink: bool = False,
):
    case = root / name
    parent = case / "parent"
    workspace = case / "health"
    workspace.mkdir(parents=True)
    commit, tree = write_parent(parent, remote=remote, escaping_symlink=escaping_symlink)
    profile_path = case / "profile.json"
    profile_path.write_text(json.dumps(profile(commit, tree), indent=2) + "\n", encoding="utf-8")
    return case, parent, workspace, profile_path, commit, tree


def run_materializer(
    parent: pathlib.Path,
    workspace: pathlib.Path,
    profile_path: pathlib.Path,
    receipt: pathlib.Path,
    *,
    mode: str = "pinned",
    replace: bool = False,
) -> subprocess.CompletedProcess[str]:
    args = [
        "python3",
        str(SCRIPT),
        "--checkout",
        str(parent),
        "--workspace-root",
        str(workspace),
        "--profile",
        str(profile_path),
        "--mode",
        mode,
        "--receipt",
        str(receipt),
    ]
    if replace:
        args.append("--replace-existing")
    return command(*args, check=False)


def expect_failure(result: subprocess.CompletedProcess[str], fragment: str) -> None:
    if result.returncode == 0:
        raise SystemExit(f"expected failure containing {fragment!r}, command succeeded")
    combined = result.stdout + result.stderr
    if fragment not in combined:
        raise SystemExit(f"expected failure containing {fragment!r}, got:\n{combined}")


def main() -> None:
    with tempfile.TemporaryDirectory() as raw:
        root = pathlib.Path(raw)

        # Positive pinned materialization binds exact Git identity and copied bytes.
        case, parent, workspace, profile_path, commit, tree = setup_case(root, "pinned-success")
        receipt = case / "receipt.json"
        result = run_materializer(parent, workspace, profile_path, receipt)
        if result.returncode != 0:
            raise SystemExit(result.stdout + result.stderr)
        value = json.loads(receipt.read_text())
        assert value["authority"] == "PinnedDependencySourceMaterialized"
        assert value["observed_source"]["commit_sha"] == commit
        assert value["observed_source"]["tree_sha"] == tree
        assert len(value["exports"]) == 3
        assert receipt.with_name(receipt.name + ".sha256").is_file()
        assert value["exports"][1]["content_sha256"] == value["exports"][2]["content_sha256"]

        # Exact commit substitution is rejected.
        case, parent, workspace, profile_path, _, _ = setup_case(root, "commit-mismatch")
        value = json.loads(profile_path.read_text())
        value["commit_sha"] = "0" * 40
        profile_path.write_text(json.dumps(value) + "\n")
        expect_failure(run_materializer(parent, workspace, profile_path, case / "receipt.json"), "parent commit mismatch")

        # Non-hex object identities are rejected before Git comparison.
        case, parent, workspace, profile_path, _, _ = setup_case(root, "invalid-object-id")
        value = json.loads(profile_path.read_text())
        value["tree_sha"] = "z" * 40
        profile_path.write_text(json.dumps(value) + "\n")
        expect_failure(run_materializer(parent, workspace, profile_path, case / "receipt.json"), "hexadecimal Git object ID")

        # Exact tree substitution is rejected independently from commit matching.
        case, parent, workspace, profile_path, _, _ = setup_case(root, "tree-mismatch")
        value = json.loads(profile_path.read_text())
        value["tree_sha"] = "0" * 40
        profile_path.write_text(json.dumps(value) + "\n")
        expect_failure(run_materializer(parent, workspace, profile_path, case / "receipt.json"), "parent tree mismatch")

        # Dirty tracked or untracked state is never materialized as Git-tree evidence.
        case, parent, workspace, profile_path, _, _ = setup_case(root, "dirty")
        (parent / "untracked.txt").write_text("not in tree\n")
        expect_failure(run_materializer(parent, workspace, profile_path, case / "receipt.json"), "parent checkout is dirty")

        # Repository-origin substitution fails even when commit/tree happen to match the profile.
        case, parent, workspace, profile_path, _, _ = setup_case(
            root,
            "wrong-repository",
            remote="https://github.com/Luminous-Dynamics/not-mycelix.git",
        )
        expect_failure(run_materializer(parent, workspace, profile_path, case / "receipt.json"), "parent repository mismatch")

        # Source traversal is rejected at profile validation.
        case, parent, workspace, profile_path, _, _ = setup_case(root, "source-traversal")
        value = json.loads(profile_path.read_text())
        value["exports"][0]["source"] = "../escape"
        profile_path.write_text(json.dumps(value) + "\n")
        expect_failure(run_materializer(parent, workspace, profile_path, case / "receipt.json"), "non-traversing relative path")

        # Clean, Git-tracked links cannot cause exported bytes to depend on files outside the export root.
        case, parent, workspace, profile_path, _, _ = setup_case(root, "symlink-escape", escaping_symlink=True)
        expect_failure(run_materializer(parent, workspace, profile_path, case / "receipt.json"), "symlink escapes export")

        # Required manifests are checked after copy rather than assumed from directory identity.
        case, parent, workspace, profile_path, _, _ = setup_case(root, "missing-manifest")
        value = json.loads(profile_path.read_text())
        value["exports"][0]["required_manifests"].append("missing/Cargo.toml")
        profile_path.write_text(json.dumps(value) + "\n")
        expect_failure(run_materializer(parent, workspace, profile_path, case / "receipt.json"), "required manifest missing")

        # Existing sibling trees require an explicit replacement decision.
        case, parent, workspace, profile_path, _, _ = setup_case(root, "existing-destination")
        existing = workspace.parent / "crates"
        existing.mkdir()
        (existing / "sentinel").write_text("preserve me\n")
        expect_failure(run_materializer(parent, workspace, profile_path, case / "receipt.json"), "--replace-existing")
        assert (existing / "sentinel").read_text() == "preserve me\n"

        # Two textual destinations that resolve to one sibling are rejected before either is overwritten.
        case, parent, workspace, profile_path, _, _ = setup_case(root, "destination-alias")
        real = workspace.parent / "real-dest"
        alias = workspace.parent / "alias-dest"
        os.symlink("real-dest", alias)
        value = json.loads(profile_path.read_text())
        value["exports"] = [
            {
                "source": "crates",
                "destination": "../real-dest",
                "required_manifests": ["mycelix-bridge-common/Cargo.toml"],
            },
            {
                "source": "crates",
                "destination": "../alias-dest",
                "required_manifests": ["mycelix-bridge-common/Cargo.toml"],
            },
        ]
        profile_path.write_text(json.dumps(value) + "\n")
        expect_failure(run_materializer(parent, workspace, profile_path, case / "receipt.json"), "resolve to the same path")
        assert not real.exists()

        # Receipt creation is create-only; prior evidence cannot be overwritten.
        case, parent, workspace, profile_path, _, _ = setup_case(root, "receipt-create-only")
        receipt = case / "receipt.json"
        receipt.write_text("historical evidence\n")
        expect_failure(run_materializer(parent, workspace, profile_path, receipt), "refusing to overwrite existing receipt artifact")
        assert receipt.read_text() == "historical evidence\n"

        # A clean successor may be sampled, but only as compatibility evidence.
        case, parent, workspace, profile_path, _, _ = setup_case(root, "compatibility-success")
        (parent / "newer.txt").write_text("successor\n")
        git(parent, "add", ".")
        git(parent, "commit", "-m", "successor")
        receipt = case / "receipt.json"
        result = run_materializer(parent, workspace, profile_path, receipt, mode="compatibility")
        if result.returncode != 0:
            raise SystemExit(result.stdout + result.stderr)
        value = json.loads(receipt.read_text())
        assert value["authority"] == "CompatibilityOnly"
        assert value["observed_source"]["commit_sha"] != value["declared_source"]["commit_sha"]

        # The same successor remains invalid in pinned mode.
        case, parent, workspace, profile_path, _, _ = setup_case(root, "successor-pinned")
        (parent / "newer.txt").write_text("successor\n")
        git(parent, "add", ".")
        git(parent, "commit", "-m", "successor")
        expect_failure(run_materializer(parent, workspace, profile_path, case / "receipt.json"), "parent commit mismatch")

    print("parent source materializer adversarial self-test: PASS")


if __name__ == "__main__":
    main()
