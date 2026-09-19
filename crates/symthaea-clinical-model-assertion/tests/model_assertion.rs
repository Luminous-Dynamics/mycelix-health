use mycelix_clinical_fact_snapshot::clinical_fact_snapshot_digest_v1;
use mycelix_clinical_semantics::{
    ClinicalFact, ClinicalValue, CodeableConcept, Coding, FactProvenance, Quantity, SubjectRef,
    TransformationProvenance, Uncertainty,
};
use mycelix_symthaea_clinical_admission_v2::{
    admit_symthaea_clinical_inference_v2, AdmittedSymthaeaInferenceV2,
    SymthaeaAdmissionCeilingV2, SymthaeaClinicalAdmissionPolicyV2,
    SYMTHAEA_CLINICAL_ADMISSION_POLICY_V2_VERSION,
};
use mycelix_symthaea_clinical_evidence_context::{
    compose_verified_symthaea_evidence_context_v1, VerifiedSymthaeaEvidenceContextV1,
};
use mycelix_symthaea_clinical_fact_binding::{
    verify_symthaea_clinical_fact_bindings_v1, SymthaeaFactBindingPolicyV1,
    SYMTHAEA_FACT_BINDING_POLICY_VERSION,
};
use mycelix_symthaea_clinical_model_assertion::{
    build_verified_symthaea_model_assertion_v1, ModelAssertionKindV1,
    SymthaeaModelAssertionError,
};
use mycelix_symthaea_clinical_wire_v2::{
    verify_symthaea_clinical_wire_v2, SymthaeaCalibrationStatusV2,
    SYMTHAEA_CLINICAL_ENVELOPE_V2_VERSION, SYMTHAEA_CLINICAL_WIRE_V2_VERSION,
};

const VECTOR_HEX: &str = include_str!(
    "../../symthaea-clinical-wire-v2/fixtures/clinical_inference_wire_v2.hex"
);

fn fixture() -> Vec<u8> {
    let compact: Vec<u8> = VECTOR_HEX
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    assert_eq!(compact.len() % 2, 0);
    compact
        .chunks_exact(2)
        .map(|pair| (hex(pair[0]) << 4) | hex(pair[1]))
        .collect()
}

fn hex(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        b'A'..=b'F' => value - b'A' + 10,
        _ => panic!("invalid fixture hex"),
    }
}

fn fact() -> ClinicalFact {
    ClinicalFact {
        fact_id: "fact-1".into(),
        subject: SubjectRef {
            resource_type: "Patient".into(),
            id: "patient-a".into(),
        },
        concept: CodeableConcept {
            coding: vec![Coding {
                system: "http://loinc.org".into(),
                code: "718-7".into(),
                display: Some("Hemoglobin [Mass/volume] in Blood".into()),
                version: Some("2.80".into()),
            }],
            text: Some("Hemoglobin".into()),
        },
        value: ClinicalValue::Quantity(Quantity::ucum(13.7, "g/dL")),
        effective_at_micros: 900,
        provenance: FactProvenance {
            source_system: "https://ehr.example/fhir".into(),
            source_resource_type: "Observation".into(),
            source_resource_id: "obs-1".into(),
            source_version: Some("7".into()),
            recorded_at_micros: 1_000,
            asserted_by: Some("Practitioner/clinician-a".into()),
            transformation: Some(TransformationProvenance {
                software: "mycelix-fhir-bridge".into(),
                version: "1.0.0".into(),
                operation: "observation_to_clinical_fact".into(),
                input_fact_ids: vec!["source-observation-obs-1".into()],
            }),
        },
        uncertainty: Some(Uncertainty {
            confidence: Some(0.98),
            interpretation: Some("laboratory measurement confidence".into()),
        }),
    }
}

fn wire_bound_to_fact(fact: &ClinicalFact) -> Vec<u8> {
    let mut bytes = fixture();
    let digest = clinical_fact_snapshot_digest_v1(fact).unwrap().into_bytes();
    let needle = [11u8; 32];
    let mut replacements = 0;
    let mut index = 0;
    while index + needle.len() <= bytes.len() {
        if bytes[index..index + needle.len()] == needle {
            bytes[index..index + needle.len()].copy_from_slice(&digest);
            replacements += 1;
            index += needle.len();
        } else {
            index += 1;
        }
    }
    assert_eq!(replacements, 2);
    bytes
}

