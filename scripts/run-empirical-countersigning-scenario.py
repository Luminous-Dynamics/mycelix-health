#!/usr/bin/env python3
"""Execute one canonical scenario without translating unsupported faults into simulations."""
from __future__ import annotations

import argparse
import json
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCENARIOS = ROOT / "tests/countersigning/scenarios.json"


def fail(message: str, code: int = 1) -> "NoReturn":
    print(f"empirical scenario failed: {message}", file=sys.stderr)
    raise SystemExit(code)


def load(path: pathlib.Path) -> dict:
    try:
        value = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {path}: {error}")
    if not isinstance(value, dict):
        fail(f"{path} must contain an object")
    return value


def checkpoint_actions(faults: list[str]) -> tuple[list[dict], list[str]]:
    actions: list[dict] = []
    unsupported: list[str] = []
    for fault in faults:
        parts = fault.split(":")
        if len(parts) == 3 and parts[0] == "restart" and parts[2] in {"accepted", "committed"}:
            actions.append({
                "stage": parts[2],
                "participant_name": parts[1],
                "action": "restart",
                "once": True,
            })
        else:
            unsupported.append(fault)
    return actions, unsupported


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--scenario-id", required=True)
    parser.add_argument("--inventory", type=pathlib.Path, required=True)
    parser.add_argument("--fixtures", type=pathlib.Path, required=True)
    parser.add_argument("--output-dir", type=pathlib.Path, required=True)
    parser.add_argument("--lab-state", type=pathlib.Path, required=True)
    parser.add_argument("--mutation-digest", required=True)
    parser.add_argument("--deployment-digest", required=True)
    parser.add_argument("--policy-digest", required=True)
    args = parser.parse_args()

    matrix = load(SCENARIOS)
    scenario = next((item for item in matrix.get("scenarios", []) if item.get("id") == args.scenario_id), None)
    if not isinstance(scenario, dict):
        fail(f"unknown scenario {args.scenario_id!r}")
    fixtures = load(args.fixtures)
    patient = fixtures.get("patient")
    if not isinstance(patient, dict) or not isinstance(patient.get("action_hash"), str):
        fail("fixture inventory has no patient action hash")
    output = args.output_dir.resolve()
    if output.exists():
        fail(f"output directory already exists: {output}")
    output.mkdir(parents=True)
    actions, unsupported = checkpoint_actions(list(scenario.get("faults", [])))
    observation = {
        "schema_version": 1,
        "scenario_id": args.scenario_id,
        "operation": scenario.get("operation"),
        "model_expected": scenario.get("expected"),
        "faults": scenario.get("faults", []),
        "supported_fault_actions": actions,
        "unsupported_faults": unsupported,
        "runner_exit_code": None,
        "observed_execution_status": "unsupported" if unsupported else "pending",
    }
    if unsupported:
        (output / "scenario-observation.json").write_text(json.dumps(observation, indent=2, sort_keys=True) + "\n")
        shutil.copy2(args.fixtures, output / "fixture-inventory.json")
        print(f"scenario {args.scenario_id} is unsupported by the canonical executor: {', '.join(unsupported)}")
        raise SystemExit(3)

    config_path = output / "live-config.json"
    command = [
        sys.executable, str(ROOT / "scripts/render-countersigning-live-config.py"),
        "--inventory", str(args.inventory),
        "--output", str(config_path),
        "--scenario-id", args.scenario_id,
        "--operation", str(scenario["operation"]),
        "--patient-binding", patient["action_hash"],
        "--mutation-digest", args.mutation_digest,
        "--deployment-digest", args.deployment_digest,
        "--policy-digest", args.policy_digest,
        "--lab-state", str(args.lab_state),
    ]
    subprocess.run(command, cwd=ROOT, check=True)
    config = load(config_path)
    config["checkpoint_actions"] = actions
    config_path.write_text(json.dumps(config, indent=2) + "\n")
    shutil.copy2(args.fixtures, output / "fixture-inventory.json")

    result = subprocess.run([
        "node", str(ROOT / "scripts/run-live-countersigning.mjs"),
        str(config_path), str(output),
    ], cwd=ROOT)
    observation["runner_exit_code"] = result.returncode
    outcome_path = output / "execution-outcome.json"
    if outcome_path.is_file():
        outcome = load(outcome_path)
        observation["observed_execution_status"] = outcome.get("status", "inconclusive")
        observation["runner_error"] = outcome.get("error")
    else:
        observation["observed_execution_status"] = "inconclusive"
        observation["runner_error"] = "runner produced no execution outcome"

    chain_output = output / "chain-dht-summary.json"
    capture = subprocess.run([
        "node", str(ROOT / "scripts/capture-countersigning-chain-state.mjs"),
        str(args.inventory), str(chain_output),
    ], cwd=ROOT, capture_output=True, text=True)
    observation["chain_capture_exit_code"] = capture.returncode
    if capture.returncode != 0:
        observation["chain_capture_error"] = (capture.stderr or capture.stdout).strip()[:1000]
    (output / "scenario-observation.json").write_text(json.dumps(observation, indent=2, sort_keys=True) + "\n")
    # The differential analyzer, not the transport process exit code, decides
    # whether an observed refusal matches the preregistered scenario.
    raise SystemExit(0)


if __name__ == "__main__":
    main()
