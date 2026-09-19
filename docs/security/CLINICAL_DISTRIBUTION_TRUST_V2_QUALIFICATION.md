# Clinical Distribution Trust v2 Qualification Checklist

## Scope

Qualifies only the currentness-bound evaluator trust receipt.

It does not qualify conductor provenance, detector science, model effectiveness, patient applicability, clinical decision thresholds, practitioner authority, or presentation authority.

## Required proof inputs

- [ ] validated structural distribution assessment;
- [ ] exact structural distribution policy;
- [ ] exact evaluator trust policy;
- [ ] v1 verified evaluator admission;
- [ ] exact currentness policy;
- [ ] live `VerifiedDistributionEvaluatorCurrentnessV1`.

## v1 trust preservation

- [ ] v2 calls the existing v1 trust evaluator rather than reimplementing it;
- [ ] evaluator admission remains required at assessment time;
- [ ] evaluator admission remains required at trust-evaluation time;
- [ ] structural-policy identity remains exact;
- [ ] detector identity remains exact;
- [ ] trust-policy identity remains exact;
- [ ] external admission-evidence identity remains exact.

## Currentness binding

- [ ] currentness policy digest equals the supplied currentness policy;
- [ ] currentness proof is active at trust-evaluation time;
- [ ] currentness detector equals trust-policy detector;
- [ ] currentness structural-policy digest equals v1 receipt policy digest;
- [ ] currentness trust-policy digest equals v1 receipt trust-policy digest;
- [ ] currentness admission-evidence digest equals v1 receipt admission-evidence digest;
- [ ] expired currentness rejects;
- [ ] substituted currentness policy rejects;
- [ ] detector substitution rejects;
- [ ] structural-policy substitution rejects;
- [ ] trust-policy substitution rejects;
- [ ] admission-evidence substitution rejects.

## Receipt contract

- [ ] `DistributionTrustReceiptV2` is non-serializable;
- [ ] receipt retains exact v1 trust-receipt digest;
- [ ] receipt retains exact currentness-policy digest;
- [ ] receipt retains exact currentness digest;
- [ ] receipt retains currentness expiry;
- [ ] receipt digest is domain-separated and binary framed;
- [ ] changing currentness identity changes receipt identity;
- [ ] changing v1 receipt identity changes receipt identity.

## Focused Rust gates

```bash
cargo fmt --check -- crates/clinical-distribution-trust-v2
cargo check -p mycelix-clinical-distribution-trust-v2 --all-targets
cargo clippy -p mycelix-clinical-distribution-trust-v2 --all-targets -- -D warnings
cargo test -p mycelix-clinical-distribution-trust-v2
```

## Workspace gate

- [ ] `crates/clinical-distribution-trust-v2` is a workspace member;
- [ ] all direct dependencies are explicit;
- [ ] unit tests exercise the actual currentness composition path rather than constructing the opaque currentness proof directly.

## Required integration/adversarial evidence

Before any model-backed presentation path consumes this receipt:

- [ ] exact conductor/cell/DNA provenance for currentness inputs is qualified;
- [ ] exact runtime admission/lease source entry definitions are qualified;
- [ ] currentness cannot be fabricated through a modified coordinator path;
- [ ] currentness expiry is rechecked at the eventual presentation decision;
- [ ] admission-only v1 receipt is rejected by the future model-backed presentation API;
- [ ] v2 receipt from another detector/policy/admission lineage is rejected;
- [ ] stale v2 receipt cannot be replayed after currentness expiry.

## Promotion rule

Do **not** add a model-backed `ClinicalPresentationPermit` in this tranche.

A later promotion tranche must separately prove that the clinical assertion representation, model semantics, calibrated uncertainty, missing evidence, decision rule, distribution status, v2 trust receipt, practitioner authority and intended-use policy all refer to the same exact subject/model/execution lineage.

## Evidence policy

Branch existence, PR mergeability, static review, or queued CI are not qualification. Only exact-head gates that actually execute successfully count as software evidence.