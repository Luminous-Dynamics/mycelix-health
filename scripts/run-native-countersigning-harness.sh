#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EVIDENCE_DIR="${COUNTERSIGNING_EVIDENCE_DIR:-$ROOT/target/countersigning-evidence}"

python3 "$ROOT/scripts/check-countersigning-matrix.py"

if [[ -n "${COUNTERSIGNING_LIVE_CONFIG:-}" ]]; then
  command -v node >/dev/null 2>&1 || { echo "node is required for externally managed live countersigning" >&2; exit 2; }
  command -v npm >/dev/null 2>&1 || { echo "npm is required for externally managed live countersigning" >&2; exit 2; }
  [[ -f "$COUNTERSIGNING_LIVE_CONFIG" ]] || { echo "COUNTERSIGNING_LIVE_CONFIG does not exist" >&2; exit 2; }
  (cd "$ROOT/sdk" && npm run build --silent)
  node "$ROOT/scripts/run-live-countersigning.mjs" "$COUNTERSIGNING_LIVE_CONFIG" "$EVIDENCE_DIR"
  python3 "$ROOT/scripts/package-countersigning-evidence.py" "$EVIDENCE_DIR"
  printf 'Verified live countersigning evidence written to %s\n' "$EVIDENCE_DIR"
  exit 0
fi

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
