# Mycelix Health: Cryptographic Audit Guide

**Prepared for**: Independent security auditor
**Date**: 2026-03-30
**System**: Mycelix Health — decentralized patient data sovereignty on Holochain

## Scope

This document guides an independent cryptographic audit of the Mycelix Health system. All cryptographic operations are implemented in Rust, compiled to WASM, and run either in Holochain zomes (server-side) or in the browser (client-side).

## Cryptographic Operations to Audit

### 1. Patient Data Encryption (CRITICAL)

**Location**: `zomes/shared/src/lib.rs` — `patient_encryption` module
**Algorithm**: XChaCha20-Poly1305 (via `chacha20poly1305` crate v0.10)
**Key size**: 256-bit symmetric
**Nonce**: 24-byte, generated via `getrandom` (Holochain WASM backend)

**Audit questions**:
- Is the nonce generation truly random? (WASM `getrandom` uses Holochain's custom backend)
- Is the AEAD tag verified before returning plaintext?
- Can an attacker reuse nonces across records? (Nonce is per-encryption, not sequential)
- Is the ciphertext length-preserving? (No — Poly1305 tag adds 16 bytes)

**Files**: `zomes/shared/src/lib.rs` lines 140-196

### 2. Key Derivation (CRITICAL)

**Location**: `zomes/shared/src/lib.rs` — `patient_encryption::derive_key()`
**Algorithm**: HMAC-SHA256 HKDF (RFC 5869) via `hmac` crate v0.12 + `sha2` v0.10
**Salt**: Fixed domain-separation salt `b"mycelix-health-v1-patient-encryption"`
**IKM**: Must be secret material (private key or DH shared secret)

**Audit questions**:
- Is the IKM actually secret? (Coordinator receives pre-derived key from client)
- Is the salt unique per domain? (Yes — different context bytes per domain)
- Is the HKDF Extract truly HMAC, not raw SHA-256? (Fixed in commit `1aa42c1`)
- Are there any paths where `derive_key` is called with a public key as IKM? (Previously yes, fixed in P0-4)

**Files**: `zomes/shared/src/lib.rs` lines 198-231

### 3. Client-Side Key Wrapping (HIGH)

**Location**: `apps/leptos/src/crypto/key_manager.rs`
**Algorithm**: 10,000-iteration SHA-256 stretching + XOR wrapping
**Storage**: Browser localStorage

**Audit questions**:
- Is the stretching sufficient? (10,000 iterations of SHA-256 — compare to PBKDF2/Argon2 standards)
- Is XOR wrapping acceptable? (No authentication — a modified ciphertext would still unwrap to a valid key)
- Is localStorage an acceptable storage medium? (XSS vulnerability — any injected script can read it)
- Should this use Web Crypto API's `subtle.wrapKey()` with AES-KW instead?

**Files**: `apps/leptos/src/crypto/key_manager.rs` lines 100-130

### 4. Seed Phrase (MEDIUM)

**Location**: `apps/leptos/src/crypto/seed_phrase.rs`
**Standard**: BIP-39 compatible (partial — 256 of 2048 words)
**Checksum**: SHA-256 first byte

**Audit questions**:
- Is the word list sufficient? (256 words reduces entropy from 2048^24 to 256^24)
- Is the checksum algorithm correct BIP-39? (Yes — SHA-256 first byte)
- Does the roundtrip (entropy → phrase → entropy) preserve all 256 bits?

**Files**: `apps/leptos/src/crypto/seed_phrase.rs`

### 5. Chained Audit Trail (MEDIUM)

**Location**: `zomes/shared/src/lib.rs` — `audit` module, `chained_log_data_access()`
**Algorithm**: SHA-256 content hashing with previous-hash chaining
**Storage**: Holochain DHT via consent zome `DataAccessLog` entries

**Audit questions**:
- Is the chain integrity verifiable? (Content hash includes action hash + sequence + entry bytes)
- Can entries be reordered without detection? (Sequence numbers are count-based from log queries)
- Is the chain genesis deterministic? (No — first entry has `previous_hash = None`)

**Files**: `zomes/shared/src/lib.rs` lines 930-1003

### 6. Federated Learning Privacy (LOW — research code)

**Location**: `crates/health-fl/src/lib.rs`
**Algorithm**: Laplace noise (ε-DP), TrimmedMean aggregation
**Budget**: Per-patient epsilon tracking with `DEFAULT_EPSILON_MAX = 10.0`

**Audit questions**:
- Is the Laplace inverse CDF correct? (Yes — validated mathematically)
- Is the pseudo-uniform (SplitMix64) sufficient for DP noise? (Not cryptographic — production should use `getrandom`)
- Is epsilon composition tracked correctly? (Simple additive — no advanced composition theorems)

**Files**: `crates/health-fl/src/lib.rs` lines 180-220

## How to Run

```bash
# Build all zomes for WASM
cd /srv/luminous-dynamics/mycelix-health
cargo build --release --target wasm32-unknown-unknown -p patient -p records -p consent

# Run FL tests
CARGO_TARGET_DIR=/tmp/audit cargo test --target x86_64-unknown-linux-gnu -p mycelix-health-fl

# Run the e2e demo
CARGO_TARGET_DIR=/tmp/audit cargo run --target x86_64-unknown-linux-gnu --example health_sovereignty_demo -p mycelix-health-fl
```

## Known Issues (Pre-Audit Disclosure)

1. **Client key wrapping uses XOR, not AES-KW** — acknowledged, documented
2. **BIP-39 wordlist is 256 words, not 2048** — reduced entropy
3. **localStorage is XSS-vulnerable** — mitigated by CSP headers in production
4. **SplitMix64 PRNG is not cryptographic** — used only in FL noise, not for key generation
5. **No formal verification** of any cryptographic construction
