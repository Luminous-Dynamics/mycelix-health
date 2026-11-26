# Gateway Operation Ledger v1

**Status:** typed coordinator/store/adapter contract plus deterministic unit tests, stacked on the fail-closed route authorization work in [PR #329](https://github.com/Luminous-Dynamics/mycelix-health/pull/329). There is no production durable store or EHR adapter wired by this increment.

## Why a route guard is not enough

A route guard proves that a caller may initiate an operation at request time. It does not prove the remote EHR or Holochain side effect committed, nor whether a timed-out request actually changed remote state. Repeating a request after a lost response can duplicate work. An in-memory Map also disappears on process restart and cannot arbitrate concurrent gateway instances.

The operation ledger is a separate state machine from authorization. The route guard authenticates and authorizes; the operation ledger binds an execution attempt and its outcome; the external adapter establishes what happened remotely.

## Identity and request binding

`deriveGatewayOperationKeyV1` hashes a domain-separated tuple of:
- authenticated tenant;
- authenticated principal binding;
- opaque patient binding;
- registered EHR connection;
- action;
- client Idempotency-Key.

The raw caller key is never used as a global database key. Every operation also carries a SHA-256 request digest and resource-set digest. A durable store must compare the complete scope and digest on every duplicate. Reusing a key with a different request body or authority scope is a conflict, not a replay.

The request digest must be computed over canonical immutable request data by trusted gateway code. Do not hash arbitrary non-canonical JSON text or allow the client to choose the digest.

## State machine

```text
accepted/absent
      |
      | atomic claim
      v
  executing
   |   |   \
   |   |    \ timeout, crash, lost acknowledgement
   |   |     v
   |   |  outcome_unknown
   |   |     |       \
   |   |     |        \ no proof / adapter unavailable
   |   |     |         -> remain unknown (defer)
   |   |     |
   |   |     +-- committed proven -> committed
   |   |     +-- not-applied proven -> failed_known_no_effect
   |   |
   |   +-- remote success + durable receipt -> committed
   +------ explicit proof no effect -> failed_known_no_effect
```

A committed record is immutable and duplicate requests return its original receipt. An executing record is never reclaimed solely because a worker lease expires. An outcome-unknown record is reconciled; it is never treated as a fresh operation.

A failed-known-no-effect record may be claimed again with the same idempotency key and request digest, but only after a fresh authority check. This state is reserved for explicit proof that no external side effect occurred.

## Execution protocol

1. The route authorization provider resolves the authenticated tenant/principal, patient binding, connection, purpose/recipient and action. Callers cannot select those authority fields.
2. Trusted gateway code computes the exact request digest and scoped operation key.
3. The durable store atomically inserts or finds the operation row. Same key/different digest is conflict; committed returns replay; executing returns in-progress; outcome-unknown returns defer.
4. A unique claim is required before side effects.
5. The operation coordinator revalidates current Consent/revocation authority immediately before the external call. Denial or authority unavailability prevents the side effect.
6. The external adapter receives the same operation key as remote Idempotency-Key when the remote system supports that mechanism.
7. A completed external result is persisted with receipt ID and result digest. A store-write failure after remote success leaves the operation unresolved; it does not permit a fresh execution.
8. If outcome is unknown, use a remote status/reconciliation API when available. If only same-key idempotent replay is available, revalidate authority before resubmitting the same key because the original request may never have arrived.
9. If neither remote idempotency nor a reliable lookup exists, the operation remains unknown for operator reconciliation. Do not blindly retry.

## What the interfaces mean

- `GatewayOperationStoreV1` must be backed by durable transactional storage with unique scoped keys and compare-and-swap/version checks. The unit tests use a test-only Map implementation; that is not production storage.
- `GatewayOperationReauthorizerV1` must query current authority and create a fresh short-lived receipt.
- `GatewayExternalOperationAdapterV1` advertises remote idempotency and reconciliation capabilities explicitly. A capability cannot be inferred from HTTP 200 or a transport library retry.
- `GatewayOperationCoordinatorV1` orchestrates the above and returns committed/replayed/denied/deferred outcomes. It has not yet been wired into FHIR/sync routes because no production durable store or adapter capability provider has been configured.

## Tests added

The Vitest suite covers:
- committed result replay and one external call;
- request-digest collision;
- operation-key scope binding across tenant/principal/patient/connection/action;
- current authority recheck immediately before side effect;
- unknown outcome blocks fresh execution;
- lookup-based reconciliation;
- same-key remote replay with fresh authorization;
- revoked grant blocks a possible first delivery on replay;
- no adapter capability leaves unknown outcome unresolved;
- ledger outage prevents execution;
- known-no-effect result allows a later same-key claim only after another authorization check;
- fresh receipt digest is stored after same-key remote replay;
- 32 concurrent duplicates yield one external execution;
- ledger completion-write failure leaves the operation in progress pending reconciliation.

These are state-machine/reference tests, not production durability tests. The fake store is deliberately confined to the test file.

## Production work still required

- Choose and migrate a durable backing store (PostgreSQL transaction/unique constraint is one option; do not use process memory).
- Implement a production store with atomic insert-or-read, claim, CAS state transitions, lease observation, durable audit intent and replay behavior.
- Wire the coordinator into FHIR Bundle ingestion, synchronous pull, bidirectional sync and conflict resolution.
- Require worker-time authorization for any background task before it is re-enabled.
- Implement EHR-specific remote idempotency/status lookup capabilities and prove them with fault injection against sandbox systems.
- Bind each FHIR bundle resource to patient identity, source version and resource digest; a Bundle-level allow is not blanket permission for all entries.
- Emit separate receipts for request acceptance, EHR operation, Holochain ingestion, audit persistence and reconciliation.

No claim of exactly-once execution across a remote EHR and Holochain DHT follows from these interfaces or tests. Where atomic cross-system commit is unavailable, the honest guarantee is at-most-one gateway attempt per durable key plus explicit unknown-outcome reconciliation.