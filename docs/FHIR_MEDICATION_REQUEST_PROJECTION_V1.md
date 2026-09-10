# Strict FHIR R4 MedicationRequest Projection v1

## Purpose

This profile defines the subset of FHIR R4 `MedicationRequest` that Mycelix may promote into an exact medication-activation workflow artifact.

FHIR validity and activation safety are intentionally different questions. Valid FHIR that cannot be projected without semantic loss is rejected/quarantined rather than guessed.

## Required activation path

`MedicationRequest`
-> exact patient binding
-> exact active status + order-like intent
-> typed medication concept
-> typed dosage/timing/route/UCUM units
-> strict requester reference
-> provider/principal resolution
-> requester evidence lineage
-> `MedicationRequestArtifact`
-> exact artifact digest
-> practitioner authority for `Prescribe`
-> authenticated principal equality
-> freshness checks
-> dispense validity check
-> `MedicationActivationCapability`

A bare `MedicationOrder` is no longer sufficient at the high-assurance FHIR activation boundary.

## v1 accepted semantics

- `status=active` after medication-semantics classification;
- order-like MedicationRequest intent;
- `medicationCodeableConcept` with machine-readable coding;
- Patient subject that binds exactly to the expected patient;
- concrete Practitioner/PractitionerRole requester reference;
- optional requester identifier used as an additional intersection constraint;
- one or more dosage instructions;
- coded route;
- exact scheduled frequency/period, or coded PRN with no unmodeled timing constraints;
- exactly one `doseAndRate` element per dosage in v1;
- quantity/range dose;
- optional quantity/range/ratio rate;
- typed max-dose constraints;
- UCUM quantities without comparator loss;
- optional dispense quantity/refills/validity period;
- coded `reasonCode`;
- narrative MedicationRequest notes preserved in the artifact identity.

## Fail-closed cases

v1 rejects rather than approximates:

- `medicationReference` requiring unresolved external medication-resource semantics;
- patient mismatch or unresolved patient reference;
- requester with no concrete reference;
- unresolved or ambiguous practitioner identity;
- requester reference/identifier disagreement;
- inactive/draft/on-hold/entered-in-error or non-order workflow state at activation;
- free-text-only dosage;
- uncoded PRN (`asNeededBoolean=true`);
- complex Timing features not represented by the v1 medication algebra;
- multiple `doseAndRate` elements;
- conflicting dose/rate choice elements;
- quantity comparators (`<`, `<=`, `>=`, `>`), because the current `Quantity` type cannot represent them losslessly;
- `reasonReference`;
- substitution semantics;
- prior-prescription semantics;
- dispense performer/expected-supply-duration semantics;
- dosage additionalInstruction/type semantics that would otherwise be silently dropped.

These are coverage gaps, not permissions to discard data. Future versions should model the semantics explicitly before accepting them.

## Requester identity

`requester.reference` is an external FHIR identity claim. It is not an authenticated Mycelix principal.

The artifact can only be constructed after `mycelix-provider-principal` resolves that external claim through verified provider/principal binding evidence. The resulting artifact retains:

- resolved principal binding;
- provider-record digest;
- provider author-binding evidence digest;
- provider status/revocation evidence digest;
- resolution timestamp.

The workflow layer then requires the authenticated principal to equal the resolved requester principal and requires the resolution to remain fresh.

## Professional authority remains separate

Requester resolution answers: "which authenticated principal does this external practitioner claim refer to?"

It does not answer: "may that principal prescribe this medication here?"

That second decision remains:

`issuer trust -> credential resolution/status -> clinical authority policy -> AuthorityPermit(Prescribe, exact artifact) -> MedicationActivationCapability`

## Artifact identity

`MedicationRequestArtifact` is serializable for deterministic internal v1 hashing but not deserializable into a trusted resolved artifact. Construction requires an in-process resolved requester.

Its digest uses the `MedicationRequestArtifact` domain in `mycelix-clinical-integrity`; the same encoded bytes in another domain do not share an identity.

## Non-claims

This profile does not determine dose appropriateness, drug-drug interactions, allergy safety, renal/hepatic adjustment, pregnancy safety, formulary coverage, controlled-substance legality, dispensing authorization, or administration authorization.

Those are separate evidence/authority domains that must compose explicitly rather than being inferred from successful FHIR projection.
