# Mycelix Audit Chain Core

Reference semantics for audit-evidence P0 #178.

## Chain identity

The reference uses **one linear chain per author per subject**. It does not infer a single total order from a distributed patient-wide collection.

The exact structural identity is:

`(opaque subject context, author, author-scoped chain ID)`

A `ChainId` is not assumed globally unique across authors. The subject context is included directly in every append transcript so a valid chain cannot be silently reinterpreted as evidence for another person/context.

For each append, the candidate binds:

- opaque subject context;
- opaque author ID;
- opaque author-scoped chain ID;
- sequence;
- exact predecessor entry commitment;
- protected/business-event digest;
- occurrence time;
- externally supplied entry commitment.

`transcript_bytes()` provides canonical serializer-independent bytes for a separately qualified hash/signature adapter.

## Genesis theorem

Genesis is valid only when the predecessor query positively returns `KnownAbsent` for the exact `(subject, author, chain)` identity.

The following can never become genesis:

- network/dependency unavailable;
- authorization/authentication failure;
- decode failure;
- malformed/invalid predecessor state;
- an empty `Complete` collection with ambiguous provenance.

## Verification theorem

A complete retrieved chain is sorted by explicit sequence before verification, so retrieval order is not chain order.

Verification rejects:

- mixed subject/author/chain identity;
- sequence gaps;
- predecessor mismatch;
- duplicate sequence;
- competing children/forks;
- malformed genesis.

## Concurrency/forks

Two different candidates for the same exact `(subject, author, chain, sequence, predecessor)` are an explicit `Fork`.

The reference detects this condition; it does not invent an automatic winner. Production Holochain integration must define whether per-author source-chain ordering prevents this or how a fork is surfaced/resolved.

## Multi-author checkpoints

`AuditCheckpoint` records exactly one verified chain head per author for one subject context. Heads whose subject does not match the checkpoint are rejected; duplicate authors are rejected.

Because chain IDs are author-scoped, equal `ChainId` bytes under different authors are permitted and explicitly tested. Canonical checkpoint ordering is `(author, chain_id)`.

The checkpoint does **not** claim that events across authors have a single global linear order.

A cryptographic adapter may hash/sign the canonical checkpoint transcript to create a patient/auditor-visible checkpoint commitment without exposing detailed PHI event content.

## Privacy boundary

The chain operates on `EventDigest`, not human-readable audit detail. Detailed events can remain protected under #157/#164 while integrity commitments use opaque IDs/digests where distributed verification is justified.

## Deliberate boundaries

This crate does not:

- hash or sign transcripts;
- store Holochain entries;
- prove source-chain concurrency behavior;
- make detailed audit entries DHT-public;
- prove atomicity between a business operation and audit persistence;
- qualify the current `chained_log_data_access()` implementation;
- make regulatory-compliance claims.

Those are integration/qualification proof lines after these semantics are accepted.

Tracks #178. Related #164, #157, #182.
