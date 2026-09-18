# Clinical Population Accountant Canonical Index v1

## Purpose

Close the caller-supplied-read-set gap from P0 #92 without claiming that an eventually consistent DHT can prove global absence.

The high-assurance path distinguishes:

1. DHT-valid trusted accountant state;
2. canonical admission into one exact accountant lineage;
3. bounded nested runtime snapshot of admitted states and corrections;
4. conflict-preserving canonical reduction.

## Deterministic lineage identity

The synthetic index anchor is derived only from the already-public tuple:

- release-policy digest;
- accountant-instance digest;
- accountant-method digest.

It contains no query, output, patient, cohort, or protected receipt identity.

The synthetic anchor entry is never committed; only its deterministic `EntryHash` is used as the link base.

## Canonical admission

A trusted accountant state can exist without index admission, but the high-assurance canonical read model ignores unadmitted state.

A `LineageToStates` link is valid only when:

- the target resolves as a DHT-valid trusted accountant state;
- the link base equals the deterministic anchor derived from that exact state's lineage;
- the link author is the same agent that authored the target state.

Third parties cannot canonize another verifier's state. Admission links are append-only.

## Bounded nested materialization

`materialize_canonical_population_accountant_lineage` performs:

1. network-backed admitted-state read A;
2. for every admitted state, correction-set read A, materialization, correction-set read B;
3. network-backed admitted-state read B and requires A == B;
4. final correction-set read C for every state and requires it still equals A/B.

V1 bounds:

- at most 4,096 admitted states per lineage;
- at most 256 corrections per state;
- at most 16,384 corrections across one snapshot.

Any membership/correction race observed inside the window causes the materialization to fail and require retry.

## Canonical application boundary

`mycelix-clinical-population-accountant-canonical-state` accepts only the nested runtime snapshot type. It rechecks:

- read-boundary marker;
- observation time ordering;
- state and correction counts;
- duplicate state/correction actions;
- exact lineage tuple for every state;
- exact correction target and private-receipt commitment.

It then invokes the fork-preserving reducer and renames the outward current state to:

`ObservedCurrentWithinBoundedCanonicalRead`.

There is deliberately no unqualified `Current` result.

## Fork semantics

Multiple admitted genesis states, sibling successors, or multiple live leaves become explicit conflict. Timestamp, action order, insertion order, and state ID never choose a winner.

## Non-claims

This design does not prove global DHT completeness. An unpropagated state/admission/correction can remain unseen by one conductor. The returned snapshot proves only that this conductor's network-backed observed membership remained stable through the bounded closure sequence.

The pure canonical adapter trusts that its snapshot came from the runtime materializer; deserializing a compatible wire object is not cryptographic proof of runtime provenance.

P0 #72 exact source-entry-definition proof remains a promotion concern for cross-zome addressable dependencies.

## Security principle

Canonical means **admitted under the canonical index**, not **globally complete truth**.
