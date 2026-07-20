#!/usr/bin/env python3
"""Verify release, wire, envelope, source-manifest, and migration identities agree."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import re
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
RUST_WIRE = ROOT / "crates/health-wire/src/lib.rs"
TS_RUNTIME = ROOT / "sdk/src/crypto/health-runtime.ts"
TS_AAD = ROOT / "sdk/src/crypto/encrypted-record-aad.ts"
PY_EVIDENCE = ROOT / "scripts/release-evidence.py"
MANIFEST = ROOT / "release/health-v1.json"
SIGNED_EVIDENCE = ROOT / "release/health-v1.signed-evidence.json"


def fail(message: str) -> "NoReturn":
    raise SystemExit(f"release compatibility check failed: {message}")


def load(path: pathlib.Path) -> dict[str, Any]:
    value = json.loads(path.read_text())
    if not isinstance(value, dict):
        fail(f"{path.relative_to(ROOT)} must contain an object")
    return value


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def match_int(path: pathlib.Path, pattern: str, label: str) -> int:
    match = re.search(pattern, path.read_text())
    if not match:
        fail(f"cannot locate {label} in {path.relative_to(ROOT)}")
    return int(match.group(1))


def match_str(path: pathlib.Path, pattern: str, label: str) -> str:
    match = re.search(pattern, path.read_text())
    if not match:
        fail(f"cannot locate {label} in {path.relative_to(ROOT)}")
    return match.group(1)


def build_report() -> dict[str, Any]:
    manifest = load(MANIFEST)
    signed = load(SIGNED_EVIDENCE)
    evidence = signed.get("evidence")
    if not isinstance(evidence, dict):
        fail("signed release evidence lacks an evidence object")
    rust = {
        "wire_schema_version": match_int(RUST_WIRE, r"HEALTH_WIRE_SCHEMA_VERSION:\s*u16\s*=\s*(\d+)", "Rust wire schema"),
        "migration_epoch": match_int(RUST_WIRE, r"HEALTH_SCHEMA_MIGRATION_EPOCH:\s*u32\s*=\s*(\d+)", "Rust migration epoch"),
        "release_id": match_str(RUST_WIRE, r"HEALTH_RELEASE_ID:\s*&str\s*=\s*\"([^\"]+)\"", "Rust release ID"),
        "envelope_version": match_int(RUST_WIRE, r"ENCRYPTED_RECORD_ENVELOPE_VERSION:\s*u8\s*=\s*(\d+)", "Rust envelope version"),
    }
    typescript = {
        "wire_schema_version": match_int(TS_RUNTIME, r"HEALTH_WIRE_SCHEMA_VERSION\s*=\s*(\d+)\s+as const", "TypeScript wire schema"),
        "migration_epoch": match_int(TS_RUNTIME, r"HEALTH_SCHEMA_MIGRATION_EPOCH\s*=\s*(\d+)\s+as const", "TypeScript migration epoch"),
        "release_id": match_str(TS_RUNTIME, r"HEALTH_RELEASE_ID\s*=\s*'([^']+)'\s+as const", "TypeScript release ID"),
        "envelope_version": match_int(TS_AAD, r"ENCRYPTED_RECORD_ENVELOPE_VERSION\s*=\s*(\d+)\s+as const", "TypeScript envelope version"),
    }
    python = {
        "wire_schema_version": match_int(PY_EVIDENCE, r"WIRE_SCHEMA_VERSION\s*=\s*(\d+)", "Python wire schema"),
        "migration_epoch": match_int(PY_EVIDENCE, r"SCHEMA_MIGRATION_EPOCH\s*=\s*(\d+)", "Python migration epoch"),
    }
    migration_path = ROOT / f"release/migrations/epoch-{rust['migration_epoch']}.json"
    migration = load(migration_path)
    source_manifest_hex = bytes(evidence.get("source_manifest_sha256", [])).hex()
    expected = {
        "release_id": manifest.get("release_id"),
        "wire_schema_version": rust["wire_schema_version"],
        "migration_epoch": rust["migration_epoch"],
        "envelope_version": rust["envelope_version"],
        "source_manifest_sha256": sha256(MANIFEST),
    }
    failures: list[str] = []
    if manifest.get("schema_version") != 1:
        failures.append("release manifest schema version is not 1")
    if rust != typescript:
        failures.append("Rust and TypeScript release constants differ")
    if python["wire_schema_version"] != rust["wire_schema_version"] or python["migration_epoch"] != rust["migration_epoch"]:
        failures.append("release-evidence generator constants differ from the wire crate")
    if evidence.get("release_id") != expected["release_id"] or evidence.get("wire_schema_version") != expected["wire_schema_version"]:
        failures.append("signed evidence release or wire identity differs")
    if evidence.get("schema_migration_epoch") != expected["migration_epoch"]:
        failures.append("signed evidence migration epoch differs")
    if source_manifest_hex != expected["source_manifest_sha256"]:
        failures.append("signed evidence source-manifest digest differs")
    if migration.get("schema_version") != 1 or migration.get("release_id") != expected["release_id"]:
        failures.append("migration record release identity differs")
    if migration.get("migration_epoch") != expected["migration_epoch"] or migration.get("wire_schema_version") != expected["wire_schema_version"]:
        failures.append("migration record epoch or wire schema differs")
    if migration.get("encrypted_record_envelope_version") != expected["envelope_version"]:
        failures.append("migration record envelope version differs")
    compatibility = migration.get("compatibility")
    if not isinstance(compatibility, dict) or compatibility.get("writes_require_current_epoch") is not True:
        failures.append("migration record must require current-epoch writes")
    if compatibility.get("minimum_readable_epoch") > expected["migration_epoch"] or compatibility.get("maximum_readable_epoch") < expected["migration_epoch"]:
        failures.append("migration compatibility range excludes the current epoch")
    migration_plan = migration.get("migration")
    if not isinstance(migration_plan, dict) or not str(migration_plan.get("rollback", "")).strip():
        failures.append("migration record lacks an explicit rollback boundary")
    return {
        "schema_version": 1,
        "release_id": expected["release_id"],
        "status": "verified" if not failures else "refused",
        "expected": expected,
        "rust": rust,
        "typescript": typescript,
        "release_evidence_generator": python,
        "migration_record": str(migration_path.relative_to(ROOT)),
        "migration_record_sha256": sha256(migration_path),
        "release_manifest_sha256": sha256(MANIFEST),
        "signed_evidence_sha256": sha256(SIGNED_EVIDENCE),
        "failures": failures,
    }


def write_create_only(path: pathlib.Path, report: dict[str, Any]) -> None:
    if path.exists():
        fail(f"refusing to overwrite {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2, sort_keys=True)
        handle.write("\n")
    os.chmod(path, 0o600)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=pathlib.Path)
    args = parser.parse_args()
    report = build_report()
    if args.output:
        write_create_only(args.output.resolve(), report)
        print(args.output.resolve())
    else:
        print(json.dumps(report, indent=2, sort_keys=True))
    if report["status"] != "verified":
        for failure in report["failures"]:
            print(f"- {failure}")
        raise SystemExit(4)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        fail(str(error))
