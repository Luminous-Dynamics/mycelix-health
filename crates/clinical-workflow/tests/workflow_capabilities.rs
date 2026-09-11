use mycelix_clinical_authority::{
    evaluate_authority, AuthorityPermit, AuthorityPurpose, AuthorityRequest, CredentialKind,
    EvidenceDigest, JurisdictionCode, PrincipalBinding, ResolvedCredentialEvidence, ScopeCode,
};
use mycelix_clinical_integrity::{hash_canonical_bytes, DigestDomain, VerifiedDigest};
use mycelix_clinical_quarantine::{
    ArtifactDigest, CommitmentScheme, DigestAlgorithm, OpaqueCommitment, QuarantineEvent,
    QuarantineReasonCode, QuarantineScope, ResolutionDisposition,
};
use mycelix_clinical_workflow::{
    authorize_quarantine_resolution, authorize_resolved_medication_activation,
    hash_quarantine_event, WorkflowError, WorkflowPolicyV1,
};
use mycelix_fhir_medication_semantics::{
    project_active_medication_request_strict, MedicationRequestArtifact,
};
use mycelix_medication_safety::{
    evaluate_medication_safety, hash_safety_knowledge_artifact, CheckRequirement,
    ContextComponent, ContextEvidenceState, ContextRequirement, KnowledgeCoverage,
    MedicationSafetyClearance, MedicationSafetyContextSnapshot, MedicationSafetyPolicyV1,
    SafetyCheckKind, SafetyCheckOutcome, SafetyContextKind, VerifiedSafetyCheck,
};
use mycelix_medication_safety_trust::{
    evaluate_safety_source_trust, SafetySourceTrustPolicyV1, SafetySourceTrustReceipt,
    VerifiedEvaluatorAdmission, VerifiedKnowledgeAdmission,
};
use mycelix_provider_principal::{
    ExternalIdentifier, ExternalReferenceBinding, ExternalResourceKey,
    VerifiedProviderPrincipalBinding,
};
use serde_json::{json, Value};

fn principal(byte: u8) -> PrincipalBinding {
    PrincipalBinding([byte; 32])
}

fn jurisdiction() -> JurisdictionCode {
    JurisdictionCode {
        system: "urn:iso:std:iso:3166-2".into(),
        code: "US-TX".into(),
    }
}

fn scope(code: &str) -> ScopeCode {
    ScopeCode {
        system: "https://mycelix.health/authority-scope/v1".into(),
        code: code.into(),
    }
}

fn authority_policy_digest(bytes: &[u8]) -> VerifiedDigest {
    hash_canonical_bytes(DigestDomain::AuthorityPolicy, bytes).unwrap()
}

fn workflow_policy() -> WorkflowPolicyV1 {
    WorkflowPolicyV1 {
        schema_version: 1,
        max_authority_age_micros: 60_000_000,
        max_requester_resolution_age_micros: 60_000_000,
        max_safety_clearance_age_micros: 60_000_000,
        max_safety_source_trust_age_micros: 60_000_000,
        max_future_skew_micros: 1_000_000,
    }
}

fn credential(
    p: PrincipalBinding,
    scope_code: &str,
    seed: u8,
) -> ResolvedCredentialEvidence {
    ResolvedCredentialEvidence::from_verified_record(
        CredentialKind::PractitionerLicense,
        p,
        vec![jurisdiction()],
        vec![scope(scope_code)],
        0,
        Some(10_000_000_000),
        None,
        EvidenceDigest([seed; 32]),
        EvidenceDigest([seed.wrapping_add(1); 32]),
        EvidenceDigest([seed.wrapping_add(2); 32]),
    )
    .unwrap()
}

fn authority_for(
    target: VerifiedDigest,
    policy: VerifiedDigest,
    purpose: AuthorityPurpose,
    p: PrincipalBinding,
    scope_code: &str,
    evaluated_at: i64,
) -> AuthorityPermit {
    let request = AuthorityRequest {
        principal: p,
        target_artifact_digest: EvidenceDigest(target.value()),
        policy_digest: EvidenceDigest(policy.value()),
        purpose,
        jurisdiction: jurisdiction(),
        required_credential_kinds: vec![CredentialKind::PractitionerLicense],
        required_scopes: vec![scope(scope_code)],
        evaluated_at_micros: evaluated_at,
    };
    evaluate_authority(&request, &[credential(p, scope_code, 11)])
        .unwrap()
        .into_permit()
        .expect("authority should be granted")
}

