# Symthaea Clinical Model Assertion v1 Qualification Checklist

## Scope

Qualifies preservation of verified probabilistic model semantics only.

It does not qualify a clinical threshold, decision rule, presentation workflow, diagnosis or treatment.

## Static contract

- [ ] input requires `VerifiedSymthaeaEvidenceContextV1`;
- [ ] input requires exact `AdmittedSymthaeaInferenceV2`;
- [ ] exact wire digest is rechecked;
- [ ] exact admission-policy digest is rechecked;
- [ ] subject, statement and semantic labels are rechecked;
- [ ] v1 supports only Prediction and RiskEstimate;
- [ ] independently verified typed ClinicalFact evidence is required;
- [ ] ClinicalDecisionSupport assertion requires calibrated probability;
- [ ] uncertainty/missingness/alternatives are preserved;
- [ ] producer distribution state is explicitly non-authoritative;
- [ ] exact model identity and typed lineages are preserved;
- [ ] no `EvaluationState` is introduced;
- [ ] no threshold/decision rule is embedded;
- [ ] no presentation or treatment authority API exists;
- [ ] output is non-serializable.

## Focused commands

```bash
cargo fmt --check -- crates/symthaea-clinical-model-assertion
cargo check -p mycelix-symthaea-clinical-model-assertion --all-targets
cargo clippy -p mycelix-symthaea-clinical-model-assertion --all-targets -- -D warnings
cargo test -p mycelix-symthaea-clinical-model-assertion
```

## Required tests

- [ ] frozen Symthaea vector traverses binary verification -> admission -> local ClinicalFact binding -> evidence context -> model assertion;
- [ ] resulting assertion remains Prediction rather than a criterion state;
- [ ] calibrated probability survives the full chain;
- [ ] typed ClinicalFact evidence survives the full chain;
- [ ] nonzero assertion identity is produced;
- [ ] same wire under a substituted admission policy cannot reuse an existing evidence context;
- [ ] unsupported claim kinds fail closed;
- [ ] ClinicalDecisionSupport prediction without calibration fails closed.

## Next semantic boundary

Any later conversion from a model assertion to `EvaluationState` must require a separate, exact decision-rule policy that binds at minimum:

- model assertion kind;
- endpoint/event definition;
- prediction horizon;
- target population;
- threshold or decision function;
- calibration/validation evidence;
- operating characteristics at that threshold;
- intended action/workflow;
- validity window/version;
- Mycelix qualification ceiling.

The decision rule must not inherit validity merely because the model assertion is verified.

## Evidence policy

Do not label the model assertion tranche qualified until exact-head focused gates actually execute and pass.
