# FHIR Source Revision Contract v1

This is a dependency-free, offline reference classifier for the source-revision work tracked by Health issue #302. It is not a production revision store and does not modify the FHIR bridge.

## Paradigm

Separate three things that are often collapsed:

1. **Logical resource identity:** tenant + authenticated source principal + FHIR resource type + logical resource ID.
2. **Source revision identity:** the logical identity plus opaque FHIR version ID, canonicalization profile and payload digest.
3. **Current projection:** a derived view that advances only when a source-specific ordering contract justifies promotion.

An imported resource is not a mutable row whose contents are silently replaced. It is an immutable source assertion. The current projection is a separate decision.

## Reference dispositions

- First revision: can establish the initial projection.
- Exact replay: same logical identity, version ID, digest and canonicalization profile; idempotent, no mutation.
- Version equivocation: same version ID but a different payload digest; quarantine.
- Canonicalization-profile conflict: same version ID/digest bytes but a different canonicalization profile; quarantine until re-canonicalized under a comparable profile.
- Newer by source sequence: promote only when the incoming sequence is strictly greater under the same documented ordering profile.
- Older by source sequence: preserve as history, never roll the projection backwards.
- Source-sequence collision: distinct versions claim the same sequence under one profile; quarantine.
- Order unknown: preserve as immutable history but do not promote the current projection.

FHIR meta.versionId is treated as opaque. The classifier never lexically or numerically sorts it. Source lastUpdated and local receive time are stored separately and never used to infer source revision order.

## Run

    bash ./scripts/run-fhir-source-revision-benchmark.sh

Equivalent commands:

    cargo fmt --manifest-path prototypes/fhir-source-revision-bench/Cargo.toml -- --check
    HOST="$(rustc -vV | sed -n 's/^host: //p')"
    cargo test --manifest-path prototypes/fhir-source-revision-bench/Cargo.toml --target "$HOST" --locked

The nested crate has no external dependencies and does not join the production Cargo workspace.

## Trust and implementation boundaries

- The source principal must come from a verified gateway/connection mapping under #289. This crate only requires a non-empty value; it does not authenticate it.
- The digest must be computed from canonical payload bytes using the named profile. This crate accepts a fixed 32-byte digest and does not implement hashing or canonicalization.
- The source ordering profile must be declared and validated by an authenticated source adapter. A local timestamp or opaque version ID is not an ordering contract.
- The classifier does not perform clinical reconciliation, issuer verification, consent checks, Holochain writes or clinician-presentation promotion.
- Corrections and entered-in-error states remain explicit source assertions. They do not automatically become clinical truth.

## Frozen v1 coverage

The test suite covers first revision, exact replay, newer and late-arriving revisions, same-version/different-digest equivocation, canonicalization profile mismatch, missing version IDs, opaque version IDs, timestamp non-ordering, mismatched ordering profiles, source-sequence collision, tenant/source isolation and invalid identity.

## Next production integration

1. Authenticate source principals and bind them to a tenant/connection (#289).
2. Introduce a versioned immutable revision record and stable digest profile.
3. Keep logical resource anchors as indexes to revision history, not as mutable deduplication flags.
4. Separate append-only revision acceptance from current-projection promotion.
5. Add conductor restart/replay tests and exact old-revision retrieval checks.
6. Integrate correction semantics with the existing laboratory evidence and CARE-001 policies (#143/#158).
