#!/usr/bin/env python3
"""Produce an ordered, machine-readable explanation of promotion refusals."""
from __future__ import annotations

import argparse
import importlib.util
import json
import pathlib
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("health_promotion_diagnostics", ROOT / "scripts/clinical-promotion-diagnostics.py")
assert SPEC and SPEC.loader
D = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(D)


class ExplanationError(ValueError):
    pass


def normalize_reasons(report: dict[str, Any], policy: dict[str, Any]) -> list[dict[str, Any]]:
    if report.get("report_kind") == "promotion-refusal":
        item = report.get("reason")
        if not isinstance(item, dict):
            raise ExplanationError("promotion refusal report lacks a reason")
        return [item]
    results = report.get("stage_results")
    graph = report.get("reason_graph")
    if not isinstance(results, list) or not isinstance(graph, dict):
        raise ExplanationError("diagnostic report lacks stage results or a reason graph")
    root_stages = set(graph.get("root_cause_stages", []))
    if not root_stages and report.get("status") == "verified":
        return []
    reasons: list[dict[str, Any]] = []
    for result in results:
        if result.get("stage") not in root_stages:
            continue
        for item in result.get("reasons", []):
            if isinstance(item, dict):
                reasons.append(item)
    if not reasons and report.get("status") != "verified":
        reasons.append(D.reason(policy, "INTERNAL_ERROR", "refused report has no root-cause reason", stage="promotion"))
    return reasons


def explain(reports: list[tuple[pathlib.Path, dict[str, Any]]], policy: dict[str, Any]) -> dict[str, Any]:
    combined: list[dict[str, Any]] = []
    report_refs: list[dict[str, Any]] = []
    for path, report in reports:
        reasons = normalize_reasons(report, policy)
        report_refs.append(
            {
                "name": path.name,
                "sha256": D.sha256_file(path),
                "report_id": report.get("report_id"),
                "status": report.get("status"),
                "root_reason_count": len(reasons),
            }
        )
        combined.extend(reasons)
    deduplicated: dict[tuple[str, str, str], dict[str, Any]] = {}
    stage_index = {stage: index for index, stage in enumerate(policy["stage_order"])}
    for item in combined:
        code = str(item.get("code"))
        stage = str(item.get("stage"))
        message = D.sanitize_text(item.get("message", ""), policy)
        key = (stage, code, message)
        deduplicated[key] = {
            "stage": stage,
            "code": code,
            "message": message,
            "owner": item.get("owner") or policy.get("reason_codes", {}).get(code, {}).get("owner"),
            "remediation": item.get("remediation") or policy.get("reason_codes", {}).get(code, {}).get("remediation"),
        }
    ordered = sorted(
        deduplicated.values(),
        key=lambda item: (stage_index.get(str(item["stage"]), 999), str(item["code"]), str(item["message"])),
    )
    by_owner: dict[str, list[int]] = {}
    for index, item in enumerate(ordered, start=1):
        item["priority"] = index
        by_owner.setdefault(str(item.get("owner") or "unknown"), []).append(index)
    identity = {"policy_sha256": D.sha256_file(D.DEFAULT_POLICY), "reports": report_refs, "root_causes": ordered}
    digest = D.sha256_bytes(D.canonical_bytes(identity))
    return {
        "schema_version": 1,
        "report_kind": "promotion-explanation",
        "report_id": f"health-promotion-explanation-{digest[:24]}",
        "status": "verified" if not ordered else "action-required",
        "release_id": policy.get("release_id"),
        "policy_sha256": D.sha256_file(D.DEFAULT_POLICY),
        "source_reports": report_refs,
        "root_causes": ordered,
        "owner_queue": by_owner,
        "claims": {
            "only_root_causes_are_prioritized": True,
            "blocked_downstream_stages_are_not_misreported_as_independent_failures": True,
            "raw_report_bodies_are_not_retained": True,
            "remediation_does_not_weaken_promotion_policy": True,
        },
        "report_digest_sha256": digest,
    }


def self_test() -> None:
    policy = D.load_policy()
    stages = []
    for stage in policy["stage_order"]:
        if stage == "preflight":
            stages.append(
                D.stage_result(
                    policy,
                    stage,
                    "unavailable",
                    reasons=[D.reason(policy, "PREREQUISITE_TOOL_MISSING", "nix unavailable", stage=stage)],
                )
            )
        else:
            stages.append(D.stage_result(policy, stage, "skipped"))
    report = D.build_report(policy, stages, source_revision=None, inputs={}, report_kind="rehearsal")
    with __import__("tempfile").TemporaryDirectory() as raw:
        path = pathlib.Path(raw) / "rehearsal.json"
        D.write_create_only(path, report)
        output = explain([(path, report)], policy)
        assert output["status"] == "action-required"
        assert len(output["root_causes"]) == 1
        assert output["root_causes"][0]["code"] == "PREREQUISITE_TOOL_MISSING"
    print("clinical promotion explanation self-test: ok")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", type=pathlib.Path, action="append", default=[])
    parser.add_argument("--output", type=pathlib.Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return 0
    if not args.report or args.output is None:
        parser.error("at least one --report and --output are required")
    policy = D.load_policy()
    reports = [(path.resolve(), D.load_json(path.resolve())) for path in args.report]
    result = explain(reports, policy)
    D.write_create_only(args.output.resolve(), result)
    print(args.output.resolve())
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ExplanationError, D.DiagnosticError, OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        print(f"clinical promotion explanation error: {error}")
        raise SystemExit(1)
