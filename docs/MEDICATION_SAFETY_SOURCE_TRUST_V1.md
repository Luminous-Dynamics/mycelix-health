# Medication Safety Source Trust v1

Status: **experimental / qualification subject**.

## Why this layer exists

`mycelix-medication-safety` can prove that an exact set of medication-safety checks is complete, fresh, bound to an exact patient-context snapshot, and clear under an exact safety policy. That still does not answer a different question:

> Does this deployment trust the evaluator and the exact knowledge snapshot that produced each check?

This layer keeps those questions separate.

## Central invariant

A safety check is admitted only when both of these independently match the check itself:

- an active evaluator admission for the exact evaluator digest and check kind;
- an active knowledge admission for the exact knowledge-artifact digest and check kind.

Both admissions must belong to the same exact `MedicationSafetyTrustPolicy` digest, and that trust policy is itself bound to one exact `MedicationSafetyPolicy` digest.

## Trust policy

`SafetySourceTrustPolicyV1` identifies the deployment policy used to qualify medication-safety sources and binds the exact safety policy it is permitted to support.

The policy digest uses the `MedicationSafetyTrustPolicy` domain. The resulting source-trust receipt uses the separate `MedicationSafetyTrustReceipt` domain.

## Evaluator admission

A `VerifiedEvaluatorAdmission` binds:

- exact evaluator artifact/identity digest;
- allowed `SafetyCheckKind` values;
- validity interval;
- optional revocation time;
- exact trust-policy digest;
- evidence digest for the admission decision.

An evaluator admitted for one check kind is not implicitly trusted for another.

## Knowledge admission

A `VerifiedKnowledgeAdmission` binds:

- exact medication-safety knowledge artifact/version digest;
- allowed `SafetyCheckKind` values;
- validity interval;
- optional revocation time;
- exact trust-policy digest;
- evidence digest for the admission decision.

A newer, older, modified, unknown, expired, or revoked knowledge snapshot is therefore a different object and must be admitted separately.

## Source-trust receipt

`evaluate_safety_source_trust(...)` requires an exact safety-policy digest and the exact `VerifiedSafetyCheck` set. For each check it requires both an active evaluator admission and an active knowledge admission under the same trust policy.

Success produces an opaque, non-cloneable, non-serializable `SafetySourceTrustReceipt` containing:

- exact safety-policy digest;
- exact safety-source trust-policy digest;
- exact safety-check evaluation digests;
- evaluator-admission evidence digests;
- knowledge-admission evidence digests;
- trust evaluation time;
- domain-separated receipt digest.

The final medication workflow consumes this receipt together with `MedicationSafetyClearance`. The two must cover the same exact safety-evaluation digest set.

## Four independent medication activation lineages

The high-assurance activation path now preserves four different proofs:

1. **Requester identity** — the external FHIR requester resolves to the authenticated Mycelix principal.
2. **Professional authority** — that principal may prescribe this exact artifact in this jurisdiction under the expected authority policy.
3. **Clinical safety evidence** — required patient context and safety checks are complete, fresh, policy-covered, and clear.
4. **Source trust** — the exact evaluators and knowledge snapshots behind those checks are admitted under the expected deployment trust policy.

No one proof substitutes for another.

## Trust-root boundary and non-claim

This pure crate does not create institutional legitimacy from nothing. `VerifiedEvaluatorAdmission` and `VerifiedKnowledgeAdmission` are intended to be created only after a trusted adapter verifies deployment configuration, signatures, registry/DHT state, revocation, and provenance.

A process that lets an untrusted caller choose both the trust policy and the admission records can still manufacture a self-consistent local story. Production integration must therefore pin the expected trust policy at a boundary the untrusted clinical request cannot choose, such as validated DHT/configuration state or another verifier-owned authority root.

Accordingly, this PR improves composition safety and auditability but is **not yet a production trust root** by itself.

## Required follow-on integration

Before production promotion:

- define the canonical signed/DHT representation of evaluator and knowledge admissions;
- bind admission creation/revocation to authorized trust administrators at integrity validation;
- pin the active safety-source trust policy independently of request input;
- prove admission status/revocation at the same execution lineage used to authorize activation;
- add conductor-level adversarial tests against a modified coordinator;
- ensure the development CDS seed corpus cannot be admitted as `CompleteForPolicy` in production profiles;
- wire the real prescription persistence/activation path so it cannot bypass `MedicationActivationCapability`.

## Adversarial properties covered at the pure layer

The current tests reject:

- unknown evaluator;
- revoked evaluator;
- evaluator used for an unapproved check kind;
- unknown knowledge artifact;
- revoked knowledge artifact;
- evaluator admission from another trust policy;
- trust policy replay across another medication-safety policy;
- source-trust receipt replay against a different safety-evaluation set;
- stale source-trust receipts at final workflow activation.
