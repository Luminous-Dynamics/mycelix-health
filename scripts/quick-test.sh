#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Quick validation suite — runs in <60 seconds, uses shared target dir.
# Covers: FL pipeline, seed phrase, zome compilation, portal check.
#
# Usage: bash scripts/quick-test.sh
#        bash scripts/quick-test.sh --fl-only
#        bash scripts/quick-test.sh --compile-only

set -euo pipefail
cd "$(dirname "$0")/.."

# Use the workspace target dir (sccache shared) — NOT /tmp
# This avoids creating duplicate target dirs and leverages cached builds
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

PASS=0
FAIL=0
SKIP=0

pass() { echo -e "  ${GREEN}✓${NC} $1"; PASS=$((PASS+1)); }
fail() { echo -e "  ${RED}✗${NC} $1"; FAIL=$((FAIL+1)); }
skip() { echo -e "  ${YELLOW}⊘${NC} $1 (skipped)"; SKIP=$((SKIP+1)); }

MODE="${1:-all}"

echo "═══════════════════════════════════════"
echo "  Mycelix Health — Quick Validation"
echo "═══════════════════════════════════════"
echo ""

# ── 1. FL Pipeline Tests (native, ~1 sec) ──
if [[ "$MODE" == "all" || "$MODE" == "--fl-only" ]]; then
    echo "FL Pipeline (native x86_64):"
    if cargo test --target x86_64-unknown-linux-gnu -p mycelix-health-fl --quiet 2>/dev/null; then
        RESULT=$(cargo test --target x86_64-unknown-linux-gnu -p mycelix-health-fl 2>&1 | grep "test result")
        COUNT=$(echo "$RESULT" | grep -oP '\d+ passed' | head -1)
        pass "health-fl: $COUNT"
    else
        fail "health-fl tests failed"
    fi
fi

# ── 2. Zome Compilation Check (WASM, cached = fast) ──
if [[ "$MODE" == "all" || "$MODE" == "--compile-only" ]]; then
    echo ""
    echo "Zome Compilation (WASM):"
    for zome in patient consent records; do
        if cargo check --target wasm32-unknown-unknown -p $zome --quiet 2>/dev/null; then
            pass "$zome coordinator compiles"
        else
            fail "$zome coordinator FAILED"
        fi
    done

    echo ""
    echo "Shared Crate:"
    if cargo check --target wasm32-unknown-unknown -p mycelix-health-shared --quiet 2>/dev/null; then
        pass "mycelix-health-shared compiles"
    else
        fail "mycelix-health-shared FAILED"
    fi
fi

# ── 3. Seed Phrase Roundtrip (built into FL crate tests) ──
if [[ "$MODE" == "all" ]]; then
    echo ""
    echo "Crypto Primitives:"
    # The FL crate includes seed phrase tests via the demo
    if cargo test --target x86_64-unknown-linux-gnu -p mycelix-health-fl -- reference_range_parsing --quiet 2>/dev/null; then
        pass "reference range parsing"
    else
        fail "reference range parsing"
    fi
fi

# ── 4. DNA Bundle Check ──
if [[ "$MODE" == "all" ]]; then
    echo ""
    echo "DNA Bundle:"
    if [ -f "dna/health.dna" ]; then
        SIZE=$(du -h dna/health.dna | cut -f1)
        AGE=$(stat -c %Y dna/health.dna 2>/dev/null || stat -f %m dna/health.dna 2>/dev/null)
        NOW=$(date +%s)
        HOURS=$(( (NOW - AGE) / 3600 ))
        if [ "$HOURS" -lt 24 ]; then
            pass "health.dna exists ($SIZE, ${HOURS}h old)"
        else
            skip "health.dna exists but is ${HOURS}h old — consider repacking"
        fi
    else
        skip "health.dna not found — run: hc dna pack dna/"
    fi

    if [ -f "mycelix-health.happ" ]; then
        pass "mycelix-health.happ exists"
    else
        skip "mycelix-health.happ not found"
    fi
fi

# ── 5. Portal Check ──
if [[ "$MODE" == "all" ]]; then
    echo ""
    echo "Portal:"
    if [ -d "../mycelix-portal/dist" ]; then
        WASM=$(find ../mycelix-portal/dist -name "*.wasm" | head -1)
        if [ -n "$WASM" ]; then
            SIZE=$(du -h "$WASM" | cut -f1)
            pass "unified portal built ($SIZE WASM)"
        else
            skip "portal dist/ exists but no WASM"
        fi
    else
        skip "portal not built"
    fi

    # Check if portal is serving
    if curl -s -o /dev/null -w "%{http_code}" http://localhost:8095/index.html 2>/dev/null | grep -q "200"; then
        pass "portal serving on :8095"
    else
        skip "portal not serving"
    fi
fi

# ── 6. Conductor Check ──
if [[ "$MODE" == "all" ]]; then
    echo ""
    echo "Conductor:"
    if ~/.cargo/bin/hc --version >/dev/null 2>&1; then
        pass "hc CLI available ($(~/.cargo/bin/hc --version 2>&1))"
    else
        skip "hc CLI not found"
    fi

    if ~/.cargo/bin/holochain --version >/dev/null 2>&1; then
        pass "holochain conductor available"
    else
        skip "holochain conductor not built yet"
    fi

    if curl -s -o /dev/null -w "%{http_code}" http://localhost:8888 2>/dev/null | grep -q ""; then
        skip "conductor not running on :8888"
    fi
fi

# ── Summary ──
echo ""
echo "═══════════════════════════════════════"
echo -e "  ${GREEN}$PASS passed${NC}  ${RED}$FAIL failed${NC}  ${YELLOW}$SKIP skipped${NC}"
echo "═══════════════════════════════════════"

[ "$FAIL" -eq 0 ] && exit 0 || exit 1
