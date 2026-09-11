# Medication Administration v1

Status: experimental / draft qualification

## Purpose

`mycelix-medication-administration` represents clinician-performed medication administration as a distinct evidence-bearing transition.

Dispensing proves that a medication was released. It does **not** prove that the medication was administered to or taken by the patient. Legacy `MedicationAdherence.dose_taken` remains historical/adherence evidence and is not upgraded into qualified clinician administration.

## Narrow v1 scope

V1 covers clinician-performed administration only when all of the following exist:

- an exact active `MedicationRequestArtifact`;
- exactly one current qualified or policy-permitted emergency activation lineage;
- one exact finalized dispense receipt in a conflict-free dispense lineage;
- an independently verified patient/clinical-subject binding;
- an authenticated administrator principal;
- `AuthorityPurpose::Administer` targeted to the exact administration event;
- typed medication product, dose, route, optional method/site, and effective time;
- exact administration/authority policies and bounded freshness.

Patient self-report, caregiver report, device adherence, facility-stock administration, emergency-stock administration, and medication possession are intentionally separate future evidence classes.

## Administration state

The event vocabulary preserves:

- `Performed`;
- `PartiallyPerformed`;
- `NotDone` with a typed reason;
- `Indeterminate` with protected rationale commitment.

Only `Performed`, or policy-permitted `PartiallyPerformed`, may mint the in-process `MedicationAdministrationCapability`. `NotDone` and `Indeterminate` remain documentable evidence but cannot be promoted into performed administration.

## Exact clinical semantics

For v1 performed administration:

- `dosage_index` identifies one exact ordered dosage instruction;
- product coding must match the ordered medication coding;
- route must match the exact ordered route;
- required method/site must be present and match;
- an ordered concrete `Quantity` dose is required;
- ordered dose ranges are rejected rather than converted into a guessed point dose;
- a full `Performed` event must match the ordered dose exactly;
- a partial event must be explicitly policy-enabled and its dose must be positive and below the ordered dose;
- actual and ordered dose units must be identical typed units.

## Four independent proof lineages

A performed administration capability composes four independent facts:

1. **Activation** — this exact medication artifact is currently qualified/emergency-active.
2. **Supply** — this exact finalized dispense exists in a conflict-free, contiguous dispense lineage.
3. **Patient binding** — this exact clinical subject resolves to the intended patient record.
4. **Professional authority** — this exact authenticated administrator has `Administer` authority for this exact event/policy/jurisdiction.

No one proof substitutes for another. In particular, `Prescribe` and `Dispense` authority do not imply `Administer` authority.

## Time semantics

The clinical administration time is distinct from workflow authorization/recording time. Policy bounds retrospective charting via `max_recording_delay_micros` and bounds tolerated future clock skew.

This permits realistic delayed chart entry without erasing the distinction between when care occurred and when the record was authorized.

## Privacy

`MedicationAdministrationEventV1` is intended to remain a protected high-detail artifact. A consumed capability produces `MedicationAdministrationReceiptV1`, which carries domain-separated evidence identities and provenance rather than duplicating patient/medication/dose details onto a distributed ledger.

A later DHT attestation layer should authorize one exact receipt and publish only the minimum public coordination evidence required by deployment policy.

## Trust-boundary non-claims

The pure crate does not prove that:

- a `MedicationActivationView` or `MedicationDispenseView` was actually materialized from DHT-valid records;
- the patient-record/binding evidence issuer is institutionally trusted merely because an adapter constructed `VerifiedPatientSubjectBinding`;
- a clinician actually performed the physical act merely because software produced a receipt;
- a later clinical outcome was caused by the administration.

Those are separate adapter, DHT, physical-world, and causal-evidence boundaries.

## Fail-closed behavior

Administration capability creation fails for at least:

- no current activation or activation conflict;
- terminated/wrong-medication activation;
- emergency-origin activation when policy disallows it;
- conflicted/incomplete/missing finalized dispense supply;
- patient subject mismatch;
- stale/future patient/activation/dispense/authority evidence;
- wrong administrator principal;
- wrong authority purpose, target, policy, or jurisdiction;
- wrong product, route, method, site, dose, or dosage index;
- ordered dose ranges in v1;
- excessive retrospective recording delay;
- `NotDone` or `Indeterminate` presented as performed care.

## Qualification targets

Before promotion beyond experimental status require exact-head build/test evidence and integration/conductor tests for:

- `Prescribe`/`Dispense` cannot substitute for `Administer`;
- wrong patient or patient-binding evidence fails;
- activation and dispense provenance substitution fails;
- emergency provenance survives into the administration receipt;
- conflict/incomplete dispense state cannot become administration supply;
- wrong dose/route/product/site/method fails;
- performed vs partial vs not-done vs indeterminate state separation;
- stale/future evidence and excessive recording delay fail;
- duplicate publication is idempotent at the future DHT attestation layer;
- conflicting administration claims remain visible rather than last-write-wins;
- modified coordinators cannot bypass DHT receipt-attestation validation.
