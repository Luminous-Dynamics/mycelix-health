# Medication Activation DHT Trust v1

Status: **experimental / qualification required**

## Goal

Bridge the in-process high-assurance medication workflow into Holochain without allowing an ordinary or modified coordinator to manufacture a `qualified` active-medication claim.

The design intentionally separates three things:

1. **clinical evidence computation** — performed by the pure typed/evidence/authority/safety stack;
2. **audit receipt** — a serializable record of what that stack concluded, which is evidence but not authority merely because it deserializes;
3. **DHT activation attestation** — a privacy-minimized public claim that is accepted only when an explicitly DNA-rooted authority authorized one exact receipt and one exact attestation payload.

## Why native CapGrant is not the trust root

Holochain capability grants govern who may call a zome function. They do not prevent a modified coordinator owned by the same agent from writing another entry path to its own source chain.

The activation authorization must therefore be enforced by the **integrity zome**, using dependencies that remote validators can verify.

## DNA-pinned root

The health DNA carries:

```yaml
properties:
  medication_activation:
    schema_version: 1
    root_authorities: []
    max_verifier_authorization_duration_micros: 900000000
```

`root_authorities` is intentionally empty in the repository manifest. The default DNA therefore cannot issue qualified medication activation authorizations.

A real test/production deployment must deliberately repack the DNA with the approved root AgentPubKeys. That changes the DNA hash and makes the trust decision explicit at deployment time.

The integrity zome reads this configuration via `dna_info()?.modifiers.properties`.

## Exact-target authorization

A `MedicationActivationVerifierAuthorization` is created only by a DNA-pinned root and binds:

- one verifier/grantee AgentPubKey;
- one exact `MedicationActivationReceipt` digest;
- one exact `MedicationActivationAttestation` digest;
- one exact authority-policy digest;
- one exact medication-safety-policy digest;
- one exact medication-safety-source-trust-policy digest;
- one exact workflow-policy digest;
- a bounded validity window.

It is not a reusable role grant.

The code has an absolute v1 maximum authorization lifetime of **15 minutes**, even if DNA properties are accidentally configured with a larger value.

## Why authorization deletion is not revocation

Holochain `must_get_*` retrieves addressable content while intentionally ignoring mutable metadata such as later updates/deletes. Therefore a validator must not assume that deleting an authorization causes `must_get_valid_record(authorization_hash)` to stop resolving the original valid record.

v1 consequently does **not** model verifier-authorization revocation as deletion. Authorization entries are append-only, exact-target, and short-lived.

If a root no longer trusts a verifier, it stops issuing future exact authorizations. An already-issued authorization has only its short remaining validity window and can authorize only its pre-approved receipt/attestation.

A future design may add stronger online/epoch/status machinery, but must prove the semantics under conductor tests before claiming instant revocation.

## Action time, not request time

`QualifiedMedicationActivation` carries no caller-selected `activated_at` field. The integrity validator checks the Holochain **action timestamp** against the root authorization window.

This avoids allowing request data to choose the time used for authority validity.

## Privacy-minimized attestation

The public `MedicationActivationAttestationV1` carries only:

- an opaque activation id;
- exact medication-artifact digest;
- exact activation-receipt digest;
- exact safety-context digest;
- exact source-trust receipt digest;
- exact authority/safety/source-trust/workflow policy digests.

It does not require patient name, medication name, dosage text, diagnosis, allergy data, lab values, or raw safety evidence on the distributed attestation surface.

Sensitive receipt/evidence material may remain in an encrypted/local institutional store subject to clinical retention and audit policy.

## Activation publication

A `QualifiedMedicationActivation` contains:

- the exact `MedicationActivationAttestationV1`;
- the root authorization action hash.

Integrity validation:

1. retrieves the authorization with `must_get_valid_record`;
2. requires activation author == authorization grantee;
3. uses the activation action timestamp and requires it inside the authorization window;
4. recomputes the exact attestation digest;
5. requires exact receipt digest equality;
6. requires exact authority/safety/source-trust/workflow policy equality.

Changing any safety-significant field changes the attestation digest and invalidates the authorization.

## Duplicate publication

Holochain can contain more than one create action for identical content. v1 therefore does not use action-count uniqueness as a safety property.

High-assurance read models should treat the activation receipt/attestation digest as the semantic identity and de-duplicate equivalent publications. A duplicate does not create a second clinical authorization.

## Activation revocation

`MedicationActivationRevocation` is an append-only corrective event referencing:

- an exact activation action;
- the activation's exact receipt digest;
- a typed revocation reason;
- an optional commitment to sensitive rationale.

It may be authored by the activation verifier or a DNA-pinned root. It cannot modify or erase the original evidence history.

Future active-medication read models should derive state from qualified activation + revocation lineage rather than mutating `Prescription.status`.

## Audit receipt boundary

`MedicationActivationAuditReceiptV1` is serializable for audit/persistence, but that does **not** make arbitrary deserialized bytes authoritative. Its validation establishes shape/domain consistency only.

A root/verifier service must independently re-establish the receipt's referenced evidence/policy lineage before issuing a DHT verifier authorization. The root authorization is the explicit bridge from reviewed receipt to distributed qualified-attestation authority.

## Emergency/manual path

Emergency/manual medication action is deliberately **not** encoded as `QualifiedMedicationActivation` in v1.

A future emergency path must be distinguishable as something equivalent to:

`Indeterminate + HumanOverride + reason + authority + provenance`

It must not become `Cleared` merely because the system allowed a human to proceed.

## Qualification gates before production

At minimum:

- Rust format/clippy/build/test on the exact combined head;
- WASM builds for both new zomes;
- DNA packaging with empty roots proves fail-closed behavior;
- test DNA with a known root proves authorized issuance succeeds;
- non-root authorization issuance fails;
- wrong verifier cannot publish activation;
- exact receipt substitution fails;
- attestation field substitution fails;
- policy substitution fails;
- activation outside authorization window fails;
- update/delete attempts fail;
- forged/mismatched revocations fail;
- modified-coordinator conductor test proves direct entry commit cannot bypass integrity validation;
- read-model tests prove duplicate attestation is idempotent and revocation removes semantic activation from the current-state view.

## Non-claims

- The DHT does not itself prove a medication is medically appropriate; it proves a DNA-rooted verifier authorized this exact evidence receipt/attestation.
- The serialized receipt does not recreate the in-process capability.
- Root-key compromise remains a high-impact trust event; production roots need strong operational security and eventually threshold/rotation design.
- v1 does not claim instantaneous verifier revocation; exact-target short-lived authorization bounds that limitation explicitly.
- Existing legacy `Prescription(status = Active)` records are historical data, not retroactively qualified activations.
