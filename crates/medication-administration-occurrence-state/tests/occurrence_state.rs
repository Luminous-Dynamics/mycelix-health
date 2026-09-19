use holo_hash::ActionHash;
use medication_administration_integrity::{
    MedicationAdministrationAttestationV1, QualifiedMedicationAdministration,
};
use medication_administration_occurrence_integrity::{
    MedicationAdministrationOccurrenceAttestationV1,
    MedicationAdministrationOccurrenceBindingCorrection, OccurrenceBindingCorrectionReason,
    QualifiedMedicationAdministrationOccurrenceBinding,
};
use mycelix_clinical_integrity::{DigestAlgorithm, DigestDomain, StoredDigest};
use mycelix_medication_administration_occurrence_state::{
    reduce_occurrence_aware_administration_state, OccurrenceBindingCorrectionRecord,
    OccurrenceBindingPublicationRecord, OccurrenceCurrentState,
    UnresolvedCurrentAdministrationReason,
};
use mycelix_medication_administration_state::AdministrationPublicationRecord;

fn stored(domain: DigestDomain, seed: u8) -> StoredDigest {
    StoredDigest {
        algorithm: DigestAlgorithm::Blake3_256,
        domain,
        value: [seed; 32],
    }
}

fn action(seed: u8) -> ActionHash {
    ActionHash::from_raw_36(vec![seed; 36])
}

fn publication(action_seed: u8, id: &str, receipt_seed: u8) -> AdministrationPublicationRecord {
    AdministrationPublicationRecord {
        action_hash: action(action_seed),
        administration: QualifiedMedicationAdministration {
            attestation: MedicationAdministrationAttestationV1 {
                schema_version: 1,
                administration_id: id.into(),
                administration_receipt_digest: stored(
                    DigestDomain::MedicationAdministrationReceipt,
                    receipt_seed,
                ),
                event_digest: stored(
                    DigestDomain::MedicationAdministrationEvent,
                    receipt_seed.wrapping_add(1),
                ),
                medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 10),
                activation_semantic_receipt_digest: stored(
                    DigestDomain::MedicationActivationReceipt,
                    11,
                ),
                finalized_dispense_receipt_digest: stored(
                    DigestDomain::MedicationDispenseReceipt,
                    12,
                ),
                administrator_principal_binding: [13; 32],
                authority_policy_digest: stored(DigestDomain::AuthorityPolicy, 14),
                administration_policy_digest: stored(
                    DigestDomain::MedicationAdministrationPolicy,
                    15,
                ),
            },
            verifier_authorization_hash: action(200),
        },
    }
}

fn binding(
    binding_action_seed: u8,
    source: &AdministrationPublicationRecord,
    private_binding_seed: u8,
    occurrence_seed: u8,
) -> OccurrenceBindingPublicationRecord {
    let attestation = &source.administration.attestation;
    OccurrenceBindingPublicationRecord {
        action_hash: action(binding_action_seed),
        binding: QualifiedMedicationAdministrationOccurrenceBinding {
            attestation: MedicationAdministrationOccurrenceAttestationV1 {
                schema_version: 1,
                administration_hash: source.action_hash.clone(),
                occurrence_binding_digest: stored(
                    DigestDomain::MedicationAdministrationOccurrenceBinding,
                    private_binding_seed,
                ),
                administration_receipt_digest: attestation.administration_receipt_digest,
                event_digest: attestation.event_digest,
                occurrence_digest: stored(
                    DigestDomain::MedicationAdministrationOccurrence,
                    occurrence_seed,
                ),
                medication_artifact_digest: attestation.medication_artifact_digest,
                dosage_index: 0,
            },
            verifier_authorization_hash: action(201),
        },
    }
}

