# CDS Seed Data

Reference drug interaction database for the Clinical Decision Support zome.

## Contents

- `drug_interactions.json` — 30 clinically significant drug-drug interactions
  - Covers: anticoagulants, antibiotics, CNS drugs, statins, cardiac drugs
  - Sources: FDA Drug Safety Communications, DrugBank, ACC/AHA Guidelines, BMJ
  - Severity levels: 4 Contraindicated, 22 Major, 4 Moderate

- `drug_allergy_interactions.json` — 12 drug-allergy cross-reactivity records
  - Covers: beta-lactams, fluoroquinolones, NSAIDs, sulfonamides, opioids, contrast media
  - Sources: AAAAI, FDA, ACR Manual on Contrast Media

## Loading

The data can be loaded via the CDS zome's `create_drug_interaction` and
`create_drug_allergy_interaction` extern functions. Each entry maps directly
to the zome's entry types.

### Via hc sandbox (development)

```bash
# Load drug interactions
for entry in $(jq -c '.[]' drug_interactions.json); do
  hc sandbox call --running=33800 call-zome \
    --cell-id mycelix-health \
    --zome cds \
    --fn create_drug_interaction \
    --payload "$entry"
done

# Load allergy interactions
for entry in $(jq -c '.[]' drug_allergy_interactions.json); do
  hc sandbox call --running=33800 call-zome \
    --cell-id mycelix-health \
    --zome cds \
    --fn create_drug_allergy_interaction \
    --payload "$entry"
done
```

### Via Rust test harness

See the integration test at `tests/src/cds_seed_test.rs` (if present).

## Data Sources

All interactions sourced from publicly available clinical references:
- FDA Drug Safety Communications (fda.gov)
- DrugBank Open Data (drugbank.com)
- ACC/AHA Clinical Practice Guidelines
- AAAAI Practice Parameters
- ACR Manual on Contrast Media
- BMJ Clinical Evidence
- PubMed (specific PMIDs cited per interaction)

## Maintenance

This is a seed dataset for development and testing. Production deployments
should integrate with live drug interaction databases (e.g., First Databank,
Medispan, or DrugBank API) for comprehensive coverage.
