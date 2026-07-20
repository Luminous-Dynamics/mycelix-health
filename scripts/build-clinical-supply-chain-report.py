#!/usr/bin/env python3
"""Execute and record the fail-closed Health clinical supply-chain boundary."""
from __future__ import annotations
import argparse, hashlib, json, os, pathlib, subprocess, tempfile
from typing import Any
ROOT = pathlib.Path(__file__).resolve().parents[1]
FILES = {
    "supply_chain_policy": ROOT / "release/supply-chain-policy.json",
    "sbom": ROOT / "release/health-v1.sbom.cdx.json",
    "github_actions_lock": ROOT / "release/github-actions-lock.json",
    "cargo_lock": ROOT / "Cargo.lock",
    "sdk_lock": ROOT / "sdk/package-lock.json",
    "gateway_lock": ROOT / "services/ehr-gateway/package-lock.json",
    "deny_config": ROOT / "deny.toml",
}
class ReportError(ValueError): pass

def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def run(command: list[str]) -> dict[str, Any]:
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)
    if result.returncode:
        raise ReportError(f"command failed ({' '.join(command)}): {result.stderr.strip() or result.stdout.strip()}")
    return {"command": command, "stdout_sha256": hashlib.sha256(result.stdout.encode()).hexdigest()}

def build(source_revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    if len(source_revision) != 40 or any(c not in "0123456789abcdef" for c in source_revision):
        raise ReportError("source revision must be a lowercase 40-character Git SHA")
    policy = json.loads(FILES["supply_chain_policy"].read_text())
    exceptions = policy.get("exceptions", [])
    return {
        "schema_version": 1, "status": "verified", "release_id": "health-v1",
        "source_revision": source_revision,
        "materials": {name: {"path": str(path.relative_to(ROOT)), "sha256": sha256(path)} for name, path in FILES.items()},
        "reviewed_exceptions": [{"ecosystem": x["ecosystem"], "advisory_id": x["advisory_id"], "package": x["package"], "expires_utc": x["expires_utc"], "owner": x["owner"]} for x in exceptions],
        "commands": commands,
        "claims": {
            "npm_online_audit_completed": True,
            "cargo_deny_completed": True,
            "exceptions_are_exact_and_unexpired": True,
            "sbom_matches_lockfiles": True,
            "github_actions_are_immutably_pinned": True
        }
    }

def write(path: pathlib.Path, value: dict[str, Any]) -> None:
    if path.exists(): raise ReportError(f"refusing to overwrite {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x") as h: json.dump(value, h, indent=2, sort_keys=True); h.write("\n")
    os.chmod(path, 0o600)

def self_test() -> None:
    value = build("1"*40, [{"command":["self-test"],"stdout_sha256":"0"*64}])
    assert value["status"] == "verified" and value["materials"]["sbom"]["sha256"] == sha256(FILES["sbom"])
    with tempfile.TemporaryDirectory() as raw:
        target = pathlib.Path(raw)/"report.json"; write(target, value); assert target.stat().st_mode & 0o777 == 0o600
    print("clinical supply-chain report self-test: ok")

def main() -> int:
    p=argparse.ArgumentParser(); p.add_argument("--source-revision"); p.add_argument("--output",type=pathlib.Path); p.add_argument("--self-test",action="store_true"); a=p.parse_args()
    if a.self_test: self_test(); return 0
    if not a.source_revision or not a.output: raise ReportError("--source-revision and --output are required")
    commands=[]
    for command in [
        ["python3","scripts/check-supply-chain-policy.py"],
        ["python3","scripts/check-node-production-deps.py","--self-test","--audit"],
        ["python3","scripts/generate-supply-chain-sbom.py","--check"],
        ["python3","scripts/check-supply-chain-sbom.py"],
        ["python3","scripts/check-github-actions-lock.py"],
        ["cargo","deny","check"]
    ]: commands.append(run(command))
    write(a.output.resolve(), build(a.source_revision, commands)); print(a.output.resolve()); return 0
if __name__ == "__main__":
    try: raise SystemExit(main())
    except (ReportError,OSError,KeyError,ValueError,json.JSONDecodeError) as e: print(f"clinical supply-chain report error: {e}"); raise SystemExit(1)
