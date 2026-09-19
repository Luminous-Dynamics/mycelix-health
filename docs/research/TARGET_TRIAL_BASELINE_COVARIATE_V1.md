# Target-Trial Baseline Covariate Measurement V1

**Status:** research-only source contract. Authored tests are not executable qualification evidence.

## Purpose

This layer proves which exact pre-time-zero clinical measurement was selected to represent a declared baseline confounder for one observational target-trial participant.

A confounder name in an analysis plan is not sufficient evidence of the value used for that participant.

## Evidence chain

```text
TargetTrialProtocolV2
        +
TargetTrialEmulationPlanV2
        ↓
TimeZeroReceiptV2
        ↓
BaselineMeasurementPolicyV1
        +
CandidateSetEvidence
        +
exact ClinicalFact snapshots
        ↓
BaselineCovariateMeasurementReceiptV1
```

## Measurement policy

A policy binds:

- exact confounder id;
- exact live confounder-definition artifact from the emulation plan;
- exact concept system/code/version;
- relative baseline measurement window;
- deterministic selection rule;
- exact policy evidence.

V1 supports:

```text
UniqueOnly
LatestAtOrBeforeEnd
```

The policy window must end at or before time zero and may not extend later than the confounder's protocol/emulation baseline cutoff.

## Candidate facts

Every supplied candidate fact must:

- be machine-actionable;
- belong to the exact participant from the verified time-zero receipt;
- contain the exact concept system/code/version named by the policy;
- have `effective_at_micros` inside the declared baseline window;
- have a unique `ClinicalFactSnapshotDigestV1`.

The receipt stores the canonically ordered candidate snapshot identities considered by the selection rule.

Changing patient, value, effective time, provenance, transformation lineage or uncertainty changes the underlying snapshot identity.

## Candidate-set completeness boundary

The local Rust verifier proves that **supplied** candidate facts are valid candidates and that selection is deterministic.

It cannot independently prove that an EHR/database query returned every possible qualifying row.

Therefore each receipt also binds an exact `candidate_set_evidence` artifact representing the extraction/query result boundary.

```text
valid local candidate set
!=
proof of globally complete source retrieval
```

A future source adapter or qualified query engine should strengthen that external evidence.

## Selection semantics

### UniqueOnly

- zero candidates → `MissingMeasurement`;
- exactly one → selected;
- more than one → fail closed.

### LatestAtOrBeforeEnd

- zero candidates → `MissingMeasurement`;
- otherwise choose the candidate with latest `effective_at_micros`;
- multiple distinct candidate snapshots tied at the latest instant → fail closed.

V1 does not silently break ties.

## Missing measurement semantics

`MissingMeasurement` means only:

> no qualifying measurement was available in the supplied candidate set under this exact policy.

It does **not** mean:

- the patient lacks the condition;
- the covariate is zero/normal/false;
- the source system had complete observation;
- imputation is justified.

No imputation occurs in this layer.

## Receipt identity

The receipt binds:

- protocol digest;
- emulation-plan digest;
- participant identity;
- exact time-zero receipt digest and timestamp;
- confounder id;
- measurement-policy digest;
- candidate-set evidence;
- derived absolute baseline window;
- all candidate fact snapshot identities + effective times;
- selected fact identity/time or explicit missing state.

`VerifiedBaselineCovariateMeasurementV1` is non-serializable and is produced only after rebuilding from the exact supplied evidence.

## Authored source tests

The source suite covers:

1. unique measurement selection;
2. explicit missing measurement;
3. multiple-candidate rejection under `UniqueOnly`;
4. deterministic latest selection;
5. tied-latest fail-closed behavior;
6. late/post-baseline candidate rejection;
7. terminology-version substitution rejection;
8. cross-patient candidate rejection;
9. policy window cannot extend past the declared baseline cutoff.

These are authored tests only until executed by an accepted toolchain/CI subject.

## Non-claims

These artifacts do **not** establish:

- that the declared confounder set is sufficient;
- no unmeasured confounding;
- exchangeability;
- positivity/overlap;
- covariate balance;
- correct functional form;
- missing-at-random assumptions;
- valid imputation;
- correct adjustment/weighting;
- causal effect;
- treatment benefit or harm;
- diagnosis or clinical authority.

## Downstream

The next layers should construct:

1. an exact baseline-covariate matrix manifest;
2. explicit missingness diagnostics;
3. treatment-group overlap/positivity diagnostics;
4. covariate-balance evidence;
5. only then adjustment/weighting and estimator execution receipts.
