use mycelix_clinical_integrity::{hash_canonical_bytes, DigestDomain, VerifiedDigest};
use mycelix_medication_safety::{
    KnowledgeCoverage, SafetyCheckKind, SafetyCheckOutcome, VerifiedSafetyCheck,
};
use mycelix_medication_safety_trust::{
    evaluate_safety_source_trust, SafetySourceTrustError, SafetySourceTrustPolicyV1,
    VerifiedEvaluatorAdmission, VerifiedKnowledgeAdmission,
};

fn clinical(seed: u8) -> VerifiedDigest {
    hash_canonical_bytes(DigestDomain::ClinicalArtifact, &[seed]).unwrap()
}

fn safety_policy(label: &[u8]) -> VerifiedDigest {
    hash_canonical_bytes(DigestDomain::MedicationSafetyPolicy, label).unwrap()
}

fn knowledge(seed: u8) -> VerifiedDigest {
    hash_canonical_bytes(DigestDomain::MedicationSafetyKnowledge, &[seed]).unwrap()
}

fn trust_policy(safety: VerifiedDigest, id: &str) -> SafetySourceTrustPolicyV1 {
    SafetySourceTrustPolicyV1 {
        schema_version: 1,
        policy_id: id.into(),
        safety_policy_digest: safety.stored(),
        max_future_skew_micros: 0,
    }
}

fn check(
    safety: VerifiedDigest,
    evaluator: VerifiedDigest,
    kb: VerifiedDigest,
) -> VerifiedSafetyCheck {
    VerifiedSafetyCheck::from_verified_evaluation(
        SafetyCheckKind::DrugAllergy,
        hash_canonical_bytes(DigestDomain::MedicationRequestArtifact, b"rx-a").unwrap(),
        hash_canonical_bytes(DigestDomain::MedicationSafetyContext, b"ctx-a").unwrap(),
        safety,
        kb,
        evaluator,
        KnowledgeCoverage::CompleteForPolicy,
        SafetyCheckOutcome::Clear,
        0,
        100,
        100,
    )
    .unwrap()
}

#[test]
fn unknown_knowledge_artifact_is_not_admitted() {
    let safety = safety_policy(b"safety-a");
    let policy = trust_policy(safety, "trust-a");
    let trust_digest = policy.verified_digest().unwrap();
    let evaluator = clinical(1);
    let checked_knowledge = knowledge(2);
    let admitted_knowledge = knowledge(3);
    let check = check(safety, evaluator, checked_knowledge);
    let evaluators = vec![VerifiedEvaluatorAdmission::from_verified_record(
        evaluator,
        vec![SafetyCheckKind::DrugAllergy],
        0,
        Some(1_000),
        None,
        trust_digest,
        clinical(4),
    )
    .unwrap()];
    let knowledge = vec![VerifiedKnowledgeAdmission::from_verified_record(
        admitted_knowledge,
        vec![SafetyCheckKind::DrugAllergy],
        0,
        Some(1_000),
        None,
        trust_digest,
        clinical(5),
    )
    .unwrap()];

    assert!(matches!(
        evaluate_safety_source_trust(
            safety,
            &policy,
            &[check],
            &evaluators,
            &knowledge,
            150,
        ),
        Err(SafetySourceTrustError::KnowledgeNotAdmitted(
            SafetyCheckKind::DrugAllergy
        ))
    ));
}

#[test]
fn revoked_knowledge_artifact_is_not_admitted() {
    let safety = safety_policy(b"safety-a");
    let policy = trust_policy(safety, "trust-a");
    let trust_digest = policy.verified_digest().unwrap();
    let evaluator = clinical(1);
    let kb = knowledge(2);
    let check = check(safety, evaluator, kb);
    let evaluators = vec![VerifiedEvaluatorAdmission::from_verified_record(
        evaluator,
        vec![SafetyCheckKind::DrugAllergy],
        0,
        Some(1_000),
        None,
        trust_digest,
        clinical(4),
    )
    .unwrap()];
    let knowledge = vec![VerifiedKnowledgeAdmission::from_verified_record(
        kb,
        vec![SafetyCheckKind::DrugAllergy],
        0,
        Some(1_000),
        Some(120),
        trust_digest,
        clinical(5),
    )
    .unwrap()];

    assert!(matches!(
        evaluate_safety_source_trust(
            safety,
            &policy,
            &[check],
            &evaluators,
            &knowledge,
            150,
        ),
        Err(SafetySourceTrustError::KnowledgeNotAdmitted(
            SafetyCheckKind::DrugAllergy
        ))
    ));
}

#[test]
fn evaluator_admission_from_another_trust_policy_cannot_be_replayed() {
    let safety = safety_policy(b"safety-a");
    let policy = trust_policy(safety, "trust-a");
    let other_policy = trust_policy(safety, "trust-b");
    let trust_digest = policy.verified_digest().unwrap();
    let other_trust_digest = other_policy.verified_digest().unwrap();
    let evaluator = clinical(1);
    let kb = knowledge(2);
    let check = check(safety, evaluator, kb);
    let evaluators = vec![VerifiedEvaluatorAdmission::from_verified_record(
        evaluator,
        vec![SafetyCheckKind::DrugAllergy],
        0,
        Some(1_000),
        None,
        other_trust_digest,
        clinical(4),
    )
    .unwrap()];
    let knowledge = vec![VerifiedKnowledgeAdmission::from_verified_record(
        kb,
        vec![SafetyCheckKind::DrugAllergy],
        0,
        Some(1_000),
        None,
        trust_digest,
        clinical(5),
    )
    .unwrap()];

    assert!(matches!(
        evaluate_safety_source_trust(
            safety,
            &policy,
            &[check],
            &evaluators,
            &knowledge,
            150,
        ),
        Err(SafetySourceTrustError::EvaluatorNotAdmitted(
            SafetyCheckKind::DrugAllergy
        ))
    ));
}
