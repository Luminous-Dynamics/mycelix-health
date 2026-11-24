#!/usr/bin/env sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
MANIFEST="$ROOT/prototypes/temporal-evidence-integrity-bench/Cargo.toml"
FIXTURES="$ROOT/prototypes/temporal-evidence-integrity-bench/fixtures"
HOST="$(rustc -vV | sed -n 's/^host: //p')"
sha256sum "$FIXTURES/v1-task-inputs.tsv" "$FIXTURES/v1-events.tsv" "$FIXTURES/v1-cases.tsv" "$FIXTURES/v1-answer-key-events.tsv"
cargo fmt --locked --manifest-path "$MANIFEST" -- --check
cargo clippy --locked --manifest-path "$MANIFEST" --target "$HOST" --all-targets -- -D warnings
cargo test --locked --manifest-path "$MANIFEST" --target "$HOST"
cargo run --locked --manifest-path "$MANIFEST" --target "$HOST" --bin naive_baseline
