# Mycelix Health: Independent Audit Package

**For**: Security auditor, compliance consultant, legal reviewer
**System**: Decentralized patient health data sovereignty platform
**Stack**: Holochain 0.6.0 + Rust + Leptos WASM

## Documents Included

| Document | Purpose | Path |
|----------|---------|------|
| COMPLIANCE_STATUS.md | Regulatory gap analysis (HIPAA, 42 CFR Part 2, GDPR) | `/COMPLIANCE_STATUS.md` |
| CRYPTO_AUDIT_GUIDE.md | Cryptographic implementation review guide | `/CRYPTO_AUDIT_GUIDE.md` |
| CLINICAL_DEPLOYMENT_CHECKLIST.md | Pre-deployment verification | `/docs/CLINICAL_DEPLOYMENT_CHECKLIST.md` |
| PILOT_PROPOSAL.md | Hospital trial plan | `/docs/PILOT_PROPOSAL.md` |
| RUNNING.md | System operation guide | `/RUNNING.md` |
| This document | Audit package overview | `/docs/AUDIT_PACKAGE.md` |

## Audit Scope

### 1. Cryptographic Review (CRITICAL)
- **Patient encryption**: XChaCha20-Poly1305 (chacha20poly1305 v0.10)
- **Key derivation**: HMAC-HKDF (hmac v0.12 + sha2 v0.10)
- **Post-quantum**: ML-KEM-768 hybrid encryption (health-crypto crate)
- **Key wrapping**: 10K-iteration SHA-256 stretching + XOR (KNOWN WEAKNESS — see CRYPTO_AUDIT_GUIDE.md)
- **Seed phrase**: BIP-39 with full 2048-word English wordlist
- **Audit trail**: SHA-256 chained entries with content hashing

**Files**: `zomes/shared/src/lib.rs`, `crates/health-crypto/src/lib.rs`, `apps/leptos/src/crypto/`

### 2. Access Control Review
- **Consent enforcement**: `require_authorization()` in shared crate
- **42 CFR Part 2**: `check_redisclosure()`, `check_sensitive_category_consent()`
- **Emergency access**: Break-glass with `EmergencyAccess` + `AccessNotification`
- **Minors protection**: Guardian consent, sensitive category restrictions
- **Admin**: `require_admin_authorization()` with bootstrap mode

**Files**: `zomes/consent/coordinator/src/lib.rs`, `zomes/shared/src/lib.rs`

### 3. Penetration Testing Targets
- **localStorage XSS**: Key material stored in browser localStorage
- **Consent bypass**: Can an unauthorized agent call `decrypt_record()` directly?
- **Emergency abuse**: Can break-glass be triggered without legitimate emergency?
- **Nonce reuse**: Are XChaCha20 nonces ever repeated?
- **DHT snooping**: Can a malicious DHT participant read encrypted entries?

### 4. Regulatory Review
- **HIPAA**: Encryption at rest, audit controls, right to amend, minimum necessary
- **42 CFR Part 2**: Segregated substance abuse access, re-disclosure prevention
- **GDPR Article 17**: Cryptographic erasure via `request_crypto_erasure()`
- **21st Century Cures Act**: FHIR R4 interoperability (Epic sandbox verified)

### 5. Accessibility Review
- **WCAG 2.1 AA**: Color contrast, keyboard navigation, screen reader support
- **Portal**: Leptos WASM with WebGL (needs accessible fallback assessment)
- **Reduced motion**: `prefers-reduced-motion` kills all animation

## How to Run the System

```bash
# Prerequisites
~/.cargo/bin/holochain --version  # Should show 0.6.0
~/.cargo/bin/hc --version         # Should show 0.6.0

# Start conductor
cd /srv/luminous-dynamics/mycelix-health
echo "" | hc sandbox --piped generate -a 9999 mycelix-health.happ --run=8888

# Serve portal
cd /srv/luminous-dynamics/mycelix-portal/dist
python3 -m http.server 8095

# Run tests
cargo test --target x86_64-unknown-linux-gnu -p mycelix-health-fl -p mycelix-health-crypto -p mycelix-health-zkp
# Expected: 35 pass, 0 fail

# Run benchmarks
bash scripts/benchmark-health.sh
```

## Known Issues (Pre-Disclosed)

1. **Key wrapping uses XOR, not AES-KW** — acknowledged, planned fix
2. **localStorage is XSS-vulnerable** — mitigated by CSP headers in production
3. **SplitMix64 PRNG in DP noise** — not cryptographic, used only for FL noise seeding
4. **No formal verification** of any cryptographic construction
5. **Sweettest e2e tests are stubs** — document the call pattern but don't execute against conductor yet

## Contact

Tristan Stoltz — Luminous Dynamics
tristan.stoltz@evolvingresonantcocreationism.com
Richardson, TX
