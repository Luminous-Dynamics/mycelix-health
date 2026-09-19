#!/usr/bin/env python3
"""Independently verify a portable Cargo.lock repair candidate.

The verifier does not import the candidate generator. It recreates the exact product
subject in a fresh detached worktree, independently rematerializes pinned parent
source, installs the candidate lock, and reruns `cargo metadata --locked` inside the
subject's committed Nix `.#ci` shell.

Verification authority remains repair-only: `VerifiedRepairCandidateOnly`.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import shutil
import subprocess
import tempfile
import tomllib
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]
MATERIALIZER = ROOT / "scripts/materialize-parent-mycelix.py"
SOURCE_VERIFIER = ROOT / "scripts/verify-parent-source-materialization.py"
CANDIDATE_SCHEMA = "mycelix-health-cargo-lock-candidate/v1"
VERIFICATION_SCHEMA = "mycelix-health-cargo-lock-candidate-verification/v1"
HEX = frozenset("0123456789abcdefABCDEF")


def fail(message: str) -> NoReturn:
    raise SystemExit(f"lock candidate verification failed: {message}")


def run(*args: str, cwd: pathlib.Path | None = None) -> str:
    try:
        result = subprocess.run(args, cwd=cwd, check=True, capture_output=True, text=True)
    except (OSError, subprocess.CalledProcessError) as error:
        detail = ""
        if isinstance(error, subprocess.CalledProcessError):
            detail = f"\nstdout:\n{error.stdout or ''}\nstderr:\n{error.stderr or ''}"
        fail(f"command failed: {' '.join(args)}{detail}")
    return result.stdout.strip()


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


def full_sha(value: Any, *, label: str) -> str:
    if not isinstance(value, str) or len(value) != 40 or any(char not in HEX for char in value):
        fail(f"{label} must be a full 40-character hexadecimal Git SHA")
    return value.lower()


def full_digest(value: Any, *, label: str) -> str:
    if not isinstance(value, str) or len(value) != 64 or any(char not in HEX for char in value):
        fail(f"{label} must be a hexadecimal SHA-256 digest")
    return value.lower()


def load_object(path: pathlib.Path, *, label: str) -> tuple[dict[str, Any], bytes]:
    try:
        raw = path.read_bytes()
        value = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {label} {path}: {error}")
    if not isinstance(value, dict):
        fail(f"{label} must be a JSON object")
    return value, raw


def sidecar_digest(path: pathlib.Path, *, label: str) -> str:
    sidecar = pathlib.Path(str(path) + ".sha256")
    try:
        text = sidecar.read_text(encoding="utf-8").strip().split()[0]
    except (OSError, IndexError) as error:
        fail(f"cannot read {label} sidecar: {error}")
    expected = full_digest(text, label=f"{label} sidecar")
    actual = sha256_file(path)
    if actual != expected:
        fail(f"{label} sidecar digest mismatch")
    return actual


def write_create_only(path: pathlib.Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() or path.is_symlink():
        fail(f"refusing to overwrite verification artifact: {path}")
    try:
        with path.open("xb") as handle:
            handle.write(data)
    except FileExistsError:
        fail(f"refusing to overwrite verification artifact: {path}")


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

    def registry(items: list[dict[str, Any]]) -> set[tuple[str, str, str]]:
        return {
            (str(p["name"]), str(p["version"]), str(p.get("source", "")))
            for p in items
            if str(p.get("source", "")).startswith("registry+")
        }

    def local(items: list[dict[str, Any]]) -> set[tuple[str, str]]:
        return {(str(p["name"]), str(p["version"])) for p in items if "source" not in p}

    def git_packages(items: list[dict[str, Any]]) -> set[tuple[str, str, str]]:
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

    before_reg = registry(before)
    after_reg = registry(after)
    before_local = local(before)
    after_local = local(after)
    before_git = git_packages(before)
    after_git = git_packages(after)
    before_versions = versions(before_reg)
    after_versions = versions(after_reg)
    changed = {
        name: {"before": before_versions[name], "after": after_versions[name]}
        for name in sorted(before_versions.keys() & after_versions.keys())
        if before_versions[name] != after_versions[name]
    }
    # Round-trip through JSON so tuples normalize to arrays exactly as candidate JSON does.
    return json.loads(
        json.dumps(
            {
                "added_local_packages": sorted(after_local - before_local),
                "removed_local_packages": sorted(before_local - after_local),
                "added_registry_packages": sorted(after_reg - before_reg),
                "removed_registry_packages": sorted(before_reg - after_reg),
                "changed_registry_versions": changed,
                "added_git_packages": sorted(after_git - before_git),
                "removed_git_packages": sorted(before_git - after_git),
            }
        )
    )


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


def git_file_bytes(repo: pathlib.Path, subject_sha: str, path: str) -> bytes:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo), "show", f"{subject_sha}:{path}"],
            check=True,
            capture_output=True,
        )
    except (OSError, subprocess.CalledProcessError) as error:
        fail(f"cannot read committed {path} from subject {subject_sha}: {error}")
    return result.stdout


def verify(args: argparse.Namespace) -> dict[str, Any]:
    subject_checkout = args.subject_checkout.resolve()
    parent_checkout = args.parent_checkout.resolve()
    candidate_dir = args.candidate_dir.resolve()
    output = args.verification.resolve()
    harness = ROOT.resolve()
    for directory, label in (
        (subject_checkout, "subject checkout"),
        (parent_checkout, "parent checkout"),
        (candidate_dir, "candidate directory"),
    ):
        if not directory.is_dir():
            fail(f"{label} does not exist: {directory}")
    if git(harness, "status", "--porcelain=v1", "--untracked-files=all"):
        fail("portable lock-candidate verification harness must be fully clean")

    candidate_path = candidate_dir / "lock-candidate.json"
    candidate, candidate_raw = load_object(candidate_path, label="candidate report")
    if candidate.get("schema") != CANDIDATE_SCHEMA or candidate.get("authority") != "RepairCandidateOnly":
        fail("candidate report schema/authority mismatch")
    if candidate_raw != canonical_bytes(candidate):
        fail("candidate report is not canonical JSON")
    candidate_report_sha = sidecar_digest(candidate_path, label="candidate report")

    subject = candidate.get("subject")
    harness_binding = candidate.get("harness")
    locks = candidate.get("locks")
    environment = candidate.get("environment")
    audit = candidate.get("audit")
    for value, label in (
        (subject, "subject"),
        (harness_binding, "harness"),
        (locks, "locks"),
        (environment, "environment"),
        (audit, "audit"),
    ):
        if not isinstance(value, dict):
            fail(f"candidate {label} object is missing")

    subject_sha = full_sha(subject.get("sha"), label="candidate subject SHA")
    subject_tree = full_sha(subject.get("tree_sha"), label="candidate subject tree")
    git(subject_checkout, "cat-file", "-e", f"{subject_sha}^{{commit}}")
    if git(subject_checkout, "rev-parse", f"{subject_sha}^{{tree}}") != subject_tree:
        fail("candidate subject tree does not match repository object")

    harness_sha = full_sha(harness_binding.get("sha"), label="candidate harness SHA")
    harness_tree = full_sha(harness_binding.get("tree_sha"), label="candidate harness tree")
    if git(harness, "rev-parse", "HEAD") != harness_sha:
        fail("candidate harness SHA does not equal verifier harness HEAD")
    if git(harness, "rev-parse", "HEAD^{tree}") != harness_tree:
        fail("candidate harness tree does not equal verifier harness tree")
    for key, path in (
        ("generator_sha256", harness / "scripts/prepare-cargo-lock-candidate.py"),
        ("materializer_sha256", MATERIALIZER),
        ("source_verifier_sha256", SOURCE_VERIFIER),
    ):
        expected = full_digest(harness_binding.get(key), label=f"candidate harness {key}")
        if expected != sha256_file(path):
            fail(f"candidate harness {key} does not match verifier harness bytes")

    before_path = candidate_dir / "Cargo.lock.before"
    candidate_lock_path = candidate_dir / "Cargo.lock.candidate"
    for path in (before_path, candidate_lock_path):
        if not path.is_file():
            fail(f"candidate artifact is missing: {path.name}")
    committed_before = git_file_bytes(subject_checkout, subject_sha, "Cargo.lock")
    if before_path.read_bytes() != committed_before:
        fail("Cargo.lock.before does not equal exact subject Cargo.lock bytes")
    if full_digest(locks.get("before_sha256"), label="before lock digest") != sha256_file(before_path):
        fail("before lock digest mismatch")
    if full_digest(locks.get("candidate_sha256"), label="candidate lock digest") != sha256_file(candidate_lock_path):
        fail("candidate lock digest mismatch")
    if bool(locks.get("changed")) != (before_path.read_bytes() != candidate_lock_path.read_bytes()):
        fail("candidate changed flag is inconsistent with lock bytes")
    recomputed_audit = lock_audit(before_path, candidate_lock_path)
    if audit != recomputed_audit:
        fail("candidate lock audit does not match independently recomputed movement")

    required_packages = candidate.get("required_packages")
    if not isinstance(required_packages, list) or any(not isinstance(item, str) for item in required_packages):
        fail("candidate required_packages is malformed")
    requested_packages = sorted(set(args.require_package))
    if requested_packages and sorted(required_packages) != requested_packages:
        fail("candidate required package set differs from verifier expectation")

    source_report_path = candidate_dir / "parent-source-verification.json"
    source_report, source_raw = load_object(source_report_path, label="candidate source verification")
    source_sha = sha256_file(source_report_path)
    if full_digest(candidate.get("source_verification_sha256"), label="candidate source verification digest") != source_sha:
        fail("candidate source verification digest mismatch")
    if source_raw != canonical_bytes(source_report):
        fail("candidate source verification is not canonical JSON")
    sidecar_digest(source_report_path, label="candidate source verification")

    temporary_root = pathlib.Path(tempfile.mkdtemp(prefix="mycelix-health-lock-verify-"))
    worktree = temporary_root / "health"
    worktree_registered = False
    local_source_receipt = temporary_root / "parent-source.json"
    local_source_verification = temporary_root / "parent-source-verification.json"
    try:
        run("git", "-C", str(subject_checkout), "worktree", "add", "--detach", str(worktree), subject_sha)
        worktree_registered = True
        if git(worktree, "rev-parse", "HEAD") != subject_sha or git(worktree, "rev-parse", "HEAD^{tree}") != subject_tree:
            fail("verification worktree does not reproduce exact subject")
        profile = worktree / "release/ci-parent-source-profile-v1.json"
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
            str(local_source_receipt),
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
            str(local_source_receipt),
            "--verification",
            str(local_source_verification),
        )
        if sha256_file(local_source_verification) != source_sha:
            fail("independently reconstructed parent-source verification differs from candidate")

        shutil.copy2(candidate_lock_path, worktree / "Cargo.lock")
        system = current_system(worktree)
        if environment.get("system") != system:
            fail("candidate system identity differs from verifier environment")
        if full_digest(environment.get("flake_lock_sha256"), label="flake lock digest") != sha256_file(worktree / "flake.lock"):
            fail("candidate flake.lock digest mismatch")
        if full_digest(environment.get("rust_toolchain_sha256"), label="rust toolchain digest") != sha256_file(worktree / "rust-toolchain.toml"):
            fail("candidate rust-toolchain digest mismatch")
        ci_drv = run(
            "nix",
            "eval",
            "--raw",
            "--no-write-lock-file",
            f".#devShells.{system}.ci.drvPath",
            cwd=worktree,
        )
        if environment.get("ci_dev_shell_drv") != ci_drv:
            fail("candidate CI dev-shell derivation differs from verifier")

        rustc_vv = nix_ci(worktree, "rustc", "-vV")
        cargo_vv = nix_ci(worktree, "cargo", "-vV")
        locked_metadata = nix_ci(
            worktree,
            "cargo",
            "metadata",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
        )
        metadata_value = json.loads(locked_metadata)
        package_names = {str(item.get("name")) for item in metadata_value.get("packages", [])}
        missing = [name for name in required_packages if name not in package_names]
        if missing:
            fail(f"locked metadata is missing required package(s): {', '.join(missing)}")
        status = git(worktree, "status", "--porcelain=v1", "--untracked-files=all")
        unexpected = [line for line in status.splitlines() if line and not line.endswith(" Cargo.lock")]
        if unexpected:
            fail(f"verification changed tracked/untracked product files other than Cargo.lock: {unexpected}")
        diff = git(worktree, "diff", "--", "Cargo.lock")
        diagnostic_diff = candidate_dir / "Cargo.lock.diff"
        if diagnostic_diff.is_file():
            expected_diff = diagnostic_diff.read_text(encoding="utf-8").rstrip("\n")
            if diff != expected_diff:
                fail("candidate Cargo.lock.diff does not match independently reconstructed diff")

        verification = {
            "schema": VERIFICATION_SCHEMA,
            "authority": "VerifiedRepairCandidateOnly",
            "result": "pass",
            "candidate_report_sha256": candidate_report_sha,
            "subject": {"sha": subject_sha, "tree_sha": subject_tree},
            "harness": {
                "sha": harness_sha,
                "tree_sha": harness_tree,
                "verifier_sha256": sha256_file(pathlib.Path(__file__).resolve()),
            },
            "source_verification_sha256": source_sha,
            "locks": {
                "before_sha256": sha256_file(before_path),
                "candidate_sha256": sha256_file(candidate_lock_path),
            },
            "environment": {
                "system": system,
                "flake_lock_sha256": sha256_file(worktree / "flake.lock"),
                "rust_toolchain_sha256": sha256_file(worktree / "rust-toolchain.toml"),
                "ci_dev_shell_drv": ci_drv,
                "rustc_vV_sha256": sha256_bytes((rustc_vv + "\n").encode()),
                "cargo_vV_sha256": sha256_bytes((cargo_vv + "\n").encode()),
                "locked_metadata_sha256": sha256_bytes((locked_metadata + "\n").encode()),
            },
            "required_packages": required_packages,
            "audit": recomputed_audit,
            "non_claims": [
                "VerifiedRepairCandidateOnly is not permission to commit or merge the candidate lock.",
                "This verification is dependency-repair evidence, not repository qualification or clinical evidence.",
                "Registry/git movement still requires explicit review before a replacement exact subject is created.",
            ],
        }
        verification_bytes = canonical_bytes(verification)
        write_create_only(output, verification_bytes)
        write_create_only(pathlib.Path(str(output) + ".sha256"), (sha256_bytes(verification_bytes) + "\n").encode())
        return verification
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
    parser.add_argument("--parent-checkout", type=pathlib.Path, required=True)
    parser.add_argument("--candidate-dir", type=pathlib.Path, required=True)
    parser.add_argument("--verification", type=pathlib.Path, required=True)
    parser.add_argument("--require-package", action="append", default=[])
    return parser.parse_args()


def main() -> None:
    verification = verify(parse_args())
    print(json.dumps(verification, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
