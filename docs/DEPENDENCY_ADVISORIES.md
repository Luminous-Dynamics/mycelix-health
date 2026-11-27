# Dependency Advisory Status

**Reviewed:** 2026-11-27  
**Scope:** active Rust workspace graph in Mycelix Health; not archived crates.

This file records the distinction between defects fixed in this repository and advisories that remain in upstream transitive dependencies. A failing `cargo deny` result is not reclassified as a pass by this document.

## Repository-owned remediation

| Finding | Repository action | Expected state |
|---|---|---|
| `RUSTSEC-2026-0204` — `crossbeam-epoch` pointer validity | Lockfile updated from 0.9.18 to 0.9.20, the version that contains the upstream fix | Cleared when exact-head `cargo deny` confirms no vulnerable graph entry |
| Four first-party zomes missing license metadata | Added `license.workspace = true` to the affected manifests | Cleared in the reviewed workspace build |
| Holochain SDK Node ESM runtime failure | SDK locks `libsodium-wrappers` at 0.7.13 and verifies the installed version before tests | Prevents the observed 0.7.16 missing `libsodium.mjs` resolution path |
| Unused direct workspace `chrono` declaration | Removed after confirming no active workspace member inherits it | Direct dependency removed; transitive occurrence remains |

## Upstream transitive advisories still open

### RUSTSEC-2026-0543 — `chrono` is unmaintained

The active Holochain 0.6 dependency graph pulls `chrono` through `holochain_timestamp`. The advisory states that all chrono versions are affected and does not provide a safe upgrade version. Removing the unused direct workspace declaration does not remove the transitive dependency.

### RUSTSEC-2026-0441 — `pin-utils` is unmaintained

The active Holochain graph pulls `pin-utils` through `must_future`. The advisory's suggested code-level migration requires the upstream consumers to replace the deprecated macros (for example with `core::pin::pin!` or `pin-project-lite`). No lockfile-only version bump can safely remove it.

## Policy

- Do not add broad advisory ignores to make `cargo deny` green.
- Do not move a production health workspace to an upstream prerelease solely to silence an advisory.
- Track upstream fixes and re-run `cargo deny check` on the exact resolved graph.
- If a temporary exception becomes unavoidable, require maintainer approval, an impact analysis, a review date, and a narrow advisory-specific entry.

## Upstream references

- [RUSTSEC-2026-0204](https://rustsec.org/advisories/RUSTSEC-2026-0204)
- [RUSTSEC-2026-0543](https://rustsec.org/advisories/RUSTSEC-2026-0543)
- [RUSTSEC-2026-0441](https://rustsec.org/advisories/RUSTSEC-2026-0441)
- [Holochain 0.6 to 0.7 upgrade guidance](https://developer.holochain.org/resources/upgrade-0.6-to-0.7/)
- [HDK 0.7.1 dependency metadata](https://docs.rs/crate/hdk/0.7.1)
- [Holochain 0.7.1 dependency lock](https://docs.rs/crate/holochain/0.7.1/source/Cargo.lock)
