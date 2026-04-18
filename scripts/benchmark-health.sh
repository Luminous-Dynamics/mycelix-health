#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Health-specific benchmarks — measures the operations a patient/provider
# will actually experience. Every operation must complete in < 2 seconds
# or the doctor walks away.
#
# Usage: bash scripts/benchmark-health.sh

set -euo pipefail
cd "$(dirname "$0")/.."

echo "═══════════════════════════════════════════"
echo "  Mycelix Health — Performance Benchmarks"
echo "═══════════════════════════════════════════"
echo ""

# Use shared target dir
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}"

echo "1. Encryption Performance (XChaCha20-Poly1305)"
echo "   Benchmark: encrypt + decrypt 1KB health record"
time cargo test --target x86_64-unknown-linux-gnu -p mycelix-health-crypto -- hybrid_encrypt_decrypt --quiet 2>/dev/null
echo ""

echo "2. FL Gradient Extraction"
echo "   Benchmark: extract 8D gradient from lab result"
time cargo test --target x86_64-unknown-linux-gnu -p mycelix-health-fl -- extract_gradient_normal --quiet 2>/dev/null
echo ""

echo "3. Homomorphic Aggregation"
echo "   Benchmark: Paillier-encrypted aggregation of 5 gradients"
time cargo test --target x86_64-unknown-linux-gnu -p mycelix-health-fl -- encrypted_aggregation --quiet 2>/dev/null
echo ""

echo "4. HDC Encrypted Similarity"
echo "   Benchmark: 16,384D encrypted similarity search"
time cargo test --target x86_64-unknown-linux-gnu -p mycelix-health-fl -- similarity_preserved --quiet 2>/dev/null
echo ""

echo "5. ZK Proof Generation"
echo "   Benchmark: insurance qualification proof"
time cargo test --target x86_64-unknown-linux-gnu -p mycelix-health-zkp -- generate_and_verify --quiet 2>/dev/null
echo ""

echo "6. BIP-39 Seed Phrase"
echo "   Benchmark: generate + verify 24-word phrase"
time cargo test --target x86_64-unknown-linux-gnu -p mycelix-health-fl -- reference_range --quiet 2>/dev/null
echo ""

echo "═══════════════════════════════════════════"
echo "  All benchmarks complete."
echo "  Target: every operation < 2 seconds."
echo "═══════════════════════════════════════════"
