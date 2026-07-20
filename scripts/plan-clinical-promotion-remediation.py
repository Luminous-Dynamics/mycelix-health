#!/usr/bin/env python3
"""Create a bounded, non-executable remediation plan from a verified runner import."""
from __future__ import annotations

import argparse
import importlib.util
import json
import pathlib
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]
POLICY = ROOT / "release/clinical-promotion-remediation-policy.json"


def module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


D = module("health_promotion_diagnostics", ROOT / "scripts/clinical-promotion-diagnostics.py")
IMPORT = module("health_promotion_run_import", ROOT / "scripts/import-clinical-promotion-run.py")


class RemediationPlanError(ValueError):
    pass


def fail(message: str) -> NoReturn:
    raise RemediationPlanError(message)


def load_policy(path: pathlib.Path = POLICY) -> dict[str, Any]:
    value = D.load_json(path)
    if value.get("schema_version") != 1 or value.get("release_id") != "health-v1":
        fail("remediation policy identity drifted")
    actions = value.get("allowed_action_types")
    mapping = value.get("reason_action_map")
    statuses = value.get("plan_statuses")
    claims = value.get("claims")
    if not isinstance(actions, dict) or not actions:
        fail("remediation policy has no allowed action types")
    if not isinstance(mapping, dict) or set(mapping.values()) - set(actions):
        fail("remediation policy reason map references unknown actions")
    diagnostics = D.load_policy()
    if set(mapping) != set(diagnostics["reason_codes"]):
        fail("remediation policy must cover every diagnostic reason exactly once")
    if not isinstance(statuses, list) or len(statuses) != len(set(statuses)):
        fail("remediation policy statuses are invalid")
    if not isinstance(claims, dict) or any(item is not True for item in claims.values()):
        fail("remediation policy claims must all be true")
    for name, action in actions.items():
        if not isinstance(action, dict) or not isinstance(action.get("instruction"), str) or not action["instruction"]:
            fail(f"remediation action lacks a bounded instruction: {name}")
        if "command" in action or "script" in action:
            fail(f"remediation action must not contain executable content: {name}")
    value["_policy_path"] = str(path.resolve())
    return value


def status_for(actions: list[dict[str, Any]]) -> str:
    if not actions:
        return "no-action-required"
    action_types = {item["action_type"] for item in actions}
    if "repair_deterministic_tooling" in action_types:
        return "engineering-action-required"
    if action_types <= {"restore_prerequisite", "rerun_attestation_verification"}:
        return "infrastructure-action-required"
    return "operator-action-required"


def build_plan(import_path: pathlib.Path, imported: dict[str, Any], policy: dict[str, Any]) -> dict[str, Any]:
    IMPORT.verify_import_report(imported)
    roots = imported["explanation"]["root_causes"]
    actions: list[dict[str, Any]] = []
    seen: set[tuple[str, str, str]] = set()
    for root in roots:
        code = root["code"]
        action_type = policy["reason_action_map"].get(code)
        action_policy = policy["allowed_action_types"].get(action_type)
        if not isinstance(action_policy, dict):
            fail(f"no bounded action exists for diagnostic reason: {code}")
        key = (root["stage"], code, action_type)
        if key in seen:
            continue
        seen.add(key)
        identity = {
            "stage": root["stage"],
            "reason_code": code,
            "action_type": action_type,
            "source_import_digest_sha256": imported["report_digest_sha256"],
        }
        action_digest = D.sha256_bytes(D.canonical_bytes(identity))
        actions.append(
            {
                "action_id": f"health-remediation-{action_digest[:20]}",
                "priority": len(actions) + 1,
                "stage": root["stage"],
                "reason_code": code,
                "action_type": action_type,
                "owner": action_policy["owner"],
                "mutation_class": action_policy["mutation_class"],
                "instruction": action_policy["instruction"],
                "requires_human_review": True,
                "executes_commands": False,
                "may_modify_promotion_evidence": False,
                "may_modify_protected_policy": False,
            }
        )
    plan_status = status_for(actions)
    if plan_status not in policy["plan_statuses"]:
        fail("derived remediation status is not allowed")
    source = {
        "name": import_path.name,
        "sha256": D.sha256_file(import_path),
        "report_id": imported["report_id"],
        "report_digest_sha256": imported["report_digest_sha256"],
        "repository": imported["repository"],
        "source_revision": imported["source_revision"],
        "run_id": imported["run_id"],
        "run_attempt": imported["run_attempt"],
    }
    identity = {
        "remediation_policy_sha256": D.sha256_file(pathlib.Path(policy["_policy_path"])),
        "source_import": source,
        "status": plan_status,
        "actions": actions,
        "protected_paths": policy["protected_paths"],
        "claims": policy["claims"],
    }
    digest = D.sha256_bytes(D.canonical_bytes(identity))
    result = {
        "schema_version": 1,
        "report_kind": "clinical-promotion-remediation-plan",
        "plan_id": f"health-promotion-remediation-{digest[:24]}",
        "status": plan_status,
        "release_id": policy["release_id"],
        "repository": imported["repository"],
        "source_revision": imported["source_revision"],
        "run_id": imported["run_id"],
        "run_attempt": imported["run_attempt"],
        "remediation_policy_sha256": identity["remediation_policy_sha256"],
        "source_import": source,
        "actions": actions,
        "protected_paths": policy["protected_paths"],
        "claims": policy["claims"],
        "plan_digest_sha256": digest,
    }
    forbidden = [str(item).lower() for item in policy.get("forbidden_plan_content", [])]
    action_text = D.canonical_bytes(actions).decode("utf-8").lower()
    if any(fragment in action_text for fragment in forbidden):
        fail("generated remediation action contains forbidden policy-bypass language")
    return result


