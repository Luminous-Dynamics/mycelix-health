#!/usr/bin/env bash
set -euo pipefail

FHIR_BASE_URL="${FHIR_BASE_URL:-http://127.0.0.1:8080/fhir}"
ARTIFACT_DIR="${ARTIFACT_DIR:-./artifacts/fhir-qualification-$(date -u +%Y%m%dT%H%M%SZ)}"
mkdir -p "$ARTIFACT_DIR"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'Required command not found: %s\n' "$1" >&2
    exit 2
  }
}
require_command curl
require_command jq
require_command sha256sum

printf 'FHIR base URL: %s\n' "$FHIR_BASE_URL"
printf 'Evidence directory: %s\n' "$ARTIFACT_DIR"

# A CapabilityStatement is recorded as declared capability evidence only.
metadata_status="$(curl --silent --show-error -o "$WORK_DIR/metadata.json" -w '%{http_code}' \
  -H 'Accept: application/fhir+json' "$FHIR_BASE_URL/metadata")"
test "$metadata_status" = "200"
jq -e '
  .resourceType == "CapabilityStatement"
  and .fhirVersion == "4.0.1"
' "$WORK_DIR/metadata.json" >/dev/null
metadata_sha256="$(sha256sum "$WORK_DIR/metadata.json" | awk '{print $1}')"
cp "$WORK_DIR/metadata.json" "$ARTIFACT_DIR/capability-statement.json"
printf 'CapabilityStatement: HTTP %s; sha256=%s\n' "$metadata_status" "$metadata_sha256"

transaction_declared="$(jq -r '
  [.rest[]? | select(.mode == "server") | .interaction[]? | select(.code == "transaction")]
  | length > 0
' "$WORK_DIR/metadata.json")"
test "$transaction_declared" = "true"

run_id="$(date -u +%s)-$RANDOM-$RANDOM"
identifier_system="urn:mycelix:qualification:synthetic"
patient_identifier="patient-$run_id"
provenance_identifier="provenance-$run_id"
patient_full_url="urn:uuid:00000000-0000-4000-8000-000000000001"
provenance_full_url="urn:uuid:00000000-0000-4000-8000-000000000002"
recorded_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# A minimal synthetic Patient and its Provenance are submitted in one FHIR R4
# transaction. Both use conditional create so replay has an observable bound.
jq -n \
  --arg patient_full_url "$patient_full_url" \
  --arg provenance_full_url "$provenance_full_url" \
  --arg identifier_system "$identifier_system" \
  --arg patient_identifier "$patient_identifier" \
  --arg provenance_identifier "$provenance_identifier" \
  --arg recorded_at "$recorded_at" '
  {
    resourceType: "Bundle",
    type: "transaction",
    entry: [
      {
        fullUrl: $patient_full_url,
        resource: {
          resourceType: "Patient",
          active: true,
          identifier: [{system: $identifier_system, value: $patient_identifier}],
          name: [{family: "MycelixQualificationSynthetic"}]
        },
        request: {
          method: "POST",
          url: "Patient",
          ifNoneExist: ("identifier=" + $identifier_system + "|" + $patient_identifier)
        }
      },
      {
        fullUrl: $provenance_full_url,
        resource: {
          resourceType: "Provenance",
          identifier: [{system: $identifier_system, value: $provenance_identifier}],
          target: [{reference: $patient_full_url}],
          recorded: $recorded_at,
          activity: {
            coding: [{
              system: "http://terminology.hl7.org/CodeSystem/v3-DataOperation",
              code: "CREATE",
              display: "Create"
            }]
          },
          agent: [{who: {display: "synthetic Mycelix qualification harness"}}]
        },
        request: {
          method: "POST",
          url: "Provenance",
          ifNoneExist: ("identifier=" + $identifier_system + "|" + $provenance_identifier)
        }
      }
    ]
  }
' > "$WORK_DIR/transaction.json"

