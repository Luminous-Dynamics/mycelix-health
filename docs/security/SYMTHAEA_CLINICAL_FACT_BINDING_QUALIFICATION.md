# Symthaea ClinicalFact Binding v1 Qualification Checklist

## Build gates

- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy --workspace -- -D warnings`
- [ ] `cargo build --workspace`
- [ ] host-target tests pass
- [ ] exact head/toolchain/lockfile recorded

## Upstream proof gates

- [ ] Symthaea v2 bytes pass independent Mycelix binary-wire verification
- [ ] exact Symthaea wire digest retained
- [ ] only evidence namespace `mycelix/clinical-fact-snapshot/v1` is treated as this binding type
- [ ] no raw v1 Symthaea digest is accepted as a fact snapshot

## Fact binding gates

- [ ] at least one Mycelix ClinicalFact evidence item required
- [ ] evidence artifact ID equals supplied `ClinicalFact.fact_id`
- [ ] supplied fact passes `validate_machine_actionable`
- [ ] local `ClinicalFactSnapshotDigestV1` computed from actual fact
- [ ] local snapshot digest equals typed Symthaea evidence digest exactly
- [ ] evidence role preserved
- [ ] every referenced fact supplied exactly once
- [ ] duplicate fact ID input rejected
- [ ] missing referenced fact rejected
- [ ] unreferenced extra fact rejected

## Subject binding gates

- [ ] envelope subject required for fact binding
- [ ] external subject namespace must match explicit policy
- [ ] policy explicitly maps external namespace to Mycelix resource type
- [ ] Mycelix fact resource type matches policy
- [ ] Mycelix fact subject ID equals Symthaea inference subject ID
- [ ] wrong-patient substitution rejected

## Snapshot substitution gates

- [ ] effective time substitution rejected by digest mismatch
- [ ] value/unit substitution rejected by digest mismatch
- [ ] provenance substitution rejected by digest mismatch
- [ ] transformation-lineage substitution rejected by digest mismatch
- [ ] uncertainty substitution rejected by digest mismatch
- [ ] subject substitution rejected before or at snapshot verification

## Domain / authority gates

- [ ] output retains dedicated `ClinicalFactSnapshotDigestV1`
- [ ] output retains exact Symthaea wire digest
- [ ] output retains binding-policy digest
- [ ] no direct cast/copy into generic `ContentDigest`
- [ ] no API mints `ClinicalPresentationPermit`
- [ ] no API creates diagnostic/treatment/prescribing authority

## Non-claims

PASS establishes exact cross-system binding from a typed Symthaea evidence identity to an actual locally validated Mycelix ClinicalFact snapshot. It does not establish source truth, recency, causal relevance, model validity, detector validity, clinical utility, regulatory clearance, practitioner authority, presentation authority, or treatment authority.
