# Prescription Activation Migration v1

Status: **experimental / qualification required**

## Purpose

The legacy `Prescription` entry mixes storage of a medication order with a mutable `status` field. That is insufficient for high-assurance clinical activation because `status = Active` does not prove requester identity, prescribing authority, safety coverage, safety-source trust, or workflow qualification.

The migration therefore separates **stored medication intent** from **qualified activation**.

## v1 invariant

> A newly committed legacy `Prescription` MUST NOT establish a new active medication merely by setting `status = Active`.

The prescriptions integrity zome enforces this directly. New legacy records may be retained in a non-active state for compatibility/migration, but promotion into `Active` is rejected. Existing historical active records remain readable as legacy data and MUST be marked as such by high-assurance consumers.

## Qualified activation evidence

The future qualified activation persistence boundary MUST bind, at minimum:

1. exact `MedicationRequestArtifact` digest;
2. external requester -> authenticated principal resolution evidence;
3. exact professional authority policy and supporting credential/issuer/status evidence;
4. exact medication-safety policy;
5. exact patient-context snapshot;
6. exact safety-check evaluation digests;
7. exact source-trust policy and evaluator/knowledge admission evidence;
8. exact workflow policy;
9. activation time and activating Holochain author;
10. lineage/version identity sufficient to make replay and substitution detectable.

The current pure capability stack proves these relationships in-process but does not, by itself, make a serialized DHT claim trustworthy. DHT persistence must independently validate the evidence lineage it accepts or bind it to a verifier/trust root that the integrity layer can authenticate.

## Legacy emergency/manual operation

Clinical emergencies must not depend on availability of a CDS or external knowledge service. However, unavailable or incomplete decision support MUST NOT be encoded as `Safe`.

A future emergency/manual activation path should therefore be represented explicitly as:

`Indeterminate + HumanOverride + reason + authority + provenance`

and remain distinguishable from a normally qualified activation.

It must never be normalized into `Cleared` merely because a clinician exercised an override.

## Migration phases

### Phase A — fail closed on legacy activation

- reject new legacy `Prescription(status = Active)` commits;
- reject legacy update transitions into `Active`;
- permit existing historical active records to remain readable;
- harden legacy update/delete/link author and target consistency;
- stop CDS outage/skipped/malformed results from being labeled `Safe`.

### Phase B — qualified activation record

Introduce a DHT representation for activation evidence whose verifier/trust roots are not request-controlled. The record should be append-only; correction/revocation should be new lineage events rather than mutation of clinical facts.

### Phase C — read model migration

High-assurance `active medications` queries should derive state from qualified activation/revocation lineage. Legacy `Prescription.status` should become compatibility metadata only.

### Phase D — fill/dispense hardening

A dispense event should require a valid activation lineage plus pharmacist/pharmacy authority appropriate to the jurisdiction and workflow. Mutable refill counters should eventually become derived state from immutable dispense events rather than cross-actor mutation of the original prescription.

## Non-claims

- This migration does not establish that any particular medication is safe or appropriate.
- Existing historical active prescriptions are not retroactively reclassified as qualified.
- A digest alone is not proof that the referenced evidence was trustworthy.
- A coordinator check alone is not a DHT authorization boundary.
- Emergency override is not equivalent to evidence-based clearance.
