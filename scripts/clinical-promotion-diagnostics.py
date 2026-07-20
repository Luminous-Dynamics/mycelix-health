#!/usr/bin/env python3
"""Shared deterministic reason-graph helpers for clinical release promotion.

The module intentionally stores only bounded, sanitized summaries. It never records
raw command output, environment values, authentication material, or report bodies.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import re
import tempfile
from typing import Any, Iterable, NoReturn

ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_POLICY = ROOT / "release/clinical-promotion-diagnostics-policy.json"
HEX40 = re.compile(r"^[0-9a-f]{40}$")
SENSITIVE_ASSIGNMENT = re.compile(
    r"(?i)(token|secret|password|passphrase|private[_-]?key|authorization|cookie)\s*[:=]\s*([^\s,;]+)"
)
BEARER = re.compile(r"(?i)\bbearer\s+[A-Za-z0-9._~+/=-]+")
PRIVATE_KEY = re.compile(r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----.*?-----END [A-Z0-9 ]*PRIVATE KEY-----", re.S)


class DiagnosticError(ValueError):
    """Raised when a diagnostic artifact is malformed or unsafe."""


def fail(message: str) -> NoReturn:
    raise DiagnosticError(message)


def load_json(path: pathlib.Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise DiagnosticError(f"{path.name} is not valid JSON") from error
    if not isinstance(value, dict):
        fail(f"{path.name} must contain a JSON object")
    return value


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    return sha256_bytes(path.read_bytes())


def load_policy(path: pathlib.Path = DEFAULT_POLICY) -> dict[str, Any]:
    policy = load_json(path)
    if policy.get("schema_version") != 1:
        fail("diagnostics policy schema_version must be 1")
    stages = policy.get("stage_order")
    dependencies = policy.get("stage_dependencies")
    reason_codes = policy.get("reason_codes")
    if not isinstance(stages, list) or not stages or len(stages) != len(set(stages)):
        fail("diagnostics policy stage_order must be a unique non-empty list")
    if not isinstance(dependencies, dict) or set(dependencies) != set(stages):
        fail("diagnostics policy dependencies must cover every stage exactly once")
    for stage, deps in dependencies.items():
        if not isinstance(deps, list) or any(dep not in stages for dep in deps):
            fail(f"diagnostics policy contains invalid dependencies for {stage}")
        if stage in deps:
            fail(f"diagnostics stage {stage} depends on itself")
    if not isinstance(reason_codes, dict) or not reason_codes:
        fail("diagnostics policy reason_codes must be a non-empty object")
    policy["_policy_path"] = str(path.resolve())
    return policy


def sanitize_text(value: Any, policy: dict[str, Any], *, command_output: bool = False) -> str:
    text = str(value)
    replacement = str(policy.get("redaction", {}).get("replacement", "[REDACTED]"))
    text = PRIVATE_KEY.sub(replacement, text)
    text = BEARER.sub(f"Bearer {replacement}", text)
    text = SENSITIVE_ASSIGNMENT.sub(lambda match: f"{match.group(1)}={replacement}", text)
    for name, raw in os.environ.items():
        if not raw or len(raw) < 6:
            continue
        fragments = policy.get("redaction", {}).get("environment_name_fragments", [])
        if any(fragment.upper() in name.upper() for fragment in fragments):
            text = text.replace(raw, replacement)
    limits = policy.get("output_limits", {})
    limit = int(
        limits.get(
            "maximum_command_output_characters" if command_output else "maximum_message_characters",
            8192 if command_output else 2048,
        )
    )
    if limit < 64:
        fail("diagnostics output limit is unreasonably small")
    if len(text) > limit:
        text = text[: limit - 16] + "...[TRUNCATED]"
    return text


def reason(
    policy: dict[str, Any],
    code: str,
    message: str,
    *,
    stage: str,
    details: dict[str, Any] | None = None,
) -> dict[str, Any]:
    reason_policy = policy.get("reason_codes", {}).get(code)
    if not isinstance(reason_policy, dict):
        fail(f"unknown diagnostic reason code: {code}")
    item: dict[str, Any] = {
        "code": code,
        "stage": stage,
        "message": sanitize_text(message, policy),
        "owner": reason_policy.get("owner"),
        "remediation": reason_policy.get("remediation"),
    }
    if details:
        sanitized: dict[str, Any] = {}
        for key, value in sorted(details.items()):
            if isinstance(value, (bool, int, float)) or value is None:
                sanitized[str(key)] = value
            elif isinstance(value, str):
                sanitized[str(key)] = sanitize_text(value, policy)
            elif isinstance(value, list):
                sanitized[str(key)] = [sanitize_text(v, policy) for v in value[:16]]
            else:
                sanitized[str(key)] = sanitize_text(json.dumps(value, sort_keys=True), policy)
        item["details"] = sanitized
    return item


def stage_result(
    policy: dict[str, Any],
    stage: str,
    status: str,
    *,
    reasons: Iterable[dict[str, Any]] = (),
    evidence: dict[str, Any] | None = None,
) -> dict[str, Any]:
    if stage not in policy.get("stage_order", []):
        fail(f"unknown diagnostic stage: {stage}")
    if status not in policy.get("statuses", []):
        fail(f"unknown diagnostic status: {status}")
    reason_list = list(reasons)
    max_reasons = int(policy.get("output_limits", {}).get("maximum_reason_count", 64))
    if len(reason_list) > max_reasons:
        fail(f"stage {stage} has too many reasons")
    if status == "verified" and reason_list:
        fail(f"verified stage {stage} cannot carry refusal reasons")
    result: dict[str, Any] = {"stage": stage, "status": status, "reasons": reason_list}
    if evidence:
        result["evidence"] = evidence
    return result


def build_reason_graph(policy: dict[str, Any], stage_results: Iterable[dict[str, Any]]) -> dict[str, Any]:
    order = list(policy["stage_order"])
    by_stage = {str(item.get("stage")): item for item in stage_results}
    if set(by_stage) != set(order):
        missing = sorted(set(order) - set(by_stage))
        extra = sorted(set(by_stage) - set(order))
        fail(f"stage result coverage mismatch; missing={missing}, extra={extra}")
    nodes: list[dict[str, Any]] = []
    root_causes: list[str] = []
    blocked: dict[str, list[str]] = {}
    for stage in order:
        item = by_stage[stage]
        dependencies = list(policy["stage_dependencies"][stage])
        failed_dependencies = [dep for dep in dependencies if by_stage[dep].get("status") != "verified"]
        if failed_dependencies:
            blocked[stage] = failed_dependencies
        if item.get("status") in {"refused", "unavailable"} and not failed_dependencies:
            root_causes.append(stage)
        nodes.append(
            {
                "stage": stage,
                "status": item.get("status"),
                "dependencies": dependencies,
                "blocked_by": failed_dependencies,
                "reason_codes": [entry.get("code") for entry in item.get("reasons", [])],
            }
        )
    return {"nodes": nodes, "root_cause_stages": root_causes, "blocked_stages": blocked}


def build_input_manifest(inputs: dict[str, pathlib.Path]) -> dict[str, Any]:
    manifest: dict[str, Any] = {}
    for role, path in sorted(inputs.items()):
        resolved = path.resolve(strict=False)
        item: dict[str, Any] = {"role": role, "name": path.name, "exists": path.exists()}
        if path.is_file():
            item.update({"kind": "file", "size_bytes": path.stat().st_size, "sha256": sha256_file(path)})
        elif path.is_dir():
            item.update({"kind": "directory"})
        else:
            item.update({"kind": "missing"})
        # Do not retain absolute paths. The basename and content digest are sufficient.
        item["resolved_name"] = resolved.name
        manifest[role] = item
    return manifest


def build_report(
    policy: dict[str, Any],
    stage_results: Iterable[dict[str, Any]],
    *,
    source_revision: str | None,
    inputs: dict[str, pathlib.Path],
    report_kind: str,
) -> dict[str, Any]:
    results = list(stage_results)
    graph = build_reason_graph(policy, results)
    statuses = {item["status"] for item in results}
    if "refused" in statuses:
        status = "refused"
    elif "unavailable" in statuses:
        status = "unavailable"
    elif "verified" in statuses:
        status = "verified"
    else:
        status = "refused"
    manifest = build_input_manifest(inputs)
    policy_path = pathlib.Path(str(policy.get("_policy_path", DEFAULT_POLICY)))
    identity = {
        "policy_sha256": sha256_file(policy_path),
        "report_kind": report_kind,
        "source_revision": source_revision,
        "inputs": manifest,
        "stage_results": results,
    }
    digest = sha256_bytes(canonical_bytes(identity))
    return {
        "schema_version": 1,
        "report_kind": report_kind,
        "report_id": f"health-promotion-{report_kind}-{digest[:24]}",
        "status": status,
        "release_id": policy.get("release_id"),
        "source_revision": source_revision,
        "policy_sha256": sha256_file(policy_path),
        "inputs": manifest,
        "stage_results": results,
        "reason_graph": graph,
        "report_digest_sha256": digest,
    }


def write_create_only(path: pathlib.Path, value: dict[str, Any]) -> None:
    if path.exists():
        fail(f"refusing to overwrite {path.name}")
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    with path.open("x", encoding="utf-8") as handle:
        json.dump(value, handle, indent=2, sort_keys=True)
        handle.write("\n")
    os.chmod(path, 0o600)


def self_test() -> None:
    policy = load_policy()
    secret = "super-secret-token-value"
    os.environ["HEALTH_TEST_TOKEN"] = secret
    try:
        sanitized = sanitize_text(f"Authorization: Bearer {secret}; token={secret}", policy)
        assert secret not in sanitized and "[REDACTED]" in sanitized
        results = []
        for stage in policy["stage_order"]:
            if stage == "preflight":
                results.append(
                    stage_result(
                        policy,
                        stage,
                        "unavailable",
                        reasons=[reason(policy, "PREREQUISITE_TOOL_MISSING", "nix is missing", stage=stage)],
                    )
                )
            else:
                results.append(stage_result(policy, stage, "skipped"))
        graph = build_reason_graph(policy, results)
        assert graph["root_cause_stages"] == ["preflight"]
        with tempfile.TemporaryDirectory() as raw:
            root = pathlib.Path(raw)
            input_file = root / "input.json"
            input_file.write_text("{}\n", encoding="utf-8")
            report = build_report(
                policy,
                results,
                source_revision="1" * 40,
                inputs={"input": input_file},
                report_kind="self-test",
            )
            output = root / "report.json"
            write_create_only(output, report)
            assert output.stat().st_mode & 0o777 == 0o600
            assert load_json(output)["report_id"] == report["report_id"]
    finally:
        os.environ.pop("HEALTH_TEST_TOKEN", None)
    print("clinical promotion diagnostics self-test: ok")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if not args.self_test:
        parser.error("only --self-test is supported by the shared diagnostics module")
    self_test()
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (DiagnosticError, OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        print(f"clinical promotion diagnostics error: {error}")
        raise SystemExit(1)
