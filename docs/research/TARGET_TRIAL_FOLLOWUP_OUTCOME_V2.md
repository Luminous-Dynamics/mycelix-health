# Target-Trial Follow-Up and Outcome Receipts V2

**Status:** research-only source contract. Authored tests are not executable qualification evidence.

## Purpose

V2 binds participant-level follow-up and outcome ascertainment to the exact `TargetTrialProtocolV2`, `TargetTrialEmulationPlanV2`, `CohortEntryReceiptV2`, and `TimeZeroReceiptV2` evidence chain.

The central theorem is deliberately asymmetric: truncated observation may preserve a positive event that was actually observed, but it may not turn an unobserved remainder of an outcome window into evidence of outcome absence.

## Evidence chain

```text
CohortEntryReceiptV2
        +
TimeZeroReceiptV2
        ↓
FollowUpReceiptV2
        ↓
exact relative outcome phenotype
+ exact time-zero anchor evidence
+ concrete coverage evidence
+ observable subject facts
        ↓
RelativePhenotypeEvaluationV2
        ↓
OutcomeAscertainmentReceiptV2
```

These artifacts remain upstream of risk-set construction, person-time accounting, competing-event handling, estimator execution, and causal-result claims.

## Follow-up interval

Follow-up is represented as the half-open interval:

```text
[time_zero_micros, observed_until_micros)
```

Therefore a fact whose effective time is exactly `observed_until_micros` is not treated as observed before follow-up ended.

The protocol horizon is derived as:

```text
planned_horizon_end = time_zero + protocol.follow_up.maximum_duration_micros
```

and arithmetic overflow fails closed.

A valid receipt requires:

- `observed_until_micros > time_zero_micros`;
- `observed_until_micros <= planned_horizon_end_micros`;
- exact protocol digest;
- exact emulation-plan digest;
- exact cohort-entry digest;
- exact time-zero receipt digest;
- exact subject and strategy coordinates.

## Typed follow-up end

`FollowUpEndV2` distinguishes three evidence states:

1. `PlannedHorizonComplete` — observation reached the exact protocol horizon;
2. `ProtocolEndCondition` — a protocol-declared end condition occurred, bound to exact event evidence;
3. `ObservationalCensoring` — observational follow-up ended under the exact censoring plan, bound to exact event evidence.

A planned-horizon receipt must end exactly at the protocol horizon. A protocol end condition not declared by the live protocol fails closed. An observational censoring receipt must bind the live emulation plan's exact censoring-plan identity.

No free-text end reason is accepted as methodological authority.

## Outcome evidence bundle

`OutcomeEvidenceV2` groups the semantically coupled inputs needed to ascertain one outcome:

- `outcome_id`;
- exact `RelativePhenotypeDefinitionV2`;
- exact `PhenotypeEvaluationContextV2`;
- exact subject `ClinicalFact` evidence.

This typed bundle replaces a long positional verifier API so definition/context/fact inputs are less likely to be swapped accidentally at the trust boundary.

## Outcome-definition binding

The supplied relative phenotype definition must exactly match the protocol outcome's:

- phenotype id;
- version;
- canonical definition digest.

Changing outcome-window semantics therefore changes the phenotype-definition digest and invalidates reuse under the old protocol identity.

## Time-zero anchoring

Outcome evaluation requires:

```text
context.anchor_micros == time_zero.time_zero_micros
context.anchor_evidence == time_zero_anchor_evidence_v2(time_zero)
```

Thus a timestamp alone is insufficient. Changing the source evidence supporting the same nominal time zero changes the anchor evidence and prevents silent receipt reuse.

## Outcome-window bounds

For this simple V2 target-trial model, an outcome window must satisfy:

```text
start_offset_micros >= 0
end_offset_micros <= protocol.follow_up.maximum_duration_micros
start_offset_micros < end_offset_micros
```

Therefore an outcome definition may not consume pre-time-zero observations and may not claim an ascertainment horizon beyond the protocol's declared follow-up duration.

More complex landmark, grace-period, delayed-entry, sequential-trial, recurrent-event, or cloning designs require explicit future versioned artifacts rather than weakening these rules implicitly.

## Observable facts

Every supplied fact must:

- be machine-actionable under clinical semantics;
- belong to the exact cohort-entry subject.

Only facts satisfying:

```text
fact.effective_at_micros < follow_up.observed_until_micros
```

are passed to the phenotype evaluator.

Facts at or after the half-open follow-up boundary cannot support the receipt.

## Coverage and censoring theorem

If follow-up ends before the relative outcome window ends, the evaluation context may not claim `Complete` coverage for that truncated window.

This preserves the core asymmetry:

```text
positive event observed before censoring
        → may establish Satisfied

no positive event + truncated follow-up
        → remains Indeterminate

complete outcome window + complete coverage + no event
        → may establish NotSatisfied
```

`lost to follow-up` must never silently become `no outcome`.

## Outcome receipt

`OutcomeAscertainmentReceiptV2` binds:

- exact protocol digest;
- exact emulation-plan digest;
- exact subject and strategy;
- exact cohort-entry digest;
- exact time-zero receipt digest;
- exact follow-up receipt digest;
- exact outcome id;
- exact phenotype-definition digest;
- exact evaluation-context digest;
- exact phenotype-evaluation digest;
- exact outcome operationalization;
- derived absolute outcome window;
- final `CriterionStateV2`;
- exact supporting fact digests.

`verify_outcome_ascertainment_receipt_v2()` rebuilds the receipt from the supplied live evidence and requires the rebuilt receipt digest to equal the serialized receipt digest.

## Adversarial source tests

The source test suite covers at least:

- planned horizon must end exactly at the protocol horizon;
- undeclared protocol end conditions fail closed;
- censoring-plan substitution fails closed;
- positive outcome before early censoring may remain `Satisfied`;
- no outcome under early censoring remains `Indeterminate`;
- complete-coverage claims over truncated windows fail closed;
- a fact exactly at follow-up end is not observed;
- complete full follow-up may establish `NotSatisfied`;
- outcome-definition substitution fails closed;
- changing the follow-up receipt breaks outcome replay;
- outcome windows beginning before time zero fail closed;
- outcome windows extending beyond protocol follow-up fail closed.

These authored tests are design evidence only until executed in the intended toolchain and bound to an exact commit/environment lineage.

## Methodological boundary

An outcome receipt is evidence about **ascertainment under a declared observational window**. It is not itself:

- a person-time contribution;
- a risk-set membership decision;
- a competing-risk classification;
- an adherence judgment;
- a censoring weight;
- a treatment-effect estimate;
- evidence that exchangeability or positivity holds.

Those require separate, explicitly versioned evidence artifacts.

## Non-claims

These artifacts do **not** establish:

- random assignment;
- exchangeability;
- positivity/overlap;
- consistency;
- no unmeasured confounding;
- causal effect;
- treatment benefit or harm;
- diagnostic truth;
- clinical recommendation;
- clinician-presentation authority;
- regulatory clearance;
- autonomous clinical action.

## Next layer

The next research boundary should construct estimand-specific subject analysis contributions from exact cohort-entry, follow-up, and outcome receipts. It should make person-time/risk-set membership, competing-event handling, and exclusions explicit before any estimator is allowed to consume an analysis set.
