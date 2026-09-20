# Qualification External Anti-Rollback Anchor V1

Tracks QUAL-EVID-005 / #216.

## Problem

A complete rollback of the local checkpoint volume can restore a previously valid epoch, predecessor chain, nonce set and time floor together. Internal signatures and consistency checks cannot prove that a newer state once existed.

The missing evidence must live outside the rollback domain.

## Reuse decision

Do not create a Health-specific transparency log.

Existing Luminous work already demonstrates the needed protocol family:

- Mycelix verifier-transparency consistency links, checkpoint succession, temporal conflict evidence, witness-set rotation and recovery;
- Symthaea fabrication signed transparency checkpoints and persistent predecessor/rollback tracking;
- independent transparency witness quorums across organizations/regions with trust/revocation checks and algorithm diversity.

The domain-neutral extraction is tracked in `Luminous-Dynamics/luminous-dynamics#1962`.

Health therefore owns only the domain adapter and reconciliation theorem.

## Privacy-minimal anchor subject

The external backend receives opaque commitments rather than patient/care content:

```text
subject domain
+ lineage-binding digest
+ checkpoint digest
+ checkpoint epoch
+ lineage-state commitment
+ verified-head digest
```

The #217 verified-head digest commits to the richer qualification/current-head identity.

## External anchor record

The backend proof additionally exposes:

```text
anchor sequence
previous anchor digest
anchor nonce
anchor digest
security-time floor
witness-set epoch
witness-quorum digest
trust-snapshot digest
observed-at
expires-at
```

The Health crate treats a production `VerifiedExternalAnchorProof` implementation as an authority adapter that must be independently qualified.

## Atomic progression

### Prepare

Health builds `AnchorUpdateIntent` containing the exact local state, next sequence, predecessor, nonce and security-time floor.

The intent is explicitly non-authorizing.

### External commit

A separately qualified backend commits/witnesses the intent and returns verified evidence.

### Finalize

Health requires exact equality of:

- state;
- sequence;
- predecessor;
- nonce;
- security-time floor.

Only then can it emit `AnchoredQualificationHead`.

## Crash recovery

```text
local older than external
    -> LocalRollbackDetected

local newer than external
    -> ExternalAnchorBehind

same checkpoint epoch, different state
    -> SameEpochStateConflict

exact local/external match
    -> AnchoredQualificationHead
```

A crash after local prepare but before external commit leaves no authority.

A crash after external commit but before local finalize can recover deterministically by reconciling the surviving local #217 state against the external proof.

## Time-floor refresh

An unchanged checkpoint state may receive a new anchor sequence solely to raise the independently witnessed security-time floor. This does not rewrite checkpoint history.

## Bootstrap

V1 only permits a fresh anchor chain to bootstrap at checkpoint epoch 1. Existing later histories require an explicit, separately governed migration/bootstrap ceremony.

This prevents deployment of the feature from silently blessing an arbitrary historical local head as the external root of trust.

## Product authority

The stronger `AnchoredProfile208ProductAuthorityGate` requires an already anchored current head before delegating to #219's full qualification/profile/current-head product gate.

The resulting `AnchoredProfile208ProductAuthorityToken` retains both #219 evidence ancestry and external witness provenance.

## Fail-closed availability

```text
anchor unavailable
anchor stale
anchor expired
anchor conflict
anchor behind
local rollback
backend equivocation
        -> no product authority
```

Availability loss never increases authority.

## Non-claims

This reference does not prove any concrete external backend, witness-quorum honesty, hardware security, global consensus, endpoint security, clinical validity or regulatory compliance.
