#![deny(unsafe_code)]
//! Deployment-scoped trust admission for medication safety evidence.
//!
//! `mycelix-medication-safety` answers whether required evidence is complete and
//! clear under a safety policy. This crate answers the separate trust question:
//! were the evaluator and knowledge artifact for every required check actually
//! admitted by deployment policy at the time of use?

use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use mycelix_medication_safety::{SafetyCheckKind, VerifiedSafetyCheck};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000;
const TRUST_POLICY_SCHEMA_TAG: &[u8] = b"mycelix-health/medication-safety-trust-policy-v1";
const TRUST_RECEIPT_SCHEMA_TAG: &[u8] = b"mycelix-health/medication-safety-trust-receipt-v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SafetySourceTrustPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    /// Exact medication-safety policy this trust policy is allowed to qualify.
    pub safety_policy_digest: StoredDigest,
    pub max_future_skew_micros: i64,
}

impl SafetySourceTrustPolicyV1 {
    pub fn validate(&self) -> Result<(), SafetySourceTrustError> {
        if self.schema_version != 1 {
            return Err(SafetySourceTrustError::UnsupportedPolicyVersion(
                self.schema_version,
            ));
        }
        if self.policy_id.trim().is_empty() {
            return Err(SafetySourceTrustError::MissingPolicyId);
        }
        self.safety_policy_digest.validate_shape()?;
        if self.safety_policy_digest.domain != DigestDomain::MedicationSafetyPolicy {
            return Err(SafetySourceTrustError::SafetyPolicyDomainMismatch);
        }
        if self.max_future_skew_micros < 0
            || self.max_future_skew_micros > MAX_FUTURE_SKEW_MICROS
        {
            return Err(SafetySourceTrustError::InvalidFutureSkew);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, SafetySourceTrustError> {
        self.validate()?;
        hash_json(
            DigestDomain::MedicationSafetyTrustPolicy,
            TRUST_POLICY_SCHEMA_TAG,
            self,
        )
    }
}

/// Evidence that deployment configuration admitted a particular evaluator artifact.
///
/// This object intentionally has no serde implementation. The adapter constructing it
/// is responsible for verifying the configuration/signature/registry record represented
/// by `admission_evidence_digest`. This crate then enforces scope, policy and time.
pub struct VerifiedEvaluatorAdmission {
    evaluator_digest: StoredDigest,
    allowed_checks: Vec<SafetyCheckKind>,
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    revoked_at_micros: Option<i64>,
    trust_policy_digest: StoredDigest,
    admission_evidence_digest: StoredDigest,
}

impl VerifiedEvaluatorAdmission {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_record(
        evaluator_digest: VerifiedDigest,
        allowed_checks: Vec<SafetyCheckKind>,
        valid_from_micros: i64,
        valid_until_micros: Option<i64>,
        revoked_at_micros: Option<i64>,
        trust_policy_digest: VerifiedDigest,
        admission_evidence_digest: VerifiedDigest,
    ) -> Result<Self, SafetySourceTrustError> {
        evaluator_digest.require_domain(DigestDomain::ClinicalArtifact)?;
        trust_policy_digest.require_domain(DigestDomain::MedicationSafetyTrustPolicy)?;
        admission_evidence_digest.require_domain(DigestDomain::ClinicalArtifact)?;
        validate_check_set(&allowed_checks)?;
        validate_window(valid_from_micros, valid_until_micros, revoked_at_micros)?;
        Ok(Self {
            evaluator_digest: evaluator_digest.stored(),
            allowed_checks,
            valid_from_micros,
            valid_until_micros,
            revoked_at_micros,
            trust_policy_digest: trust_policy_digest.stored(),
            admission_evidence_digest: admission_evidence_digest.stored(),
        })
    }

    fn active_at(&self, at_micros: i64) -> bool {
        at_micros >= self.valid_from_micros
            && self
                .valid_until_micros
                .map(|until| at_micros < until)
                .unwrap_or(true)
            && self
                .revoked_at_micros
                .map(|revoked| at_micros < revoked)
                .unwrap_or(true)
    }

    fn admits(&self, check: &VerifiedSafetyCheck, trust_policy: StoredDigest) -> bool {
        self.trust_policy_digest == trust_policy
            && self.evaluator_digest == check.evaluator_digest()
            && self.allowed_checks.contains(&check.kind())
    }
}

/// Evidence that deployment policy admitted one exact knowledge snapshot/version.
pub struct VerifiedKnowledgeAdmission {
    knowledge_digest: StoredDigest,
    allowed_checks: Vec<SafetyCheckKind>,
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    revoked_at_micros: Option<i64>,
    trust_policy_digest: StoredDigest,
    admission_evidence_digest: StoredDigest,
}

impl VerifiedKnowledgeAdmission {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_record(
        knowledge_digest: VerifiedDigest,
        allowed_checks: Vec<SafetyCheckKind>,
        valid_from_micros: i64,
        valid_until_micros: Option<i64>,
        revoked_at_micros: Option<i64>,
        trust_policy_digest: VerifiedDigest,
        admission_evidence_digest: VerifiedDigest,
    ) -> Result<Self, SafetySourceTrustError> {
        knowledge_digest.require_domain(DigestDomain::MedicationSafetyKnowledge)?;
        trust_policy_digest.require_domain(DigestDomain::MedicationSafetyTrustPolicy)?;
        admission_evidence_digest.require_domain(DigestDomain::ClinicalArtifact)?;
        validate_check_set(&allowed_checks)?;
        validate_window(valid_from_micros, valid_until_micros, revoked_at_micros)?;
        Ok(Self {
            knowledge_digest: knowledge_digest.stored(),
            allowed_checks,
            valid_from_micros,
            valid_until_micros,
            revoked_at_micros,
            trust_policy_digest: trust_policy_digest.stored(),
            admission_evidence_digest: admission_evidence_digest.stored(),
        })
    }

    fn active_at(&self, at_micros: i64) -> bool {
        at_micros >= self.valid_from_micros
            && self
                .valid_until_micros
                .map(|until| at_micros < until)
                .unwrap_or(true)
            && self
                .revoked_at_micros
                .map(|revoked| at_micros < revoked)
                .unwrap_or(true)
    }

    fn admits(&self, check: &VerifiedSafetyCheck, trust_policy: StoredDigest) -> bool {
        self.trust_policy_digest == trust_policy
            && self.knowledge_digest == check.knowledge_digest()
            && self.allowed_checks.contains(&check.kind())
    }
}

#[derive(Serialize)]
struct ReceiptDigestMaterial {
    safety_policy_digest: StoredDigest,
    trust_policy_digest: StoredDigest,
    check_evaluation_digests: Vec<StoredDigest>,
    evaluator_admission_evidence_digests: Vec<StoredDigest>,
    knowledge_admission_evidence_digests: Vec<StoredDigest>,
    evaluated_at_micros: i64,
}

/// Single-owner proof that every supplied safety check was admitted through an active
/// evaluator record and an active exact-knowledge record under one deployment policy.
/// Intentionally non-cloneable and non-serializable.
pub struct SafetySourceTrustReceipt {
    safety_policy_digest: StoredDigest,
    trust_policy_digest: StoredDigest,
    check_evaluation_digests: Vec<StoredDigest>,
    evaluator_admission_evidence_digests: Vec<StoredDigest>,
    knowledge_admission_evidence_digests: Vec<StoredDigest>,
    evaluated_at_micros: i64,
    receipt_digest: StoredDigest,
}

impl SafetySourceTrustReceipt {
    pub fn safety_policy_digest(&self) -> StoredDigest {
        self.safety_policy_digest
    }

    pub fn trust_policy_digest(&self) -> StoredDigest {
        self.trust_policy_digest
    }

    pub fn check_evaluation_digests(&self) -> &[StoredDigest] {
        &self.check_evaluation_digests
    }

    pub fn evaluator_admission_evidence_digests(&self) -> &[StoredDigest] {
        &self.evaluator_admission_evidence_digests
    }

    pub fn knowledge_admission_evidence_digests(&self) -> &[StoredDigest] {
        &self.knowledge_admission_evidence_digests
    }

    pub fn evaluated_at_micros(&self) -> i64 {
        self.evaluated_at_micros
    }

    pub fn receipt_digest(&self) -> StoredDigest {
        self.receipt_digest
    }
}

pub fn evaluate_safety_source_trust(
    safety_policy_digest: VerifiedDigest,
    trust_policy: &SafetySourceTrustPolicyV1,
    checks: &[VerifiedSafetyCheck],
    evaluator_admissions: &[VerifiedEvaluatorAdmission],
    knowledge_admissions: &[VerifiedKnowledgeAdmission],
    at_micros: i64,
) -> Result<SafetySourceTrustReceipt, SafetySourceTrustError> {
    safety_policy_digest.require_domain(DigestDomain::MedicationSafetyPolicy)?;
    trust_policy.validate()?;
    if trust_policy.safety_policy_digest != safety_policy_digest.stored() {
        return Err(SafetySourceTrustError::SafetyPolicyMismatch);
    }
    if checks.is_empty() {
        return Err(SafetySourceTrustError::NoSafetyChecks);
    }

    let trust_policy_digest = trust_policy.verified_digest()?;
    let stored_trust_policy = trust_policy_digest.stored();
    let mut seen_checks = HashSet::new();
    let mut check_evaluation_digests = Vec::with_capacity(checks.len());
    let mut evaluator_evidence = Vec::with_capacity(checks.len());
    let mut knowledge_evidence = Vec::with_capacity(checks.len());

    for check in checks {
        if !seen_checks.insert(check.kind()) {
            return Err(SafetySourceTrustError::DuplicateCheckKind(check.kind()));
        }

        let evaluation_digest = check.evaluation_digest();
        validate_stored_domain(evaluation_digest, DigestDomain::MedicationSafetyEvaluation)?;
        validate_stored_domain(check.evaluator_digest(), DigestDomain::ClinicalArtifact)?;
        validate_stored_domain(
            check.knowledge_digest(),
            DigestDomain::MedicationSafetyKnowledge,
        )?;

        let evaluator = evaluator_admissions
            .iter()
            .filter(|admission| admission.active_at(at_micros))
            .filter(|admission| admission.admits(check, stored_trust_policy))
            .min_by_key(|admission| admission.admission_evidence_digest.value)
            .ok_or(SafetySourceTrustError::EvaluatorNotAdmitted(check.kind()))?;

        let knowledge = knowledge_admissions
            .iter()
            .filter(|admission| admission.active_at(at_micros))
            .filter(|admission| admission.admits(check, stored_trust_policy))
            .min_by_key(|admission| admission.admission_evidence_digest.value)
            .ok_or(SafetySourceTrustError::KnowledgeNotAdmitted(check.kind()))?;

        check_evaluation_digests.push(evaluation_digest);
        evaluator_evidence.push(evaluator.admission_evidence_digest);
        knowledge_evidence.push(knowledge.admission_evidence_digest);
    }

    let material = ReceiptDigestMaterial {
        safety_policy_digest: safety_policy_digest.stored(),
        trust_policy_digest: stored_trust_policy,
        check_evaluation_digests: check_evaluation_digests.clone(),
        evaluator_admission_evidence_digests: evaluator_evidence.clone(),
        knowledge_admission_evidence_digests: knowledge_evidence.clone(),
        evaluated_at_micros: at_micros,
    };
    let receipt_digest = hash_json(
        DigestDomain::MedicationSafetyTrustReceipt,
        TRUST_RECEIPT_SCHEMA_TAG,
        &material,
    )?;

    Ok(SafetySourceTrustReceipt {
        safety_policy_digest: material.safety_policy_digest,
        trust_policy_digest: material.trust_policy_digest,
        check_evaluation_digests,
        evaluator_admission_evidence_digests: evaluator_evidence,
        knowledge_admission_evidence_digests: knowledge_evidence,
        evaluated_at_micros: at_micros,
        receipt_digest: receipt_digest.stored(),
    })
}

fn validate_check_set(checks: &[SafetyCheckKind]) -> Result<(), SafetySourceTrustError> {
    if checks.is_empty() {
        return Err(SafetySourceTrustError::AdmissionHasNoChecks);
    }
    let mut seen = HashSet::new();
    for check in checks {
        if !seen.insert(*check) {
            return Err(SafetySourceTrustError::DuplicateAdmissionCheck(*check));
        }
    }
    Ok(())
}

fn validate_window(
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    revoked_at_micros: Option<i64>,
) -> Result<(), SafetySourceTrustError> {
    if valid_until_micros.is_some_and(|until| until <= valid_from_micros) {
        return Err(SafetySourceTrustError::InvalidValidityWindow);
    }
    if revoked_at_micros.is_some_and(|revoked| revoked < valid_from_micros) {
        return Err(SafetySourceTrustError::RevocationPredatesValidity);
    }
    Ok(())
}

fn validate_stored_domain(
    digest: StoredDigest,
    expected: DigestDomain,
) -> Result<(), SafetySourceTrustError> {
    digest.validate_shape()?;
    if digest.domain != expected {
        return Err(SafetySourceTrustError::StoredDigestDomainMismatch {
            expected,
            actual: digest.domain,
        });
    }
    Ok(())
}

fn hash_json<T: Serialize>(
    domain: DigestDomain,
    schema_tag: &[u8],
    value: &T,
) -> Result<VerifiedDigest, SafetySourceTrustError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| SafetySourceTrustError::Serialization(error.to_string()))?;
    let mut framed = Vec::with_capacity(schema_tag.len() + 1 + encoded.len());
    framed.extend_from_slice(schema_tag);
    framed.push(0);
    framed.extend_from_slice(&encoded);
    Ok(hash_canonical_bytes(domain, &framed)?)
}

