#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST="$ROOT/prototypes/audit-causal-dag-bench/Cargo.toml"
HOST_TARGET="$(rustc -vV | awk '/^host:/{print $2}')"

cargo fmt --manifest-path "$MANIFEST" -- --check
cargo clippy --manifest-path "$MANIFEST" --all-targets --locked --target "$HOST_TARGET" -- -D warnings
cargo test --manifest-path "$MANIFEST" --locked --target "$HOST_TARGET"
