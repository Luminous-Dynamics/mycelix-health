#!/usr/bin/env python3
"""Validate the canonical five-conductor countersigning lab contract."""
from __future__ import annotations

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
TOPOLOGY = ROOT / "tests/countersigning/lab-topology.json"
SCENARIOS = ROOT / "tests/countersigning/scenarios.json"
PACKAGE = ROOT / "sdk/package.json"
EXPECTED = ["patient", "provider", "reviewer", "validator_a", "validator_b"]


def fail(message: str) -> None:
    raise SystemExit(f"canonical countersigning lab check failed: {message}")


def load(path: pathlib.Path) -> dict:
    try:
        value = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {path.relative_to(ROOT)}: {error}")
    if not isinstance(value, dict):
        fail(f"{path.relative_to(ROOT)} must contain a JSON object")
    return value


def main() -> None:
    topology = load(TOPOLOGY)
    scenarios = load(SCENARIOS)
    package = load(PACKAGE)

    if topology.get("schema_version") != 1:
        fail("topology schema_version must be 1")
    if topology.get("lab_id") != "mycelix-health-countersigning-v1":
        fail("unexpected lab_id")
    if topology.get("installed_app_id") != "mycelix-health" or topology.get("role_name") != "health":
        fail("installed app and role must match the authoritative hApp manifest")
    if topology.get("happ_path") != "mycelix-health.happ":
        fail("the canonical lab must consume the packed authoritative hApp")
    if topology.get("holochain_version") != "0.6.1":
        fail("the canonical lab must remain pinned to Holochain 0.6.1")
    if topology.get("holochain_client_version") != package.get("dependencies", {}).get("@holochain/client"):
        fail("topology and SDK @holochain/client versions disagree")

    origin = topology.get("allowed_origin")
    if not isinstance(origin, str) or not re.fullmatch(r"[a-z0-9-]{8,80}", origin):
        fail("allowed_origin must be a bounded explicit origin token")
    if origin == "*":
        fail("wildcard origins are forbidden")

    conductors = topology.get("conductors")
    if not isinstance(conductors, list) or [item.get("name") for item in conductors if isinstance(item, dict)] != EXPECTED:
        fail(f"conductors must be in exact canonical order: {EXPECTED}")

    admin_ports: set[int] = set()
    app_ports: set[int] = set()
    for item in conductors:
        if not isinstance(item, dict):
            fail("each conductor must be an object")
        for field, target in (("admin_port", admin_ports), ("app_port", app_ports)):
            value = item.get(field)
            if not isinstance(value, int) or not (1024 <= value <= 65535):
                fail(f"{item.get('name')} {field} is invalid")
            if value in target:
                fail(f"duplicate {field}: {value}")
            target.add(value)
    if admin_ports & app_ports:
        fail("admin and app port sets must not overlap")

    if scenarios.get("required_conductors") != EXPECTED:
        fail("scenario matrix and topology conductor sets disagree")
    known = set(EXPECTED)
    for scenario in scenarios.get("scenarios", []):
        participants = scenario.get("participants")
        if not isinstance(participants, list) or not participants:
            fail(f"scenario {scenario.get('id')} has no participants")
        if not set(participants) <= known:
            fail(f"scenario {scenario.get('id')} references an unknown conductor")

    serialized = TOPOLOGY.read_text().lower()
    for forbidden in ("token_base64", "app_auth_token", "private_key", "passphrase"):
        if forbidden in serialized:
            fail(f"topology must not contain secret field {forbidden}")

    print("canonical five-conductor lab contract: ok")


if __name__ == "__main__":
    main()
