# Clinical Workflow Capabilities v1

## Purpose

A valid credential, authority decision, or qualified clinical assertion must not become a reusable role token.

The v1 workflow boundary converts those upstream results into **single-owner, exact-target capabilities** for specific clinical workflow steps.

## Central invariant

> A valid permit for the wrong artifact, purpose, policy, principal, jurisdiction, or time is invalid.

## Inputs checked at capability conversion

The workflow layer requires and checks:

- exact artifact digest computed under the correct clinical-integrity domain;
- exact authority-policy digest;
- exact authority purpose;
- authenticated workflow principal;
- exact jurisdiction;
- bounded authority-evaluation age;
- bounded future clock skew;
- domain-specific safety state (for example, active medication-order semantics or supervised clinical qualification).

The upstream `AuthorityPermit` is consumed by the conversion function. The resulting workflow capability implements no `Clone`, `Copy`, `Serialize`, `Deserialize`, or `Debug`.

## Clinical presentation

`authorize_clinical_presentation` requires:

- `AuthorityPurpose::ClinicalReview`;
- authority bound to the exact evidence-capsule digest;
- the evidence capsule to pass the existing supervised-clinical qualification gate;
- matching principal/jurisdiction/policy/freshness.

This composes evidence qualification and human professional authority rather than letting either stand alone.

## Medication activation

`authorize_medication_activation` requires:

- the exact `MedicationOrder` to pass `validate_active_order_candidate`;
- `AuthorityPurpose::Prescribe`;
- authority bound to the exact medication-order digest;
- matching principal/jurisdiction/policy/freshness.

A clinical-review permit cannot be replayed as prescribing authority, and prescribing authority for order A cannot activate order B.

This still does not authorize dispensing or administration. Those are separate authority purposes and should receive separate workflow capabilities.

## Quarantine resolution

`authorize_quarantine_resolution` requires:

- an exact `UnderReview` quarantine event;
- `AuthorityPurpose::ClinicalReview`;
- authority bound to that event's exact digest;
- one exact terminal disposition (`Promote` or `Discard`).

The returned `QuarantineResolutionCapability::resolve` consumes itself and re-hashes the event before creating the transition. If case identity, sequence, or event bytes changed after authorization, resolution fails closed.

## Freshness

`WorkflowPolicyV1` is itself domain-separated and hashed.

v1 allows deployments to choose stricter values but refuses policies that permit:

- authority older than 24 hours; or
- future clock skew above five minutes.

High-impact workflows such as prescribing should normally use substantially shorter authority lifetimes.

## Canonical artifact identity

The initial v1 adapters hash explicit schema-tagged compact JSON encodings for their fixed Rust structures. This is intentionally versioned and treated as an internal canonicalization contract, not a claim that arbitrary JSON is canonical across languages.

Cross-language canonical serialization should be standardized separately before these digests become an external protocol contract.

## Remaining hardening

The next integration steps should:

1. make clinician-facing CDS endpoints require `ClinicalPresentationCapability`;
2. make prescription activation/signing require `MedicationActivationCapability`;
3. route quarantine terminal transitions exclusively through `QuarantineResolutionCapability`;
4. resolve FHIR requester/practitioner references to the authenticated `PrincipalBinding` rather than trusting textual references;
5. migrate legacy bare `EvidenceDigest([u8; 32])` fields toward domain-aware verified digest types;
6. add separate capability paths for dispense, administer, trial eligibility review, and research ethics review.

## Non-claims

A workflow capability proves that the configured software checks were satisfied for one exact workflow target. It does not establish clinical effectiveness, legal authority by itself, regulatory approval, correct treatment, or fitness for an intended use without the upstream evidence and external validation those policies require.
