# Clinical causality read materialization v1

## Purpose

Provide one canonical coordinator path for materializing a known qualified causal-attestation publication and every correction link observed by the local conductor without upgrading eventual-consistency observations into a claim of global completeness.

## Returned boundary

`CausalAttestationPublicationSnapshotV1` always carries:

- `read_boundary = NetworkBackedStableDoubleRead`;
- the exact attestation action hash and decoded publication;
- every correction target observed in the stable link set;
- the observed correction-target count;
- observation start/end timestamps.

`NetworkBackedStableDoubleRead` means only:

1. the coordinator requested network-backed link metadata;
2. it captured the correction-link target set;
3. it materialized and decoded every observed correction target;
4. every correction was verified to target the requested attestation and opaque receipt-commitment lineage;
5. a second network-backed link query returned the same target set.

It does **not** mean:

- no unpropagated correction exists;
- no other publication shares the same opaque receipt commitment;
- the conductor has a globally complete DHT view;
- the snapshot is safe to upgrade into global `Current` truth;
- a missing record is proven not to exist elsewhere.

## Fail-closed behavior

Materialization fails when:

- the requested attestation is unavailable;
- an observed correction target cannot be fetched/decoded;
- a correction targets another attestation;
- a correction crosses opaque receipt-commitment lineage;
- correction fan-out exceeds 256 observed targets;
- either link target is not an `ActionHash`;
- the correction target set changes between the two reads.

Duplicate links to the same correction action are semantically idempotent and deduplicated before comparison.

## Network-read caveat

Holochain is eventually consistent. Network-backed reads improve freshness but cannot prove absence of data that has not propagated to the queried view. The returned boundary is therefore intentionally narrower than `Complete`, `Authoritative`, or `Global`.

## Relationship to the reducer

The #82 reducer remains a pure projection of supplied records. This materializer strengthens how correction records for one known publication are collected, but it does not solve discovery of every publication sharing one opaque receipt commitment. P0 #83 tracks canonical commitment-scoped publication discovery and non-upgradable read-boundary semantics.

## Privacy

The materializer does not introduce patient, medication, symptom, assessor, or clinical-event identifiers. It operates on already-public attestation/correction actions and their opaque receipt commitment.

No stable cross-key correlation identifier is introduced. Commitment-key rotation may intentionally split public lineages unless a separately reviewed protected migration proof is used.
