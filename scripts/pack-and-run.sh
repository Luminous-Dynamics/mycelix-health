#!/usr/bin/env bash
# Pack the health DNA and start a conductor.
# Run from within `nix develop`:
#   cd /srv/luminous-dynamics/mycelix-health
#   nix develop
#   bash scripts/pack-and-run.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

echo "=== Step 1: Verify WASM zomes ==="
MISSING=0
for zome in patient_integrity provider_integrity records_integrity \
            prescriptions_integrity consent_integrity bridge_integrity \
            patient provider records prescriptions consent health_bridge; do
    WASM="target/wasm32-unknown-unknown/release/${zome}.wasm"
    if [ -f "$WASM" ]; then
        SIZE=$(du -h "$WASM" | cut -f1)
        echo "  ✓ ${zome}.wasm ($SIZE)"
    else
        echo "  ✗ ${zome}.wasm MISSING — run: cargo build --release --target wasm32-unknown-unknown -p $zome"
        MISSING=$((MISSING + 1))
    fi
done

if [ "$MISSING" -gt 0 ]; then
    echo ""
    echo "ERROR: $MISSING zome(s) missing. Build them first."
    exit 1
fi

echo ""
echo "=== Step 2: Pack DNA ==="
hc dna pack dna/ -o dna/health.dna
echo "  ✓ dna/health.dna"

echo ""
echo "=== Step 3: Pack hApp ==="
hc app pack . -o mycelix-health.happ
echo "  ✓ mycelix-health.happ"

echo ""
echo "=== Step 4: Start conductor ==="
echo "Starting Holochain sandbox on port 8888..."
echo "The portal at http://localhost:8095 will detect it automatically."
echo ""
echo "" | hc sandbox --piped generate -a 9999 mycelix-health.happ --run=8888
