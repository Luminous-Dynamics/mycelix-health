# Clinical Population Interactive Release V1

Status: **experimental / draft**

This tranche closes the composition gap between the pure population-release preflight and the DNA-rooted/canonically discovered privacy-accountant state.

## Authority boundary

`mycelix-clinical-population-release` remains useful for validating request shape, DP mechanism metadata, exact query/output binding, and protected accountant-receipt continuity. Its `PopulationReleaseCapabilityV1` is **preflight evidence only for interactive DP**.

A production interactive release executor MUST require `TrustedInteractivePopulationReleaseCapabilityV1` from `mycelix-clinical-population-interactive-release`. The two capability types are intentionally unrelated and non-interchangeable.

Static pre-specified reports remain governed by their static suppression/composition path.

## Required interactive chain

```text
protected accountant receipt before
        +
protected accountant receipt after
        +
exact DP query / mechanism / output
        |
        v
recompute keyed receipt commitments
        |
        v
DNA-qualified trusted public states
        |
        v
same-author canonical admission
        |
        v
nested bounded canonical snapshot
        |
        v
sole ObservedCurrentWithinBoundedCanonicalRead leaf
        |
        v
exact after.previous_state == trusted before action
        |
        v
TrustedInteractivePopulationReleaseCapabilityV1
```

## Private-receipt commitment

The commitment preimage is deterministic:

1. fixed domain string `mycelix.health.population-accountant-private-receipt.v1`;
2. frame version byte;
3. big-endian encoded JSON length;
4. serialized `PrivacyAccountantReceiptV1` bytes.

The selected public scheme is then applied with a secret 32-byte key:

- HMAC-SHA-256; or
- keyed BLAKE3.

The secret key is never serialized by the gate and is zeroed when the key wrapper is dropped. A zero key is rejected.

Both the DNA-root verifier that authorizes the public accountant state and the trusted interactive release gate must use the same commitment helper/contract. The public DHT never receives the private receipt or commitment key.

V1 rejects a commitment-scheme change inside a single interactive transition. Key/scheme rotation requires separately reviewed migration semantics rather than silent continuity.

## Exact binding requirements

The gate rejects unless all of the following hold:

- the request is a structurally valid `InteractiveDifferentialPrivacy` request;
- request policy digest equals the canonical accountant lineage policy;
- accountant instance and method equal the canonical lineage;
- the bounded canonical reducer returns exactly one usable current leaf;
- that leaf is the request's protected `accountant_after` state;
- the leaf's predecessor action exists in the same canonical snapshot;
- the predecessor is the request's protected `accountant_before` state;
- both protected receipts recompute to the public keyed commitments;
- public/private sequence, query count, cumulative privacy loss, instance and method match exactly;
- request-level query/mechanism/output binding remains valid through the lower-level release validator.

A fork, review-required state, absent predecessor, wrong key, stale snapshot or cross-policy/accountant lineage fails closed.

## Freshness

V1 uses two conservative software ceilings:

- canonical read must close no more than 60 seconds after the protected `accountant_after.observed_at_micros`;
- a trusted capability may only be consumed within 60 seconds after canonical snapshot closure.

These checks constrain evidence semantics but do **not** create a trusted clock. Production executors must pass a runtime-trusted clock (for example conductor `sys_time()`) when consuming the capability.

## Receipt

Consuming the trusted capability emits `TrustedInteractivePopulationReleaseReceiptV1`, binding:

- exact request digest;
- exact release-policy digest;
- exact source finding and public output;
- exact trusted before/after accountant action hashes;
- accountant sequence/query count;
- bounded canonical observation interval;
- requested and released timestamps.

The receipt is evidence, not reusable authority.

## Non-claims

This tranche does not claim:

- global DHT completeness;
- that queued CI is executable qualification;
- that the DP mechanism mathematics have been independently audited;
- that a caller-provided release timestamp is a trusted clock;
- that #92 conductor race tests are complete;
- that the lower-level generic interactive preflight capability has been removed from the API.

Until the legacy generic interactive capability is removed/deprecated at the executor boundary and conductor qualification is complete, broad interactive release remains experimental.
