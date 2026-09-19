# FHIR Laboratory Adapter V1

Status: **research-only / staged / unqualified**

## Purpose

`mycelix-fhir-laboratory-semantics` is a strict FHIR R4 adapter that projects a bounded laboratory-report graph into `LaboratoryResultV1`.

It is deliberately stricter than the generic FHIR Observation adapter because laboratory evidence depends on relationships among multiple resources and on context that must not be guessed.

## Input graph

V1 resolves exactly:

```text
DiagnosticReport
    ├── subject -> Patient/{id}
    ├── performer -> expected laboratory identity (when present)
    └── result -> Observation/{id}
                      ├── subject -> same Patient/{id}
                      └── specimen -> Specimen/{id}
                                         └── subject -> same Patient/{id}
```

Relative `ResourceType/id` references are required in V1. Unresolved or duplicate resource identities fail closed.

## LOINC release binding

The adapter policy supplies the exact accepted LOINC release.

The Observation must contain exactly one LOINC coding and that coding must include an explicit `Coding.version` equal to the accepted release.

The adapter never upgrades an unversioned LOINC code into a versioned evidence identity by assumption.

## Laboratory value

V1 supports:

- `Observation.valueQuantity` with exact UCUM system/code;
- `Observation.valueCodeableConcept` with coded semantics.

String/narrative-only values are not projected as machine-actionable laboratory evidence in V1.

## Specimen requirement

Unlike the semantic core, which can represent explicitly missing specimen context, the strict FHIR adapter V1 requires `Observation.specimen` to resolve to a concrete `Specimen` resource with:

- exact specimen id;
- coded specimen type;
- patient binding;
- `collection.collectedDateTime`.

An unresolved specimen reference is an error, not `Missing(NotProvided)`.

## Effective-time restriction

The current `LaboratoryResultV1` core does not yet have an independent `effective_at_micros` field.

To avoid semantic loss, FHIR adapter V1 therefore requires:

```text
Observation.effectiveDateTime == Specimen.collection.collectedDateTime
```

If they differ, projection fails with `EffectiveTimeNotRepresentable`.

This is intentionally conservative. A later laboratory artifact revision should preserve effective time independently and remove this restriction.

## Reference intervals

FHIR `Observation.referenceRange` does not by itself establish the exact reference-population evidence used to derive the interval.

V1 therefore requires a separate `ReferencePopulationBindingV1` for every projected reference range. The binding supplies the versioned `ReferencePopulationV1` evidence identity.

Unused bindings fail closed so a policy cannot silently carry context for the wrong Observation/range.

## Corrected/amended results

FHIR status alone is not enough to prove which exact historical laboratory artifact is superseded.

For `amended`, `corrected`, or equivalent states, V1 requires an explicit `PriorResultBindingV1` containing the prior result id and prior laboratory snapshot digest.

Historical evidence is not overwritten.

## Provenance

The adapter preserves:

- source FHIR system;
- Observation resource id/version;
- DiagnosticReport accession identifier when present;
- deployment laboratory identity;
- performer reference when available;
- Observation issued time or DiagnosticReport issued fallback;
- explicit transformation provenance naming this adapter;
- input resource identities for DiagnosticReport, Observation, and Specimen.

## Qualification targets

Before this adapter is treated as qualified:

1. format and Clippy with warnings denied;
2. package tests;
3. complete laboratory bundle fixture;
4. cross-patient substitution rejection;
5. unresolved specimen rejection;
6. duplicate resource-id rejection;
7. LOINC-release substitution rejection;
8. unit substitution rejection through the laboratory semantic core;
9. effective-time mismatch rejection;
10. missing/unused reference-population binding rejection;
11. corrected-result prior-binding tests;
12. exact source-resource/version provenance assertions;
13. exact-head lock-consistent execution.

## Explicit non-claims

The adapter does not establish:

- diagnostic accuracy;
- reference-interval applicability to a specific patient;
- biomarker validity;
- disease presence/absence;
- causal explanation;
- treatment indication;
- clinician/patient presentation authority;
- medical-device clearance;
- autonomous clinical action.

Its theorem is only:

> this exact bounded FHIR laboratory graph can be projected into this exact laboratory evidence artifact without the specific ambiguities V1 refuses.
