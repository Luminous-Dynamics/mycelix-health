# Clinical Semantics Kernel v1

Status: proposed executable contract

## Purpose

This document defines the minimum semantic contract that every clinical value crossing a Mycelix-Health trust boundary must satisfy before it can be treated as machine-actionable clinical data.

The goal is to prevent syntactically valid but clinically ambiguous data from silently entering records, decision support, trials, or research pipelines.

## Core invariant

A machine-actionable clinical assertion MUST bind:

1. a subject,
2. a typed value,
3. a clinical concept,
4. an effective time,
5. provenance,
6. units when the value is quantitative,
7. an explicit uncertainty / interpretation state when applicable.

If the system cannot establish those bindings, the value MUST remain narrative-only or be rejected at a stricter boundary. It MUST NOT be silently promoted into a typed clinical fact.

## ClinicalValue algebra

The shared representation should converge on the following semantic forms:

- `Quantity { value, unit, system, code }`
- `CodeableConcept { codings, text }`
- `Range { low, high }`
- `Ratio { numerator, denominator }`
- `Boolean`
- `Integer`
- `Decimal`
- `DateTime`
- `Reference { resource_type, id }`
- `Narrative { text, reason_not_typed }`

`Narrative` is intentionally a fallback. It is never equivalent to a typed value.

## Quantity rules

A quantitative value intended for computation MUST NOT be represented as a bare number or a number plus display-only unit text.

The preferred representation is UCUM-bound:

```text
value: 5.5
unit: "mmol/L"
system: "http://unitsofmeasure.org"
code: "mmol/L"
```

At ingestion boundaries:

- missing unit on a unit-bearing concept is `Indeterminate` or invalid according to the profile,
- unknown unit systems are not normalized by guessing,
- conversion must preserve the original value/unit as provenance,
- conversion must record the conversion function/version,
- incompatible dimensions are rejected.

## Subject binding

Every imported clinical resource MUST be bound to the expected patient/subject before persistence.

A resource with an unambiguous subject reference that conflicts with the target patient MUST be rejected.

A resource whose subject cannot be resolved MUST NOT be silently attached to the target patient. It may be quarantined as `UnresolvedReference` for explicit reconciliation.

This deliberately strengthens the current permissive behavior for references that cannot be compared reliably.

## Coded concepts

Clinical concepts should use established coding systems where available, including:

- LOINC for observations/labs,
- SNOMED CT for clinical concepts where licensing/deployment permits,
- RxNorm for US medication concepts,
- ICD only where classification/billing semantics are actually intended,
- UCUM for units.

Display strings are not stable identifiers.

## Provenance

Every machine-actionable clinical fact MUST carry or resolve to provenance sufficient to answer:

- where did this value originate?
- who or what asserted it?
- when was it observed vs recorded?
- which source resource/version produced it?
- which transformation produced the current representation?
- what software/schema/profile version performed that transformation?

Derived facts MUST additionally bind their input fact identifiers.

## Four-state evaluation

Clinical logic MUST prefer four-state outcomes over unsafe booleans:

- `Satisfied`
- `NotSatisfied`
- `Indeterminate`
- `NotApplicable`

Missing data is not evidence of absence.

This applies especially to trial eligibility, contraindications, quality measures, and decision support.

## FHIR R4 ingestion contract

For FHIR R4 imports:

1. validate resource type and profile expectations,
2. resolve the expected subject,
3. preserve resource id and source system,
4. prefer native typed `value[x]` forms over string coercion,
5. preserve `valueQuantity.value`, `unit`, `system`, and `code`,
6. preserve coding system + code + display for coded concepts,
7. preserve effective time separately from ingestion time,
8. record parsing/normalization provenance,
9. reject unambiguous cross-patient references,
10. quarantine unresolved subject references at strict clinical boundaries.

## Trial semantics

Free-text inclusion/exclusion criteria remain useful for human-readable protocol text, but MUST NOT be the authoritative executable eligibility representation.

A future computable criterion should bind:

- criterion id,
- human-readable text,
- computable expression reference (CQL where appropriate),
- required facts/concepts,
- temporal window,
- unit expectations,
- missing-data behavior,
- protocol version.

Eligibility results should use the four-state model above and include reasons/evidence.

## Evidence capsule boundary

Any decision-support output that could materially influence care SHOULD bind an evidence capsule containing:

- intended use,
- patient/context inputs,
- missing inputs,
- knowledge artifact and version,
- algorithm/model and version,
- output and uncertainty,
- contraindications / exclusions checked,
- known limitations,
- execution timestamp,
- approval/escalation requirement.

## Non-goals

This contract does not claim regulatory compliance, clinical validity, or autonomous diagnostic authority.

It is a semantic safety floor on which those higher assurance claims can later be built and independently validated.

## Promotion gate

No new clinical feature should be promoted to a higher assurance tier merely because it compiles or passes schema validation. Promotion requires evidence that semantic invariants are enforced under malformed, missing, contradictory, cross-subject, and unit-sensitive inputs.
