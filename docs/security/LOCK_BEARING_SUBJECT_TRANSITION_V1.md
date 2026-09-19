# Lock-Bearing Replacement Subject Transition V1

## Purpose

The portable repair and review stack can establish that a `Cargo.lock` candidate is mechanically valid, independently verified, and explicitly reviewed without rejected dependency movement.

One dangerous manual step remained: copying those exact reviewed bytes into a new Git subject without accidentally changing another file, choosing the wrong parent, or qualifying a commit whose tree differs from what was reviewed.

This contract closes that gap without automating the human acceptance decision.

## Authority chain

```text
RepairCandidateOnly
        |
        v
VerifiedRepairCandidateOnly
        |
        v
ReviewStatementOnly
        |
        v
MechanicalReviewCoverageComplete
        |
        |  separate human acceptance decision
        v
PreparedReplacementSubjectOnly
        |
        |  human creates child commit
        v
VerifiedReplacementSubjectIdentityOnly
        |
        v
fresh exact-head --locked qualification
```

No authority in this document is clinical authority.

## Prepared replacement tree

`scripts/prepare-lock-bearing-subject.py` consumes:

- canonical `lock-candidate.json` plus sidecar;
- canonical `lock-candidate-verification.json` plus sidecar;
- canonical sealed `lock-movement-review-statement.json` plus sidecar;
- canonical `lock-movement-review-verification.json` plus sidecar;
- exact `Cargo.lock.candidate` bytes;
- a Git checkout containing the frozen subject object.

It requires:

- `RepairCandidateOnly`;
- successful `VerifiedRepairCandidateOnly`;
- `ReviewStatementOnly` bound to that exact independent candidate verification;
- `MechanicalReviewCoverageComplete` bound to that exact review statement and independent candidate verification;
- complete review with zero rejected movement;
- exact subject commit/tree equality;
- exact candidate-lock digest equality throughout the evidence chain.

It then creates a disposable detached worktree at the frozen subject, copies only the exact candidate `Cargo.lock`, and requires the worktree status to be exactly:

```text
 M Cargo.lock
```

A temporary Git index is initialized from the frozen subject tree. Only the candidate `Cargo.lock` is staged in that temporary index, and `git write-tree` produces the exact future replacement tree SHA.

The output authority is:

`PreparedReplacementSubjectOnly`

The receipt records:

- frozen subject commit/tree;
- proposed replacement tree;
- required changed path `["Cargo.lock"]`;
- before/candidate lock SHA-256;
- exact upstream repair/review evidence digests;
- exact preparation harness identity.

The program creates **no commit** and moves **no branch**.

## Human commit remains explicit

If the reviewed graph is accepted, the operator may create a new commit whose:

- sole parent is the frozen clinical subject;
- tree is exactly the proposed replacement tree;
- only changed path is `Cargo.lock`;
- `Cargo.lock` bytes exactly equal the reviewed candidate.

The commit message, author, committer, signature, and branch name do not alter the dependency-tree theorem. Organizational policy may separately require signatures or approved authorship.

## Independent replacement verification

`scripts/verify-lock-bearing-subject.py` does not trust the prepared plan's upstream evidence digests in isolation.

It independently reloads and validates the exact candidate, candidate verification, sealed review statement, review verification, and their sidecars, then requires the prepared plan to bind those exact bytes.

For a proposed replacement commit it independently proves:

1. the object exists and is a commit;
2. its tree equals the precomputed replacement tree;
3. it has exactly one parent;
4. that sole parent is exactly the frozen subject SHA;
5. the frozen parent tree matches the reviewed subject tree;
6. the parent-to-child changed path set is exactly `["Cargo.lock"]`;
7. child `Cargo.lock` bytes hash to the reviewed candidate digest;
8. parent `Cargo.lock` bytes hash to the recorded before digest.

Successful verification emits:

`VerifiedReplacementSubjectIdentityOnly`

## Important non-equivalence

```text
VerifiedReplacementSubjectIdentityOnly
    != build/test qualification
    != merge approval
    != clinical evidence
```

The replacement commit must still receive fresh exact-head execution using the committed lock and the pinned dependency-source theorem.

## Adversarial harness

`scripts/check-lock-bearing-subject.py` exercises the transition with miniature Git repositories and synthetic canonical evidence.

It covers at least:

- exact Cargo.lock-only child commit;
- same proposed tree with wrong/no parent;
- extra-file movement;
- wrong lock bytes;
- prepared-plan upstream evidence substitution with a recomputed sidecar;
- substitution of the independently verified repair evidence itself.

Authored tests are not execution evidence until actually run.

## P0 #141 consequence

For the current clinical-assurance repair, the intended sequence becomes:

```text
97e991a75d0565f1e7769c2a11ac94e0b2f2aded
        |
        v
reviewed portable Cargo.lock candidate
        |
        v
complete no-rejection movement review
        |
        v
precompute exact replacement tree
        |
        v
explicit human acceptance + child commit
        |
        v
verify exact replacement commit identity
        |
        v
fresh exact-head --locked qualification
```

The old subject remains unchanged and unqualified.

## Non-claims

This transition does not establish:

- reviewer cryptographic identity;
- organizational approval;
- repository qualification PASS;
- scientific validity;
- calibration or OOD detector adequacy;
- clinical safety or effectiveness;
- clinician/patient presentation authority;
- diagnosis, prescribing, dispensing, administration, or treatment authority;
- regulatory or legal compliance, clearance, or approval.
