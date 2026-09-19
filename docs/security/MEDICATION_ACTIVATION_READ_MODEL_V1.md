# Medication Activation Read Model v1

## Status

Experimental. This contract is not clinical qualification evidence until exact-head CI and conductor-level qualification pass.

## Problem

Legacy `Prescription.status == Active` is not sufficient evidence that a medication crossed the high-assurance activation path.

The system now has distinct provenance classes:

- ordinary qualified activation;
- emergency override activation;
- historical legacy prescription state.

A canonical read model must preserve that distinction. It must not flatten them to `active: bool`.

## Canonical state

`mycelix-medication-activation-state` reduces already DHT-validated records into:

- `CurrentActivationState::None`;
- `CurrentActivationState::One(CurrentActivation::Qualified(..))`;
- `CurrentActivationState::One(CurrentActivation::EmergencyOverride(..))`;
- `CurrentActivationState::Conflict { .. }`;
- terminated qualified/emergency history;
- `legacy_unqualified` historical Active records.

A legacy Active prescription is never promoted into canonical current activation.

## Semantic identity

Qualified activation identity is the exact `MedicationActivationReceipt` digest.

Emergency override activation identity is the exact `EmergencyMedicationOverrideReceipt` digest.

Multiple DHT actions carrying the exact same attestation/receipt are treated as idempotent publications of one semantic lineage. If the same receipt digest appears with different attestation fields, reduction fails closed.

## Terminal lineage

A qualified revocation must reference:

- an activation action present in the reducer input; and
- the exact activation receipt digest of that action.

An emergency termination must similarly reference:

- an emergency activation action present in the input; and
- the exact override receipt digest.

A valid terminal event against any duplicate publication terminates the semantic receipt lineage. This prevents duplicate publication from resurrecting already terminated clinical state.

Orphan or receipt-mismatched terminal events are errors, not ignored metadata.

## Conflict semantics

The reducer never uses last-write-wins.

If more than one distinct qualified/emergency semantic lineage remains current, the result is `CurrentActivationState::Conflict`.

Consumers must reconcile the conflict rather than choosing based on input order, DHT arrival order, activation ID, or local wall clock.

This is deliberate: the current reducer does not yet possess enough evidence to prove safe supersession ordering across independent lineages.

## Legacy migration

Historical legacy Active prescriptions remain visible in `legacy_unqualified` so operators can identify records requiring migration/reconciliation.

They are excluded from canonical current activation even when their old status says `Active`.

The existing `get_active_prescriptions()` endpoint therefore remains a legacy compatibility surface and must not be used as clinical qualification evidence once the new DHT adapter/read endpoint lands.

## Privacy

The pure reducer does not introduce new distributed indexes. It operates on records supplied by an adapter.

Privacy-safe discovery/indexing must be designed separately. In particular, the project should not introduce patient-identifier or medication-name public indexes merely to make canonical activation lookup convenient.

## Trust boundary

This reducer is not a substitute for Holochain integrity validation.

Its input adapter must supply records that already crossed the corresponding integrity zomes. The reducer independently rechecks semantic digest domains and terminal linkage so incomplete or malformed fetched record sets fail closed.

## Required qualification

Before promotion:

1. unit/property tests for qualified, emergency, duplicate, terminal, orphan, mismatch, conflict, and legacy states;
2. exact-head `cargo fmt`, Clippy, build, and tests;
3. conductor tests proving a modified coordinator cannot create invalid activation/termination records;
4. read tests sourced from actual DHT-valid records rather than handcrafted structs alone;
5. privacy review of discovery/indexing;
6. migration of clinical callers away from legacy `Prescription.status` semantics.

## Non-claims

- `CurrentActivationState::One` does not prove medication administration or dispensing.
- A current emergency override is not equivalent to a fully qualified safety clearance.
- Legacy records remain readable but not qualified.
- The reducer does not infer supersession from timestamps alone.
