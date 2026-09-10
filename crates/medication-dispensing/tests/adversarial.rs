use holo_hash::ActionHash;
use mycelix_clinical_authority::PrincipalBinding;
use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestAlgorithm, DigestDomain, StoredDigest,
};
use mycelix_medication_activation_state::{
    CurrentActivation, CurrentActivationState, LegacyPrescriptionRecord, LegacyPrescriptionStatus,
    MedicationActivationView, QualifiedActivationLineage, ActivationLifecycle,
};
use mycelix_medication_dispensing::{
    resolve_current_activation_for_dispense, DispenseError, DispenseLedgerSnapshotV1,
    PriorDispenseEvidence, VerifiedPharmacyContext,
};

fn action(byte: u8) -> ActionHash {
    ActionHash::from_raw_36(vec![byte; 36])
}

fn stored(domain: DigestDomain, byte: u8) -> StoredDigest {
    StoredDigest {
        algorithm: DigestAlgorithm::Blake3_256,
        domain,
        value: [byte; 32],
    }
}

fn verified(domain: DigestDomain, byte: u8) -> mycelix_clinical_integrity::VerifiedDigest {
    hash_canonical_bytes(domain, &[byte]).unwrap()
}

fn qualified_lineage(
    medication_artifact_digest: StoredDigest,
    receipt_byte: u8,
) -> QualifiedActivationLineage {
    QualifiedActivationLineage {
        activation_id: format!("qa-{receipt_byte}"),
        activation_action_hashes: vec![action(receipt_byte)],
        medication_artifact_digest,
        activation_receipt_digest: stored(DigestDomain::MedicationActivationReceipt, receipt_byte),
        safety_context_digest: stored(DigestDomain::MedicationSafetyContext, 3),
        safety_trust_receipt_digest: stored(DigestDomain::MedicationSafetyTrustReceipt, 4),
        authority_policy_digest: stored(DigestDomain::AuthorityPolicy, 5),
        safety_policy_digest: stored(DigestDomain::MedicationSafetyPolicy, 6),
        safety_trust_policy_digest: stored(DigestDomain::MedicationSafetyTrustPolicy, 7),
        workflow_policy_digest: stored(DigestDomain::WorkflowPolicy, 8),
        lifecycle: ActivationLifecycle::Current,
    }
}

#[test]
fn legacy_active_does_not_become_dispenseable_activation() {
    let medication = verified(DigestDomain::MedicationRequestArtifact, 1);
    let view = MedicationActivationView {
        current: CurrentActivationState::None,
        history: Vec::new(),
        legacy_unqualified: vec![LegacyPrescriptionRecord {
            action_hash: action(20),
            prescription_id: "legacy-active".into(),
            status: LegacyPrescriptionStatus::Active,
        }],
    };

    assert!(matches!(
        resolve_current_activation_for_dispense(&view, medication, 100),
        Err(DispenseError::NoCurrentActivation)
    ));
}

#[test]
fn multiple_current_lineages_are_not_last_write_wins() {
    let medication = verified(DigestDomain::MedicationRequestArtifact, 1);
    let lineage_a = qualified_lineage(medication.stored(), 10);
    let lineage_b = qualified_lineage(medication.stored(), 11);
    let view = MedicationActivationView {
        current: CurrentActivationState::Conflict {
            current: vec![
                CurrentActivation::Qualified(lineage_a),
                CurrentActivation::Qualified(lineage_b),
            ],
        },
        history: Vec::new(),
        legacy_unqualified: Vec::new(),
    };

    assert!(matches!(
        resolve_current_activation_for_dispense(&view, medication, 100),
        Err(DispenseError::ActivationConflict)
    ));
}

#[test]
fn one_exact_current_qualified_lineage_resolves() {
    let medication = verified(DigestDomain::MedicationRequestArtifact, 1);
    let receipt = stored(DigestDomain::MedicationActivationReceipt, 10);
    let view = MedicationActivationView {
        current: CurrentActivationState::One(CurrentActivation::Qualified(qualified_lineage(
            medication.stored(),
            10,
        ))),
        history: Vec::new(),
        legacy_unqualified: Vec::new(),
    };

    let resolved = resolve_current_activation_for_dispense(&view, medication, 100).unwrap();
    assert_eq!(resolved.medication_artifact_digest(), medication.stored());
    assert!(matches!(
        resolved.provenance(),
        mycelix_medication_dispensing::DispenseActivationProvenance::Qualified {
            activation_receipt_digest
        } if *activation_receipt_digest == receipt
    ));
}

#[test]
fn pharmacy_context_requires_domain_specific_evidence() {
    let principal = PrincipalBinding([9; 32]);
    let result = VerifiedPharmacyContext::from_verified_evidence(
        principal,
        "pharmacy-1".into(),
        verified(DigestDomain::ClinicalArtifact, 1),
        verified(DigestDomain::PharmacyAffiliationEvidence, 2),
        verified(DigestDomain::PharmacyStatusEvidence, 3),
        0,
        Some(1_000),
        None,
        10,
    );

    assert!(matches!(
        result,
        Err(DispenseError::Integrity(_))
    ));
}

#[test]
fn duplicate_prior_receipt_cannot_fill_two_slots() {
    let medication = verified(DigestDomain::MedicationRequestArtifact, 1);
    let activation = stored(DigestDomain::MedicationActivationReceipt, 2);
    let receipt = verified(DigestDomain::MedicationDispenseReceipt, 3);
    let first = PriorDispenseEvidence::from_verified_receipt(0, receipt).unwrap();
    let replay = PriorDispenseEvidence::from_verified_receipt(1, receipt).unwrap();

    let result = DispenseLedgerSnapshotV1::from_verified_receipts(
        medication,
        activation,
        vec![first, replay],
        100,
    );
    assert!(matches!(
        result,
        Err(DispenseError::DuplicatePriorDispenseReceipt)
    ));
}
