use clinical_distribution_lease_integrity::ClinicalDistributionEvaluatorLeaseV1;
use clinical_distribution_lease_zome::{
    DistributionEvaluatorLeaseReadBoundaryV1, DistributionEvaluatorPositiveLeaseObservationV1,
};
use clinical_distribution_trust_integrity::{
    DistributionEvaluatorAdmissionProposalV1, QualifiedDistributionEvaluatorAdmission,
    RuntimeArtifactIdentityV1, RuntimeContentDigestV1,
};
use clinical_distribution_trust_zome::{
    DistributionEvaluatorAdmissionSnapshotV1, DistributionEvaluatorSnapshotReadBoundaryV1,
};
use hdi::prelude::Timestamp;
use holo_hash::ActionHash;
use mycelix_clinical_distribution_currentness::{
    compose_distribution_evaluator_currentness_v1, DistributionEvaluatorCurrentnessPolicyV1,
    VerifiedDistributionEvaluatorCurrentnessV1,
    DISTRIBUTION_EVALUATOR_CURRENTNESS_POLICY_VERSION,
};
use mycelix_clinical_distribution_trust::DistributionEvaluatorTrustPolicyV1;
use mycelix_clinical_evidence::{ArtifactIdentity, ContentDigest};
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
use mycelix_symthaea_clinical_distribution_trust_v3::*;
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
        policy_id: "runtime-trust-admission-v1".into(),
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
        policy_id: "runtime-trust-binding-v1".into(),
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

