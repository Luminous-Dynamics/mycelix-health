# CI Subject / Harness Separation V1

## Purpose

A candidate software subject must not be allowed to define the verifier that declares the same subject qualified.

For evidence-bearing CI, Mycelix Health treats these as separate exact identities:

```text
ProductSubject
!= QualificationHarness
!= DependencySource
!= DependencyGraph
!= ExecutionEnvironment
!= EvidenceReceipt
!= EvidenceVerification
```

This contract applies first to the staged reproducible-vs-compatibility CI lanes and is intended as a reusable pattern for exact-subject qualification.

## Product subject

The product subject is the exact `mycelix-health` commit supplied to a qualification or compatibility run.

It contributes product inputs such as:

- Rust/source code;
- `Cargo.toml` / committed `Cargo.lock`;
- `flake.lock` / `rust-toolchain.toml`;
- the subject-declared parent-source profile.

The subject is **not** authoritative for the materializer, source verifier, final CI receipt builder, final CI receipt verifier, or workflow implementation that interprets those inputs.

The product identity recorded in final evidence includes both:

```text
subject.sha
subject.tree_sha
```

The final builder/verifier require no tracked/index drift. Untracked state is not treated as product authority; the reserved `_ci` subtree is introduced only after the candidate is checked and is used solely for secondary checkouts.

## Qualification harness

The harness is the exact Git commit containing the workflow that is actually executing.

For GitHub Actions v1:

```text
HARNESS_SHA = github.workflow_sha
```

The run separately checks out:

1. `SUBJECT_SHA` at the workspace root;
2. `HARNESS_SHA` at `_ci/harness`;
3. the required parent Mycelix dependency snapshot at `_ci/mycelix-parent`.

Before secondary checkout, the product subject must not already contain `_ci`.

All evidence-producing/interpreting programs execute from `_ci/harness`, including:

- `scripts/materialize-parent-mycelix.py`;
- `scripts/verify-parent-source-materialization.py`;
- `scripts/build-ci-evidence-receipt.py`;
- `scripts/verify-ci-evidence-receipt.py`;
- their adversarial contract tests.

Changing any of those programs changes the harness tree and therefore the harness commit identity.

The final evidence records:

```text
harness.sha
harness.tree_sha
```

and requires the harness checkout to be fully clean, including untracked-file state.

## Workflow identity binding

Both the final receipt builder and independent final verifier require:

- `GITHUB_ACTIONS=true`;
- repository = `Luminous-Dynamics/mycelix-health`;
- event = reviewed `workflow_dispatch` v1 admission path;
- `GITHUB_WORKFLOW_SHA == HARNESS_SHA`;
- `GITHUB_WORKFLOW_REF` names the exact expected workflow path;
- harness checkout `HEAD == HARNESS_SHA`;
- product checkout `HEAD == SUBJECT_SHA`;
- exact recorded action pins actually occur in the exact harness workflow bytes.

A different workflow cannot invoke the receipt machinery and silently inherit reviewed-workflow authority merely by supplying equivalent-looking arguments.

## Evidence-directory separation

Mutable evidence is written under `$RUNNER_TEMP`, outside the product and harness checkouts.

Both final evidence programs reject evidence paths inside either source tree.

The reserved `_ci` namespace is checked absent before secondary checkout. Therefore a candidate cannot pre-populate the harness/parent checkout namespace and cannot make mutable evidence files part of the product tree being evaluated.

This creates three distinct filesystem roles:

```text
GITHUB_WORKSPACE/
    product candidate
    _ci/harness
    _ci/mycelix-parent

RUNNER_TEMP/
    diagnostics and evidence only
```

## Cross-object evidence binding

The final CI receipt does not accept a collection of unrelated hashes.

It requires:

```text
source_verification.profile_sha256
    == SHA256(product source profile)

source_verification.materializer_sha256
    == SHA256(exact harness materializer)

source_verification.verifier_sha256
    == SHA256(exact harness source verifier)

source_verification.mode
    == requested CI evidence mode
```

The receipt also binds:

