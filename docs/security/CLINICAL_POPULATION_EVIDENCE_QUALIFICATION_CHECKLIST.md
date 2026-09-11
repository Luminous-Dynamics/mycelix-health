# Clinical population evidence v1 — qualification checklist

This checklist is test/review design, not runtime evidence.

## Build / lint

- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] workspace build/test passes on the exact PR head
- [ ] population-evidence tests pass on the exact PR head

## Safety signal boundary

- [ ] signal candidate rejects zero/insufficient case counts
- [ ] distinct-subject count cannot be zero or exceed case count
- [ ] policy-required distinct-subject evidence cannot be omitted
- [ ] signal artifact has no denominator/risk/effect field
- [ ] `ConfirmedForFurtherAssessment` is documented/tested as non-causal
- [ ] signal policy and candidate use distinct digest domains

## Denominator association boundary

- [ ] cohort denominator cannot be zero
- [ ] binary outcomes cannot exceed observed persons
- [ ] exposed/comparator cohorts cannot be identical
- [ ] policy can require identical outcome definitions
- [ ] policy can require identical follow-up definitions
- [ ] policy can require compatible denominator kinds
- [ ] risk/odds measures reject person-time denominators
- [ ] incidence-rate ratio rejects person-at-risk denominators
- [ ] association artifact is never exposed as a causal-effect estimate

## Causal effect boundary

- [ ] association digest must use `ClinicalPopulationAssociation`
- [ ] design must be allowed by the bound causal-effect policy
- [ ] design specification evidence is required
- [ ] confounder strategy evidence is required
- [ ] diagnostics evidence is required
- [ ] exact estimator-result evidence is required
- [ ] policy-required sensitivity analysis fails closed when absent
- [ ] policy-required negative-control review fails closed when absent
- [ ] no automatic signal->association or association->effect conversion API exists

## Replication boundary

- [ ] fewer than two findings fails closed
- [ ] duplicate finding digests fail closed
- [ ] mixed finding classes fail closed
- [ ] independence evidence is required
- [ ] replication-method evidence is required
- [ ] replication conclusion does not automatically upgrade evidence class

## Integrity / provenance

- [ ] all population artifact classes use distinct digest domains
- [ ] stored digests are treated as claims until exact artifact re-verification
- [ ] higher-assurance qualification requires source/method trust evidence
- [ ] estimator/reproducibility capsule design is reviewed before clinical/research promotion

## Privacy

- [ ] no population counts/strata/results are published to the DHT by this tranche
- [ ] small-cell suppression is specified before any aggregate release
- [ ] query composition/budgeting is addressed before interactive aggregate release
- [ ] differential privacy, if used, is policy-bound and not silently substituted for raw statistics

## Promotion rule

Do not promote beyond experimental until every applicable item has executable exact-head evidence. Source review, a mergeable PR, or queued CI is not qualification evidence.