fn clinical_digest(seed: u8) -> VerifiedDigest {
    hash_canonical_bytes(DigestDomain::ClinicalArtifact, &[seed]).unwrap()
}

fn provider_binding(p: PrincipalBinding) -> VerifiedProviderPrincipalBinding {
    VerifiedProviderPrincipalBinding::from_verified_provider_record(
        p,
        vec![ExternalReferenceBinding {
            source_system: "https://ehr.example/fhir".into(),
            resource: ExternalResourceKey {
                resource_type: "Practitioner".into(),
                id: "prac-1".into(),
            },
        }],
        vec![ExternalIdentifier {
            system: "http://hl7.org/fhir/sid/us-npi".into(),
            value: "1234567893".into(),
        }],
        0,
        Some(10_000_000_000),
        None,
        clinical_digest(21),
        clinical_digest(22),
        clinical_digest(23),
    )
    .unwrap()
}

fn medication_request(id: &str) -> Value {
    json!({
        "resourceType": "MedicationRequest",
        "id": id,
        "status": "active",
        "intent": "order",
        "medicationCodeableConcept": {
            "coding": [{
                "system": "http://www.nlm.nih.gov/research/umls/rxnorm",
                "code": "860975"
            }]
        },
        "subject": { "reference": "Patient/patient-a" },
        "requester": {
            "reference": "Practitioner/prac-1",
            "type": "Practitioner",
            "identifier": {
                "system": "http://hl7.org/fhir/sid/us-npi",
                "value": "1234567893"
            }
        },
        "dosageInstruction": [{
            "sequence": 1,
            "text": "Take one tablet twice daily",
            "timing": {
                "repeat": { "frequency": 2, "period": 1, "periodUnit": "d" }
            },
            "route": {
                "coding": [{"system": "http://snomed.info/sct", "code": "26643006"}]
            },
            "doseAndRate": [{
                "doseQuantity": {
                    "value": 1,
                    "system": "http://unitsofmeasure.org",
                    "code": "{tablet}"
                }
            }]
        }]
    })
}

fn medication_artifact(
    id: &str,
    p: PrincipalBinding,
    resolved_at: i64,
) -> MedicationRequestArtifact {
    let bindings = vec![provider_binding(p)];
    project_active_medication_request_strict(
        &medication_request(id),
        "patient-a",
        "https://ehr.example/fhir",
        resolved_at,
        &bindings,
    )
    .unwrap()
}

fn safety_policy() -> MedicationSafetyPolicyV1 {
    MedicationSafetyPolicyV1 {
        schema_version: 1,
        policy_id: "workflow-test-medication-safety-v1".into(),
        required_context: vec![ContextRequirement {
            kind: SafetyContextKind::AllergiesIntolerances,
            max_age_micros: Some(1_000_000),
            allow_verified_absent: true,
            allow_not_applicable: false,
        }],
        required_checks: vec![CheckRequirement {
            kind: SafetyCheckKind::DrugAllergy,
            max_evaluation_age_micros: 1_000_000,
            max_knowledge_age_micros: 1_000_000,
        }],
        max_future_skew_micros: 0,
    }
}

struct SafetyBundle {
    clearance: MedicationSafetyClearance,
    source_trust: SafetySourceTrustReceipt,
    safety_policy_digest: VerifiedDigest,
    trust_policy_digest: VerifiedDigest,
}