- exact workflow bytes;
- exact receipt builder bytes;
- exact final receipt verifier bytes;
- exact action pins;
- product/harness commit and tree identities;
- `Cargo.lock`, `flake.lock`, `rust-toolchain.toml`;
- Cargo/flake metadata;
- Nix CI dev-shell derivation;
- GitHub run/attempt/runner fingerprints.

## Independent final verification

The canonical final CI receipt is not the terminal authority object.

After the builder emits it, `scripts/verify-ci-evidence-receipt.py` independently reconstructs the joined evidence graph without importing the builder.

It rechecks:

- receipt canonicalization and sidecar;
- product commit/tree and tracked cleanliness;
- harness commit/tree and full cleanliness;
- executing workflow SHA/ref/path/bytes;
- harness tool bytes and action pins;
- source-verification canonicalization, sidecar, mode, authority and exact cross-bindings;
- lock/flake/toolchain and metadata hashes;
- Nix derivation;
- current Actions run/attempt/runner identity.

Only then may it emit one of:

```text
VerifiedReproducibleRepositoryCIEvidence
VerifiedCompatibilityOnlyObservation
```

The second can never be upgraded into the first merely because every compatibility gate passed.

## Evidence chain

```text
ProductSubject
      |
      +-- profile / lock / flake / toolchain
      |
      v
ExactHarness
      |
      +-- workflow
      +-- materializer
      +-- source verifier
      +-- receipt builder
      +-- receipt verifier
      |
      v
SourceMaterializationReceipt
      |
      v
IndependentSourceVerification
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

## PASS creation boundary

The staged workflows initialize only non-authoritative diagnostic state:

```text
authority=ExecutionDiagnosticsOnly
```

or:

```text
authority=CompatibilityDiagnosticsOnly
```

The canonical PASS receipt is created only after all mandatory build/test/postflight gates succeed. The verified authority object is created only after the independent final verifier succeeds.

Artifact upload may use `if: always()` to retain diagnostics after failure, but:

```text
artifact retained
!= PASS receipt exists
!= verified PASS exists
```

## Pinned vs compatibility authority

Pinned receipt:

```text
ReproducibleRepositoryCIEvidence
```

Pinned independent verification:

```text
VerifiedReproducibleRepositoryCIEvidence
```

Moving-parent receipt:

```text
CompatibilityOnly
```

Moving-parent independent verification:

```text
VerifiedCompatibilityOnlyObservation
```

A green compatibility result cannot satisfy a pinned qualification prerequisite.

## Harness evolution

Changing the harness creates a new execution lineage.

A later harness may improve provenance checks, test coverage, evidence serialization, runner controls, or verification logic. Those improvements are not retroactive.

```text
PASS(subject A, harness H1)
!= PASS(subject A, harness H2)
```

unless a higher-level policy explicitly admits both lineages for the same claim.

## Runner boundary

The harness records hosted-runner fingerprints, but the runner remains an execution substrate rather than product/scientific authority.

This contract does not claim bit-for-bit hermeticity across arbitrary kernels, CPUs, or GitHub-hosted images.

A future self-hosted or alternate-runner adapter requires its own execution-identity/trust profile and must not silently reuse hosted-runner authority.

## Relationship to clinical assurance

This infrastructure is not part of the currently frozen runtime-trust #139/#140 subject unless deliberately integrated into a replacement subject.

P0 #141 still blocks that lineage on a reviewed committed `Cargo.lock` and actual exact-head execution.

The subject/harness theorem improves future evidence construction; it cannot turn a currently queued workflow into a PASS.

## Non-claims

Subject/harness separation and verified CI evidence do not establish:

- scientific validity of a model or decision rule;
- patient/population calibration;
- OOD detector scientific adequacy;
- clinical effectiveness or safety;
- clinician/patient presentation authority;
- diagnosis, prescription, dispensing, administration, or treatment authority;
- HIPAA, GDPR, POPIA, medical-device, or other legal/regulatory compliance;
- regulatory clearance or approval.

They establish narrower software-evidence properties: exact subject identity, exact independently reviewed harness identity, explicit dependency/source/environment bindings, and independently verified CI-result provenance.
