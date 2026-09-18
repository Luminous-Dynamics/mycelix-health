# Clinical Causality v1

Status: experimental / qualification required.

## Purpose

`mycelix-clinical-causality` separates three concepts that must not collapse:

1. an observed clinical event;
2. temporal association with an exposure;
3. causal interpretation of that association.

The core invariant is:

`observed after exposure != caused by exposure`

## Observed event

An `ObservedClinicalEventV1` contains one subject-consistent set of machine-actionable `ClinicalFact`s, an exact patient-subject binding evidence digest, onset/end timing, and recording time. Its digest uses the `ClinicalObservedEvent` domain.

The pure crate validates the facts and internal subject consistency. It does not independently prove that the opaque patient-binding digest actually binds those facts' external subject identity; that belongs to the adapter/runtime evidence boundary.

## Exposure

Medication v1 binds the event to an exact:

- `MedicationRequestArtifact`;
- qualified administration receipt;
- computable administration occurrence;
- patient-subject binding;
- administration interval.

## Temporal relationship

V1 derives one of:

- `EventBeforeExposure`;
- `OverlapsExposure`;
- `EventAfterExposure { latency_micros }`;
- `Indeterminate`.

An event that began before exposure with no known resolution time is `Indeterminate`, not automatically `EventBeforeExposure`.

Neither `EventBeforeExposure` nor `Indeterminate` can support a positive causal conclusion.

## Evidence factors

The v1 vocabulary includes temporal plausibility, objective confirmation, dechallenge, rechallenge, dose response, known mechanism, prior literature/cases, alternative etiology review, concomitant exposure review, baseline comparison, and aggregate signal evidence.

Evidence direction is preserved as support, challenge, mixed, no-finding, or unknown. Every state except `Unknown` requires at least one referenced evidence artifact. In particular, `NoFinding` is not an evidence-free assertion.

## Assessment

The internal conclusion vocabulary is deliberately small:

- `Indeterminate`;
- `TemporalAssociationOnly`;
- `EvidenceSuggestsRelationship`;
- `EvidenceSupportsRelationship`;
- `EvidenceAgainstRelationship`.

This is not claimed to be a universal international causality scale. External causality systems may be preserved as `system + version + label`; their label never automatically upgrades the internal conclusion.

A versioned policy controls required review kinds and minimum distinct non-temporal support kinds. Duplicate evidence within one evidence kind cannot manufacture independent support.

The strict default requires alternative-etiology and concomitant-exposure review before a positive conclusion.

## Regulatory interpretation

The crate intentionally does not equate its conclusion enum with a regulatory filing decision. A later jurisdiction/protocol-specific adapter may interpret `EvidenceSuggestsRelationship` or stronger against a policy equivalent to a `reasonable possibility` threshold, but that interpretation must become a separate domain-separated artifact.

## Non-claims

This tranche does not prove:

- physical administration occurred;
- the patient-binding evidence is institutionally trusted;
- literature or evaluator sources are institutionally trusted;
- a population-level drug effect from one case;
- an individual causal relationship from a population signal alone;
- regulatory reportability;
- assessor professional authority;
- clinical qualification.

## Qualification gates

Before production use:

- exact adapter/runtime verification of every referenced evidence artifact;
- assessor authority policy and freshness;
- trusted evidence-source admission;
- DHT/runtime assessment attestation;
- legacy adverse-event migration tests;
- regulatory interpretation tests per intended jurisdiction/protocol;
- exact-head formatting, Clippy, unit/property/fuzz tests;
- clinical/pharmacovigilance expert review.
