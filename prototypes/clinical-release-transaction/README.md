# Clinical Release Transaction Benchmark v1

**Status:** dependency-free Rust reference contract and deterministic adversarial tests. This is not a production EHR adapter, an exactly-once guarantee for arbitrary remote systems, or a clinical/security certification.

## Why this exists

An authorization decision is a statement about a request at a point in time. It does not by itself prove that a later side effect still uses the same patient, tenant, resource version, recipient, purpose, consent revision or revocation state.

This benchmark measures the execution boundary. The actual backend transaction must compare current authority and resource versions, consume the nonce/idempotency key, persist an audit intent bound to the full request, perform the release, and record its outcome.

## Files

- src/lib.rs: typed receipt, backend/verifier traits, release orchestration and Mutex-backed reference transaction backend.
- scenarios-v1.json: machine-readable adversarial scenario manifest (21 scenarios).
- Cargo.lock: dependency-free locked test target.

## Outcomes

- Released: the backend confirms one committed side effect.
- AlreadyReleased: the same idempotency key is reconciled to its original committed receipt.
- Denied: an established binding, signature, expiry, revocation, resource-version or replay failure.
- Deferred: current authority cannot be established, the backend lacks atomic-commit capability, storage is unavailable, or the prior result is unknown.

Deferral never authorizes an action. Unknown outcomes must be reconciled under the same idempotency key; this reference service never invents a new key or blindly repeats a release.

## Production adapter contract

TransactionalReleaseBackend::commit_once is deliberately strong. A conforming production adapter must implement a single atomic compare-and-commit transaction that:

1. Rechecks current Consent ID/version, revocation epoch and policy version.
2. Checks the exact current resource version and digest.
3. Ensures the revocation view is fresh.
4. Reserves the nonce and idempotency key.
5. Durably appends an audit intent bound to the full request and resource.
6. Performs the side effect at most once.
7. Durably records the outcome and release receipt.

If the target system cannot provide this boundary, the adapter must return defer or document its weaker consistency model. An outbox and idempotent remote API can reduce duplicate effects, but do not automatically create a cross-system atomic transaction.

The in-memory Mutex backend is a test oracle for the state-machine contract only. The threaded nonce and revocation races exercise two critical concurrent histories, but they are not exhaustive model checking and do not prove a production database adapter is linearizable. It does not demonstrate that Holochain, PostgreSQL, SMART, or a remote FHIR server implements this transaction.

The ReceiptVerifier trait is an explicit boundary: production must supply a real cryptographic verifier with key rotation, revocation and algorithm policy. The test verifier is a stub and must never be used for production signatures.

Execution identity and time are also trusted adapter inputs. A production gateway must derive the actor from authenticated transport/session context and the time from a trusted server clock, not from request body fields.

## Running

```bash
cargo fmt --manifest-path prototypes/clinical-release-transaction/Cargo.toml -- --check
cargo test --manifest-path prototypes/clinical-release-transaction/Cargo.toml --locked --target x86_64-unknown-linux-gnu
cargo clippy --manifest-path prototypes/clinical-release-transaction/Cargo.toml --locked --target x86_64-unknown-linux-gnu -- -D warnings
```

The crate is excluded from the large Holochain workspace so this benchmark runs independently, without sibling workspace crates or a conductor.

## Safety metrics and targets

- Unauthorized side effects: 0.
- Duplicate side effects per idempotency key: 0.
- Side effects after revocation/resource-version drift: 0.
- Side effects without durable audit intent: 0.
- Unknown outcome blindly replayed under a new key: 0.
- 32 concurrent duplicate attempts: exactly one committed side effect.\n- Distinct idempotency keys racing on one nonce: exactly one committed side effect.\n- Consent revocation racing release: linearizable order, with zero release after revocation.
- Correct replay/reconciliation on deterministic fixtures: 100%.

These are reference-test targets, not measured production results.
