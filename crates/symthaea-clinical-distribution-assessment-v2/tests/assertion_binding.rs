use mycelix_clinical_fact_snapshot::clinical_fact_snapshot_digest_v1;
use mycelix_clinical_semantics::{
    ClinicalFact, ClinicalValue, CodeableConcept, Coding, FactProvenance, Quantity, SubjectRef,
    TransformationProvenance, Uncertainty,
};
use mycelix_symthaea_clinical_admission_v2::{
    admit_symthaea_clinical_inference_v2, SymthaeaAdmissionCeilingV2,
    SymthaeaClinicalAdmissionPolicyV2, SYMTHAEA_CLINICAL_ADMISSION_POLICY_V2_VERSION,
};
use mycelix_symthaea_clinical_distribution_assessment_v2::*;
use mycelix_symthaea_clinical_evidence_context::compose_verified_symthaea_evidence_context_v1;
use mycelix_symthaea_clinical_fact_binding::{
    verify_symthaea_clinical_fact_bindings_v1, SymthaeaFactBindingPolicyV1,
    SYMTHAEA_FACT_BINDING_POLICY_VERSION,
};
use mycelix_symthaea_clinical_model_assertion::{
    build_verified_symthaea_model_assertion_v1, VerifiedSymthaeaModelAssertionV1,
};
use mycelix_symthaea_clinical_wire_v2::{
    verify_symthaea_clinical_wire_v2, SYMTHAEA_CLINICAL_ENVELOPE_V2_VERSION,
    SYMTHAEA_CLINICAL_WIRE_V2_VERSION,
};

const VECTOR_HEX: &str = include_str!(
    "../../symthaea-clinical-wire-v2/fixtures/clinical_inference_wire_v2.hex"
);

