# Clinical Population Release Qualification Checklist

## Source / build

- [ ] Exact branch/head SHA recorded.
- [ ] `cargo fmt -- --check` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `mycelix-clinical-population-release` is a workspace member.

## Static report mode

- [ ] Threshold below the software floor fails.
- [ ] Deployment threshold can be stricter than the software floor.
- [ ] Primary suppression evidence is required.
- [ ] Complementary/secondary suppression evidence is required.
- [ ] Composition/differencing review evidence is required.
- [ ] Suppression evidence artifacts cannot silently collapse to one digest.
- [ ] Report cell counts are bounded and overflow-safe.
- [ ] Wrong source-finding digest domain fails closed.
- [ ] Repeated/ad-hoc query use is not exposed through the static path.
- [ ] Adversarial complementary-cell/differencing fixtures execute successfully.

## Differential-privacy mode

- [ ] No threshold-only interactive mode exists.
- [ ] Privacy unit definition is bound.
- [ ] Adjacency definition is bound.
- [ ] Contribution/clipping bounds are bound.
- [ ] Sensitivity-analysis evidence is bound.
- [ ] Exact mechanism implementation/version is bound.
- [ ] Randomness-source evidence is bound.
- [ ] Epsilon/delta have canonical non-floating representation.
- [ ] Delta outside `[0,1]` fails closed.
- [ ] Disallowed mechanism kind fails closed.
- [ ] Per-release privacy limit is enforced.
- [ ] Cumulative privacy limit is enforced.
- [ ] Query-count limit is enforced.

## Accountant lineage

- [ ] Genesis requires sequence/query count zero and zero cumulative loss.
- [ ] Successor binds exact predecessor receipt digest.
- [ ] Sequence gaps fail closed.
- [ ] Query-count gaps fail closed.
- [ ] Cumulative privacy loss cannot decrease.
- [ ] Observation time cannot move backwards.
- [ ] Successor binds exact query specification.
- [ ] Successor binds exact DP mechanism digest.
- [ ] Successor binds exact public output.
- [ ] Replay/fork fixtures execute successfully.
- [ ] Deployment-selected accountant instance/method is policy-bound.
- [ ] Verifier-owned accountant trust/state-root admission is implemented and qualified.

## Capability / receipt

- [ ] Only validated requests mint `PopulationReleaseCapabilityV1` through the safe API.
- [ ] Capability is non-serializable.
- [ ] Receipt binds exact request/policy/source/output/mode.
- [ ] Release time cannot precede request time.
- [ ] Stored/wire release digests are treated as claims until exact artifact verification.

## Privacy and governance

- [ ] Static floor is documented as a minimum, not universal sufficiency.
- [ ] Jurisdiction/deployment policy may require higher static thresholds.
- [ ] No raw cohort/case-set object is broadly published by this crate.
- [ ] No patient-level data is added to release receipts.
- [ ] Release does not upgrade signal -> association -> causality.
- [ ] Composition across other release systems has an explicit governance/accounting plan.
- [ ] NIST SP 800-226 implementation hazards are reviewed for each qualified DP mechanism.
- [ ] Disclosure review verifies minimum-necessary output for approved purpose.

## Promotion rule

Do not describe a population output as privacy-qualified for broad release until every applicable item above has exact-head executable evidence. A well-formed pure-crate receipt is not sufficient evidence of trusted suppression review or a trusted differential-privacy accountant.