fn admission_policy(wire: &[u8], policy_id: &str) -> SymthaeaClinicalAdmissionPolicyV2 {
    let envelope = verify_symthaea_clinical_wire_v2(wire).unwrap();
    SymthaeaClinicalAdmissionPolicyV2 {
        schema_version: SYMTHAEA_CLINICAL_ADMISSION_POLICY_V2_VERSION,
        policy_id: policy_id.into(),
        accepted_wire_version: SYMTHAEA_CLINICAL_WIRE_V2_VERSION,
        accepted_envelope_version: SYMTHAEA_CLINICAL_ENVELOPE_V2_VERSION,
        accepted_engine: envelope.execution.engine.clone(),
        accepted_model: envelope.execution.model.clone(),
        accepted_runtime_digest: envelope.execution.runtime_digest,
        accepted_configuration_digest: envelope.execution.configuration_digest,
        accepted_operation: envelope.execution.operation.clone(),
        allowed_claim_kinds: vec![envelope.semantics.claim_kind],
        allowed_evidence_stages: vec![envelope.semantics.evidence_stage],
        required_intended_use: envelope.semantics.intended_use,
        required_subject_namespace: Some("fhir/Patient".into()),
        required_subject_binding_namespace: Some(
            "mycelix/patient-subject-binding-evidence/v1".into(),
        ),
        allowed_claim_evidence_namespaces: vec![
            "mycelix/clinical-fact-snapshot/v1".into(),
        ],
        required_claim_evidence_namespaces: vec![
            "mycelix/clinical-fact-snapshot/v1".into(),
        ],
        required_producer_distribution_namespace: Some(
            "symthaea/ood-detector-evidence/v1".into(),
        ),
        max_inference_age_micros: 10_000,
        max_future_skew_micros: 100,
        require_calibrated_probability: true,
        qualification_ceiling: SymthaeaAdmissionCeilingV2::ShadowClinical,
    }
}

fn binding_policy() -> SymthaeaFactBindingPolicyV1 {
    SymthaeaFactBindingPolicyV1 {
        schema_version: SYMTHAEA_FACT_BINDING_POLICY_VERSION,
        policy_id: "assertion-binding-v1".into(),
        external_subject_namespace: "fhir/Patient".into(),
        mycelix_subject_resource_type: "Patient".into(),
    }
}

fn admitted_and_context(
    wire: &[u8],
    fact: ClinicalFact,
    policy_id: &str,
) -> (AdmittedSymthaeaInferenceV2, VerifiedSymthaeaEvidenceContextV1) {
    let admission =
        admit_symthaea_clinical_inference_v2(wire, &admission_policy(wire, policy_id), 1_100)
            .unwrap();
    let bindings =
        verify_symthaea_clinical_fact_bindings_v1(wire, &[fact], &binding_policy()).unwrap();
    let context = compose_verified_symthaea_evidence_context_v1(&admission, &bindings).unwrap();
    (admission, context)
}

#[test]
fn verified_prediction_remains_model_assertion_not_criterion_state() {
    let fact = fact();
    let wire = wire_bound_to_fact(&fact);
    let (admission, context) = admitted_and_context(&wire, fact, "assertion-admission-v1");

    let assertion = build_verified_symthaea_model_assertion_v1(&context, &admission).unwrap();

    assert_eq!(assertion.kind(), ModelAssertionKindV1::Prediction);
    assert_eq!(assertion.subject_id(), "patient-a");
    assert_eq!(assertion.statement(), "Candidate risk prediction");
    assert_eq!(
        assertion.uncertainty().calibration_status,
        SymthaeaCalibrationStatusV2::Calibrated
    );
    assert!(assertion.uncertainty().calibrated_probability.is_some());
    assert_eq!(assertion.typed_fact_evidence().len(), 1);
    assert_ne!(assertion.assertion_digest().as_bytes(), &[0u8; 32]);
}

#[test]
fn same_wire_under_substituted_admission_policy_cannot_reuse_context() {
    let fact = fact();
    let wire = wire_bound_to_fact(&fact);
    let (_original_admission, context) =
        admitted_and_context(&wire, fact, "assertion-admission-original");
    let substituted = admit_symthaea_clinical_inference_v2(
        &wire,
        &admission_policy(&wire, "assertion-admission-substituted"),
        1_100,
    )
    .unwrap();

    assert_eq!(
        build_verified_symthaea_model_assertion_v1(&context, &substituted).err(),
        Some(SymthaeaModelAssertionError::AdmissionPolicyMismatch)
    );
}
