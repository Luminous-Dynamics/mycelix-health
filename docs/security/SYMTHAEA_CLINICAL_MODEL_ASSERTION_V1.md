# Symthaea Clinical Model Assertion v1

## Purpose

A model prediction is not a clinical criterion state.

The existing Mycelix clinical semantics use the four-state evaluation algebra:

- `Satisfied`;
- `NotSatisfied`;
- `Indeterminate`;
- `NotApplicable`.

That is appropriate for explicit criteria, rules and workflow predicates. It is not a faithful representation of a model probability or risk estimate.

`mycelix-symthaea-clinical-model-assertion` therefore preserves a verified model output as a distinct non-authorizing object. A later, separately qualified decision-rule/threshold policy may project a model assertion into a criterion state when that mapping is scientifically and clinically justified.

## Inputs

Construction requires both:

- `VerifiedSymthaeaEvidenceContextV1`; and
- the exact `AdmittedSymthaeaInferenceV2` from which that context was derived.

The builder rechecks:

- exact wire digest;
- exact admission-policy digest;
- exact subject;
- exact statement;
- claim kind;
- evidence stage;
- intended use.

This prevents a valid evidence context from being rebound to a same-looking inference under another deployment policy.

## Supported v1 model claims

Only:

- `Prediction`;
- `RiskEstimate`.

Other Symthaea claim classes remain outside this model-assertion contract. In particular, causal, diagnostic-support and treatment-support claims should not be silently coerced into prediction semantics.

## Preserved model semantics

`VerifiedSymthaeaModelAssertionV1` retains:

- exact wire/context/admission-policy identities;
- Mycelix qualification ceiling;
- subject ID;
- model assertion kind;
- exact statement;
- evidence stage;
- population applicability;
- intended use;
- epistemic/aleatoric uncertainty;
- calibrated probability and calibration identity;
- missing evidence;
- alternative hypotheses;
- producer-reported distribution assessment;
- exact model identity and typed model lineages;
- execution operation;
- generation time;
- independently verified typed ClinicalFact evidence.

## Clinical-decision-support calibration rule

A ClinicalDecisionSupport model assertion is rejected unless:

- calibration status is `Calibrated`; and
- a calibrated probability is present.

This does not prove that the calibration is scientifically adequate; admission v2 separately binds the exact typed calibration evidence lineage. It only prevents a clinical-facing model assertion from losing numerical calibration state before a later decision policy.

## Producer OOD evidence

Producer distribution state is preserved and explicitly exposed as producer evidence only.

`InDistribution` does not imply independent Mycelix distribution trust. The separate distribution-assessment/evaluator-trust/currentness chain remains mandatory.

## No EvaluationState

This artifact has no `EvaluationState` field.

A value such as `0.72` risk is not converted to `Satisfied` merely because a model emitted it. A threshold such as `risk >= 0.70` is a separate clinical decision rule with its own population, endpoint, operating point, costs, calibration and validation requirements.

## Assertion identity

The non-serializable assertion has a domain-separated digest under:

`mycelix.health.symthaea-clinical-model-assertion.v1`

It binds the exact wire, evidence context, admission policy, model-assertion kind, subject and generation time.

## Non-claims

A verified model assertion does not establish:

- a clinical criterion outcome;
- diagnostic truth;
- causal truth;
- treatment benefit;
- independent OOD trust;
- threshold validity;
- clinical utility;
- evaluator currentness;
- practitioner authority;
- regulatory clearance;
- clinician-presentation authority.
