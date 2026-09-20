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

Reference evidence cannot satisfy deployment integration evidence. For example,
a ProtectedEnvelopeV2 semantic PASS cannot stand in for the production
AEAD/KDF/serialization/WASM proof.

## Cutover freeze

A migration rehearsal is unsafe if v1 writes can continue after the rehearsed
snapshot. The v2 contract therefore requires a separately qualified
`LegacyWriteFreezeIntegration` proof and a concrete `CutoverFreezeReceipt`.

The receipt binds:

- exact deployment identity;
- exact legacy-state digest;
- non-zero freeze epoch;
- proof that legacy writes are blocked;
- a unique evidence identity.

After the freeze is admitted, the write target is `DenyDuringCutover`. It stays
that way through rehearsal, authority admission and activation authorization.
Only actual `V2Active` switches writes to `ProtectedV2`.

## Migration rehearsal

The rehearsal receipt must bind the exact frozen legacy-state digest and freeze
epoch, as well as positive evidence for idempotency, conflict detection, no
plaintext fallback, unresolved-route accounting and the fact that historical
legacy-public exposure may persist.

A rehearsal of snapshot A cannot authorize activation after the system has moved
to snapshot B.

## Activation authority

Technical evidence, freeze and rehearsal still do not authorize deployment.
`ActivationAuthorityReceipt` must bind:

- the exact deployment;
- the canonical evidence-set transcript;
- the exact freeze evidence ID, frozen-state digest and freeze epoch;
- the exact rehearsal evidence ID;
- a finite validity window;
- an opaque authority reference;
- explicit acknowledgement of historical public exposure.

Signature/governance verification is intentionally a separate adapter theorem.
CI status, merge permission, ownership or first-caller behavior cannot substitute
for explicit activation authority.

## Time theorem

Authority validity is checked at three separate transitions:

1. authority admission;
2. activation authorization;
3. final `V2Active` transition.

The state remembers the latest observed authority time and rejects clock rollback.
An authority that has been observed expired at T=100 cannot be made valid again by
retrying with T=99.

The production adapter must still supply a trusted clock; the reference core does
not claim trusted-time implementation.

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

`ActivationState` owns its phase privately; callers cannot directly construct a
`V2Active` phase. There is no generic force/override path.

After activation, any protected read failure becomes
`DenyNoLegacyFallback(...)`; it never re-enables legacy plaintext reads.

## Historical exposure

The activation candidate, evidence receipt and rehearsal model preserve the fact
that prior public-DHT observations may remain observable. Migration is not
cryptographic recall.

## Deliberate boundaries

This crate does not:

- query GitHub or decide whether a CI result is truthful;
- verify evidence/signature/governance artifacts;
- implement a trusted clock;
- hash or sign the evidence transcript;
- perform the production write freeze or migration;
- inspect exact Holochain WASM/DNA bytes itself;
- provide a rollback/unfreeze governance protocol;
- make privacy, clinical, HIPAA, POPIA, GDPR or other legal-compliance claims.

It defines the fail-closed evidence/cutover/authority theorem that separately
qualified production adapters must satisfy.