transaction_status="$(curl --silent --show-error --output "$WORK_DIR/transaction-response.json" \
  --write-out '%{http_code}' \
  -H 'Content-Type: application/fhir+json' \
  -H 'Accept: application/fhir+json' \
  --data-binary @"$WORK_DIR/transaction.json" "$FHIR_BASE_URL")"
case "$transaction_status" in
  2??) ;;
  *)
    cat "$WORK_DIR/transaction-response.json" >&2
    printf 'FHIR transaction failed: HTTP %s\n' "$transaction_status" >&2
    exit 1
    ;;
esac
jq -e '
  .resourceType == "Bundle"
  and .type == "transaction-response"
  and (.entry | length) == 2
' "$WORK_DIR/transaction-response.json" >/dev/null
cp "$WORK_DIR/transaction.json" "$ARTIFACT_DIR/transaction-request.json"
cp "$WORK_DIR/transaction-response.json" "$ARTIFACT_DIR/transaction-response.json"

search_total() {
  local resource_type="$1"
  local identifier_value="$2"
  curl --silent --show-error --fail --get "$FHIR_BASE_URL/$resource_type" \
    -H 'Accept: application/fhir+json' \
    --data-urlencode "_count=10" \
    --data-urlencode "identifier=$identifier_system|$identifier_value" \
    | jq -r '.total // 0'
}

patient_count="$(search_total Patient "$patient_identifier")"
provenance_count="$(search_total Provenance "$provenance_identifier")"
test "$patient_count" = "1"
test "$provenance_count" = "1"

# Replay the exact transaction. Conditional create should keep one Patient and
# one Provenance. This probes FHIR-level conditional create, not a distributed
# Mycelix idempotency guarantee.
replay_status="$(curl --silent --show-error --output "$WORK_DIR/replay-response.json" \
  --write-out '%{http_code}' \
  -H 'Content-Type: application/fhir+json' \
  -H 'Accept: application/fhir+json' \
  --data-binary @"$WORK_DIR/transaction.json" "$FHIR_BASE_URL")"
case "$replay_status" in
  2??) ;;
  *)
    cat "$WORK_DIR/replay-response.json" >&2
    printf 'FHIR conditional replay failed: HTTP %s\n' "$replay_status" >&2
    exit 1
    ;;
esac
test "$(search_total Patient "$patient_identifier")" = "1"
test "$(search_total Provenance "$provenance_identifier")" = "1"
cp "$WORK_DIR/replay-response.json" "$ARTIFACT_DIR/replay-response.json"

# Verify rollback: create a new marker Patient, then in the same transaction
# update the existing Patient with a deliberately impossible If-Match version.
# The operation must fail and the first create must not remain persisted.
existing_patient_json="$WORK_DIR/existing-patient.json"
curl --silent --show-error --fail --get "$FHIR_BASE_URL/Patient" \
  -H 'Accept: application/fhir+json' \
  --data-urlencode "_count=1" \
  --data-urlencode "identifier=$identifier_system|$patient_identifier" \
  > "$existing_patient_json"
existing_id="$(jq -r '.entry[0].resource.id // empty' "$existing_patient_json")"
existing_version="$(jq -r '.entry[0].resource.meta.versionId // empty' "$existing_patient_json")"
test -n "$existing_id"
test -n "$existing_version"
version_before_concurrency="$existing_version"
if_match_header="W/\\"${existing_version}\\""

# Two concurrent writers start from exactly the same resource version. The
# server must accept one If-Match update and reject the stale competitor.
jq -n \
  --arg id "$existing_id" \
  --arg system "$identifier_system" \
  --arg value "$patient_identifier" '
  {
    resourceType: "Patient",
    id: $id,
    active: true,
    identifier: [{system: $system, value: $value}],
    name: [{family: "MycelixConcurrentWriterA"}]
  }
' > "$WORK_DIR/concurrent-a.json"

jq -n \
  --arg id "$existing_id" \
  --arg system "$identifier_system" \
  --arg value "$patient_identifier" '
  {
    resourceType: "Patient",
    id: $id,
    active: false,
    identifier: [{system: $system, value: $value}],
    name: [{family: "MycelixConcurrentWriterB"}]
  }
