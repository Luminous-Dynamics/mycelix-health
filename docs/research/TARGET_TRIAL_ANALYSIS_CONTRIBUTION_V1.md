# Target-Trial Fixed-Horizon Analysis Contribution V1

**Status:** research-only source contract. Authored tests are not executable qualification evidence.

## Purpose

This layer binds participant-level target-trial evidence into fixed-horizon binary analysis contributions **before** any estimator is allowed to consume them.

It exists to prevent an estimator from receiving an unqualified table where eligibility, time zero, treatment classification, follow-up, censoring and outcome ascertainment have been flattened into unaudited columns.

## Upstream evidence chain

```text
TargetTrialProtocolV2
        +
TargetTrialEmulationPlanV2
        ↓
TimeZeroReceiptV2
        ↓
EligibilityReceiptV2
        ↓
StrategyClassificationReceiptV2
        ↓
CohortEntryReceiptV2
        ↓
FollowUpReceiptV2
        ↓
OutcomeAscertainmentReceiptV2
        ↓
FixedHorizonAnalysisContributionV1
```

The builder re-verifies the follow-up and outcome receipts from their exact upstream evidence before constructing the contribution.

## V1 estimand scope

V1 deliberately supports only:

- `CausalContrastV2::IntentionToTreat`;
- `EffectMeasureV2::RiskDifference` or `RiskRatio`;
- `CompetingEventStrategyV2::NotApplicable`.

V1 rejects:

- per-protocol contrasts;
- hazard ratios;
- survival differences;
- mean differences;
- generic/other effect measures;
- competing-risk estimands;
- censor-at-competing-event semantics;
- composite competing-event reinterpretation;
- time-varying treatment;
- cloning/grace-period strategies;
- weighting or effect estimation.

Per-protocol analysis requires explicit adherence/deviation evidence. Hazard/person-time analysis requires exact event-time evidence. Competing-event strategies require their own versioned semantics.

## Contribution states

The contribution retains one of three states:

```text
ObservedEvent
ObservedNonEvent
OutcomeIndeterminate
```

Mapping from ascertainment is exact:

```text
Satisfied     -> ObservedEvent
NotSatisfied  -> ObservedNonEvent
Indeterminate -> OutcomeIndeterminate
```

`OutcomeIndeterminate` is never silently converted into a non-event.

A `NotSatisfied` outcome may become `ObservedNonEvent` only when the declared outcome window was completely observed. This is rechecked at this layer even though the upstream ascertainment layer already enforces compatible semantics.

## Bound identities

Each contribution binds:

- protocol digest;
- emulation-plan digest;
- exact estimand id + estimand-definition evidence;
- exact effect measure;
- exact competing-event strategy;
- subject identity;
- selected strategy and treatment/comparator role;
- cohort-entry receipt digest;
- time-zero receipt digest;
- follow-up receipt digest;
- outcome-ascertainment receipt digest;
- time zero;
- observed follow-up boundary;
- outcome id;
- exact outcome window;
- contribution state.

Any substitution changes contribution identity or fails verification.

## Opaque verified contribution

`VerifiedFixedHorizonContributionV1` is not serializable and is produced only after exact upstream reproduction.

The manifest builder consumes these verified wrappers rather than arbitrary serialized contribution structs.

## Contribution manifest

`FixedHorizonAnalysisManifestV1` groups contributions under one exact:

- protocol;
- emulation plan;
- estimand;
- estimand definition;
- effect measure;
- competing-event policy.

Entries are canonically sorted by `(subject_resource_type, subject_id)` and duplicate subjects fail closed.

Indeterminate participants remain present in the manifest.

This is deliberate: missingness/selection handling must be a downstream, explicit evidence-bearing decision rather than silent row deletion.

## Not an estimator-ready analysis set

A contribution manifest does **not** imply that an estimator may immediately use all rows.

Before estimation, later layers still need to address as appropriate:

- indeterminate outcomes and missingness policy;
- positivity/overlap;
- baseline confounder measurement;
- covariate balance;
- selection/censoring assumptions;
- weighting or standardization policy;
- estimator implementation and execution identity;
- sensitivity/control analyses.

## Person-time boundary

V1 is fixed-horizon binary evidence only.

`OutcomeAscertainmentReceiptV2` currently does not preserve a canonical qualifying event timestamp. Therefore V1 must not be described as:

- person-time evidence;
- incidence-rate evidence;
- hazard/risk-set evidence;
- survival-time evidence.

Those require the event-time evidence work tracked separately.

## Authored source tests

The source suite covers:

1. observed event preservation;
2. complete observed non-event preservation;
3. indeterminate preservation;
4. hazard-ratio rejection;
5. per-protocol rejection;
6. competing-risk rejection;
7. canonical multi-subject manifest ordering;
8. preservation of indeterminate participants in the manifest;
9. duplicate-subject rejection.

These are authored tests only until executed by an accepted toolchain/CI subject.

## Non-claims

These artifacts do **not** establish:

- random assignment;
- exchangeability;
- positivity;
- no unmeasured confounding;
- correct missing-data handling;
- covariate balance;
- correct censoring weights;
- estimator validity;
- causal effect;
- treatment benefit or harm;
- diagnosis;
- clinical recommendation;
- clinician presentation authority;
- regulatory clearance;
- autonomous therapeutic action.
