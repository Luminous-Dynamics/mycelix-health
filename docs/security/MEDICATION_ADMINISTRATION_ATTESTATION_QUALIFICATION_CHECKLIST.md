# Medication Administration Attestation v1 — Qualification Checklist

Status: draft / not qualified

## Source/build gates

- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] build `medication_administration_integrity` for `wasm32-unknown-unknown --release`
- [ ] build `medication_administration` coordinator for `wasm32-unknown-unknown --release`
- [ ] package the DNA with administration zomes present

## Pure/DHT semantic gates

- [ ] attestation digest is deterministic and domain-separated
- [ ] changing `administration_id` changes attestation digest
- [ ] empty/zero/wrong-domain evidence is rejected
- [ ] ordinary and emergency activation receipt domains are accepted distinctly
- [ ] wrong receipt/event/medication/activation/dispense/principal/policy binding is rejected
- [ ] authorization validity interval is positive and <= configured/hard maximum
- [ ] verifier action must occur inside the exact authorization interval
- [ ] update/delete of authorization, administration, and correction entries is rejected
- [ ] correction `Other` requires a nonzero protected-rationale commitment

## Conductor/adversarial gates

Use a real packaged DNA and at least root/verifier/attacker agents.

- [ ] default DNA with `root_authorities: []` rejects authorization creation
- [ ] non-root cannot create verifier authorization
- [ ] root can create one exact short-lived authorization
- [ ] wrong verifier cannot publish the authorized administration
- [ ] correct verifier cannot publish after expiration
- [ ] correct verifier cannot alter `administration_id`
- [ ] correct verifier cannot substitute any receipt/evidence/policy digest
- [ ] equivalent duplicate publication is preserved as duplicate provenance, not duplicate clinical administration
- [ ] unauthorized correction is rejected
- [ ] correction targeting a different semantic receipt is rejected
- [ ] correction link crossing administration lineage is rejected
- [ ] update/delete and correction-link delete are rejected from a modified coordinator/raw call path

## Read-model gates

Before consumers may treat DHT administration records as canonical state:

- [ ] reducer groups equivalent publications by semantic administration receipt
- [ ] corrections apply to the semantic receipt lineage, not just one publication action
- [ ] corrected/entered-in-error administrations do not remain `CurrentPerformed`
- [ ] competing distinct current administration receipts are explicit conflict when they represent the same intended administration slot/event
- [ ] no last-write-wins rule exists
- [ ] incomplete referenced read sets fail closed
- [ ] legacy `MedicationAdherence { dose_taken: true }` cannot become qualified administration

## Deployment gates

- [ ] production roots are explicitly documented and independently controlled
- [ ] verifier process revalidates protected receipt/evidence before requesting root authorization
- [ ] root keys are not exposed to ordinary application/coordinator processes
- [ ] verifier/root audit logs bind exact receipt + attestation digest
- [ ] PHI is absent from public DHT attestation and public correction rationale
- [ ] emergency provenance is visible in downstream administration reads

No checklist completion may be inferred from source presence alone. Exact-head execution evidence is required.
