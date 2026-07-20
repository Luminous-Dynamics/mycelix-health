#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EVIDENCE_DIR="${COUNTERSIGNING_EVIDENCE_DIR:-$ROOT/target/countersigning-evidence}"

python3 "$ROOT/scripts/check-countersigning-matrix.py"

required=(hc holochain lair-keystore)
missing=()
for binary in "${required[@]}"; do
  command -v "$binary" >/dev/null 2>&1 || missing+=("$binary")
done

if [[ ! -f "$ROOT/mycelix-health.happ" ]]; then
  missing+=("mycelix-health.happ")
fi

if (( ${#missing[@]} > 0 )); then
  printf 'native countersigning harness NOT RUN: missing %s\n' "${missing[*]}" >&2
  printf 'No success evidence was emitted. Build the pinned hApp and provide Holochain 0.6.1 tools.\n' >&2
  exit 2
fi

mkdir -p "$EVIDENCE_DIR"
printf 'The repository contains the conductor topology and scenario contract, but the live launcher must be supplied by the canonical Nix/Holochain environment.\n' >&2
printf 'Refusing to create a passing evidence bundle from this generic shell environment.\n' >&2
exit 3
