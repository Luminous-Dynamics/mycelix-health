# Symthaea Clinical Wire Verification V1 — Qualification Checklist

Passing this checklist establishes only cross-repository wire compatibility and local structural verification. It does not establish clinical validity or authority.

## Independent schema gates

- [ ] Mycelix verifier has no Symthaea crate/git dependency;
- [ ] envelope schema version mismatch fails closed;
- [ ] claim-semantics schema version mismatch fails closed;
- [ ] unknown fields fail closed;
- [ ] zero digests fail closed;
- [ ] zero execution nonce fails closed;
- [ ] missing/duplicate execution input digests fail closed;
- [ ] clinical decision-support intended use without subject binding fails closed;
- [ ] empty/duplicate evidence IDs fail closed;
- [ ] invalid uncertainty/calibrated probability fails closed;
- [ ] calibrated status without probability + calibration evidence fails closed;
- [ ] known distribution status without detector evidence fails closed.

## Canonical wire gates

- [ ] Symthaea and Mycelix contain byte-identical v1 conformance vectors;
- [ ] Symthaea parser accepts the vector and reproduces it byte-for-byte;
- [ ] Mycelix independent parser accepts the vector and reproduces it byte-for-byte;
- [ ] leading/trailing whitespace is rejected as non-canonical;
- [ ] key reordering is rejected as non-canonical;
- [ ] ignored/unknown-field injection is rejected;
- [ ] model substitution changes wire identity;
- [ ] evidence substitution changes wire identity;
- [ ] execution-nonce substitution changes wire identity.

## Digest framing gates

- [ ] derive-key context matches the public Symthaea v1 contract exactly;
- [ ] wire-version framing matches exactly;
- [ ] schema-tag framing matches exactly;
- [ ] byte-length framing matches exactly;
- [ ] both repositories produce the same digest for the shared canonical vector;
- [ ] a hard-coded cross-repository expected digest is added once exact-head tooling produces the reference value.

## Admission separation gates

- [ ] successful wire verification creates no `ClinicalPresentationPermit`;
- [ ] wire evidence cannot self-select Mycelix `QualificationLevel`;
- [ ] Symthaea `evidence_stage` cannot automatically promote Mycelix qualification;
- [ ] Symthaea intended-use declaration cannot authorize presentation;
- [ ] accepted engine/model lineages are controlled by a separate Mycelix admission policy;
- [ ] OOD/distribution trust remains governed by the separate distribution assurance/trust/runtime chain.

## Promotion rule

Keep this tranche draft until exact-head CI passes and the shared conformance vector is demonstrated on both repository heads. Even then, treat it as wire compatibility only.
