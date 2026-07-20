#!/usr/bin/env python3
"""Build a reviewed, non-executable change manifest for one remediation plan."""
from __future__ import annotations

import argparse
import importlib.util
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("health_remediation_governance", ROOT / "scripts/clinical-remediation-governance.py")
assert spec and spec.loader
G = importlib.util.module_from_spec(spec)
spec.loader.exec_module(G)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository", type=pathlib.Path, default=ROOT)
    parser.add_argument("--base-revision", required=True)
    parser.add_argument("--target-revision", required=True)
    parser.add_argument("--remediation-plan", type=pathlib.Path, required=True)
    parser.add_argument("--policy", type=pathlib.Path, default=G.POLICY)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    policy = G.load_policy(args.policy.resolve())
    value = G.build_change_manifest(
        args.repository.resolve(),
        args.base_revision,
        args.target_revision,
        args.remediation_plan.resolve(),
        policy,
    )
    plan = G.load_json(args.remediation_plan.resolve())
    G.verify_change_manifest(value, plan, policy)
    G.write_create_only(args.output.resolve(), value)
    print(args.output.resolve())
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (G.GovernanceError, OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        print(f"clinical remediation change manifest failed: {error}")
        raise SystemExit(1)
