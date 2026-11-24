#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
MANIFEST="$ROOT/prototypes/decision-sufficiency-bench/Cargo.toml"
FIXTURE="$ROOT/prototypes/decision-sufficiency-bench/fixtures/v1-cases.tsv"

echo "Mycelix Decision Sufficiency Benchmark v1"
echo "Repository: $ROOT"
echo "Fixture catalogue: $FIXTURE"
sha256sum "$FIXTURE"

cargo fmt --manifest-path "$MANIFEST" -- --check
cargo test --manifest-path "$MANIFEST" --locked
