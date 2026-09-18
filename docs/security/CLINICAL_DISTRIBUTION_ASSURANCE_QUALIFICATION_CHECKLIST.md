# Clinical Distribution Assurance V1 — Qualification Checklist

Passing these checks establishes only the software properties named below. It does not establish clinical effectiveness, regulatory status, detector validity, or trusted detector execution.

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

## Promotion/preflight gates

- [ ] deterministic/rule-backed supervised capsules preserve the existing presentation-permit path;
- [ ] model-backed `SupervisedClinical` capsule cannot obtain a permit through `evaluate_for_clinical_presentation`;
- [ ] `NotRun` remains distinct and blocked;
- [ ] `Unavailable` remains distinct and blocked;
- [ ] `Indeterminate` remains distinct and blocked;
- [ ] `OutOfDistribution` remains distinct and blocked;
- [ ] valid structural `InDistribution` still passes through all pre-existing missing-data, evaluation-state, and human-review checks;
- [ ] rejected human review blocks even with structurally valid in-distribution evidence;
- [ ] critical missing data blocks even with structurally valid in-distribution evidence;
- [ ] indeterminate clinical assertion blocks even with structurally valid in-distribution evidence;
- [ ] structurally valid `InDistribution` with no other blocker returns `BlockedModelRequiresTrustedDistributionAdmission`;
- [ ] structural distribution preflight never returns a `ClinicalPresentationPermit`.

## Adversarial structural gates

- [ ] replay one distribution assessment against a changed model output -> reject;
- [ ] replay against another patient/subject -> reject;
- [ ] reuse a detector result after model-version change -> reject;
- [ ] use a policy-mismatched detector with a syntactically valid result -> reject;
- [ ] change reference population while keeping detector constant -> reject;
- [ ] change reference-domain digest while keeping label constant -> reject;
- [ ] future-date assessment beyond allowed skew -> reject;
- [ ] race/replay old in-distribution assessment after capsule supersession -> reject because capsule binding changes;
- [ ] model-backed caller attempts legacy gate bypass -> blocked without permit.

## Required trusted-admission child tranche

Before any model-backed output can become clinician-presentable:

- [ ] define deployment-scoped detector/evaluator trust policy;
- [ ] create non-serializable verified evaluator admission from independently verified configuration/signature/registry/DHT evidence;
- [ ] bind admission to exact detector artifact and exact distribution policy digest;
- [ ] enforce validity window and append-only revocation/supersession semantics;
- [ ] qualify exact structural assessment under one active evaluator admission;
- [ ] emit a non-serializable distribution trust receipt bound to assessment + policy + admission evidence;
- [ ] future model-backed presentation API accepts the trust receipt, never raw serializable assessment/policy objects;
- [ ] runtime/DHT/config qualification proves an untrusted caller cannot choose its own trust root;
- [ ] modified-adapter tests prove raw `InDistribution` JSON cannot mint a presentation permit.

## Cross-repository Symthaea admission follow-up

Before Symthaea outputs can use the trusted path in a deployment:

- [ ] define a versioned Symthaea inference wire contract independent of Rust crate dependency identity;
- [ ] validate accepted envelope schema versions explicitly;
- [ ] pin accepted Symthaea engine/model lineages under deployment policy;
- [ ] preserve calibration evidence, OOD evidence, missingness, alternatives, execution identity, and subject binding;
- [ ] prove stronger Symthaea evidence labels cannot self-promote Mycelix qualification level;
- [ ] handle execution nonce/replay semantics explicitly;
- [ ] conductor/integration tests prove a modified adapter cannot mint a supervised permit without trusted distribution admission.

## Promotion rule

Keep this tranche draft/experimental until exact-head CI passes and the structural adversarial gates are demonstrated. Even after that, model-backed clinician presentation remains intentionally blocked until the trusted-admission child tranche and its runtime qualification are complete.
