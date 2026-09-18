# Clinical Distribution Assurance V1

## Status

Draft structural-preflight implementation for P1 #101. This tranche does **not** establish a trusted detector/evaluator runtime and therefore cannot by itself authorize model-backed clinical presentation.

It is not evidence of clinical effectiveness, model validity, detector validity, or regulatory clearance.

## Problem

A model can be well calibrated on its development population and still behave unpredictably when the current patient/input is outside the population or representation on which its behavior was established. A missing detector, failed detector, indeterminate detector, and explicit out-of-distribution result must therefore remain distinct from `InDistribution`.

The existing `ClinicalEvidenceCapsule` already preserves model/execution identity, evidence, missing requirements, alternatives, uncertainty, intended use, and human authority. V1 deliberately does not add an optional OOD boolean to that capsule. Instead, model-backed workflows use a separate evidence-bearing distribution assessment.

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

## Exact structural binding

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

This establishes exact structural linkage. It does **not** prove that the named detector actually executed or that deployment policy trusts the evaluator that emitted the assessment.

## Numeric detector semantics

Numeric thresholds are optional because not every distribution detector exposes a one-dimensional score.

When a numeric boundary is supplied, V1 records both the score/threshold and the direction (`HigherMeansMoreInDistribution` or `LowerMeansMoreInDistribution`). For explicit `InDistribution` and `OutOfDistribution` states, the declared state must agree with the supplied numeric boundary.

This does not prove the detector or threshold is scientifically valid. It prevents the serialized result from contradicting its own declared decision rule.

## Structural policy

`ClinicalDistributionPolicyV1` pins:

- exact detector artifact;
- exact reference-population label;
- exact reference-domain digest;
- maximum evidence age;
- maximum permitted future clock skew.

Structural validation rejects model, subject, capsule, detector, reference-population, reference-domain, stale-evidence, and excessive-future-skew substitution.

## Promotion split

### Deterministic/rule-based capsule

`evaluate_for_clinical_presentation(capsule)` preserves the existing behavior for supervised capsules without a model identity.

### Model-backed capsule: legacy path closed

If `capsule.execution.model.is_some()` and qualification is `SupervisedClinical`, the capsule-only path returns:

`BlockedModelRequiresDistributionAssessment`

A model-backed caller therefore cannot reuse the deterministic/rule presentation path.

### Model-backed structural distribution preflight

The caller may run:

`evaluate_model_distribution_preflight(capsule, assessment, policy, now)`

Valid negative states produce explicit blocked decisions:

- `BlockedDistributionNotRun`
- `BlockedDistributionUnavailable`
- `BlockedDistributionIndeterminate`
- `BlockedOutOfDistribution`

Structural substitution or stale evidence is an error, not a gate decision.

A structurally valid `InDistribution` result continues through all pre-existing clinical gates (indeterminate state, critical missing data, and human review). If those gates would otherwise succeed, V1 returns:

`BlockedModelRequiresTrustedDistributionAdmission`

No `ClinicalPresentationPermit` is minted from serializable distribution preflight evidence.

## Why `InDistribution` remains blocked

`ClinicalDistributionAssessmentV1` and `ClinicalDistributionPolicyV1` are serializable pure-Rust artifacts. A malicious in-process caller could fabricate a structurally coherent assessment using the expected model/detector/policy identities.

Therefore:

`structurally valid InDistribution != trusted detector execution`

A child trust tranche must introduce a non-serializable evaluator-admission / trust receipt rooted in deployment configuration, signed registry evidence, DNA/runtime policy, or an equivalently explicit trust boundary. Only that trusted receipt may unlock a future model-backed presentation permit.

This mirrors the existing medication-safety architecture, which separates correctness/coverage from evaluator and knowledge-source admission.

## Future trusted distribution admission

The next trust tranche should bind at minimum:

- exact structural distribution assessment digest;
- exact distribution policy digest;
- exact detector/evaluator artifact;
- exact admission/trust-policy identity;
- admission evidence identity;
- validity window and revocation state;
- exact model/capsule binding already established by this tranche;
- short freshness window;
- non-serializable trust receipt.

A runtime/DHT qualification layer must then prove that an untrusted caller cannot manufacture its own admission root.

## Research and shadow workflows

Experimental, offline-validation, and shadow-clinical capsules remain unable to obtain clinician-presentation permits regardless of distribution status. They may preserve OOD evidence for research/validation analysis.

## Threat model covered by this tranche

V1 structural preflight is designed to fail closed against:

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

It deliberately does **not** claim to prevent an in-process caller from fabricating a fresh, structurally valid assessment. That is the next trust boundary.

## Non-claims

V1 does not prove:

- that the selected OOD detector actually executed;
- that the detector/evaluator is institutionally trusted;
- that the selected OOD detector is scientifically appropriate;
- that its reference population is representative;
- that `InDistribution` means the model is clinically correct;
- that calibration generalizes to the current patient;
- that a model is clinically effective or safe;
- that a clinician should follow the model output;
- regulatory clearance or approval.

Distribution preflight is one evidence line. Trusted evaluator admission, evidence quality, calibration, intended-use validation, human review, and clinical judgment remain separate boundaries.
