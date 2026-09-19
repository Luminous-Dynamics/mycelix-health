#!/usr/bin/env python3
"""End-to-end adversarial self-test for portable Cargo.lock candidate v1.

The test uses miniature Git repositories plus a deterministic fake `nix` command,
so it exercises exact-subject detached worktrees, source materialization, candidate
generation, lock movement audit, and independent executable verification without
network/package downloads.
"""
from __future__ import annotations

import hashlib
import json
import os
import pathlib
import subprocess
import tempfile
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
GENERATOR = ROOT / "scripts/prepare-cargo-lock-candidate.py"
VERIFIER = ROOT / "scripts/verify-cargo-lock-candidate.py"
TARGET_PACKAGE = "mycelix-symthaea-clinical-distribution-trust-v3"


def command(*args: str, cwd: pathlib.Path | None = None, env: dict[str, str] | None = None, check: bool = True):
    return subprocess.run(args, cwd=cwd, env=env, check=check, capture_output=True, text=True)


def git(repo: pathlib.Path, *args: str) -> str:
    return command("git", *args, cwd=repo).stdout.strip()


def init_repo(repo: pathlib.Path, name: str) -> None:
    repo.mkdir(parents=True)
    git(repo, "init")
    git(repo, "config", "user.email", "portable-lock-test@example.invalid")
    git(repo, "config", "user.name", name)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def write_parent(root: pathlib.Path) -> tuple[pathlib.Path, str, str]:
    parent = root / "parent"
    init_repo(parent, "Portable Parent Fixture")
    git(parent, "remote", "add", "origin", "https://github.com/Luminous-Dynamics/mycelix.git")
    manifest = parent / "crates/fixture-dependency/Cargo.toml"
    manifest.parent.mkdir(parents=True)
    manifest.write_text("[package]\nname='fixture-dependency'\nversion='0.1.0'\n", encoding="utf-8")
    git(parent, "add", ".")
    git(parent, "commit", "-m", "fixture parent")
    return parent, git(parent, "rev-parse", "HEAD"), git(parent, "rev-parse", "HEAD^{tree}")


def write_product(root: pathlib.Path, parent_sha: str, parent_tree: str) -> tuple[pathlib.Path, str, bytes]:
    product = root / "product"
    init_repo(product, "Portable Product Fixture")
    profile = {
        "schema": "mycelix-health-parent-source-profile/v1",
        "profile_id": "portable-lock-test-v1",
        "authority": "DeclaredDependencySourceIdentityOnly",
        "repository": "Luminous-Dynamics/mycelix",
        "commit_sha": parent_sha,
        "tree_sha": parent_tree,
        "exports": [
            {
                "source": "crates",
                "destination": "../crates",
                "required_manifests": ["fixture-dependency/Cargo.toml"],
            }
        ],
        "non_claims": ["fixture only"],
    }
    files = {
        "Cargo.toml": "[workspace]\nmembers=[]\nresolver='2'\n",
        "Cargo.lock": "version = 4\n\n[[package]]\nname = \"existing-local\"\nversion = \"0.1.0\"\n",
        "flake.lock": "{}\n",
        "rust-toolchain.toml": "[toolchain]\nchannel='1.96.0'\n",
        "release/ci-parent-source-profile-v1.json": json.dumps(profile, indent=2, sort_keys=True) + "\n",
    }
    for relative, content in files.items():
        target = product / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
    git(product, "add", ".")
    git(product, "commit", "-m", "fixture product")
    return product, git(product, "rev-parse", "HEAD"), (product / "Cargo.lock").read_bytes()


def fake_nix(root: pathlib.Path) -> pathlib.Path:
    bindir = root / "bin"
    bindir.mkdir()
    script = bindir / "nix"
    script.write_text(
        f'''#!/usr/bin/env python3
import json, pathlib, sys
args = sys.argv[1:]
if args[:4] == ["eval", "--impure", "--raw", "--expr"]:
    print("x86_64-linux", end="")
    raise SystemExit(0)
if args[:4] == ["flake", "metadata", "--json", "--no-write-lock-file"]:
    print(json.dumps({{"fixture": True}}))
    raise SystemExit(0)
if len(args) >= 4 and args[:3] == ["eval", "--raw", "--no-write-lock-file"]:
    print("/nix/store/fake-mycelix-health-ci.drv", end="")
    raise SystemExit(0)
if len(args) >= 5 and args[:4] == ["develop", "--no-write-lock-file", ".#ci", "--command"]:
    cmd = args[4:]
    if cmd == ["rustc", "-vV"]:
        print("rustc 1.96.0 (fixture)\\nhost: x86_64-unknown-linux-gnu")
        raise SystemExit(0)
    if cmd == ["cargo", "-vV"]:
        print("cargo 1.96.0 (fixture)\\nhost: x86_64-unknown-linux-gnu")
        raise SystemExit(0)
    if cmd[:2] == ["cargo", "metadata"]:
        locked = "--locked" in cmd
        lock = pathlib.Path.cwd() / "Cargo.lock"
        candidate = ''' + repr(
            "version = 4\n\n"
            "[[package]]\nname = \"existing-local\"\nversion = \"0.1.0\"\n\n"
            "[[package]]\nname = \"mycelix-symthaea-clinical-distribution-trust-v3\"\nversion = \"0.1.0\"\n\n"
            "[[package]]\nname = \"fixture-registry\"\nversion = \"2.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"0000000000000000000000000000000000000000000000000000000000000000\"\n"
        ) + '''
        if not locked:
            lock.write_text(candidate)
        elif lock.read_text() != candidate:
            print("candidate lock not installed", file=sys.stderr)
            raise SystemExit(9)
        print(json.dumps({{"packages": [
            {{"name": "existing-local", "version": "0.1.0"}},
            {{"name": "''' + TARGET_PACKAGE + '''", "version": "0.1.0"}},
            {{"name": "fixture-registry", "version": "2.0.0"}}
        ]}}))
        raise SystemExit(0)
print("unsupported fake nix invocation: " + repr(args), file=sys.stderr)
raise SystemExit(8)
''',
        encoding="utf-8",
    )
    script.chmod(0o755)
    return bindir