' > "$WORK_DIR/concurrent-b.json"

start_gate="$WORK_DIR/concurrent-start"
(
  while [[ ! -e "$start_gate" ]]; do sleep 0.01; done
  curl --silent --show-error --output "$WORK_DIR/concurrent-a-response.json" \
    --write-out '%{http_code}' \
    -X PUT \
    -H "If-Match: $if_match_header" \
    -H 'Content-Type: application/fhir+json' \
    -H 'Accept: application/fhir+json' \
    --data-binary @"$WORK_DIR/concurrent-a.json" \
    "$FHIR_BASE_URL/Patient/$existing_id" > "$WORK_DIR/concurrent-a-status.txt"
) &
writer_a_pid=$!
(
  while [[ ! -e "$start_gate" ]]; do sleep 0.01; done
  curl --silent --show-error --output "$WORK_DIR/concurrent-b-response.json" \
    --write-out '%{http_code}' \
    -X PUT \
    -H "If-Match: W/\\"$existing_version\\"" \
    -H 'Content-Type: application/fhir+json' \
    -H 'Accept: application/fhir+json' \
    --data-binary @"$WORK_DIR/concurrent-b.json" \
    "$FHIR_BASE_URL/Patient/$existing_id" > "$WORK_DIR/concurrent-b-status.txt"
) &
writer_b_pid=$!
: > "$start_gate"
if ! wait "$writer_a_pid"; then
  cat "$WORK_DIR/concurrent-a-response.json" >&2 || true
  printf 'Concurrent writer A failed at transport level\n' >&2
  exit 1
fi
if ! wait "$writer_b_pid"; then
  cat "$WORK_DIR/concurrent-b-response.json" >&2 || true
  printf 'Concurrent writer B failed at transport level\n' >&2
  exit 1
fi

concurrent_a_status="$(cat "$WORK_DIR/concurrent-a-status.txt")"
concurrent_b_status="$(cat "$WORK_DIR/concurrent-b-status.txt")"
success_count=0
stale_count=0
for status in "$concurrent_a_status" "$concurrent_b_status"; do
  [[ "$status" = "200" ]] && ((success_count += 1))
  [[ "$status" = "412" ]] && ((stale_count += 1))
done
test "$success_count" = "1"
test "$stale_count" = "1"

curl --silent --show-error --fail "$FHIR_BASE_URL/Patient/$existing_id" \
  -H 'Accept: application/fhir+json' > "$WORK_DIR/concurrent-after.json"
version_after_concurrent="$(jq -r '.meta.versionId // empty' "$WORK_DIR/concurrent-after.json")"
test -n "$version_after_concurrent"
test "$version_after_concurrent" != "$existing_version"
cp "$WORK_DIR/concurrent-a-status.txt" "$ARTIFACT_DIR/concurrent-a-http-status.txt"
cp "$WORK_DIR/concurrent-b-status.txt" "$ARTIFACT_DIR/concurrent-b-http-status.txt"
cp "$WORK_DIR/concurrent-a-response.json" "$ARTIFACT_DIR/concurrent-a-response.json"
cp "$WORK_DIR/concurrent-b-response.json" "$ARTIFACT_DIR/concurrent-b-response.json"

# Use the winning writer's latest version for the transaction rollback probe.
existing_version="$version_after_concurrent"
rollback_identifier="rollback-$run_id"
jq -n \
  --arg identifier_system "$identifier_system" \
  --arg rollback_identifier "$rollback_identifier" \
  --arg existing_id "$existing_id" \
  --arg if_match 'W/"999999999"' '
  {
    resourceType: "Bundle",
    type: "transaction",
    entry: [
      {
        resource: {
          resourceType: "Patient",
          active: true,
          identifier: [{system: $identifier_system, value: $rollback_identifier}],
          name: [{family: "RollbackMustNotPersist"}]
        },
        request: {method: "POST", url: "Patient"}
      },
      {
        resource: {
          resourceType: "Patient",
          id: $existing_id,
          active: false
        },
        request: {
          method: "PUT",
          url: ("Patient/" + $existing_id),
          ifMatch: $if_match
        }
      }
    ]
  }
