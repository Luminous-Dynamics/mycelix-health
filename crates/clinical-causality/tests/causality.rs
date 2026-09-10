use mycelix_clinical_causality::*;
use mycelix_clinical_integrity::{DigestAlgorithm, DigestDomain, StoredDigest};
use mycelix_clinical_semantics::{
    ClinicalFact, ClinicalValue, CodeableConcept, Coding, FactProvenance, Quantity, SubjectRef,
};

fn digest(domain: DigestDomain, seed: u8) -> StoredDigest {
    StoredDigest {
        algorithm: DigestAlgorithm::Blake3_256,
        domain,
        value: [seed; 32],
    }
}

fn fact(id: &str, at: i64) -> ClinicalFact {
    ClinicalFact {
        fact_id: id.into(),
        subject: SubjectRef {
            resource_type: "Patient".into(),
            id: "patient-a".into(),
        },
        concept: CodeableConcept {
            coding: vec![Coding {
                system: "http://snomed.info/sct".into(),
                code: "422587007".into(),
                display: Some("Nausea".into()),
                version: None,
            }],
            text: Some("Nausea".into()),
        },
        value: ClinicalValue::Quantity(Quantity::ucum(1.0, "1")),
        effective_at_micros: at,
        provenance: FactProvenance {
            source_system: "test".into(),
            source_resource_type: "Observation".into(),
            source_resource_id: id.into(),
            source_version: None,
            recorded_at_micros: at,
            asserted_by: Some("test-observer".into()),
            transformation: None,
        },
        uncertainty: None,
    }
}

fn observed(onset: i64, end: Option<i64>) -> ObservedClinicalEventV1 {
    ObservedClinicalEventV1 {
        schema_version: 1,
        event_id: "ae-1".into(),
        patient_subject_binding_evidence_digest: digest(
            DigestDomain::PatientSubjectBindingEvidence,
            1,
        ),
        facts: vec![fact("obs-1", onset)],
        onset_micros: onset,
        end_micros: end,
        recorded_at_micros: onset.saturating_add(10),
    }
}

fn exposure(start: i64, end: Option<i64>) -> MedicationAdministrationExposureV1 {
    MedicationAdministrationExposureV1 {
        medication_artifact_digest: digest(DigestDomain::MedicationRequestArtifact, 2),
        administration_receipt_digest: digest(DigestDomain::MedicationAdministrationReceipt, 3),
        administration_occurrence_digest: digest(
            DigestDomain::MedicationAdministrationOccurrence,
            4,
        ),
        patient_subject_binding_evidence_digest: digest(
            DigestDomain::PatientSubjectBindingEvidence,
            1,
        ),
        exposure_start_micros: start,
        exposure_end_micros: end,
    }
}

fn evidence(kind: CausalEvidenceKindV1, direction: EvidenceDirectionV1, seed: u8) -> CausalEvidenceItemV1 {
    CausalEvidenceItemV1 {
        kind,
        direction,
        evidence_digests: if matches!(direction, EvidenceDirectionV1::Unknown | EvidenceDirectionV1::NoFinding) {
            Vec::new()
        } else {
            vec![digest(DigestDomain::EvidenceCapsule, seed)]
        },
    }
}

fn required_reviews() -> Vec<CausalEvidenceItemV1> {
    vec![
        evidence(
            CausalEvidenceKindV1::AlternativeEtiologyReview,
            EvidenceDirectionV1::NoFinding,
            20,
        ),
        evidence(
            CausalEvidenceKindV1::ConcomitantExposureReview,
            EvidenceDirectionV1::NoFinding,
            21,
        ),
    ]
}

#[test]
fn event_before_exposure_cannot_be_upgraded_to_positive_causality() {
    let association = ExposureAssociationV1::derive(&observed(10, Some(20)), exposure(100, None)).unwrap();
    let policy = CausalAssessmentPolicyV1::strict_default("policy-v1");
    let mut items = required_reviews();
    items.push(evidence(
        CausalEvidenceKindV1::KnownMechanism,
        EvidenceDirectionV1::SupportsRelationship,
        30,
    ));
    let result = CausalAssessmentV1::create(
        &association,
        &policy,
        CausalConclusionV1::EvidenceSuggestsRelationship,
        items,
        Vec::new(),
        "expert-review",
        "1",
        None,
        None,
        200,
    );
    assert!(matches!(
        result,
        Err(CausalityError::PositiveConclusionPredatesExposure)
    ));
}

