# Medication Administration Attestation v1

Status: experimental / draft qualification

This contract defines the first DHT boundary for clinician-performed medication administration. It is intentionally downstream of the pure `mycelix-medication-administration` capability/receipt layer.

## Purpose

A private `MedicationAdministrationReceiptV1` is not itself a DHT-valid clinical fact. Before publication, a deployment-trusted verifier must revalidate the protected receipt and its referenced evidence, then obtain an exact, short-lived authorization from a DNA-pinned administration root.

The DHT publishes only a privacy-minimized projection of that verified lineage.

## Fail-closed deployment root

DNA properties contain:

```yaml
medication_administration:
  schema_version: 1
  root_authorities: []
  max_verifier_authorization_duration_micros: 900000000
```

The repository ships with an empty root set. Qualified administration publication is therefore disabled until a deployment deliberately repacks the DNA with trusted root AgentPubKeys. This trust-root change is part of the DNA hash.

Administration roots are intentionally independent from prescription, activation, emergency-override, and dispensing roots.

## Publication chain

1. A clinician-performed administration event passes the pure administration gate.
2. The non-cloneable capability is consumed into a private administration receipt.
3. A deployment verifier revalidates the receipt and referenced protected evidence.
4. A DNA-pinned root issues a short-lived authorization for one exact verifier and one exact receipt lineage.
5. Only that verifier may publish the matching `QualifiedMedicationAdministration` during the authorization window.
6. Peers validate exact field equality against the root authorization.

No reusable verifier role is created by this protocol.

## Public attestation contents

`MedicationAdministrationAttestationV1` contains only:

- administration identifier;
- administration receipt digest;
- private event digest;
- MedicationRequest artifact digest;
- qualified/emergency activation semantic receipt digest;
- finalized dispense receipt digest;
- administrator principal binding;
- authority-policy digest;
- administration-policy digest.

It intentionally omits patient identity, medication name, dose, route, site, method, diagnosis, notes, and narrative rationale.

## Corrections

Qualified administration entries are append-only. They cannot be updated or deleted.

Corrections use `MedicationAdministrationCorrection` with typed reasons including entered-in-error, wrong patient, wrong medication, wrong dose, wrong route, duplicate documentation, and source-evidence revocation. Sensitive explanatory text remains protected; the public record may contain only a commitment to that rationale.

A correction may be authored by the original qualified-administration publisher or a DNA-pinned administration root. Correction links must bind the exact administration action and semantic receipt lineage and are append-only.

The canonical current interpretation must therefore be derived from:

`qualified administration + valid correction lineage`

rather than from mutable status or deletion metadata.

## Why deletion is not revocation

Holochain dependency retrieval with `must_get_*` does not make later mutable metadata such as delete/update activity part of the referenced record proof. Administration authorization and correction semantics therefore do not rely on deleting an old grant or entry.

Verifier authorization is exact-target and short-lived; clinical correction is append-only.

## Security invariants

- an unconfigured DNA cannot validate a qualified administration;
- a non-root cannot mint verifier authorization;
- a verifier authorization is exact receipt + event + medication + activation + dispense + principal + policies;
- a verifier cannot publish outside its authorization validity window;
- wrong receipt/event/medication/activation/dispense/principal/policy substitution fails closed;
- `Prescribe`/`Dispense` authority cannot be reconstructed from a DHT attestation as `Administer` authority;
- emergency-origin activation remains visible through its digest domain;
- corrections never erase the original claim;
- raw PHI is not required in the distributed administration entry.

## Non-claims

This zome does **not** prove:

- that the physical administration occurred merely because an attestation was committed;
- that an upstream patient-binding adapter is institutionally trustworthy;
- that a private receipt was clinically correct unless the configured verifier actually performs the required revalidation;
- causal attribution between administration and later outcomes;
- global completeness of a read query;
- regulatory or clinical qualification.

Those are separate physical-world, deployment, evidence, read-model, and qualification boundaries.

## Required qualification

Before promotion beyond experimental status, require exact-head build/test plus conductor tests proving at minimum:

- empty-root DNA rejects verifier authorization;
- non-root authorization creation is rejected;
- wrong verifier cannot publish;
- expired/not-yet-valid verifier authorization cannot publish;
- every exact-target substitution fails;
- update/delete of trust/evidence entries is rejected;
- unauthorized correction is rejected;
- wrong-lineage correction/link is rejected;
- duplicate equivalent publication is handled deterministically by the future administration-state reducer;
- competing/corrected administration claims never become a last-write-wins `performed=true` state.
