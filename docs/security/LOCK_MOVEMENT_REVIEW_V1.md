# Cargo.lock Movement Review Boundary V1

## Purpose

A mechanically valid Cargo.lock repair candidate can still contain collateral dependency movement that is undesirable or unexplained.

This contract ensures that every movement observed by the lock audit is explicitly addressed before a candidate can be considered for a separate human acceptance decision.

It does **not** automate or authenticate that acceptance decision.

## Required upstream evidence

The review stack may be created only for an exact repair candidate that already has both:

`RepairCandidateOnly`

and independent:

`VerifiedRepairCandidateOnly`

The review template, sealed statement, and coverage verification bind the SHA-256 identity of that exact independent verification receipt in addition to the candidate report, candidate lock, audit, subject, and harness identities.

Therefore the review line is explicitly downstream of independent executable candidate verification rather than merely downstream of the producer's candidate report.

This binding does not upgrade `VerifiedRepairCandidateOnly`; it only makes the evidence dependency explicit.

## Evidence stages

### 1. Review template

Produced by:

`scripts/prepare-lock-movement-review.py`

Authority:

`ReviewTemplateOnly`

Every audit movement is represented exactly once with a stable movement ID and starts as:

`disposition = unreviewed`

The template binds:

- exact candidate report digest;
- exact independent candidate-verification digest;
- candidate lock digest;
- lock-audit digest;
- product subject identity;
- repair harness identity.

The template is intentionally editable human input. It is not final canonical review evidence after editing.

### 2. Sealed review statement

Produced by:

`scripts/seal-lock-movement-review.py`

Authority:

`ReviewStatementOnly`

Before sealing, the sealer rechecks the canonical candidate report, canonical independent candidate verification and their sidecars. It requires the candidate verification to be a passing `VerifiedRepairCandidateOnly` receipt bound to the supplied candidate report, candidate lock, subject, and audit.

The sealer then re-derives every movement from `lock-audit.json` and requires:

- no missing movement;
- no extra movement;
- no duplicate movement ID;
- exact category/value equality;
- an explicit disposition for every movement;
- non-empty rationale for every movement.

Allowed v1 dispositions are:

- `expected`
- `necessary`
- `accepted`
- `rejected`

The sealer canonicalizes the statement and writes a create-only SHA-256 sidecar.

`review_reference` is preserved only as context. V1 does not cryptographically authenticate the reviewer or that external reference.

### 3. Independent coverage verification

Produced by:

`scripts/verify-lock-movement-review.py`

Terminal authority:

`MechanicalReviewCoverageComplete`

The verifier independently rechecks:

- candidate report canonical bytes and sidecar;
- independent `VerifiedRepairCandidateOnly` canonical bytes and sidecar;
- candidate-verification binding to the candidate report/lock/subject/audit;
- sealed review canonical bytes and sidecar;
- the exact movement set reconstructed from `lock-audit.json`.

It records whether any movement is explicitly rejected and may be invoked with `--require-no-rejections` when a caller needs that stricter mechanical gate.

## Core authority boundary

```text
RepairCandidateOnly
        |
        v
VerifiedRepairCandidateOnly
        |
        v
ReviewTemplateOnly
        |
        v
human edits dispositions/rationales
        |
        v
ReviewStatementOnly
        |
        v
independent reconstruction
        |
        v
MechanicalReviewCoverageComplete
```

None of the review authorities means:

```text
human identity authenticated
        or
candidate approved for commit
        or
repository qualified
```

In particular:

`MechanicalReviewCoverageComplete != ApprovedDependencyGraph`

## Why sealing is separate from editing

Human editing tools may reformat JSON, reorder keys, or leave stale digest sidecars.

The editable template therefore is not treated as canonical evidence after modification.

The sealer consumes the edited draft, revalidates every candidate/verification binding and movement identity, and creates a fresh canonical statement plus sidecar.

This prevents JSON formatting behavior from becoming review identity semantics.

## Relationship to P0 #141

For the current clinical-assurance lock repair:

1. generate `RepairCandidateOnly`;
2. independently establish `VerifiedRepairCandidateOnly`;
3. create the bound movement review template;
4. explicitly disposition every movement;
5. seal `ReviewStatementOnly`;
6. independently establish `MechanicalReviewCoverageComplete`;
7. inspect whether any movement is rejected and review every rationale;
8. make a separate human decision whether the exact candidate lock should be copied into a new branch from the frozen clinical subject;
9. if accepted, create a new immutable lock-bearing product subject;
10. run fresh exact-head `--locked` qualification.

The review stack never modifies the old clinical subject.

## Non-claims

This contract does not establish:

- cryptographic reviewer identity;
- organizational approval;
- permission to commit or merge the candidate;
- repository qualification PASS;
- scientific or clinical validity;
- clinical safety or effectiveness;
- presentation, diagnosis, prescribing, dispensing, administration, or treatment authority;
- regulatory or legal compliance, clearance, or approval.
