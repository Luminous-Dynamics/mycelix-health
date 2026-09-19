# Reproducible CI vs Moving-Parent Compatibility V1

## Purpose

`mycelix-health` consumes sibling path dependencies from `Luminous-Dynamics/mycelix`.
That creates two legitimate but different questions:

1. **Reproducibility:** does this exact Health subject pass against its declared parent-source snapshot and committed dependency graph?
2. **Compatibility:** does this exact Health subject still pass against a newly sampled parent `mycelix/main` snapshot?

These questions must never share one authority label.

A second separation is equally important:

`ProductSubject != QualificationHarness`.

The candidate product subject may declare product inputs, but it does not supply the evidence interpreter that declares its own CI result authoritative.

## Evidence classes

### Lane A — pinned reproducible repository CI

Workflow:

`.github/workflows/ci-reproducible-pinned.yml`

Initial trigger policy: **manual only** via `workflow_dispatch`.

The operator supplies an exact 40-character Health subject SHA. The workflow:

1. checks out that exact Health SHA and proves `HEAD == subject_sha`;
2. proves the candidate does not already own the reserved `_ci` secondary-checkout namespace;
3. separately checks out the exact reviewed harness at `github.workflow_sha`;
4. reads the product subject's `release/ci-parent-source-profile-v1.json`;
5. checks out the exact declared parent Mycelix commit;
6. runs adversarial materializer, source-verifier, receipt-builder, and final-verifier contract tests from the harness;
7. materializes sibling source in `pinned` mode using the harness materializer;
8. independently verifies the source-materialization receipt;
9. stores mutable diagnostics/evidence under `$RUNNER_TEMP`, outside both product and harness checkouts;
10. binds `Cargo.lock`, `flake.lock`, `rust-toolchain.toml`, source profile, harness tools, exact workflow bytes, and action pins;
11. evaluates the committed `.#ci` Nix dev shell without rewriting `flake.lock`;
12. requires `cargo metadata --locked`;
13. runs workspace formatting, host-target Clippy with `-D warnings`, and host-target tests;
14. proves the product has no tracked/index drift and the harness is fully clean;
15. writes a canonical `ReproducibleRepositoryCIEvidence` receipt only after every mandatory gate succeeds;
16. independently verifies that final receipt and emits `VerifiedReproducibleRepositoryCIEvidence`.

A partially executed or failed workflow may retain diagnostic artifacts beginning with:

`authority=ExecutionDiagnosticsOnly`

but cannot contain a verified PASS receipt merely because an artifact archive exists.

### Lane B — moving-parent compatibility

Workflow:

`.github/workflows/ci-moving-parent-compatibility.yml`

Initial trigger policy: **manual only** via `workflow_dispatch`.

The product and harness identities are fixed exactly as in the pinned lane, but the parent dependency checkout deliberately samples current `mycelix/main`.

The source materializer runs in `compatibility` mode and the independent source verifier must emit:

`VerifiedCompatibilityObservation`

After all build/test gates, the canonical final receipt may contain only:

`CompatibilityOnly`

and the independent final verifier may contain only:

`VerifiedCompatibilityOnlyObservation`

A completely green compatibility result therefore means only:

> this exact Health subject was compatible with the exact sampled parent commit/tree under the recorded Health lock, reviewed harness, and execution environment.

It can never satisfy pinned qualification or clinical promotion prerequisites.

## Evidence hierarchy

The v1 evidence chain is:

```text
ExecutionDiagnosticsOnly
        |
        v
ParentSourceMaterializationReceipt
        |
        v
IndependentParentSourceVerification
        |
        v
Locked build / lint / test gates
        |
        v
CanonicalCIEvidenceReceipt
        |
        v
IndependentCIEvidenceVerification
```

The last step is intentionally separate from the receipt builder.

A successful pinned chain ends at:

`VerifiedReproducibleRepositoryCIEvidence`

A successful moving-parent chain ends at:

`VerifiedCompatibilityOnlyObservation`.

## Producer/verifier separation

The parent-source materializer and verifier are different programs:

- `scripts/materialize-parent-mycelix.py`
- `scripts/verify-parent-source-materialization.py`

Likewise the final CI receipt producer and verifier are different programs:

