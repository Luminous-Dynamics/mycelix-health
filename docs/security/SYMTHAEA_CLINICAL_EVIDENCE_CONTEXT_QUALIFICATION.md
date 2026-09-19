# Symthaea Clinical Evidence Context v1 Qualification Checklist

## Scope

Qualifies proof composition between exact Symthaea v2 admission and exact Mycelix ClinicalFact binding.

## Static contract

- [ ] composition requires `AdmittedSymthaeaInferenceV2`;
- [ ] composition requires `VerifiedSymthaeaClinicalFactBindingSetV1`;
- [ ] exact wire digests must match;
- [ ] exact subject IDs must match;
- [ ] admitted Mycelix ClinicalFact evidence and verified fact bindings must have exact coverage;
- [ ] artifact IDs are rechecked;
- [ ] evidence roles are rechecked;
- [ ] ClinicalFact snapshot digests are rechecked;
- [ ] duplicate verified facts are rejected;
- [ ] output is non-serializable;
- [ ] output retains admission-policy and fact-binding-policy digests;
- [ ] context digest binds predecessor proof identities + subject + typed facts;
- [ ] no clinical promotion or presentation API exists.

## Focused commands

```bash
cargo fmt --check -- crates/symthaea-clinical-evidence-context
cargo check -p mycelix-symthaea-clinical-evidence-context --all-targets
cargo clippy -p mycelix-symthaea-clinical-evidence-context --all-targets -- -D warnings
cargo test -p mycelix-symthaea-clinical-evidence-context
```

## Required adversarial tests

- [ ] exact admission + exact fact bindings compose;
- [ ] different wire digest fails;
- [ ] subject mismatch fails;
- [ ] incomplete fact-binding coverage fails;
- [ ] role substitution fails;
- [ ] snapshot-digest substitution fails;
- [ ] artifact-ID substitution fails;
- [ ] context digest changes when predecessor policy identity changes.

## Downstream containment

Even after qualification, the context is not a ClinicalEvidenceCapsule and not presentation authority.

A future capsule projection must separately bind:

- subject representation;
- assertion kind;
- statement;
- intended use;
- Mycelix qualification level;
- typed facts;
- admission ceiling.

Final clinical presentation still requires independent OOD trust/currentness and practitioner authority.
