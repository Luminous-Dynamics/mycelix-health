# Mycelix Audit Chain Core

Reference semantics for audit-evidence P0 #178.

## Chain model

The reference uses **one linear chain per author**. It does not infer a single total order from a distributed patient-wide collection.

For each append, the candidate binds:

- opaque chain ID;
- opaque author ID;
- sequence;
- exact predecessor entry commitment;
- protected/business-event digest;
- occurrence time;
- externally supplied entry commitment.

`transcript_bytes()` provides a canonical serializer-independent byte transcript for a separately qualified hash/signature adapter.

## Genesis theorem

Genesis is valid only when the predecessor query positively returns `KnownAbsent` for the exact `(chain, author)` identity.

The following can never become genesis:

- network/dependency unavailable;
- authorization/authentication failure;
- decode failure;
- malformed/invalid predecessor state;
- an empty `Complete` collection with ambiguous provenance.

## Verification theorem

A complete retrieved chain is sorted by explicit sequence before verification, so retrieval order is not chain order.

Verification rejects:

- mixed authors;
- mixed chain identities;
- sequence gaps;
- predecessor mismatch;
- duplicate sequence;
- competing children/forks;
- malformed genesis.

## Concurrency/forks

Two different candidates for the same `(chain, author, sequence, predecessor)` are an explicit `Fork`.

The reference detects this condition; it does not invent an automatic winner. Production Holochain integration must define whether per-author source-chain ordering prevents this or how a fork is surfaced/resolved.

## Multi-author checkpoints

`AuditCheckpoint` records a deterministic set of per-author chain heads for a subject. It does **not** claim those events have a single global linear order.

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
