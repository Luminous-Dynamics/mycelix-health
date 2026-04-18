# mycelix-health — Public Contract

**Status**: Active (Q2 Leptos binding). Submodule pinned via parent monorepo at `/srv/luminous-dynamics/mycelix-health/`.
**Holochain**: 0.6.0 (hdk 0.6, hdi 0.7).
**Crypto**: PQC via `crates/health-crypto`; ZKP via `crates/health-zkp` (Winterfell STARK backend, DASTARK pipeline).

---

## Zome inventory

17 active zomes + 1 utility crate (`shared`). 338 total `hdk_extern` functions.

| Zome | Role | hdk_externs | Primary entry points |
|---|---|---|---|
| **bridge** | Cross-cluster facade | 15 | `submit_health_attestation`, `verify_health_credential`, `query_health_record_access` |
| **cds** | Clinical Decision Support | 18 | `check_drug_interaction`, `check_allergy_conflict`, `suggest_dose_adjustment` |
| **consent** | Patient consent graph | 63 | `grant_consent`, `revoke_consent`, `get_active_consents`, `query_consent_scope` |
| **credentials** | Practitioner credentials | 9 | `issue_practitioner_credential`, `verify_credential`, `revoke_credential` |
| **dividends** | Health-pool payouts | 31 | `record_contribution`, `calculate_dividend`, `claim_dividend` |
| **fhir_bridge** | FHIR resource I/O | 3 | `import_fhir_resource`, `export_fhir_bundle`, `query_fhir_by_code` |
| **fhir_mapping** | FHIR ↔ internal shape | 16 | `map_patient_to_fhir`, `map_observation_to_fhir`, `resolve_code_system` |
| **insurance** | Coverage + claims | 13 | `submit_claim`, `verify_coverage`, `adjudicate_claim` |
| **mental_health** | Behavioral-health records | 31 | `log_session`, `query_treatment_plan`, `record_assessment` |
| **nutrition** | Dietary tracking | 18 | `log_meal`, `query_nutrient_profile`, `recommend_intake` |
| **patient** | Patient records root | 14 | `create_patient_record`, `update_demographics`, `query_patient_summary` |
| **prescriptions** | e-Prescribe + CDS gate | 16 | `write_prescription`, `verify_with_cds`, `dispense_prescription` |
| **provider** | Provider profiles | 13 | `register_provider`, `update_provider_credentials`, `query_provider_affiliations` |
| **provider_directory** | Searchable directory | 11 | `search_providers_by_specialty`, `get_provider_availability` |
| **records** | Clinical record store | 31 | `submit_record`, `query_records_by_type`, `audit_access_history` |
| **telehealth** | Remote encounters | 14 | `schedule_encounter`, `start_session`, `record_notes` |
| **trials** | Clinical trial enrollment | 12 | `enroll_participant`, `record_trial_event`, `query_participant_history` |

---

## Cross-cluster boundaries

When running in the unified Mycelix hApp, health zomes can dispatch to:

| Target role | Via | Purpose |
|---|---|---|
| `identity` | `identity_bridge`, `did_registry` | Provider identity verification; patient DID resolution |
| `personal` | `personal_bridge`, `health_vault` | Personal-vault storage of sensitive records |
| `finance` (future) | via bridge | Insurance adjudication settlement |

Routes declared in `crates/mycelix-bridge-common/src/routing_registry.rs`
(`HEALTH_TO_PERSONAL`, `HEALTH_TO_IDENTITY`).

---

## Crypto invariants

1. **All PHI fields encrypted at rest** via `health-crypto` before DHT submission.
2. **Consent-scoped access**: every record access requires a valid consent entry returning `true` from `consent.query_consent_scope`.
3. **Attestation freshness**: health-attestations use the shared `ConsciousnessAttestation.validate_with_freshness` path (`crates/mycelix-bridge-common/src/consciousness_zkp.rs`) for replay prevention.
4. **CDS safety checks are best-effort, not authoritative** — RISC-0 compute limits prevent exhaustive drug-interaction search; downstream consumers must treat `check_drug_interaction` negative as "no known conflict found", not "safe".

---

## Build + test

```bash
cd /srv/luminous-dynamics/mycelix-health

# Workspace check
cargo check --workspace              # ~9s warm

# Test suite (500+ tests across 30 files)
cargo test --workspace

# Build WASM zomes for conductor
cargo build --workspace --target wasm32-unknown-unknown --release

# Pack DNA
hc dna pack dna/ -o dna/health.dna

# Pack hApp bundle
hc app pack . -o mycelix-health.happ
```

---

## Archived scope (reference)

`_archive-2026-02-15/` holds 22 zomes parked for later phases (not deleted):
advocate, chronic_care, commons, disaster_response, dividends (v1 pre-refactor),
federated_learning, hdc_genetics, i18n, and others. See its README.md.
**Do not delete this directory.** These zomes are strategic options we may
restore when product demand emerges (e.g., federated-learning across health data,
genomic consent flows).

---

## Leptos frontend status (Q2 work in progress)

`apps/leptos/` currently runs against mock clients. Week 2 of the revival plan:
replace `mock_consent_client`, `mock_records_client`, `mock_access_event_client`
with real `HolochainProviderAuto` calls to the zomes listed above. Week 3:
wire `patient → consent → records` and `patient → identity` integration paths.

See `MYCELIX_ARCHITECTURE_PLAN.md` / `can-you-explore-deeper-dreamy-pancake.md`
(Part B.2) in the parent monorepo for the full revival roadmap.
