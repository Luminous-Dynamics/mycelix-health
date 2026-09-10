# Clinical Causality v1 — evidence-first plan

Status: experimental design contract. This document does not establish clinical or regulatory qualification.

## Problem

The legacy trial `AdverseEvent` stores one causality enum directly on the event. Current integrity validation checks event ID/term and seriousness shape, but does not prove the evidence lineage behind that causal conclusion.

The high-assurance path must preserve the distinction between an adverse event and an adverse reaction / suspected causal relationship.

## Core invariant

`observed after exposure != caused by exposure`

An event is valid evidence even when causality is unknown. Stronger causal interpretation must be a separate evidence-bearing artifact.

## V1 evidence graph

A causal assessment binds:

- exact observed-event identity;
- exact exposure association;
- exact patient/subject binding evidence;
- observation/exposure intervals and derived temporal relation;
- objective-confirmation evidence, when available;
- dechallenge/rechallenge evidence, when applicable;
- dose-response evidence, when applicable;
- known-mechanism/prior-evidence references;
- competing etiologies and concomitant exposures;
- baseline/comparator/aggregate evidence, when available;
- explicit missing evidence;
- assessment method/version and assessment time;
- uncertainty.

## Conservative relationship classes

V1 does not claim a universal international causality scale. It uses a small internal evidence state:

- `Indeterminate`
- `TemporalAssociationOnly`
- `EvidenceSuggestsRelationship`
- `EvidenceSupportsRelationship`
- `EvidenceAgainstRelationship`

External/local scales are carried as labels with exact system/version provenance and do not automatically change the internal evidence state.

## Policy

A versioned assessment policy defines the minimum evidence required for stronger classes. V1 enforces at least:

- temporality alone cannot produce `EvidenceSuggestsRelationship` or stronger;
- a positive relationship requires review of alternative etiologies;
- `EvidenceSupportsRelationship` requires more than one non-temporal supporting evidence kind;
- missing policy-required evidence forces `Indeterminate`;
- contradictory evidence is preserved rather than discarded;
- absence of a known mechanism does not establish `EvidenceAgainstRelationship`.

## Regulatory interpretation

Regulatory/reporting meaning is a separate derived artifact. A jurisdiction/protocol policy may interpret `EvidenceSuggestsRelationship` or stronger as meeting a `reasonable possibility` threshold, but the causal kernel itself does not file reports or claim regulatory compliance.

## Non-claims

- temporal proximity is not causation;
- dechallenge/rechallenge is not automatically dispositive;
- one case does not establish population-level causal effect;
- a population safety signal does not by itself prove an individual case was caused by the exposure;
- assessor labels are not self-authenticating authority evidence;
- this v1 kernel is not a substitute for medical/scientific judgment.

## Integration sequence

1. Pure causal-evidence/assessment crate.
2. Exact medication-administration occurrence exposure adapter.
3. Legacy trial adverse-event adapter that preserves the original report without upgrading its causality enum.
4. Authority + DNA-rooted causal-assessment attestation.
5. Population-level pharmacovigilance signal layer.
6. Regulatory export adapters (including E2B/ICSR where applicable) only after qualification.
