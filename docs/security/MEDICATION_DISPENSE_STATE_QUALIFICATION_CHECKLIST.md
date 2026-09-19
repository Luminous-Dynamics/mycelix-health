# Medication Dispense State v1 — Qualification Checklist

This checklist is intentionally stricter than source review. The reducer remains experimental until all applicable gates are evidenced on the exact subject.

## Pure reducer gates

- duplicate publication of the same semantic final receipt is idempotent;
- two semantic final receipts for one slot produce `Conflict`;
- same receipt + same grantee retry authorization does not create false conflict;
- same receipt + different grantee produces conflict;
- valid allocator-equivocation evidence contaminates the slot;
- retry-equivalent equivocation evidence is rejected;
- orphan finalization fails closed;
- missing refill predecessor fails closed;
- wrong predecessor lineage fails closed;
- later finalized slot after a structural gap fails closed;
- deterministic output is independent of input ordering;
- conflict never extends the conflict-free finalized prefix.

## Integration gates

- materializer proves its query is complete-for-purpose for the requested lineage;
- raw `AllocatorEquivocationEvidence` is never treated as semantic truth without reducer validation;
- legacy `PrescriptionFill` is not mixed into the qualified refill ledger as if it were a new finalized receipt;
- emergency-origin activation provenance remains present in upstream activation identity;
- no consumer derives a clinical decision from a naked refill count without lineage health.

## Conductor gates

- modified coordinator cannot inject malformed authorization/finalization/equivocation records;
- concurrent duplicate publication remains idempotent;
- allocator conflicting grants are observable and reduce to conflict;
- incomplete DHT materialization is surfaced rather than normalized;
- DNA-pinned allocator selection remains stable for the exact DNA properties.

## Required upstream alignment

Before promotion, the medication-dispense integrity zome's equivocation predicate must match the reducer definition: same slot authorizations conflict only when the final semantic receipt or grantee differs. Authorization ID/window-only changes are retry-equivalent.
