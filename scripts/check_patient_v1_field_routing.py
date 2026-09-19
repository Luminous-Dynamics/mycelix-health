#!/usr/bin/env python3
"""Fail if the packaged v1 Patient schema drifts from the migration routing contract."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE = ROOT / "zomes/patient/integrity/src/lib.rs"
ROUTES = ROOT / "crates/patient-profile-v2/legacy_field_routes.json"

ALLOWED_ROUTES = {
    "protected_identifier",
    "protected_demographic",
    "protected_sourced_clinical_datum",
    "protected_communication_profile",
    "allergy_domain_migration",
    "condition_domain_migration",
    "medication_domain_migration",
    "protected_identity_binding_migration",
    "legacy_reliability_observation",
    "protected_provenance",
}

FIELD_RE = re.compile(r"^\s*pub\s+([A-Za-z_][A-Za-z0-9_]*)\s*:")
STRUCT_RE = re.compile(r"\bpub\s+struct\s+Patient\s*\{")


def patient_struct_body(source: str) -> str:
    match = STRUCT_RE.search(source)
    if not match:
        raise ValueError("could not locate `pub struct Patient {` in source")

    brace_start = source.find("{", match.start())
    depth = 0
    for index in range(brace_start, len(source)):
        char = source[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return source[brace_start + 1 : index]
    raise ValueError("unterminated Patient struct")


def extract_fields(source: str) -> list[str]:
    body = patient_struct_body(source)
    fields: list[str] = []
    for line in body.splitlines():
        match = FIELD_RE.match(line)
        if match:
            fields.append(match.group(1))
    if not fields:
        raise ValueError("Patient struct contained no public fields")
    return fields


def load_routes() -> tuple[int, list[tuple[str, str]]]:
    data = json.loads(ROUTES.read_text(encoding="utf-8"))
    if data.get("schema") != "mycelix-health/patient-v1-field-routes/v1":
        raise ValueError("unexpected routing schema identifier")
    declared = data.get("declared_field_count")
    rows = data.get("fields")
    if not isinstance(declared, int) or not isinstance(rows, list):
        raise ValueError("routing contract missing declared_field_count/fields")

    parsed: list[tuple[str, str]] = []
    seen: set[str] = set()
    for row in rows:
        if not isinstance(row, dict):
            raise ValueError("field route row must be an object")
        field = row.get("field")
        route = row.get("route")
        if not isinstance(field, str) or not isinstance(route, str):
            raise ValueError("field/route values must be strings")
        if field in seen:
            raise ValueError(f"duplicate routing field: {field}")
        if route not in ALLOWED_ROUTES:
            raise ValueError(f"unsupported route for {field}: {route}")
        seen.add(field)
        parsed.append((field, route))

    if declared != len(parsed):
        raise ValueError(
            f"declared_field_count={declared} but routing contract has {len(parsed)} rows"
        )
    return declared, parsed


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    args = parser.parse_args()

    try:
        declared, routes = load_routes()
        source_fields = extract_fields(args.source.read_text(encoding="utf-8"))
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"PATIENT-V1-ROUTING FAIL: {exc}", file=sys.stderr)
        return 1

    route_fields = [field for field, _ in routes]
    if len(source_fields) != declared:
        print(
            f"PATIENT-V1-ROUTING FAIL: source has {len(source_fields)} fields; "
            f"contract declares {declared}",
            file=sys.stderr,
        )
        print(f"source fields: {source_fields}", file=sys.stderr)
        return 1

    if source_fields != route_fields:
        source_set = set(source_fields)
        route_set = set(route_fields)
        missing = [field for field in source_fields if field not in route_set]
        stale = [field for field in route_fields if field not in source_set]
        print("PATIENT-V1-ROUTING FAIL: exact field/order contract drifted", file=sys.stderr)
        if missing:
            print(f"unrouted source fields: {missing}", file=sys.stderr)
        if stale:
            print(f"stale routing fields: {stale}", file=sys.stderr)
        if not missing and not stale:
            print(f"source order: {source_fields}", file=sys.stderr)
            print(f"route order:  {route_fields}", file=sys.stderr)
        return 1

    route_counts: dict[str, int] = {}
    for _, route in routes:
        route_counts[route] = route_counts.get(route, 0) + 1

    print(f"PATIENT-V1-ROUTING PASS: {declared} fields classified")
    for route in sorted(route_counts):
        print(f"  {route}: {route_counts[route]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
