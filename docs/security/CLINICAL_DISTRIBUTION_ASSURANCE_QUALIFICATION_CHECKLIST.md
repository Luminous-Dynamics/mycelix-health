# Clinical Distribution Assurance V1 — Qualification Checklist

Passing these checks establishes only the software properties named below. It does not establish clinical effectiveness, regulatory status, or scientific validity of a detector/model.

## Pure Rust / unit gates

- [ ] unknown assessment schema version fails closed;
- [ ] unknown policy schema version fails closed;
- [ ] missing/empty assessment and policy identities fail closed;
- [ ] malformed artifact/digest/subject/reference-population fields fail closed;
- [ ] `InDistribution`, `OutOfDistribution`, and detector-returned `Indeterminate` require detector evidence;
- [ ] numeric score/threshold must be finite;
- [ ] numeric score/threshold/direction cannot contradict explicit in/out state;
- [ ] exact model substitution fails;
- [ ] exact subject substitution fails;
- [ ] exact capsule substitution fails;
- [ ] detector substitution fails;
- [ ] reference-population substitution fails;
- [ ] reference-domain substitution fails;
- [ ] stale assessment fails;
- [ ] excessive future skew fails;
- [ ] assessment and policy digests are deterministic and domain-separated.

## Promotion gates

- [ ] deterministic/rule-backed supervised capsules preserve the existing promotion path;
- [ ] model-backed `SupervisedClinical` capsule cannot obtain a permit through `evaluate_for_clinical_presentation`;
- [ ] `NotRun` cannot obtain a permit;
- [ ] `Unavailable` cannot obtain a permit;
- [ ] `Indeterminate` cannot obtain a permit;
- [ ] `OutOfDistribution` cannot obtain a permit;
- [ ] valid `InDistribution` still passes through all pre-existing missing-data, evaluation-state, and human-review gates;
- [ ] rejected human review blocks even with valid in-distribution evidence;
- [ ] critical missing data blocks even with valid in-distribution evidence;
- [ ] indeterminate clinical assertion blocks even with valid in-distribution evidence;
- [ ] AI-qualified permit carries exact distribution-assessment and distribution-policy digests;
- [ ] capsule-only deterministic permit carries no distribution digests.

## Adversarial / integration gates

- [ ] replay one distribution assessment against a changed model output -> reject;
- [ ] replay against another patient/subject -> reject;
- [ ] reuse a detector result after model-version change -> reject;
- [ ] use an unapproved detector with a syntactically valid result -> reject;
- [ ] change reference population while keeping detector constant -> reject;
- [ ] change reference-domain digest while keeping label constant -> reject;
- [ ] future-date assessment beyond allowed skew -> reject;
- [ ] race/replay old in-distribution assessment after capsule supersession -> reject because capsule binding changes;
- [ ] model-backed caller attempts legacy gate bypass -> blocked without permit.

## Cross-repository Symthaea admission follow-up

Before Symthaea outputs can use this path in a deployment:

- [ ] define a versioned Symthaea inference wire contract independent of Rust crate dependency identity;
- [ ] validate accepted envelope schema versions explicitly;
- [ ] pin accepted Symthaea engine/model lineages under deployment policy;
- [ ] preserve calibration evidence, OOD evidence, missingness, alternatives, execution identity, and subject binding;
- [ ] prove stronger Symthaea evidence labels cannot self-promote Mycelix qualification level;
- [ ] handle execution nonce/replay semantics explicitly;
- [ ] conductor/integration tests prove a modified adapter cannot mint a supervised permit without distribution qualification.

## Promotion rule

Keep this tranche draft/experimental until exact-head CI passes and the adversarial integration gates are demonstrated. A passing unit test suite alone is not clinical qualification.
