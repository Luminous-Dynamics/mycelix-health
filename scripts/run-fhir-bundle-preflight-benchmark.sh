#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
HOST="$(rustc -vV | sed -n 's/^host: //p')"

if [[ -z "$HOST" ]]; then
  echo "Unable to determine Rust host target" >&2
  exit 2
fi

CORPUS="$ROOT/benchmarks/clinical-evidence-integrity/fhir-bundle-preflight-v1.json"

echo "FHIR Bundle Preflight Benchmark v1"
echo "Repository: $ROOT"
echo "Host target: $HOST"
echo "Corpus: $CORPUS"
sha256sum "$CORPUS"

cargo test \
  --manifest-path "$ROOT/Cargo.toml" \
  -p fhir_bridge_coordinator \
  --target "$HOST" \
  fhir_bundle_preflight_v1_corpus \
  -- --nocapture
