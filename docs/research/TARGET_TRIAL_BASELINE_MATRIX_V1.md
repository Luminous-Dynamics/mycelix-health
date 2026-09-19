# Target-Trial Baseline Covariate Matrix V1

**Status:** research-only source contract. Authored tests are not executable qualification evidence.

## Purpose

`BaselineCovariateMatrixV1` creates a canonical subject × baseline-confounder identity/missingness matrix before imputation, feature encoding, overlap diagnostics, balance assessment, weighting, adjustment, or estimator execution.

The matrix composes two already-separated evidence lines:

```text
VerifiedFixedHorizonContributionV1
        +
VerifiedBaselineCovariateMeasurementV1
        ↓
BaselineCovariateMatrixV1
```

It does not create analysis-ready covariate values.

## Core distinction

```text
selected clinical measurement identity
        !=
analysis feature value
```

A baseline measurement receipt establishes which exact clinical fact snapshot was selected under an exact measurement policy, or preserves `MissingMeasurement`.

It does not decide:

- numeric scaling;
- unit conversion;
- categorical encoding;
- transformations;
- winsorization;
- spline basis;
- missing indicators;
- imputation;
- reference categories.

Those require a later explicit encoding contract.

## Inputs

The builder accepts only:

- live `TargetTrialProtocolV2`;
- live `TargetTrialEmulationPlanV2`;
- opaque `VerifiedFixedHorizonContributionV1` values;
- opaque `VerifiedBaselineCovariateMeasurementV1` values.

The fixed-horizon analysis manifest is rebuilt from the verified contribution objects rather than accepted as arbitrary serialized rows.

## Matrix coordinates

The matrix binds:

- exact protocol digest;
- exact emulation-plan digest;
- exact estimand id;
- exact fixed-horizon analysis-manifest digest;
- canonical confounder columns;
- canonical subject rows;
- exact contribution digest per subject;
- exact time-zero receipt digest per subject;
- exact baseline-measurement receipt digest per cell;
- selected fact snapshot/effective time or explicit missingness;
- one exact measurement-policy digest per confounder column.

The matrix identity uses serializer-independent, domain-separated canonical binary framing.

## Complete Cartesian product

For `N` analysis subjects and `M` declared baseline confounders, the matrix requires exactly `N × M` verified cells.

For every `(subject, confounder)` pair there must be one and only one verified baseline measurement receipt.

Fail closed on:

- missing cells;
- extra cells;
- unknown subjects;
- unknown confounders;
- duplicate cells;
- duplicate analysis subjects;
- duplicate declared confounders.

## Measurement-policy consistency

Every subject in one confounder column must use the same exact `measurement_policy_digest`.

This prevents one participant from being measured under, for example, a latest-value policy while another silently uses a unique-only or differently evidenced policy under the same column name.

A changed measurement policy requires a different matrix identity.

## Time-zero binding

Every baseline measurement receipt must bind the exact time-zero receipt digest and timestamp carried by that subject's fixed-horizon contribution.

The same timestamp with different time-zero source evidence is not interchangeable.

This protects baseline measurement from post-hoc rebinding to another observational assignment event.

## Missingness semantics

`MissingMeasurement` remains a first-class cell state.

It means only:

> no qualifying measurement was available under the exact supplied measurement and extraction evidence.

It does **not** mean:

- the clinical condition was absent;
- the covariate value was zero;
- false;
- normal;
- reference category;
- complete EHR absence.

The matrix never drops missing rows or cells and performs no imputation.

## Canonical order

Columns are strictly ordered by `confounder_id`.

Rows are strictly ordered by `(subject_resource_type, subject_id)`.

Cells must align one-to-one with the canonical column order.

A deserialized matrix that reorders rows or columns is rejected rather than silently normalized during hashing.

## External completeness boundary

The matrix can prove that the supplied verified measurement receipts form a complete Cartesian product relative to the exact contribution manifest and declared confounders.

It does **not** prove that each underlying EHR/data-source query was globally complete.

That external extraction boundary remains represented by each measurement receipt's exact `candidate_set_evidence` and must be qualified separately if stronger completeness claims are required.

Therefore:

```text
matrix completeness
    !=
source-system retrieval completeness
```

## Authored adversarial source cases

The source test suite covers:

1. two-subject × two-confounder canonical assembly;
2. explicit missing measurement preserved as a cell state;
3. missing Cartesian cell rejected;
4. duplicate cell rejected;
5. mixed measurement-policy identity for one confounder rejected;
6. same timestamp with substituted time-zero evidence rejected;
7. missing-vs-selected measurement changes matrix identity;
8. noncanonical row order rejected;
9. noncanonical column order rejected.

These are authored tests only until executed in a qualified environment.

## Deliberate non-scope

V1 does not establish or perform:

- feature extraction from selected clinical values;
- unit normalization;
- categorical encoding;
- imputation;
- missing-data modeling;
- positivity or overlap assessment;
- covariate balance;
- propensity scores;
- weighting;
- regression adjustment;
- estimator execution;
- sensitivity analysis;
- causal effect estimation.

## Non-claims

This artifact does **not** establish:

- confounder sufficiency;
- exchangeability;
- no unmeasured confounding;
- positivity;
- balance;
- correct functional form;
- correct missingness assumptions;
- causal effect;
- treatment benefit or harm;
- diagnostic truth;
- clinical recommendation;
- clinician presentation authority;
- regulatory clearance;
- autonomous action.

## Next layer

The next prerequisite should be an explicit baseline-covariate encoding contract that converts selected fact snapshots into analysis values under versioned, evidence-bound policies while preserving missingness and exact source identity.

Only after an encoded matrix exists should missingness diagnostics, overlap/positivity, baseline balance, weighting/adjustment, and estimator admission be allowed to consume the cohort.