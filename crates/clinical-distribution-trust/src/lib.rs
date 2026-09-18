#![deny(unsafe_code)]
//! Deployment-scoped trust admission for clinical distribution/OOD evidence.
//!
//! `mycelix-clinical-distribution-assessment` answers whether one serialized
//! distribution assessment is structurally coherent and exactly bound to a
//! model/capsule/subject/policy. This crate answers the separate trust question:
//! was the detector/evaluator admitted by deployment policy at the relevant
//! times?
//!
//! This is still a pure Rust trust-composition layer. The adapter constructing a
//! `VerifiedDistributionEvaluatorAdmission` must independently verify the
//! configuration, signature, registry record, DHT record, or equivalent evidence
//! represented by the admission digest. Production use still requires a
//! separately qualified runtime/configuration trust root.

use mycelix_clinical_distribution_assessment::{
    ClinicalDistributionPolicyV1, ClinicalDistributionStatusV1,
    ValidatedClinicalDistributionAssessment,
};
use mycelix_clinical_evidence::{ArtifactIdentity, ContentDigest};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CLINICAL_DISTRIBUTION_TRUST_POLICY_VERSION: u16 = 1;
const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000;
const DERIVE_KEY_CONTEXT: &str = "mycelix.health.clinical-distribution-trust.v1";
const TRUST_POLICY_DOMAIN: &[u8] = b"clinical-distribution-trust-policy";
const TRUST_RECEIPT_DOMAIN: &[u8] = b"clinical-distribution-trust-receipt";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DistributionEvaluatorTrustPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    /// Exact structural distribution policy this trust policy may qualify.
    pub distribution_policy_digest: ContentDigest,
    /// Exact detector/evaluator artifact admitted by this policy.
    pub required_detector: ArtifactIdentity,
    pub max_future_skew_micros: i64,
}

impl DistributionEvaluatorTrustPolicyV1 {
    pub fn validate(&self) -> Result<(), ClinicalDistributionTrustError> {
        if self.schema_version != CLINICAL_DISTRIBUTION_TRUST_POLICY_VERSION {
            return Err(ClinicalDistributionTrustError::UnsupportedTrustPolicyVersion {
                found: self.schema_version,
                expected: CLINICAL_DISTRIBUTION_TRUST_POLICY_VERSION,
            });
        }
        if self.policy_id.trim().is_empty() {
            return Err(ClinicalDistributionTrustError::MissingTrustPolicyId);
        }
        validate_digest(&self.distribution_policy_digest)?;
        validate_artifact(&self.required_detector)?;
        if self.max_future_skew_micros < 0
            || self.max_future_skew_micros > MAX_FUTURE_SKEW_MICROS
        {
            return Err(ClinicalDistributionTrustError::InvalidFutureSkew);
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<ContentDigest, ClinicalDistributionTrustError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|_| ClinicalDistributionTrustError::SerializationFailure)?;
        Ok(domain_digest(TRUST_POLICY_DOMAIN, &bytes))
    }
}

/// Non-serializable evidence that deployment policy admitted one exact
/// distribution detector/evaluator artifact.
///
/// The adapter constructing this value is responsible for independently
/// verifying the external admission record represented by
/// `admission_evidence_digest`. Calling this constructor is not itself proof that
/// the external record is authentic.
pub struct VerifiedDistributionEvaluatorAdmission {
    detector: ArtifactIdentity,
    trust_policy_digest: ContentDigest,
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    revoked_at_micros: Option<i64>,
    admission_evidence_digest: ContentDigest,
}

