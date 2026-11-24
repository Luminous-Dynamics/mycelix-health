#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
HOST="$(rustc -vV | sed -n 's/^host: //p')"

if [[ -z "$HOST" ]]; then
  echo "Unable to determine Rust host target" >&2
  exit 2
fi

echo "FHIR Subject Binding Benchmark v1"
echo "Repository: $ROOT"
echo "Host target: $HOST"
echo "Corpus: benchmarks/clinical-evidence-integrity/fhir-subject-binding-v1.json"
sha256sum "$ROOT/benchmarks/clinical-evidence-integrity/fhir-subject-binding-v1.json"

cargo test \
  --manifest-path "$ROOT/Cargo.toml" \
  -p fhir_bridge \
  --target "$HOST" \
  fhir_subject_binding_v1_corpus \
  -- --nocapture
