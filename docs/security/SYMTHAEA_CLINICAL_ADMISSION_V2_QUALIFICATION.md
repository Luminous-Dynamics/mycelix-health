# Symthaea Clinical Admission v2 Qualification Checklist

## Scope

Qualifies deployment admission preflight behavior only.

## Static contract

- [ ] independent v2 wire verifier is used before policy admission;
- [ ] policy version, wire version and envelope version are pinned;
- [ ] engine/model/schema/typed lineage identities are exact;
- [ ] runtime/configuration/operation are exact;
- [ ] claim kinds and evidence stages are explicit allowlists;
- [ ] intended use is exact;
- [ ] clinical decision support requires subject namespace;
- [ ] clinical decision support requires subject-binding evidence namespace;
- [ ] clinical decision support requires at least one exact evidence namespace;
- [ ] required evidence namespaces are a subset of allowed namespaces;
- [ ] every observed claim namespace must be allowed;
- [ ] every required claim namespace must be observed;
- [ ] age/future-skew bounds are fail closed;
- [ ] calibrated probability can require exact typed calibration identity agreement;
- [ ] critical missing evidence blocks clinical-decision-support preflight;
- [ ] producer OutOfDistribution blocks clinical-decision-support preflight;
- [ ] producer InDistribution does not create independent OOD trust;
- [ ] qualification ceiling has no SupervisedClinical variant;
- [ ] policy digest is serializer-independent and binds every policy field;
- [ ] admission result is non-serializable and cannot mint presentation authority.

## Focused commands

```bash
cargo fmt --check -- crates/symthaea-clinical-admission-v2
cargo check -p mycelix-symthaea-clinical-admission-v2 --all-targets
cargo clippy -p mycelix-symthaea-clinical-admission-v2 --all-targets -- -D warnings
cargo test -p mycelix-symthaea-clinical-admission-v2
```

## Required adversarial tests

- [ ] exact frozen v2 vector + exact policy admits as preflight;
- [ ] model-policy substitution fails;
- [ ] unapproved claim-evidence namespace fails;
- [ ] required but absent evidence namespace fails;
- [ ] stale inference fails;
- [ ] subject-binding namespace substitution fails;
- [ ] clinical policy cannot omit required evidence namespace;
- [ ] policy digest changes when evidence namespace contract changes;
- [ ] runtime/configuration/operation substitutions fail;
- [ ] future-dated inference beyond allowed skew fails;
- [ ] explicit producer OOD fails;
- [ ] critical missing evidence fails.

## Downstream prerequisites before presentation

Admission v2 is not sufficient for model-backed clinical presentation.

Required independent proof lines remain:

- exact ClinicalFact snapshot binding;
- strict typed ClinicalEvidenceCapsule v2 representation;
- independent distribution assessment;
- trusted evaluator admission;
- DNA-rooted evaluator authorization;
- positive short-lived currentness lease;
- practitioner authority;
- final composed presentation gate.

## Evidence policy

Do not mark this tranche qualified merely because the branch exists or a PR is mergeable. Record only exact-head gates that actually execute successfully.
