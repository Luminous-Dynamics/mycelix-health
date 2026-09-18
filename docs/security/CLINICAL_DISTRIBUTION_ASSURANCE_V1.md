# Clinical Distribution Assurance V1

## Status

Draft implementation for P1 #101. This layer is an assurance boundary for model-backed clinical presentation; it is not evidence of clinical effectiveness, model validity, or regulatory clearance.

## Problem

A model can be well calibrated on its development population and still behave unpredictably when the current patient/input is outside the population or representation on which its behavior was established. A missing detector, failed detector, indeterminate detector, and explicit out-of-distribution result must therefore remain distinct from `InDistribution`.

The existing `ClinicalEvidenceCapsule` already preserves model/execution identity, evidence, missing requirements, alternatives, uncertainty, intended use, and human authority. V1 deliberately does not add an optional OOD boolean to that capsule. Instead, supervised model-backed presentation requires a separate evidence-bearing distribution assessment.

## Core state model

`ClinicalDistributionStatusV1` has exactly five states:

- `NotRun`
- `Unavailable`
- `Indeterminate`
- `InDistribution`
- `OutOfDistribution`

No state aliases another. In particular:

- `NotRun != InDistribution`
- `Unavailable != InDistribution`
- `Indeterminate != InDistribution`
- absence of an assessment is not evidence of in-distribution operation.

## Exact binding

`ClinicalDistributionAssessmentV1` binds:

- exact patient/clinical subject;
- exact model artifact identity;
- exact detector artifact identity;
- reference-population label;
- exact reference-domain digest;
- exact evidence-capsule binding digest;
- detector status;
- optional numeric score/threshold/direction;
- detector/output evidence digest when the detector produced a result;
- assessment time.

The capsule binding digest is domain-separated BLAKE3 over the exact serialized `ClinicalEvidenceCapsule`. Changing the statement, subject, model, execution identity, evidence, uncertainty, intended use, authority, or any other capsule field changes the binding.

## Numeric detector semantics

Numeric thresholds are optional because not every distribution detector exposes a one-dimensional score.

When a numeric boundary is supplied, V1 records both the score/threshold and the direction (`HigherMeansMoreInDistribution` or `LowerMeansMoreInDistribution`). For explicit `InDistribution` and `OutOfDistribution` states, the declared state must agree with the supplied numeric boundary.

This does not prove the detector or threshold is scientifically valid. It prevents the serialized result from contradicting its own declared decision rule.

## Deployment policy

`ClinicalDistributionPolicyV1` pins:

- exact detector artifact;
- exact reference-population label;
- exact reference-domain digest;
- maximum evidence age;
- maximum permitted future clock skew.

Validation rejects model, subject, capsule, detector, reference-population, reference-domain, stale-evidence, and excessive-future-skew substitution.

## Promotion split

### Deterministic/rule-based capsule

`evaluate_for_clinical_presentation(capsule)` preserves the existing behavior for supervised capsules without a model identity.

### Model-backed capsule

If `capsule.execution.model.is_some()` and qualification is `SupervisedClinical`, the capsule-only path returns:

`BlockedModelRequiresDistributionAssessment`

The caller must instead use:

`evaluate_model_for_clinical_presentation(capsule, assessment, policy, now)`

Only a valid `InDistribution` result can continue into the pre-existing supervised clinical gate. Other valid states produce explicit blocked decisions:

- `BlockedDistributionNotRun`
- `BlockedDistributionUnavailable`
- `BlockedDistributionIndeterminate`
- `BlockedOutOfDistribution`

Structural substitution or stale evidence is an error, not a gate decision.

## Permit binding

A model-backed `ClinicalPresentationPermit` preserves:

- exact capsule identity;
- exact subject;
- capsule issuance time;
- exact distribution-assessment digest;
- exact distribution-policy digest.

The permit remains non-serializable and cannot be reconstructed from untrusted JSON.

## Research and shadow workflows

Experimental, offline-validation, and shadow-clinical capsules remain unable to obtain clinician-presentation permits regardless of distribution status. They may preserve OOD evidence for research/validation analysis.

## Threat model covered by V1

V1 is designed to fail closed against:

- model substitution;
- patient/subject substitution;
- detector substitution;
- reference-population substitution;
- reference-domain substitution;
- assessment replay against a changed capsule;
- stale assessment replay;
- excessive future-dated assessment;
- numeric score/status contradiction;
- bypass through the legacy capsule-only supervised path.

## Non-claims

V1 does not prove:

- that the selected OOD detector is scientifically appropriate;
- that its reference population is representative;
- that `InDistribution` means the model is clinically correct;
- that calibration generalizes to the current patient;
- that a model is clinically effective or safe;
- that a clinician should follow the model output;
- regulatory clearance or approval.

Distribution assurance is one additional required proof line, not a substitute for evidence quality, calibration, intended-use validation, human review, or clinical judgment.
