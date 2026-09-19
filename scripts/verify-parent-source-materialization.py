#!/usr/bin/env python3
"""Independently verify parent-Mycelix source materialization evidence v1.

This verifier intentionally does not import the materializer. It independently
checks the source profile, Git identity, receipt canonicalization, export bytes,
contained symlink semantics, required manifests, and receipt sidecar before it
emits a create-only verification report.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import stat
import subprocess
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_PROFILE = ROOT / "release/ci-parent-source-profile-v1.json"
MATERIALIZER = ROOT / "scripts/materialize-parent-mycelix.py"
PROFILE_SCHEMA = "mycelix-health-parent-source-profile/v1"
RECEIPT_SCHEMA = "mycelix-health-parent-source-materialization/v1"
VERIFICATION_SCHEMA = "mycelix-health-parent-source-verification/v1"
EXPORT_DOMAIN = b"mycelix-health-export-digest/v1\0"
HEX_DIGITS = frozenset("0123456789abcdefABCDEF")


def fail(message: str) -> NoReturn:
    raise SystemExit(f"parent source verification failed: {message}")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    return sha256_bytes(path.read_bytes())


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def load_object(path: pathlib.Path, *, label: str) -> tuple[dict[str, Any], bytes]:
    try:
        raw = path.read_bytes()
        value = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot load {label} {path}: {error}")
    if not isinstance(value, dict):
        fail(f"{label} must contain a JSON object")
    return value, raw


def git(checkout: pathlib.Path, *args: str) -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(checkout), *args],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError) as error:
        fail(f"git {' '.join(args)} failed for {checkout}: {error}")
    return result.stdout.strip()


def normalize_remote(value: str) -> str:
    value = value.strip()
    for prefix in (
        "https://github.com/",
        "http://github.com/",
        "ssh://git@github.com/",
        "git@github.com:",
    ):
        if value.startswith(prefix):
            value = value[len(prefix) :]
            break
    if value.endswith(".git"):
        value = value[:-4]
    return value.strip("/")


def full_git_sha(value: Any, *, label: str) -> str:
    if not isinstance(value, str) or len(value) != 40 or any(char not in HEX_DIGITS for char in value):
        fail(f"{label} must be a full hexadecimal Git object ID")
    return value


def clean_relative(value: Any, *, label: str) -> pathlib.Path:
    if not isinstance(value, str):
        fail(f"{label} must be a string")
    path = pathlib.Path(value)
    if path.is_absolute() or not path.parts or any(part in {"", ".", ".."} for part in path.parts):
        fail(f"{label} must be a non-traversing relative path: {value!r}")
    return path


def destination_path(workspace: pathlib.Path, raw: Any) -> pathlib.Path:
    if not isinstance(raw, str):
        fail("destination must be a string")
    declared = pathlib.Path(raw)
    if declared.is_absolute() or len(declared.parts) < 2 or declared.parts[0] != "..":
        fail(f"destination must name a sibling below the workspace parent: {raw!r}")
    if any(part in {"", ".", ".."} for part in declared.parts[1:]):
        fail(f"destination contains traversal after leading '..': {raw!r}")
    workspace = workspace.resolve()
    parent = workspace.parent.resolve()
    resolved = (workspace / declared).resolve()
    try:
        resolved.relative_to(parent)
    except ValueError:
        fail(f"destination escapes workspace parent: {raw!r}")
    try:
        resolved.relative_to(workspace)
    except ValueError:
        pass
    else:
        fail(f"destination resolves inside Health workspace: {raw!r}")
    if resolved == parent:
        fail(f"destination resolves to workspace parent: {raw!r}")
    return resolved


def validate_links(root: pathlib.Path) -> None:
    root = root.resolve()
    for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix()):
        info = os.lstat(path)
        if not stat.S_ISLNK(info.st_mode):
            continue
        target = pathlib.Path(os.readlink(path))
        if target.is_absolute():
            fail(f"absolute symlink in export: {path.relative_to(root)} -> {target}")
        resolved = (path.parent / target).resolve(strict=False)
        try:
            resolved.relative_to(root)
        except ValueError:
            fail(f"symlink escapes export: {path.relative_to(root)} -> {target}")


def export_digest(root: pathlib.Path) -> str:
    root = root.resolve()
    if not root.is_dir():
        fail(f"export root is not a directory: {root}")
    validate_links(root)
    digest = hashlib.sha256()
    digest.update(EXPORT_DOMAIN)
    for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix()):
        relative = path.relative_to(root).as_posix().encode()
        info = os.lstat(path)
        if stat.S_ISLNK(info.st_mode):
            digest.update(b"L\0" + relative + b"\0" + os.readlink(path).encode() + b"\0")
        elif stat.S_ISREG(info.st_mode):
            executable = b"1" if info.st_mode & 0o111 else b"0"
            digest.update(b"F\0" + relative + b"\0" + executable + b"\0")
            with path.open("rb") as handle:
                while True:
                    block = handle.read(1024 * 1024)
                    if not block:
                        break
                    digest.update(block)
            digest.update(b"\0")
        elif stat.S_ISDIR(info.st_mode):
            continue
        else:
            fail(f"unsupported filesystem entry in export: {path}")
    return digest.hexdigest()


def read_sidecar(receipt_path: pathlib.Path) -> str:
    sidecar = pathlib.Path(str(receipt_path) + ".sha256")
    try:
        value = sidecar.read_text(encoding="utf-8").strip()
    except OSError as error:
        fail(f"cannot read receipt sidecar {sidecar}: {error}")
    if len(value) != 64 or any(char not in HEX_DIGITS for char in value):
        fail("receipt sidecar must contain one hexadecimal SHA-256 digest")
    return value.lower()


def write_create_only(path: pathlib.Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() or path.is_symlink():
        fail(f"refusing to overwrite verification artifact: {path}")
    try:
        with path.open("xb") as handle:
            handle.write(data)
    except FileExistsError:
        fail(f"refusing to overwrite verification artifact: {path}")


def verify(args: argparse.Namespace) -> dict[str, Any]:
    checkout = args.checkout.resolve()
    workspace = args.workspace_root.resolve()
    profile_path = args.profile.resolve()
    receipt_path = args.receipt.resolve()
    verification_path = args.verification.resolve()

    profile, profile_raw = load_object(profile_path, label="source profile")
    receipt, receipt_raw = load_object(receipt_path, label="materialization receipt")
    if profile.get("schema") != PROFILE_SCHEMA:
        fail(f"unsupported profile schema: {profile.get('schema')!r}")
    if receipt.get("schema") != RECEIPT_SCHEMA:
        fail(f"unsupported receipt schema: {receipt.get('schema')!r}")
    if receipt_raw != canonical_bytes(receipt):
        fail("materialization receipt is not in canonical JSON representation")
    receipt_sha = sha256_bytes(receipt_raw)
    if read_sidecar(receipt_path) != receipt_sha:
        fail("materialization receipt sidecar digest mismatch")

    if receipt.get("profile_sha256") != sha256_bytes(profile_raw):
        fail("receipt profile digest does not match supplied profile bytes")
    if receipt.get("materializer_sha256") != sha256_file(MATERIALIZER):
        fail("receipt materializer digest does not match current materializer bytes")
    if receipt.get("profile_id") != profile.get("profile_id"):
        fail("receipt profile_id differs from supplied profile")

    declared = receipt.get("declared_source")
    observed = receipt.get("observed_source")
    if not isinstance(declared, dict) or not isinstance(observed, dict):
        fail("receipt source identity objects are missing")
    expected_declared = {
        "repository": profile.get("repository"),
        "commit_sha": profile.get("commit_sha"),
        "tree_sha": profile.get("tree_sha"),
    }
    if declared != expected_declared:
        fail("receipt declared source differs from source profile")
    full_git_sha(declared.get("commit_sha"), label="declared commit")
    full_git_sha(declared.get("tree_sha"), label="declared tree")

    if not checkout.is_dir():
        fail(f"parent checkout is missing: {checkout}")
    actual_repository = normalize_remote(git(checkout, "config", "--get", "remote.origin.url"))
    actual_commit = full_git_sha(git(checkout, "rev-parse", "HEAD"), label="observed commit")
    actual_tree = full_git_sha(git(checkout, "rev-parse", "HEAD^{tree}"), label="observed tree")
    if git(checkout, "status", "--porcelain=v1", "--untracked-files=all"):
        fail("parent checkout is dirty during verification")
    if actual_repository.lower() != str(profile.get("repository", "")).lower():
        fail("verified checkout repository differs from source profile")
    expected_observed = {
        "repository": actual_repository,
        "commit_sha": actual_commit,
        "tree_sha": actual_tree,
        "clean_checkout": True,
    }
    if observed != expected_observed:
        fail("receipt observed source differs from independently observed checkout")

    mode = receipt.get("mode")
    authority = receipt.get("authority")
    if mode == "pinned":
        if authority != "PinnedDependencySourceMaterialized":
            fail("pinned receipt has incorrect authority")
        if actual_commit != declared["commit_sha"] or actual_tree != declared["tree_sha"]:
            fail("pinned receipt source no longer matches declared commit/tree")
        verified_authority = "VerifiedPinnedDependencySourceMaterialization"
    elif mode == "compatibility":
        if authority != "CompatibilityOnly":
            fail("compatibility receipt has incorrect authority")
        verified_authority = "VerifiedCompatibilityObservation"
    else:
        fail(f"unsupported receipt mode: {mode!r}")

    profile_exports = profile.get("exports")
    receipt_exports = receipt.get("exports")
    if not isinstance(profile_exports, list) or not isinstance(receipt_exports, list):
        fail("profile/receipt exports must be lists")
    if len(profile_exports) != len(receipt_exports):
        fail("receipt export count differs from source profile")

    resolved_destinations: set[pathlib.Path] = set()
    verified_exports: list[dict[str, Any]] = []
    for index, (declared_export, receipt_export) in enumerate(zip(profile_exports, receipt_exports, strict=True)):
        if not isinstance(declared_export, dict) or not isinstance(receipt_export, dict):
            fail(f"export {index} is not an object")
        source_rel = clean_relative(declared_export.get("source"), label=f"exports[{index}].source")
        destination_raw = declared_export.get("destination")
        if receipt_export.get("source") != source_rel.as_posix() or receipt_export.get("destination") != destination_raw:
            fail(f"receipt export {index} identity differs from profile")

        source = (checkout / source_rel).resolve()
        try:
            source.relative_to(checkout)
        except ValueError:
            fail(f"source export escapes checkout: {source_rel}")
        destination = destination_path(workspace, destination_raw)
        if destination in resolved_destinations:
            fail(f"multiple exports resolve to one destination: {destination}")
        resolved_destinations.add(destination)

        source_sha = export_digest(source)
        destination_sha = export_digest(destination)
        if source_sha != destination_sha:
            fail(f"source/destination export digest mismatch at index {index}")
        if receipt_export.get("content_sha256") != source_sha:
            fail(f"receipt export digest mismatch at index {index}")

        required = declared_export.get("required_manifests")
        if not isinstance(required, list) or receipt_export.get("required_manifests") != required:
            fail(f"required manifest set differs at export {index}")
        for manifest_raw in required:
            manifest_rel = clean_relative(manifest_raw, label=f"exports[{index}].required_manifest")
            manifest = (destination / manifest_rel).resolve()
            try:
                manifest.relative_to(destination)
            except ValueError:
                fail(f"required manifest escapes destination at export {index}: {manifest_raw}")
            if not manifest.is_file():
                fail(f"required manifest missing at export {index}: {manifest_raw}")

        verified_exports.append(
            {
                "source": source_rel.as_posix(),
                "destination": destination_raw,
                "content_sha256": source_sha,
            }
        )

    report = {
        "schema": VERIFICATION_SCHEMA,
        "authority": verified_authority,
        "mode": mode,
        "source_receipt_sha256": receipt_sha,
        "profile_sha256": sha256_bytes(profile_raw),
        "materializer_sha256": sha256_file(MATERIALIZER),
        "verifier_sha256": sha256_file(pathlib.Path(__file__).resolve()),
        "observed_source": expected_observed,
        "exports": verified_exports,
        "non_claims": [
            "Source verification is not Cargo.lock consistency or build/test qualification.",
            "Compatibility observations cannot satisfy pinned exact-subject qualification.",
            "Source verification establishes no clinical validity, safety, authority, or regulatory status.",
        ],
    }
    report_bytes = canonical_bytes(report)
    sidecar_path = pathlib.Path(str(verification_path) + ".sha256")
    if sidecar_path.exists() or sidecar_path.is_symlink():
        fail(f"refusing to overwrite verification sidecar: {sidecar_path}")
    write_create_only(verification_path, report_bytes)
    write_create_only(sidecar_path, (sha256_bytes(report_bytes) + "\n").encode())
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", type=pathlib.Path, required=True)
    parser.add_argument("--workspace-root", type=pathlib.Path, default=ROOT)
    parser.add_argument("--profile", type=pathlib.Path, default=DEFAULT_PROFILE)
    parser.add_argument("--receipt", type=pathlib.Path, required=True)
    parser.add_argument("--verification", type=pathlib.Path, required=True)
    return parser.parse_args()


def main() -> None:
    report = verify(parse_args())
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
