# Portable Cargo.lock Repair Candidate V1

## Purpose

P0 #141 requires a reviewed, committed `Cargo.lock` before the clinical-assurance subject can receive exact `--locked` qualification.

The existing GitHub Actions repair workflow is currently blocked by hosted-runner admission. Lock reconciliation therefore must not depend on GitHub Actions as its only execution transport.

This contract defines a portable repair-only harness that can run from:

- a local trusted Nix workstation;
- a self-hosted execution environment;
- GitHub Actions;
- another transport that preserves the same exact input identities.

Transport does not change repair authority.

## Programs

Generator:

`scripts/prepare-cargo-lock-candidate.py`

Independent verifier:

`scripts/verify-cargo-lock-candidate.py`

Adversarial end-to-end harness:

`scripts/check-portable-lock-candidate.py`

The verifier does not import the generator.

## Core safety property

The caller's product checkout is never the reconciliation worktree.

The generator creates a disposable detached Git worktree at the exact requested product commit:

```text
caller checkout
      |
      +-- exact commit/tree identity
      |
      v
fresh detached worktree
      |
      +-- pinned parent source materialization
      +-- committed flake.lock / rust-toolchain.toml
      +-- nix develop .#ci
      +-- cargo metadata (reconcile)
      +-- cargo metadata --locked
      |
      v
review-only candidate artifacts
```

The caller's checked-out `Cargo.lock` is not modified by reconciliation.

## Source identity

The exact product subject supplies `release/ci-parent-source-profile-v1.json`.

Parent source is materialized through the separately reviewed CI harness:

- `scripts/materialize-parent-mycelix.py`;
- `scripts/verify-parent-source-materialization.py`.

The generator refuses to continue unless independent parent-source verification yields:

`VerifiedPinnedDependencySourceMaterialization`.

## Environment identity

Reconciliation uses the exact product subject's committed:

- `flake.lock`;
- `rust-toolchain.toml`;
- `.#ci` Nix dev shell.

The generator records:

- Nix current system;
- flake metadata;
- CI dev-shell derivation;
- `rustc -vV`;
- `cargo -vV`;
- unlocked Cargo metadata;
- locked Cargo metadata;
- pre/post lock hashes.

The independent verifier recreates the exact subject in a second detached worktree and independently re-evaluates the Nix CI shell and `cargo metadata --locked` using the candidate lock.

## Candidate identity

The canonical candidate report schema is:

`mycelix-health-cargo-lock-candidate/v1`

Its authority is always:

`RepairCandidateOnly`.

It binds:

- exact product commit and tree;
- exact repair harness commit and tree;
- generator/materializer/source-verifier hashes;
- exact independently verified parent-source report;
- committed lock digest;
- candidate lock digest;
- whether lock bytes changed;
- required package set;
- Nix system / flake / toolchain / CI derivation identities;
- independently computed dependency movement audit.

## Independent verification

The verification schema is:

`mycelix-health-cargo-lock-candidate-verification/v1`.

Its terminal authority is:

`VerifiedRepairCandidateOnly`.

The verifier independently checks:

1. candidate report canonical representation and sidecar;
2. exact product commit/tree from Git object identity;
3. exact repair harness commit/tree and tool hashes;
4. `Cargo.lock.before` exactly equals the committed subject lock bytes;
5. candidate lock hash and changed flag;
6. full dependency movement audit;
7. exact required package set;
8. canonical candidate source-verification report and sidecar;
9. independently reconstructed parent-source verification in a fresh worktree;
10. exact `flake.lock` and Rust toolchain hashes;
11. exact Nix CI dev-shell derivation;
12. fresh `rustc -vV` and `cargo -vV` identities;
13. fresh `cargo metadata --locked` using the candidate lock;
14. required packages present in locked metadata;
15. no product mutation beyond the installed candidate `Cargo.lock` inside the disposable worktree;
16. diagnostic lock diff agrees with the reconstructed worktree diff.

The verifier emits its own environment and locked-metadata hashes rather than accepting producer claims about those executions.

## Dependency movement audit

The audit separates:

- added/removed local packages;
- added/removed registry package tuples;
- registry package names whose version sets changed;
- added/removed git-source package tuples.

A candidate can be mechanically valid while still containing undesirable collateral movement.

Therefore:

```text
VerifiedRepairCandidateOnly
!= reviewed dependency graph
!= permission to commit
!= exact qualification
```

Registry/git movement remains a human-review or separately qualified policy decision.

## Required package pins

Callers may specify one or more `--require-package` arguments.

The clinical-assurance #141 repair should require at minimum:

`mycelix-symthaea-clinical-distribution-trust-v3`.

The exact required-package set is bound into the candidate report and must match the verifier expectation.

## Adversarial test harness

`check-portable-lock-candidate.py` creates miniature product and parent Git repositories plus a deterministic fake `nix` executable.

This tests worktree and evidence semantics without network downloads.

The suite covers:

- successful repair-only generation;
- original caller `Cargo.lock` remains byte-identical;
- successful independent executable verification;
- candidate lock tampering;
- semantic audit tampering with a recomputed report sidecar;
- verifier required-package substitution.

The fixture parent repository uses the same normalized `Luminous-Dynamics/mycelix` origin required by the real source theorem.

## Intended #141 use

A legitimate repair path is:

```text
exact clinical subject
      +
clean exact parent Mycelix checkout
      +
portable reviewed repair harness
      |
      v
RepairCandidateOnly
      |
      v
VerifiedRepairCandidateOnly
      |
      v
review lock movement
      |
      +-- reject -> no subject change
      |
      `-- accept
             |
             v
commit exact candidate lock
             |
             v
NEW exact product subject
             |
             v
fresh --locked qualification
```

No candidate or verification artifact retroactively changes the identity or evidence status of the old subject.

## Non-claims

Neither `RepairCandidateOnly` nor `VerifiedRepairCandidateOnly` establishes:

- repository qualification PASS;
- scientific validity;
- patient/population calibration;
- OOD detector adequacy;
- clinical effectiveness or safety;
- clinician/patient presentation authority;
- diagnosis, prescribing, dispensing, administration, or treatment authority;
- legal/regulatory compliance, clearance, or approval.
