# Archived Health hApp Zomes

**Date**: 2026-02-15
**Reason**: Scope reduction to MVP core (Phase 1: 7 zomes)

## What Was Archived

22 Tier 3 zomes moved here from `zomes/`:

| Zome | Domain | LOC (approx) |
|------|--------|--------------|
| advocate | Patient advocacy | ~800 |
| zkhealth | ZK health proofs | ~600 |
| twin | Digital twin | ~500 |
| dividends | Health dividends | ~400 |
| commons | Health commons | ~500 |
| immunity | Immunization tracking | ~600 |
| moment | Health moments | ~400 |
| sdoh | Social determinants | ~500 |
| mental_health | Mental health | ~600 |
| chronic_care | Chronic care mgmt | ~500 |
| pediatric | Pediatric care | ~500 |
| research_commons | Research sharing | ~400 |
| trial_matching | Clinical trial matching | ~500 |
| irb | IRB review board | ~400 |
| federated_learning | FL for health | ~600 |
| population_health | Population analytics | ~500 |
| ips | Intl Patient Summary | ~400 |
| i18n | Internationalization | ~300 |
| disaster_response | Health disaster resp | ~500 |
| verifiable_credentials | Health VCs | ~500 |
| mobile_support | Mobile health | ~400 |
| hdc_genetics | HDC genetics | ~500 |

## What Remains Active

**Tier 1 MVP (7 zomes)**: patient, provider, records, prescriptions, consent, bridge, shared

**Tier 2 Deferred (commented out in Cargo.toml)**: trials, insurance, fhir_mapping, fhir_bridge, cds, provider_directory, telehealth, nutrition + hdc crates

## How to Restore

To restore a zome:

1. Move the zome directory back: `mv _archive-2026-02-15/ZOME_NAME zomes/ZOME_NAME`
2. Add workspace members to `Cargo.toml`
3. Add zome entries to `dna/dna.yaml` and `dnas/health/workdir/dna.yaml`
4. Run `cargo check --target wasm32-unknown-unknown` to verify
