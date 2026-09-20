# Patient v2 Cutover Composition Core

Reference semantics for PATIENT-PRIV-007 (#207).

## Core theorem

```text
qualified activation-side receipt
+
qualified production-migration receipt
!=
production activation

unless every cross-cutover identity is coherent
and the final authority binds the exact protected destination.
```

This crate is deliberately thinner than #204 and #206. It does not repeat their migration, freeze, capability, or activation semantics. It composes already-verified adapter receipts and freezes only the final cross-receipt and durable-commit theorem.

## Lower-layer contract versions

V1 refuses unsupported lower-layer contracts:

- activation side: contract v2;
- migration-result side: contract v1.

A future lower-layer theorem change therefore cannot silently inherit old composition authority.

## Exact cutover identity

Both sides must name the same:

- `DeploymentIdentity`;
- freeze evidence ID;
- frozen legacy-state digest;
- freeze epoch;
- rehearsal evidence ID.

The migration side must additionally be qualified, latest-active, and not abandoned.

## Final authority

`FinalActivationAuthority` must bind the exact:

- cutover identity;
- evidence-set identity;
- migration binding;
- protected destination;
- opaque authority identity/binding;
- finite validity interval;
- historical-public-exposure acknowledgement.

The exact authority identity and binding admitted during composition must be the same authority presented at final commit. Authority substitution is rejected.

## Composition commitment

`CompositionReceipt::transcript_bytes()` is a canonical structural transcript.

A separate `CompositionCommitmentReceipt` represents an already-qualified external digest/hash adapter over that transcript. This crate does **not** claim to implement or verify the cryptographic hash itself.

## Durable prepare

`DurablePrepareReceipt` binds:

- exact composition digest;
- non-zero activation epoch;
- durable prepare receipt ID;
- `prepared_at_micros`;
- positive durability evidence.

The prepared timestamp becomes a monotonic security-time floor and is restored during crash recovery.

## Durable activation receipt

`PatientV2ActivationReceipt` binds directly:

- exact prepare receipt;
- exact composition digest;
- activation epoch;
- `activated_at_micros`;
- exact cutover identity;
- exact protected destination;
- exact final authority identity/binding;
- `v2_discoverable = true`;
- `legacy_v1_read_only = true`;
- positive durability evidence.

Activation time may not precede durable prepare time.

## Replay

An exact replay of the already-admitted durable activation receipt returns `IdempotentReplay`.

A different second activation receipt after `V2Active` returns `ConflictingReplay`.

## Crash recovery

The coordinator has no `LegacyV1Writable` phase.

Recovery can reconstruct only:

```text
CompositionAdmitted
ActivationCommitPrepared
V2Active
```

An activation receipt without its durable prepare is rejected.

A recovered durable prepare restores its `prepared_at_micros` time fence. A recovered durable activation restores its `activated_at_micros` fence.

Before a durable prepare exists, trusted-time continuity across a process crash remains a production-adapter responsibility; this reference does not claim an in-memory timestamp survived that crash.

## Non-claims

This crate does not:

- verify #204 or #206 evidence truth itself;
- perform migration;
- persist Holochain state;
- implement signatures/governance verification;
- implement trusted time;
- implement cryptographic hashing;
- provide a distributed transaction protocol;
- prove confidentiality;
- provide legal/regulatory compliance.

It proves only structural composition, identity coherence, replay behavior, and durable prepare/activation admission semantics for already-verified inputs.
