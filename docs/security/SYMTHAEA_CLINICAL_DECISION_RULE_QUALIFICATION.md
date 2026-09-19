# Symthaea Clinical Decision Rule v1 Qualification Checklist

## Scope

Qualifies only explicit application of one evidence-bound probability threshold rule to one verified Symthaea model assertion.

It does not qualify clinical effectiveness, a particular threshold's medical utility, independent OOD trust, practitioner authority, or clinician presentation.

## Policy integrity

- [ ] policy schema version is exact;
- [ ] rule/endpoint/population/care-setting tokens are non-empty and canonical;
- [ ] exact model identity is bound;
- [ ] prediction horizon is positive and <= implementation ceiling;
- [ ] probability threshold is finite and within `[0,1]`;
- [ ] comparator is explicit;
- [ ] intended use is explicit;
- [ ] minimum population applicability is explicit;
- [ ] allowed evidence stages are non-empty and duplicate-free;
- [ ] exact producer calibration evidence is bound;
- [ ] operating-characteristics evidence identity is non-zero and typed;
- [ ] at least one external validation/replication evidence identity is present;
- [ ] external validation identities are duplicate-free;
- [ ] missing-evidence abstention policy is explicit;
- [ ] assertion age/future-skew policy is bounded;
- [ ] qualification ceiling is explicit;
- [ ] policy digest is serializer-independent.

## Assertion binding

- [ ] exact assertion kind matches;
- [ ] exact model identity matches;
- [ ] intended use matches;
- [ ] assertion applicability meets rule minimum;
- [ ] assertion evidence stage is explicitly allowed;
- [ ] rule ceiling never exceeds assertion ceiling;
- [ ] assertion is fresh enough;
- [ ] future assertion beyond skew bound rejects;
- [ ] assertion calibration status is `Calibrated`;
- [ ] calibrated probability is present, finite and in `[0,1]`;
- [ ] uncertainty calibration evidence equals policy calibration evidence;
- [ ] model calibration evidence equals assertion calibration evidence.

## Missingness / abstention

- [ ] `RequireNone` abstains on any missing evidence;
- [ ] `IndeterminateOnImportantOrCritical` ignores only informational missingness;
- [ ] `IndeterminateOnCriticalOnly` abstains on critical missingness;
- [ ] abstention happens before numerical threshold classification.

## Numerical semantics

- [ ] `>=` exact equality triggers;
- [ ] `>` exact equality does not trigger;
- [ ] `<=` exact equality triggers;
- [ ] `<` exact equality does not trigger;
- [ ] comparator changes decision-rule policy identity;
- [ ] threshold changes policy identity;
- [ ] result digest binds exact assertion + policy + state + probability + evaluation time.

## End-to-end fixture

The integration test must obtain the model assertion through the actual chain:

```text
frozen Symthaea binary v2 vector
        -> independent Mycelix wire verification
        -> typed deployment admission
        -> actual local ClinicalFact snapshot binding
        -> evidence-context composition
        -> VerifiedSymthaeaModelAssertionV1
        -> explicit decision-rule evaluation
```

Do not construct the opaque model assertion directly in the decision-rule tests.

## Adversarial tests

- [ ] exact probability with `>=` vs `>` produces different states and identities;
- [ ] model substitution rejects;
- [ ] calibration evidence substitution rejects;
- [ ] evidence-stage substitution rejects;
- [ ] insufficient population applicability rejects;
- [ ] rule ceiling above assertion ceiling rejects;
- [ ] stale assertion rejects;
- [ ] future assertion beyond skew rejects;
- [ ] missing required validation evidence rejects policy validation;
- [ ] zero validation evidence digest rejects;
- [ ] duplicate validation evidence rejects.

## Focused Rust gates

```bash
cargo fmt --check -- crates/symthaea-clinical-decision-rule
cargo check -p mycelix-symthaea-clinical-decision-rule --all-targets
cargo clippy -p mycelix-symthaea-clinical-decision-rule --all-targets -- -D warnings
cargo test -p mycelix-symthaea-clinical-decision-rule
```

## Authority containment

- [ ] no `ClinicalPresentationPermit` dependency or constructor;
- [ ] no prescription/treatment action API;
- [ ] no diagnosis constructor;
- [ ] no conversion to `ClinicalEvidenceCapsule` in this tranche;
- [ ] no independent OOD trust claim;
- [ ] `Triggered` documentation remains numerical-rule semantics only.

## Scientific qualification still required

Exact binding of operating-characteristic and external-validation evidence does not establish their sufficiency. Before a threshold may move beyond shadow evaluation, separately evaluate endpoint definition, cohort selection, prevalence, calibration, sensitivity/specificity, predictive values, decision-curve/net benefit, subgroup behavior, missingness, transportability and independent replication with appropriate uncertainty intervals.

## Evidence policy

Branch existence, PR mergeability, static review, or queued CI are not qualification. Only exact-head gates that actually execute successfully count as software evidence.