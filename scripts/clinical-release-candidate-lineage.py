#!/usr/bin/env python3
"""Authorize, observe, and verify append-only clinical release-candidate lineage."""
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


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser()
    subs = value.add_subparsers(dest="command", required=True)
    authorize = subs.add_parser("authorize")
    authorize.add_argument("--prior-lineage", type=pathlib.Path)
    authorize.add_argument("--remediation-plan", type=pathlib.Path, required=True)
    authorize.add_argument("--change-manifest", type=pathlib.Path, required=True)
    authorize.add_argument("--approval-bundle", type=pathlib.Path, required=True)
    authorize.add_argument("--trust-store", type=pathlib.Path, default=G.TRUST_STORE)
    authorize.add_argument("--policy", type=pathlib.Path, default=G.POLICY)
    authorize.add_argument("--verification-time-utc", required=True)
    authorize.add_argument("--output", type=pathlib.Path, required=True)
    observe = subs.add_parser("observe")
    observe.add_argument("--lineage", type=pathlib.Path, required=True)
    observe.add_argument("--runner-import", type=pathlib.Path, required=True)
    observe.add_argument("--policy", type=pathlib.Path, default=G.POLICY)
    observe.add_argument("--output", type=pathlib.Path, required=True)
    verify = subs.add_parser("verify")
    verify.add_argument("--lineage", type=pathlib.Path, required=True)
    verify.add_argument("--policy", type=pathlib.Path, default=G.POLICY)
    return value


def main() -> int:
    args = parser().parse_args()
    policy = G.load_policy(args.policy.resolve())
    if args.command == "authorize":
        trust = G.load_trust_store(args.trust_store.resolve(), policy)
        prior = G.load_json(args.prior_lineage.resolve()) if args.prior_lineage else None
        result = G.authorize_candidate(
            prior,
            G.load_json(args.remediation_plan.resolve()),
            G.load_json(args.change_manifest.resolve()),
            G.load_json(args.approval_bundle.resolve()),
            policy,
            trust,
            args.verification_time_utc,
        )
        G.write_create_only(args.output.resolve(), result)
        print(args.output.resolve())
    elif args.command == "observe":
        result = G.observe_candidate(
            G.load_json(args.lineage.resolve()), G.load_json(args.runner_import.resolve()), policy
        )
        G.write_create_only(args.output.resolve(), result)
        print(args.output.resolve())
    else:
        lineage = G.load_json(args.lineage.resolve())
        G.verify_lineage(lineage, policy)
        print(lineage["lineage_id"])
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (G.GovernanceError, G.IMPORT.RunnerImportError, OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        print(f"clinical release-candidate lineage failed: {error}")
        raise SystemExit(1)
