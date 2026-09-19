# Target-Trial Baseline Covariate Encoding V1

**Status:** research-only source contract. This branch is not compile-qualified or causally qualified.

## Purpose

This layer converts the exact selected `ClinicalFact` snapshots referenced by a verified `BaselineCovariateMatrixV1` into explicitly declared analysis values.

It exists because:

```text
selected clinical measurement identity
        !=
analysis feature value
```

No downstream overlap, balance, weighting, adjustment, or estimator should infer its own representation from the raw clinical value ad hoc.

## Upstream authority

The builder consumes:

- opaque `VerifiedBaselineCovariateMatrixV1`;
- exactly one `BaselineCovariateEncodingPolicyV1` per confounder column;
- the exact source `ClinicalFact` set referenced by all selected matrix cells.

It does not accept an arbitrary serialized baseline matrix as upstream authority.

## V1 encoding rules

V1 deliberately supports only four explicit rules:

1. `Boolean` → exact Boolean;
2. `Integer` → exact signed 64-bit integer;
3. `Decimal` → exact finite IEEE-754 bit pattern;
4. `QuantityUcumExact` → exact finite IEEE-754 bit pattern + exact expected UCUM code.

V1 does **not** silently encode:

- coded concepts;
- ranges;
- ratios;
- references;
- datetimes;
- narratives.

Those require future explicit contracts if needed.

## No implicit conversion

V1 performs no:

- integer-to-floating coercion;
- unit conversion;
- categorical mapping;
- normalization or z-scoring;
- winsorization;
- spline/basis expansion;
- interaction construction;
- missing-value imputation.

A quantity must already be machine-actionable UCUM and must carry the exact unit code named by the encoding policy.

## Encoding policy

Each column policy binds:

- exact confounder id;
- exact baseline measurement-policy digest;
- exact encoding rule;
- exact versioned policy evidence.

The policy set must equal the baseline matrix column set exactly.

A changed encoding rule, expected UCUM code, policy evidence, or upstream measurement policy changes the encoding-policy identity.

## Exact source-fact set

For every selected baseline cell, the builder recomputes `ClinicalFactSnapshotDigestV1` from the supplied typed fact.

The unique set of supplied fact digests must equal the unique set of selected snapshot digests in the baseline matrix.

Therefore:

- missing selected fact → reject;
- substituted fact → reject;
- extra unreferenced fact → reject;
- duplicate supplied snapshot → reject;
- cross-patient fact → reject.

Missing matrix cells require no source fact and remain explicit `Missing` values.

## Output identity

`EncodedBaselineCovariateMatrixV1` binds:

- exact upstream baseline-matrix digest;
- exact protocol/emulation/estimand coordinates inherited from that matrix;
- exact analysis-manifest digest;
- canonical row and column order;
- exact measurement-policy digest per column;
- exact encoding-policy digest per column;
- exact measurement-receipt digest per cell;
- exact selected fact snapshot digest per non-missing cell;
- exact encoded value or explicit missingness.

Identity uses serializer-independent domain-separated binary framing.

## Missingness

`Missing` is a first-class encoded value.

It is not converted to:

- zero;
- false;
- NaN;
- normal;
- reference category;
- population mean.

Imputation, if ever used, must be a separate evidence-bound research layer.

## Verification

`verify_encoded_baseline_covariate_matrix_v1()` rebuilds the encoded matrix from:

- the opaque verified baseline matrix;
- the exact policies;
- the exact typed source facts;

and requires the rebuilt identity to equal the supplied serialized matrix identity.

## Required hardening before source-freeze

Before this tranche is called source-frozen:

1. add end-to-end tests using real verified baseline-matrix objects;
2. test Boolean, Integer, Decimal and exact-UCUM Quantity rules;
3. test missingness preservation;
4. test source-fact omission/extra/substitution;
5. test patient substitution;
6. test policy/measurement-policy mismatch;
7. test UCUM-unit mismatch;
8. test canonical row/column rejection;
9. independently reject non-finite `DecimalBits` / quantity bit patterns after deserialization.

## Non-claims

This artifact does **not** establish:

- correct confounder set;
- exchangeability;
- no unmeasured confounding;
- positivity/overlap;
- balance;
- correct functional form;
- correct missing-data assumptions;
- causal effect;
- treatment benefit or harm;
- clinical recommendation;
- diagnostic truth;
- clinical authority.

## Downstream

Once this layer is source-frozen and executed, downstream research can add, in order:

1. explicit missingness diagnostics;
2. positivity/overlap diagnostics;
3. baseline balance diagnostics;
4. explicit imputation/adjustment contracts when required;
5. estimator admission and execution receipts.