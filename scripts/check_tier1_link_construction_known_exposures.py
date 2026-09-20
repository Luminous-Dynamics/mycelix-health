#!/usr/bin/env python3
"""Guard the exact known Tier-1 link-construction metadata exposures.

This is a containment/characterization gate, not a remediation or confidentiality
claim. Every legacy exposure listed in the JSON is expected to remain until its
replacement migration is separately qualified. The gate fails if a known pattern
disappears without inventory review OR if another occurrence is introduced.
"""

from __future__ import annotations

import json
import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / "docs" / "security" / "tier1_link_construction_known_exposures.json"
VALID_SEVERITIES = {"moderate", "high", "critical"}


def fail(message: str) -> None:
    print(f"ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_inventory() -> dict:
    try:
        return json.loads(INVENTORY.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"cannot load known-exposure inventory: {exc}")


def main() -> None:
    inventory = load_inventory()
    patterns = inventory.get("patterns")
    if not isinstance(patterns, list) or not patterns:
        fail("inventory.patterns must be a non-empty array")

    declared = inventory.get("subject", {}).get("known_exposure_pattern_count")
    if declared != len(patterns):
        fail(f"declared pattern count {declared!r} != actual inventory rows {len(patterns)}")

    ids: set[str] = set()
    severity_counts: Counter[str] = Counter()
    total_expected_occurrences = 0

    source_cache: dict[Path, str] = {}

    for index, item in enumerate(patterns):
        if not isinstance(item, dict):
            fail(f"pattern {index} is not an object")
        for field in ("id", "file", "regex", "expected_count", "severity", "reason", "replacement"):
            if field not in item:
                fail(f"pattern {index} missing {field}")

        pattern_id = item["id"]
        if not isinstance(pattern_id, str) or not pattern_id:
            fail(f"pattern {index} id must be a non-empty string")
        if pattern_id in ids:
            fail(f"duplicate exposure id: {pattern_id}")
        ids.add(pattern_id)

        severity = item["severity"]
        if severity not in VALID_SEVERITIES:
            fail(f"invalid severity for {pattern_id}: {severity!r}")
        severity_counts[severity] += 1

        expected = item["expected_count"]
        if not isinstance(expected, int) or expected < 1:
            fail(f"expected_count for {pattern_id} must be integer >= 1")
        total_expected_occurrences += expected

        rel_path = Path(item["file"])
        path = ROOT / rel_path
        if not path.is_file():
            fail(f"source file missing for {pattern_id}: {rel_path}")
        text = source_cache.setdefault(path, path.read_text(encoding="utf-8"))

        try:
            compiled = re.compile(item["regex"], re.DOTALL)
        except re.error as exc:
            fail(f"invalid regex for {pattern_id}: {exc}")

        actual = len(compiled.findall(text))
        if actual != expected:
            fail(
                f"known exposure drift for {pattern_id}: expected {expected} occurrence(s), "
                f"found {actual}. Do not silently add/remove legacy exposure; update the reviewed "
                "inventory and replacement plan."
            )

        if not isinstance(item["reason"], str) or not item["reason"].strip():
            fail(f"reason must be non-empty for {pattern_id}")
        if not isinstance(item["replacement"], str) or not item["replacement"].strip():
            fail(f"replacement must be non-empty for {pattern_id}")

    print("Tier-1 link construction known-exposure guard: CHARACTERIZATION PASS")
    print(f"  inventory rows: {len(patterns)}")
    print(f"  expected source occurrences: {total_expected_occurrences}")
    print(f"  source files covered: {len(source_cache)}")
    print("  severities:")
    for severity in sorted(severity_counts):
        print(f"    {severity}: {severity_counts[severity]}")
    print()
    print("NOTE: this PASS means the reviewed legacy exposure set has not drifted.")
    print("It does NOT mean those exposures are safe, fixed, private, or compliant.")


if __name__ == "__main__":
    main()
