# Health DNA Build Instructions

## Current State (2026-02-07)

**✅ DNA Bundle Complete: `dna/health.dna` (8.0MB)**

The DNA manifest (`dna/dna.yaml`) has been updated for Holochain 0.6 compatibility:
- ✅ `manifest_version: "0"`
- ✅ `path:` instead of `bundled:`
- ✅ `origin_time` removed

All zomes (11 integrity + 11 coordinator) are compiled and bundled.

## Rebuilding

### Prerequisites

Use the mycelix-workspace environment (has working Holochain 0.6.0):

```bash
cd /srv/luminous-dynamics/mycelix-workspace
nix develop
```

### Build All Zomes

```bash
cd /srv/luminous-dynamics/mycelix-health
cargo build --release --target wasm32-unknown-unknown
```

### Pack DNA Bundle

```bash
cd /srv/luminous-dynamics/mycelix-workspace
nix develop --command bash -c "cd /srv/luminous-dynamics/mycelix-health && hc dna pack dna/"
```

## Included Zomes (22 total)

### Integrity Zomes (11)
- patient_integrity, provider_integrity, records_integrity
- prescriptions_integrity, consent_integrity, trials_integrity
- insurance_integrity, bridge_integrity, commons_integrity
- fhir_mapping_integrity, fhir_bridge_integrity

### Coordinator Zomes (11)
- patient, provider, records, prescriptions, consent
- trials, insurance, bridge (health_bridge), commons
- fhir_mapping, fhir_bridge

## Testing

```bash
cd /srv/luminous-dynamics/mycelix-workspace
nix develop --command bash -c "cd /srv/luminous-dynamics/mycelix-health && cargo test"
```

## Flake Issue

The mycelix-health `flake.nix` references Holochain 0.3.6 which has broken Nix dependencies.
Use mycelix-workspace environment until the flake is updated.