' > "$WORK_DIR/rollback-transaction.json"

rollback_status="$(curl --silent --show-error --output "$WORK_DIR/rollback-response.json" \
  --write-out '%{http_code}' \
  -H 'Content-Type: application/fhir+json' \
  -H 'Accept: application/fhir+json' \
  --data-binary @"$WORK_DIR/rollback-transaction.json" "$FHIR_BASE_URL")"
if [[ "$rollback_status" != "412" ]]; then
  cat "$WORK_DIR/rollback-response.json" >&2
  printf 'Expected HTTP 412 for the stale If-Match transaction; got HTTP %s\n' "$rollback_status" >&2
  exit 1
fi
test "$(search_total Patient "$rollback_identifier")" = "0"

curl --silent --show-error --fail "$FHIR_BASE_URL/Patient/$existing_id" \
  -H 'Accept: application/fhir+json' > "$WORK_DIR/existing-after-rollback.json"
version_after="$(jq -r '.meta.versionId // empty' "$WORK_DIR/existing-after-rollback.json")"
test "$version_after" = "$existing_version"
cp "$WORK_DIR/rollback-response.json" "$ARTIFACT_DIR/rollback-response.json"

# This is deliberately an incomplete qualification result. The harness does
# not test consent revocation at commit, cross-service recovery, recipient-side
# effects, independent signing, or security/audit configuration.
jq -n \
  --arg timestamp "$recorded_at" \
  --arg base_url "$FHIR_BASE_URL" \
  --arg metadata_sha256 "$metadata_sha256" \
  --arg transaction_http "$transaction_status" \
  --arg replay_http "$replay_status" \
  --arg rollback_http "$rollback_status" \
  --arg concurrent_a_http "$concurrent_a_status" \
  --arg concurrent_b_http "$concurrent_b_status" \
  --arg version_before_concurrency "$version_before_concurrency" \
  --arg version_after_concurrency "$version_after_concurrent" \
  --arg patient_count "$patient_count" \
  --arg provenance_count "$provenance_count" \
  --arg existing_version_before "$existing_version" \
  --arg existing_version_after "$version_after" '
  {
    schema_version: 1,
    harness: "mycelix-hapi-r4-postgres-probes-v1",
    observed_at: $timestamp,
    fhir_base_url: $base_url,
    capability_statement_sha256: $metadata_sha256,
    declared_transaction_interaction: true,
    probes: {
      patient_provenance_transaction: {result: "passed", http_status: $transaction_http},
      conditional_create_replay: {
        result: "passed",
        http_status: $replay_http,
        patient_count_after_replay: ($patient_count | tonumber),
        provenance_count_after_replay: ($provenance_count | tonumber)
      },
      concurrent_if_match_writers: {
        result: "passed",
        writer_a_http_status: $concurrent_a_http,
        writer_b_http_status: $concurrent_b_http,
        version_before: $version_before_concurrency,
        version_after: $version_after_concurrency,
        accepted_writers: 1,
        stale_rejections: 1
      },
      stale_if_match_transaction_rollback: {
        result: "passed",
        http_status: $rollback_http,
        rollback_marker_count_after_failure: 0,
        existing_resource_version_before: $existing_version_before,
        existing_resource_version_after: $existing_version_after
      }
    },
    qualification_verdict: "defer",
    defer_reasons: [
      "No Mycelix consent-revocation/commit fence was exercised",
      "No gateway/outbox crash or lost-acknowledgement reconciliation was exercised",
      "No independent recipient-effect witness was exercised",
      "No independent cryptographic qualification receipt was issued",
      "No authentication, authorization or enterprise audit layer was configured by this local test stack"
    ],
    phi_used: false
  }
' > "$ARTIFACT_DIR/qualification-evidence.json"

printf '\nProbe slice passed. Overall qualification remains DEFER.\n'
printf 'Evidence: %s/qualification-evidence.json\n' "$ARTIFACT_DIR"
