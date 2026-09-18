# Clinical Distribution Trust V1

## Status

Draft pure-Rust trust-composition layer stacked on the structural distribution/OOD preflight from #102.

This tranche introduces deployment-scoped evaluator admission and a non-serializable trust receipt. It still does **not** prove that the deployment trust root itself is authentic; runtime/configuration/DHT qualification remains a separate boundary.

## Why this layer exists

`ClinicalDistributionAssessmentV1` can prove exact structural binding to a capsule, subject, model, detector, reference domain, and freshness policy. Because that object is serializable, it cannot by itself prove that the named detector/evaluator was actually admitted by the deployment.

This layer separates:

`structural distribution preflight`

from:

`deployment-scoped evaluator admission`

and produces a non-serializable receipt only when both lineages match.

## Trust policy

`DistributionEvaluatorTrustPolicyV1` binds:

- exact structural distribution-policy digest;
- exact detector/evaluator artifact;
- trust-policy identity;
- bounded future-skew semantics.

## Verified evaluator admission

`VerifiedDistributionEvaluatorAdmission` is deliberately non-serializable. It binds:

- exact detector/evaluator artifact;
- exact trust-policy digest;
- validity window;
- optional revocation time;
- exact admission-evidence digest.

The constructor is named `from_verified_record` intentionally: the adapter calling it is responsible for independently verifying the configuration/signature/registry/DHT evidence represented by the admission digest.

Creating the Rust value is not itself proof that the external record is authentic.

## Trust receipt

`evaluate_distribution_trust(...)` requires:

- a structurally validated distribution assessment;
- the exact structural distribution policy used for that assessment;
- a trust policy bound to that exact structural policy;
- an evaluator admission bound to that exact trust policy and detector;
- admission active both when the assessment was produced and when trust is evaluated;
- bounded future skew.

Success yields a non-cloneable/non-serializable `DistributionTrustReceipt` binding:

- exact structural assessment digest;
- exact structural distribution-policy digest;
- exact trust-policy digest;
- exact evaluator-admission evidence digest;
- exact distribution status;
- assessment time;
- trust-evaluation time;
- domain-separated receipt digest.

## Important containment

This tranche does **not** yet unlock model-backed `ClinicalPresentationPermit` issuance.

The reason is that a pure Rust adapter could still call `from_verified_record` dishonestly unless the deployment independently pins and qualifies the source of evaluator admissions.

A runtime/configuration/DHT child tranche must prove that untrusted callers cannot choose their own trust roots or fabricate admission evidence before model-backed presentation authority can be enabled.

## Threats covered here

The pure trust composition fails closed on:

- structural-policy substitution;
- trust-policy substitution;
- detector substitution;
- admission belonging to another trust policy;
- evaluator admission inactive when assessment was produced;
- evaluator admission expired/revoked before trust evaluation;
- excessive future skew.

## Non-claims

V1 does not prove:

- authenticity of the external evaluator-admission record;
- detector scientific validity;
- model clinical validity;
- representativeness of the reference population;
- regulatory clearance;
- clinical effectiveness;
- authority to present a model result to a clinician.

Those remain separate trust, scientific, regulatory, and workflow boundaries.
