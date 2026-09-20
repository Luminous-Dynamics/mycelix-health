# Patient v2 Activation Gate

Reference semantics for PATIENT-PRIV-005 (#203).

## Core theorem

```text
reference qualified
    != integration qualified
    != legacy writes frozen
    != migration rehearsed
    != authority to deploy
    != V2 active
```

This crate exists so a collection of green reference tests cannot silently become
a production Patient v2 cutover decision.

## Required evidence

The gate has 20 mandatory proof lines. Reference proofs bind exact contract
IDs/versions; deployment proofs bind the exact activation `DeploymentIdentity`:
source commit, DNA manifest, integrity/coordinator WASM sets, toolchain and
privacy-contract epoch.

Only `EvidenceState::Qualified` satisfies a requirement. `SourceStaged`,
`QueuedInfrastructure`, `Failed`, `Stale`, `Missing` and `Superseded` all deny.
Queued CI is therefore never a PASS.

Reference evidence cannot satisfy deployment integration evidence. A
ProtectedEnvelopeV2 semantic PASS, for example, cannot stand in for production
AEAD/KDF/serialization/WASM qualification.

## Cutover freeze

A migration rehearsal is unsafe if v1 writes can continue after the rehearsed
snapshot. The v2 contract therefore requires a separately qualified
`LegacyWriteFreezeIntegration` proof and a concrete `CutoverFreezeReceipt`.

The receipt binds exact deployment identity, exact legacy-state digest, non-zero
freeze epoch, proof that legacy writes are blocked, and a unique evidence ID.

After freeze admission, writes become `DenyDuringCutover`. They remain denied
through rehearsal, authority admission and activation authorization. Only actual
`V2Active` switches writes to `ProtectedV2`.

## Migration rehearsal

The rehearsal receipt must bind the exact frozen legacy-state digest and freeze
epoch and positive evidence for idempotency, conflict detection, no plaintext
fallback, unresolved-route accounting, and the fact that historical public
exposure may persist.

A rehearsal of snapshot A cannot authorize activation after the system has moved
to snapshot B.

## Activation authority

Technical evidence, freeze and rehearsal still do not authorize deployment.
`ActivationAuthorityReceipt` binds the exact deployment, canonical evidence
transcript, exact freeze evidence/digest/epoch, exact rehearsal evidence ID, a
finite validity interval, an opaque authority reference and explicit historical
exposure acknowledgement.

Signature/governance verification remains a separate adapter theorem. CI status,
merge permission, ownership or first-caller behavior cannot substitute for the
artifact.

## Time theorem

Authority validity is checked at authority admission, activation authorization,
and final activation. The canonical state also maintains a monotonic security
time fence. Once a time T has been observed, later security-sensitive transitions
cannot claim a time earlier than T.

The production adapter must still provide a trusted clock; this core does not
claim trusted-time implementation.

## Explicit cutover abort

Fail-closed security must not create an unrecoverable availability trap. A frozen
cutover may therefore be abandoned before activation, but only with a separately
verified, finite `CutoverAbortReceipt` bound to:

- exact deployment;
- exact freeze receipt;
- opaque authority reference;
- opaque reason digest;
- finite validity interval.

Abort is allowed only while the cutover is frozen but not active. A successful
abort destroys the abandoned candidate/evidence/freeze/rehearsal/activation
state and returns to `LegacyV1Writable`. A later attempt must start again with a
fresh candidate/freeze/rehearsal/authority chain.

The security clock is preserved across abort, so resetting the cutover cannot
resurrect an expired authority. Abort is structurally rejected after `V2Active`
and after `LegacyV1ReadOnly`.

## State machine

```text
LegacyV1Writable
    -> CandidatePrepared
    -> EvidenceComplete
    -> LegacyWritesFrozen
    -> MigrationRehearsed
    -> ActivationAuthorityAdmitted
    -> V2ActivationAuthorized
    -> V2Active
    -> LegacyV1ReadOnly
```

There is also one explicit pre-activation recovery transition:

```text
LegacyWritesFrozen | MigrationRehearsed |
ActivationAuthorityAdmitted | V2ActivationAuthorized
    -- verified CutoverAbortReceipt --> LegacyV1Writable
```

`ActivationState` is a recovery-aware wrapper around a private inner core. Callers
cannot directly construct `V2Active` or bypass the cross-cutover time fence.
There is no generic force/override activation API.

After activation, protected-read failures become `DenyNoLegacyFallback(...)` and
never re-enable legacy plaintext reads.

## Historical exposure

The activation candidate/evidence/rehearsal model preserves the fact that prior
public-DHT observations may remain observable. Migration is not cryptographic
recall.

## Deliberate boundaries

This crate does not:

- query GitHub or decide whether a CI result is truthful;
- verify evidence/signature/governance artifacts;
- implement a trusted clock;
- hash or sign evidence transcripts;
- perform the production write freeze or migration;
- implement the external governance/signature verification for abort authority;
- inspect exact Holochain WASM/DNA bytes itself;
- make privacy, clinical, HIPAA, POPIA, GDPR or other legal-compliance claims.

It defines the fail-closed evidence/cutover/authority/recovery theorem that
separately qualified production adapters must satisfy.
