# Clinical Causality v1 — evidence-first plan

Status: design input for the next stacked tranche. This document does not itself establish runtime or clinical qualification.

## Problem

The legacy trial `AdverseEvent` stores a single causality enum directly on the report. Its current integrity validation establishes basic record shape but does not establish the evidence lineage behind that causal conclusion.

For the high-assurance path, temporal sequence and causal attribution must remain separate facts.

## Core invariant

`observed after exposure != caused by exposure`

An adverse event may be recorded and reported with unknown causality. A stronger causal interpretation requires separately preserved evidence.

## V1 evidence graph

A causal assessment should bind:

- exact observed-event identity;
- exact exposure identity (for medication v1, the computable administration occurrence and qualified administration receipt);
- patient/subject binding evidence;
- event onset/observation interval;
- exposure interval;
- signed temporal relation;
- objective-confirmation evidence, when available;
- dechallenge/rechallenge evidence, when applicable;
- dose-response evidence, when applicable;
- known-mechanism/prior-evidence references;
- competing etiologies/concomitant exposures;
- baseline/comparator/aggregate evidence where available;
- missing evidence and uncertainty;
- assessor identity/authority and assessment method/version.

## Assessment semantics

V1 must not invent a universal certainty scale. It should separate:

1. machine-checkable evidence sufficiency;
2. a conservative relationship conclusion;
3. an optional external/local causality scale label with exact scale/version provenance;
4. regulatory/reporting interpretations derived under an explicit jurisdiction/protocol policy.

The conservative relationship conclusion should distinguish at least:

- `Indeterminate`
- `TemporalAssociationOnly`
- `EvidenceSuggestsRelationship`
- `EvidenceSupportsRelationship`
- `EvidenceAgainstRelationship`

`EvidenceSuggestsRelationship` is the minimum class that may support a policy interpretation equivalent to a regulatory `reasonable possibility`; exact policy remains jurisdiction/protocol specific.

## Non-claims

- temporal proximity is not causation;
- dechallenge/rechallenge is not automatically dispositive;
- absence of a known interaction/mechanism is not proof of no relationship;
- an assessor label is not self-authenticating evidence;
- one case report does not establish population-level causal effect;
- population signal and individual-case causality are related but distinct evidence classes.

## Integration sequence

1. Pure causal-evidence/assessment crate.
2. Exact administration-occurrence exposure adapter.
3. Legacy trial adverse-event adapter that preserves the original report without upgrading its causality enum.
4. DNA-rooted assessment attestation.
5. Pharmacovigilance signal aggregation as a separate population-level layer.
6. Regulatory export adapters (e.g. ICSR/E2B) only after the evidence model is qualified.
