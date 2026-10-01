# Emergency Medication DHT Trust v1

## Purpose

Emergency medication activation is a **different clinical provenance class** from ordinary qualified activation.

The pure emergency capability from `mycelix-emergency-medication-override` proves that one exact, activation-valid MedicationRequest crossed the emergency uncertainty policy with:

- resolved requester identity;
- professional prescribing authority;
- an evidence-bearing non-cleared medication-safety assessment;
- a versioned emergency policy;
- typed emergency reason;
- protected rationale commitment.

That in-process capability is consumed into an `EmergencyMedicationOverrideReceiptV1`. A serialized receipt remains evidence, not authority. This zome pair adds the distributed trust boundary that allows an institution/deployment to attest that it revalidated one exact private receipt.

## Structural separation

Emergency state is intentionally placed in a separate zome:

- ordinary: `medication_activation`
- emergency: `medication_emergency`

The emergency public type is `EmergencyMedicationActivation`, not `QualifiedMedicationActivation`.

A future combined read model must preserve this distinction, for example:

```text
MedicationActivationState::Qualified(...)
MedicationActivationState::EmergencyOverride(...)
```

It must not flatten both into `active: true` or otherwise erase provenance.

## Privacy-minimized attestation

`EmergencyMedicationAttestationV1` carries only:

- exact MedicationRequest artifact digest;
- exact emergency override receipt digest;
- exact medication-safety assessment digest;
- exact medication-safety policy digest;
- exact professional-authority policy digest;
- exact emergency policy digest;
- `EmergencySafetyBasis::{Indeterminate, RequiresReview}`;
- an opaque activation ID.

It does **not** require patient identity, drug name, diagnosis, dosage, allergies, labs, clinical notes, or emergency rationale text on the DHT.

`EmergencySafetyBasis` deliberately has no `Cleared` or `Blocked` variant. Fully cleared medication belongs on ordinary qualified activation. Known-danger evidence is not uncertainty and requires a separately designed break-glass path if one is ever clinically justified.

## DNA-pinned emergency trust root

The integrity zome reads `dna_info()?.modifiers.properties.medication_emergency`.

The repository manifest ships with:

```yaml
medication_emergency:
  schema_version: 1
  root_authorities: []
  max_verifier_authorization_duration_micros: 900000000
```

This means emergency activation is disabled by default. A test or production deployment must deliberately repack the DNA with its trusted emergency root AgentPubKeys. Ordinary activation roots do not automatically become emergency roots.

The integrity zome independently hard-caps v1 verifier authorization lifetime at 15 minutes. A deployment may package a shorter limit.

## Exact-target verifier authorization

A DNA root can issue `EmergencyMedicationVerifierAuthorization` for exactly:

- one verifier AgentPubKey;
- one private emergency override receipt digest;
- one public emergency attestation digest;
- one medication-safety assessment digest;
- one safety policy digest;
- one professional authority policy digest;
- one emergency policy digest;
- one safety basis;
- one short validity window.

This is not a reusable emergency role grant.

The root/verifier service is responsible for revalidating the private receipt/evidence before issuing this authorization. The DHT does not pretend to infer a private safety result from a digest it cannot inspect.

## Activation validation

To publish `EmergencyMedicationActivation`, integrity validation requires:

1. the referenced root-issued authorization exists and is valid;
2. activation author equals the named verifier grantee;
3. the Holochain activation action timestamp lies inside the authorization window;
4. the exact public attestation digest matches the authorization;
5. exact override receipt, safety assessment, safety policy, authority policy, emergency policy, and safety basis all match the root authorization.

Changing any safety-significant field invalidates the authorization.

## No delete-as-revocation assumption

Holochain `must_get_*` does not make later delete/update metadata part of inductive validity. Therefore this design does not rely on deleting verifier authorization records to revoke their past validity.

All emergency trust/evidence entries are append-only. Authorizations are exact-target and short-lived. If a root no longer trusts a verifier, it stops issuing future authorizations.

Emergency medication state itself is ended/corrected with append-only `EmergencyMedicationTermination` events.

## Termination

Termination references:

- exact emergency activation action;
- exact emergency override receipt digest;
- typed termination reason;
- optional commitment to protected rationale.

The activation verifier or a DNA-pinned emergency root can publish a termination. Termination links are append-only.

## Threat model and non-claims

This layer protects against an ordinary/modified coordinator minting an emergency activation without the exact DNA-rooted authorization.

It does not prove that a root authority is operationally uncompromised. Root-key compromise is high impact and production deployments should use strong key custody; threshold/rotation support is a later hardening target.

The DHT does not independently rerun private medical reasoning. It verifies that a deployment-pinned authority approved one exact evidence receipt and public projection.

No emergency activation in this design is represented as `Cleared`.