def run_generator(product: pathlib.Path, subject: str, parent: pathlib.Path, output: pathlib.Path, env: dict[str, str]):
    return command(
        "python3",
        str(GENERATOR),
        "--subject-checkout",
        str(product),
        "--subject-sha",
        subject,
        "--parent-checkout",
        str(parent),
        "--output-dir",
        str(output),
        "--require-package",
        TARGET_PACKAGE,
        env=env,
        check=False,
    )


def run_verifier(product: pathlib.Path, parent: pathlib.Path, output: pathlib.Path, env: dict[str, str], verification_name: str = "verification.json"):
    return command(
        "python3",
        str(VERIFIER),
        "--subject-checkout",
        str(product),
        "--parent-checkout",
        str(parent),
        "--candidate-dir",
        str(output),
        "--verification",
        str(output / verification_name),
        "--require-package",
        TARGET_PACKAGE,
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


def rewrite_candidate_report(output: pathlib.Path, mutate) -> None:
    path = output / "lock-candidate.json"
    value = json.loads(path.read_text(encoding="utf-8"))
    mutate(value)
    raw = canonical_bytes(value)
    path.write_bytes(raw)
    (output / "lock-candidate.json.sha256").write_text(sha256_bytes(raw) + "\n", encoding="utf-8")


def main() -> None:
    with tempfile.TemporaryDirectory() as raw:
        root = pathlib.Path(raw)
        parent, parent_sha, parent_tree = write_parent(root)
        product, subject, original_lock = write_product(root, parent_sha, parent_tree)
        bindir = fake_nix(root)
        env = os.environ.copy()
        env["PATH"] = str(bindir) + os.pathsep + env.get("PATH", "")

        output = root / "candidate"
        expect_success(run_generator(product, subject, parent, output, env))
        assert (product / "Cargo.lock").read_bytes() == original_lock
        assert not git(product, "status", "--porcelain=v1", "--untracked-files=all")
        report = json.loads((output / "lock-candidate.json").read_text(encoding="utf-8"))
        assert report["authority"] == "RepairCandidateOnly"
        assert report["locks"]["changed"] is True
        assert [TARGET_PACKAGE, "0.1.0"] in report["audit"]["added_local_packages"]
        expect_success(run_verifier(product, parent, output, env))
        verification = json.loads((output / "verification.json").read_text(encoding="utf-8"))
        assert verification["authority"] == "VerifiedRepairCandidateOnly"

        # Candidate bytes cannot be changed after generation.
        (output / "Cargo.lock.candidate").write_text("version = 4\n", encoding="utf-8")
        expect_failure(run_verifier(product, parent, output, env, "tampered-lock.json"), "candidate lock digest mismatch")

        # Restore the deterministic candidate by regenerating a new independent case.
        root2 = pathlib.Path(raw) / "case2"
        root2.mkdir()
        parent2, parent_sha2, parent_tree2 = write_parent(root2)
        product2, subject2, _ = write_product(root2, parent_sha2, parent_tree2)
        output2 = root2 / "candidate"
        expect_success(run_generator(product2, subject2, parent2, output2, env))

        # Semantic report tampering remains detectable even with a recomputed sidecar.
        rewrite_candidate_report(output2, lambda value: value["audit"].__setitem__("added_local_packages", []))
        expect_failure(run_verifier(product2, parent2, output2, env), "audit does not match")

        root3 = pathlib.Path(raw) / "case3"
        root3.mkdir()
        parent3, parent_sha3, parent_tree3 = write_parent(root3)
        product3, subject3, _ = write_product(root3, parent_sha3, parent_tree3)
        output3 = root3 / "candidate"
        expect_success(run_generator(product3, subject3, parent3, output3, env))
        expect_failure(
            command(
                "python3",
                str(VERIFIER),
                "--subject-checkout",
                str(product3),
                "--parent-checkout",
                str(parent3),
                "--candidate-dir",
                str(output3),
                "--verification",
                str(output3 / "wrong-package.json"),
                "--require-package",
                "different-package",
                env=env,
                check=False,
            ),
            "required package set differs",
        )

    print("portable Cargo.lock candidate adversarial self-test: PASS")


if __name__ == "__main__":
    main()
