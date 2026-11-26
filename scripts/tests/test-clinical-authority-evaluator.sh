#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
MANIFEST="$ROOT/benchmarks/clinical-evidence-integrity/authority-semantics-v1.json"
ORACLE="$ROOT/benchmarks/clinical-evidence-integrity/example-oracle-results-v1.json"
EVALUATOR="$ROOT/scripts/evaluate-clinical-authority-semantics.sh"

"$EVALUATOR" "$MANIFEST" "$ORACLE" >/dev/null

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

# A single unauthorized release of the FHIR-valid inactive Consent must fail.
jq '
  .results |= map(
    if .case_id == "deny_inactive_consent"
    then .decision = "allow" | .reason_codes = ["authorized"]
    else . end
  )
' "$ORACLE" > "$tmp"

set +e
"$EVALUATOR" "$MANIFEST" "$tmp" >/dev/null 2>&1
status=$?
set -e
if [[ "$status" -ne 2 ]]; then
  echo "Expected the permissive candidate to fail with status 2; got $status" >&2
  exit 1
fi

# Missing a case must fail structural preflight rather than improve the score.
jq '.results |= map(select(.case_id != "deny_inactive_consent"))' "$ORACLE" > "$tmp"
set +e
"$EVALUATOR" "$MANIFEST" "$tmp" >/dev/null 2>&1
status=$?
set -e
if [[ "$status" -ne 1 ]]; then
  echo "Expected the incomplete candidate to fail with status 1; got $status" >&2
  exit 1
fi

echo "PASS: oracle, unauthorized-release negative control, and incomplete-result rejection"
