# Clinical Distribution Trust v2

## Purpose

`mycelix-clinical-distribution-trust-v2` makes positive evaluator runtime currentness mandatory for the future model-backed distribution trust path.

The existing v1 trust layer is preserved and reused. v2 does not weaken or duplicate its admission-time checks; it composes them with the separately qualified runtime-currentness proof.

## Proof chain

```text
validated structural OOD assessment
        +
exact structural distribution policy
        +
exact evaluator trust policy
        +
v1 verified evaluator admission
        ↓
v1 DistributionTrustReceipt
        +
VerifiedDistributionEvaluatorCurrentnessV1
        +
exact currentness policy
        ↓
DistributionTrustReceiptV2
```

A v2 receipt therefore proves more than admission-only trust: it binds the exact trust result to a live currentness proof that required both a bounded no-observed-revocation snapshot and a positive root-issued evaluator lease.

## Existing v1 checks retained

`evaluate_distribution_trust_v2(...)` first relies on v1 trust evaluation, retaining checks for:

- exact structural distribution policy;
- exact detector/evaluator identity;
- exact evaluator trust policy;
- evaluator admission active at assessment time;
- evaluator admission active at trust-evaluation time;
- future-skew bounds;
- exact external admission-evidence digest.

v2 does not reinterpret these checks.

## Mandatory currentness checks

The supplied currentness proof must:

- be produced under the exact supplied currentness policy;
- still be active at trust-evaluation time;
- bind the same detector as the evaluator trust policy;
- bind the same structural distribution-policy digest as the v1 trust receipt;
- bind the same evaluator trust-policy digest as the v1 trust receipt;
- bind the same external admission-evidence digest as the v1 trust receipt.

A mismatch on any of these identities fails closed.

## Receipt identity

`DistributionTrustReceiptV2` is non-serializable and binds:

- structural assessment digest;
- structural distribution-policy digest;
- evaluator trust-policy digest;
- external evaluator admission-evidence digest;
- exact v1 trust-receipt digest;
- exact currentness-policy digest;
- exact runtime-currentness digest;
- structural distribution status;
- assessment time;
- trust-evaluation time;
- currentness expiry.

The v2 receipt digest uses explicit binary framing under the domain:

`mycelix.health.clinical-distribution-trust-receipt-v2.v1`

## Expiry semantics

A currentness proof cannot be used at or after its `valid_until_micros` boundary.

A v2 trust receipt records that expiry. Downstream code must not treat the receipt as indefinitely reusable for presentation. Any future presentation permit must verify that its bound currentness evidence remains valid at the permit decision time or require a freshly evaluated v2 receipt.

## Relationship to v1

The original `DistributionTrustReceipt` remains useful for historical/offline/non-presentation workflows that need to represent admission-qualified evaluator trust.

It is **not sufficient** for the future model-backed clinician-presentation path.

That future path must require `DistributionTrustReceiptV2` (or a strictly stronger successor), not merely the admission-only receipt.

## Authority containment

A valid v2 trust receipt does not establish:

- that the model output is clinically correct;
- that the patient is in-distribution merely because the evaluator is trusted;
- calibration validity;
- decision-rule validity;
- missing-data sufficiency;
- clinician or practitioner authority;
- regulatory clearance;
- presentation authority;
- diagnosis or treatment authority.

It establishes only that one exact structural distribution result was evaluated under the existing trust policy while the exact evaluator runtime lineage also carried a live currentness proof.