use mycelix_symthaea_clinical_wire::{
    parse_canonical_symthaea_clinical_inference_v1,
    verify_symthaea_clinical_inference_wire_digest_v1,
};

const VECTOR_V1: &[u8] = include_bytes!("../fixtures/clinical_inference_wire_v1.json");

#[test]
fn canonical_cross_repo_vector_is_accepted_independently_by_mycelix() {
    let envelope = parse_canonical_symthaea_clinical_inference_v1(VECTOR_V1)
        .expect("canonical Symthaea v1 vector must parse independently");
    let regenerated = serde_json::to_vec(&envelope)
        .expect("independent mirror must serialize the canonical vector");
    assert_eq!(regenerated.as_slice(), VECTOR_V1);
    assert!(verify_symthaea_clinical_inference_wire_digest_v1(VECTOR_V1).is_ok());
}

#[test]
fn conformance_vector_remains_bound_to_expected_model_and_subject() {
    let envelope = parse_canonical_symthaea_clinical_inference_v1(VECTOR_V1)
        .expect("canonical Symthaea v1 vector must parse independently");
    assert_eq!(envelope.execution.model.model.name, "clinical-model");
    assert_eq!(envelope.execution.model.model.version, "1.0.0");
    let subject = envelope.subject.expect("clinical vector requires a subject");
    assert_eq!(subject.namespace, "fhir/Patient");
    assert_eq!(subject.subject_id, "patient-a");
}
