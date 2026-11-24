#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
HOST="$(rustc -vV | sed -n 's/^host: //p')"

if [[ -z "$HOST" ]]; then
  echo "Unable to determine Rust host target" >&2
  exit 2
fi

CORPUS_DIR="$ROOT/benchmarks/clinical-evidence-integrity"

echo "Clinical Evidence Integrity FHIR Corpus Suite v1"
echo "Repository: $ROOT"
echo "Commit: $(git -C "$ROOT" rev-parse HEAD)"
echo "Rust: $(rustc --version)"
echo "Host target: $HOST"
sha256sum \
  "$CORPUS_DIR/fhir-subject-binding-v1.json" \
  "$CORPUS_DIR/observation-temporal-fidelity-v1.json" \
  "$CORPUS_DIR/fhir-bundle-preflight-v1.json"

cargo test \
  --manifest-path "$ROOT/Cargo.toml" \
  -p fhir_bridge_coordinator \
  --target "$HOST" \
  v1_corpus \
  -- --nocapture