- `scripts/build-ci-evidence-receipt.py`
- `scripts/verify-ci-evidence-receipt.py`

Neither verifier imports its producer.

The final verifier independently rechecks:

- canonical receipt representation and sidecar;
- exact product commit and Git tree;
- product tracked/index cleanliness;
- exact harness commit and Git tree;
- fully clean harness state;
- exact executing workflow SHA/ref/path/bytes;
- exact harness tool digests;
- action pins present in the exact workflow bytes;
- parent-source verification report and its sidecar;
- source profile/materializer/verifier cross-bindings;
- `Cargo.lock`, `flake.lock`, `rust-toolchain.toml` and metadata hashes;
- exact Nix CI dev-shell derivation;
- GitHub run/attempt/runner identity;
- pinned versus compatibility authority semantics.

Adversarial tests include semantic receipt tampering with a recomputed sidecar, product/harness substitution, tool-hash substitution, source-verification substitution, tracked product drift, dirty harness state, execution-identity substitution, and create-only verification artifacts.

## Evidence-directory isolation

Mutable evidence is stored under `$RUNNER_TEMP`, not under either source checkout.

The product subject must not own `_ci` before the harness is introduced. `_ci` is reserved for the exact harness and parent dependency checkouts.

The receipt builder and independent verifier reject an evidence directory located inside either product or harness checkout.

This prevents a candidate from pre-seeding authoritative evidence paths or making evidence bytes part of the product tree being evaluated.

## Symlink and destination containment

Source exports may contain symlinks only when the resolved target remains inside the same exported tree. Absolute links and links escaping the export root fail closed.

Two declared destinations that resolve to the same filesystem path fail before materialization. Existing sibling source trees are replaced only when the caller explicitly requests `--replace-existing`.

These constraints prevent a source receipt from covering one byte graph while Cargo follows undeclared filesystem state.

## Lock semantics

Both lanes use the committed Health `Cargo.lock` with `--locked`.

For the pinned lane:

`pinned source + reviewed Cargo.lock + locked execution`

is the required dependency theorem.

For the compatibility lane, `--locked` deliberately asks whether the sampled parent remains compatible with the Health subject's already-declared graph. If parent changes require dependency-graph reconciliation, compatibility fails instead of silently rewriting the Health lock.

A separate repair workflow may generate a candidate lock, but candidate generation is never qualification authority.

## Execution environment

Both lanes use:

- `ubuntu-24.04` as the transport runner class;
- pinned `actions/checkout` implementation;
- pinned Nix installer action and Nix 2.19.2 bootstrap;
- committed `flake.lock` + `rust-toolchain.toml`;
- `nix develop --no-write-lock-file .#ci` for Rust/Cargo/native dependencies;
- pinned artifact-upload implementation.

The GitHub runner image is recorded as an execution fingerprint. The model does **not** claim bit-for-bit hermeticity across arbitrary kernels, CPUs, or hosted-runner images.

## Manual-only admission during runner congestion

These workflows are intentionally staged with `workflow_dispatch` only.

They must not be automatically activated while the current Actions backlog is infrastructure-indeterminate. Staging their source off-PR creates no runner demand.

When runner capacity is healthy and the workflows are reviewed/merged, automatic trigger policy can be considered separately. Trigger expansion is an operations change, not evidence of product correctness.

## Relationship to current clinical-assurance qualification

This tranche is not part of the frozen #139/#140 runtime-trust subject.

P0 #141 still requires the current clinical assurance `Cargo.lock` to be generated, audited, committed into a replacement exact subject, and then qualified by actual executable evidence.

This CI infrastructure is reusable for later subjects; it does not retroactively qualify `97e991a7...`.

## Non-claims

Neither lane establishes by itself:

- scientific validity of Symthaea predictions;
- clinical calibration, effectiveness, or safety;
- OOD detector scientific adequacy;
- clinician or patient presentation authority;
- diagnosis, prescribing, dispensing, administration, or treatment authority;
- HIPAA, GDPR, POPIA, medical-device, or other legal/regulatory compliance;
- regulatory clearance or approval.

A verified repository-CI PASS is software/build evidence. A verified compatibility PASS is sampled ecosystem-compatibility evidence. Neither is clinical truth.
