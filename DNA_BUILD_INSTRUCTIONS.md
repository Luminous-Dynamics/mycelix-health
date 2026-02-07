# Health DNA Build Instructions

## Current State (2026-02-07)

The DNA manifest (`dna/dna.yaml`) has been updated for Holochain 0.6 compatibility:
- ✅ `manifest_version: "0"`
- ✅ `path:` instead of `bundled:`
- ✅ `origin_time` removed

Currently using a **minimal manifest** with 11 integrity zomes that have compiled WASM.

## Building Coordinator Zomes

The coordinator zomes need to be compiled before the DNA can include them.

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

### Build Specific Coordinators

```bash
cargo build --release --target wasm32-unknown-unknown \
  -p patient -p provider -p records -p consent \
  -p prescriptions -p trials -p insurance -p bridge
```

## After Building

1. Update `dna/dna.yaml` to include coordinator zomes
2. Repack the DNA: `hc dna pack dna/`
3. Test with sweettest

## Existing WASM Files (27)

All integrity zomes are compiled. Coordinator zomes pending:
- patient, provider, records, consent
- prescriptions, trials, insurance, bridge
- advocate, zkhealth, twin, dividends
- commons, immunity, moment
- fhir_mapping, fhir_bridge

## Flake Issue

The mycelix-health `flake.nix` references Holochain 0.3.6 which has broken Nix dependencies.
Use mycelix-workspace environment until the flake is updated.
