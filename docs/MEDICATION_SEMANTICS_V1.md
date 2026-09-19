# Medication Semantics v1

## Purpose

`mycelix-medication-semantics` defines the minimum typed semantics required before a medication request may be treated as a machine-interpretable order candidate.

It does **not** authorize prescribing, dispensing, administration, substitution, or autonomous treatment.

## Why this exists

The existing Mycelix prescription and FHIR mapping layers preserve useful medication information, but several safety-significant concepts can still be represented as free text: strength, SIG, timing, maximum dose, and quantity units. Free text is valuable for display and patient instructions, but it is not a safe computational identity.

Medication semantics therefore follow the same rule established by the Clinical Semantics Kernel:

> preserve narrative, but never silently promote narrative into machine-actionable meaning.

## Core model

A `MedicationOrder` binds:

- patient/subject identity;
- coded medication identity;
- optional typed product strength;
- FHIR MedicationRequest status;
- FHIR MedicationRequest intent;
- one or more typed dosage instructions for active/on-hold order-like requests;
- optional typed dispense quantity;
- refill count;
- requester reference;
- authored time when available;
- source/version provenance.

A `DosageInstruction` binds:

- optional sequence;
- narrative SIG for human display/audit;
- patient instructions;
- typed administration timing;
- coded route;
- optional coded method and site;
- typed dose quantity/range;
- optional typed administration rate;
- optional maximum dose per period, administration, and lifetime.

## Intent is not authority

FHIR MedicationRequest intent is preserved rather than collapsed:

- `proposal` -> non-authorizing proposal
- `plan` -> non-authorizing plan
- `option` -> non-authorizing option
- order-like intents -> order workflow classification depending on status

An active `intent=order` is only an **active order candidate**. It does not prove that the requester:

- is a licensed practitioner;
- holds the correct current credential;
- is acting within scope of practice;
- is authorized in the relevant jurisdiction;
- has produced a valid required signature;
- may prescribe the requested controlled substance;
- has satisfied organizational policy.

Those checks belong to a separate authority verifier. No downstream medication action should infer them from `MedicationRequest.intent` alone.

## Typed timing

v1 supports:

- one-time administration;
- scheduled administration as `frequency / UCUM-coded time period`;
- as-needed administration with a coded indication and optional minimum interval.

Scheduled frequency must be greater than zero. Time periods must be positive and use an explicit UCUM time code supported by v1.

## Typed dose and rate

Dose is represented as a `Quantity` or `Range` from the Clinical Semantics Kernel. Rate may be a `Quantity`, `Range`, or `Ratio`. Maximum dose per period uses a typed ratio whose denominator must itself be a valid time quantity.

Display-only units are insufficient for machine actionability.

## Workflow states

`workflow_class()` deliberately distinguishes:

- `NonAuthorizingProposal`
- `NonAuthorizingPlan`
- `NonAuthorizingOption`
- `ActiveOrderIntent`
- `OnHoldOrderIntent`
- `InactiveOrderIntent`
- `Draft`
- `EnteredInError`
- `Unknown`

Only `ActiveOrderIntent` may pass `validate_active_order_candidate()`, and even then external authority verification is still required.

## Safety invariants

1. Active/on-hold order-like requests require typed dosage semantics.
2. Active/on-hold order-like requests require an explicit requester reference.
3. Narrative SIG does not substitute for typed dosage.
4. Medication, route, method, site, and PRN indication are coded when used computationally.
5. Quantities used computationally retain UCUM system + code.
6. Scheduled time periods are positive and time-coded.
7. Maximum-dose periods have a typed time denominator.
8. `entered-in-error` cannot classify as an active order.
9. `on-hold` cannot classify as active.
10. FHIR request intent never substitutes for verified prescriber authority.

## Next integration

1. Add a strict FHIR R4 MedicationRequest -> `MedicationOrder` adapter.
2. Resolve MedicationRequest subject/requester references using the same no-guessing policy as Observation ingestion.
3. Add a practitioner authority verifier that produces an opaque prescribing-authority permit.
4. Require both an `ActiveOrderIntent` and verified authority permit before order presentation/activation workflows.
5. Adapt the existing prescription zome so new writes require typed medication semantics while preserving legacy entries for migration/audit.
6. Add dosage normalization and comparison only after unit-conversion semantics are independently qualified.

## Non-claims

This crate does not establish clinical appropriateness, dose correctness, interaction safety, pharmacogenomic suitability, regulatory compliance, or authorization to prescribe/dispense/administer. It establishes a typed semantic boundary on which those later checks can safely depend.
