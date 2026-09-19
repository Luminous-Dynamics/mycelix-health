# Laboratory Result Semantics V1

Status: **research-only / staged / unqualified**

## Purpose

`mycelix-clinical-laboratory-semantics` defines an evidence-grade laboratory-result artifact for Mycelix Health.

A laboratory result is not represented as only `code + value + unit`. V1 preserves the context needed to understand what was actually measured and how:

- exact patient/subject binding;
- exact LOINC analyte code **and terminology release/version**;
- typed result value;
- UCUM quantity semantics for quantitative results;
- specimen identity/type and collection/receipt times when known;
- explicit missing specimen state when not known;
- assay/method identity when known;
- explicit missing assay/method state when not known;
- laboratory/source/accession/analyzer provenance;
- typed result status;
- reference intervals with their exact reference-population evidence identity;
- detection/quantification limits when supplied;
- interpretation/flag values kept separate from the measured result;
- uncertainty;
- correction/supersession lineage;
- serializer-independent artifact identity.

## Terminology policy

The semantic core does **not** hard-code a particular LOINC release.

At the time this contract was drafted, LOINC 2.83 (released 2026-08-19) is the current public release. That fact belongs in adapter/deployment policy, not in the timeless semantic type.

Every admitted analyte identity therefore carries:

```text
system
code
version/release
display?  (non-authoritative)
```

V1 requires the analyte system to be `http://loinc.org` and rejects a missing terminology version.

A future terminology-policy layer should separately establish which LOINC releases are admitted for a deployment and how older releases are reconciled without rewriting historical evidence.

## Missingness is evidence

Real-world laboratory data is often incomplete. V1 does not turn missing context into defaults.

For specimen and assay/method context the artifact records either:

```text
Known(value)
```

or one explicit missing reason:

```text
NotProvided
Unknown
NotApplicable
Redacted
```

`Missing(NotProvided)` is therefore cryptographically different from a known method/specimen and from the other missingness states.

## Reference intervals

A reference interval is not merely a pair of numbers. V1 binds each interval to a `ReferencePopulationV1`, including a versioned external evidence artifact identity.

The interval and quantitative result must use compatible UCUM units. Low/high inversion fails closed.

Changing the reference population/context changes the laboratory-result snapshot identity even when the measured value is identical.

## Analytical limits

For quantitative results V1 can preserve:

- lower detection limit;
- lower quantification limit;
- upper quantification limit;
- upper detection limit.

All supplied limits must be machine-actionable UCUM quantities in the same unit as the reported result.

The presence of a limit does not itself mean the reported value is censored. A later tranche should model explicit less-than/greater-than/censored result semantics rather than overloading a numeric value.

## Corrections and supersession

`Amended`, `Corrected`, and `Appended` results must name the exact prior result ID and prior laboratory snapshot digest.

Historical evidence is not overwritten.

```text
result v1
   ↓ exact digest
result v2 (Corrected)
   ↓ supersedes v1
result v3 ...
```

A corrected result is new evidence with lineage, not mutation of the old evidence object.

## ClinicalFact compatibility

A validated `LaboratoryResultV1` may project into the existing `ClinicalFact` kernel for compatibility.

The projection intentionally preserves only the common clinical-fact subset:

- subject;
- analyte concept;
- typed result value;
- effective time;
- source provenance;
- uncertainty.

The projection also returns the exact laboratory snapshot digest so callers can retain the richer source identity.

`ClinicalFact` alone must not be treated as proof of the full laboratory context.

## Serializer-independent identity

The V1 snapshot framing is independent of JSON/serde representation.

It binds all semantic fields using explicit binary framing, including exact IEEE-754 bits for numeric values and exact vector order.

Current V1 wire tags are frozen by tests. Future incompatible framing requires a new artifact version/domain.

## FHIR interoperability boundary

The existing generic FHIR Observation adapter is intentionally insufficient to claim full laboratory semantics because it does not preserve all V1 context.

A later dedicated adapter should consume the complete laboratory-report graph:

```text
Patient
Specimen
Observation
DiagnosticReport
performer / organization / device context
```

and emit `LaboratoryResultV1` only when the required relationships are unambiguous.

The adapter must preserve exact source resource IDs/versions and must never infer a unit, terminology release, specimen, method, reference population, or correction relationship from display text.

## Qualification targets

Before this tranche is considered qualified:

1. `cargo fmt --check` for the package;
2. Clippy with warnings denied;
3. package tests;
4. canonical identity determinism;
5. patient substitution tests;
6. unit substitution tests;
7. method substitution tests;
8. specimen substitution/timeline tests;
9. reference-population substitution tests;
10. correction/supersession tests;
11. exact terminology-release tests;
12. FHIR laboratory-report fixtures in the adapter tranche;
13. lock-consistent exact-head execution.

## Explicit non-claims

V1 does **not** establish:

- diagnostic accuracy;
- biomarker validity;
- disease presence or absence;
- causal explanation;
- treatment indication;
- reference-interval applicability to a particular patient;
- clinician/patient presentation authority;
- medical-device status;
- regulatory clearance/approval;
- autonomous clinical action.

In particular:

```text
abnormal flag != diagnosis
reference interval != patient-specific normality
laboratory value != causal explanation
LaboratoryResultV1 != clinical authority
```