fn safety_bundle(
    artifact: &MedicationRequestArtifact,
    cleared_at: i64,
    trust_evaluated_at: i64,
) -> SafetyBundle {
    let context = MedicationSafetyContextSnapshot::from_verified_components(
        artifact,
        vec![ContextComponent::from_verified_evidence(
            SafetyContextKind::AllergiesIntolerances,
            ContextEvidenceState::VerifiedAbsent,
            clinical_digest(31),
            cleared_at,
            None,
        )
        .unwrap()],
        cleared_at,
    )
    .unwrap();
    let safety_policy = safety_policy();
    let safety_policy_digest = safety_policy.verified_digest().unwrap();
    let evaluator_digest = clinical_digest(32);
    let knowledge_digest =
        hash_safety_knowledge_artifact(b"qualified-test-allergy-knowledge-v1").unwrap();
    let check = VerifiedSafetyCheck::from_verified_evaluation(
        SafetyCheckKind::DrugAllergy,
        artifact.verified_digest().unwrap(),
        context.verified_digest().unwrap(),
        safety_policy_digest,
        knowledge_digest,
        evaluator_digest,
        KnowledgeCoverage::CompleteForPolicy,
        SafetyCheckOutcome::Clear,
        0,
        cleared_at,
        cleared_at,
    )
    .unwrap();
    let checks = vec![check];
    let clearance = evaluate_medication_safety(
        artifact,
        &context,
        &safety_policy,
        &checks,
        cleared_at,
    )
    .unwrap()
    .into_clearance()
    .expect("safety clearance expected");

    let trust_policy = SafetySourceTrustPolicyV1 {
        schema_version: 1,
        policy_id: "workflow-test-safety-source-trust-v1".into(),
        safety_policy_digest: safety_policy_digest.stored(),
        max_future_skew_micros: 0,
    };
    let trust_policy_digest = trust_policy.verified_digest().unwrap();
    let evaluators = vec![VerifiedEvaluatorAdmission::from_verified_record(
        evaluator_digest,
        vec![SafetyCheckKind::DrugAllergy],
        0,
        Some(10_000_000_000),
        None,
        trust_policy_digest,
        clinical_digest(33),
    )
    .unwrap()];
    let knowledge = vec![VerifiedKnowledgeAdmission::from_verified_record(
        knowledge_digest,
        vec![SafetyCheckKind::DrugAllergy],
        0,
        Some(10_000_000_000),
        None,
        trust_policy_digest,
        clinical_digest(34),
    )
    .unwrap()];
    let source_trust = evaluate_safety_source_trust(
        safety_policy_digest,
        &trust_policy,
        &checks,
        &evaluators,
        &knowledge,
        trust_evaluated_at,
    )
    .unwrap();

    SafetyBundle {
        clearance,
        source_trust,
        safety_policy_digest,
        trust_policy_digest,
    }
}

fn expect_workflow_error<T>(result: Result<T, WorkflowError>) -> WorkflowError {
    match result {
        Ok(_) => panic!("expected workflow denial"),
        Err(error) => error,
    }
}

#[test]
fn medication_authority_cannot_cross_artifact_boundary() {
    let p = principal(1);
    let authority_policy = authority_policy_digest(b"prescribing-policy-v1");
    let artifact_a = medication_artifact("order-a", p, 100);
    let artifact_b = medication_artifact("order-b", p, 100);
    let authority = authority_for(
        artifact_a.verified_digest().unwrap(),
        authority_policy,
        AuthorityPurpose::Prescribe,
        p,
        "prescribe",
        100,
    );
    let safety = safety_bundle(&artifact_b, 150, 150);

    let error = expect_workflow_error(authorize_resolved_medication_activation(
        &artifact_b,
        safety.clearance,
        safety.source_trust,
        safety.safety_policy_digest,
        safety.trust_policy_digest,
        authority,
        authority_policy,
        &workflow_policy(),
        p,
        &jurisdiction(),
        200,
    ));
    assert!(matches!(error, WorkflowError::TargetDigestMismatch));
}

#[test]
fn safety_clearance_cannot_cross_medication_boundary() {
    let p = principal(2);
    let authority_policy = authority_policy_digest(b"prescribing-policy-v1");
    let artifact_a = medication_artifact("order-a", p, 100);
    let artifact_b = medication_artifact("order-b", p, 100);
    let authority = authority_for(
        artifact_b.verified_digest().unwrap(),
        authority_policy,
        AuthorityPurpose::Prescribe,
        p,
        "prescribe",
        100,
    );
    let safety = safety_bundle(&artifact_a, 150, 150);

    let error = expect_workflow_error(authorize_resolved_medication_activation(
        &artifact_b,
        safety.clearance,
        safety.source_trust,
        safety.safety_policy_digest,
        safety.trust_policy_digest,
        authority,
        authority_policy,
        &workflow_policy(),
        p,
        &jurisdiction(),
        200,
    ));
    assert!(matches!(
        error,
        WorkflowError::SafetyClearanceMedicationMismatch
    ));
}

