#!/usr/bin/env python3
"""Compare one empirical countersigning run with the preregistered scenario model."""
from __future__ import annotations

import argparse
import json
import pathlib

COMPATIBLE = {
    "completed": {"completed"},
    "rejected": {"rejected"},
    "recover_or_manual_review": {"recoverable", "manual_review"},
    "wait_then_complete": {"completed", "recoverable"},
    "manual_review": {"manual_review"},
    "manual_review_without_force_policy": {"manual_review"},
}


def fail(message: str) -> "NoReturn":
    raise SystemExit(f"countersigning differential analysis failed: {message}")


def load(path: pathlib.Path, required: bool = True) -> dict | None:
    if not path.is_file():
        if required:
            fail(f"required artifact is missing: {path.name}")
        return None
    try:
        value = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {path}: {error}")
    if not isinstance(value, dict):
        fail(f"{path.name} must contain an object")
    return value


def classify(root: pathlib.Path, observation: dict) -> tuple[str, list[str]]:
    notes: list[str] = []
    if observation.get("observed_execution_status") == "unsupported":
        return "unsupported", ["executor cannot faithfully inject one or more requested faults"]
    outcome = load(root / "execution-outcome.json", required=False)
    if outcome and outcome.get("status") == "verified":
        marker = load(root / "LIVE_COUNTERSIGNING_VERIFIED.json", required=False)
        if not marker or marker.get("status") != "verified":
            return "inconclusive", ["verified outcome is missing independently verified evidence marker"]
        return "completed", notes

    timeline = load(root / "session-state-timeline.json", required=False)
    states: list[dict] = []
    if timeline and isinstance(timeline.get("observations"), list):
        for point in timeline["observations"]:
            if isinstance(point, dict) and isinstance(point.get("states"), list):
                states.extend(item for item in point["states"] if isinstance(item, dict))
    unknown = [item for item in states if item.get("state") == "unknown"]
    if unknown:
        mixed = False
        for item in unknown:
            for outcome_item in item.get("outcomes", []):
                decisions = set(outcome_item.get("decisions", [])) if isinstance(outcome_item, dict) else set()
                if "Complete" in decisions and "Abandoned" in decisions:
                    mixed = True
        if mixed or any(item.get("force_abandon") or item.get("force_publish") for item in unknown):
            return "manual_review", ["native session recovery evidence is conflicting or force-directed"]
        return "recoverable", ["native conductor reported an unresolved session with bounded recovery state"]

    error = str((outcome or {}).get("error") or observation.get("runner_error") or "").lower()
    rejection_terms = ("mismatch", "invalid", "rejected", "refused", "expired", "does not match", "not accepted")
    if any(term in error for term in rejection_terms):
        return "rejected", ["runner failed closed on invalid or substituted ceremony evidence"]
    if states:
        return "inconclusive", ["session states were captured but no terminal native outcome was established"]
    return "inconclusive", ["no verified result or native recovery state was available"]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("evidence_dir", type=pathlib.Path)
    args = parser.parse_args()
    root = args.evidence_dir.resolve()
    observation = load(root / "scenario-observation.json")
    assert observation is not None
    observed, notes = classify(root, observation)
    expected = observation.get("model_expected")
    allowed = COMPATIBLE.get(str(expected), set())
    matches = observed in allowed
    coverage_gap = observed == "unsupported"
    contradictions = [] if matches or coverage_gap else [
        f"model expected {expected!r}, empirical classification was {observed!r}"
    ]
    report = {
        "schema_version": 1,
        "scenario_id": observation.get("scenario_id"),
        "model_expected": expected,
        "empirical_classification": observed,
        "model_compatible": matches,
        "coverage_gap": coverage_gap,
        "contradictions": contradictions,
        "notes": notes,
        "promotion_eligible": matches and observed == "completed" and not contradictions,
        "limits": [
            "classification does not prove DHT finality",
            "entry bodies are intentionally absent from chain summaries",
            "unsupported faults are not approximated",
        ],
    }
    target = root / "differential-report.json"
    if target.exists():
        fail("differential-report.json already exists; refusing to overwrite evidence")
    target.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(target)
    if contradictions:
        raise SystemExit(4)
    if coverage_gap:
        raise SystemExit(3)


if __name__ == "__main__":
    main()
