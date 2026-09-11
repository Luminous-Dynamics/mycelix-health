# Clinical causality commitment index v1

## Purpose

Provide a privacy-preserving canonical-admission and discovery surface for public qualified causal attestations that share one opaque qualified-receipt commitment.

The index does not make every syntactically valid causal attestation canonical. Canonical status requires a separate DHT-valid admission link under the deterministic commitment-scoped index base.

## Why admission is separate

A modified coordinator can publish a DHT-valid causal attestation and omit any best-effort indexing call. Therefore the system must not treat mere attestation existence as equivalent to canonical high-assurance publication.

V1 uses this rule:

> valid causal attestation + valid commitment-index admission = canonical publication candidate

A valid but unindexed attestation remains noncanonical evidence and must not enter the canonical high-assurance reducer through the new adapter path.

## Deterministic index identity

The link base is the `EntryHash` of a synthetic `CausalCommitmentIndexAnchorV1` containing:

- schema version 1;
- the already-public opaque commitment scheme;
- the already-public opaque commitment value.

The synthetic anchor entry does not need to be committed. The integrity zome rejects committed index-anchor entries; only the deterministic hash is used as the link base.

The commitment scheme is part of index identity, so identical 32-byte values under HMAC-SHA-256 and keyed BLAKE3 do not collide semantically.

## Admission validation

For `CommitmentToAttestations` links, integrity validation requires:

1. base is an `EntryHash`;
2. target is an `ActionHash`;
3. target resolves through `must_get_valid_record`;
4. target decodes as a qualified causal attestation and passes its shape validation;
5. deterministic base recomputed from the target's opaque receipt commitment exactly matches the supplied base;
6. link author equals the target attestation action author.

A third party therefore cannot canonize another verifier's attestation, and an attestation cannot be indexed under another receipt commitment.

Admission links are append-only. Link deletion is rejected.

## Canonical discovery snapshot

`materialize_causal_commitment_index_snapshot`:

1. derives the deterministic anchor from the requested commitment;
2. performs a network-backed first link read;
3. bounds observed fan-out to 256 links;
4. deduplicates/sorts action targets;
5. materializes every observed attestation with network-backed reads;
6. rechecks each target's exact commitment;
7. performs a second network-backed index read;
8. fails if the observed target set changed.

The result carries `NetworkBackedStableDoubleRead`, exact observation timestamps, anchor hash, observed target count, and every materialized index-admitted attestation.

## Epistemic boundary

The index solves **canonical admission/discovery**, not eventual-consistency impossibility.

A stable double-read means the observed canonical target set did not change around materialization. It does not prove an admission link has not been authored elsewhere and not yet propagated.

Canonical readers must preserve this bounded read classification. They must not rename it to global completeness.

## Privacy

The index base contains only a deterministic hash of the opaque keyed commitment that is already public in the attestation. It introduces no patient, medication, event, symptom, assessor, dosage, or research identifier.

There is intentionally no stable cross-key correlator. Rotating the private commitment key may move the same protected receipt to a different public index root. Cross-key continuity requires a separately reviewed protected migration proof.

## Source-entry-type limitation

The index validator currently resolves a DHT-valid target and structurally decodes it as `QualifiedCausalAssessmentAttestation`, then binds admission to the target author and exact commitment. Exact source zome/entry-definition proof remains a promotion blocker under P0 #72.

## Non-claims

This index does not establish clinical truth, causal effect, population treatment effect, regulatory reportability, global DHT completeness, or that every noncanonical attestation is malicious. It establishes only the v1 canonical-admission contract described above.
