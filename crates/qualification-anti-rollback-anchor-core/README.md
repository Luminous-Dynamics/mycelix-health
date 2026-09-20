# Qualification Anti-Rollback Anchor Core

Status: source-staged QUAL-EVID-005 / #216 reference.

This crate closes the whole-volume rollback gap left deliberately outside the local #217 checkpoint store.

## Core theorem

```text
locally durable + self-verifying checkpoint history
!=
proof that a newer valid history was not rolled back wholesale
```

Authority therefore requires an independently verified external anchor.

## Privacy-minimal exported state

The external rollback domain receives only opaque commitments:

- lineage-binding digest;
- #217 checkpoint digest;
- #217 checkpoint epoch;
- #217 lineage-state commitment;
- #217 verified-head digest.

The verified-head digest commits to the richer governed identity while avoiding disclosure of care data or raw qualification subject information to the witness backend.

## External verifier boundary

`VerifiedExternalAnchorProof` is an authority-bearing adapter trait. A production implementation must itself verify the backend's signatures, witness quorum, trust/revocation state, monotonic history, conflict rules and recovery semantics.

The intended reusable implementation is the generic transparency/witness extraction tracked in `Luminous-Dynamics/luminous-dynamics#1962`.

This crate does not accept a caller-supplied `verified: bool`.

## Prepare / commit / recovery

### Bootstrap

V1 only bootstraps checkpoint epoch 1. Bringing a pre-existing later lineage under external anchoring requires an explicit migration/bootstrap ceremony.

### Successor

A successor requires:

- same semantic lineage;
- exact next checkpoint epoch, or the exact same state for a security-time refresh;
- monotonic security time;
- next anchor sequence;
- exact predecessor anchor digest;
- a fresh nonce.

### Finalize

`AnchorUpdateIntent` is never authority. Finalization requires independently verified external evidence matching the exact prepared state, sequence, predecessor, nonce and security-time floor.

### Restart

`reconcile_existing_anchor()` classifies local/external state fail-closed:

```text
local checkpoint epoch < external epoch
    -> LocalRollbackDetected

local checkpoint epoch > external epoch
    -> ExternalAnchorBehind

same epoch + different state commitment
    -> SameEpochStateConflict

exact state + fresh verified anchor
    -> AnchoredQualificationHead
```

No arrival-order winner is selected.

## Final #208 gate

`AnchoredProfile208ProductAuthorityGate` is a stronger successor to #219's product gate. It consumes an `AnchoredQualificationHead`, never a bare #217 head, and emits `AnchoredProfile208ProductAuthorityToken` retaining:

- #219 qualification/profile/checkpoint provenance;
- external anchor digest and sequence;
- witness-set epoch;
- witness-quorum digest;
- anchor trust-snapshot digest;
- externally witnessed security-time floor;
- anchor observation/expiry window.

## Explicit non-claims

This crate does not itself establish:

- cryptographic validity of an external backend;
- TPM/HSM security;
- transparency-log consistency proofs;
- witness independence;
- global consensus;
- protection after compromise of both endpoint and external quorum;
- correctness of the underlying qualification theorem;
- Patient-v2 activation;
- clinical validity or legal/regulatory compliance.

Each backend adapter requires separate qualification evidence.
