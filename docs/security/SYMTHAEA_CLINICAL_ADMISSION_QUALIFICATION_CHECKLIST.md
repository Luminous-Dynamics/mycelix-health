# Symthaea Clinical Admission Preflight V1 — Qualification Checklist

Passing this checklist establishes only that exact canonical Symthaea inference artifacts are admitted according to exact Mycelix deployment policy. It does not establish clinical authority.

## Policy shape

- [ ] unknown policy schema version fails closed;
- [ ] unknown wire/envelope version fails closed;
- [ ] empty policy/artifact/operation/subject-namespace fields fail closed;
- [ ] zero engine/model/schema/lineage/runtime/config digests fail closed;
- [ ] allowed claim kinds are non-empty and unique;
- [ ] allowed evidence stages are non-empty and unique;
- [ ] clinical-decision-support policy requires a subject namespace;
- [ ] maximum inference age is positive and bounded;
- [ ] maximum future skew is non-negative and bounded;
- [ ] policy digest is deterministic and domain-separated.

## Exact lineage admission

- [ ] engine substitution fails;
- [ ] model/version substitution fails;
- [ ] input-schema substitution fails;
- [ ] output-schema substitution fails;
- [ ] training-lineage substitution fails;
- [ ] evaluation-lineage substitution fails;
- [ ] calibration-lineage substitution fails;
- [ ] runtime substitution fails;
- [ ] configuration substitution fails;
- [ ] operation substitution fails;
- [ ] claim-kind outside policy fails;
- [ ] evidence-stage outside policy fails;
- [ ] intended-use substitution fails;
- [ ] subject-namespace substitution fails.

## Execution binding

- [ ] every attached evidence digest must occur in execution inputs;
- [ ] post-hoc evidence attachment fails;
- [ ] subject-binding evidence must occur in execution inputs;
- [ ] post-hoc subject binding/rebinding fails;
- [ ] generation-before-execution fails;
- [ ] future-dated inference beyond skew fails;
- [ ] stale inference fails.

## Calibration / missingness / distribution preflight

- [ ] required calibrated inference must declare `Calibrated`;
- [ ] required model calibration evidence must be present;
- [ ] inference/model calibration evidence must match exactly;
- [ ] critical missing evidence blocks clinical-decision-support admission;
- [ ] producer-declared OOD blocks clinical-decision-support admission;
- [ ] producer-declared in-distribution does not satisfy Mycelix distribution assurance.

## Qualification ceiling

- [ ] admission ceiling is chosen by Mycelix policy, never derived from Symthaea evidence stage;
- [ ] `ReplicatedClinicalEvidence` cannot self-promote the ceiling;
- [ ] ceiling type has no `SupervisedClinical` variant;
- [ ] successful admission cannot mint `ClinicalPresentationPermit`;
- [ ] preflight digest remains evidence only.

## Cross-stack gates still required

- [ ] independent Symthaea wire verifier qualified;
- [ ] independent Mycelix distribution/OOD assessment qualified;
- [ ] evaluator trust receipt qualified;
- [ ] DNA-rooted evaluator admission qualified;
- [ ] revocation snapshot boundary preserved;
- [ ] trusted runtime/conductor adapter independently qualified;
- [ ] human/practitioner authority independently qualified;
- [ ] final clinical-promotion composition independently qualified.

## Promotion rule

Keep this tranche draft until exact-head CI passes and the adversarial gates above execute. Do not treat admission preflight as clinical effectiveness, clinical validation, or presentation authority.
