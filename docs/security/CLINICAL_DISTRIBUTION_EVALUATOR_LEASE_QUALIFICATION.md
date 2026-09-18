# Clinical Distribution Evaluator Positive Lease v1 Qualification Checklist

## Scope

Qualifies only the DNA-rooted positive evaluator lease primitive and its bounded network-backed observation.

It does not qualify detector science, model clinical effectiveness, final runtime-currentness composition, practitioner authority or presentation authority.

## Integrity contract

- [ ] packaged DNA root set remains empty by default;
- [ ] lease is root-authored directly in v1;
- [ ] no delegated lease issuer exists;
- [ ] lease binds exact admission action hash;
- [ ] lease binds exact admission proposal digest;
- [ ] lease binds exact detector identity;
- [ ] lease binds exact distribution-policy digest;
- [ ] lease binds exact evaluator-trust-policy digest;
- [ ] lease duration is hard-capped at <= 300,000,000 microseconds;
- [ ] action timestamp must fall inside lease validity window;
- [ ] lease cannot begin before underlying admission validity;
- [ ] lease cannot outlive underlying admission validity;
- [ ] entries are immutable and non-deletable;
- [ ] admission->lease links are append-only;
- [ ] sequence zero has no predecessor;
- [ ] sequence > 0 names an exact predecessor and increments by exactly one;
- [ ] supersession cannot cross admission/detector/policy lineage;
- [ ] supersession cannot move valid_from backward;
- [ ] referenced admission exact app-entry-definition is verified in conductor/integration qualification.

## Read-side contract

- [ ] network-backed lease-link read A;
- [ ] all observed lease targets are materialized with a hard count bound;
- [ ] each observed lease is checked against exact admission/proposal/detector/policies;
- [ ] network-backed lease-link read B;
- [ ] changed A/B target set fails closed;
- [ ] duplicate highest sequence fails closed;
- [ ] missing supersession ancestor fails closed;
- [ ] non-consecutive sequence fails closed;
- [ ] cycles fail closed;
- [ ] competing observed lease lineages fail closed;
- [ ] unique highest-sequence lease must itself be valid at observation completion;
- [ ] expired highest sequence does not resurrect an older lease;
- [ ] result remains explicitly `NetworkBackedStableDoubleRead` evidence.

## Focused Rust gates

```bash
cargo fmt --check -- \
  zomes/clinical_distribution_lease/integrity \
  zomes/clinical_distribution_lease/coordinator
cargo check -p clinical_distribution_lease_integrity --all-targets
cargo check -p clinical_distribution_lease --all-targets
cargo clippy -p clinical_distribution_lease_integrity --all-targets -- -D warnings
cargo clippy -p clinical_distribution_lease --all-targets -- -D warnings
cargo test -p clinical_distribution_lease_integrity
cargo test -p clinical_distribution_lease
```

## DNA/package gates

- [ ] both new zomes are workspace members;
- [ ] both WASM artifacts build;
- [ ] integrity zome is packaged into `dna/dna.yaml`;
- [ ] coordinator depends on lease integrity and distribution-trust integrity;
- [ ] default DNA with empty roots cannot commit a valid lease;
- [ ] repacked test DNA with one root can issue one bounded lease;
- [ ] non-root author cannot issue lease even through a modified coordinator.

## Conductor/adversarial gates

- [ ] admission A lease replayed against B fails;
- [ ] detector substitution fails;
- [ ] distribution-policy substitution fails;
- [ ] trust-policy substitution fails;
- [ ] lease beyond admission expiry fails;
- [ ] >5-minute lease fails;
- [ ] future-dated lease fails;
- [ ] duplicate sequence/competing lineage fails read-side currentness observation;
- [ ] missing predecessor fails;
- [ ] conductor restart preserves exact lease action identity and expiry;
- [ ] stable-double-read marker survives adapter boundary;
- [ ] exact source-entry-definition proof rejects structurally compatible wrong-entry records.

## Mandatory composition rule

Do not interpret a valid lease observation as final evaluator currentness.

Final runtime currentness additionally requires:

- exact matching admission/revocation snapshot;
- zero **observed** revocations in that snapshot;
- exact same admission/proposal lineage;
- trusted conductor/cell/DNA provenance;
- non-serializable adapter output.

Even then, model-backed clinical presentation remains gated by structural OOD assessment, evaluator trust, patient/evidence/calibration/missingness and practitioner-authority proof lines.

## Evidence policy

Branch existence, PR mergeability, or queued CI are not qualification. Record only exact-head gates that actually execute successfully.