#[test]
fn source_trust_policy_substitution_is_rejected() {
    let p = principal(3);
    let authority_policy = authority_policy_digest(b"prescribing-policy-v1");
    let artifact = medication_artifact("order-a", p, 100);
    let authority = authority_for(
        artifact.verified_digest().unwrap(),
        authority_policy,
        AuthorityPurpose::Prescribe,
        p,
        "prescribe",
        100,
    );
    let safety = safety_bundle(&artifact, 150, 150);
    let wrong_trust_policy = hash_canonical_bytes(
        DigestDomain::MedicationSafetyTrustPolicy,
        b"different-source-trust-policy",
    )
    .unwrap();

    let error = expect_workflow_error(authorize_resolved_medication_activation(
        &artifact,
        safety.clearance,
        safety.source_trust,
        safety.safety_policy_digest,
        wrong_trust_policy,
        authority,
        authority_policy,
        &workflow_policy(),
        p,
        &jurisdiction(),
        200,
    ));
    assert!(matches!(
        error,
        WorkflowError::SafetyTrustPolicyDigestMismatch
    ));
}

#[test]
fn requester_principal_substitution_is_rejected_before_action() {
    let requester = principal(4);
    let attacker = principal(5);
    let authority_policy = authority_policy_digest(b"prescribing-policy-v1");
    let artifact = medication_artifact("order-a", requester, 100);
    let authority = authority_for(
        artifact.verified_digest().unwrap(),
        authority_policy,
        AuthorityPurpose::Prescribe,
        requester,
        "prescribe",
        100,
    );
    let safety = safety_bundle(&artifact, 150, 150);

    let error = expect_workflow_error(authorize_resolved_medication_activation(
        &artifact,
        safety.clearance,
        safety.source_trust,
        safety.safety_policy_digest,
        safety.trust_policy_digest,
        authority,
        authority_policy,
        &workflow_policy(),
        attacker,
        &jurisdiction(),
        200,
    ));
    assert!(matches!(error, WorkflowError::RequesterPrincipalMismatch));
}

#[test]
fn stale_source_trust_cannot_be_reused() {
    let p = principal(6);
    let authority_policy = authority_policy_digest(b"prescribing-policy-v1");
    let artifact = medication_artifact("order-a", p, 0);
    let authority = authority_for(
        artifact.verified_digest().unwrap(),
        authority_policy,
        AuthorityPurpose::Prescribe,
        p,
        "prescribe",
        11,
    );
    let safety = safety_bundle(&artifact, 11, 0);
    let strict = WorkflowPolicyV1 {
        schema_version: 1,
        max_authority_age_micros: 100,
        max_requester_resolution_age_micros: 100,
        max_safety_clearance_age_micros: 100,
        max_safety_source_trust_age_micros: 10,
        max_future_skew_micros: 0,
    };

    let error = expect_workflow_error(authorize_resolved_medication_activation(
        &artifact,
        safety.clearance,
        safety.source_trust,
        safety.safety_policy_digest,
        safety.trust_policy_digest,
        authority,
        authority_policy,
        &strict,
        p,
        &jurisdiction(),
        11,
    ));
    assert!(matches!(error, WorkflowError::SafetySourceTrustStale));
}

#[test]
fn clinical_review_authority_cannot_be_replayed_as_prescribing() {
    let p = principal(7);
    let authority_policy = authority_policy_digest(b"review-policy-v1");
    let artifact = medication_artifact("order-a", p, 100);
    let authority = authority_for(
        artifact.verified_digest().unwrap(),
        authority_policy,
        AuthorityPurpose::ClinicalReview,
        p,
        "prescribe",
        100,
    );
    let safety = safety_bundle(&artifact, 150, 150);

    let error = expect_workflow_error(authorize_resolved_medication_activation(
        &artifact,
        safety.clearance,
        safety.source_trust,
        safety.safety_policy_digest,
        safety.trust_policy_digest,
        authority,
        authority_policy,
        &workflow_policy(),
        p,
        &jurisdiction(),
        200,
    ));
    assert!(matches!(error, WorkflowError::WrongAuthorityPurpose { .. }));
}

