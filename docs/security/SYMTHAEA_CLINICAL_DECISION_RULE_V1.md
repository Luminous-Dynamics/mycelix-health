# Symthaea Clinical Decision Rule v1

## Purpose

A calibrated probability is not a clinical criterion.

`mycelix-symthaea-clinical-decision-rule` creates an explicit, evidence-bound boundary between:

```text
VerifiedSymthaeaModelAssertionV1
        probability / uncertainty
                 +
ClinicalDecisionRulePolicyV1
        endpoint / horizon / threshold / evidence
                 ↓
VerifiedClinicalDecisionRuleResultV1
        Triggered | NotTriggered | Indeterminate
```

The result is still non-authorizing. It does not recommend an intervention or permit clinician-facing presentation.

## Why this layer exists

Without a separate rule artifact, application code could silently convert a prediction such as `0.72` into `Satisfied` or `high risk` using an undocumented threshold. That would merge two different scientific claims:

1. the model estimates a probability; and
2. a particular operating point is appropriate for a particular endpoint, population, horizon and workflow.

v1 makes claim 2 independently identifiable and reviewable.

## Rule policy

`ClinicalDecisionRulePolicyV1` binds:

- policy schema version and rule ID;
- accepted model-assertion kind;
- exact model identity and typed model lineages;
- endpoint namespace + endpoint ID;
- prediction horizon;
- target population;
- care setting;
- intended use;
- minimum population applicability;
- explicitly allowed evidence stages;
- comparator (`>=`, `>`, `<=`, `<` semantics);
- probability threshold;
- exact producer calibration evidence;
- independent operating-characteristics evidence;
- at least one external validation/replication evidence identity;
- missing-evidence abstention policy;
- assertion age/future-skew limits;
- Mycelix qualification ceiling.

The policy digest is serializer-independent and binary framed.

## Threshold evidence is not self-validating

`DecisionRuleEvidenceIdentityV1` provides exact identities for operating-characteristic and external validation artifacts.

Binding those identities prevents substitution. It does **not** by itself establish that a study is methodologically sound, representative, adequately powered, replicated, regulator-accepted or clinically useful. Those claims require separate scientific qualification.

## Calibration binding

The rule requires the assertion to be calibrated and to carry a calibrated probability.

The exact calibration evidence must agree across:

- the decision rule policy;
- assertion uncertainty;
- exact model identity.

A calibration-evidence substitution fails closed.

## Applicability and evidence stage

A rule is not allowed to treat a model evaluated only on one cohort as if it had validated target-population applicability.

The assertion's applicability must meet the explicit minimum required by the rule, and its evidence stage must be one of the rule's explicit allowed stages.

Evidence-stage admission is set membership, not an implicit claim that later enum variants are universally stronger for every clinical purpose.

## Missingness / abstention

v1 supports three explicit policies:

- `RequireNone`: any declared missing evidence produces `Indeterminate`;
- `IndeterminateOnImportantOrCritical`;
- `IndeterminateOnCriticalOnly`.

This means missing data cannot be silently ignored by threshold evaluation.

## Authority ceiling

The rule's Mycelix qualification ceiling may never exceed the ceiling of the model assertion.

A `ValidatedOffline` model assertion therefore cannot become a `ShadowClinical` decision-rule result merely because a threshold policy asks for it.

There is no clinician-presentation or treatment authority in this artifact.

## Result semantics

`VerifiedClinicalDecisionRuleResultV1` retains:

- exact model-assertion digest;
- exact decision-rule policy digest;
- domain-separated result digest;
- `Triggered`, `NotTriggered`, or `Indeterminate` state;
- subject ID;
- endpoint identity;
- prediction horizon;
- exact calibrated probability;
- comparator + threshold;
- qualification ceiling;
- evaluation time.

`Triggered` means only that the explicitly declared numerical rule evaluated true.

It does **not** mean:

- disease is present;
- diagnosis is established;
- treatment is indicated;
- benefit exceeds harm;
- the patient should be alerted;
- a clinician-facing card may be shown.

## Next composition

A future shadow-workflow projection may combine an exact decision-rule result with:

- strict typed evidence capsule v2;
- currentness-bound distribution trust v2;
- independent threshold-policy scientific qualification;
- patient/subject binding;
- missingness/calibration checks;
- intended-use policy.

Even then, clinician presentation should remain a later independently qualified boundary.