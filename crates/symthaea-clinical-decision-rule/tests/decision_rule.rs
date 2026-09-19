use mycelix_clinical_fact_snapshot::clinical_fact_snapshot_digest_v1;
use mycelix_clinical_semantics::{
    ClinicalFact, ClinicalValue, CodeableConcept, Coding, FactProvenance, Quantity, SubjectRef,
    TransformationProvenance, Uncertainty,
};
use mycelix_symthaea_clinical_admission_v2::{
    admit_symthaea_clinical_inference_v2, SymthaeaAdmissionCeilingV2,
    SymthaeaClinicalAdmissionPolicyV2, SYMTHAEA_CLINICAL_ADMISSION_POLICY_V2_VERSION,
};
use mycelix_symthaea_clinical_decision_rule::{
    evaluate_clinical_decision_rule_v1, ClinicalDecisionRuleError,
    ClinicalDecisionRulePolicyV1, ClinicalDecisionRuleStateV1,
    DecisionRuleEvidenceIdentityV1, MissingEvidencePolicyV1, ProbabilityComparatorV1,
    CLINICAL_DECISION_RULE_POLICY_VERSION, DECISION_RULE_EVIDENCE_IDENTITY_VERSION,
};
use mycelix_symthaea_clinical_evidence_context::compose_verified_symthaea_evidence_context_v1;
use mycelix_symthaea_clinical_fact_binding::{
    verify_symthaea_clinical_fact_bindings_v1, SymthaeaFactBindingPolicyV1,
    SYMTHAEA_FACT_BINDING_POLICY_VERSION,
};
use mycelix_symthaea_clinical_model_assertion::{
    build_verified_symthaea_model_assertion_v1, ModelAssertionKindV1,
};
use mycelix_symthaea_clinical_wire_v2::{
    verify_symthaea_clinical_wire_v2, SymthaeaEvidenceIdentityV2,
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

fn binding_policy() -> SymthaeaFactBindingPolicyV1 {
    SymthaeaFactBindingPolicyV1 {
        schema_version: SYMTHAEA_FACT_BINDING_POLICY_VERSION,
        policy_id: "decision-rule-binding-v1".into(),
        external_subject_namespace: "fhir/Patient".into(),
        mycelix_subject_resource_type: "Patient".into(),
    }
}

fn build_assertion(
    ceiling: SymthaeaAdmissionCeilingV2,
) -> mycelix_symthaea_clinical_model_assertion::VerifiedSymthaeaModelAssertionV1 {
    let fact = fact();
    let wire = wire_bound_to_fact(&fact);
    let envelope = verify_symthaea_clinical_wire_v2(&wire).unwrap();
    let admission_policy = SymthaeaClinicalAdmissionPolicyV2 {
        schema_version: SYMTHAEA_CLINICAL_ADMISSION_POLICY_V2_VERSION,
        policy_id: "decision-rule-admission-v1".into(),
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
        qualification_ceiling: ceiling,
    };
    let admission = admit_symthaea_clinical_inference_v2(&wire, &admission_policy, 1_100).unwrap();
    let bindings =
        verify_symthaea_clinical_fact_bindings_v1(&wire, &[fact], &binding_policy()).unwrap();
    let context = compose_verified_symthaea_evidence_context_v1(&admission, &bindings).unwrap();
    build_verified_symthaea_model_assertion_v1(&context, &admission).unwrap()
}

fn independent_evidence(namespace: &str, id: &str, byte: u8) -> DecisionRuleEvidenceIdentityV1 {
    DecisionRuleEvidenceIdentityV1 {
        schema_version: DECISION_RULE_EVIDENCE_IDENTITY_VERSION,
        namespace: namespace.into(),
        artifact_id: id.into(),
        digest: [byte; 32],
    }
}

fn policy(
    assertion: &mycelix_symthaea_clinical_model_assertion::VerifiedSymthaeaModelAssertionV1,
    comparator: ProbabilityComparatorV1,
    threshold: f64,
    ceiling: SymthaeaAdmissionCeilingV2,
) -> ClinicalDecisionRulePolicyV1 {
    let calibration: SymthaeaEvidenceIdentityV2 = assertion
        .uncertainty()
        .calibration_evidence
        .clone()
        .expect("fixture carries calibration evidence");
    ClinicalDecisionRulePolicyV1 {
        schema_version: CLINICAL_DECISION_RULE_POLICY_VERSION,
        rule_id: "risk-endpoint-threshold-v1".into(),
        accepted_assertion_kind: ModelAssertionKindV1::Prediction,
        required_model: assertion.model().clone(),
        endpoint_namespace: "mycelix/clinical-endpoint/v1".into(),
        endpoint_id: "example-risk-endpoint".into(),
        prediction_horizon_micros: 86_400_000_000,
        target_population: "validated-target-population-v1".into(),
        care_setting: "shadow-clinical".into(),
        required_intended_use: assertion.intended_use(),
        required_minimum_applicability: assertion.applicability(),
        allowed_evidence_stages: vec![assertion.evidence_stage()],
        comparator,
        probability_threshold: threshold,
        required_calibration_evidence: calibration,
        operating_characteristics_evidence: independent_evidence(
            "mycelix/decision-rule-operating-characteristics/v1",
            "ops-1",
            0x41,
        ),
        external_validation_evidence: vec![independent_evidence(
            "mycelix/decision-rule-external-validation/v1",
            "validation-1",
            0x51,
        )],
        missing_evidence_policy: MissingEvidencePolicyV1::IndeterminateOnCriticalOnly,
        max_assertion_age_micros: 10_000,
        max_future_skew_micros: 100,
        qualification_ceiling: ceiling,
    }
}

#[test]
fn exact_threshold_rule_is_explicit_and_reproducible() {
    let assertion = build_assertion(SymthaeaAdmissionCeilingV2::ShadowClinical);
    let probability = assertion
        .uncertainty()
        .calibrated_probability
        .expect("fixture has calibrated probability");

    let triggered = evaluate_clinical_decision_rule_v1(
        &assertion,
        &policy(
            &assertion,
            ProbabilityComparatorV1::GreaterThanOrEqual,
            probability,
            SymthaeaAdmissionCeilingV2::ShadowClinical,
        ),
        1_100,
    )
    .unwrap();
    assert_eq!(triggered.state(), ClinicalDecisionRuleStateV1::Triggered);

    let not_triggered = evaluate_clinical_decision_rule_v1(
        &assertion,
        &policy(
            &assertion,
            ProbabilityComparatorV1::GreaterThan,
            probability,
            SymthaeaAdmissionCeilingV2::ShadowClinical,
        ),
        1_100,
    )
    .unwrap();
    assert_eq!(
        not_triggered.state(),
        ClinicalDecisionRuleStateV1::NotTriggered
    );
    assert_ne!(
        triggered.policy_digest().as_bytes(),
        not_triggered.policy_digest().as_bytes()
    );
    assert_ne!(
        triggered.result_digest().as_bytes(),
        not_triggered.result_digest().as_bytes()
    );
}

#[test]
fn decision_rule_cannot_raise_assertion_authority_ceiling() {
    let assertion = build_assertion(SymthaeaAdmissionCeilingV2::ValidatedOffline);
    let probability = assertion.uncertainty().calibrated_probability.unwrap();
    let result = evaluate_clinical_decision_rule_v1(
        &assertion,
        &policy(
            &assertion,
            ProbabilityComparatorV1::GreaterThanOrEqual,
            probability,
            SymthaeaAdmissionCeilingV2::ShadowClinical,
        ),
        1_100,
    );
    assert_eq!(
        result.err(),
        Some(ClinicalDecisionRuleError::QualificationEscalation)
    );
}

#[test]
fn calibration_evidence_substitution_is_rejected() {
    let assertion = build_assertion(SymthaeaAdmissionCeilingV2::ShadowClinical);
    let probability = assertion.uncertainty().calibrated_probability.unwrap();
    let mut rule = policy(
        &assertion,
        ProbabilityComparatorV1::GreaterThanOrEqual,
        probability,
        SymthaeaAdmissionCeilingV2::ShadowClinical,
    );
    rule.required_calibration_evidence.artifact_id = "different-calibration".into();
    assert_eq!(
        evaluate_clinical_decision_rule_v1(&assertion, &rule, 1_100).err(),
        Some(ClinicalDecisionRuleError::CalibrationEvidenceMismatch)
    );
}
