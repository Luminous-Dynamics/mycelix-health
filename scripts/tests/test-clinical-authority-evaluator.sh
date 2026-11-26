#!/usr/bin/env bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
MANIFEST="$ROOT/benchmarks/clinical-evidence-integrity/authority-semantics-v1.json"
ORACLE="$ROOT/benchmarks/clinical-evidence-integrity/example-oracle-results-v1.json"
EVALUATOR="$ROOT/scripts/evaluate-clinical-authority-semantics.sh"
"$EVALUATOR" "$MANIFEST" "$ORACLE" >/dev/null
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

# A FHIR-valid inactive Consent cannot be promoted to an allow.
jq '.results |= map(if .case_id == "deny_inactive_consent" then .decision = "allow" | .reason_codes = ["authorized"] else . end)' "$ORACLE" > "$tmp"
set +e; "$EVALUATOR" "$MANIFEST" "$tmp" >/dev/null 2>&1; status=$?; set -e
[[ "$status" -eq 2 ]] || { echo "Expected unauthorized allow to fail with status 2; got $status" >&2; exit 1; }

# An unavailable verifier cannot be promoted to an allow either.
jq '.results |= map(if .case_id == "defer_verifier_unavailable" then .decision = "allow" | .reason_codes = ["authorized"] else . end)' "$ORACLE" > "$tmp"
set +e; "$EVALUATOR" "$MANIFEST" "$tmp" >/dev/null 2>&1; status=$?; set -e
[[ "$status" -eq 2 ]] || { echo "Expected verifier-outage allow to fail with status 2; got $status" >&2; exit 1; }

# Missing clinical event time under date scope must not be replaced by ingestion time.
jq '.results |= map(if .case_id == "deny_missing_clinical_event_time" then .decision = "allow" | .reason_codes = ["authorized"] else . end)' "$ORACLE" > "$tmp"
set +e; "$EVALUATOR" "$MANIFEST" "$tmp" >/dev/null 2>&1; status=$?; set -e
[[ "$status" -eq 2 ]] || { echo "Expected missing event-time allow to fail with status 2; got $status" >&2; exit 1; }

# An incomplete candidate must fail structural validation.
jq '.results |= map(select(.case_id != "deny_inactive_consent"))' "$ORACLE" > "$tmp"
set +e; "$EVALUATOR" "$MANIFEST" "$tmp" >/dev/null 2>&1; status=$?; set -e
[[ "$status" -eq 1 ]] || { echo "Expected incomplete candidate to fail with status 1; got $status" >&2; exit 1; }
echo "PASS: oracle, unauthorized-release controls, outage control and incomplete-result rejection"
