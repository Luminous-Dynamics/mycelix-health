# Clinical Phenotype V1

Status: **research-only / staged / unqualified**

## Purpose

`mycelix-clinical-phenotype` defines evidence-bound longitudinal cohort/phenotype evaluation over validated `ClinicalFact` snapshots.

The V1 problem is intentionally narrow:

> When may a research system treat the absence of a recorded clinical event as evidence?

The answer in V1 is: **only when the relevant observation domain is explicitly complete for the declared time window.**

## Observation window

Every phenotype definition binds an exact half-open interval:

```text
[start_micros, end_micros)
```

Facts outside the window cannot satisfy or refute criteria.

The window also carries one or more named coverage domains. Each domain has:

- a stable domain name;
- status `Complete`, `Incomplete`, or `Unknown`;
- a versioned evidence artifact establishing the coverage claim.

A coverage statement is evidence about a particular data source/domain/window. It is not a claim that the patient's global medical history is complete.

## Criteria

V1 contains only four criterion forms:

```text
FactPresent
FactAbsent
All
Any
```

`FactPresent` binds:

- criterion id;
- exact terminology system/code/version;
- minimum fact count;
- required coverage domain.

`FactAbsent` binds:

- criterion id;
- exact terminology system/code/version;
- required coverage domain.

The simple V1 vocabulary is deliberate. Numeric thresholds, temporal sequences, medications/exposures, diagnosis ontologies, negation, and derived model outputs should be added as separate qualified tranches rather than hidden in generic expressions.

## Missingness and negative evidence

### Presence

If enough matching facts are observed, `FactPresent` is `Satisfied` even when coverage is incomplete.

If too few facts are observed:

- `Complete` coverage -> `NotSatisfied`;
- `Incomplete` or `Unknown` coverage -> `Indeterminate`.

### Absence

If a matching fact is observed, `FactAbsent` is `NotSatisfied` regardless of coverage completeness.

If no matching fact is observed:

- `Complete` coverage -> `Satisfied`;
- `Incomplete` or `Unknown` coverage -> `Indeterminate`.

Therefore:

```text
no record found != event did not happen
```

unless the declared coverage evidence makes that inference admissible.

## Composition

For `All`:

1. any `NotSatisfied` child -> `NotSatisfied`;
2. otherwise any `Indeterminate` child -> `Indeterminate`;
3. otherwise -> `Satisfied`.

For `Any`:

1. any `Satisfied` child -> `Satisfied`;
2. otherwise any `Indeterminate` child -> `Indeterminate`;
3. otherwise -> `NotSatisfied`.

This prevents unknown evidence from being silently coerced to false.

## Fact identity

Every input `ClinicalFact` is validated and assigned its serializer-independent `ClinicalFactSnapshotDigestV1`.

V1 rejects:

- mixed-patient inputs;
- duplicate ClinicalFact snapshot identities;
- facts with invalid machine-actionable semantics.

Matching is terminology-version exact. A fact coded as LOINC 2.82 does not silently satisfy a criterion explicitly bound to 2.83.

## Definition and evaluation identity

Both the phenotype definition and resulting evaluation have domain-separated serializer-independent identities.

Changing any of the following changes the corresponding identity:

- phenotype id/version;
- observation window;
- coverage status or coverage-evidence artifact;
- criterion tree;
- terminology release;
- subject;
- criterion result state;
- supporting/refuting ClinicalFact snapshots.

## Research uses

V1 is intended to support:

- reproducible cohort construction;
- external-validation cohorts;
- biomarker studies;
- safety/pharmacovigilance cohorts;
- longitudinal trajectory research;
- later target-trial eligibility logic;
- later OMOP-backed research adapters.

## Qualification targets

Before V1 is qualified:

1. format check;
2. Clippy with warnings denied;
3. package tests;
4. exact definition/evaluation digest determinism;
5. mixed-patient rejection;
6. duplicate-fact rejection;
7. exact terminology-version matching;
8. boundary tests for `[start,end)`;
9. absence under complete/incomplete/unknown coverage;
10. positive evidence under incomplete coverage;
11. `All` / `Any` indeterminate propagation;
12. criterion-count/depth limits;
13. lock-consistent exact-head execution.

## Explicit non-claims

V1 does **not** establish:

- diagnosis;
- disease absence;
- causal effect;
- treatment indication;
- screening recommendation;
- patient-specific risk;
- clinical decision support authority;
- clinician/patient presentation authority;
- autonomous clinical action.

A phenotype result is research cohort evidence, not a medical conclusion.