fn structural_policy() -> SymthaeaClinicalDistributionPolicyV2 {
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

fn validated() -> (
    ValidatedSymthaeaClinicalDistributionAssessmentV2,
    SymthaeaClinicalDistributionPolicyV2,
) {
    let assertion = assertion();
    let policy = structural_policy();
    let validated = validate_symthaea_distribution_assessment_v2(
        &assertion,
        &assessment(&assertion),
        &policy,
        1_150,
    )
    .unwrap();
    (validated, policy)
}

fn currentness_policy() -> DistributionEvaluatorCurrentnessPolicyV1 {
    DistributionEvaluatorCurrentnessPolicyV1 {
        schema_version: DISTRIBUTION_EVALUATOR_CURRENTNESS_POLICY_VERSION,
        policy_id: "symthaea-runtime-currentness-v1".into(),
        max_observation_age_micros: 1_000_000,
        max_inter_observation_gap_micros: 500_000,
    }
}

fn runtime_digest(digest: &ContentDigest) -> RuntimeContentDigestV1 {
    RuntimeContentDigestV1 {
        algorithm: digest.algorithm.clone(),
        value: digest.value.clone(),
    }
}

fn runtime_artifact(artifact: &ArtifactIdentity) -> RuntimeArtifactIdentityV1 {
    RuntimeArtifactIdentityV1 {
        name: artifact.name.clone(),
        version: artifact.version.clone(),
        digest: runtime_digest(&artifact.digest),
    }
}

fn runtime_proposal(
    trust_policy: &DistributionEvaluatorTrustPolicyV1,
) -> DistributionEvaluatorAdmissionProposalV1 {
    DistributionEvaluatorAdmissionProposalV1 {
        schema_version: 1,
        admission_id: "symthaea-runtime-admission-v3".into(),
        detector: runtime_artifact(&trust_policy.required_detector),
        distribution_policy_digest: runtime_digest(&trust_policy.distribution_policy_digest),
        trust_policy_digest: runtime_digest(&trust_policy.digest().unwrap()),
        external_admission_evidence_digest: RuntimeContentDigestV1 {
            algorithm: "blake3-256".into(),
            value: "external-admission-evidence-v3".into(),
        },
        valid_from: Timestamp::from_micros(1_000),
        valid_until: Some(Timestamp::from_micros(2_000_000)),
    }
}

fn action_hash(seed: u8) -> ActionHash {
    ActionHash::from_raw_36(vec![seed; 36])
}

fn currentness_from_proposal(
    proposal: DistributionEvaluatorAdmissionProposalV1,
    policy: &DistributionEvaluatorCurrentnessPolicyV1,
    verified_at_micros: i64,
    lease_valid_until_micros: i64,
) -> VerifiedDistributionEvaluatorCurrentnessV1 {
    let admission_action_hash = action_hash(4);
    let proposal_digest = proposal.digest().unwrap();
    let snapshot = DistributionEvaluatorAdmissionSnapshotV1 {
        admission_action_hash: admission_action_hash.clone(),
        admission: QualifiedDistributionEvaluatorAdmission {
            proposal: proposal.clone(),
            verifier_authorization_hash: action_hash(7),
        },
        revocations: vec![],
        read_boundary: DistributionEvaluatorSnapshotReadBoundaryV1::NetworkBackedStableDoubleRead,
        observed_revocation_target_count: 0,
        observation_started_at: Timestamp::from_micros(1_050),
        observation_completed_at: Timestamp::from_micros(1_080),
    };
    let lease = DistributionEvaluatorPositiveLeaseObservationV1 {
        admission_action_hash: admission_action_hash.clone(),
        admission_proposal_digest: proposal_digest,
        selected_lease_action_hash: action_hash(8),
        selected_lease: ClinicalDistributionEvaluatorLeaseV1 {
            schema_version: 1,
            lease_id: "symthaea-runtime-lease-v3".into(),
            admission_hash: admission_action_hash,
            admission_proposal_digest: proposal_digest,
            detector: proposal.detector.clone(),
            distribution_policy_digest: proposal.distribution_policy_digest.clone(),
            trust_policy_digest: proposal.trust_policy_digest.clone(),
            lease_sequence: 0,
            supersedes_lease_hash: None,
            valid_from: Timestamp::from_micros(1_000),
            valid_until: Timestamp::from_micros(lease_valid_until_micros),
        },
        observed_lease_target_count: 1,
        read_boundary: DistributionEvaluatorLeaseReadBoundaryV1::NetworkBackedStableDoubleRead,
        observation_started_at: Timestamp::from_micros(1_060),
        observation_completed_at: Timestamp::from_micros(1_090),
    };
    compose_distribution_evaluator_currentness_v1(
        &snapshot,
        &lease,
        policy,
        verified_at_micros,
    )
    .unwrap()
}

fn trust_policy(
    policy: &SymthaeaClinicalDistributionPolicyV2,
) -> DistributionEvaluatorTrustPolicyV1 {
    build_symthaea_distribution_trust_policy_v3(
        "symthaea-runtime-trust-v3",
        policy,
        0,
    )
    .unwrap()
}

#[test]
fn exact_assertion_native_runtime_trust_composes() {
    let (structural, policy) = validated();
    let trust = trust_policy(&policy);
    let current_policy = currentness_policy();
    let currentness = currentness_from_proposal(
        runtime_proposal(&trust),
        &current_policy,
        1_110,
        5_000,
    );

    let receipt = evaluate_symthaea_distribution_trust_v3(
        &structural,
        &policy,
        &trust,
        &current_policy,
        &currentness,
        1_150,
    )
    .unwrap();

    assert_eq!(receipt.assertion_digest(), structural.assertion_digest());
    assert_eq!(receipt.wire_digest(), structural.wire_digest());
    assert_eq!(receipt.evidence_context_digest(), structural.evidence_context_digest());
    assert_eq!(receipt.admission_policy_digest(), structural.admission_policy_digest());
    assert_eq!(receipt.subject_id(), "patient-a");
    assert_eq!(receipt.assessed_at_micros(), 1_120);
    assert_eq!(receipt.currentness_verified_at_micros(), 1_110);
    assert_eq!(receipt.trusted_at_micros(), 1_150);
    assert_ne!(receipt.receipt_digest().as_bytes(), &[0u8; 32]);
}

#[test]
fn evaluator_currentness_must_preexist_assessment() {
    let (structural, policy) = validated();
    let trust = trust_policy(&policy);
    let current_policy = currentness_policy();
    let currentness = currentness_from_proposal(
        runtime_proposal(&trust),
        &current_policy,
        1_130,
        5_000,
    );

    assert_eq!(
        evaluate_symthaea_distribution_trust_v3(
            &structural,
            &policy,
            &trust,
            &current_policy,
            &currentness,
            1_150,
        )
        .err(),
        Some(ClinicalDistributionTrustV3Error::CurrentnessEstablishedAfterAssessment)
    );
}

#[test]
fn evaluator_currentness_must_cover_assessment_and_trust_use() {
    let (structural, policy) = validated();
    let trust = trust_policy(&policy);
    let current_policy = currentness_policy();

    let expired_before_assessment = currentness_from_proposal(
        runtime_proposal(&trust),
        &current_policy,
        1_110,
        1_115,
    );
    assert_eq!(
        evaluate_symthaea_distribution_trust_v3(
            &structural,
            &policy,
            &trust,
            &current_policy,
            &expired_before_assessment,
            1_150,
        )
        .err(),
        Some(ClinicalDistributionTrustV3Error::CurrentnessExpiredAtAssessment)
    );

    let expires_before_trust = currentness_from_proposal(
        runtime_proposal(&trust),
        &current_policy,
        1_110,
        1_140,
    );
    assert_eq!(
        evaluate_symthaea_distribution_trust_v3(
            &structural,
            &policy,
            &trust,
            &current_policy,
            &expires_before_trust,
            1_150,
        )
        .err(),
        Some(ClinicalDistributionTrustV3Error::CurrentnessInactiveAtTrustEvaluation)
    );

    let live = currentness_from_proposal(
        runtime_proposal(&trust),
        &current_policy,
        1_110,
        5_000,
    );
    assert_eq!(
        evaluate_symthaea_distribution_trust_v3(
            &structural,
            &policy,
            &trust,
            &current_policy,
            &live,
            1_115,
        )
        .err(),
        Some(ClinicalDistributionTrustV3Error::AssessmentAfterTrustEvaluation)
    );
}

#[test]
fn policy_detector_and_runtime_lineage_substitution_fail_closed() {
    let (structural, policy) = validated();
    let trust = trust_policy(&policy);
    let current_policy = currentness_policy();
    let currentness = currentness_from_proposal(
        runtime_proposal(&trust),
        &current_policy,
        1_110,
        5_000,
    );

    let mut wrong_policy = trust_policy(&policy);
    wrong_policy.distribution_policy_digest.value.push('0');
    assert_eq!(
        evaluate_symthaea_distribution_trust_v3(
            &structural,
            &policy,
            &wrong_policy,
            &current_policy,
            &currentness,
            1_150,
        )
        .err(),
        Some(ClinicalDistributionTrustV3Error::TrustPolicyStructuralPolicyMismatch)
    );

    let mut wrong_detector = trust_policy(&policy);
    wrong_detector.required_detector.digest.value.push('0');
    assert_eq!(
        evaluate_symthaea_distribution_trust_v3(
            &structural,
            &policy,
            &wrong_detector,
            &current_policy,
            &currentness,
            1_150,
        )
        .err(),
        Some(ClinicalDistributionTrustV3Error::TrustPolicyDetectorMismatch)
    );

    let other_trust = build_symthaea_distribution_trust_policy_v3(
        "symthaea-runtime-trust-v3-other",
        &policy,
        0,
    )
    .unwrap();
    let other_currentness = currentness_from_proposal(
        runtime_proposal(&other_trust),
        &current_policy,
        1_110,
        5_000,
    );
    assert_eq!(
        evaluate_symthaea_distribution_trust_v3(
            &structural,
            &policy,
            &trust,
            &current_policy,
            &other_currentness,
            1_150,
        )
        .err(),
        Some(ClinicalDistributionTrustV3Error::CurrentnessTrustPolicyMismatch)
    );
}

#[test]
fn receipt_identity_changes_when_exact_assessment_changes() {
    let assertion = assertion();
    let policy = structural_policy();
    let first_external = assessment(&assertion);
    let first = validate_symthaea_distribution_assessment_v2(
        &assertion,
        &first_external,
        &policy,
        1_150,
    )
    .unwrap();

    let mut second_external = first_external.clone();
    second_external.assessment_id = "symthaea-ood-assessment-2".into();
    second_external.assessment_evidence.as_mut().unwrap().digest[0] ^= 1;
    let second = validate_symthaea_distribution_assessment_v2(
        &assertion,
        &second_external,
        &policy,
        1_150,
    )
    .unwrap();

    let trust = trust_policy(&policy);
    let current_policy = currentness_policy();
    let currentness = currentness_from_proposal(
        runtime_proposal(&trust),
        &current_policy,
        1_110,
        5_000,
    );

    let first_receipt = evaluate_symthaea_distribution_trust_v3(
        &first,
        &policy,
        &trust,
        &current_policy,
        &currentness,
        1_150,
    )
    .unwrap();
    let second_receipt = evaluate_symthaea_distribution_trust_v3(
        &second,
        &policy,
        &trust,
        &current_policy,
        &currentness,
        1_150,
    )
    .unwrap();

    assert_ne!(first.assessment_digest(), second.assessment_digest());
    assert_ne!(first_receipt.receipt_digest(), second_receipt.receipt_digest());
    assert_eq!(first_receipt.assertion_digest(), second_receipt.assertion_digest());
}