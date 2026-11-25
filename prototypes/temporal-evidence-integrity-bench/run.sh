#!/usr/bin/env sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
MANIFEST="$ROOT/prototypes/temporal-evidence-integrity-bench/Cargo.toml"
FIXTURES="$ROOT/prototypes/temporal-evidence-integrity-bench/fixtures"
HOST="$(rustc -vV | sed -n 's/^host: //p')"
sha256sum "$FIXTURES/v1-task-inputs.tsv" "$FIXTURES/v1-events.tsv" "$FIXTURES/v1-cases.tsv" "$FIXTURES/v1-answer-key-events.tsv"
cargo fmt --manifest-path "$ROOT/prototypes/temporal-evidence-agent-contract/Cargo.toml" -- --check
cargo fmt --locked --manifest-path "$MANIFEST" -- --check
CONTRACT="$ROOT/prototypes/temporal-evidence-agent-contract/Cargo.toml"
cargo clippy --locked --manifest-path "$CONTRACT" --target "$HOST" --all-targets -- -D warnings
cargo test --locked --manifest-path "$CONTRACT" --target "$HOST"
TREE="$(cargo tree --locked --manifest-path "$CONTRACT" --target "$HOST")"
printf '%s\n' "$TREE"
if printf '%s\n' "$TREE" | grep -Fq 'mycelix-temporal-evidence-integrity-bench'; then
  echo "Candidate contract unexpectedly depends on evaluator package" >&2
  exit 1
fi
cargo clippy --locked --manifest-path "$MANIFEST" --target "$HOST" --all-targets -- -D warnings
cargo test --locked --manifest-path "$MANIFEST" --target "$HOST"
cargo run --locked --manifest-path "$MANIFEST" --target "$HOST" --bin naive_baseline
cargo run --locked --manifest-path "$MANIFEST" --target "$HOST" --bin revision_baseline
