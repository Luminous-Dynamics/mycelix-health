#!/usr/bin/env python3
"""Compare two staged Health release builds byte-for-byte and emit a create-only report."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import tempfile
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
POLICY = ROOT / "release/reproducibility-policy.json"


def fail(message: str) -> "NoReturn":
    raise SystemExit(f"release reproducibility comparison failed: {message}")


def load(path: pathlib.Path) -> dict[str, Any]:
    value = json.loads(path.read_text())
    if not isinstance(value, dict):
        fail(f"{path} must contain an object")
    return value


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def files(root: pathlib.Path) -> set[str]:
    return {
        path.relative_to(root).as_posix()
        for path in root.rglob("*")
        if path.is_file()
    }


def compare(first: pathlib.Path, second: pathlib.Path, policy_path: pathlib.Path, first_context: pathlib.Path | None = None, second_context: pathlib.Path | None = None) -> dict[str, Any]:
    policy = load(policy_path)
    if first.resolve() == second.resolve() and policy.get("require_distinct_build_roots") is True:
        fail("build roots must be distinct")
    required = policy.get("required_artifacts")
    if not isinstance(required, list) or not required or len(required) != len(set(required)):
        fail("reproducibility policy has no unique artifact list")
    first_files = files(first)
    second_files = files(second)
    required_set = set(required)
    missing_first = sorted(required_set - first_files)
    missing_second = sorted(required_set - second_files)
    extra_first = sorted(first_files - required_set)
    extra_second = sorted(second_files - required_set)
    comparisons = []
    mismatches = []
    for relative in sorted(required):
        left = first / relative
        right = second / relative
        if not left.is_file() or not right.is_file():
            continue
        left_hash = sha256(left)
        right_hash = sha256(right)
        match = left_hash == right_hash and left.stat().st_size == right.stat().st_size
        item = {
            "artifact": relative,
            "first_sha256": left_hash,
            "second_sha256": right_hash,
            "first_size_bytes": left.stat().st_size,
            "second_size_bytes": right.stat().st_size,
            "byte_identical": match,
        }
        comparisons.append(item)
        if not match:
            mismatches.append(relative)
    forbid_extra = policy.get("forbid_extra_artifacts") is True
    failures = []
    if missing_first:
        failures.append("first build is missing required artifacts")
    if missing_second:
        failures.append("second build is missing required artifacts")
    if forbid_extra and extra_first:
        failures.append("first build contains unreviewed artifacts")
    if forbid_extra and extra_second:
        failures.append("second build contains unreviewed artifacts")
    if mismatches:
        failures.append("release artifacts are not byte-identical")
    context_comparisons = []
    context_mismatches = []
    required_context = policy.get("required_context_files", [])
    if required_context:
        if not first_context or not second_context:
            failures.append("release context roots were not provided")
        else:
            for relative in required_context:
                left = first_context / relative
                right = second_context / relative
                if not left.is_file() or not right.is_file():
                    context_mismatches.append(relative)
                    continue
                left_hash = sha256(left)
                right_hash = sha256(right)
                identical = left_hash == right_hash and left.stat().st_size == right.stat().st_size
                context_comparisons.append({
                    "artifact": relative,
                    "first_sha256": left_hash,
                    "second_sha256": right_hash,
                    "byte_identical": identical,
                })
                if not identical:
                    context_mismatches.append(relative)
            if context_mismatches:
                failures.append("release build contexts are not byte-identical")
    return {
        "schema_version": 1,
        "release_id": policy.get("release_id"),
        "status": "verified" if not failures else "refused",
        "policy_sha256": sha256(policy_path),
        "first_root": str(first),
        "second_root": str(second),
        "required_artifact_count": len(required),
        "comparisons": comparisons,
        "missing_first": missing_first,
        "missing_second": missing_second,
        "extra_first": extra_first,
        "extra_second": extra_second,
        "mismatches": mismatches,
        "context_comparisons": context_comparisons,
        "context_mismatches": context_mismatches,
        "failures": failures,
    }


def write_create_only(path: pathlib.Path, value: dict[str, Any]) -> None:
    if path.exists():
        fail(f"refusing to overwrite {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x", encoding="utf-8") as handle:
        json.dump(value, handle, indent=2, sort_keys=True)
        handle.write("\n")
    os.chmod(path, 0o600)


def self_test() -> None:
    policy = load(POLICY)
    with tempfile.TemporaryDirectory() as raw:
        root = pathlib.Path(raw)
        first = root / "first"
        second = root / "second"
        for directory in (first, second):
            for relative in policy["required_artifacts"]:
                path = directory / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(f"artifact:{relative}\n".encode())
        for directory in (first.parent / "context-a", first.parent / "context-b"):
            directory.mkdir()
            (directory / "build-context.json").write_text("{}\n")
            (directory / "nix-closure.json").write_text("{}\n")
        report = compare(first, second, POLICY, first.parent / "context-a", first.parent / "context-b")
        assert report["status"] == "verified"
        (second / policy["required_artifacts"][0]).write_bytes(b"tampered")
        report = compare(first, second, POLICY, first.parent / "context-a", first.parent / "context-b")
        assert report["status"] == "refused" and report["mismatches"]
    print("release reproducibility comparison self-test: ok")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--first", type=pathlib.Path)
    parser.add_argument("--second", type=pathlib.Path)
    parser.add_argument("--policy", type=pathlib.Path, default=POLICY)
    parser.add_argument("--output", type=pathlib.Path)
    parser.add_argument("--first-context", type=pathlib.Path)
    parser.add_argument("--second-context", type=pathlib.Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if not args.first or not args.second or not args.output:
        fail("--first, --second, and --output are required")
    first = args.first.resolve()
    second = args.second.resolve()
    if not first.is_dir() or not second.is_dir():
        fail("both build roots must exist")
    report = compare(
        first, second, args.policy.resolve(),
        args.first_context.resolve() if args.first_context else None,
        args.second_context.resolve() if args.second_context else None,
    )
    write_create_only(args.output.resolve(), report)
    print(args.output.resolve())
    if report["status"] != "verified":
        raise SystemExit(4)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        fail(str(error))
