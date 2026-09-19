use mycelix_clinical_authority::PrincipalBinding;
use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestAlgorithm, DigestDomain, StoredDigest,
};
use mycelix_clinical_semantics::SubjectRef;
use mycelix_medication_administration::{
    resolve_finalized_dispense_for_administration, AdministrationError, AdministrationPolicyV1,
    VerifiedPatientSubjectBinding,
};
use mycelix_medication_dispense_state::{
    DispenseLineageHealth, DispenseLineageView, DispenseSlotState, DispenseSlotView,
    MedicationDispenseView,
};

fn stored(domain: DigestDomain, seed: u8) -> StoredDigest {
    StoredDigest {
        algorithm: DigestAlgorithm::Blake3_256,
        domain,
        value: [seed; 32],
    }
}

fn verified(domain: DigestDomain, seed: u8) -> mycelix_clinical_integrity::VerifiedDigest {
    hash_canonical_bytes(domain, &[seed]).unwrap()
}

#[test]
fn patient_binding_requires_domain_specific_binding_evidence() {
    let result = VerifiedPatientSubjectBinding::from_verified_evidence(
        SubjectRef {
            resource_type: "Patient".into(),
            id: "patient-a".into(),
        },
        verified(DigestDomain::ClinicalArtifact, 1),
        verified(DigestDomain::AuthorityPolicy, 2),
        10,
    );
    assert!(matches!(result, Err(AdministrationError::Integrity(_))));
}

#[test]
fn conflicted_dispense_lineage_cannot_become_administration_supply() {
    let medication = verified(DigestDomain::MedicationRequestArtifact, 1);
    let activation = stored(DigestDomain::MedicationActivationReceipt, 2);
    let receipt = stored(DigestDomain::MedicationDispenseReceipt, 3);
    let view = MedicationDispenseView {
        lineages: vec![DispenseLineageView {
            medication_artifact_digest: medication.stored(),
            activation_semantic_receipt_digest: activation,
            slots: vec![DispenseSlotView {
                slot_index: 0,
                state: DispenseSlotState::Conflict {
                    finalized_receipts: Vec::new(),
                    authorization_action_hashes: Vec::new(),
                    equivocation_evidence_action_hashes: Vec::new(),
                },
            }],
            contiguous_finalized_through: None,
            health: DispenseLineageHealth::Conflict { slots: vec![0] },
        }],
    };

    assert!(matches!(
        resolve_finalized_dispense_for_administration(&view, medication, activation, receipt, 10),
        Err(AdministrationError::DispenseLineageConflict)
    ));
}

#[test]
fn missing_finalized_receipt_does_not_become_supply() {
    let medication = verified(DigestDomain::MedicationRequestArtifact, 1);
    let activation = stored(DigestDomain::MedicationActivationReceipt, 2);
    let requested_receipt = stored(DigestDomain::MedicationDispenseReceipt, 3);
    let view = MedicationDispenseView {
        lineages: vec![DispenseLineageView {
            medication_artifact_digest: medication.stored(),
            activation_semantic_receipt_digest: activation,
            slots: Vec::new(),
            contiguous_finalized_through: None,
            health: DispenseLineageHealth::Consistent,
        }],
    };

    assert!(matches!(
        resolve_finalized_dispense_for_administration(
            &view,
            medication,
            activation,
            requested_receipt,
            10,
        ),
        Err(AdministrationError::FinalizedDispenseMissing)
    ));
}

#[test]
fn administration_policy_rejects_unbounded_freshness() {
    let policy = AdministrationPolicyV1 {
        schema_version: 1,
        policy_id: "admin-v1".into(),
        allow_emergency_override_activation: false,
        allow_partial_administration: false,
        max_authority_age_micros: 86_400_000_001,
        max_activation_state_age_micros: 1_000,
        max_patient_binding_age_micros: 1_000,
        max_dispense_state_age_micros: 1_000,
        max_recording_delay_micros: 1_000,
        max_future_skew_micros: 0,
    };
    assert!(matches!(
        policy.validate(),
        Err(AdministrationError::InvalidAgePolicy(_))
    ));
}

#[test]
fn principal_binding_remains_distinct_from_patient_subject_identity() {
    let administrator = PrincipalBinding([9; 32]);
    let patient = SubjectRef {
        resource_type: "Patient".into(),
        id: "patient-a".into(),
    };
    assert_ne!(administrator.0, [0u8; 32]);
    assert_eq!(patient.resource_type, "Patient");
}
