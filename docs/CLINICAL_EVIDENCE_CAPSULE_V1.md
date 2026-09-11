# Clinical Evidence Capsule v1

Status: **experimental design contract**. This document and crate do not claim clinical validation, regulatory clearance, or authority to diagnose or treat.

## Purpose

`mycelix-clinical-evidence` defines the evidence envelope that sits between clinical computation and downstream workflows. A CDS rule, Symthaea model, trial matcher, or research analysis may compute a result, but the result is not promoted merely because execution succeeded.

The promotion boundary requires explicit answers to:

1. **Who is this about?** — subject identity.
2. **What is being asserted?** — assertion kind, statement, optional coded concept, and four-state result.
3. **Which clinical facts were actually used?** — fact IDs, roles, and optional snapshot digests.
4. **Which external evidence was relied upon?** — versioned, content-addressed guideline/protocol/study/knowledge artifacts.
5. **What information is missing?** — explicit requirements with criticality.
6. **What alternatives or contraindicating evidence exists?** — not only the favored conclusion.
7. **How uncertain is the result?** — calibrated confidence only when calibration is actually established.
8. **What exact software/knowledge/model produced it?** — version + digest + optional environment digest.
9. **For what intended use is it qualified?** — experimental, offline validation, shadow clinical, or supervised clinical.
10. **What human authority is required?** — informational, advisory, or clinician-review-required.

## Safety invariants

- Missing critical information is represented explicitly and blocks promotion.
- `Indeterminate` cannot exist without at least one missing requirement.
- Determinate recommendations, alerts, eligibility assessments, predictions, and guideline-applicability conclusions require fact evidence.
- Recommendations and safety alerts require at least one content-addressed evidence source.
- Fact evidence IDs and evidence source IDs are unique within a capsule.
- Execution identity is content-addressed; an engine/model/rule name alone is insufficient.
- Numerical confidence must be finite and in `[0, 1]`; absence is preferable to invented precision.
- Clinician-review-required authority cannot be marked `NotRequired`.
- Approved/rejected human review must identify reviewer and review time.
- v1 intentionally defines **no autonomous therapeutic authority**.

## Relationship to the stack

```text
FHIR / device / patient / clinician source
                |
                v
       Clinical Semantics Kernel
       typed ClinicalFact + provenance
                |
                v
       CDS / Symthaea / CQL / trial logic
                |
                v
       Clinical Evidence Capsule
   facts + sources + missing data + execution
     + uncertainty + intended use + authority
                |
                v
          promotion decision
                |
       +--------+---------+
       |                  |
    blocked          permitted next stage
       |                  |
 reconciliation      human/clinical workflow
```

## Qualification principle

Qualification belongs to an **intended use**, not to a model or algorithm in the abstract. A component validated for one population, setting, input contract, or use case does not automatically inherit authority elsewhere.

The intended progression is:

```text
Experimental
  -> ValidatedOffline
  -> ShadowClinical
  -> SupervisedClinical
```

No later level may be inferred from successful compilation, unit tests, retrospective accuracy, or the existence of a clinician-facing UI. Each transition requires its own evidence.

## Planned integrations

1. make the CDS layer emit capsules rather than unstructured recommendation strings;
2. make trial eligibility emit evidence-bearing `Eligible / Ineligible / Indeterminate / NotApplicable` assessments;
3. bind CQL/WHO SMART/ICH M11 artifacts by exact version and digest;
4. bind Symthaea execution/reproducibility capsules into `ExecutionIdentity`;
5. persist review/override/supersession lineage through Mycelix authority and audit mechanisms;
6. prevent experimental/shadow outputs from crossing into active clinical presentation through an explicit qualification gate.

The last item is intentionally called out as a separate enforcement step: metadata describing qualification is useful only when workflow code is unable to bypass it.
