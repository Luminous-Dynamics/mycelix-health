# Clinical Transaction Proof Graph v1

**Status:** isolated Rust reference verifier and synthetic fixtures. The verifier is structural and trust-policy driven; it is not a production cryptographic implementation, FHIR adapter, or clinical-safety certification.

## Research objective

Given one frozen operation attempt, can an independent verifier reconstruct the chain from request identity to an exact recipient-observed effect without relying on wall-clock ordering or a single system's success flag?

This prototype composes evidence obligations already being developed elsewhere in Mycelix Health:

- #343 / PR #344 — release receipt and atomic commit contract.
- PR #346 — gateway operation ledger and unknown-outcome reconciliation.
- #338 / PR #355 — causal audit evidence.
- #356 / PR #357 — active FHIR adapter qualification.
- #334 / PR #335 — source authority and FHIR Provenance binding.
- #360 — real Holochain consent/shared-zome path qualification.

It is a verifier and schema experiment, not a replacement event store or parallel authorization policy.

## Model

A graph binds one operation and one attempt to:
- opaque patient, tenant, authenticated actor and recipient references;
- purpose, action, exact resource-set digest, source system and source resource version/digest;
- consent revision digest, revocation epoch, policy digest and idempotency-key digest;
- event identities and digests, parent-event digest references and per-actor sequences;
- component build/configuration digests and independent qualification freshness;
- a commit result, delivery result and recipient-side observation of the exact resource set.

No clinical payload or direct identifier belongs in a public proof graph. Binding values should be keyed/canonical digests or opaque references whose disclosure policy is explicit.

## Key design rules

1. Causal parent references establish ordering. Wall-clock timestamps are retained as observations but are not used as a global order oracle.
2. Every event in one proof graph must bind the same operation, attempt, patient, tenant, actor, recipient, purpose, action, source revision, consent revision, policy, resource set and idempotency key.
3. Each component must have a fresh qualification from an independent issuer, matching the exact build and configuration digests.
4. Cryptographic digest and signature verification is an injected boundary (EvidenceAuthenticator). The fixture authenticator in unit tests is deliberately not cryptographic. Production must supply a reviewed verifier, trust roots, key rotation and key-revocation semantics.
5. Missing, unavailable or stale evidence returns Defer. Tampering, identity mismatch, invalid transition, cycle or resource-set mismatch returns Rejected.
6. A commit or delivery outcome of Unknown remains unresolved. Reconciliation must extend the same operation/attempt and idempotency binding; an unknown result is never promoted to success or denial without new evidence.
7. A sender-side transport acknowledgement is not a recipient-side effect witness. A successful release proof requires a recipient-role event bound to the expected recipient and exact resource-set digest.
8. No distributed exactly-once guarantee is implied. A local transactional backend and a remote recipient are separate trust/atomicity boundaries.

## Run

The crate has no third-party dependencies and is excluded from the large Holochain workspace.

    cargo fmt --manifest-path research/clinical-transaction-proof-graph/Cargo.toml -- --check
    cargo test --manifest-path research/clinical-transaction-proof-graph/Cargo.toml --locked
    cargo clippy --manifest-path research/clinical-transaction-proof-graph/Cargo.toml --locked -- -D warnings

The synthetic authenticator only checks deterministic fixture strings. It does not verify real signatures or hashes. The qualification registry is also a test fixture. These tests validate graph logic only.

## Current v1 test families (22 Rust unit tests)

- valid flow with deliberately out-of-order wall-clock observations;
- explicit authenticated-subject binding and spoofed-identity rejection;
- commit-unknown and delivery-unknown reconciliation on the same attempt;
- parent digest mutation and missing parent evidence;
- cross-patient binding mismatch;
- expired or self-issued component qualification;
- unresolved commit outcome;
- sender acknowledgement incorrectly presented as recipient effect;
- incorrect recipient resource-set digest;
- causal cycle, sequence gaps, and unlinked same-actor sequence forks;
- conflicting terminal commit outcomes and duplicate recipient effect witnesses;
- unrelated later success cannot erase an earlier unreconciled unknown commit/delivery result;
- invalid signature fixture;
- verified commit rejection without delivery.

## Next evidence gates

1. Add canonical serialization and a reviewed cryptographic verifier (do not roll a hash or signature primitive by hand).
2. Bind FHIR R4 Provenance and AuditEvent references to source version and event graph.
3. Feed frozen outputs from PR #344, PR #346, PR #355 and PR #357 through the same verifier rather than creating parallel fixtures.
4. Add a real durable database adapter and synthetic recipient sandbox with crash/retry/lost-ack tests.
5. Scale to 10k-event graphs and report p50/p95/p99 verification time, graph size, memory, false deferrals and evidence acquisition cost.

## Claim boundary

This experiment does not demonstrate that a real FHIR server, Holochain DHT, database, OAuth identity provider or recipient system satisfies these contracts. It makes no PHI, HIPAA, 42 CFR Part 2, clinical-safety, regulatory-certification or production-readiness claim.
