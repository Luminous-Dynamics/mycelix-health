#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
evaluator="$root/scripts/evaluate-fhir-transaction-qualification.sh"
fixture="$root/research/fhir-transaction-adapter-qualification/example-evidence-v1.json"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required" >&2
  exit 64
fi

now=1795737600 # 2026-11-27T00:00:00Z
export MYCELIX_NOW_EPOCH="$now"

expect_state() {
  local file="$1"
  local expected="$2"
  local actual
  local status=0
  bash "$evaluator" "$file" >"$tmp/result.json" || status=$?
  actual="$(jq -r '.qualification_state' "$tmp/result.json")"
  if [[ "$actual" != "$expected" ]]; then
    echo "expected $expected, got $actual (exit $status)" >&2
    cat "$tmp/result.json" >&2
    exit 1
  fi
  case "$expected" in
    qualified) [[ "$status" -eq 0 ]] ;;
    not_qualified) [[ "$status" -eq 2 ]] ;;
    defer) [[ "$status" -eq 3 ]] ;;
  esac
}

# The checked-in example has no actual probe evidence and must defer.
expect_state "$fixture" defer

# Build a fully synthetic positive record. Digests here are test sentinels,
# not evidence of a real server or cryptographic proof of the referenced blob.
jq --arg completed "2026-11-27T00:00:00Z" '
  .probe_evidence.completed_at = $completed
  | .probe_evidence.probes |= map(
      .result = "pass"
      | .evidence_sha256 = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
      | .observed_at = $completed
    )
' "$fixture" >"$tmp/qualified.json"
expect_state "$tmp/qualified.json" qualified

# An explicit required-probe failure is a positive reason not to qualify.
jq '.probe_evidence.probes[0].result = "fail"' "$tmp/qualified.json" >"$tmp/failed.json"
expect_state "$tmp/failed.json" not_qualified

# Missing probe evidence must defer.
jq '.probe_evidence.probes = [.probe_evidence.probes[0]]' "$tmp/qualified.json" >"$tmp/missing.json"
expect_state "$tmp/missing.json" defer

# Old evidence must not remain valid when the freshness window has elapsed.
jq '.probe_evidence.completed_at = "2026-11-25T00:00:00Z"' "$tmp/qualified.json" >"$tmp/stale.json"
expect_state "$tmp/stale.json" defer

# Capability drift between the active probe and captured metadata must defer.
jq '.probe_evidence.capability_sha256 = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"' \
  "$tmp/qualified.json" >"$tmp/drift.json"
expect_state "$tmp/drift.json" defer

# A server that explicitly does not advertise transactions is not qualified.
jq '.capability_observation.resource.rest[0].interaction = []' \
  "$tmp/qualified.json" >"$tmp/no-transaction.json"
expect_state "$tmp/no-transaction.json" not_qualified

# Duplicate required evidence rows are ambiguous and must defer.
jq '.probe_evidence.probes += [.probe_evidence.probes[0]]' \
  "$tmp/qualified.json" >"$tmp/duplicate.json"
expect_state "$tmp/duplicate.json" defer

# Probe evidence beyond the allowed future clock skew must defer.
jq '.probe_evidence.completed_at = "2026-11-27T01:00:00Z"' \
  "$tmp/qualified.json" >"$tmp/future.json"
expect_state "$tmp/future.json" defer

# A different claimed software identity must not inherit qualification.
jq '.probe_evidence.server_software_version = "9.9.9"' \
  "$tmp/qualified.json" >"$tmp/software-drift.json"
expect_state "$tmp/software-drift.json" defer

# Missing a component that participated in the end-to-end probes must defer.
jq '.execution_context.components |= map(select(.role != "recipient_sandbox"))' \
  "$tmp/qualified.json" >"$tmp/missing-component.json"
expect_state "$tmp/missing-component.json" defer

# Duplicated component roles make the execution environment ambiguous.
jq '.execution_context.components += [(.execution_context.components[0] | .component_id = "gateway-copy")]' \
  "$tmp/qualified.json" >"$tmp/duplicate-component.json"
expect_state "$tmp/duplicate-component.json" defer

# Probe results bound to another component inventory must defer.
jq '.probe_evidence.component_inventory_sha256 = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"' \
  "$tmp/qualified.json" >"$tmp/component-drift.json"
expect_state "$tmp/component-drift.json" defer

# Stale component identity/configuration evidence must not qualify.
jq '.execution_context.components[0].observed_at = "2026-11-25T00:00:00Z"' \
  "$tmp/qualified.json" >"$tmp/stale-component.json"
expect_state "$tmp/stale-component.json" defer

# Unexpected component roles cannot be smuggled into the qualified path.
jq '.execution_context.components[0].role = "untracked_proxy"' \
  "$tmp/qualified.json" >"$tmp/unexpected-component.json"
expect_state "$tmp/unexpected-component.json" defer

# Malformed build digests must not be treated as trustworthy inventory evidence.
jq '.execution_context.components[0].build_sha256 = "not-a-digest"' \
  "$tmp/qualified.json" >"$tmp/malformed-component-digest.json"
expect_state "$tmp/malformed-component-digest.json" defer

echo "FHIR transaction adapter qualification evaluator: all tests passed"
