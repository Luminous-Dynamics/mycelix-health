#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
MANIFEST="$ROOT/benchmarks/clinical-evidence-integrity/authority-semantics-v1.json"
ORACLE="$ROOT/benchmarks/clinical-evidence-integrity/example-oracle-results-v1.json"
EVALUATOR="$ROOT/scripts/evaluate-clinical-authority-semantics.sh"
REFERENCE="$ROOT/scripts/run-clinical-authority-reference.sh"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
baseline="$tmpdir/reference-results.json"
score="$tmpdir/reference-score.json"
unlabelled_manifest="$tmpdir/unlabelled-manifest.json"
unlabelled_results="$tmpdir/unlabelled-results.json"
mutated="$tmpdir/mutated-results.json"

# The context-derived reference candidate must independently reproduce all
# public policy fixtures, including expected reason codes and defer semantics.
bash "$REFERENCE" "$MANIFEST" "$baseline"
bash "$EVALUATOR" "$MANIFEST" "$baseline" "$score" >/dev/null
jq -e '
  .case_count == 38
  and .exact_decision_accuracy == 1
  and .unauthorized_allow_count == 0
  and .false_denial_count == 0
  and .unexpected_defer_on_allow_count == 0
  and .required_restricted_reason_coverage == 1
  and .metamorphic_pair_count == 35
  and .metamorphic_pair_failures == 0
  and .authority_gate_pass == true
  and .metamorphic_gate_pass == true
' "$score" >/dev/null || {
  echo "Reference policy candidate did not exactly satisfy the public development contract" >&2
  cat "$score" >&2
  exit 1
}

# Non-anticipativity control: removing expected labels must not change any
# candidate-visible decision, reason, evidence reference or policy version.
jq 'del(.cases[].expected_decision, .cases[].expected_reason_code)' "$MANIFEST" > "$unlabelled_manifest"
bash "$REFERENCE" "$unlabelled_manifest" "$unlabelled_results"
cmp <(jq -S '.results' "$baseline") <(jq -S '.results' "$unlabelled_results")

# A FHIR-valid inactive Consent cannot be promoted to an allow.
jq '.results |= map(if .case_id == "deny_inactive_consent" then .decision = "allow" | .reason_codes = ["authorized"] else . end)' "$baseline" > "$mutated"
set +e; bash "$EVALUATOR" "$MANIFEST" "$mutated" >/dev/null 2>&1; status=$?; set -e
[[ "$status" -eq 2 ]] || { echo "Expected inactive-consent allow to fail with status 2; got $status" >&2; exit 1; }

# An unavailable verifier cannot be promoted to an allow.
jq '.results |= map(if .case_id == "defer_verifier_unavailable" then .decision = "allow" | .reason_codes = ["authorized"] else . end)' "$baseline" > "$mutated"
set +e; bash "$EVALUATOR" "$MANIFEST" "$mutated" >/dev/null 2>&1; status=$?; set -e
[[ "$status" -eq 2 ]] || { echo "Expected verifier-outage allow to fail with status 2; got $status" >&2; exit 1; }

# Missing clinical event time under date scope must not be replaced by ingestion time.
jq '.results |= map(if .case_id == "deny_missing_clinical_event_time" then .decision = "allow" | .reason_codes = ["authorized"] else . end)' "$baseline" > "$mutated"
set +e; bash "$EVALUATOR" "$MANIFEST" "$mutated" >/dev/null 2>&1; status=$?; set -e
[[ "$status" -eq 2 ]] || { echo "Expected missing event-time allow to fail with status 2; got $status" >&2; exit 1; }

# Missing privacy Consent subject remains an authority denial despite the
# companion FHIR core validator's warnings-only behavior for this fixture.
jq '.results |= map(if .case_id == "deny_missing_consent_subject" then .decision = "allow" | .reason_codes = ["authorized"] else . end)' "$baseline" > "$mutated"
set +e; bash "$EVALUATOR" "$MANIFEST" "$mutated" >/dev/null 2>&1; status=$?; set -e
[[ "$status" -eq 2 ]] || { echo "Expected missing-subject allow to fail with status 2; got $status" >&2; exit 1; }

# Provenance, conflict and audit failures must not be admitted.
for case_id in \
  deny_missing_provenance_binding \
  deny_invalid_provenance_signature \
  deny_unresolved_source_conflict \
  deny_invalid_audit_chain \
  deny_unauthenticated_audit_actor \
  defer_source_registry_unavailable; do
  jq --arg id "$case_id" '.results |= map(if .case_id == $id then .decision = "allow" | .reason_codes = ["authorized"] else . end)' "$baseline" > "$mutated"
  set +e; bash "$EVALUATOR" "$MANIFEST" "$mutated" >/dev/null 2>&1; status=$?; set -e
  [[ "$status" -eq 2 ]] || { echo "Expected $case_id allow to fail with status 2; got $status" >&2; exit 1; }
done

# Public scorer fixture still validates the scorer independently.
bash "$EVALUATOR" "$MANIFEST" "$ORACLE" >/dev/null

# A missing case is a malformed candidate and must fail preflight.
jq '.results |= map(select(.case_id != "deny_inactive_consent"))' "$baseline" > "$mutated"
set +e; bash "$EVALUATOR" "$MANIFEST" "$mutated" >/dev/null 2>&1; status=$?; set -e
[[ "$status" -eq 1 ]] || { echo "Expected incomplete candidate to fail with status 1; got $status" >&2; exit 1; }

echo "PASS: context-derived reference, exact score, label-invariance, unauthorized-release controls, and malformed-result rejection"
