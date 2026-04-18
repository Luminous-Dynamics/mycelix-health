# Mycelix Health — Running System

**Status**: LIVE as of March 31, 2026

## Quick Start

```bash
# 1. Start conductor (if not running)
export PATH="$HOME/.cargo/bin:$PATH"
cd /srv/luminous-dynamics/mycelix-health
echo "" | hc sandbox --piped generate -a 9999 mycelix-health.happ --run=8888

# 2. Serve portal (if not running)
cd /srv/luminous-dynamics/mycelix-portal/dist
python3 -m http.server 8095 &

# 3. Open
# http://localhost:8095/index.html
```

## Installed Tools

| Tool | Version | Path |
|------|---------|------|
| holochain | 0.6.0 | ~/.cargo/bin/holochain |
| hc | 0.6.0 | ~/.cargo/bin/hc |
| lair-keystore | 0.6.3 | ~/.cargo/bin/lair-keystore |
| trunk | 0.21.x | ~/.cargo/bin/trunk |

## Artifacts

| Artifact | Size | Path |
|----------|------|------|
| DNA | 4.7MB | dna/health.dna |
| hApp | 4.7MB | mycelix-health.happ |
| Portal WASM | 396KB (release) | mycelix-portal/dist/ |
| WASM zomes | 12 files | target/wasm32-unknown-unknown/release/ |

## Ports

| Port | Service |
|------|---------|
| 8888 | Holochain app interface (WebSocket) |
| 33743 | Holochain admin interface |
| 8095 | Unified portal |
| 8094 | Health standalone portal |

## Rebuild Commands

```bash
# Rebuild all zomes
cargo build --release --target wasm32-unknown-unknown \
  -p patient_integrity -p provider_integrity -p records_integrity \
  -p prescriptions_integrity -p consent_integrity -p bridge_integrity \
  -p patient -p provider -p records -p prescriptions -p consent -p health_bridge

# Repack DNA
~/.cargo/bin/hc dna pack dna/ -o dna/health.dna

# Repack hApp
~/.cargo/bin/hc app pack . -o mycelix-health.happ

# Rebuild portal (release)
cd /srv/luminous-dynamics/mycelix-portal
~/.cargo/bin/trunk build --release

# Run tests
bash scripts/quick-test.sh
```

## What This System Does

- Patient-controlled XChaCha20-Poly1305 encryption
- HMAC-HKDF key derivation (RFC 5869)
- Consent management with 42 CFR Part 2 re-disclosure prevention
- SHA-256 chained audit trail
- Federated learning with Laplace DP noise + TrimmedMean Byzantine defense
- Emergency break-glass access with audit + patient notification
- Minors protection (guardian consent with sensitive category restrictions)
- Consciousness Orb portal with 4 phenotypes + domain-reactive WebGL shader
```