fn fixture() -> Vec<u8> {
    let compact: Vec<u8> = VECTOR_HEX
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
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

fn admission_policy(wire: &[u8]) -> SymthaeaClinicalAdmissionPolicyV2 {
    let envelope = verify_symthaea_clinical_wire_v2(wire).unwrap();
    SymthaeaClinicalAdmissionPolicyV2 {
        schema_version: SYMTHAEA_CLINICAL_ADMISSION_POLICY_V2_VERSION,
        policy_id: "distribution-binding-admission-v1".into(),
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
        policy_id: "distribution-binding-v1".into(),
        external_subject_namespace: "fhir/Patient".into(),
        mycelix_subject_resource_type: "Patient".into(),
    }
}

fn assertion() -> VerifiedSymthaeaModelAssertionV1 {
    let fact = fact();
    let wire = wire_bound_to_fact(&fact);
    let admission =
        admit_symthaea_clinical_inference_v2(&wire, &admission_policy(&wire), 1_100).unwrap();
    let bindings =
        verify_symthaea_clinical_fact_bindings_v1(&wire, &[fact], &binding_policy()).unwrap();
    let context = compose_verified_symthaea_evidence_context_v1(&admission, &bindings).unwrap();
    build_verified_symthaea_model_assertion_v1(&context, &admission).unwrap()
}

fn detector() -> DistributionArtifactIdentityV2 {
    DistributionArtifactIdentityV2 {
        identity_version: SYMTHAEA_DISTRIBUTION_IDENTITY_VERSION,
        name: "mycelix-ood-detector".into(),
        version: "1.0.0".into(),
        digest: [41; 32],
    }
}

fn reference_domain() -> DistributionEvidenceIdentityV2 {
    DistributionEvidenceIdentityV2 {
        identity_version: SYMTHAEA_DISTRIBUTION_IDENTITY_VERSION,
        namespace: "mycelix/distribution-reference-domain/v1".into(),
        artifact_id: "adult-outpatient-v1".into(),
        digest: [42; 32],
    }
}

fn policy() -> SymthaeaClinicalDistributionPolicyV2 {
    SymthaeaClinicalDistributionPolicyV2 {
        schema_version: SYMTHAEA_DISTRIBUTION_POLICY_V2_VERSION,
        policy_id: "symthaea-ood-policy-v2".into(),
        required_detector: detector(),
        required_reference_population: "adult-outpatient-v1".into(),
        required_reference_domain: reference_domain(),
        max_age_micros: 1_000,
        max_future_skew_micros: 10,
    }
}

fn assessment(assertion: &VerifiedSymthaeaModelAssertionV1) -> SymthaeaClinicalDistributionAssessmentV2 {
    SymthaeaClinicalDistributionAssessmentV2 {
        schema_version: SYMTHAEA_DISTRIBUTION_ASSESSMENT_V2_VERSION,
        assessment_id: "symthaea-ood-assessment-1".into(),
        assertion_digest: *assertion.assertion_digest().as_bytes(),
        wire_digest: *assertion.wire_digest().as_bytes(),
        evidence_context_digest: *assertion.context_digest().as_bytes(),
        admission_policy_digest: *assertion.admission_policy_digest().as_bytes(),
        subject_id: assertion.subject_id().into(),
        model_identity_digest: *symthaea_model_identity_digest_v1(assertion.model())
            .unwrap()
            .as_bytes(),
        detector: detector(),
        reference_population: "adult-outpatient-v1".into(),
        reference_domain: reference_domain(),
        status: SymthaeaDistributionAssessmentStatusV2::InDistribution,
        numeric_boundary: Some(SymthaeaDistributionNumericBoundaryV2 {
            score: 0.8,
            threshold: 0.5,
            direction: SymthaeaDistributionDirectionV2::HigherMeansMoreInDistribution,
        }),
        assessment_evidence: Some(DistributionEvidenceIdentityV2 {
            identity_version: SYMTHAEA_DISTRIBUTION_IDENTITY_VERSION,
            namespace: "mycelix/ood-assessment-evidence/v2".into(),
            artifact_id: "assessment-output-1".into(),
            digest: [43; 32],
        }),
        assessed_at_micros: 1_120,
    }
}

#[test]
fn exact_assertion_binding_validates() {
    let assertion = assertion();
    let validated = validate_symthaea_distribution_assessment_v2(
        &assertion,
        &assessment(&assertion),
        &policy(),
        1_150,
    )
    .unwrap();

    assert_eq!(validated.assertion_digest(), assertion.assertion_digest().as_bytes());
    assert_eq!(validated.wire_digest(), assertion.wire_digest().as_bytes());
    assert_eq!(validated.evidence_context_digest(), assertion.context_digest().as_bytes());
    assert_eq!(
        validated.admission_policy_digest(),
        assertion.admission_policy_digest().as_bytes()
    );
    assert_eq!(validated.subject_id(), "patient-a");
    assert_eq!(
        validated.status(),
        SymthaeaDistributionAssessmentStatusV2::InDistribution
    );
}

#[test]
fn cross_inference_binding_substitutions_fail_closed() {
    let assertion = assertion();

    let mut cases: Vec<(
        SymthaeaClinicalDistributionAssessmentV2,
        SymthaeaDistributionAssessmentV2Error,
    )> = Vec::new();

    let mut changed = assessment(&assertion);
    changed.assertion_digest[0] ^= 1;
    cases.push((changed, SymthaeaDistributionAssessmentV2Error::AssertionDigestMismatch));

    let mut changed = assessment(&assertion);
    changed.wire_digest[0] ^= 1;
    cases.push((changed, SymthaeaDistributionAssessmentV2Error::WireDigestMismatch));

    let mut changed = assessment(&assertion);
    changed.evidence_context_digest[0] ^= 1;
    cases.push((
        changed,
        SymthaeaDistributionAssessmentV2Error::EvidenceContextDigestMismatch,
    ));

    let mut changed = assessment(&assertion);
    changed.admission_policy_digest[0] ^= 1;
    cases.push((
        changed,
        SymthaeaDistributionAssessmentV2Error::AdmissionPolicyDigestMismatch,
    ));

    let mut changed = assessment(&assertion);
    changed.subject_id = "patient-b".into();
    cases.push((changed, SymthaeaDistributionAssessmentV2Error::SubjectMismatch));

    let mut changed = assessment(&assertion);
    changed.model_identity_digest[0] ^= 1;
    cases.push((changed, SymthaeaDistributionAssessmentV2Error::ModelIdentityMismatch));

    for (assessment, expected) in cases {
        assert_eq!(
            validate_symthaea_distribution_assessment_v2(
                &assertion,
                &assessment,
                &policy(),
                1_150,
            )
            .err(),
            Some(expected)
        );
    }
}

#[test]
fn policy_and_freshness_substitutions_fail_closed() {
    let assertion = assertion();

    let mut wrong_detector = policy();
    wrong_detector.required_detector.digest[0] ^= 1;
    assert_eq!(
        validate_symthaea_distribution_assessment_v2(
            &assertion,
            &assessment(&assertion),
            &wrong_detector,
            1_150,
        )
        .err(),
        Some(SymthaeaDistributionAssessmentV2Error::DetectorMismatch)
    );

    let mut wrong_domain = policy();
    wrong_domain.required_reference_domain.digest[0] ^= 1;
    assert_eq!(
        validate_symthaea_distribution_assessment_v2(
            &assertion,
            &assessment(&assertion),
            &wrong_domain,
            1_150,
        )
        .err(),
        Some(SymthaeaDistributionAssessmentV2Error::ReferenceDomainMismatch)
    );

    let mut stale = assessment(&assertion);
    stale.assessed_at_micros = 100;
    assert_eq!(
        validate_symthaea_distribution_assessment_v2(&assertion, &stale, &policy(), 2_000)
            .err(),
        Some(SymthaeaDistributionAssessmentV2Error::StaleAssessment)
    );

    let mut future = assessment(&assertion);
    future.assessed_at_micros = 1_200;
    assert_eq!(
        validate_symthaea_distribution_assessment_v2(&assertion, &future, &policy(), 1_150)
            .err(),
        Some(SymthaeaDistributionAssessmentV2Error::AssessmentFromFuture)
    );
}
