#!/usr/bin/env python3
"""Validate the native countersigning multi-conductor scenario matrix."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MATRIX = ROOT / "tests" / "countersigning" / "scenarios.json"
REQUIRED_OPERATIONS = {
    "sensitive_record_release": ("patient", "provider"),
    "provider_correction": ("provider", "reviewer"),
    "break_glass_acknowledgement": ("patient", "provider", "reviewer"),
}
REQUIRED_FAULT_PREFIXES = {
    "restart:",
    "delay_gossip:",
    "partition:",
    "authority_decision:",
    "tamper:",
    "policy:",
    "identity:",
    "replay:",
}


def fail(message: str) -> None:
    raise SystemExit(f"countersigning matrix check failed: {message}")


def main() -> None:
    payload = json.loads(MATRIX.read_text())
    if payload.get("schema_version") != 1:
        fail("unsupported schema version")
    conductors = payload.get("required_conductors")
    if conductors != ["patient", "provider", "reviewer", "validator_a", "validator_b"]:
        fail("the five-conductor topology changed without a schema revision")
    scenarios = payload.get("scenarios")
    if not isinstance(scenarios, list) or len(scenarios) < 12:
        fail("the adversarial matrix is too small")

    ids: set[str] = set()
    covered_operations: set[str] = set()
    covered_faults: set[str] = set()
    expected_outcomes: set[str] = set()
    for scenario in scenarios:
        scenario_id = scenario.get("id")
        if not isinstance(scenario_id, str) or not scenario_id or scenario_id in ids:
            fail("scenario IDs must be non-empty and unique")
        ids.add(scenario_id)
        operation = scenario.get("operation")
        if operation not in REQUIRED_OPERATIONS:
            fail(f"{scenario_id}: unknown operation {operation!r}")
        participants = tuple(scenario.get("participants", []))
        if participants != REQUIRED_OPERATIONS[operation]:
            fail(f"{scenario_id}: participant roles do not match {operation}")
        covered_operations.add(operation)
        faults = scenario.get("faults")
        if not isinstance(faults, list) or not all(isinstance(fault, str) for fault in faults):
            fail(f"{scenario_id}: faults must be strings")
        for fault in faults:
            for prefix in REQUIRED_FAULT_PREFIXES:
                if fault.startswith(prefix):
                    covered_faults.add(prefix)
        expected = scenario.get("expected")
        if not isinstance(expected, str) or not expected:
            fail(f"{scenario_id}: expected outcome is missing")
        expected_outcomes.add(expected)

    if covered_operations != set(REQUIRED_OPERATIONS):
        fail("not every high-risk clinical operation is covered")
    missing_faults = REQUIRED_FAULT_PREFIXES - covered_faults
    if missing_faults:
        fail(f"missing fault classes: {sorted(missing_faults)}")
    if not {"completed", "rejected", "manual_review"}.issubset(expected_outcomes):
        fail("success, rejection, and manual-review outcomes must all be represented")

    canonical = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode()
    print(f"countersigning matrix ok: {len(scenarios)} scenarios")
    print(f"sha256={hashlib.sha256(canonical).hexdigest()}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, TypeError, KeyError) as error:
        fail(str(error))
