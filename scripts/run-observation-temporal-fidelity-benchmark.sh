#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
HOST="$(rustc -vV | sed -n 's/^host: //p')"

if [[ -z "$HOST" ]]; then
  echo "Unable to determine Rust host target" >&2
  exit 2
fi

echo "Observation Temporal Fidelity Benchmark v1"
echo "Repository: $ROOT"
echo "Host target: $HOST"
echo "Corpus: benchmarks/clinical-evidence-integrity/observation-temporal-fidelity-v1.json"
sha256sum "$ROOT/benchmarks/clinical-evidence-integrity/observation-temporal-fidelity-v1.json"

cargo test \
  --manifest-path "$ROOT/Cargo.toml" \
  -p fhir_bridge_coordinator \
  --target "$HOST" \
  observation_temporal_fidelity_v1_corpus \
  -- --nocapture
