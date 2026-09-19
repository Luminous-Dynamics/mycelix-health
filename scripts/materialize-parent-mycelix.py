#!/usr/bin/env python3
"""Materialize parent-Mycelix path dependencies with explicit source identity.

This helper separates two evidence classes:

- pinned: the checkout commit/tree must equal the committed source profile;
- compatibility: any clean observed parent checkout may be sampled, but the
  resulting receipt is explicitly CompatibilityOnly.

The helper establishes dependency-source/materialization identity only. It does
not run Cargo and cannot emit build, test, qualification, or clinical authority.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import shutil
import stat
import subprocess
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_PROFILE = ROOT / "release/ci-parent-source-profile-v1.json"
PROFILE_SCHEMA = "mycelix-health-parent-source-profile/v1"
RECEIPT_SCHEMA = "mycelix-health-parent-source-materialization/v1"
HEX_DIGITS = frozenset("0123456789abcdefABCDEF")


def fail(message: str) -> NoReturn:
    raise SystemExit(f"parent source materialization failed: {message}")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    return sha256_bytes(path.read_bytes())


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


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
    prefixes = (
        "https://github.com/",
        "http://github.com/",
        "ssh://git@github.com/",
        "git@github.com:",
    )
    for prefix in prefixes:
        if value.startswith(prefix):
            value = value[len(prefix) :]
            break
    if value.endswith(".git"):
        value = value[:-4]
    return value.strip("/")


def validate_git_sha(value: Any, *, field: str) -> str:
    if not isinstance(value, str) or len(value) != 40 or any(char not in HEX_DIGITS for char in value):
        fail(f"{field} must be a full 40-character hexadecimal Git object ID")
    return value


def clean_relative(value: str, *, field: str) -> pathlib.Path:
    path = pathlib.Path(value)
    if path.is_absolute() or not path.parts or any(part in {"", ".", ".."} for part in path.parts):
        fail(f"{field} must be a non-traversing relative path: {value!r}")
    return path


def sibling_destination(workspace: pathlib.Path, value: str) -> pathlib.Path:
    path = pathlib.Path(value)
    if path.is_absolute() or len(path.parts) < 2 or path.parts[0] != "..":
        fail(f"destination must name a sibling below the workspace parent: {value!r}")
    if any(part in {"", ".", ".."} for part in path.parts[1:]):
        fail(f"destination contains traversal after its leading '..': {value!r}")

    workspace = workspace.resolve()
    parent = workspace.parent.resolve()
    target = (workspace / path).resolve()
    try:
        target.relative_to(parent)
    except ValueError:
        fail(f"destination escapes workspace parent: {value!r}")
    try:
        target.relative_to(workspace)
    except ValueError:
        pass
    else:
        fail(f"destination resolves back inside the health workspace: {value!r}")
    if target == parent:
        fail(f"destination may not replace the workspace parent: {value!r}")
    return target


def load_profile(path: pathlib.Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot load source profile {path}: {error}")
    if not isinstance(value, dict):
        fail("source profile must be a JSON object")
    if value.get("schema") != PROFILE_SCHEMA:
        fail(f"unsupported source-profile schema: {value.get('schema')!r}")

    for key in ("profile_id", "repository", "commit_sha", "tree_sha", "exports"):
        if key not in value:
            fail(f"source profile missing {key!r}")
    for key in ("profile_id", "repository"):
        if not isinstance(value[key], str) or not value[key].strip():
            fail(f"source profile {key} must be a non-empty string")
    validate_git_sha(value["commit_sha"], field="source profile commit_sha")
    validate_git_sha(value["tree_sha"], field="source profile tree_sha")
    if not isinstance(value["exports"], list) or not value["exports"]:
        fail("source profile exports must be a non-empty list")

    seen_destination_text: set[str] = set()
    for index, export in enumerate(value["exports"]):
        if not isinstance(export, dict):
            fail(f"export {index} must be an object")
        for key in ("source", "destination", "required_manifests"):
            if key not in export:
                fail(f"export {index} missing {key!r}")
        if not isinstance(export["source"], str) or not isinstance(export["destination"], str):
            fail(f"export {index} source/destination must be strings")
        clean_relative(export["source"], field=f"exports[{index}].source")
        destination = export["destination"]
        if destination in seen_destination_text:
            fail(f"duplicate destination in source profile: {destination}")
        seen_destination_text.add(destination)
        manifests = export["required_manifests"]
        if not isinstance(manifests, list) or not manifests:
            fail(f"export {index} required_manifests must be a non-empty list")
        for manifest in manifests:
            if not isinstance(manifest, str):
                fail(f"export {index} required_manifests entries must be strings")
            clean_relative(manifest, field=f"exports[{index}].required_manifests")
    return value


def validate_symlink_containment(root: pathlib.Path) -> None:
    """Reject links whose resolved target can escape the exported source tree."""
    root = root.resolve()
    for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix()):
        info = os.lstat(path)
        if not stat.S_ISLNK(info.st_mode):
            continue
        target = pathlib.Path(os.readlink(path))
        if target.is_absolute():
            fail(f"absolute symlink is not allowed in export {root}: {path.relative_to(root)} -> {target}")
        resolved = (path.parent / target).resolve(strict=False)
        try:
            resolved.relative_to(root)
        except ValueError:
            fail(f"symlink escapes export {root}: {path.relative_to(root)} -> {target}")


def export_digest(root: pathlib.Path) -> str:
    """Digest relative paths, executable semantics, file bytes, and contained symlinks."""
    root = root.resolve()
    validate_symlink_containment(root)
    digest = hashlib.sha256()
    digest.update(b"mycelix-health-export-digest/v1\0")
    entries = sorted(root.rglob("*"), key=lambda path: path.relative_to(root).as_posix())
    for path in entries:
        relative = path.relative_to(root).as_posix().encode()
        info = os.lstat(path)
        if stat.S_ISLNK(info.st_mode):
            digest.update(b"L\0" + relative + b"\0" + os.readlink(path).encode() + b"\0")
        elif stat.S_ISREG(info.st_mode):
            executable = b"1" if info.st_mode & 0o111 else b"0"
            digest.update(b"F\0" + relative + b"\0" + executable + b"\0")
            with path.open("rb") as handle:
                for block in iter(lambda: handle.read(1024 * 1024), b""):
                    digest.update(block)
            digest.update(b"\0")
        elif stat.S_ISDIR(info.st_mode):
            continue
        else:
            fail(f"unsupported filesystem entry in export {root}: {path}")
    return digest.hexdigest()


def remove_existing(path: pathlib.Path) -> None:
    if path.is_symlink() or path.is_file():
        path.unlink()
    elif path.exists():
        shutil.rmtree(path)


def ensure_create_only(path: pathlib.Path) -> None:
    if path.exists() or path.is_symlink():
        fail(f"refusing to overwrite existing receipt artifact: {path}")


def write_create_only(path: pathlib.Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with path.open("xb") as handle:
            handle.write(data)
    except FileExistsError:
        fail(f"refusing to overwrite existing receipt artifact: {path}")


def materialize(args: argparse.Namespace) -> dict[str, Any]:
    profile_path = args.profile.resolve()
    checkout = args.checkout.resolve()
    workspace = args.workspace_root.resolve()
    receipt_path = args.receipt.resolve()
    sidecar_path = pathlib.Path(str(receipt_path) + ".sha256")
    profile = load_profile(profile_path)

    if not checkout.is_dir():
        fail(f"parent checkout does not exist: {checkout}")
    if not workspace.is_dir():
        fail(f"health workspace does not exist: {workspace}")
    ensure_create_only(receipt_path)
    ensure_create_only(sidecar_path)

    observed_commit = git(checkout, "rev-parse", "HEAD")
    observed_tree = git(checkout, "rev-parse", "HEAD^{tree}")
    validate_git_sha(observed_commit, field="observed parent commit")
    validate_git_sha(observed_tree, field="observed parent tree")
    dirty = git(checkout, "status", "--porcelain=v1", "--untracked-files=all")
    if dirty:
        fail("parent checkout is dirty; materialized bytes would not equal the observed Git tree")

    remote = git(checkout, "config", "--get", "remote.origin.url")
    observed_repository = normalize_remote(remote)
    if observed_repository.lower() != profile["repository"].lower():
        fail(
            f"parent repository mismatch: expected {profile['repository']!r}, "
            f"observed {observed_repository!r}"
        )

    if args.mode == "pinned":
        if observed_commit != profile["commit_sha"]:
            fail(f"parent commit mismatch: expected {profile['commit_sha']}, observed {observed_commit}")
        if observed_tree != profile["tree_sha"]:
            fail(f"parent tree mismatch: expected {profile['tree_sha']}, observed {observed_tree}")
        authority = "PinnedDependencySourceMaterialized"
    else:
        authority = "CompatibilityOnly"

    planned_exports: list[tuple[dict[str, Any], pathlib.Path, pathlib.Path, pathlib.Path]] = []
    resolved_destinations: set[pathlib.Path] = set()
    for export in profile["exports"]:
        source_rel = clean_relative(export["source"], field="export.source")
        source = (checkout / source_rel).resolve()
        try:
            source.relative_to(checkout)
        except ValueError:
            fail(f"export source escapes parent checkout: {source_rel}")
        if not source.is_dir():
            fail(f"export source is not a directory: {source_rel}")
        destination = sibling_destination(workspace, export["destination"])
        if destination in resolved_destinations:
            fail(f"multiple declared destinations resolve to the same path: {destination}")
        resolved_destinations.add(destination)
        for evidence_path in (receipt_path, sidecar_path):
            try:
                evidence_path.relative_to(destination)
            except ValueError:
                pass
            else:
                fail(f"receipt artifact may not be written inside materialized destination: {evidence_path}")
        planned_exports.append((export, source_rel, source, destination))

    export_receipts: list[dict[str, Any]] = []
    for export, source_rel, source, destination in planned_exports:
        source_digest = export_digest(source)
        if destination.exists() or destination.is_symlink():
            if not args.replace_existing:
                fail(f"destination already exists; pass --replace-existing explicitly: {destination}")
            remove_existing(destination)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(source, destination, symlinks=True)
        destination_digest = export_digest(destination)
        if destination_digest != source_digest:
            fail(
                f"materialized export digest mismatch for {source_rel}: "
                f"source {source_digest}, destination {destination_digest}"
            )

        manifests: list[str] = []
        for raw_manifest in export["required_manifests"]:
            manifest_rel = clean_relative(raw_manifest, field="required_manifest")
            manifest = (destination / manifest_rel).resolve()
            try:
                manifest.relative_to(destination)
            except ValueError:
                fail(f"required manifest escapes destination: {raw_manifest}")
            if not manifest.is_file():
                fail(f"required manifest missing after materialization: {raw_manifest}")
            manifests.append(manifest_rel.as_posix())

        export_receipts.append(
            {
                "source": source_rel.as_posix(),
                "destination": export["destination"],
                "content_sha256": source_digest,
                "required_manifests": manifests,
            }
        )

    receipt = {
        "schema": RECEIPT_SCHEMA,
        "profile_id": profile["profile_id"],
        "profile_sha256": sha256_file(profile_path),
        "materializer_sha256": sha256_file(pathlib.Path(__file__).resolve()),
        "mode": args.mode,
        "authority": authority,
        "declared_source": {
            "repository": profile["repository"],
            "commit_sha": profile["commit_sha"],
            "tree_sha": profile["tree_sha"],
        },
        "observed_source": {
            "repository": observed_repository,
            "commit_sha": observed_commit,
            "tree_sha": observed_tree,
            "clean_checkout": True,
        },
        "exports": export_receipts,
        "claims": [
            "The recorded parent checkout was clean when sibling sources were materialized.",
            "Every symlink in the exported source graph remained contained within its export root.",
            "Each copied export matched the source export digest after materialization.",
        ],
        "non_claims": profile.get("non_claims", []),
    }

    receipt_bytes = canonical_bytes(receipt)
    write_create_only(receipt_path, receipt_bytes)
    write_create_only(sidecar_path, (sha256_bytes(receipt_bytes) + "\n").encode())
    return receipt


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", type=pathlib.Path, required=True, help="clean parent Mycelix Git checkout")
    parser.add_argument("--workspace-root", type=pathlib.Path, default=ROOT, help="mycelix-health workspace root")
    parser.add_argument("--profile", type=pathlib.Path, default=DEFAULT_PROFILE)
    parser.add_argument("--mode", choices=("pinned", "compatibility"), default="pinned")
    parser.add_argument("--receipt", type=pathlib.Path, required=True)
    parser.add_argument(
        "--replace-existing",
        action="store_true",
        help="explicitly replace the declared sibling destinations before materialization",
    )
    return parser.parse_args()


def main() -> None:
    receipt = materialize(parse_args())
    print(json.dumps(receipt, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
