# Medication Safety Assessment Receipt v1

## Purpose

`MedicationSafetyClearance` is intentionally only produced when the medication-safety policy fully clears. That is correct for ordinary activation, but emergency/manual care also needs durable evidence for a non-cleared result.

This tranche adds `mycelix-medication-safety-assessment`, which reruns the canonical v1 medication-safety evaluator and emits an evidence-bearing receipt for **all** outcomes:

- `Cleared`
- `RequiresReview`
- `Blocked`
- `Indeterminate`

The primary use is to make later emergency/manual override evidence precise. An override must be able to say which exact medication artifact, patient-context snapshot, safety policy, supplied safety checks, and typed reasons produced uncertainty or review requirements.

## Trust boundary

A serialized `MedicationSafetyAssessmentReceiptV1` is evidence, not authority. Deserializing JSON cannot construct `VerifiedMedicationSafetyAssessment`.

`VerifiedMedicationSafetyAssessment` is produced only by `evaluate_with_assessment_receipt(...)`, which executes the existing coverage-aware evaluator against in-process verified inputs and then hashes the resulting receipt in the dedicated `MedicationSafetyAssessment` digest domain.

This does not make an external drug database trustworthy. Evaluator/knowledge admission remains the separate medication-safety source-trust layer.

## Receipt contents

The v1 receipt binds:

- exact `MedicationRequestArtifact` digest;
- exact medication-safety context digest;
- exact medication-safety policy digest;
- every supplied safety-check kind and evaluation digest;
- final four-state decision;
- typed decision reasons;
- assessment time.

Missing checks are represented by typed `MissingCheck(...)` reasons rather than fabricated check digests. The same applies to missing context.

## Safety invariants

1. `Cleared` cannot contain warning/failure reasons.
2. A non-cleared receipt must explain why it did not clear.
3. Supplied check kinds cannot be duplicated in a receipt.
4. All digests must have the expected domain.
5. Receipt identity is domain-separated from individual check evidence, clearance, trust admission, and medication activation.
6. The receipt does not itself authorize medication activation.

## Emergency-care direction

The next layer should consume `VerifiedMedicationSafetyAssessment`, not a serialized receipt or free-form statement. Initial emergency override policy should allow only `Indeterminate` and carefully scoped `RequiresReview` outcomes. `Blocked` represents known-danger evidence and should remain non-overridable in the v1 emergency path; if a future break-glass pathway is clinically required for a known contraindication, it should receive a separate dual-authority design rather than weakening this boundary.

## Non-claims

- This receipt does not establish clinical correctness of an evaluator.
- It does not establish institutional trust in a knowledge source.
- It does not authorize a clinician.
- It does not itself make an emergency override valid.
- It does not convert legacy CDS `Safe` responses into qualified evidence.
