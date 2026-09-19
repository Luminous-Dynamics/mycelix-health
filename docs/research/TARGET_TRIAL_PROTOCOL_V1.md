# Target Trial Protocol + Observational Emulation Plan V1

Status: **research-only / staged / unqualified**

## Purpose

`mycelix-target-trial-protocol` separates three things that must not be collapsed:

1. the hypothetical randomized target trial we would ideally conduct;
2. the observational-data emulation plan used to approximate that protocol;
3. any causal estimate later produced by executing that plan.

V1 defines only (1) and (2). There is intentionally no field for an effect estimate.

The structure follows the 2025 TARGET reporting guidance: eligibility, treatment strategies, assignment, time zero/follow-up, outcomes, causal contrasts/estimands, identifying assumptions, analysis plan, and a separate description of how each component is operationalized in observational data.

## TargetTrialProtocolV1

The protocol binds:

- protocol id/version;
- explicit causal question;
- rationale/prior-evidence artifacts;
- eligibility phenotype identity;
- at least two treatment strategies;
- the **hypothetical randomized assignment** procedure;
- time-zero definition;
- follow-up duration/end conditions;
- one or more outcome phenotype identities;
- at least one primary outcome;
- causal estimands and effect-measure definitions;
- competing-event strategy;
- identifying assumptions;
- analysis-plan identity;
- sensitivity-analysis plan identities.

The protocol does not know which EHR, claims system, OMOP instance, registry, or analytics package will be used to emulate it.

## TargetTrialEmulationPlanV1

The emulation plan binds the exact protocol digest and separately defines how it is mapped to observational data.

It records:

- explicit statement that the design is observational;
- data sources, including original purpose, type, setting, geography, and source period;
- eligibility operationalization;
- one classification rule for every protocol treatment strategy;
- observational time-zero operationalization;
- one outcome operationalization for every protocol outcome;
- baseline confounder definitions;
- censoring plan;
- optional adherence plan;
- optional time-varying-confounding plan;
- missing-data plan;
- estimator plan;
- software and execution-environment identities;
- vocabulary/mapping/data-model artifacts such as OMOP vocabulary snapshots.

The emulation plan cannot be rebound to a different protocol without changing/failing its identity.

## Randomization boundary

The protocol describes the assignment that the hypothetical target trial would use.

The emulation plan describes **observational classification** into those strategies.

These are not equivalent:

```text
hypothetical random assignment
        !=
observational treatment classification
```

A valid `TargetTrialEmulationPlanV1` therefore provides no evidence that actual people were randomized.

## Time zero

The protocol has a versioned `time_zero_definition`.

The emulation plan has a separate versioned `time_zero_operationalization` describing how that time is identified in observational data.

Later subject-level receipt work should prove that, for every included person, eligibility assessment, treatment classification, and follow-up start are aligned to the emulated time zero.

This is deliberately left as an explicit later proof rather than a boolean on the protocol.

## Baseline confounder guard

Every V1 baseline confounder carries a latest permissible measurement offset relative to time zero.

V1 rejects:

```text
measurement_window_end_offset_micros > 0
```

so a variable labeled "baseline confounder" cannot silently use post-baseline information.

This does not by itself establish that all confounders have been measured, nor that exchangeability holds.

## Estimands

Each estimand binds:

- treatment strategy;
- comparator strategy;
- outcome;
- causal contrast (`IntentionToTreat` or `PerProtocol`);
- effect-measure family;
- competing-event handling;
- a versioned estimand-definition artifact.

Each estimand must reference strategies/outcomes declared by the protocol.

V1 includes an estimand **specification**, not an estimand **value**.

## Data-source identity

Observational source context is part of emulation identity because transportability and data quality depend on where the data came from.

Each source binds:

- source id;
- original purpose;
- data-source type;
- clinical/research setting;
- geography;
- covered calendar/data period;
- exact source-evidence identity.

Changing the source setting or period therefore changes the emulation-plan identity.

## Mapping to OMOP

OMOP is intentionally not hard-coded into the target-trial protocol.

When an emulation uses OMOP, its plan should bind the exact:

- OMOP CDM version;
- vocabulary snapshot;
- concept-set definitions;
- Mycelix -> OMOP mapping policies;
- source data identity.

This makes an OMOP emulation one implementation of the target trial, not the definition of the scientific question itself.

## Phenotype binding

Eligibility and outcome definitions use exact phenotype id/version/digest identities.

A later integration tranche should consume `PhenotypeDefinitionDigestV1` values directly and prove that subject-level phenotype evaluations correspond to the exact definitions named in the protocol/emulation.

## Serializer-independent identity

Both protocol and emulation plan have separate domain-separated BLAKE3 identities using explicit binary framing.

Identity changes when scientifically material inputs change, including:

- protocol question/version;
- eligibility/outcome phenotype;
- treatment strategy definitions;
- time zero/follow-up;
- estimand;
- identifying assumptions;
- data sources;
- operationalizations;
- confounder timing;
- estimator/missing-data/censoring plans;
- software/environment;
- vocabulary/mapping artifacts.

## Next evidence tranches

V1 deliberately stops before cohort execution. Follow-on proof objects should include:

1. subject-level eligibility receipt;
2. subject-level time-zero receipt;
3. treatment-strategy classification receipt;
4. outcome-ascertainment receipt;
5. censoring/adherence longitudinal state;
6. exact cohort-assembly identity;
7. positivity/overlap diagnostics;
8. baseline-covariate balance diagnostics;
9. estimator execution identity;
10. sensitivity-analysis execution identities;
11. negative/positive control analyses where appropriate;
12. final causal-result capsule integrated with Mycelix clinical causality.

## Qualification targets

Before V1 is qualified:

- `cargo fmt --check`;
- Clippy with warnings denied;
- package tests;
- exact protocol/emulation digest determinism;
- protocol substitution rejection;
- complete strategy/outcome operationalization coverage;
- unknown/duplicate strategy/outcome rejection;
- post-baseline confounder rejection;
- data-source identity substitution tests;
- software/environment identity substitution tests;
- serializer-independent framing tests;
- exact-head lock-consistent execution.

## Explicit non-claims

V1 does **not** establish:

- actual randomization;
- exchangeability;
- absence of unmeasured confounding;
- positivity/overlap;
- consistency;
- correct causal-model specification;
- successful control of informative censoring;
- causal effect;
- treatment benefit or harm;
- clinical recommendation;
- clinician/patient presentation authority;
- autonomous clinical action.

In particular:

```text
TargetTrialProtocolV1 != randomized trial
TargetTrialEmulationPlanV1 != causal estimate
observational classification != random assignment
specified assumption != demonstrated assumption
```
