# Clinical Distribution Evaluator Positive Lease v1

## Problem

`clinical_distribution_trust` deliberately exposes evaluator revocation state as a bounded `NetworkBackedStableDoubleRead`. That proves only that the **observed** revocation target set remained stable while the snapshot was materialized. It cannot prove global absence of an unpropagated revocation.

Therefore:

`no revocation observed != evaluator currently trusted`

## Positive currentness primitive

This tranche adds a separate DNA integrity/coordinator pair:

- `clinical_distribution_lease_integrity`;
- `clinical_distribution_lease`.

A `ClinicalDistributionEvaluatorLeaseV1` is a short-lived positive assertion by a DNA-pinned root for one exact evaluator-admission lineage.

## Root authority

Lease v1 reuses the existing `clinical_distribution_trust.root_authorities` DNA property.

The packaged DNA keeps this root set empty. Therefore neither evaluator admission nor a positive evaluator lease can validate until a deployment deliberately repacks the DNA with root AgentPubKeys, changing the DNA hash.

v1 allows **root-issued leases only**. There is no delegated lease-issuer role.

## Exact lease binding

A lease binds:

- exact admission action hash;
- exact admission proposal digest;
- exact detector artifact identity;
- exact structural distribution-policy digest;
- exact evaluator trust-policy digest;
- lease sequence;
- exact superseded lease action, when sequence > 0;
- `valid_from`;
- `valid_until`.

The lease is valid only when all fields agree with the referenced qualified evaluator admission.

## Hard duration bound

Lease duration is hard-capped by integrity validation at **5 minutes / 300,000,000 microseconds**.

This bound is not merely a coordinator preference and is not configurable upward through DNA properties.

The lease action timestamp itself must fall inside the lease validity window. Future-dated leases are therefore rejected rather than tolerated through an open-ended clock-skew allowance.

## Admission validity containment

A lease cannot:

- begin before the evaluator admission's `valid_from`;
- outlive the admission's `valid_until`, when one exists;
- substitute the detector;
- substitute either distribution or trust policy identity.

## Immutable / append-only semantics

Lease entries cannot be updated or deleted.

Expiry, not deletion, ends their positive authority interval.

Admission -> lease links are append-only.

## Supersession lineage

Sequence zero must have no predecessor.

Every sequence > 0 must name one exact prior lease and increment its sequence by exactly one. The predecessor must:

- predate the new lease action;
- be authored by a DNA root;
- target the same admission/proposal;
- bind the same detector/policies.

The new lease cannot move `valid_from` backward.

## Read-side epistemic boundary

`materialize_distribution_evaluator_positive_lease_observation(...)` performs:

1. network-backed admission materialization;
2. network-backed lease-link read A;
3. bounded materialization of all observed lease targets;
4. exact admission/proposal/detector/policy checks;
5. network-backed lease-link read B;
6. rejection if A != B;
7. unique highest-sequence selection;
8. exact supersession-chain reconstruction;
9. rejection of competing observed lineages;
10. rejection if the selected highest-sequence lease is not valid at observation completion.

The result is explicitly:

`DistributionEvaluatorPositiveLeaseObservationV1`

with read boundary:

`NetworkBackedStableDoubleRead`

It is intentionally **not** named `CurrentEvaluator`, `TrustedEvaluator`, or similar.

## Why expired higher sequence blocks older lease reuse

If a newer observed lease superseded an older lease and later expires, the older lease is not resurrected merely because its own `valid_until` was longer.

The unique highest observed sequence must itself be currently valid.

This prevents stale-authority fallback.

## Required composition

Final runtime currentness must compose two independent observations:

```text
DistributionEvaluatorAdmissionSnapshotV1
  - exact admission
  - observed revocations
  - NetworkBackedStableDoubleRead

                 +

DistributionEvaluatorPositiveLeaseObservationV1
  - exact same admission/proposal
  - positive root lease
  - unique lease lineage
  - NetworkBackedStableDoubleRead

                 ↓

trusted runtime-currentness adapter
```

The adapter must reject any **observed** revocation and must preserve both observation boundaries.

The positive lease does not claim global absence of an unseen revocation. It limits how long positive root currentness can survive without renewal.

## Remaining runtime proof boundary

A trusted conductor/adapter still must prove that its non-serializable in-process currentness object was constructed from the exact Holochain records and cell/DNA identity rather than caller-fabricated DTOs.

Exact source-entry-definition verification remains a qualification gate for referenced admission/lease records.

## Non-claims

A valid positive evaluator lease does not prove:

- detector scientific validity;
- model effectiveness;
- patient applicability;
- absence of every globally unpropagated revocation;
- regulatory clearance;
- practitioner authority;
- clinician-presentation authority;
- diagnostic or treatment correctness.
