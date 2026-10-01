# Medication Dispense State v1

Status: experimental / draft qualification

## Purpose

`mycelix-medication-dispense-state` is the canonical read-side reducer for the new evidence-bearing dispense lineage. It consumes already DHT-valid medication-dispense authorizations, finalized dispense records, and allocator-equivocation evidence and derives a deterministic view without collapsing provenance into a mutable `filled`/`active` boolean.

It is not a DHT integrity substitute and it does not infer global absence from local absence.

## Core invariants

1. Only a finalized `MedicationDispenseReceipt` consumes a refill slot.
2. A `MedicationDispensePreflightReceipt` is evidence only and never consumes a refill slot.
3. Multiple publication actions carrying the exact same semantic final receipt are idempotent provenance, not multiple fills.
4. Different semantic final receipts for the same medication + activation + slot are an explicit `Conflict`.
5. Conflicting allocator authorizations remain visible even before physical finalization.
6. Retry-equivalent authorizations for the same final receipt and same grantee are not treated as equivocation merely because authorization IDs or validity windows differ.
7. Allocator-equivocation evidence is rechecked at the view boundary; evidence naming retry-equivalent authorizations fails closed.
8. A refill authorization for slot N > 0 must close over the exact finalized N-1 record in the supplied read set.
9. A later finalized slot without structurally present earlier finalized slots is incomplete history, not a clean refill count.
10. Conflict never has a last-write-wins winner.

## Output model

A medication/activation lineage contains ordered slot views. Each slot is one of:

- `Finalized` — exactly one semantic finalized dispense and no visible conflict;
- `AuthorizationEvidenceOnly` — release authority exists but no finalized dispense is present in the supplied read set;
- `Conflict` — incompatible authorizations, allocator-equivocation evidence, and/or multiple semantic finalized receipts are visible.

The lineage also carries `contiguous_finalized_through`, the highest contiguous conflict-free finalized slot beginning at slot zero. A conflict may be structurally present for gap detection, but it never extends this conflict-free prefix.

## Complete-for-purpose read-set requirement

The reducer does not query Holochain. Callers must materialize a complete-for-purpose read set for the medication/activation lineage they intend to evaluate.

In particular:

- a finalized record whose authorization is missing is rejected as orphaned;
- a refill authorization whose predecessor finalization is absent is rejected as an incomplete predecessor read set;
- a finalized slot appearing after a structural gap is rejected as incomplete finalized history.

These checks do not prove global completeness. They prevent a known-partial supplied set from being silently interpreted as complete.

## Equivocation semantics

For one semantic refill slot, two allocator authorizations conflict when either:

- their final semantic receipt digests differ; or
- their grantees differ.

Changes only to authorization ID or validity window are treated as retry/reauthorization metadata when the exact same grantee is authorized to publish the exact same final receipt.

This semantic is intentionally stricter than treating any unequal serialized authorization as equivocation. The read model rejects an equivocation record that does not name genuinely conflicting release authority.

The current medication-dispense integrity zome must be aligned to this same definition before qualification; until then the reducer provides the safer read-side boundary but does not erase the validator mismatch.

## Privacy

The reducer operates on privacy-minimized attestation identities. It does not need patient name, medication name, diagnosis, clinical notes, or raw safety evidence. Those remain referenced by domain-separated digest lineage.

## Non-claims

This reducer does not:

- establish that an allocator is trustworthy;
- prevent allocator equivocation before it occurs;
- provide a linearizable global query;
- convert authorization into physical medication release;
- prove that a supplied DHT query is globally complete;
- replace conductor/integrity validation.

## Qualification targets

Before promotion beyond experimental status, require exact-head build/test evidence plus conductor/integration tests for at least:

- duplicate final publication idempotency;
- same-slot different final receipts -> conflict;
- same final receipt + same grantee retry -> no false conflict;
- same final receipt + different grantee -> conflict;
- explicit valid equivocation contaminates slot;
- over-broad/non-conflicting equivocation evidence is rejected;
- missing authorization for final receipt fails closed;
- missing predecessor read set fails closed;
- predecessor lineage mismatch fails closed;
- structural finalized-history gaps fail closed;
- deterministic ordering independent of input order;
- conflict never extends the conflict-free finalized prefix.
