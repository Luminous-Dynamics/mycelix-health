# Medication Safety Evidence v1

Status: **experimental / qualification subject**. This contract does not establish clinical efficacy, regulatory clearance, or production suitability.

## Central invariant

> No finding is not the same as sufficient evidence of safety.

A medication may cross the high-assurance activation boundary only when all of the following independently hold:

1. the exact FHIR `MedicationRequest` has been projected into a strict typed `MedicationRequestArtifact`;
2. the external requester has resolved unambiguously to the authenticated Mycelix principal;
3. the principal has current professional authority for `Prescribe`, in the required jurisdiction, against the exact medication artifact;
4. a versioned medication-safety policy has been satisfied against an exact patient-context snapshot;
5. every policy-required safety check has fresh evidence with `CompleteForPolicy` knowledge coverage and a `Clear` outcome;
6. the resulting `MedicationSafetyClearance` is still fresh at activation time and is consumed when the final capability is issued.

## Decision lattice

`MedicationSafetyDecision` is intentionally not a boolean:

- `Cleared` — every required context component and safety check is present, current, correctly bound, completely covered for the policy, and clear.
- `RequiresReview` — evidence is complete enough to interpret but one or more required checks returned a warning.
- `Blocked` — at least one required check reports a blocking/contraindicating condition. This dominates missing evidence.
- `Indeterminate` — required evidence is missing, stale, future-dated, expired, partially covered, skipped, or itself indeterminate.

Only `Cleared` creates the opaque, non-cloneable, non-serializable `MedicationSafetyClearance`.

## Exact evidence bindings

A clearance binds:

- `MedicationRequestArtifact` digest (`MedicationRequestArtifact` domain);
- patient-context snapshot digest (`MedicationSafetyContext` domain);
- safety-policy digest (`MedicationSafetyPolicy` domain);
- all required safety-check evaluation digests (`MedicationSafetyEvaluation` domain);
- clearance time.

Each safety-check evaluation separately binds:

- medication artifact;
- patient-context snapshot;
- safety policy;
- exact knowledge artifact/version digest;
- evaluator identity/artifact digest;
- knowledge coverage classification;
- outcome and finding count;
- evaluation time;
- knowledge-as-of time.

The exact knowledge snapshot is identified under the separate `MedicationSafetyKnowledge` digest domain. The repository's development CDS seed tables are not treated as comprehensive production coverage.

## Context is evidence, not an empty vector

A context component must distinguish:

- `Available` — relevant evidence is present;
- `VerifiedAbsent` — an evidence-bearing process established absence;
- `NotApplicable` — an evidence-bearing applicability decision established that the domain does not apply.

An empty allergy list or medication list is therefore not automatically proof that there are no allergies or concomitant medications.

The v1 vocabulary can represent active medications, allergies/intolerances, active conditions, renal function, hepatic function, pregnancy/lactation, age, weight, and pharmacogenomics. A policy chooses which are required for a particular workflow; v1 does not claim every medication always requires every domain.

## Safety checks

The v1 vocabulary includes drug-drug interaction, drug-allergy, duplicate therapy, drug-disease contraindication, dose range, renal adjustment, hepatic adjustment, pregnancy/lactation, age/weight, and pharmacogenomic checks.

The kernel does **not** implement a proprietary drug database. Qualified adapters may evaluate these checks against external or local knowledge systems, but the exact knowledge artifact/version and coverage claim must be carried as evidence.

## Workflow composition

The high-assurance medication capability path is:

`MedicationRequestArtifact`
+ `MedicationSafetyClearance`
+ expected `MedicationSafetyPolicy` digest
+ `AuthorityPermit(Prescribe)`
+ workflow policy
+ authenticated principal
+ jurisdiction
-> `MedicationActivationCapability`

The safe API rejects:

- clearance for another medication artifact;
- clearance produced under another safety policy;
- stale/future clearance;
- stale/future requester resolution;
- wrong authenticated principal;
- wrong authority purpose;
- stale/future professional authority;
- authority for another artifact or policy.

The final capability preserves requester-resolution evidence, professional-authority evidence, safety context, safety policy, and check-evaluation digests as separate lineages.

## Known limitation: evaluator/knowledge admission

`VerifiedSafetyCheck` v1 establishes exact digest binding and coverage semantics, but it does not yet itself prove that a particular evaluator or knowledge provider is trusted by deployment policy. Before production promotion, add a verifier-owned evaluator/knowledge admission capability or an equivalent signed trust-policy adapter. The final deployment must not allow arbitrary callers to self-designate an evaluator or knowledge artifact as qualified.

This limitation is intentionally explicit: cryptographic identity of evidence is not the same thing as institutional or clinical trust in that evidence.

## Legacy CDS migration

The current CDS `SafetyAssessment::Safe` path must not be accepted as a `MedicationSafetyClearance`. Existing CDS responses can remain useful as legacy/user-facing information, but the strict boundary requires coverage-aware evidence. Issue #46 tracks migration/hardening of the legacy assessment semantics.

## Qualification requirements

Before promotion beyond experimental status, require at minimum:

- exact-head CI for all workspace crates;
- adversarial tests for skipped checks and partial/unknown knowledge coverage;
- stale/future context and evaluation tests;
- cross-medication, cross-context, and cross-policy replay tests;
- provider/requester identity substitution tests;
- evaluator/knowledge trust admission tests;
- negative tests against the development seed corpus being labeled comprehensive;
- integration tests proving legacy prescription/CDS paths cannot bypass the final capability boundary;
- clinical and regulatory review of intended use and policy profiles.
