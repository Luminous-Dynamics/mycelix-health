use mycelix_symthaea_clinical_admission::{
    admit_symthaea_clinical_inference_v1, SymthaeaAdmissionCeilingV1,
    SymthaeaAdmissionError, SymthaeaClinicalAdmissionPolicyV1,
};
use mycelix_symthaea_clinical_wire::{
    parse_canonical_symthaea_clinical_inference_v1, SymthaeaClinicalInferenceEnvelopeV1,
    SymthaeaDigestAlgorithmV1, SymthaeaDigestV1,
};

const VECTOR_V1: &[u8] = include_bytes!(
    "../../symthaea-clinical-wire/fixtures/clinical_inference_wire_v1.json"
);

fn execution_input_bound_vector() -> SymthaeaClinicalInferenceEnvelopeV1 {
    let mut envelope = parse_canonical_symthaea_clinical_inference_v1(VECTOR_V1).unwrap();
    let subject_digest = envelope.subject.as_ref().unwrap().binding_evidence_digest;
    envelope.execution.input_evidence_digests.push(subject_digest);
    envelope
}

fn policy_for(
    envelope: &SymthaeaClinicalInferenceEnvelopeV1,
) -> SymthaeaClinicalAdmissionPolicyV1 {
    SymthaeaClinicalAdmissionPolicyV1 {
        schema_version: 1,
        policy_id: "calibration-lineage-regression-v1".into(),
        accepted_wire_identity_version: 1,
        accepted_envelope_schema_version: 1,
        accepted_engine: envelope.execution.engine.clone(),
        accepted_model: envelope.execution.model.clone(),
        accepted_runtime_digest: envelope.execution.runtime_digest,
        accepted_configuration_digest: envelope.execution.configuration_digest,
        accepted_operation: envelope.execution.operation.clone(),
        allowed_claim_kinds: vec![envelope.semantics.claim_kind],
        allowed_evidence_stages: vec![envelope.semantics.evidence_stage],
        required_intended_use: envelope.semantics.intended_use,
        required_subject_namespace: Some("fhir/Patient".into()),
        max_inference_age_micros: 10_000,
        max_future_skew_micros: 100,
        require_calibrated_probability: true,
        qualification_ceiling: SymthaeaAdmissionCeilingV1::ShadowClinical,
    }
}

#[test]
fn model_identity_can_match_while_inference_calibration_lineage_is_rejected() {
    let mut envelope = execution_input_bound_vector();

    // Change the admitted model's calibration lineage first, then build policy
    // from that exact model identity. This intentionally passes the outer model
    // identity gate while leaving the inference-level calibration evidence at
    // the original digest.
    envelope.execution.model.calibration_evidence_digest = Some(SymthaeaDigestV1 {
        algorithm: SymthaeaDigestAlgorithmV1::Blake3_256,
        value: [55u8; 32],
    });
    let policy = policy_for(&envelope);

    let bytes = serde_json::to_vec(&envelope).unwrap();
    assert!(matches!(
        admit_symthaea_clinical_inference_v1(&bytes, &policy, 1_100),
        Err(SymthaeaAdmissionError::CalibrationEvidenceMismatch)
    ));
}
