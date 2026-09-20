# Qualification Adapter Profile Core

Reference semantics for QUAL-EVID-002 (#211), stacked on QUAL-EVID-001 (#209 / draft #210).

## Core theorem

```text
valid governed qualification
!= valid authority for this product seam
```

A product authority boundary must independently verify that an already-governed qualification was issued for the exact seam it is attempting to authorize.

## Finite seam registry

V1 freezes a finite `AdapterSeamId` vocabulary:

- `Patient204ActivationSide`
- `Patient206MigrationResult`
- `Patient208CompositionCommitment`
- `Patient208DurablePrepare`
- `Patient208DurableActivation`
- `Audit190Checkpoint`
- `Monitoring184Assessment`
- `SecurityP0Closure`

There is no free-form seam string and no `AnyQualification` production token.

Only the first real seam is implemented in this tranche: `Patient208CompositionCommitment`.

## #208 composition profile

`Profile208CompositionCommitment` binds the exact:

- semantic qualification lineage;
- #208 composition contract/version through `ContractRef`;
- exact composition subject digest;
- mandatory cutover/context digest;
- claim-profile digest;
- governance-policy digest;
- trust-store digest;
- deployment-evidence digest;
- positive maximum consumption age;
- profile digest supplied by a separately qualified profile/configuration adapter.

The profile constructs its lineage binding internally with `AdmissionScope::Cutover`; callers cannot reinterpret the same profile as `Reference`, `Monitoring`, or another admission scope.

## Independent seam checks

`Profile208CompositionCommitmentGate::convert(...)` rejects unless all of the following remain true at consumption time:

1. receipt kind is exactly `CompositionCommitmentQualification`;
2. qualification semantic-lineage binding equals the exact profile binding;
3. local monotonic ledger is bound to that same lineage;
4. governance-policy digest matches;
5. trust-store digest matches;
6. deployment evidence matches exactly;
7. externally verified current-head checkpoint is valid for the same lineage;
8. qualification receipt is the exact current head named by that checkpoint;
9. receipt is not yet expired;
10. receipt age is within the seam-specific maximum;
11. local receipt status remains admissible/active;
12. the gate's consumption clock has not moved backwards.

A migration-result receipt, P0 closure, unrelated deployment qualification, or stale/superseded composition receipt cannot become this token.

## Why current head is separate from local admissibility

QUAL-EVID-001 deliberately maintains a per-lineage in-memory monotonic ledger. Admitting sequence N+1 does not automatically mark sequence N superseded; an older receipt can remain `Active` until governance records a disposition.

For high-authority product consumption, this crate therefore requires both:

```text
local receipt admissible
+
verified current lineage head == this receipt
```

`VerifiedLineageHead` is an adapter assertion in this reference. Production must derive it from a separately qualified durable/checkpoint protocol. This crate does not pretend that process-local vector order is globally current distributed truth.

## Time rollback containment

The seam gate observes consumption time before semantic checks. After observing T=1000, a retry at T=200 returns `ClockRollback` even if the earlier timestamp would otherwise make the receipt fresh.

The in-memory clock fence is not a cross-process clock theorem. Production restart continuity must be provided by trusted/durable adapter evidence.

## Typed output

Successful conversion returns only:

`Profile208CompositionCommitmentToken`

The token is intentionally non-cloneable and has no public constructor. It preserves:

- governed receipt digest;
- adapter-profile digest;
- exact semantic lineage binding;
- governance-policy digest;
- trust-store digest;
- deployment evidence;
- current-head checkpoint digest;
- issue/expiry times.

A later integration with #208 should consume this typed token rather than a raw `qualified: bool` or generic `VerifiedQualification`.

## Parent API extension

This stacked child adds read-only metadata accessors to #210's scoped verified qualification:

- receipt kind;
- deployment-evidence digest;
- issued-at time;
- expires-at time.

The raw verifier/model remain private. The entire `QualificationLineageBinding` is compared opaquely for equality rather than exposing its internal subject/context/contract fields individually.

## Qualification target

The dedicated workflow runs both:

1. the modified parent `qualification-receipt-core`; and
2. this child `qualification-adapter-profile-core`.

Both must pass exact Rust 1.96 formatting, locked tests and `-D warnings` Clippy under one exact child head.

## Non-claims

This crate does not establish:

- authenticity of the adapter-profile digest;
- cryptographic receipt/signature verification;
- trust-store correctness;
- governance-policy correctness;
- trusted clock correctness;
- durable consumption-clock persistence;
- truth/freshness of `VerifiedLineageHead` beyond the external adapter assertion;
- durable/distributed monotonic lineage checkpointing;
- #208 product integration;
- migration correctness;
- confidentiality or clinical validity;
- legal/regulatory compliance.

It establishes the product-seam conversion theorem for independently qualified receipt/profile/checkpoint inputs.
