# CI Parent Source Identity V1

## Scope

This contract separates **dependency-source identity** from Cargo resolution, build/test outcome, compatibility, and clinical authority.

`mycelix-health` has several path dependencies that normally live in the parent `Luminous-Dynamics/mycelix` repository. A standalone Health checkout therefore cannot be reproduced from its own Git commit alone unless the parent source bytes are also bound.

CI-SOURCE-001A introduces:

- `release/ci-parent-source-profile-v1.json` — the declared parent source identity and export map;
- `scripts/materialize-parent-mycelix.py` — a fail-closed materializer;
- `scripts/check-parent-source-materializer.py` — adversarial source-identity tests.

This tranche does not modify the current frozen clinical runtime-trust subject and does not request GitHub Actions execution by itself.

## Evidence classes

### Pinned source mode

Pinned mode requires all of the following before any export is accepted:

1. the checkout origin resolves to `Luminous-Dynamics/mycelix`;
2. the checkout is clean, including untracked-file state;
3. exact commit SHA equals the profile commit;
4. exact Git tree SHA equals the profile tree;
5. every declared source path exists;
6. every destination remains a sibling below the Health workspace parent;
7. each copied export has the same deterministic content digest before and after materialization;
8. every required manifest exists after materialization.

A successful receipt has:

`authority = PinnedDependencySourceMaterialized`

That authority means only that the declared dependency source was materialized with the recorded identities.

It is **not** Cargo/build/qualification authority.

### Compatibility mode

Compatibility mode still requires:

- the expected GitHub repository origin;
- a clean observed checkout;
- the same bounded export map;
- byte-preserving materialization;
- required manifests.

It does **not** require the observed commit/tree to equal the pinned profile.

Its receipt is always:

`authority = CompatibilityOnly`

Therefore:

`latest parent sampled successfully != exact-subject qualification`

and:

`latest parent sampled unsuccessfully != failure of the pinned subject`

The observation belongs to a separate compatibility lineage.

## Materialized-byte identity

Git commit/tree identity is necessary but the standalone Health build consumes copied sibling directories, not Git object identifiers directly.

For every export, the materializer computes a deterministic digest over:

- relative file paths;
- executable-bit semantics;
- regular-file bytes;
- symlink paths and targets.

The digest is computed before and after copy and must match.

This creates the chain:

`repository + commit + tree -> export map -> copied byte digest`

A future build receipt should bind these export digests in addition to the parent Git identity.

## Current pinned parent profile

The v1 profile declares:

- repository: `Luminous-Dynamics/mycelix`;
- commit: `a85369699099d4c7524e502e531735eed4ab36f4`;
- tree: `6f7ff861547fdff31dba9a284f845e7a2a736d7a`.

Exports:

1. `crates` -> `../crates`;
2. `mycelix-core` -> `../mycelix-core`;
3. `mycelix-core` -> `../mycelix-workspace/mycelix-core`.

The required manifests include the path dependencies currently used by standalone Health qualification:

- `mycelix-bridge-common`;
- `mycelix-leptos-core`;
- `mycelix-zkp-core`;
- `mycelix-fl`.

Updating the pinned parent commit/tree is a reviewed dependency-source change. It must not happen implicitly because parent `main` moved.

## Intended CI split

This contract is the source-identity primitive for #142.

### Lane A — reproducible repository CI

Future pinned CI should:

1. check out the exact Health subject;
2. check out the exact parent commit from the profile;
3. run this materializer in `pinned` mode;
4. retain the source-materialization receipt and digest;
5. require committed `Cargo.lock` consistency with `cargo metadata --locked`;
6. run build/test/lint gates under the declared toolchain/environment;
7. prove `Cargo.lock` and tracked source immutability postflight;
8. bind the materialization receipt into the CI evidence output.

### Lane B — moving-parent compatibility

Future compatibility CI should:

1. sample current parent `main` deliberately;
2. run this materializer in `compatibility` mode;
3. record the observed parent commit/tree and export digests;
4. run the desired compatibility checks;
5. report the result as compatibility with that sampled parent only.

The compatibility lane must never satisfy an exact-head qualification prerequisite.

## Adversarial requirements

The self-test requires fail-closed behavior for:

- parent commit substitution;
- parent tree substitution;
- dirty parent checkout;
- repository-origin substitution;
- source path traversal;
- missing required manifest;
- implicit overwrite of existing sibling source trees.

It also requires that a clean newer parent can be observed in compatibility mode only with `CompatibilityOnly` authority.

## Relationship to #141

This contract does not repair the stale Health `Cargo.lock` tracked by #141.

The correct dependency theorem remains:

`pinned source identity + reviewed Cargo.lock + locked execution`

not merely:

`pinned source identity`.

A lock reconciliation candidate must still be generated, audited, committed as a new exact subject, and requalified.

## Non-claims

A successful source-materialization receipt does not establish:

- Cargo.lock consistency;
- successful compilation, linting, testing, packaging, or Holochain execution;
- compatibility beyond the exact sampled source graph;
- software qualification of any clinical theorem;
- clinical validity, calibration, effectiveness, or safety;
- clinician or patient presentation authority;
- diagnostic, prescribing, dispensing, administration, or treatment authority;
- legal or regulatory compliance, clearance, or approval.
