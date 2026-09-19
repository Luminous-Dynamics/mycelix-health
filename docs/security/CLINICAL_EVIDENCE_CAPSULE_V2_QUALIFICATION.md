# Clinical Evidence Capsule v2 Qualification Checklist

## Scope

This checklist qualifies only the typed evidence representation layer in `mycelix-clinical-evidence-v2`.

It does not qualify scientific validity, clinical effectiveness, evaluator trust, OOD currentness, practitioner authority, presentation authority, diagnosis or treatment.

## Static contract

- [ ] crate is a workspace member;
- [ ] v1 `mycelix-clinical-evidence` remains unchanged;
- [ ] v2 has an explicit schema version;
- [ ] typed evidence identity has an explicit identity version;
- [ ] ClinicalFact namespace is exactly `mycelix/clinical-fact-snapshot/v1`;
- [ ] typed digest is fixed width and rejects all-zero values;
- [ ] fact artifact ID must equal `fact_id`;
- [ ] typed fact evidence role must equal the wrapped v1 fact role;
- [ ] duplicate typed fact IDs are rejected;
- [ ] duplicate typed identities are rejected;
- [ ] typed evidence must cover core fact evidence exactly;
- [ ] legacy generic `FactEvidence.fact_digest` is rejected in v2;
- [ ] no `promotion_state()` or clinical action API is introduced.

## Focused commands

Run on the exact proposed head:

```bash
cargo fmt --check -- crates/clinical-evidence-v2
cargo check -p mycelix-clinical-evidence-v2 --all-targets
cargo clippy -p mycelix-clinical-evidence-v2 --all-targets -- -D warnings
cargo test -p mycelix-clinical-evidence-v2
```

## Required adversarial tests

- [ ] exact typed capsule validates;
- [ ] missing typed fact evidence fails closed;
- [ ] legacy generic fact digest fails closed;
- [ ] fact-role mismatch fails closed;
- [ ] wrong evidence namespace fails closed;
- [ ] artifact-ID substitution fails closed;
- [ ] zero typed digest fails closed;
- [ ] duplicate fact evidence fails closed;
- [ ] extra typed fact evidence fails closed.

## Integration prerequisites

Before a Symthaea-produced typed fact identity is admitted into a v2 capsule as verified evidence:

- [ ] independent Symthaea v2 wire verification is qualified;
- [ ] ClinicalFact snapshot identity is qualified;
- [ ] Symthaea ClinicalFact binding is qualified;
- [ ] the adapter consumes the non-serializable verified binding result rather than raw producer identity fields.

## Promotion boundary

Even after every item above passes, the result establishes typed evidence representation only.

Model-backed clinical presentation still requires independent:

- deployment admission policy;
- distribution/OOD assessment and trust;
- DNA-rooted evaluator trust/currentness;
- practitioner authority;
- final composed promotion gate.
