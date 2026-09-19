# ClinicalFact Snapshot v1 Qualification Checklist

This checklist qualifies only the canonical snapshot-identity mechanism. It does not qualify source truth, clinical utility, or authority.

## Build / static gates

- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy --workspace -- -D warnings`
- [ ] `cargo build --workspace`
- [ ] host-target workspace tests pass
- [ ] crate has no unsafe code
- [ ] no serde/JSON dependency is introduced into the canonical snapshot encoder

## Semantic precondition gates

- [ ] valid machine-actionable `ClinicalFact` obtains bytes + digest
- [ ] narrative-only fact is rejected
- [ ] non-UCUM quantitative fact is rejected
- [ ] non-finite numerical fact is rejected
- [ ] invalid/missing subject is rejected
- [ ] incomplete provenance is rejected
- [ ] invalid uncertainty is rejected

## Determinism / substitution gates

- [ ] identical typed fact -> identical canonical bytes
- [ ] identical typed fact -> identical digest
- [ ] subject substitution changes digest
- [ ] concept/coding substitution changes digest
- [ ] clinical value substitution changes digest
- [ ] effective-time substitution changes digest
- [ ] provenance/source-resource substitution changes digest
- [ ] transformation-input substitution changes digest
- [ ] uncertainty substitution changes digest
- [ ] stored-digest verification rejects a substituted fact

## Framing gates

- [ ] snapshot version is explicitly framed
- [ ] snapshot domain tag is explicitly framed
- [ ] integer encoding is big-endian
- [ ] `f64` encoding uses exact IEEE-754 bits
- [ ] option presence is explicit
- [ ] vector lengths are explicit
- [ ] string/byte lengths are explicit
- [ ] enum discriminants are fixed/documented
- [ ] field/vector order behavior is documented
- [ ] length overflow fails closed

## Cross-domain safety

- [ ] digest is represented by dedicated `ClinicalFactSnapshotDigestV1`
- [ ] no API treats arbitrary `[u8; 32]` as a verified fact snapshot
- [ ] no Symthaea evidence digest is converted into a fact snapshot digest by type-casting/copying bytes
- [ ] future evidence crosswalk must verify the actual `ClinicalFact` before binding it

## Exact-head evidence

Record before promotion:

- exact Git commit/head;
- workflow run/job IDs;
- toolchain version;
- lockfile/environment identity;
- test counts/results;
- any ignored/skipped tests;
- known limitations.

## Explicit non-claims

A PASS establishes deterministic exact snapshot identity for validated `ClinicalFact` values only. It does **not** establish fact truth, source trust, recency, causal validity, model validity, clinical effectiveness, regulatory clearance, or authority to present/act clinically.
