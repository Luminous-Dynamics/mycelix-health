# Target-Trial Protocol V2 — Relative Phenotype Contract

**Status:** research-only source contract. Authored code/tests are not executable qualification evidence.

## Purpose

V2 replaces target-trial phenotype references that implicitly depended on absolute observation windows with reusable anchored/relative phenotype identities from `mycelix-clinical-phenotype-v2`.

The protocol specifies the hypothetical trial. The emulation plan separately specifies how that hypothetical trial is operationalized in observational data. Neither object contains a causal effect estimate.

## Core separation

```text
TargetTrialProtocolV2
    hypothetical randomized trial semantics

        !=

TargetTrialEmulationPlanV2
    observational operationalization

        !=

participant evaluation receipts
    subject + time-zero + coverage + facts

        !=

causal estimate
```

## Relative phenotype references

Eligibility and every outcome bind:

- namespace `mycelix/clinical-phenotype-definition/v2`;
- phenotype id/version;
- serializer-independent relative-phenotype definition digest.

The referenced phenotype definition owns its relative window semantics. Participant time-zero timestamps and coverage evidence are deliberately absent from protocol identity.

Therefore:

```text
change relative window semantics
    -> phenotype definition digest changes
    -> protocol digest changes

change participant time zero
    -> protocol digest unchanged
    -> participant evaluation context/evaluation identity changes
```

## Protocol V2 invariants

- exact schema version;
- non-empty protocol identity and causal question;
- rationale evidence required;
- eligibility must use the relative-phenotype-v2 namespace;
- at least two unique treatment strategies;
- hypothetical random-assignment description required;
- exact time-zero definition required;
- positive maximum follow-up and at least one declared end condition;
- at least one outcome and at least one primary outcome;
- every outcome uses a relative-phenotype-v2 identity;
- estimands reference declared, distinct strategies and declared outcomes;
- identifying assumptions are statements/evidence, not claims that assumptions hold;
- exact primary and sensitivity analysis-plan identities are preserved.

## Emulation-plan V2 invariants

- exact protocol digest;
- explicit observational-design statement;
- at least one bounded observational data source;
- every protocol strategy has exactly one observational classification rule;
- every protocol outcome has exactly one observational operationalization;
- exact eligibility and time-zero operationalizations;
- baseline-confounder measurement windows may not extend past time zero;
- exact censoring, missing-data and estimator plans;
- optional adherence/time-varying-confounding plans remain explicit;
- exact software/environment and vocabulary/mapping evidence.

## Time-zero discipline

TARGET reporting guidance distinguishes hypothetical random assignment from observational strategy classification and emphasizes aligning eligibility, classification and follow-up start at time zero. V2 keeps that alignment as a downstream subject-receipt theorem rather than encoding subject-specific timestamps in the protocol.

More complex grace-period, cloning, sequential-trial and dynamic-strategy designs require future versioned artifacts rather than weakening the simple V2 alignment contract.

## Identity

Protocol and emulation identities use explicit domain-separated BLAKE3 framing. They do not depend on JSON field order or a serializer implementation.

## Non-claims

These artifacts do **not** establish:

- randomization in the observational data;
- exchangeability;
- positivity/overlap;
- consistency;
- no unmeasured confounding;
- successful treatment classification for any participant;
- cohort membership;
- causal effect;
- treatment benefit or harm;
- diagnostic truth;
- clinical recommendation;
- clinician presentation authority;
- regulatory clearance;
- autonomous action.

## Intended next layer

Subject receipts V2 should compose in this order:

```text
exact observational time-zero candidate
        ↓
TimeZeroReceiptV2
        ↓
Relative eligibility evaluation anchored to that receipt
        ↓
EligibilityReceiptV2
        ↓
Observational strategy classification at the same time zero
        ↓
StrategyClassificationReceiptV2
        ↓
CohortEntryReceiptV2
        ↓
follow-up / relative outcome ascertainment
```