#[derive(Debug, Error)]
pub enum SafetySourceTrustError {
    #[error("unsupported medication safety trust policy version {0}")]
    UnsupportedPolicyVersion(u16),
    #[error("medication safety trust policy id is required")]
    MissingPolicyId,
    #[error("trust policy safety-policy digest must use MedicationSafetyPolicy domain")]
    SafetyPolicyDomainMismatch,
    #[error("trust policy does not qualify the supplied medication safety policy")]
    SafetyPolicyMismatch,
    #[error("trust policy future-skew allowance must be between 0 and five minutes")]
    InvalidFutureSkew,
    #[error("safety-source admission must authorize at least one check kind")]
    AdmissionHasNoChecks,
    #[error("duplicate check kind in safety-source admission: {0:?}")]
    DuplicateAdmissionCheck(SafetyCheckKind),
    #[error("safety-source admission validity window is invalid")]
    InvalidValidityWindow,
    #[error("safety-source admission revocation predates validity")]
    RevocationPredatesValidity,
    #[error("at least one safety check is required for source-trust evaluation")]
    NoSafetyChecks,
    #[error("duplicate safety check kind in trust evaluation: {0:?}")]
    DuplicateCheckKind(SafetyCheckKind),
    #[error("no active trusted evaluator admission covers safety check {0:?}")]
    EvaluatorNotAdmitted(SafetyCheckKind),
    #[error("no active trusted knowledge admission covers safety check {0:?}")]
    KnowledgeNotAdmitted(SafetyCheckKind),
    #[error("stored digest domain mismatch: expected {expected:?}, got {actual:?}")]
    StoredDigestDomainMismatch {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("failed to serialize medication safety trust artifact: {0}")]
    Serialization(String),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_medication_safety::{KnowledgeCoverage, SafetyCheckOutcome};

