# CI Evidence Stack V1 — Final Staged Review Checklist

This file records the final static review target immediately before the off-PR branch is squashed into one commit over `main`.

Required properties:

- product subject and qualification harness are separate exact Git identities;
- product tracked/index state must be clean;
- harness state must be fully clean, including untracked files;
- product and harness Git tree SHAs are bound into final evidence;
- candidate cannot pre-own `_ci` before secondary checkout;
- mutable evidence lives under `$RUNNER_TEMP`, outside product/harness trees;
- parent source materialization is independently verified;
- symlink escapes and destination aliasing fail closed;
- pinned and compatibility authority classes are distinct;
- committed `Cargo.lock` is required with `--locked`;
- Nix execution uses committed `flake.lock` / `rust-toolchain.toml`;
- workflow/action identities are exact and recorded;
- final CI receipt is canonical and create-only;
- final CI receipt is independently verified by a different program;
- a successful moving-parent run terminates at `VerifiedCompatibilityOnlyObservation`;
- no repository-CI evidence grants scientific or clinical authority;
- workflows remain manual-only while runner capacity is infrastructure-indeterminate;
- this branch remains off-PR and does not alter the frozen #139/#140 clinical subject.

Current staged evidence class remains:

`SOURCE_STAGED / STATICALLY_REVIEWED / UNEXECUTED / NOT_QUALIFIED`.
