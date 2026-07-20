#!/usr/bin/env python3
"""Prepare, sign, assemble, and verify signed remediation approvals."""
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
    prepare = subs.add_parser("prepare")
    prepare.add_argument("--remediation-plan", type=pathlib.Path, required=True)
    prepare.add_argument("--change-manifest", type=pathlib.Path, required=True)
    prepare.add_argument("--issued-at-utc", required=True)
    prepare.add_argument("--expires-at-utc", required=True)
    prepare.add_argument("--nonce", required=True)
    prepare.add_argument("--policy", type=pathlib.Path, default=G.POLICY)
    prepare.add_argument("--output", type=pathlib.Path, required=True)
    sign = subs.add_parser("sign")
    sign.add_argument("--statement", type=pathlib.Path, required=True)
    sign.add_argument("--private-key", type=pathlib.Path, required=True)
    sign.add_argument("--approver-id", required=True)
    sign.add_argument("--output", type=pathlib.Path, required=True)
    assemble = subs.add_parser("assemble")
    assemble.add_argument("--statement", type=pathlib.Path, required=True)
    assemble.add_argument("--signature", type=pathlib.Path, action="append", required=True)
    assemble.add_argument("--remediation-plan", type=pathlib.Path, required=True)
    assemble.add_argument("--change-manifest", type=pathlib.Path, required=True)
    assemble.add_argument("--trust-store", type=pathlib.Path, default=G.TRUST_STORE)
    assemble.add_argument("--policy", type=pathlib.Path, default=G.POLICY)
    assemble.add_argument("--verification-time-utc", required=True)
    assemble.add_argument("--output", type=pathlib.Path, required=True)
    verify = subs.add_parser("verify")
    verify.add_argument("--bundle", type=pathlib.Path, required=True)
    verify.add_argument("--remediation-plan", type=pathlib.Path, required=True)
    verify.add_argument("--change-manifest", type=pathlib.Path, required=True)
    verify.add_argument("--trust-store", type=pathlib.Path, default=G.TRUST_STORE)
    verify.add_argument("--policy", type=pathlib.Path, default=G.POLICY)
    verify.add_argument("--verification-time-utc", required=True)
    return value


def main() -> int:
    args = parser().parse_args()
    if args.command == "prepare":
        policy = G.load_policy(args.policy.resolve())
        plan = G.load_json(args.remediation_plan.resolve())
        manifest = G.load_json(args.change_manifest.resolve())
        value = G.build_approval_statement(plan, manifest, args.issued_at_utc, args.expires_at_utc, args.nonce, policy)
        G.verify_approval_statement(value, plan, manifest, policy)
        G.write_create_only(args.output.resolve(), value)
        print(args.output.resolve())
    elif args.command == "sign":
        statement = G.load_json(args.statement.resolve())
        value = G.sign_approval(statement, args.private_key.resolve(), args.approver_id)
        G.write_create_only(args.output.resolve(), value)
        print(args.output.resolve())
    elif args.command == "assemble":
        policy = G.load_policy(args.policy.resolve())
        trust = G.load_trust_store(args.trust_store.resolve(), policy)
        statement = G.load_json(args.statement.resolve())
        plan = G.load_json(args.remediation_plan.resolve())
        manifest = G.load_json(args.change_manifest.resolve())
        signatures = [G.load_json(path.resolve()) for path in args.signature]
        value = G.build_approval_bundle(statement, signatures, plan, manifest, policy, trust, args.verification_time_utc)
        G.verify_approval_bundle(value, plan, manifest, policy, trust, args.verification_time_utc)
        G.write_create_only(args.output.resolve(), value)
        print(args.output.resolve())
    else:
        policy = G.load_policy(args.policy.resolve())
        trust = G.load_trust_store(args.trust_store.resolve(), policy)
        bundle = G.load_json(args.bundle.resolve())
        plan = G.load_json(args.remediation_plan.resolve())
        manifest = G.load_json(args.change_manifest.resolve())
        G.verify_approval_bundle(bundle, plan, manifest, policy, trust, args.verification_time_utc)
        print(bundle["bundle_id"])
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (G.GovernanceError, OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        print(f"clinical remediation approval failed: {error}")
        raise SystemExit(1)
