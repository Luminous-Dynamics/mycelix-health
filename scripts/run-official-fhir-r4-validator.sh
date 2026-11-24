#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
LOCK="$ROOT/benchmarks/clinical-evidence-integrity/fhir-validator-lock-v1.json"
CORPUS="$ROOT/benchmarks/clinical-evidence-integrity/official-fhir-r4-core-v1.json"
OUT_DIR="${FHIR_VALIDATOR_OUTPUT_DIR:-$ROOT/.artifacts/official-fhir-r4-core-v1}"
mkdir -p "$OUT_DIR"

VERSION="$(jq -r '.release_tag' "$LOCK")"
URL="$(jq -r '.release_url' "$LOCK")"
EXPECTED_SHA="$(jq -r '.sha256' "$LOCK")"
FHIR_VERSION="$(jq -r '.fhir_version' "$LOCK")"
JAVA_MAJOR="$(jq -r '.java_major' "$LOCK")"
JAR_CACHE="${FHIR_VALIDATOR_CACHE_DIR:-$HOME/.cache/mycelix-health/fhir-validator/$VERSION}"
JAR="$JAR_CACHE/validator_cli.jar"

if [[ "$(java -version 2>&1 | head -n 1)" != *"$JAVA_MAJOR"* ]]; then
  echo "Expected Java major $JAVA_MAJOR; observed: $(java -version 2>&1 | head -n 1)" >&2
  exit 2
fi

mkdir -p "$JAR_CACHE"
if [[ ! -s "$JAR" ]]; then
  curl --fail --location --retry 4 --retry-delay 2 "$URL" --output "$JAR"
fi

echo "$EXPECTED_SHA  $JAR" | sha256sum --check
ACTUAL_SHA="$(sha256sum "$CORPUS" | cut -d ' ' -f 1)"
echo "FHIR Validator CLI: $VERSION"
echo "Validator SHA-256: $EXPECTED_SHA"
echo "Java: $(java -version 2>&1 | head -n 1)"
echo "FHIR version: $FHIR_VERSION"
echo "Corpus SHA-256: $ACTUAL_SHA"
echo "Output directory: $OUT_DIR"

case_count="$(jq -r '.cases | length' "$CORPUS")"
if [[ "$case_count" -lt 1 ]]; then
  echo "Official FHIR validator corpus is empty" >&2
  exit 1
fi

summary="$OUT_DIR/summary.tsv"
printf 'case_id\texpected\tactual_errors\tstatus\n' > "$summary"
processed=0

while IFS= read -r row; do
  case_id="$(jq -r '.id' <<< "$row")"
  expected="$(jq -r '.expected' <<< "$row")"
  if [[ "$expected" != "valid" && "$expected" != "invalid" ]]; then
    echo "Unknown expected validation state '$expected' for $case_id" >&2
    exit 1
  fi
  input="$OUT_DIR/$case_id.json"
  report="$OUT_DIR/$case_id-operation-outcome.json"
  console="$OUT_DIR/$case_id-console.txt"
  jq -c '.bundle' <<< "$row" > "$input"

  set +e
  java -Dfile.encoding=UTF-8 -jar "$JAR" "$input" -version "$FHIR_VERSION" -tx n/a -output "$report" > "$console" 2>&1
  rc=$?
  set -e

  if [[ ! -s "$report" ]]; then
    echo "Validator produced no OperationOutcome for $case_id (exit $rc)" >&2
    cat "$console" >&2
    exit 1
  fi
  if ! jq -e '.resourceType == "OperationOutcome" and (.issue | type == "array")' "$report" >/dev/null; then
    echo "Unexpected validator output for $case_id" >&2
    cat "$console" >&2
    exit 1
  fi

  errors="$(jq '[.issue[] | select(.severity == "error" or .severity == "fatal")] | length' "$report")"
  if [[ "$expected" == "valid" ]]; then
    if [[ "$errors" -ne 0 || "$rc" -ne 0 ]]; then
      echo "Expected FHIR-valid case $case_id but validator reported $errors errors (exit $rc)" >&2
      cat "$console" >&2
      jq . "$report" >&2
      exit 1
    fi
    status="pass"
  else
    if [[ "$errors" -eq 0 ]]; then
      echo "Expected FHIR-invalid case $case_id but validator reported no errors (exit $rc)" >&2
      cat "$console" >&2
      jq . "$report" >&2
      exit 1
    fi
    status="pass"
  fi

  printf '%s\t%s\t%s\t%s\n' "$case_id" "$expected" "$errors" "$status" | tee -a "$summary"
  processed=$((processed + 1))
done < <(jq -c '.cases[]' "$CORPUS")

if [[ "$processed" -ne "$case_count" ]]; then
  echo "Processed $processed of $case_count official-validator cases" >&2
  exit 1
fi

echo "Official FHIR R4 core validation passed for $processed cases."
