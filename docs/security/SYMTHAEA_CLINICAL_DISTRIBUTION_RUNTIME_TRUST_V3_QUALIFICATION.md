# Symthaea Clinical Distribution Runtime Trust V3 — Qualification Checklist

## Scope

Qualify the software theorem implemented by `mycelix-symthaea-clinical-distribution-trust-v3` without upgrading it into a clinical-validity, clinical-effectiveness, presentation-authority, or regulatory claim.

A PASS means only that the exact qualified head satisfies the executable/static checks below and preserves the required prerequisite lineages.

## Q0 — exact ancestry

The qualification subject MUST contain both prerequisite histories through the explicit two-parent integration commit:

- assertion-bound Symthaea distribution assessment head: `469f3e75f6b2b2905956657ab549e7bfdd4962ec`;
- runtime evaluator-currentness head: `2c89b88f949a9d013b4f190e071a690c978cea4d`;
- explicit integration commit: `50d5ac4aca63dce737f1f2bb91a0dce530038b33`.

Required checks include the equivalent of:

- integration commit is an ancestor of the qualification subject;
- `469f3e75...` is an ancestor of the qualification subject;
- `2c89b88...` is an ancestor of the qualification subject;
- the subject's v3 crate and qualification document are exactly those at the recorded head.

A reconstructed tree with missing ancestry is not equivalent evidence.

## Q1 — workspace/build hygiene

Run against the exact subject head:

- `cargo fmt --check`;
- `cargo clippy -p mycelix-symthaea-clinical-distribution-trust-v3 --all-targets -- -D warnings`;
- `cargo test -p mycelix-symthaea-clinical-distribution-trust-v3`.

The v3 package MUST be a root workspace member. A source directory that is not compiled as a workspace member is not qualification evidence.

## Q2 — prerequisite structural packages

The exact subject MUST also retain passing focused tests for the prerequisite proof lines used by v3, including at minimum:

- `mycelix-symthaea-clinical-distribution-assessment-v2`;
- `mycelix-clinical-distribution-currentness`;
- the clinical-distribution trust integrity/coordinator packages used by the currentness line;
- the clinical-distribution lease integrity/coordinator packages used by the currentness line.

If a prerequisite package fails, v3 MUST NOT be promoted merely because its own focused tests pass.

## Q3 — exact assertion-native binding

The tests MUST demonstrate a path from the frozen Symthaea v2 wire fixture through:

1. independent wire verification;
2. Symthaea admission;
3. Mycelix `ClinicalFact` binding;
4. evidence-context composition;
5. opaque model assertion construction;
6. assertion-bound structural distribution assessment; and
7. v3 runtime trust composition.

The receipt MUST preserve the exact assertion, wire, evidence-context, admission-policy, subject, model, structural-assessment, structural-policy, runtime-policy, and currentness identities.

## Q4 — lease-before-use ordering

A positive case MUST establish:

`currentness.verified_at <= assessment.assessed_at < currentness.valid_until`

and:

`assessment.assessed_at <= trusted_at < currentness.valid_until`.

The following adversarial cases MUST fail closed:

- evaluator currentness established only after the assessment was produced;
- evaluator currentness expired before the assessment;
- evaluator currentness expired before trust evaluation;
- trust evaluation timestamp predates the assessment.

This prevents retrospective trust promotion of an assessment produced before positive runtime currentness existed.

## Q5 — policy and lineage substitution

The following substitutions MUST fail closed:

- caller-supplied evaluator trust policy binds another structural-policy bridge;
- caller-supplied evaluator trust policy binds another detector bridge;
- runtime currentness proof binds another evaluator trust-policy digest;
- runtime currentness proof binds another currentness policy;
- runtime currentness proof binds another detector identity;
- runtime currentness proof binds another structural distribution-policy identity.

The canonical detector and structural-policy runtime bridges MUST be re-derived during evaluation rather than accepted as caller assertions.

## Q6 — exact assessment identity

Two distinct valid distribution assessments for the same exact model assertion MUST produce distinct assessment digests and distinct v3 trust-receipt digests.

This specifically guards against treating "same model + same patient" as sufficient identity for OOD trust.

## Q7 — runtime/conductor evidence boundary

Unit tests that construct admission snapshots or lease observations directly are structural tests only. They MUST NOT be reported as proof that:

- a Holochain conductor enforced the DNA root configuration;
- network-backed stable double reads behaved correctly under race/reordering;
- root authorization was authentic;
- revocation observation was globally complete; or
- a positive lease existed on a real deployment.

Qualification therefore depends on the independently qualified runtime admission/lease/currentness lineages represented by the exact prerequisite head, plus their exact workflow/conductor evidence where required by those contracts.

If those prerequisite runtime lanes are still queued, failed, cancelled, stale, or bound to another head, v3 remains unqualified regardless of focused unit-test results.

## Q8 — exact-head workflow evidence

For every qualifying workflow execution record:

- exact commit SHA;
- workflow run ID;
- relevant job ID(s);
- conclusion;
- commands/gates actually executed; and
- any produced evidence artifact identity.

A workflow queued but never assigned a runner is not a PASS.
A successful run for an ancestor or successor is not evidence for the exact frozen subject unless the qualification contract explicitly re-derives and binds that subject.

Any repair after a failed run creates a new subject head and requires fresh exact-head qualification.

## Q9 — non-claims

A software PASS for this checklist does NOT establish:

- clinical validity;
- clinical effectiveness;
- model calibration for a patient or population;
- that `InDistribution` means correct, safe, or clinically appropriate;
- scientific adequacy of the OOD detector;
- global revocation completeness;
- diagnosis authority;
- treatment or prescribing authority;
- patient-facing or clinician-facing presentation authority;
- HIPAA, GDPR, POPIA, FDA, EU MDR/IVDR, or other regulatory/legal compliance;
- regulatory clearance or approval.

Those require separate evidence and authority chains.

## Promotion rule

Do not use this subject as a prerequisite for a clinician-facing workflow merely because the v3 structural checks pass.

The next permitted composition after exact-head software qualification is a **shadow-only clinical evaluation boundary** that separately binds the decision-rule result and this v3 runtime-trust receipt to the same exact assertion/context. Any clinician presentation permission remains a later, separately qualified authority step.