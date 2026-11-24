#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
HOST="$(rustc -vV | sed -n 's/^host: //p')"
if [[ -z "$HOST" ]]; then
  echo "Unable to determine Rust host target" >&2
  exit 2
fi
MANIFEST="$ROOT/prototypes/decision-sufficiency-bench/Cargo.toml"
FIXTURE="$ROOT/prototypes/decision-sufficiency-bench/fixtures/v1-cases.tsv"

echo "Mycelix Decision Sufficiency Benchmark v1"
echo "Repository: $ROOT"
echo "Host target: $HOST"
echo "Fixture catalogue: $FIXTURE"
sha256sum "$FIXTURE"

cargo fmt --manifest-path "$MANIFEST" -- --check
cargo test --manifest-path "$MANIFEST" --target "$HOST" --locked