    fn clinical(seed: u8) -> VerifiedDigest {
        hash_canonical_bytes(DigestDomain::ClinicalArtifact, &[seed]).unwrap()
    }

    fn safety_policy() -> VerifiedDigest {
        hash_canonical_bytes(DigestDomain::MedicationSafetyPolicy, b"safety-policy").unwrap()
    }

    fn knowledge(seed: u8) -> VerifiedDigest {
        hash_canonical_bytes(DigestDomain::MedicationSafetyKnowledge, &[seed]).unwrap()
    }

    fn trust_policy(safety: VerifiedDigest) -> SafetySourceTrustPolicyV1 {
        SafetySourceTrustPolicyV1 {
            schema_version: 1,
            policy_id: "deployment-medication-safety-trust-v1".into(),
            safety_policy_digest: safety.stored(),
            max_future_skew_micros: 0,
        }
    }

    fn check(
        kind: SafetyCheckKind,
        safety: VerifiedDigest,
        evaluator: VerifiedDigest,
        kb: VerifiedDigest,
        seed: u8,
    ) -> VerifiedSafetyCheck {
        VerifiedSafetyCheck::from_verified_evaluation(
            kind,
            hash_canonical_bytes(DigestDomain::MedicationRequestArtifact, b"medication").unwrap(),
            hash_canonical_bytes(DigestDomain::MedicationSafetyContext, b"context").unwrap(),
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
    fn admitted_evaluator_and_knowledge_produce_receipt() {
        let safety = safety_policy();
        let policy = trust_policy(safety);
        let trust_digest = policy.verified_digest().unwrap();
        let evaluator = clinical(1);
        let kb = knowledge(2);
        let check = check(SafetyCheckKind::DrugAllergy, safety, evaluator, kb, 9);
        let evaluators = vec![VerifiedEvaluatorAdmission::from_verified_record(
            evaluator,
            vec![SafetyCheckKind::DrugAllergy],
            0,
            Some(1_000),
            None,
            trust_digest,
            clinical(3),
        )
        .unwrap()];
        let knowledge = vec![VerifiedKnowledgeAdmission::from_verified_record(
            kb,
            vec![SafetyCheckKind::DrugAllergy],
            0,
            Some(1_000),
            None,
            trust_digest,
            clinical(4),
        )
        .unwrap()];

        let receipt = evaluate_safety_source_trust(
            safety,
            &policy,
            &[check],
            &evaluators,
            &knowledge,
            150,
        )
        .unwrap();
        assert_eq!(receipt.safety_policy_digest(), safety.stored());
        assert_eq!(receipt.trust_policy_digest(), trust_digest.stored());
        assert_eq!(receipt.check_evaluation_digests().len(), 1);
        assert_eq!(receipt.receipt_digest().domain, DigestDomain::MedicationSafetyTrustReceipt);
    }

    #[test]
    fn unknown_evaluator_is_rejected() {
        let safety = safety_policy();
        let policy = trust_policy(safety);
        let trust_digest = policy.verified_digest().unwrap();
        let check = check(
            SafetyCheckKind::DrugAllergy,
            safety,
            clinical(9),
            knowledge(2),
            9,
        );
        let evaluators = vec![VerifiedEvaluatorAdmission::from_verified_record(
            clinical(1),
            vec![SafetyCheckKind::DrugAllergy],
            0,
            Some(1_000),
            None,
            trust_digest,
            clinical(3),
        )
        .unwrap()];
        let knowledge = vec![VerifiedKnowledgeAdmission::from_verified_record(
            knowledge(2),
            vec![SafetyCheckKind::DrugAllergy],
            0,
            Some(1_000),
            None,
            trust_digest,
            clinical(4),
        )
        .unwrap()];

        let error = evaluate_safety_source_trust(
            safety,
            &policy,
            &[check],
            &evaluators,
            &knowledge,
            150,
        )
        .err()
        .expect("unknown evaluator must fail");
        assert!(matches!(
            error,
            SafetySourceTrustError::EvaluatorNotAdmitted(SafetyCheckKind::DrugAllergy)
        ));
    }

    #[test]
    fn revoked_evaluator_is_rejected() {
        let safety = safety_policy();
        let policy = trust_policy(safety);
        let trust_digest = policy.verified_digest().unwrap();
        let evaluator = clinical(1);
        let kb = knowledge(2);
        let check = check(SafetyCheckKind::DrugAllergy, safety, evaluator, kb, 9);
        let evaluators = vec![VerifiedEvaluatorAdmission::from_verified_record(
            evaluator,
            vec![SafetyCheckKind::DrugAllergy],
            0,
            Some(1_000),
            Some(100),
            trust_digest,
            clinical(3),
        )
        .unwrap()];
        let knowledge = vec![VerifiedKnowledgeAdmission::from_verified_record(
            kb,
            vec![SafetyCheckKind::DrugAllergy],
            0,
            Some(1_000),
            None,
            trust_digest,
            clinical(4),
        )
        .unwrap()];

        let error = evaluate_safety_source_trust(
            safety,
            &policy,
            &[check],
            &evaluators,
            &knowledge,
            150,
        )
        .err()
        .expect("revoked evaluator must fail");
        assert!(matches!(
            error,
            SafetySourceTrustError::EvaluatorNotAdmitted(SafetyCheckKind::DrugAllergy)
        ));
    }

    #[test]
    fn check_kind_cannot_escape_admission_scope() {
        let safety = safety_policy();
        let policy = trust_policy(safety);
        let trust_digest = policy.verified_digest().unwrap();
        let evaluator = clinical(1);
        let kb = knowledge(2);
        let check = check(SafetyCheckKind::DrugAllergy, safety, evaluator, kb, 9);
        let evaluators = vec![VerifiedEvaluatorAdmission::from_verified_record(
            evaluator,
            vec![SafetyCheckKind::DrugDrugInteraction],
            0,
            Some(1_000),
            None,
            trust_digest,
            clinical(3),
        )
        .unwrap()];
        let knowledge = vec![VerifiedKnowledgeAdmission::from_verified_record(
            kb,
            vec![SafetyCheckKind::DrugAllergy],
            0,
            Some(1_000),
            None,
            trust_digest,
            clinical(4),
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

    #[test]
    fn trust_policy_cannot_be_replayed_across_safety_policy() {
        let safety = safety_policy();
        let policy = trust_policy(safety);
        let other_safety =
            hash_canonical_bytes(DigestDomain::MedicationSafetyPolicy, b"other-policy").unwrap();
        assert!(matches!(
            evaluate_safety_source_trust(other_safety, &policy, &[], &[], &[], 100),
            Err(SafetySourceTrustError::SafetyPolicyMismatch)
        ));
    }
}
