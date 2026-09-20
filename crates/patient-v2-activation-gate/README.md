# Patient v2 Activation Gate

Reference semantics for PATIENT-PRIV-005 (#203).

## Core theorem

```text
reference qualified
    != integration qualified
    != migration rehearsed
    != authority to deploy
    != V2 active
```

This crate exists so a set of green reference tests cannot silently become a
production Patient v2 cutover decision.

## Required evidence

The gate has a fixed mandatory proof vocabulary. Reference proofs bind exact
contract IDs/versions; deployment proofs must bind the exact activation
`DeploymentIdentity` (source, DNA manifest, integrity/coordinator WASM sets,
toolchain, privacy-contract epoch).

Only `EvidenceState::Qualified` satisfies a requirement. These states deny:

- `SourceStaged`
- `QueuedInfrastructure`
- `Failed`
- `Stale`
- `Missing`
- `Superseded`

This deliberately matches the project's evidence discipline: queued CI is not a
PASS.

## Reference vs deployment proof

A reference contract can prove a semantic theorem such as CARE-CAP attenuation
or ProtectedEnvelopeV2 representation. It cannot satisfy the separate production
adapter/integration proof bound to the exact deployment candidate.

That prevents evidence laundering of the form:

```text
reference unit tests passed
        -> therefore production crypto/storage is qualified
```

## Migration rehearsal

Migration rehearsal is a separate transition after technical evidence completion.
Its receipt requires exact deployment identity plus positive evidence for:

- idempotency;
- conflict detection;
- no plaintext fallback;
- unresolved domain-route accounting;
- historical public exposure acknowledgement.

## Activation authority

Technical evidence and rehearsal still do not authorize deployment.

`ActivationAuthorityReceipt` is an opaque already-verified authority artifact
that must bind:

- the exact deployment;
- the exact admitted evidence IDs plus rehearsal evidence ID;
- a finite validity window;
- an opaque authority reference;
- acknowledgement that historical legacy-public exposure may persist.

Signature/governance verification is intentionally a separate adapter proof
line. CI status, merge permission, first caller, or ownership cannot substitute
for this authority artifact.

## State machine

```text
LegacyV1Writable
    -> CandidatePrepared
    -> EvidenceComplete
    -> MigrationRehearsed
    -> ActivationAuthorityAdmitted
    -> V2ActivationAuthorized
    -> V2Active
    -> LegacyV1ReadOnly
```

Transitions are ordered and fail closed. There is no force/override API in v1.

Only after `V2Active` does the write target switch to protected v2. Once active,
protected read failures return `DenyNoLegacyFallback`; they never reactivate
legacy plaintext reads.

## Historical exposure

`ActivationCandidate` and qualified rehearsal construction hard-code the legacy
public-exposure acknowledgement to true. The API cannot claim that old public
DHT observations were cryptographically recalled.

## Deliberate boundaries

This crate does not:

- query GitHub/CI;
- decide whether an external run truly qualifies a proof;
- hash/sign evidence transcripts;
- verify governance/operator signatures;
- inspect Holochain WASM/DNA bytes;
- perform migration;
- make privacy, clinical, or regulatory-compliance claims.

It defines the fail-closed aggregation/activation theorem that those independently
qualified adapters must satisfy.
