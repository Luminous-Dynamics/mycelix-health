# Clinical Phenotype V2 — Anchored Relative Windows

Status: research-only source tranche. Not executed or qualified.

## Why V2 exists

V1 safely evaluates a phenotype inside one absolute observation window, but it combines two concepts that should be separate for reusable longitudinal research:

1. **definition semantics** — what evidence and relative time window define the phenotype;
2. **evaluation circumstances** — the subject-specific anchor timestamp and concrete data-completeness evidence available for that evaluation.

That distinction is required for target-trial eligibility/outcomes, repeated longitudinal evaluation, multicenter research, and any phenotype intended to be reused across subjects with different time anchors.

## V2 evidence model

```text
RelativePhenotypeDefinitionV2
  - id/version
  - relative [start_offset,end_offset)
  - criterion tree
  - exact terminology versions
  - required coverage-domain names
  - definition evidence
           |
           +----------------------+
                                  v
PhenotypeEvaluationContextV2
  - anchor timestamp
  - exact anchor evidence
  - concrete completeness status/evidence per required domain
  - additional source/context evidence
                                  |
                                  v
RelativePhenotypeEvaluationV2
  - exact definition digest
  - exact context digest
  - exact subject
  - derived absolute [start,end)
  - criterion states
  - exact ClinicalFact snapshot identities
  - final Satisfied/NotSatisfied/Indeterminate
```

## Definition invariants

- relative start must be strictly before relative end;
- criterion ids are unique and bounded in depth/count;
- every criterion concept has exact coding-system/code/version identity;
- every leaf criterion names one coverage domain declared by the definition;
- coverage **status/evidence is not part of the reusable definition**;
- definition digest is independent of subject, anchor timestamp, dataset completeness, and evaluated facts.

Required coverage-domain names are encoded in canonical sorted order so declaration ordering alone does not change definition identity.

## Evaluation-context invariants

The context must supply every required coverage domain exactly once and no undeclared domain.

Each domain has:

- `Complete`;
- `Incomplete`;
- `Unknown`;
- exact source/evidence identity.

The context also binds one exact anchor timestamp and exact anchor-evidence artifact. Changing the timestamp, anchor evidence, coverage status, coverage evidence, or context evidence changes context identity while leaving definition identity unchanged.

## Derived window

The absolute window is derived only by checked arithmetic:

```text
absolute_start = anchor + start_offset
absolute_end   = anchor + end_offset
```

Overflow fails closed.

The interval is half-open `[absolute_start, absolute_end)`.

## Absence semantics

V2 preserves the core epistemic rule from V1:

```text
positive matching fact observed
    -> may establish positive evidence even with incomplete coverage

no matching fact + Complete coverage
    -> may establish evidence of absence / insufficient presence

no matching fact + Incomplete or Unknown coverage
    -> Indeterminate
```

A contradictory positive fact refutes a `FactAbsent` criterion even when coverage is incomplete.

## Subject and fact integrity

All supplied facts are validated before time-window filtering.

Therefore a malformed or cross-patient fact cannot disappear merely because it falls outside the derived window.

Duplicate `ClinicalFactSnapshotDigestV1` identities fail closed.

## Target-trial use

Target-trial protocol V2 should reference the reusable V2 definition digest.

A subject's exact time-zero receipt can then become the phenotype evaluation anchor. This allows one target-trial eligibility/outcome definition to be applied across participants without inventing a new protocol-level phenotype digest for every subject.

Coverage completeness remains participant/window-specific evidence and therefore belongs in the evaluation context.

## Qualification targets

Before promotion beyond source-staged research code:

- workspace format;
- Clippy with warnings denied;
- unit/integration tests;
- definition identity stable across different anchors;
- context/evaluation identity changes on anchor substitution;
- context/evaluation identity changes on coverage-evidence substitution;
- exact required-domain set enforcement;
- incomplete-coverage absence remains `Indeterminate`;
- positive evidence survives incomplete coverage;
- facts outside derived window do not affect the criterion;
- cross-patient facts fail even when outside the derived window;
- duplicate fact identity rejection;
- checked-time-overflow rejection;
- exact dependency-graph qualification under the repository assurance process.

## Non-claims

V2 does **not** establish diagnosis, disease absence, treatment indication, treatment benefit/harm, causal effect, clinical recommendation, clinician presentation authority, patient alert authority, or autonomous clinical action.

It establishes only a reusable and provenance-bound research phenotype evaluation contract.