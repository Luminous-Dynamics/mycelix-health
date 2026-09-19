# Symthaea Clinical Wire v2 Independent Verification Checklist

## Build gates

- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy --workspace -- -D warnings`
- [ ] `cargo build --workspace`
- [ ] host-target workspace tests pass
- [ ] exact head/toolchain/lockfile recorded

## Independence gates

- [ ] verifier crate has no dependency on Symthaea
- [ ] verifier crate has no dependency on Mycelix v1 Symthaea verifier
- [ ] local v2 contract types are independently defined
- [ ] parser logic is local to Mycelix
- [ ] digest framing is locally reproduced from published contract

## Shared vector gates

- [ ] Mycelix fixture blob SHA is `d8f359ef04be4d4f12ecdf49427781c5af190701`
- [ ] Symthaea fixture blob SHA is the same
- [ ] fixture decodes to exactly 1260 bytes
- [ ] independent parser recovers expected schema/claim/subject/evidence/execution fields
- [ ] typed calibration identity preserved
- [ ] typed OOD detector identity preserved
- [ ] published wire digest is nonzero and deterministic

## Adversarial parser gates

- [ ] wrong magic rejected
- [ ] unsupported wire version rejected
- [ ] unsupported envelope version rejected
- [ ] invalid enum tag rejected
- [ ] invalid option tag rejected
- [ ] truncated bytes rejected
- [ ] trailing bytes rejected
- [ ] invalid UTF-8 rejected
- [ ] complete input >4 MiB rejected
- [ ] excessive string/vector lengths rejected
- [ ] zero digest rejected
- [ ] zero execution nonce rejected

## Semantic mirror gates

- [ ] clinical decision support requires subject
- [ ] evidence identity version exact
- [ ] namespace/artifact ID validation matches published contract
- [ ] execution evidence unique
- [ ] claim evidence exactly execution-bound
- [ ] subject-binding evidence exactly execution-bound
- [ ] duplicate `(namespace, artifact_id)` claim evidence rejected
- [ ] calibrated inference/model evidence identity must agree exactly
- [ ] known distribution state requires detector evidence

## Domain-separation gates

- [ ] verified Symthaea wire digest is a dedicated type
- [ ] no API converts it into a Mycelix clinical-fact snapshot digest
- [ ] no API converts raw v2 evidence digest to `ClinicalFactSnapshotDigestV1` without local fact verification
- [ ] no wire success path mints clinical presentation authority

## Next required gate

- [ ] deployment-scoped v2 admission policy pins accepted engine/model/lineages/runtime/config/use/namespaces
- [ ] explicit Mycelix `ClinicalFact` crosswalk verifies exact fact snapshot for `mycelix/clinical-fact-snapshot/v1`
- [ ] independent distribution trust/currentness chain remains required
- [ ] practitioner/human authority remains required

## Non-claims

PASS establishes independent software compatibility with the published Symthaea binary v2 wire contract only. It does not establish evidence truth, scientific validity, model effectiveness, clinical utility, detector validity, regulatory clearance, presentation authority, or treatment authority.
