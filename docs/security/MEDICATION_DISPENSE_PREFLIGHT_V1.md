# Medication Dispense Preflight v1

## Purpose

This tranche introduces a high-assurance **dispense preflight**, not a final medication-release authorization.

A successful preflight establishes that, at one evaluation point:

- the exact typed MedicationRequest artifact is valid for activation;
- the canonical medication activation view has exactly one current lineage;
- qualified vs emergency-override provenance is preserved;
- the exact full-fill request matches the order's coded medication and typed dispense quantity;
- the candidate fill slot is within the authorized initial-fill/refill count;
- the locally reconstructed prior-final-dispense ledger is contiguous and fresh;
- the authenticated principal matches the verified pharmacy context;
- pharmacy affiliation/status evidence is present, typed, current, and non-revoked;
- the professional authority permit is for `AuthorityPurpose::Dispense`, the exact dispense request, the exact authority policy, jurisdiction, and principal;
- all relevant evidence is freshness-bounded by the exact dispense policy.

The output is `MedicationDispensePreflightCapability`, consumed into a `MedicationDispensePreflightReceiptV1`.

## Critical non-claim

A preflight does **not** prove exclusive ownership of the candidate refill slot.

Two pharmacies can observe the same valid prior-final-dispense set during a partition or propagation delay. Both can therefore independently conclude that the same next slot appears available. A local DHT snapshot is not a linearizable counter and must not be described as one.

Accordingly:

```text
MedicationDispensePreflightReceipt
    !=
MedicationDispenseReceipt
```

The domains are cryptographically distinct. A preflight receipt cannot be inserted into `PriorDispenseEvidence`; only a verified final `MedicationDispenseReceipt` may consume a refill slot in later ledger snapshots.

## Required finalization boundary

Physical medication release requires a later runtime adjudication step that consumes an exact preflight receipt and produces an exclusive, exact-target slot authorization. The final design must establish at least:

1. exact medication artifact digest;
2. exact activation semantic receipt digest;
3. exact dispense request digest;
4. exact preflight receipt digest;
5. exact slot index;
6. exact pharmacy and pharmacist principal;
7. exact adjudication policy/trust root;
8. short authorization validity;
9. replay resistance;
10. a documented partition/concurrency model.

Only after that proof may the system create a final `MedicationDispenseReceipt` that later ledger snapshots count as a consumed slot.

## Concurrency options to evaluate

The next tranche should explicitly choose and prove one model instead of assuming DHT convergence provides mutual exclusion. Candidate designs include:

- one designated per-order slot allocator whose decisions are short-lived and exact-target;
- a threshold/countersigned allocator service for higher availability and reduced unilateral authority;
- a jurisdiction/pharmacy-network allocation service with auditable append-only grants;
- another mechanism with a clearly documented safety/liveness tradeoff.

A pure Holochain validation rule based on "no competing entry is currently visible" is not sufficient because validators may have different views during partitions.

## V1 scope restrictions

V1 preflight intentionally rejects or leaves out:

- partial fills;
- product substitution/equivalence inference;
- controlled-substance special rules;
- early refill policy;
- vacation/lost-medication overrides;
- mail-order split fulfillment;
- payer adjudication;
- inventory reservation;
- final physical release.

These are separate policy/evidence problems and should not be smuggled into a generic boolean.

## Pharmacy evidence trust boundary

`VerifiedPharmacyContext` proves that an adapter supplied internally coherent, correctly domain-separated, temporally valid evidence. It does not by itself prove that the deployment trusts the underlying pharmacy registry, affiliation issuer, or status source.

Because this tranche stops at preflight, that limitation is explicit rather than hidden. Final dispensing must require verifier-owned admission of the pharmacy evidence source/policy, analogous to medication-safety source admission.

## Provenance invariant

Emergency-origin medication remains emergency-origin through preflight. The preflight receipt carries either:

- `DispenseActivationProvenance::Qualified`, or
- `DispenseActivationProvenance::EmergencyOverride`.

No high-assurance API flattens these states to `active=true`.

## Qualification

This remains experimental until exact-head CI and integration/conductor tests prove:

- conflict state cannot preflight;
- terminated/no-current activation cannot preflight;
- legacy Active status cannot substitute for canonical activation;
- emergency provenance requires explicit policy opt-in;
- `Prescribe` cannot substitute for `Dispense` authority;
- wrong pharmacist/pharmacy/policy/jurisdiction/target fail closed;
- stale/future activation, authority, pharmacy, or ledger evidence fail closed;
- non-contiguous/duplicate prior final receipts fail closed;
- a preflight receipt cannot count as a prior finalized dispense;
- full-fill quantity and coded product must match exactly in v1.
