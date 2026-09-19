use mycelix_clinical_laboratory_semantics::{
    laboratory_result_snapshot_digest_v1, EvidenceFieldV1, LaboratoryProvenanceV1,
    LaboratoryResultStatusV1, LaboratoryResultV1, LaboratoryValueV1, MissingReasonV1,
    TerminologyCodeV1, LABORATORY_RESULT_VERSION, LOINC_SYSTEM,
};
use mycelix_clinical_semantics::{Quantity, SubjectRef};

fn minimal_result() -> LaboratoryResultV1 {
    LaboratoryResultV1 {
        schema_version: LABORATORY_RESULT_VERSION,
        result_id: "lab-result-minimal".to_string(),
        subject: SubjectRef {
            resource_type: "Patient".to_string(),
            id: "patient-a".to_string(),
        },
        analyte: TerminologyCodeV1 {
            system: LOINC_SYSTEM.to_string(),
            code: "14771-0".to_string(),
            version: "2.83".to_string(),
            display: None,
        },
        value: LaboratoryValueV1::Quantity(Quantity::ucum(5.5, "mmol/L")),
        status: LaboratoryResultStatusV1::Final,
        specimen: EvidenceFieldV1::Missing(MissingReasonV1::NotProvided),
        assay: EvidenceFieldV1::Missing(MissingReasonV1::NotProvided),
        reference_intervals: vec![],
        detection_limits: None,
        interpretations: vec![],
        provenance: LaboratoryProvenanceV1 {
            source_system: "https://lab.example/fhir".to_string(),
            source_resource_id: "Observation/obs-minimal".to_string(),
            source_version: Some("1".to_string()),
            laboratory_id: "lab-a".to_string(),
            accession_id: None,
            analyzed_at_micros: None,
            resulted_at_micros: 130,
            performer: None,
            transformation: None,
        },
        uncertainty: None,
        supersedes: None,
    }
}

#[test]
fn patient_substitution_changes_artifact_identity() {
    let original = minimal_result();
    let mut rebound = original.clone();
    rebound.subject.id = "patient-b".to_string();

    assert_ne!(
        laboratory_result_snapshot_digest_v1(&original).unwrap(),
        laboratory_result_snapshot_digest_v1(&rebound).unwrap()
    );
}

#[test]
fn explicit_missingness_changes_artifact_identity() {
    let original = minimal_result();
    let mut changed = original.clone();
    changed.assay = EvidenceFieldV1::Missing(MissingReasonV1::Unknown);

    assert_ne!(
        laboratory_result_snapshot_digest_v1(&original).unwrap(),
        laboratory_result_snapshot_digest_v1(&changed).unwrap()
    );
}
