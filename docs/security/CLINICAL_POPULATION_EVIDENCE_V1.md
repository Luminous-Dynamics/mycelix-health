# Clinical population evidence v1

## Purpose

Establish non-interchangeable population-evidence classes for pharmacovigilance and clinical research without silently upgrading weak evidence into incidence, risk, or causal-effect claims.

## Evidence classes

### Safety signal candidate

`SafetySignalCandidateV1` is hypothesis-generating. It may be built from spontaneous reports, disproportionality, designated medical events, temporal clustering, literature, or external regulatory signals.

It deliberately has no denominator, incidence, risk, or causal-effect field.

Its review status may be `Candidate`, `ValidatedForReview`, `ConfirmedForFurtherAssessment`, `UnderAssessment`, or `Refuted`. `ConfirmedForFurtherAssessment` is still not a causal conclusion.

### Denominator-based association

`DenominatorBasedAssociationV1` requires explicit exposed and comparator cohort summaries. Each cohort binds population, exposure, outcome, source dataset, index-time definition, follow-up definition, deduplication policy, optional strata definition, and a nonzero denominator.

V1 supports person-based binary-outcome denominators and aggregate person-time denominators. Risk/odds measures cannot be paired with person-time denominators; incidence-rate ratio cannot be paired with person-at-risk denominators.

This artifact represents statistical association, not causal effect.

### Causal effect estimate

`CausalEffectEstimateV1` requires an exact population-association digest plus:

- explicit causal study design;
- design specification evidence;
- confounder strategy evidence;
- diagnostics evidence;
- exact estimator-result evidence;
- policy-required sensitivity analysis;
- policy-required negative-control review.

The policy constrains allowed study designs. This remains a causal **estimate under a design**, not causal truth.

### Replication

`PopulationReplicationV1` requires at least two distinct finding digests of the same evidence class plus explicit independence and replication-method evidence.

Duplicate findings are rejected. Mixed signal/association/effect classes are rejected.

## Domain separation

Population signal policies, signal candidates, cohort summaries, association policies, associations, causal-effect policies, causal-effect estimates, and replication artifacts all use distinct clinical-integrity digest domains.

## Stored-digest non-claim

A `StoredDigest` is only an identity claim. Deserialization does not prove that the referenced source dataset, statistical method, association, diagnostic, or estimator result was actually recomputed or trusted.

Higher-assurance qualification must reverify exact artifact bytes and establish source/method authority before presenting a result as qualified clinical or research evidence.

## Regulatory/epistemic alignment

The design follows the important distinction used in pharmacovigilance practice: signal detection can identify unusual associations requiring investigation, but a signal or disproportionality result does not by itself establish causality or an occurrence rate. Denominator-based estimation and causal-effect estimation are separate evidence problems.

## Privacy

These v1 artifacts are protected/internal research artifacts. This tranche does not publish cohort counts, case counts, strata, or estimator results to the DHT.

A separate release layer must define privacy thresholds, suppression, aggregation, query budgeting, and any differential-privacy policy before population outputs can be published broadly.

## Non-goals

V1 is not:

- a universal epidemiology statistics engine;
- a replacement for validated statistical software;
- a causal discovery oracle;
- a regulatory signal-management workflow;
- proof that a source dataset is complete or unbiased;
- permission to expose small-cell cohort data.

## Next qualification layers

1. source-dataset and statistical-method trust receipts;
2. privacy-preserving aggregate release policy;
3. exact estimator/reproducibility capsules;
4. independent replication qualification;
5. optional DNA-rooted publication of privacy-safe aggregate findings.