impl VerifiedDistributionEvaluatorAdmission {
    pub fn from_verified_record(
        detector: ArtifactIdentity,
        trust_policy_digest: ContentDigest,
        valid_from_micros: i64,
        valid_until_micros: Option<i64>,
        revoked_at_micros: Option<i64>,
        admission_evidence_digest: ContentDigest,
    ) -> Result<Self, ClinicalDistributionTrustError> {
        validate_artifact(&detector)?;
        validate_digest(&trust_policy_digest)?;
        validate_digest(&admission_evidence_digest)?;
        validate_window(valid_from_micros, valid_until_micros, revoked_at_micros)?;
        Ok(Self {
            detector,
            trust_policy_digest,
            valid_from_micros,
            valid_until_micros,
            revoked_at_micros,
            admission_evidence_digest,
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
}

#[derive(Serialize)]
struct TrustReceiptDigestMaterial<'a> {
    assessment_digest: &'a ContentDigest,
    distribution_policy_digest: &'a ContentDigest,
    trust_policy_digest: &'a ContentDigest,
    evaluator_admission_evidence_digest: &'a ContentDigest,
    status: ClinicalDistributionStatusV1,
    assessed_at_micros: i64,
    trusted_at_micros: i64,
}

/// Non-cloneable and non-serializable proof that one exact structural
/// distribution assessment was qualified under one active evaluator admission.
pub struct DistributionTrustReceipt {
    assessment_digest: ContentDigest,
    distribution_policy_digest: ContentDigest,
    trust_policy_digest: ContentDigest,
    evaluator_admission_evidence_digest: ContentDigest,
    status: ClinicalDistributionStatusV1,
    assessed_at_micros: i64,
    trusted_at_micros: i64,
    receipt_digest: ContentDigest,
}

impl DistributionTrustReceipt {
    pub fn assessment_digest(&self) -> &ContentDigest {
        &self.assessment_digest
    }

    pub fn distribution_policy_digest(&self) -> &ContentDigest {
        &self.distribution_policy_digest
    }

    pub fn trust_policy_digest(&self) -> &ContentDigest {
        &self.trust_policy_digest
    }

    pub fn evaluator_admission_evidence_digest(&self) -> &ContentDigest {
        &self.evaluator_admission_evidence_digest
    }

    pub fn status(&self) -> ClinicalDistributionStatusV1 {
        self.status
    }

    pub fn assessed_at_micros(&self) -> i64 {
        self.assessed_at_micros
    }

    pub fn trusted_at_micros(&self) -> i64 {
        self.trusted_at_micros
    }

    pub fn receipt_digest(&self) -> &ContentDigest {
        &self.receipt_digest
    }
}

pub fn evaluate_distribution_trust(
    structural: &ValidatedClinicalDistributionAssessment,
    structural_policy: &ClinicalDistributionPolicyV1,
    trust_policy: &DistributionEvaluatorTrustPolicyV1,
    admission: &VerifiedDistributionEvaluatorAdmission,
    trusted_at_micros: i64,
) -> Result<DistributionTrustReceipt, ClinicalDistributionTrustError> {
    structural_policy
        .validate()
        .map_err(|_| ClinicalDistributionTrustError::InvalidStructuralPolicy)?;
    trust_policy.validate()?;
    if trusted_at_micros <= 0 {
        return Err(ClinicalDistributionTrustError::InvalidTrustEvaluationTime);
    }

    let structural_policy_digest = structural_policy
        .digest()
        .map_err(|_| ClinicalDistributionTrustError::InvalidStructuralPolicy)?;
    if structural.policy_digest() != &structural_policy_digest {
        return Err(ClinicalDistributionTrustError::StructuralPolicyMismatch);
    }
    if trust_policy.distribution_policy_digest != structural_policy_digest {
        return Err(ClinicalDistributionTrustError::TrustPolicyStructuralPolicyMismatch);
    }
    if trust_policy.required_detector != structural_policy.required_detector {
        return Err(ClinicalDistributionTrustError::TrustPolicyDetectorMismatch);
    }

    let trust_policy_digest = trust_policy.digest()?;
    if admission.trust_policy_digest != trust_policy_digest {
        return Err(ClinicalDistributionTrustError::AdmissionTrustPolicyMismatch);
    }
    if admission.detector != trust_policy.required_detector {
        return Err(ClinicalDistributionTrustError::AdmissionDetectorMismatch);
    }

    if !admission.active_at(structural.assessed_at_micros()) {
        return Err(ClinicalDistributionTrustError::AdmissionInactiveAtAssessment);
    }
    if !admission.active_at(trusted_at_micros) {
        return Err(ClinicalDistributionTrustError::AdmissionInactiveAtTrustEvaluation);
    }

    if structural.assessed_at_micros() > trusted_at_micros {
        let skew = structural
            .assessed_at_micros()
            .saturating_sub(trusted_at_micros);
        if skew > trust_policy.max_future_skew_micros {
            return Err(ClinicalDistributionTrustError::AssessmentTooFarInFuture);
        }
    }

    let material = TrustReceiptDigestMaterial {
        assessment_digest: structural.assessment_digest(),
        distribution_policy_digest: &structural_policy_digest,
        trust_policy_digest: &trust_policy_digest,
        evaluator_admission_evidence_digest: &admission.admission_evidence_digest,
        status: structural.status(),
        assessed_at_micros: structural.assessed_at_micros(),
        trusted_at_micros,
    };
    let bytes = serde_json::to_vec(&material)
        .map_err(|_| ClinicalDistributionTrustError::SerializationFailure)?;
    let receipt_digest = domain_digest(TRUST_RECEIPT_DOMAIN, &bytes);

    Ok(DistributionTrustReceipt {
        assessment_digest: structural.assessment_digest().clone(),
        distribution_policy_digest: structural_policy_digest,
        trust_policy_digest,
        evaluator_admission_evidence_digest: admission.admission_evidence_digest.clone(),
        status: structural.status(),
        assessed_at_micros: structural.assessed_at_micros(),
        trusted_at_micros,
        receipt_digest,
    })
}

fn validate_artifact(artifact: &ArtifactIdentity) -> Result<(), ClinicalDistributionTrustError> {
    if artifact.name.trim().is_empty() || artifact.version.trim().is_empty() {
        return Err(ClinicalDistributionTrustError::IncompleteArtifactIdentity);
    }
    validate_digest(&artifact.digest)
}

fn validate_digest(digest: &ContentDigest) -> Result<(), ClinicalDistributionTrustError> {
    if digest.algorithm.trim().is_empty() || digest.value.trim().is_empty() {
        return Err(ClinicalDistributionTrustError::InvalidDigest);
    }
    Ok(())
}

fn validate_window(
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    revoked_at_micros: Option<i64>,
) -> Result<(), ClinicalDistributionTrustError> {
    if valid_from_micros <= 0 {
        return Err(ClinicalDistributionTrustError::InvalidAdmissionWindow);
    }
    if let Some(until) = valid_until_micros {
        if until <= valid_from_micros {
            return Err(ClinicalDistributionTrustError::InvalidAdmissionWindow);
        }
    }
    if let Some(revoked) = revoked_at_micros {
        if revoked <= valid_from_micros {
            return Err(ClinicalDistributionTrustError::InvalidAdmissionWindow);
        }
    }
    Ok(())
}

fn domain_digest(domain: &[u8], payload: &[u8]) -> ContentDigest {
    let mut hasher = blake3::Hasher::new_derive_key(DERIVE_KEY_CONTEXT);
    hasher.update(&(domain.len() as u16).to_be_bytes());
    hasher.update(domain);
    hasher.update(&(payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    ContentDigest {
        algorithm: "blake3-256".into(),
        value: hasher.finalize().to_hex().to_string(),
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClinicalDistributionTrustError {
    #[error("unsupported distribution trust policy version {found}; expected {expected}")]
    UnsupportedTrustPolicyVersion { found: u16, expected: u16 },
    #[error("distribution trust policy id is required")]
    MissingTrustPolicyId,
    #[error("artifact identity is incomplete")]
    IncompleteArtifactIdentity,
    #[error("content digest is invalid")]
    InvalidDigest,
    #[error("future skew bound is invalid")]
    InvalidFutureSkew,
    #[error("admission validity/revocation window is invalid")]
    InvalidAdmissionWindow,
    #[error("trust evaluation time is invalid")]
    InvalidTrustEvaluationTime,
    #[error("structural distribution policy is invalid")]
    InvalidStructuralPolicy,
    #[error("structural preflight was produced under another distribution policy")]
    StructuralPolicyMismatch,
    #[error("trust policy does not bind this structural distribution policy")]
    TrustPolicyStructuralPolicyMismatch,
    #[error("trust policy detector does not match the structural distribution policy")]
    TrustPolicyDetectorMismatch,
    #[error("evaluator admission belongs to another trust policy")]
    AdmissionTrustPolicyMismatch,
    #[error("evaluator admission is for another detector")]
    AdmissionDetectorMismatch,
    #[error("evaluator admission was not active when the assessment was produced")]
    AdmissionInactiveAtAssessment,
    #[error("evaluator admission is not active at trust evaluation time")]
    AdmissionInactiveAtTrustEvaluation,
    #[error("assessment is too far in the future relative to trust evaluation")]
    AssessmentTooFarInFuture,
    #[error("trust receipt serialization failed")]
    SerializationFailure,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_distribution_assessment::{
        capsule_binding_digest, validate_distribution_for_capsule,
        ClinicalDistributionAssessmentV1, NumericDistributionBoundaryV1,
        NumericDistributionDirectionV1, CLINICAL_DISTRIBUTION_ASSESSMENT_VERSION,
        CLINICAL_DISTRIBUTION_POLICY_VERSION,
    };
    use mycelix_clinical_evidence::{
        ArtifactIdentity, AssertionKind, AssertionUncertainty, AuthorityMode, ClinicalAuthority,
        ClinicalEvidenceCapsule, ContentDigest, EvidenceRole, ExecutionIdentity, FactEvidence,
        HumanReview, IntendedUse, QualificationLevel, ReviewStatus,
    };
    use mycelix_clinical_semantics::{EvaluationState, SubjectRef};

    fn digest(value: &str) -> ContentDigest {
        ContentDigest {
            algorithm: "sha256".into(),
            value: value.into(),
        }
    }

    fn artifact(name: &str, version: &str, value: &str) -> ArtifactIdentity {
        ArtifactIdentity {
            name: name.into(),
            version: version.into(),
            digest: digest(value),
        }
    }

    fn capsule() -> ClinicalEvidenceCapsule {
        ClinicalEvidenceCapsule {
            capsule_id: "capsule-ai-1".into(),
            subject: SubjectRef {
                resource_type: "Patient".into(),
                id: "patient-a".into(),
            },
            kind: AssertionKind::RiskPrediction,
            statement: "Risk signal".into(),
            code: None,
            state: EvaluationState::Satisfied,
            reasons: vec!["model output".into()],
            fact_evidence: vec![FactEvidence {
                fact_id: "fact-1".into(),
                role: EvidenceRole::Supports,
                fact_digest: Some(digest("fact")),
            }],
            sources: vec![],
            missing_requirements: vec![],
            alternatives: vec![],
            uncertainty: Some(AssertionUncertainty {
                confidence: Some(0.7),
                interpretation: Some("calibrated".into()),
                calibration_reference: Some(digest("calibration")),
            }),
            execution: ExecutionIdentity {
                engine: artifact("symthaea", "1.0.0", "engine"),
                model: Some(artifact("risk-model", "1.0.0", "model")),
                knowledge_artifact: None,
                environment_digest: Some(digest("env")),
                operation: "risk".into(),
            },
            intended_use: IntendedUse {
                use_case: "decision support".into(),
                intended_user: "licensed clinician".into(),
                population: "adults".into(),
                care_setting: "outpatient".into(),
                qualification: QualificationLevel::SupervisedClinical,
            },
            authority: ClinicalAuthority {
                mode: AuthorityMode::ClinicianReviewRequired,
                review: HumanReview {
                    status: ReviewStatus::Approved,
                    reviewer: Some("Practitioner/1".into()),
                    reviewed_at_micros: Some(150),
                    notes: None,
                },
            },
            issued_at_micros: 100,
            supersedes_capsule_id: None,
        }
    }

    fn structural_policy() -> ClinicalDistributionPolicyV1 {
        ClinicalDistributionPolicyV1 {
            schema_version: CLINICAL_DISTRIBUTION_POLICY_VERSION,
            policy_id: "distribution-policy-v1".into(),
            required_detector: artifact("ood-detector", "1.0.0", "detector"),
            required_reference_population: "adult-outpatient-v1".into(),
            required_reference_domain_digest: digest("reference-domain"),
            max_age_micros: 10_000,
            max_future_skew_micros: 10,
        }
    }

    fn structural_preflight() -> (
        ValidatedClinicalDistributionAssessment,
        ClinicalDistributionPolicyV1,
    ) {
        let capsule = capsule();
        let policy = structural_policy();
        let assessment = ClinicalDistributionAssessmentV1 {
            schema_version: CLINICAL_DISTRIBUTION_ASSESSMENT_VERSION,
            assessment_id: "assessment-1".into(),
            subject: capsule.subject.clone(),
            model: capsule.execution.model.clone().unwrap(),
            detector: policy.required_detector.clone(),
            reference_population: policy.required_reference_population.clone(),
            reference_domain_digest: policy.required_reference_domain_digest.clone(),
            capsule_binding_digest: capsule_binding_digest(&capsule).unwrap(),
            status: ClinicalDistributionStatusV1::InDistribution,
            numeric_boundary: Some(NumericDistributionBoundaryV1 {
                score: 0.8,
                threshold: 0.5,
                direction: NumericDistributionDirectionV1::HigherMeansMoreInDistribution,
            }),
            assessment_evidence_digest: Some(digest("distribution-evidence")),
            assessed_at_micros: 200,
        };
        let structural =
            validate_distribution_for_capsule(&capsule, &assessment, &policy, 250).unwrap();
        (structural, policy)
    }

    fn trust_policy(
        structural_policy: &ClinicalDistributionPolicyV1,
    ) -> DistributionEvaluatorTrustPolicyV1 {
        DistributionEvaluatorTrustPolicyV1 {
            schema_version: CLINICAL_DISTRIBUTION_TRUST_POLICY_VERSION,
            policy_id: "distribution-trust-v1".into(),
            distribution_policy_digest: structural_policy.digest().unwrap(),
            required_detector: structural_policy.required_detector.clone(),
            max_future_skew_micros: 10,
        }
    }

    fn admission(
        trust_policy: &DistributionEvaluatorTrustPolicyV1,
    ) -> VerifiedDistributionEvaluatorAdmission {
        VerifiedDistributionEvaluatorAdmission::from_verified_record(
            trust_policy.required_detector.clone(),
            trust_policy.digest().unwrap(),
            100,
            Some(1_000),
            None,
            digest("admission-evidence"),
        )
        .unwrap()
    }

    #[test]
    fn exact_active_evaluator_admission_yields_nonserializable_trust_receipt() {
        let (structural, structural_policy) = structural_preflight();
        let trust_policy = trust_policy(&structural_policy);
        let admission = admission(&trust_policy);
        let receipt = evaluate_distribution_trust(
            &structural,
            &structural_policy,
            &trust_policy,
            &admission,
            300,
        )
        .unwrap();
        assert_eq!(receipt.status(), ClinicalDistributionStatusV1::InDistribution);
        assert_eq!(receipt.assessment_digest(), structural.assessment_digest());
    }

    #[test]
    fn wrong_trust_policy_cannot_reuse_admission() {
        let (structural, structural_policy) = structural_preflight();
        let trust_policy = trust_policy(&structural_policy);
        let admission = admission(&trust_policy);
        let mut changed = trust_policy.clone();
        changed.policy_id = "other-policy".into();
        assert_eq!(
            evaluate_distribution_trust(
                &structural,
                &structural_policy,
                &changed,
                &admission,
                300,
            ),
            Err(ClinicalDistributionTrustError::AdmissionTrustPolicyMismatch)
        );
    }

    #[test]
    fn detector_substitution_in_trust_policy_fails() {
        let (structural, structural_policy) = structural_preflight();
        let mut trust_policy = trust_policy(&structural_policy);
        trust_policy.required_detector = artifact("other-detector", "1.0.0", "other");
        let admission = VerifiedDistributionEvaluatorAdmission::from_verified_record(
            trust_policy.required_detector.clone(),
            trust_policy.digest().unwrap(),
            100,
            Some(1_000),
            None,
            digest("admission-evidence"),
        )
        .unwrap();
        assert_eq!(
            evaluate_distribution_trust(
                &structural,
                &structural_policy,
                &trust_policy,
                &admission,
                300,
            ),
            Err(ClinicalDistributionTrustError::TrustPolicyDetectorMismatch)
        );
    }

    #[test]
    fn revoked_admission_is_not_active_at_trust_evaluation() {
        let (structural, structural_policy) = structural_preflight();
        let trust_policy = trust_policy(&structural_policy);
        let admission = VerifiedDistributionEvaluatorAdmission::from_verified_record(
            trust_policy.required_detector.clone(),
            trust_policy.digest().unwrap(),
            100,
            Some(1_000),
            Some(250),
            digest("admission-evidence"),
        )
        .unwrap();
        assert_eq!(
            evaluate_distribution_trust(
                &structural,
                &structural_policy,
                &trust_policy,
                &admission,
                300,
            ),
            Err(ClinicalDistributionTrustError::AdmissionInactiveAtTrustEvaluation)
        );
    }
}
