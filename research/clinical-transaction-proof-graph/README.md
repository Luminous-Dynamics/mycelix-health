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

## Privacy boundary and follow-up

The `sha256:patient`, `sha256:tenant` and similar values in unit fixtures are **synthetic labels**, not a prescription to publish unkeyed hashes of MRNs, patient identifiers, dates of birth, or clinical payloads. Plain hashes of low-entropy values are susceptible to dictionary attacks and may become stable correlation handles.

The current reference type uses generic digest strings and does not yet enforce a safe commitment scheme. Production exports must not use that representation until the scoped-reference design in [#373](https://github.com/Luminous-Dynamics/mycelix-health/issues/373) is implemented and independently qualified. That work will compare transaction-scoped opaque handles, tenant/purpose/epoch-scoped keyed commitments, SD-JWT selective disclosure, and an experimental BBS derived-proof lane. A verifier should see only what it needs to check the operation's binding; it should not receive a global patient lookup capability.

This is distinct from the Holochain DHT-at-rest confidentiality programme (#164/#157/#192). A privacy-preserving proof envelope cannot compensate for plaintext clinical entries or link topology exposed by the underlying storage network, and private storage alone does not prevent over-disclosure in exported receipts.

## Key design rules

1. Causal parent references establish ordering. Wall-clock timestamps are retained as observations but are not used as a global order oracle.
2. Every event in one proof graph must bind the same operation, attempt, patient, tenant, actor, recipient, purpose, action, source revision, consent revision, policy, resource set and idempotency key.
3. Each component must have a fresh qualification from an independent issuer, matching the exact build and configuration digests.
4. Cryptographic digest and signature verification is an injected boundary (EvidenceAuthenticator). The fixture authenticator in unit tests is deliberately not cryptographic. Production must supply a reviewed verifier, trust roots, key rotation and key-revocation semantics.
5. Missing, unavailable or stale evidence returns Defer. Tampering, identity mismatch, invalid transition, cycle or resource-set mismatch returns Rejected.
6. A commit or delivery outcome of Unknown remains unresolved. Reconciliation must extend the same operation/attempt and idempotency binding; an unknown result is never promoted to success or denial without new evidence.
7. A sender-side transport acknowledgement is not a recipient-side effect witness. A successful release proof requires a recipient-role event carrying a durable effect witness bound to the expected recipient, exact operation/attempt, idempotency key and resource-set digest.
8. No distributed exactly-once guarantee is implied. A local transactional backend and a remote recipient are separate trust/atomicity boundaries.

## Scoped privacy references (research v1)

The companion `scoped-reference-v1.schema.json` defines four distinct envelope schemes:
- `transaction-random-v1`: a fresh 256-bit CSPRNG handle scoped to one operation;
- `tenant-purpose-epoch-keyed-v1`: a keyed, domain-separated pseudonym bound to an opaque tenant/purpose/epoch scope and key identifier/epoch;
- `sd-jwt-disclosure-ref-v1`: an opaque reference to an SD-JWT presentation, never the raw token;
- `bbs-derived-proof-ref-v1`: an opaque reference to an experimental BBS derived proof and issuer key epoch.

The Rust module `scoped_refs.rs` checks scheme/scope/key metadata and rejects malformed or cross-scope references, invalid scope IDs and zero key epochs. It now also defines `OperationBindingV2` / `OperationReferenceSetV2`, with separate operation-scoped handles for subject, actor, recipient, source resource, resource set, consent revision, policy revision and idempotency key, plus a tenant/purpose/epoch-keyed reference. The companion `operation-binding-v2.schema.json` defines the JSON envelope. It does **not** generate CSPRNG values, compute HMACs, verify SD-JWT, verify BBS, validate issuer signatures, check revocation status, or establish unlinkability. The v2 schema and validator are a structural input contract, not a production cryptographic implementation. The runtime receipt verifier in `qualification_receipt.rs` defines separate FHIR-server and Mycelix-adapter profiles. The server-only profile binds HAPI, PostgreSQL, the test harness and seven required probe families without implying a Mycelix adapter was present; the adapter profile additionally requires that adapter's own build/configuration identity. Both require a read-only SQL observer boundary, expiry, an explicit RFC 8785 JCS canonicalization identifier and an injected issuer-signature verification boundary. Missing signature or stale/incomplete evidence defers; failed probes, mutation-capable observers or invalid signatures reject. The clinical purpose is represented by an operation-scoped opaque reference rather than plaintext in the exported envelope; an authorized policy verifier must resolve the claim through a trusted issuer path. The v1 `OperationBinding` still exists for compatibility; new v2 integrations should use the typed envelope and a reviewed issuer/key verifier. BBS remains experimental.

The 23-case `privacy-scenarios-v1.json` identifies dictionary attacks, stable-handle reuse, cross-tenant linkage, key rotation/revocation, unauthorized re-identification, over-disclosure and metadata side channels. Most require an external key registry, presentation verifier or independent attack harness; do not report the structural validator as passing those tests.

The `assurance-benchmark-mapping-v1.json` composes the work with existing #288 longitudinal evaluation, #334 authority semantics, #343/#346 commit and recovery, #338 causal evidence, #356/#371 FHIR qualification, and #373 privacy. It explicitly forbids a single composite score from masking failures in safety gates.

## Run

The crate has no third-party dependencies and is excluded from the large Holochain workspace.

    cargo fmt --manifest-path research/clinical-transaction-proof-graph/Cargo.toml -- --check
    cargo test --manifest-path research/clinical-transaction-proof-graph/Cargo.toml --locked
    cargo clippy --manifest-path research/clinical-transaction-proof-graph/Cargo.toml --locked -- -D warnings

The synthetic authenticator only checks deterministic fixture strings. It does not verify real signatures or hashes. The qualification registry is also a test fixture. These tests validate graph logic only.

## Current verifier tests (72 Rust unit tests; current full-head pass pending)

- valid flow with deliberately out-of-order wall-clock observations;
- explicit authenticated-subject binding and spoofed-identity rejection;
- explicit Permit / Deny / Defer authority outcome; Deny and Defer cannot proceed to execution;
- commit-unknown and delivery-unknown reconciliation on the same attempt;
- parent digest mutation and missing parent evidence;
- cross-patient binding mismatch;
- expired or self-issued component qualification;
- unresolved commit outcome;
- sender acknowledgement incorrectly presented as recipient effect;
- missing recipient effect witness, wrong idempotency key/attempt and incorrect resource-set witness;
- causal cycle, sequence gaps, and same-actor sequence forks lacking a direct causal parent;
- duplicate/misplaced authority-decision rejection;
- 10,000-event synthetic graph verification as a scale smoke test;
- conflicting terminal commit outcomes and duplicate recipient effect witnesses;
- unrelated later success cannot erase an earlier unreconciled unknown commit/delivery result;
- invalid signature fixture;
- scoped-reference metadata and operation/tenant/purpose/key-epoch validation;
- v2 operation binding rejects cross-operation substitution, duplicate role handles, missing policy context, unsupported schema version, malformed scope IDs and zero key epochs.
- verified commit rejection without delivery.

## Next evidence gates

1. Add canonical serialization and a reviewed cryptographic verifier (do not roll a hash or signature primitive by hand).
2. Bind FHIR R4 Provenance and AuditEvent references to source version and event graph.
3. Feed frozen outputs from PR #344, PR #346, PR #355 and PR #357 through the same verifier rather than creating parallel fixtures.
4. Add a real durable database adapter and synthetic recipient sandbox with crash/retry/lost-ack tests.
5. Scale to 10k-event graphs and report p50/p95/p99 verification time, graph size, memory, false deferrals and evidence acquisition cost.

## Claim boundary

This experiment does not demonstrate that a real FHIR server, Holochain DHT, database, OAuth identity provider or recipient system satisfies these contracts. It makes no PHI, HIPAA, 42 CFR Part 2, clinical-safety, regulatory-certification or production-readiness claim.


## Runtime receipt probe profile update

The server-only runtime receipt now requires eight probes: CapabilityStatement, multi-resource transaction atomicity, conditional replay, deterministic stale If-Match, concurrent version conflict, 128-way concurrent conditional create, transaction rollback, and the direct read-only PostgreSQL observer. A receipt missing any one of these remains `Defer`; the receipt must not qualify a HAPI build on the strength of only a subset of transaction probes.


## Runtime receipt cryptographic verification profile

`runtime-receipt-crypto-profile-v1.json` specifies the intended implementation boundary without pretending it is already built. It freezes the signed-payload projection, canonicalization/signing contract, issuer/key resolution, audience binding, validity and revocation policy, key rotation/compromise semantics, dependency candidates, implementation gates and required known-answer tests.

The chosen research direction is RFC 8785 JCS canonical bytes signed directly with Ed25519, with SHA-256 retained as a content digest. The signer must sign the **entire canonical payload projection** (excluding only the detached signature and its own digest) so the issuer, key, audience, runtime identities, observer permissions and probe outcomes are all integrity-bound. Duplicate JSON keys must be rejected before typed deserialization. The production verifier must use strict Ed25519 verification and an independently trusted issuer/key registry; an issuer/key ID supplied in the receipt is not a trust anchor.

Candidate Rust dependencies are `serde_json_canonicalizer 0.4.1` and `ed25519-dalek 3.0.0`, subject to exact Cargo.lock generation, policy review and cross-language test vectors. `verify_strict` is required rather than permissive verification; batch verification is not an automatic substitute because weak-key checks differ. Keep the BBS path experimental; its W3C cryptosuite is still a Candidate Recommendation Draft.

The 20-case `runtime-receipt-crypto-adversarial-v1.json` is an expected-outcome corpus, not a set of passed tests. The current injected `FixtureAuthenticator` still does not perform these operations. No real issuer signature, JCS digest or key lifecycle verification is claimed until the dependencies, key registry and tests exist.


## Exact-head validation status (2026-11-27)

- Workflow #51 checked out 5ba53772b87e9e9f15a18899d730e7bfeb7ed6b7 with Rust 1.96.0. JSON preflight, rustfmt, 67 unit tests, doc tests and Clippy with -D warnings passed. That run predates the later scoped receipt registry, eighth required runtime probe and v2 privacy references.
- Workflow #57 failed only at rustfmt (five formatting differences) before tests/Clippy ran. The first formatting correction was committed as 67ab45f86c83c12486855b8643098d40e8faf43f.
- Workflow #58 failed only at rustfmt (two remaining formatting differences) before tests/Clippy ran. The final formatter output was applied in 1565e9808b5d85f607965a9c0bdb4339432fd8ac.
- Workflow #60 was cancelled before a complete exact-head validation result was produced.
- Workflow #62 (run 46047557659; checked-out commit 42bae0c9966a429a6456c9f7dd991da972fd1c72) passed schema preflight and rustfmt, then ran the tests: **71 passed, 1 failed**. The one failure was an outdated positive-receipt assertion expecting seven verified probes after the contract had intentionally been raised to eight. No test failed in graph causality, privacy-scoping or receipt rejection controls.
- Commit bbc0687b921111760418552ef9bf7fa054c039e1 corrects that fixture to expect eight. A fresh exact-head 72-test + Clippy run is required; no 72-test pass is claimed yet.
- The current source contains 72 Rust unit tests: 32 core proof-graph tests, 16 scoped-reference tests and 24 runtime-receipt/registry tests. That is a source inventory, not a passed test result.

The current proof verifier remains a structural model. Its FixtureAuthenticator is not production cryptography; real RFC 8785 canonicalization, strict Ed25519 verification, trusted issuer/key lifecycle, Holochain/FHIR integration, Mycelix commit-time consent fencing, distributed recovery and independently signed recipient evidence remain unimplemented or unqualified.
