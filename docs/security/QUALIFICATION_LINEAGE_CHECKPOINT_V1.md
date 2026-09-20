# Qualification Lineage Checkpoint V1

Tracks QUAL-EVID-004 (#215).

## Purpose

A governed qualification receipt can be authentic and locally `Active` without being the current head of its semantic lineage. V1 introduces a separate, non-recursive checkpoint protocol that can establish a durable current-head claim without turning the checkpoint into another ordinary qualification receipt.

Core theorem:

```text
Active receipt
!= current lineage head
!= governed current-head checkpoint
!= durably admitted checkpoint
!= product seam authority
```

The current product consumer is the Patient-v2 `Profile208CompositionCommitment` seam from #212.

## Non-recursive signing domains

Checkpoint statements are signed under a separate domain:

```text
MYCELIX-HEALTH-QUALIFICATION-LINEAGE-CHECKPOINT-V1\0
```

Approver attestations use:

```text
MYCELIX-HEALTH-QUALIFICATION-LINEAGE-CHECKPOINT-ATTESTATION-V1\0
```

The checkpoint does not require another ordinary qualification receipt to prove its own currentness.

## Lineage-state reconstruction

The checkpoint implementation reconstructs the supplied receipt graph from exact receipt digest, sequence, predecessor and nonce records. It independently applies contiguous disposition records to derive receipt status.

It does not use array position or arrival order as authority.

A lineage is `Forked` when, among other invalid structures:

- there is more than one genesis/root;
- a predecessor has more than one child;
- more than one leaf exists;
- the graph is disconnected;
- the chain cannot visit every receipt exactly once.

A `Forked` checkpoint contains no current-head digest/sequence/status/predecessor and cannot emit current-head authority.

V1 deliberately does not implement fork resolution. Once a forked checkpoint is durably admitted, the local store becomes `fork_locked`; a later `ActiveHead` checkpoint cannot silently restore authority. A separately governed successor protocol is required to resolve forks.

## State commitments

The checkpoint binds independent domain-separated commitments for:

- semantic lineage binding;
- full normalized receipt/disposition/time state;
- consumed receipt nonces;
- disposition state;
- the exact #214 profile-match commitment.

The profile-match report is independently reconstructed before its commitment is accepted.

## Governed signatures

Checkpoint signatures reuse the existing qualification trust-store and Ed25519 verifier machinery while keeping a distinct checkpoint policy.

Admission verifies:

- exact checkpoint digest target;
- signed attestation timestamp;
- distinct approvers and signer keys;
- signer/key identity from the trust store;
- signer active/not revoked/not compromised at attestation time;
- finite statement and attestation windows;
- required role coverage;
- minimum approvers;
- minimum independent organizations;
- real OpenSSL Ed25519 signatures.

Bundle verification independently reconstructs the complete checkpoint bundle.

## Exact profile binding

For an `ActiveHead` checkpoint, the #214 profile-match report must target the exact current receipt and semantic qualification lineage.

This prevents a valid profile match for receipt A from being combined with a current-head checkpoint for receipt B.

## Strict authority schema

`scripts/qualification-lineage-checkpoint-admission.py` is the intended product/runtime admission facade.

It rejects unknown fields in authority-bearing objects including:

- lineage binding/state;
- receipt and disposition records;
- profile-match reports and claims;
- checkpoint policies;
- trust-store approvers;
- checkpoint statements;
- approvals/bundles/claims;
- durable store state and historical entries.

The lower checkpoint script is an implementation engine and must not be treated as an unrestricted remote/UI/zome authority API.

## Local durability

Accepted checkpoint state is written by temporary-file + file `fsync` + atomic `os.replace` + directory `fsync`.

The durable store binds one semantic lineage and records:

- every accepted checkpoint epoch/digest/predecessor/nonce;
- current/forked lineage state;
- exact profile-match and lineage-state commitments;
- a persisted security-time floor;
- the full signed checkpoint bundle;
- the exact checkpoint-policy snapshot;
- the exact trust-store snapshot used for historical verification.

On every reload, the implementation:

1. checks exact store/entry shape through the strict facade;
2. checks contiguous checkpoint epochs;
3. checks exact predecessor chain;
4. checks unique checkpoint digests/nonces;
5. recomputes latest digest/epoch and fork-lock state;
6. independently re-verifies every historical signed checkpoint bundle using its exact historical policy/trust snapshot;
7. cross-checks each cached summary against its signed checkpoint statement.

Malformed, inconsistent or signature-invalid historical state fails closed.

## Monotonic security-time floor

Checkpoint admission, exact replay and current-head consumption all observe the persisted security time floor.

A later security operation cannot present an earlier time. Successful exact replay and successful head consumption advance the durable floor when they observe a later time.

This blocks a common restart/replay pattern where an expired artifact is retried with an older timestamp after a legitimate later observation.

## Verified head object

Only the latest durably admitted, unexpired, non-forked `ActiveHead` checkpoint may emit a verified head report.

The report binds:

- semantic lineage and binding digest;
- exact head receipt and sequence;
- exact checkpoint digest and signed bundle digest;
- checkpoint epoch;
- exact profile-match commitment;
- lineage-state commitment;
- checkpoint governance policy/trust store/deployment evidence;
- current verification time and checkpoint expiry.

Its claims explicitly state that product-seam conversion has **not** yet occurred. #212 remains the product-purpose gate.

## Restart qualification

The adversarial tests launch a fresh Python process after durable admission and require it to reload/reverify the checkpoint store and emit the same head.

Tests also mutate historically persisted signed bundles/statements and require restart-time verification to reject the store.

## Important non-claim: whole-volume rollback

This local persistence theorem does **not** detect an attacker restoring the entire checkpoint directory to an older, internally valid snapshot. Such a rollback also restores local epochs, nonces, predecessor chain and time floor.

QUAL-EVID-005 (#216) tracks an external monotonic anti-rollback anchor outside the local rollback domain.

Until #216 or an equivalent separately qualified backend exists:

```text
locally durable + self-verifying
!= externally rollback-resistant
```

## Other non-claims

V1 does not establish:

- truth of the underlying engineering/clinical theorem;
- global consensus across unrelated lineages;
- automatic fork resolution;
- production trusted-clock correctness;
- hardware-backed key custody;
- post-quantum signatures;
- public transparency-log inclusion;
- #212 product-seam conversion;
- Patient-v2 activation;
- legal/regulatory compliance.
