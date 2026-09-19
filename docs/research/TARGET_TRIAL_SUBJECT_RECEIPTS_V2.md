# Target-Trial Subject Receipts V2

**Status:** research-only source contract. Authored tests are not executable qualification evidence.

## Purpose

V2 binds participant-level observational cohort construction to exact `TargetTrialProtocolV2`, `TargetTrialEmulationPlanV2`, and anchored relative phenotype V2 evidence.

The key correction from V1 is ordering: observational time zero is established first, then reusable eligibility semantics are evaluated at that exact anchor.

## Evidence chain

```text
TimeZeroReceiptV2
        ↓
exact time-zero receipt digest becomes phenotype anchor evidence
        ↓
RelativePhenotypeEvaluationV2 for eligibility
        ↓
EligibilityReceiptV2
        ↓
observational strategy classification at same time zero
        ↓
StrategyClassificationReceiptV2
        ↓
CohortEntryReceiptV2
        ↓
CohortManifestV2
```

## Time-zero receipt

A time-zero receipt binds:

- exact protocol digest;
- exact emulation-plan digest;
- exact subject;
- exact selected observational data-source id + identity;
- exact timestamp;
- exact time-zero operationalization from the live plan;
- exact source/event evidence.

The timestamp must lie in the declared half-open source period `[period_start, period_end)`.

After deserialization, verification re-derives the live protocol/emulation digests and rechecks the selected data-source identity and time-zero operationalization.

## Deterministic phenotype anchor

`time_zero_anchor_evidence_v2()` derives a phenotype evidence identity from the exact time-zero receipt digest under namespace:

`mycelix/target-trial-time-zero-receipt/v2`

Eligibility evaluation requires both:

```text
context.anchor_micros == time_zero.time_zero_micros
context.anchor_evidence == derived exact time-zero receipt evidence
```

Changing only the time-zero source evidence therefore changes the receipt digest and invalidates reuse of an old eligibility context/evaluation.

## Eligibility receipt

Eligibility is not represented as a free boolean. The builder executes the exact reusable relative phenotype definition against:

- the exact time-zero anchor;
- concrete coverage evidence;
- exact subject facts.

The final state must be `Satisfied`.

The receipt binds:

- exact time-zero receipt digest;
- exact phenotype-definition digest;
- exact evaluation-context digest;
- exact phenotype-evaluation digest;
- exact eligibility operationalization;
- exact supporting fact snapshot identities.

`verify_eligibility_receipt_v2()` rebuilds the evaluation from the supplied definition/context/facts and requires the rebuilt receipt identity to equal the serialized receipt identity.

## Strategy classification

Simple V2 requires observational classification at exactly time zero:

```text
classified_at_micros == time_zero_micros
```

The receipt binds the exact strategy classification rule from the live emulation plan and exact classification evidence.

This is observational classification, not random assignment.

## Cohort entry

Cohort composition requires:

- exact live protocol/emulation binding;
- exact time-zero receipt;
- reproducible eligibility receipt;
- live-verified strategy receipt;
- one subject across all evidence;
- one aligned time zero.

The entry contains only identities/coordinates needed to identify the selected participant-trial contribution. It does not contain an effect estimate.

## Cohort manifest

Simple V2 permits one cohort entry per subject. This deliberately excludes sequential-trial and cloning designs, which require explicit future versioned artifacts.

Manifest entries are sorted canonically and deserialized manifests must already be in canonical order. Duplicate subjects fail closed.

## Methodological boundary

This design follows the target-trial discipline that eligibility, observational treatment classification, and follow-up start should align at time zero in simple emulations. More complex grace-period, cloning or sequential-trial designs must not weaken this invariant implicitly.

## Non-claims

These artifacts do **not** establish:

- actual random assignment;
- exchangeability;
- positivity/overlap;
- consistency;
- no unmeasured confounding;
- causal effect;
- treatment benefit or harm;
- outcome absence;
- clinical recommendation;
- diagnostic truth;
- clinician presentation authority;
- regulatory clearance;
- autonomous action.

## Next layer

Follow-up/outcome V2 should consume the exact `CohortEntryReceiptV2`, then evaluate each protocol outcome using the reusable relative outcome phenotype anchored to the same time-zero receipt. Early censoring without a positive outcome must yield `Indeterminate`, never an invented negative outcome.
