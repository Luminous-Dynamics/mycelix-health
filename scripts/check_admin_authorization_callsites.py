#!/usr/bin/env python3
"""Fail when require_admin_authorization() call sites drift without review."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONTRACT = ROOT / "crates/admin-authorization-core/admin_callsites.json"
COORDINATOR_GLOB = "zomes/*/coordinator/src/lib.rs"
CALL = "require_admin_authorization()?;"
FUNCTION_RE = re.compile(r"\bpub\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)\b")


def load_contract() -> dict:
    data = json.loads(CONTRACT.read_text(encoding="utf-8"))
    if data.get("schema") != "mycelix-health/admin-authorization-callsites/v1":
        raise ValueError("unexpected contract schema")
    rows = data.get("callsites")
    declared = data.get("declared_callsite_count")
    if not isinstance(rows, list) or not isinstance(declared, int):
        raise ValueError("contract requires callsites[] and declared_callsite_count")
    if declared != len(rows):
        raise ValueError(
            f"declared_callsite_count={declared} but contract has {len(rows)} rows"
        )

    seen: set[tuple[str, str]] = set()
    for row in rows:
        if not isinstance(row, dict):
            raise ValueError("callsite row must be an object")
        for key in ("zome", "function", "path", "scope", "risk"):
            if not isinstance(row.get(key), str) or not row[key]:
                raise ValueError(f"callsite row missing non-empty {key}")
        key = (row["path"], row["function"])
        if key in seen:
            raise ValueError(f"duplicate callsite contract row: {key}")
        seen.add(key)
        expected_zome = Path(row["path"]).parts[1]
        if row["zome"] != expected_zome:
            raise ValueError(
                f"zome/path mismatch for {row['function']}: {row['zome']} != {expected_zome}"
            )
        if row["scope"] not in {"tier1_packaged", "tier2_unpackaged"}:
            raise ValueError(f"invalid scope for {row['function']}: {row['scope']}")
    return data


def scan_file(path: Path) -> list[tuple[str, str, int]]:
    relative = path.relative_to(ROOT).as_posix()
    current_function: str | None = None
    found: list[tuple[str, str, int]] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        match = FUNCTION_RE.search(line)
        if match:
            current_function = match.group(1)
        if CALL in line:
            if current_function is None:
                raise ValueError(
                    f"{relative}:{line_number}: admin helper call outside tracked public function"
                )
            found.append((relative, current_function, line_number))
    return found


def main() -> int:
    try:
        data = load_contract()
        actual: list[tuple[str, str, int]] = []
        for path in sorted(ROOT.glob(COORDINATOR_GLOB)):
            actual.extend(scan_file(path))
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"ADMIN-CALLSITES FAIL: {exc}", file=sys.stderr)
        return 1

    expected_rows = data["callsites"]
    expected = {(row["path"], row["function"]) for row in expected_rows}
    actual_pairs = {(path, function) for path, function, _ in actual}

    duplicate_actual: list[tuple[str, str]] = []
    counts: dict[tuple[str, str], int] = {}
    for path, function, _ in actual:
        key = (path, function)
        counts[key] = counts.get(key, 0) + 1
    duplicate_actual = [key for key, count in counts.items() if count != 1]

    missing = sorted(expected - actual_pairs)
    unreviewed = sorted(actual_pairs - expected)
    if missing or unreviewed or duplicate_actual:
        print("ADMIN-CALLSITES FAIL: privileged helper callsite inventory drifted", file=sys.stderr)
        if missing:
            print(f"contract rows no longer present: {missing}", file=sys.stderr)
        if unreviewed:
            print(f"unreviewed new callsites: {unreviewed}", file=sys.stderr)
        if duplicate_actual:
            print(f"callsites with unexpected multiplicity: {duplicate_actual}", file=sys.stderr)
        for path, function, line in actual:
            print(f"  actual {path}:{line} -> {function}", file=sys.stderr)
        return 1

    tier1 = sum(row["scope"] == "tier1_packaged" for row in expected_rows)
    tier2 = sum(row["scope"] == "tier2_unpackaged" for row in expected_rows)
    print(
        f"ADMIN-CALLSITES PASS: {len(expected_rows)} reviewed callsites "
        f"({tier1} tier1_packaged, {tier2} tier2_unpackaged)"
    )
    for row in expected_rows:
        print(f"  {row['scope']}: {row['zome']}::{row['function']} [{row['risk']}]")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