def verify_plan(value: dict[str, Any], policy: dict[str, Any]) -> dict[str, Any]:
    if value.get("schema_version") != 1 or value.get("report_kind") != "clinical-promotion-remediation-plan":
        fail("remediation plan schema or kind is invalid")
    if value.get("release_id") != policy["release_id"] or value.get("status") not in policy["plan_statuses"]:
        fail("remediation plan identity or status drifted")
    source = value.get("source_import")
    actions = value.get("actions")
    if not isinstance(source, dict) or not isinstance(actions, list):
        fail("remediation plan source or actions are invalid")
    for index, action in enumerate(actions, start=1):
        if not isinstance(action, dict) or action.get("priority") != index:
            fail("remediation plan priorities are invalid")
        action_type = action.get("action_type")
        expected = policy["allowed_action_types"].get(action_type)
        if not isinstance(expected, dict):
            fail("remediation plan contains an unknown action")
        if action.get("owner") != expected["owner"] or action.get("mutation_class") != expected["mutation_class"]:
            fail("remediation action ownership or mutation class drifted")
        if action.get("instruction") != expected["instruction"]:
            fail("remediation action instruction drifted")
        for claim in ("requires_human_review",):
            if action.get(claim) is not True:
                fail(f"remediation action must require {claim}")
        for claim in ("executes_commands", "may_modify_promotion_evidence", "may_modify_protected_policy"):
            if action.get(claim) is not False:
                fail(f"remediation action must keep {claim} false")
    identity = {
        "remediation_policy_sha256": value.get("remediation_policy_sha256"),
        "source_import": source,
        "status": value.get("status"),
        "actions": actions,
        "protected_paths": value.get("protected_paths"),
        "claims": value.get("claims"),
    }
    digest = D.sha256_bytes(D.canonical_bytes(identity))
    if value.get("plan_digest_sha256") != digest:
        fail("remediation plan digest mismatch")
    if value.get("plan_id") != f"health-promotion-remediation-{digest[:24]}":
        fail("remediation plan id mismatch")
    if value.get("protected_paths") != policy["protected_paths"] or value.get("claims") != policy["claims"]:
        fail("remediation plan safety boundary drifted")
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--policy", type=pathlib.Path, default=POLICY)
    parser.add_argument("--runner-import", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    policy = load_policy(args.policy.resolve())
    imported_path = args.runner_import.resolve()
    imported = D.load_json(imported_path)
    plan = build_plan(imported_path, imported, policy)
    verify_plan(plan, policy)
    D.write_create_only(args.output.resolve(), plan)
    print(args.output.resolve())
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RemediationPlanError, IMPORT.RunnerImportError, D.DiagnosticError, OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        print(f"clinical promotion remediation planning error: {error}")
        raise SystemExit(1)
