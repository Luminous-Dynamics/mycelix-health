# Target-Trial Cohort Receipts V1

Status: research-only source tranche. Not executed or qualified.

## Purpose

V1 binds subject-level observational cohort assembly to one exact target-trial protocol and one exact observational emulation plan. It is an evidence/provenance layer for research cohort construction, not a causal-result layer.

The methodological boundary follows the TARGET reporting model: eligibility, treatment-strategy classification, time zero/follow-up start, outcomes, estimands, assumptions, and observational operationalization are distinct objects. In V1, subject eligibility, observational strategy classification, and follow-up start must align at one exact time zero.

## Evidence chain

```text
TargetTrialProtocolV1
        +
TargetTrialEmulationPlanV1
        |
        +--> EligibilityReceiptV1
        |      - exact eligibility phenotype evaluation
        |      - exact eligibility operationalization
        |      - exact evidence-fact snapshots
        |      - exact eligibility anchor
        |
        +--> TimeZeroReceiptV1
        |      - exact selected data source
        |      - exact source identity
        |      - exact time-zero operationalization
        |      - exact time-zero evidence
        |
        +--> StrategyClassificationReceiptV1
               - exact strategy
               - exact classification rule
               - exact classification evidence
               - exact TimeZeroReceipt digest

                 |
                 v
        CohortEntryReceiptV1
                 |
                 v
          CohortManifestV1
```

## Critical invariants

### Exact protocol/emulation binding

Every subject-level receipt carries the exact protocol and emulation-plan digests. Composition re-derives those identities from the supplied protocol/plan and fails closed on mismatch.

### Eligibility is executed, not merely declared

`build_eligibility_receipt_v1` executes the native Mycelix phenotype evaluator over validated `ClinicalFact` inputs. The phenotype id/version/digest must match the protocol eligibility definition, and the resulting state must be `Satisfied`.

The receipt preserves the exact phenotype-evaluation digest and exact evidence-fact snapshot digests.

### Plan-bound fields are revalidated after deserialization

Composition does not assume that a Rust constructor was used. It independently rechecks:

- eligibility definition digest against the protocol;
- eligibility operationalization against the emulation plan;
- selected data-source id and exact source identity;
- time zero against the selected source period;
- time-zero operationalization against the emulation plan;
- strategy id against the protocol;
- strategy-classification rule against the emulation plan;
- strategy receipt against the exact time-zero receipt digest.

A receipt with the correct top-level protocol digest but substituted nested evidence is rejected.

### One exact V1 time zero

V1 requires:

```text
eligibility_anchor_micros
    == time_zero_micros
    == classified_at_micros
```

This is intentionally strict. It prevents a later treatment-classification event from silently becoming baseline assignment and follow-up start.

More complex designs involving grace periods, cloning/censoring/weighting, dynamic treatment regimes, sequential trials, or delayed assignment require future explicitly versioned artifacts.

### Cohort identity

A V1 cohort contains at most one entry per subject. Entries are canonically sorted before manifest creation, and a deserialized manifest in noncanonical order is rejected.

The manifest binds:

- exact protocol/emulation identities;
- exact cohort-entry digests;
- exact subject/strategy/time-zero tuple per entry;
- exact cohort-assembly software/environment;
- assembly timestamp.

Changing cohort membership or any subject-level receipt changes cohort identity.

## TARGET alignment

TARGET distinguishes the hypothetical randomized target trial from how its components are operationalized using observational data. V1 preserves that distinction:

- `TargetTrialProtocolV1.assignment` is hypothetical trial design;
- `StrategyClassificationReceiptV1` is observational classification;
- no receipt claims that observational classification was random assignment.

Time zero is aligned with eligibility, strategy classification, and follow-up start to prevent silent design-time misalignment.

## Qualification targets

Before promotion beyond source-staged research code:

- workspace format;
- Clippy with warnings denied;
- unit/integration tests;
- cross-subject substitution rejection;
- protocol/emulation substitution rejection;
- eligibility-operationalization substitution rejection;
- data-source identity substitution rejection;
- time-zero operationalization substitution rejection;
- strategy-rule substitution rejection;
- exact time-zero receipt substitution rejection;
- duplicate-subject cohort rejection;
- canonical cohort ordering/identity tests;
- exact dependency-graph qualification under the repository assurance process.

## Explicit non-claims

These receipts do **not** establish:

- randomized treatment assignment;
- exchangeability;
- positivity/overlap;
- consistency;
- absence of unmeasured confounding;
- correct treatment adherence classification;
- causal effect;
- treatment benefit or harm;
- clinical recommendation;
- diagnostic truth;
- clinician presentation authority;
- patient alert authority;
- autonomous clinical action.

`CohortManifestV1` means only that the declared observational cohort membership can be traced to exact versioned research evidence and operationalizations.