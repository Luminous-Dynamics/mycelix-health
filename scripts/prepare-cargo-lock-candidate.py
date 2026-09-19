#!/usr/bin/env python3
"""Generate a review-only Cargo.lock reconciliation candidate in a detached worktree.

The caller's product checkout is never modified. The exact subject is recreated in
a disposable detached Git worktree, pinned parent Mycelix path dependencies are
materialized around that worktree, and Cargo reconciliation runs inside the
subject's committed Nix `.#ci` shell.

Output authority is always `RepairCandidateOnly`.
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
import tomllib
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]
MATERIALIZER = ROOT / "scripts/materialize-parent-mycelix.py"
SOURCE_VERIFIER = ROOT / "scripts/verify-parent-source-materialization.py"
SCHEMA = "mycelix-health-cargo-lock-candidate/v1"
HEX = frozenset("0123456789abcdefABCDEF")


def fail(message: str) -> NoReturn:
    raise SystemExit(f"lock candidate generation failed: {message}")


def run(*args: str, cwd: pathlib.Path | None = None, capture: bool = True) -> str:
    try:
        result = subprocess.run(
            args,
            cwd=cwd,
            check=True,
            capture_output=capture,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError) as error:
        detail = ""
        if isinstance(error, subprocess.CalledProcessError):
            detail = f"\nstdout:\n{error.stdout or ''}\nstderr:\n{error.stderr or ''}"
        fail(f"command failed: {' '.join(args)}{detail}")
    return result.stdout.strip() if capture else ""


def git(repo: pathlib.Path, *args: str) -> str:
    return run("git", "-C", str(repo), *args)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    try:
        return sha256_bytes(path.read_bytes())
    except OSError as error:
        fail(f"cannot hash {path}: {error}")


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def full_sha(value: str, *, label: str) -> str:
    value = value.strip()
    if len(value) != 40 or any(char not in HEX for char in value):
        fail(f"{label} must be a full 40-character hexadecimal Git SHA")
    return value.lower()


def require_outside(root: pathlib.Path, candidate: pathlib.Path, *, label: str) -> None:
    try:
        candidate.relative_to(root)
    except ValueError:
        return
    fail(f"{label} must be outside {root}")


def write_create_only(path: pathlib.Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() or path.is_symlink():
        fail(f"refusing to overwrite artifact: {path}")
    try:
        with path.open("xb") as handle:
            handle.write(data)
    except FileExistsError:
        fail(f"refusing to overwrite artifact: {path}")


def packages(lock_path: pathlib.Path) -> list[dict[str, Any]]:
    try:
        with lock_path.open("rb") as handle:
            document = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as error:
        fail(f"cannot parse Cargo lockfile {lock_path}: {error}")
    value = document.get("package", [])
    if not isinstance(value, list):
        fail(f"Cargo lockfile package table is malformed: {lock_path}")
    return value


def lock_audit(before_path: pathlib.Path, candidate_path: pathlib.Path) -> dict[str, Any]:
    before = packages(before_path)
    after = packages(candidate_path)

    def registry_set(items: list[dict[str, Any]]) -> set[tuple[str, str, str]]:
        return {
            (str(p["name"]), str(p["version"]), str(p.get("source", "")))
            for p in items
            if str(p.get("source", "")).startswith("registry+")
        }

    def local_set(items: list[dict[str, Any]]) -> set[tuple[str, str]]:
        return {
            (str(p["name"]), str(p["version"]))
            for p in items
            if "source" not in p
        }

    def git_set(items: list[dict[str, Any]]) -> set[tuple[str, str, str]]:
        return {
            (str(p["name"]), str(p["version"]), str(p.get("source", "")))
            for p in items
            if str(p.get("source", "")).startswith("git+")
        }

    def versions(items: set[tuple[str, str, str]]) -> dict[str, list[str]]:
        result: dict[str, set[str]] = {}
        for name, version, _ in items:
            result.setdefault(name, set()).add(version)
        return {name: sorted(values) for name, values in sorted(result.items())}

    before_reg = registry_set(before)
    after_reg = registry_set(after)
    before_local = local_set(before)
    after_local = local_set(after)
    before_git = git_set(before)
    after_git = git_set(after)
    before_versions = versions(before_reg)
    after_versions = versions(after_reg)

    changed_registry_versions = {
        name: {"before": before_versions[name], "after": after_versions[name]}
        for name in sorted(before_versions.keys() & after_versions.keys())
        if before_versions[name] != after_versions[name]
    }
    return {
        "added_local_packages": sorted(after_local - before_local),
        "removed_local_packages": sorted(before_local - after_local),
        "added_registry_packages": sorted(after_reg - before_reg),
        "removed_registry_packages": sorted(before_reg - after_reg),
        "changed_registry_versions": changed_registry_versions,
        "added_git_packages": sorted(after_git - before_git),
        "removed_git_packages": sorted(before_git - after_git),
    }


def write_json(path: pathlib.Path, value: Any) -> None:
    write_create_only(path, canonical_bytes(value))


def current_system(worktree: pathlib.Path) -> str:
    return run("nix", "eval", "--impure", "--raw", "--expr", "builtins.currentSystem", cwd=worktree)


def nix_ci(worktree: pathlib.Path, *command: str) -> str:
    return run(
        "nix",
        "develop",
        "--no-write-lock-file",
        ".#ci",
        "--command",
        *command,
        cwd=worktree,
    )


def generate(args: argparse.Namespace) -> dict[str, Any]:
    subject_checkout = args.subject_checkout.resolve()
    parent_checkout = args.parent_checkout.resolve()
    output = args.output_dir.resolve()
    harness = ROOT.resolve()

    for directory, label in ((subject_checkout, "subject checkout"), (parent_checkout, "parent checkout")):
        if not directory.is_dir():
            fail(f"{label} does not exist: {directory}")
    require_outside(subject_checkout, output, label="output directory")
    require_outside(harness, output, label="output directory")
    if output.exists() or output.is_symlink():
        fail(f"output directory must not already exist: {output}")
    output.mkdir(parents=True)

    subject_sha = full_sha(args.subject_sha or git(subject_checkout, "rev-parse", "HEAD"), label="subject SHA")
    git(subject_checkout, "cat-file", "-e", f"{subject_sha}^{{commit}}")
    subject_tree = full_sha(git(subject_checkout, "rev-parse", f"{subject_sha}^{{tree}}"), label="subject tree")
    harness_sha = full_sha(git(harness, "rev-parse", "HEAD"), label="harness SHA")
    harness_tree = full_sha(git(harness, "rev-parse", "HEAD^{tree}"), label="harness tree")
    if git(harness, "status", "--porcelain=v1", "--untracked-files=all"):
        fail("portable lock-candidate harness checkout must be fully clean")

    required_packages = sorted(set(args.require_package))
    temporary_root = pathlib.Path(tempfile.mkdtemp(prefix="mycelix-health-lock-candidate-"))
    worktree = temporary_root / "health"
    source_receipt = output / "parent-source.json"
    source_verification = output / "parent-source-verification.json"
    worktree_registered = False
    try:
        run("git", "-C", str(subject_checkout), "worktree", "add", "--detach", str(worktree), subject_sha)
        worktree_registered = True
        if git(worktree, "rev-parse", "HEAD") != subject_sha:
            fail("detached worktree HEAD differs from requested subject")
        if git(worktree, "rev-parse", "HEAD^{tree}") != subject_tree:
            fail("detached worktree tree differs from requested subject")
        if git(worktree, "status", "--porcelain=v1", "--untracked-files=all"):
            fail("detached product worktree is not clean before reconciliation")

        profile = worktree / "release/ci-parent-source-profile-v1.json"
        if not profile.is_file():
            fail("subject does not contain release/ci-parent-source-profile-v1.json")

        run(
            "python3",
            str(MATERIALIZER),
            "--checkout",
            str(parent_checkout),
            "--workspace-root",
            str(worktree),
            "--profile",
            str(profile),
            "--mode",
            "pinned",
            "--replace-existing",
            "--receipt",
            str(source_receipt),
        )
        run(
            "python3",
            str(SOURCE_VERIFIER),
            "--checkout",
            str(parent_checkout),
            "--workspace-root",
            str(worktree),
            "--profile",
            str(profile),
            "--receipt",
            str(source_receipt),
            "--verification",
            str(source_verification),
        )
        verification_value = json.loads(source_verification.read_text(encoding="utf-8"))
        if verification_value.get("authority") != "VerifiedPinnedDependencySourceMaterialization":
            fail("parent source verification did not produce pinned authority")

        before_lock = worktree / "Cargo.lock"
        flake_lock = worktree / "flake.lock"
        rust_toolchain = worktree / "rust-toolchain.toml"
        for required in (before_lock, flake_lock, rust_toolchain):
            if not required.is_file():
                fail(f"subject is missing required input: {required.name}")

        shutil.copy2(before_lock, output / "Cargo.lock.before")
        shutil.copy2(flake_lock, output / "flake.lock")
        shutil.copy2(rust_toolchain, output / "rust-toolchain.toml")
        write_create_only(output / "subject-sha.txt", (subject_sha + "\n").encode())
        write_create_only(output / "subject-tree.txt", (subject_tree + "\n").encode())

        system = current_system(worktree)
        flake_metadata = run("nix", "flake", "metadata", "--json", "--no-write-lock-file", cwd=worktree)
        write_create_only(output / "flake-metadata.json", (flake_metadata + "\n").encode())
        ci_drv = run(
            "nix",
            "eval",
            "--raw",
            "--no-write-lock-file",
            f".#devShells.{system}.ci.drvPath",
            cwd=worktree,
        )
        write_create_only(output / "ci-dev-shell.drv", (ci_drv + "\n").encode())
        write_create_only(output / "rustc-vV.txt", (nix_ci(worktree, "rustc", "-vV") + "\n").encode())
        write_create_only(output / "cargo-vV.txt", (nix_ci(worktree, "cargo", "-vV") + "\n").encode())

        unlocked_metadata = nix_ci(worktree, "cargo", "metadata", "--no-deps", "--format-version", "1")
        write_create_only(output / "metadata.json", (unlocked_metadata + "\n").encode())
        locked_metadata = nix_ci(
            worktree,
            "cargo",
            "metadata",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
        )
        write_create_only(output / "metadata.locked.json", (locked_metadata + "\n").encode())

        candidate_lock = worktree / "Cargo.lock"
        shutil.copy2(candidate_lock, output / "Cargo.lock.candidate")
        candidate_packages = {str(item.get("name")) for item in packages(candidate_lock)}
        missing = [name for name in required_packages if name not in candidate_packages]
        if missing:
            fail(f"candidate lock is missing required package(s): {', '.join(missing)}")

        status = git(worktree, "status", "--porcelain=v1", "--untracked-files=all")
        status_lines = [line for line in status.splitlines() if line]
        unexpected = [line for line in status_lines if not line.endswith(" Cargo.lock")]
        if unexpected:
            fail(f"reconciliation changed files other than Cargo.lock: {unexpected}")
        write_create_only(output / "git-status.txt", (status + ("\n" if status else "")).encode())
        diff = git(worktree, "diff", "--", "Cargo.lock")
        write_create_only(output / "Cargo.lock.diff", (diff + ("\n" if diff else "")).encode())

        audit = lock_audit(output / "Cargo.lock.before", output / "Cargo.lock.candidate")
        write_json(output / "lock-audit.json", audit)

        candidate_report = {
            "schema": SCHEMA,
            "authority": "RepairCandidateOnly",
            "subject": {"sha": subject_sha, "tree_sha": subject_tree},
            "harness": {
                "sha": harness_sha,
                "tree_sha": harness_tree,
                "generator_sha256": sha256_file(pathlib.Path(__file__).resolve()),
                "materializer_sha256": sha256_file(MATERIALIZER),
                "source_verifier_sha256": sha256_file(SOURCE_VERIFIER),
            },
            "source_verification_sha256": sha256_file(source_verification),
            "environment": {
                "system": system,
                "flake_lock_sha256": sha256_file(flake_lock),
                "rust_toolchain_sha256": sha256_file(rust_toolchain),
                "ci_dev_shell_drv": ci_drv,
            },
            "locks": {
                "before_sha256": sha256_file(output / "Cargo.lock.before"),
                "candidate_sha256": sha256_file(output / "Cargo.lock.candidate"),
                "changed": sha256_file(output / "Cargo.lock.before") != sha256_file(output / "Cargo.lock.candidate"),
            },
            "required_packages": required_packages,
            "audit": audit,
            "claims": [
                "The candidate was reconciled from an exact detached worktree of the recorded subject.",
                "The caller's product checkout was not used as the reconciliation worktree.",
                "Pinned parent source was independently verified before Cargo reconciliation.",
                "Locked Cargo metadata succeeded after candidate generation inside the recorded Nix CI shell.",
            ],
            "non_claims": [
                "RepairCandidateOnly is not committed dependency-graph authority.",
                "This candidate is not repository qualification or clinical evidence.",
                "Registry/git movement requires human or separately qualified policy review before commit.",
            ],
        }
        report_bytes = canonical_bytes(candidate_report)
        write_create_only(output / "lock-candidate.json", report_bytes)
        write_create_only(output / "lock-candidate.json.sha256", (sha256_bytes(report_bytes) + "\n").encode())
        return candidate_report
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


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--subject-checkout", type=pathlib.Path, required=True)
    parser.add_argument("--subject-sha", default="")
    parser.add_argument("--parent-checkout", type=pathlib.Path, required=True)
    parser.add_argument("--output-dir", type=pathlib.Path, required=True)
    parser.add_argument("--require-package", action="append", default=[])
    return parser.parse_args()


def main() -> None:
    report = generate(parse_args())
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
