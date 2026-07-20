#!/usr/bin/env python3
"""Validate the empirical countersigning-lab artifact contract."""
from __future__ import annotations

import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests/countersigning/lab-fixtures.json"
CONTRACT = ROOT / "tests/countersigning/empirical-run-contract.json"


def fail(message: str) -> "NoReturn":
    raise SystemExit(f"empirical countersigning lab check failed: {message}")


def load(path: pathlib.Path) -> dict:
    try:
        value = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {path.relative_to(ROOT)}: {error}")
    if not isinstance(value, dict):
        fail(f"{path.relative_to(ROOT)} must contain an object")
    return value


def main() -> None:
    fixtures = load(FIXTURES)
    contract = load(CONTRACT)
    if fixtures.get("schema_version") != 1 or fixtures.get("classification") != "synthetic_non_phi":
        fail("fixtures must be schema-v1 synthetic non-PHI")
    patient = fixtures.get("patient")
    if not isinstance(patient, dict) or patient.get("conductor") != "patient":
        fail("fixture set must contain the canonical patient actor")
    if not re.fullmatch(r"LAB-[A-Z0-9-]{4,48}", str(patient.get("patient_id", ""))):
        fail("patient fixture ID must be visibly synthetic and bounded")
    providers = fixtures.get("providers")
    if not isinstance(providers, list) or [item.get("conductor") for item in providers if isinstance(item, dict)] != ["provider", "reviewer"]:
        fail("provider fixtures must cover provider then reviewer")
    serialized = json.dumps(fixtures).lower()
    for forbidden in ("real_patient", "token_base64", "app_auth_token", "cap_secret", "private_key", "passphrase"):
        if forbidden in serialized:
            fail(f"fixture contract contains forbidden field {forbidden}")
    required = contract.get("required_artifacts")
    if not isinstance(required, list) or len(required) != len(set(required)) or len(required) < 6:
        fail("empirical contract requires a unique bounded artifact set")
    if "differential-report.json" not in required or "session-state-timeline.json" not in required:
        fail("empirical contract omits model comparison or native session state")
    rules = contract.get("promotion_rules")
    if not isinstance(rules, dict) or not all(rules.get(key) is True for key in (
        "require_verified_cryptographic_evidence_for_completed",
        "require_no_model_contradictions",
        "forbid_raw_phi",
        "forbid_authentication_secrets",
    )):
        fail("empirical promotion rules must fail closed")
    print("empirical countersigning lab contract: ok")


if __name__ == "__main__":
    main()
