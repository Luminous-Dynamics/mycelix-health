#!/usr/bin/env bash
set -euo pipefail
usage() { echo "Usage: $0 MANIFEST.json CANDIDATE_RESULTS.json [REPORT.json]" >&2; exit 64; }
[[ $# -ge 2 && $# -le 3 ]] || usage
MANIFEST="$1"; CANDIDATE="$2"; REPORT="${3:-}"
for file in "$MANIFEST" "$CANDIDATE"; do [[ -s "$file" ]] || { echo "Missing or empty input: $file" >&2; exit 1; }; done
command -v jq >/dev/null 2>&1 || { echo "jq is required" >&2; exit 1; }
command -v sha256sum >/dev/null 2>&1 || { echo "sha256sum is required" >&2; exit 1; }

if ! jq -e '
  . as $root
  | (.schema_version == 1)
  and (.benchmark_id | type == "string" and length > 0)
  and (.cases | type == "array" and length > 0)
  and (([$root.cases[].id] | length) == ([$root.cases[].id] | unique | length))
  and all($root.cases[];
    (.id | type == "string" and length > 0)
    and (.expected_decision == "allow" or .expected_decision == "deny" or .expected_decision == "defer")
    and (.context.source | type == "object")
    and (.context.patient | type == "object")
    and (.context.consent | type == "object")
    and (.context.request | type == "object")
    and (.context.request.as_of_time | type == "string" and length > 0)
    and (if (.expected_reason_code == "clinical_event_time_missing")
         then (.context.request.clinical_event_time == null)
         else (.context.request.clinical_event_time | type == "string" and length > 0)
         end)
    and (.context.registry | type == "object")
    and (if .expected_decision != "allow"
         then (.expected_reason_code as $reason
               | ($reason | type == "string" and length > 0)
               and ($root.reason_codes | index($reason) != null))
         else true end)
  )
  and ($root.metamorphic_pairs | type == "array")
  and all($root.metamorphic_pairs[];
    . as $pair
    | any($root.cases[]; .id == $pair.baseline_case_id and .expected_decision == "allow")
    and any($root.cases[]; .id == $pair.restricted_case_id and .expected_decision == $pair.expected_restricted_decision)
    and ($pair.expected_baseline_decision == "allow")
    and ($pair.expected_restricted_decision == "deny" or $pair.expected_restricted_decision == "defer")
  )
' "$MANIFEST" >/dev/null; then echo "Manifest failed schema/invariant preflight" >&2; exit 1; fi

BENCHMARK_ID="$(jq -r '.benchmark_id' "$MANIFEST")"
if ! jq -e --arg id "$BENCHMARK_ID" '
  (.schema_version == 1)
  and (.benchmark_id == $id)
  and (.results | type == "array")
  and all(.results[];
    (.case_id | type == "string" and length > 0)
    and (.decision == "allow" or .decision == "deny" or .decision == "defer")
    and (.reason_codes | type == "array" and all(.[]; type == "string" and length > 0))
    and (.evidence_refs | type == "array" and length > 0 and all(.[]; type == "string" and length > 0))
    and (.policy_version | type == "string" and length > 0)
    and (if .decision == "allow" then (.reason_codes | index("authorized") != null)
         else (.reason_codes | length > 0) end)
  )
  and (([.results[].case_id] | length) == ([.results[].case_id] | unique | length))
' "$CANDIDATE" >/dev/null; then echo "Candidate result file failed schema/invariant preflight" >&2; exit 1; fi

EXPECTED_IDS="$(jq -c '[.cases[].id] | sort' "$MANIFEST")"
ACTUAL_IDS="$(jq -c '[.results[].case_id] | sort' "$CANDIDATE")"
if [[ "$EXPECTED_IDS" != "$ACTUAL_IDS" ]]; then
  echo "Candidate case IDs do not exactly match the benchmark manifest" >&2
  echo "Expected: $EXPECTED_IDS" >&2; echo "Actual:   $ACTUAL_IDS" >&2; exit 1
fi
MANIFEST_SHA="$(sha256sum "$MANIFEST" | awk '{print $1}')"
CANDIDATE_SHA="$(sha256sum "$CANDIDATE" | awk '{print $1}')"
TEMP_REPORT=""
if [[ -n "$REPORT" ]]; then OUTPUT_FILE="$REPORT"; else OUTPUT_FILE="$(mktemp)"; TEMP_REPORT="$OUTPUT_FILE"; fi
trap '[[ -z "$TEMP_REPORT" ]] || rm -f "$TEMP_REPORT"' EXIT

jq -n --arg manifest_sha256 "$MANIFEST_SHA" --arg candidate_sha256 "$CANDIDATE_SHA" --slurpfile manifest "$MANIFEST" --slurpfile candidate "$CANDIDATE" '
  ($manifest[0].cases) as $cases
  | ($candidate[0].results) as $results
  | [
      $cases[] as $case
      | ($results | map(select(.case_id == $case.id)) | .[0]) as $result
      | {
          case_id: $case.id,
          expected: $case.expected_decision,
          actual: $result.decision,
          decision_correct: ($case.expected_decision == $result.decision),
          reason_correct: (
            if $case.expected_decision != "allow"
            then (($result.decision == $case.expected_decision) and
                  (($result.reason_codes | index($case.expected_reason_code)) != null))
            else true end
          )
        }
    ] as $rows
  | [
      $manifest[0].metamorphic_pairs[] as $pair
      | {
          baseline_case_id: $pair.baseline_case_id,
          restricted_case_id: $pair.restricted_case_id,
          baseline_actual: ($results | map(select(.case_id == $pair.baseline_case_id)) | .[0].decision),
          restricted_actual: ($results | map(select(.case_id == $pair.restricted_case_id)) | .[0].decision),
          expected_restricted_decision: $pair.expected_restricted_decision,
          pass: (
            (($results | map(select(.case_id == $pair.baseline_case_id)) | .[0].decision) == "allow")
            and
            (($results | map(select(.case_id == $pair.restricted_case_id)) | .[0].decision) == $pair.expected_restricted_decision)
          )
        }
    ] as $pairs
  | ([$rows[] | select(.expected != "allow")] | length) as $restricted_cases
  | ([$rows[] | select(.expected == "deny")] | length) as $expected_denies
  | ([$rows[] | select(.expected == "defer")] | length) as $expected_defers
  | ([$rows[] | select(.expected == "allow")] | length) as $expected_allows
  | ([$rows[] | select(.expected != "allow" and .actual == "allow")] | length) as $unauthorized_allows
  | ([$rows[] | select(.expected == "allow" and .actual == "deny")] | length) as $false_denials
  | ([$rows[] | select(.expected == "allow" and .actual == "defer")] | length) as $unexpected_defers
  | ([$rows[] | select(.expected == "defer" and .actual == "defer")] | length) as $correct_defers
  | ([$rows[] | select(.expected == "deny" and .actual == "deny")] | length) as $correct_denies
  | ([$rows[] | select(.decision_correct)] | length) as $correct
  | ([$rows[] | select(.reason_correct and .expected != "allow")] | length) as $reason_matches
  | ([$pairs[] | select(.pass == false)] | length) as $pair_failures
  | {
      benchmark_id: $manifest[0].benchmark_id,
      case_count: ($rows | length),
      expected_allow_count: $expected_allows,
      expected_deny_count: $expected_denies,
      expected_defer_count: $expected_defers,
      correct_decisions: $correct,
      exact_decision_accuracy: (if ($rows | length) > 0 then ($correct / ($rows | length)) else null end),
      unauthorized_allow_count: $unauthorized_allows,
      unauthorized_allow_rate: (if $restricted_cases > 0 then ($unauthorized_allows / $restricted_cases) else null end),
      false_denial_count: $false_denials,
      unexpected_defer_on_allow_count: $unexpected_defers,
      authorized_allow_recall: (if $expected_allows > 0 then (($expected_allows - $false_denials - $unexpected_defers) / $expected_allows) else null end),
      correct_denial_count: $correct_denies,
      correct_defer_count: $correct_defers,
      required_restricted_reason_coverage: (if $restricted_cases > 0 then ($reason_matches / $restricted_cases) else null end),
      metamorphic_pair_count: ($pairs | length),
      metamorphic_pair_failures: $pair_failures,
      authority_gate_pass: ($unauthorized_allows == 0),
      metamorphic_gate_pass: ($pair_failures == 0),
      manifest_sha256: $manifest_sha256,
      candidate_sha256: $candidate_sha256,
      case_results: $rows,
      metamorphic_results: $pairs
    }
' > "$OUTPUT_FILE"
cat "$OUTPUT_FILE"
if jq -e '.unauthorized_allow_count > 0 or .metamorphic_pair_failures > 0' "$OUTPUT_FILE" >/dev/null; then
  echo "FAIL: authority or metamorphic gate failed" >&2; exit 2
fi
echo "PASS: zero unauthorized allows and all restrictive metamorphic pairs passed" >&2
