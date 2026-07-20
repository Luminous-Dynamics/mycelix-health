#!/usr/bin/env python3
"""Compare verified canonical-runner promotion attempts without interpreting raw logs."""
from __future__ import annotations

import argparse
import importlib.util
import json
import pathlib
from typing import Any, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]


def module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


D = module("health_promotion_diagnostics", ROOT / "scripts/clinical-promotion-diagnostics.py")
IMPORT = module("health_promotion_run_import", ROOT / "scripts/import-clinical-promotion-run.py")


class RunComparisonError(ValueError):
    pass


def fail(message: str) -> NoReturn:
    raise RunComparisonError(message)


def signatures(value: dict[str, Any]) -> set[tuple[str, str]]:
    roots = value["explanation"]["root_causes"]
    return {(str(item["stage"]), str(item["code"])) for item in roots}


def items(values: set[tuple[str, str]]) -> list[dict[str, str]]:
    policy = D.load_policy()
    order = {stage: index for index, stage in enumerate(policy["stage_order"])}
    return [
        {"stage": stage, "reason_code": code}
        for stage, code in sorted(values, key=lambda item: (order.get(item[0], 999), item[1]))
    ]


def compare(previous_path: pathlib.Path, current_path: pathlib.Path) -> dict[str, Any]:
    previous = D.load_json(previous_path)
    current = D.load_json(current_path)
    IMPORT.verify_import_report(previous)
    IMPORT.verify_import_report(current)
    if previous["release_id"] != current["release_id"] or previous["repository"] != current["repository"]:
        fail("runner imports are not from the same release and repository")
    if previous["run_id"] == current["run_id"] and current["run_attempt"] <= previous["run_attempt"]:
        fail("current runner attempt must be later than previous attempt")
    if current["run_id"] < previous["run_id"]:
        fail("current runner run_id must not precede the previous run_id")
    before = signatures(previous)
    after = signatures(current)
    fixed = before - after
    persistent = before & after
    introduced = after - before
    if not before and not after:
        status = "clean"
    elif before and not after:
        status = "cleared"
    elif introduced:
        status = "regressed"
    elif fixed:
        status = "improved"
    else:
        status = "unchanged"
    previous_ref = {
        "name": previous_path.name,
        "sha256": D.sha256_file(previous_path),
        "report_id": previous["report_id"],
        "report_digest_sha256": previous["report_digest_sha256"],
        "source_revision": previous["source_revision"],
        "run_id": previous["run_id"],
        "run_attempt": previous["run_attempt"],
    }
    current_ref = {
        "name": current_path.name,
        "sha256": D.sha256_file(current_path),
        "report_id": current["report_id"],
        "report_digest_sha256": current["report_digest_sha256"],
        "source_revision": current["source_revision"],
        "run_id": current["run_id"],
        "run_attempt": current["run_attempt"],
    }
    identity = {
        "previous": previous_ref,
        "current": current_ref,
        "status": status,
        "fixed": items(fixed),
        "persistent": items(persistent),
        "introduced": items(introduced),
    }
    digest = D.sha256_bytes(D.canonical_bytes(identity))
    return {
        "schema_version": 1,
        "report_kind": "clinical-promotion-run-comparison",
        "report_id": f"health-promotion-run-comparison-{digest[:24]}",
        "status": status,
        "release_id": previous["release_id"],
        "repository": previous["repository"],
        "source_revision_changed": previous["source_revision"] != current["source_revision"],
        "previous": previous_ref,
        "current": current_ref,
        "fixed_root_causes": identity["fixed"],
        "persistent_root_causes": identity["persistent"],
        "introduced_root_causes": identity["introduced"],
        "claims": {
            "only_verified_imports_were_compared": True,
            "raw_logs_were_not_compared": True,
            "message_wording_does_not_define_regression_identity": True,
            "introduced_root_causes_are_never_suppressed": True,
            "comparison_does_not_authorize_promotion": True,
        },
        "report_digest_sha256": digest,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--previous", type=pathlib.Path, required=True)
    parser.add_argument("--current", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    result = compare(args.previous.resolve(), args.current.resolve())
    D.write_create_only(args.output.resolve(), result)
    print(args.output.resolve())
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RunComparisonError, IMPORT.RunnerImportError, D.DiagnosticError, OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        print(f"clinical promotion run comparison error: {error}")
        raise SystemExit(1)