#[test]
fn successful_medication_capability_binds_four_evidence_lineages() {
    let p = principal(8);
    let authority_policy = authority_policy_digest(b"prescribing-policy-v1");
    let artifact = medication_artifact("order-a", p, 100);
    let artifact_digest = artifact.verified_digest().unwrap();
    let authority = authority_for(
        artifact_digest,
        authority_policy,
        AuthorityPurpose::Prescribe,
        p,
        "prescribe",
        100,
    );
    let safety = safety_bundle(&artifact, 150, 150);
    let safety_policy_digest = safety.safety_policy_digest;
    let trust_policy_digest = safety.trust_policy_digest;

    let capability = authorize_resolved_medication_activation(
        &artifact,
        safety.clearance,
        safety.source_trust,
        safety_policy_digest,
        trust_policy_digest,
        authority,
        authority_policy,
        &workflow_policy(),
        p,
        &jurisdiction(),
        200,
    )
    .unwrap();

    assert_eq!(capability.medication_artifact_digest().value, artifact_digest.value());
    assert_eq!(capability.principal(), p);
    assert_eq!(capability.requester_resolution_evidence().len(), 3);
    assert_eq!(capability.safety_policy_digest().value, safety_policy_digest.value());
    assert_eq!(capability.safety_check_evaluation_digests().len(), 1);
    assert_eq!(capability.safety_trust_policy_digest().value, trust_policy_digest.value());
    assert_eq!(
        capability.safety_trust_receipt_digest().domain,
        DigestDomain::MedicationSafetyTrustReceipt
    );
    assert_eq!(capability.safety_evaluator_admission_evidence().len(), 1);
    assert_eq!(capability.safety_knowledge_admission_evidence().len(), 1);
}

#[test]
fn quarantine_capability_rejects_changed_event() {
    let pending = QuarantineEvent::new_pending(
        [1; 16],
        OpaqueCommitment {
            scheme: CommitmentScheme::HmacSha256,
            value: [2; 32],
        },
        QuarantineScope::ClinicalFact,
        vec![QuarantineReasonCode::SemanticValidationFailure],
        ArtifactDigest {
            algorithm: DigestAlgorithm::Blake3,
            value: [3; 32],
        },
        100,
    )
    .unwrap();
    let review = QuarantineEvent::begin_review(
        &pending,
        ArtifactDigest {
            algorithm: DigestAlgorithm::Blake3,
            value: [4; 32],
        },
        ArtifactDigest {
            algorithm: DigestAlgorithm::Blake3,
            value: [5; 32],
        },
        OpaqueCommitment {
            scheme: CommitmentScheme::HmacSha256,
            value: [6; 32],
        },
        110,
    )
    .unwrap();

    let p = principal(9);
    let authority_policy = authority_policy_digest(b"clinical-review-policy-v1");
    let authority = authority_for(
        hash_quarantine_event(&review).unwrap(),
        authority_policy,
        AuthorityPurpose::ClinicalReview,
        p,
        "clinical-review",
        120,
    );
    let capability = authorize_quarantine_resolution(
        &review,
        ResolutionDisposition::Discard,
        authority,
        authority_policy,
        &workflow_policy(),
        p,
        &jurisdiction(),
        130,
    )
    .unwrap();

    let mut changed = review.clone();
    changed.sequence += 1;
    let error = expect_workflow_error(capability.resolve(
        &changed,
        ArtifactDigest {
            algorithm: DigestAlgorithm::Blake3,
            value: [7; 32],
        },
        OpaqueCommitment {
            scheme: CommitmentScheme::HmacSha256,
            value: [8; 32],
        },
        ArtifactDigest {
            algorithm: DigestAlgorithm::Blake3,
            value: [9; 32],
        },
        None,
        140,
    ));
    assert!(matches!(error, WorkflowError::QuarantineTargetChanged));
}
