#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 <qualification-evidence.json>" >&2
}

if [[ $# -ne 1 ]]; then
  usage
  exit 64
fi

script_dir="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$script_dir/.." && pwd)"
input="$1"
manifest="$root/research/fhir-transaction-adapter-qualification/manifest-v1.json"

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required" >&2
  exit 64
fi
if [[ ! -r "$input" || ! -r "$manifest" ]]; then
  echo "input or qualification manifest is not readable" >&2
  exit 64
fi

now="${MYCELIX_NOW_EPOCH:-$(date -u +%s)}"
if [[ ! "$now" =~ ^[0-9]+$ ]]; then
  echo "MYCELIX_NOW_EPOCH must be a non-negative integer" >&2
  exit 64
fi

if ! jq -e 'type == "object"' "$input" >/dev/null 2>&1; then
  echo "qualification evidence is not a valid JSON object" >&2
  exit 65
fi

result="$(jq -n \
  --slurpfile doc "$input" \
  --slurpfile spec "$manifest" \
  --argjson now "$now" '
  def verdict($state; $reason):
    {
      qualification_state: $state,
      reason: $reason,
      benchmark_id: "mycelix.fhir-transaction-adapter-qualification.v1"
    };
  def digest_ok($value):
    ($value | type) == "string"
    and ($value | test("^sha256:[0-9a-f]{64}$"));
  def epoch_or_null($value):
    try ($value | fromdateiso8601) catch null;
  def fresh($value; $now; $max_age; $future_skew):
    (epoch_or_null($value)) as $timestamp
    | $timestamp != null
      and $timestamp <= ($now + $future_skew)
      and ($now - $timestamp) <= $max_age;

  ($doc[0] // null) as $d
  | ($spec[0] // null) as $s
  | if ($d | type) != "object" then
      verdict("defer"; "evidence_document_missing_or_invalid")
    elif ($d.schema_version != 1) or ($s.schema_version != 1) then
      verdict("defer"; "unsupported_evidence_or_manifest_schema")
    elif ($d.server | type) != "object"
      or ($d.capability_observation | type) != "object"
      or ($d.capability_observation.resource | type) != "object"
      or ($d.probe_evidence | type) != "object" then
      verdict("defer"; "required_evidence_section_missing")
    elif $d.server.fhir_version != $s.target_fhir_version
      or $d.capability_observation.resource.fhirVersion != $s.target_fhir_version then
      verdict("not_qualified"; "fhir_version_not_supported")
    elif $d.capability_observation.resource.resourceType != "CapabilityStatement"
      or $d.capability_observation.resource.kind != "instance"
      or ($d.server.base_url | type) != "string"
      or ($d.server.base_url | length) == 0
      or $d.capability_observation.resource.implementation.url != $d.server.base_url then
      verdict("not_qualified"; "capability_statement_not_bound_to_deployed_instance")
    elif ([
      $d.capability_observation.resource.rest[]?
      | select(.mode == "server")
      | .interaction[]?
      | select(.code == "transaction")
    ] | length) == 0 then
      verdict("not_qualified"; "server_does_not_advertise_system_transaction")
    elif ($d.capability_observation.resource.software.version | type) != "string"
      or $d.server.software_version != $d.capability_observation.resource.software.version then
      verdict("defer"; "server_software_identity_mismatch")
    elif (digest_ok($d.capability_observation.sha256) | not)
      or (digest_ok($d.probe_evidence.capability_sha256) | not) then
      verdict("defer"; "capability_digest_missing_or_malformed")
    elif $d.probe_evidence.capability_sha256 != $d.capability_observation.sha256 then
      verdict("defer"; "probe_evidence_bound_to_different_capability_statement")
    elif $d.probe_evidence.server_software_version != $d.server.software_version then
      verdict("defer"; "probe_evidence_bound_to_different_server_build")
    elif $d.probe_evidence.suite_id != $s.benchmark_id
      or $d.probe_evidence.suite_version != 1 then
      verdict("defer"; "probe_suite_identity_mismatch")
    elif (fresh(
      $d.capability_observation.observed_at;
      $now;
      $s.policy.maximum_evidence_age_seconds;
      $s.policy.maximum_future_clock_skew_seconds
    ) | not) then
      verdict("defer"; "capability_observation_stale_or_future_dated")
    elif (fresh(
      $d.probe_evidence.completed_at;
      $now;
      $s.policy.maximum_evidence_age_seconds;
      $s.policy.maximum_future_clock_skew_seconds
    ) | not) then
      verdict("defer"; "probe_evidence_stale_or_future_dated")
    elif ($d.probe_evidence.probes | type) != "array" then
      verdict("defer"; "probe_results_missing")
    elif any($d.probe_evidence.probes[]?; .result == "fail") then
      verdict("not_qualified"; "required_probe_failed")
    elif any($s.required_probes[] as $required;
      ([$d.probe_evidence.probes[]? | select(.id == $required.id)]) | length != 1
    ) then
      verdict("defer"; "required_probe_missing_or_duplicated")
    elif any($s.required_probes[] as $required;
      ([$d.probe_evidence.probes[]? | select(.id == $required.id)]) as $matches
      | $matches[0].result != "pass"
    ) then
      verdict("defer"; "required_probe_not_passed")
    elif any($s.required_probes[] as $required;
      ([$d.probe_evidence.probes[]? | select(.id == $required.id)]) as $matches
      | (digest_ok($matches[0].evidence_sha256) | not)
        or (fresh(
          $matches[0].observed_at;
          $now;
          $s.policy.maximum_evidence_age_seconds;
          $s.policy.maximum_future_clock_skew_seconds
        ) | not)
    ) then
      verdict("defer"; "probe_artifact_digest_or_timestamp_invalid")
    else
      verdict("qualified"; "all_required_probes_passed_for_exact_fresh_instance")
    end
')"

printf '%s\n' "$result"
state="$(jq -r '.qualification_state' <<<"$result")"
case "$state" in
  qualified) exit 0 ;;
  not_qualified) exit 2 ;;
  defer) exit 3 ;;
  *) echo "internal evaluator state error" >&2; exit 65 ;;
esac
