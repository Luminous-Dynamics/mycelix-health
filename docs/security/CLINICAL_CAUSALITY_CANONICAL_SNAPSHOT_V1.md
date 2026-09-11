# Clinical causality canonical snapshot v1

## Purpose

Materialize one bounded runtime capsule containing every **index-admitted causal publication observed** for one opaque qualified-receipt commitment plus every correction observed for those publications.

This is the runtime composition layer between canonical admission (#85), correction materialization (#84), and the conflict-preserving causal reducer (#82).

## Closure sequence

`materialize_canonical_causal_commitment_snapshot` performs:

1. network-backed commitment-index read A;
2. for every observed admitted attestation:
   - network-backed attestation fetch;
   - correction-link read A;
   - bounded materialization and exact target/commitment checks;
   - correction-link read B, which must match A;
3. network-backed commitment-index read B, which must match index read A;
4. final correction-link read C for every publication, which must still match its materialized correction set;
5. only then return the canonical snapshot.

This catches membership drift that occurs while later publications are being materialized and is stronger than independently materializing each publication without an outer closure pass.

## Bounds

- maximum observed index links per commitment: 256;
- maximum observed correction links per attestation: 256;
- maximum total materialized corrections in one canonical snapshot: 4,096;
- duplicate links to the same action are idempotent after target deduplication.

Any bound violation fails closed.

## Returned semantics

The returned snapshot carries:

- exact opaque commitment;
- deterministic index anchor hash;
- all observed index-admitted publication snapshots;
- every observed correction record for each publication;
- per-publication correction observation windows/counts;
- global observation window/counts;
- `NestedNetworkBackedStableReadsWithFinalClosureChecks` as the explicit read boundary.

## What this establishes

The capsule establishes a tightly bounded claim:

> During this conductor's materialization interval, the observed canonical index membership was stable across the outer reads, each observed publication's correction set was stable across its inner reads, and every correction set still matched at final closure.

## What this does not establish

It does not prove:

- global DHT completeness;
- absence of an admission/correction that has not propagated;
- that a noncanonical/unindexed attestation does not exist;
- clinical truth or causal effect;
- population-level causal effect;
- regulatory reportability.

The read-boundary classification must survive into every downstream state/result that uses this snapshot.

## Canonical vs noncanonical

Only attestations admitted through the validated opaque-commitment index appear in this snapshot. An otherwise valid but unindexed causal attestation is intentionally excluded from the canonical high-assurance runtime view.

That exclusion is semantic, not censorship or deletion: noncanonical attestations may still exist as DHT evidence and may later be admitted by their original author if they satisfy the admission contract.

## Privacy

The snapshot uses only already-public opaque commitments and public attestation/correction records. It introduces no patient, medication, symptom, event, dose, assessor, or cross-key stable identifier.

## Promotion blockers

- exact source zome/entry-definition proof under P0 #72;
- exact-head build/test evidence;
- conductor adversarial tests for all closure races and bounds;
- canonical reducer integration that refuses non-index-admitted publications;
- P0 #83 bounded-read semantics preserved end-to-end.
