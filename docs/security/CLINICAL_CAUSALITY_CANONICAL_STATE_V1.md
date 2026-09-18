# Clinical causality canonical state v1

## Purpose

Project one bounded canonical causal state from the exact nested runtime snapshot emitted by `clinical_causality_index`.

This layer exists so application and research code cannot accidentally feed arbitrary publication lists into the high-assurance path or silently upgrade an eventually-consistent observation into global truth.

## Input contract

`reduce_canonical_causal_snapshot` accepts only `CanonicalCausalCommitmentSnapshotV1` and rechecks:

- nonzero opaque commitment;
- deterministic commitment-index anchor hash;
- expected nested network-backed read-boundary marker;
- valid global observation window;
- publication count consistency;
- total correction count consistency;
- unique publication action hashes;
- unique correction action hashes;
- exact commitment lineage for every publication/correction;
- per-publication correction counts;
- expected correction read-boundary marker;
- correction observation windows contained inside the outer observation window;
- each correction targets the publication under which it was materialized.

Only after those checks does the layer call the lower-level conflict-preserving causal-attestation reducer.

## Epistemic vocabulary

The outward state deliberately avoids generic `Current` terminology.

The possible states are:

- `NoCanonicalPublicationObserved`;
- `ObservedCurrentWithinBoundedRead`;
- `ProjectionConflictWithinBoundedRead`;
- `SupersededWithinBoundedRead`;
- `InvalidatedWithinBoundedRead`;
- `ReviewRequiredWithinBoundedRead`;
- `PublicationSuppressedWithinBoundedRead`.

These names are part of the safety contract. They must not be replaced by a generic globally-current boolean or enum without a separately qualified completeness protocol.

## Runtime provenance non-claim

The snapshot is serializable. Therefore a hostile in-process caller can fabricate bytes shaped like `CanonicalCausalCommitmentSnapshotV1`.

This pure crate validates structure and semantics; it does **not** prove that the snapshot was actually produced by Holochain host reads. Runtime provenance belongs to the trusted conductor/adapter boundary and conductor-level qualification tests.

## Eventual-consistency non-claim

Even a genuine runtime snapshot cannot prove that an admission or correction has not yet propagated to the observing conductor.

Accordingly, this layer never claims:

- global completeness;
- global current truth;
- absence of future/unpropagated records;
- causal truth beyond the underlying qualified causal assessment.

## Privacy

The state remains scoped by the already-public opaque qualified-receipt commitment. It introduces no patient, medication, symptom, event, assessor, or cross-key correlation identifier.

## Promotion blockers

Do not promote this layer beyond experimental until:

1. exact-head workspace build/test/Clippy succeeds;
2. conductor tests prove the snapshot consumed by the trusted adapter is the nested canonical runtime snapshot;
3. P0 #72 exact source-entry-definition validation is resolved for the cross-zome dependencies;
4. P0 #83 bounded completeness semantics remain explicit and non-upgradable;
5. adversarial tests cover malformed counts, duplicate actions, cross-commitment records, cross-publication corrections, anchor substitution, stale/partial reads, and projection conflicts.
