# Dependency security status — 2026-11-27

This note records the exact findings observed during the standalone Health workspace remediation. It is not a security certification and does not replace upstream advisory review.

## Cleared or locally remediated

| Finding | Change | Evidence boundary |
|---|---|---|
| Four workspace zomes lacked a declared license and failed cargo-deny's license check | Their package manifests inherit `license.workspace = true`; the workspace package license is AGPL-3.0-or-later | The first host-runner cargo-deny output no longer reported those four first-party `unlicensed` packages |
| SDK resolved `libsodium-wrappers 0.7.16`, whose ESM entry imported a missing `libsodium.mjs` companion file in CI | SDK now declares exact `libsodium-wrappers = 0.7.13`; the package-lock resolves the same exact version | The dedicated SDK lock refresh completed successfully on branch commit `8807b22ada72cb3ae67f5fff1b0286b2623bd397`; the normal SDK tests must rerun against the newest PR head |
| `crossbeam-epoch 0.9.18` | PR [#374](https://github.com/Luminous-Dynamics/mycelix-health/pull/374) now carries the reviewed lockfile update to 0.9.20 on top of the SDK implementation branch | Confirm the exact merged tree still resolves 0.9.20 |

## Remaining upstream advisory blockers

The current Holochain dependency graph still reaches:

- `chrono 0.4.45` via `holochain_timestamp`, flagged by [RUSTSEC-2026-0543](https://rustsec.org/advisories/RUSTSEC-2026-0543) as unmaintained.
- `pin-utils 0.1.0` via `must_future` and `holo_hash`, flagged by [RUSTSEC-2026-0441](https://rustsec.org/advisories/RUSTSEC-2026-0441) as unmaintained.

The examined Holochain 0.7.1 dependency metadata still included these paths. Moving a health workspace from the 0.6 SDK family to 0.7 does not, by itself, demonstrate that either advisory is removed. A prerelease/development SDK should not be substituted solely to make cargo-deny quiet.

These findings remain visible in the deny gate on the combined stack. PR #374 does not add broad advisory ignores, lower policy severity, or claim that the transitive dependencies are safe. Its purpose is to clear the separately remediable crossbeam advisory and first-party license metadata findings while retaining the two upstream Holochain blockers. Resolution requires an upstream dependency update, a carefully scoped downstream replacement, or a time-bounded exception with owner, rationale, compensating controls and expiry explicitly reviewed.

## Reproduction

From the repository root:

~~~bash
cargo deny check
~~~

From the SDK directory:

~~~bash
npm ci
npm run typecheck
npm run build
npm test
~~~

Record exact commit, lockfile digest, toolchain and complete output for every rerun. A green Rust build is not a substitute for a green dependency audit or SDK test suite.