#[test]
fn duplicate_support_kind_does_not_fake_independent_evidence() {
    let association = ExposureAssociationV1::derive(&observed(200, Some(210)), exposure(100, None)).unwrap();
    let policy = CausalAssessmentPolicyV1::strict_default("policy-v1");
    let mut items = required_reviews();
    items.push(evidence(
        CausalEvidenceKindV1::KnownMechanism,
        EvidenceDirectionV1::SupportsRelationship,
        30,
    ));
    items.push(evidence(
        CausalEvidenceKindV1::KnownMechanism,
        EvidenceDirectionV1::SupportsRelationship,
        31,
    ));
    let result = CausalAssessmentV1::create(
        &association,
        &policy,
        CausalConclusionV1::EvidenceSupportsRelationship,
        items,
        Vec::new(),
        "expert-review",
        "1",
        None,
        None,
        220,
    );
    assert!(matches!(
        result,
        Err(CausalityError::InsufficientDistinctSupportingEvidence)
    ));
}

#[test]
fn missing_required_review_forces_positive_assessment_to_fail_closed() {
    let association = ExposureAssociationV1::derive(&observed(200, Some(210)), exposure(100, None)).unwrap();
    let policy = CausalAssessmentPolicyV1::strict_default("policy-v1");
    let items = vec![evidence(
        CausalEvidenceKindV1::KnownMechanism,
        EvidenceDirectionV1::SupportsRelationship,
        30,
    )];
    let result = CausalAssessmentV1::create(
        &association,
        &policy,
        CausalConclusionV1::EvidenceSuggestsRelationship,
        items,
        vec![MissingCausalEvidenceV1 {
            kind: CausalEvidenceKindV1::AlternativeEtiologyReview,
            reason_code: "not-yet-reviewed".into(),
        }],
        "expert-review",
        "1",
        None,
        None,
        220,
    );
    assert!(matches!(
        result,
        Err(CausalityError::PositiveConclusionWithMissingRequiredEvidence)
    ));
}

#[test]
fn external_scale_label_does_not_auto_upgrade_internal_conclusion() {
    let association = ExposureAssociationV1::derive(&observed(200, Some(210)), exposure(100, None)).unwrap();
    let policy = CausalAssessmentPolicyV1::strict_default("policy-v1");
    let assessment = CausalAssessmentV1::create(
        &association,
        &policy,
        CausalConclusionV1::TemporalAssociationOnly,
        Vec::new(),
        Vec::new(),
        "legacy-scale-import",
        "1",
        Some(ExternalCausalityScaleLabelV1 {
            system: "urn:example:legacy-scale".into(),
            version: "7".into(),
            label: "DefinitelyRelated".into(),
        }),
        None,
        220,
    )
    .unwrap();
    assert_eq!(assessment.conclusion, CausalConclusionV1::TemporalAssociationOnly);
}

#[test]
fn mixed_support_and_challenge_are_preserved() {
    let association = ExposureAssociationV1::derive(&observed(200, Some(210)), exposure(100, None)).unwrap();
    let policy = CausalAssessmentPolicyV1::strict_default("policy-v1");
    let mut items = required_reviews();
    items.push(evidence(
        CausalEvidenceKindV1::KnownMechanism,
        EvidenceDirectionV1::SupportsRelationship,
        30,
    ));
    items.push(evidence(
        CausalEvidenceKindV1::BaselineComparison,
        EvidenceDirectionV1::ChallengesRelationship,
        31,
    ));
    let assessment = CausalAssessmentV1::create(
        &association,
        &policy,
        CausalConclusionV1::EvidenceSuggestsRelationship,
        items,
        Vec::new(),
        "expert-review",
        "1",
        None,
        Some(0.55),
        220,
    )
    .unwrap();
    assert!(assessment.has_mixed_directional_evidence());
}

#[test]
fn directional_claim_without_evidence_artifact_is_rejected() {
    let association = ExposureAssociationV1::derive(&observed(200, Some(210)), exposure(100, None)).unwrap();
    let policy = CausalAssessmentPolicyV1::strict_default("policy-v1");
    let mut items = required_reviews();
    items.push(CausalEvidenceItemV1 {
        kind: CausalEvidenceKindV1::KnownMechanism,
        direction: EvidenceDirectionV1::SupportsRelationship,
        evidence_digests: Vec::new(),
    });
    let result = CausalAssessmentV1::create(
        &association,
        &policy,
        CausalConclusionV1::EvidenceSuggestsRelationship,
        items,
        Vec::new(),
        "expert-review",
        "1",
        None,
        None,
        220,
    );
    assert!(matches!(
        result,
        Err(CausalityError::DirectionalEvidenceWithoutArtifact)
    ));
}