fn binding_correction(
    correction_action_seed: u8,
    target: &OccurrenceBindingPublicationRecord,
    reason: OccurrenceBindingCorrectionReason,
) -> OccurrenceBindingCorrectionRecord {
    OccurrenceBindingCorrectionRecord {
        action_hash: action(correction_action_seed),
        correction: MedicationAdministrationOccurrenceBindingCorrection {
            correction_id: format!("binding-correction-{correction_action_seed}"),
            binding_hash: target.action_hash.clone(),
            occurrence_binding_digest: target.binding.attestation.occurrence_binding_digest,
            occurrence_digest: target.binding.attestation.occurrence_digest,
            reason,
            rationale_commitment: None,
        },
    }
}

#[test]
fn different_record_ids_same_clinical_occurrence_are_conflict() {
    let first = publication(1, "human-id-a", 20);
    let second = publication(2, "human-id-b", 30);
    let first_binding = binding(101, &first, 40, 60);
    let second_binding = binding(102, &second, 41, 60);

    let view = reduce_occurrence_aware_administration_state(
        &[first, second],
        &[],
        &[first_binding, second_binding],
        &[],
    )
    .unwrap();

    assert_eq!(view.occurrences.len(), 1);
    assert!(matches!(
        &view.occurrences[0].current,
        OccurrenceCurrentState::Conflict { .. }
    ));
    assert_eq!(view.occurrences[0].current_receipt_digests.len(), 2);
    assert!(view.unresolved_current.is_empty());
}

#[test]
fn current_receipt_without_admitted_binding_stays_unbound() {
    let first = publication(1, "human-id-a", 20);
    let view = reduce_occurrence_aware_administration_state(&[first], &[], &[], &[]).unwrap();

    assert!(view.occurrences.is_empty());
    assert_eq!(view.unresolved_current.len(), 1);
    assert!(matches!(
        &view.unresolved_current[0].reason,
        UnresolvedCurrentAdministrationReason::Unbound
    ));
}

#[test]
fn one_receipt_bound_to_two_occurrences_is_binding_conflict_not_two_doses() {
    let first = publication(1, "human-id-a", 20);
    let binding_a = binding(101, &first, 40, 60);
    let binding_b = binding(102, &first, 41, 61);

    let view = reduce_occurrence_aware_administration_state(
        &[first],
        &[],
        &[binding_a, binding_b],
        &[],
    )
    .unwrap();

    assert!(view.occurrences.is_empty());
    assert_eq!(view.unresolved_current.len(), 1);
    match &view.unresolved_current[0].reason {
        UnresolvedCurrentAdministrationReason::BindingConflict { occurrence_digests } => {
            assert_eq!(occurrence_digests.len(), 2);
        }
        _ => panic!("receipt must remain unresolved under conflicting occurrence bindings"),
    }
}

#[test]
fn different_binding_provenance_same_occurrence_is_not_clinical_conflict() {
    let first = publication(1, "human-id-a", 20);
    let binding_a = binding(101, &first, 40, 60);
    let binding_b = binding(102, &first, 41, 60);

    let view = reduce_occurrence_aware_administration_state(
        &[first],
        &[],
        &[binding_a, binding_b],
        &[],
    )
    .unwrap();

    assert_eq!(view.occurrences.len(), 1);
    assert!(matches!(
        &view.occurrences[0].current,
        OccurrenceCurrentState::One { .. }
    ));
    assert!(view.unresolved_current.is_empty());
}

#[test]
fn explicit_wrong_occurrence_correction_can_resolve_binding_conflict() {
    let first = publication(1, "human-id-a", 20);
    let binding_a = binding(101, &first, 40, 60);
    let binding_b = binding(102, &first, 41, 61);
    let correction = binding_correction(
        111,
        &binding_b,
        OccurrenceBindingCorrectionReason::WrongOccurrence,
    );

    let view = reduce_occurrence_aware_administration_state(
        &[first],
        &[],
        &[binding_a, binding_b],
        &[correction],
    )
    .unwrap();

    assert_eq!(view.occurrences.len(), 1);
    assert_eq!(
        view.occurrences[0].occurrence_digest,
        stored(DigestDomain::MedicationAdministrationOccurrence, 60)
    );
    assert!(matches!(
        &view.occurrences[0].current,
        OccurrenceCurrentState::One { .. }
    ));
    assert!(view.unresolved_current.is_empty());
}
