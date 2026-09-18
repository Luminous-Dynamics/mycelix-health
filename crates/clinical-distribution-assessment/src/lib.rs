#![deny(unsafe_code)]
//! Evidence-bound distribution-shift / out-of-distribution assessment.
//!
//! This crate deliberately keeps distribution evidence separate from the
//! clinical assertion capsule. A model-backed clinical assertion can therefore
//! be preserved as evidence even when its distribution check is missing,
//! unavailable, indeterminate, stale, or explicitly out-of-distribution.
//! Only a separately policy-qualified assessment can satisfy the downstream
//! clinician-presentation gate.

use mycelix_clinical_evidence::{ArtifactIdentity, ClinicalEvidenceCapsule, ContentDigest};
use mycelix_clinical_semantics::SubjectRef;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CLINICAL_DISTRIBUTION_ASSESSMENT_VERSION: u16 = 1;
pub const CLINICAL_DISTRIBUTION_POLICY_VERSION: u16 = 1;

const DERIVE_KEY_CONTEXT: &str = "mycelix.health.clinical-distribution-assessment.v1";
const CAPSULE_BINDING_DOMAIN: &[u8] = b"clinical-capsule-binding";
const ASSESSMENT_DOMAIN: &[u8] = b"clinical-distribution-assessment";
const POLICY_DOMAIN: &[u8] = b"clinical-distribution-policy";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClinicalDistributionStatusV1 {
    NotRun,
    Unavailable,
    Indeterminate,
    InDistribution,
    OutOfDistribution,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NumericDistributionDirectionV1 {
    HigherMeansMoreInDistribution,
    LowerMeansMoreInDistribution,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NumericDistributionBoundaryV1 {
    pub score: f64,
    pub threshold: f64,
    pub direction: NumericDistributionDirectionV1,
}

impl NumericDistributionBoundaryV1 {
    fn validate(&self) -> Result<(), ClinicalDistributionAssessmentError> {
        if !self.score.is_finite() || !self.threshold.is_finite() {
            return Err(ClinicalDistributionAssessmentError::InvalidNumericBoundary);
        }
        Ok(())
    }

    fn implies_in_distribution(&self) -> bool {
        match self.direction {
            NumericDistributionDirectionV1::HigherMeansMoreInDistribution => {
                self.score >= self.threshold
            }
            NumericDistributionDirectionV1::LowerMeansMoreInDistribution => {
                self.score <= self.threshold
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClinicalDistributionAssessmentV1 {
    pub schema_version: u16,
    pub assessment_id: String,
    pub subject: SubjectRef,
    pub model: ArtifactIdentity,
    pub detector: ArtifactIdentity,
    pub reference_population: String,
    pub reference_domain_digest: ContentDigest,
    /// Digest of the exact clinical evidence capsule this assessment belongs to.
    pub capsule_binding_digest: ContentDigest,
    pub status: ClinicalDistributionStatusV1,
    /// Optional numeric decision boundary for detectors that expose one.
    pub numeric_boundary: Option<NumericDistributionBoundaryV1>,
    /// Exact detector/output evidence. Required for detector-produced results.
    pub assessment_evidence_digest: Option<ContentDigest>,
    pub assessed_at_micros: i64,
}

impl ClinicalDistributionAssessmentV1 {
    pub fn validate(&self) -> Result<(), ClinicalDistributionAssessmentError> {
        if self.schema_version != CLINICAL_DISTRIBUTION_ASSESSMENT_VERSION {
            return Err(ClinicalDistributionAssessmentError::UnsupportedAssessmentVersion {
                found: self.schema_version,
                expected: CLINICAL_DISTRIBUTION_ASSESSMENT_VERSION,
            });
        }
        if self.assessment_id.trim().is_empty() {
            return Err(ClinicalDistributionAssessmentError::MissingAssessmentId);
        }
        validate_subject(&self.subject)?;
        validate_artifact(&self.model)?;
        validate_artifact(&self.detector)?;
        if self.reference_population.trim().is_empty() {
            return Err(ClinicalDistributionAssessmentError::MissingReferencePopulation);
        }
        validate_digest(&self.reference_domain_digest)?;
        validate_digest(&self.capsule_binding_digest)?;
        if self.assessed_at_micros <= 0 {
            return Err(ClinicalDistributionAssessmentError::InvalidAssessmentTime);
        }
        if let Some(boundary) = &self.numeric_boundary {
            boundary.validate()?;
            if matches!(
                self.status,
                ClinicalDistributionStatusV1::InDistribution
                    | ClinicalDistributionStatusV1::OutOfDistribution
            ) {
                let implied_in = boundary.implies_in_distribution();
                if (self.status == ClinicalDistributionStatusV1::InDistribution && !implied_in)
                    || (self.status == ClinicalDistributionStatusV1::OutOfDistribution
                        && implied_in)
                {
                    return Err(ClinicalDistributionAssessmentError::NumericStatusMismatch);
                }
            }
        }
        if matches!(
            self.status,
            ClinicalDistributionStatusV1::Indeterminate
                | ClinicalDistributionStatusV1::InDistribution
                | ClinicalDistributionStatusV1::OutOfDistribution
        ) && self.assessment_evidence_digest.is_none()
        {
            return Err(ClinicalDistributionAssessmentError::MissingAssessmentEvidence);
        }
        if let Some(digest) = &self.assessment_evidence_digest {
            validate_digest(digest)?;
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<ContentDigest, ClinicalDistributionAssessmentError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|_| ClinicalDistributionAssessmentError::SerializationFailure)?;
        Ok(domain_digest(ASSESSMENT_DOMAIN, &bytes))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClinicalDistributionPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub required_detector: ArtifactIdentity,
    pub required_reference_population: String,
    pub required_reference_domain_digest: ContentDigest,
    pub max_age_micros: i64,
    pub max_future_skew_micros: i64,
}

impl ClinicalDistributionPolicyV1 {
    pub fn validate(&self) -> Result<(), ClinicalDistributionAssessmentError> {
        if self.schema_version != CLINICAL_DISTRIBUTION_POLICY_VERSION {
            return Err(ClinicalDistributionAssessmentError::UnsupportedPolicyVersion {
                found: self.schema_version,
                expected: CLINICAL_DISTRIBUTION_POLICY_VERSION,
            });
        }
        if self.policy_id.trim().is_empty() {
            return Err(ClinicalDistributionAssessmentError::MissingPolicyId);
        }
        validate_artifact(&self.required_detector)?;
        if self.required_reference_population.trim().is_empty() {
            return Err(ClinicalDistributionAssessmentError::MissingReferencePopulation);
        }
        validate_digest(&self.required_reference_domain_digest)?;
        if self.max_age_micros <= 0 || self.max_future_skew_micros < 0 {
            return Err(ClinicalDistributionAssessmentError::InvalidFreshnessPolicy);
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<ContentDigest, ClinicalDistributionAssessmentError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|_| ClinicalDistributionAssessmentError::SerializationFailure)?;
        Ok(domain_digest(POLICY_DOMAIN, &bytes))
    }
}

/// Non-serializable proof that the assessment was bound to the exact capsule,
/// subject, model, detector, reference domain, policy, and freshness window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedClinicalDistributionAssessment {
    status: ClinicalDistributionStatusV1,
    assessment_digest: ContentDigest,
    policy_digest: ContentDigest,
    assessed_at_micros: i64,
}

impl ValidatedClinicalDistributionAssessment {
    pub fn status(&self) -> ClinicalDistributionStatusV1 {
        self.status
    }

    pub fn assessment_digest(&self) -> &ContentDigest {
        &self.assessment_digest
    }

    pub fn policy_digest(&self) -> &ContentDigest {
        &self.policy_digest
    }

    pub fn assessed_at_micros(&self) -> i64 {
        self.assessed_at_micros
    }
}

pub fn capsule_binding_digest(
    capsule: &ClinicalEvidenceCapsule,
) -> Result<ContentDigest, ClinicalDistributionAssessmentError> {
    let bytes = serde_json::to_vec(capsule)
        .map_err(|_| ClinicalDistributionAssessmentError::SerializationFailure)?;
    Ok(domain_digest(CAPSULE_BINDING_DOMAIN, &bytes))
}

pub fn validate_distribution_for_capsule(
    capsule: &ClinicalEvidenceCapsule,
    assessment: &ClinicalDistributionAssessmentV1,
    policy: &ClinicalDistributionPolicyV1,
    now_micros: i64,
) -> Result<ValidatedClinicalDistributionAssessment, ClinicalDistributionAssessmentError> {
    assessment.validate()?;
    policy.validate()?;
    if now_micros <= 0 {
        return Err(ClinicalDistributionAssessmentError::InvalidCurrentTime);
    }

    let model = capsule
        .execution
        .model
        .as_ref()
        .ok_or(ClinicalDistributionAssessmentError::ModelIdentityRequired)?;

    if &assessment.model != model {
        return Err(ClinicalDistributionAssessmentError::ModelMismatch);
    }
    if assessment.subject != capsule.subject {
        return Err(ClinicalDistributionAssessmentError::SubjectMismatch);
    }

    let expected_capsule_binding = capsule_binding_digest(capsule)?;
    if assessment.capsule_binding_digest != expected_capsule_binding {
        return Err(ClinicalDistributionAssessmentError::CapsuleBindingMismatch);
    }
    if assessment.detector != policy.required_detector {
        return Err(ClinicalDistributionAssessmentError::DetectorMismatch);
    }
    if assessment.reference_population != policy.required_reference_population {
        return Err(ClinicalDistributionAssessmentError::ReferencePopulationMismatch);
    }
    if assessment.reference_domain_digest != policy.required_reference_domain_digest {
        return Err(ClinicalDistributionAssessmentError::ReferenceDomainMismatch);
    }

    if assessment.assessed_at_micros > now_micros {
        let skew = assessment.assessed_at_micros.saturating_sub(now_micros);
        if skew > policy.max_future_skew_micros {
            return Err(ClinicalDistributionAssessmentError::AssessmentFromFuture);
        }
    } else {
        let age = now_micros.saturating_sub(assessment.assessed_at_micros);
        if age > policy.max_age_micros {
            return Err(ClinicalDistributionAssessmentError::StaleAssessment);
        }
    }

    Ok(ValidatedClinicalDistributionAssessment {
        status: assessment.status,
        assessment_digest: assessment.digest()?,
        policy_digest: policy.digest()?,
        assessed_at_micros: assessment.assessed_at_micros,
    })
}

fn validate_subject(subject: &SubjectRef) -> Result<(), ClinicalDistributionAssessmentError> {
    if subject.resource_type.trim().is_empty() || subject.id.trim().is_empty() {
        return Err(ClinicalDistributionAssessmentError::InvalidSubject);
    }
    Ok(())
}

fn validate_artifact(
    artifact: &ArtifactIdentity,
) -> Result<(), ClinicalDistributionAssessmentError> {
    if artifact.name.trim().is_empty() || artifact.version.trim().is_empty() {
        return Err(ClinicalDistributionAssessmentError::IncompleteArtifactIdentity);
    }
    validate_digest(&artifact.digest)
}

fn validate_digest(digest: &ContentDigest) -> Result<(), ClinicalDistributionAssessmentError> {
    if digest.algorithm.trim().is_empty() || digest.value.trim().is_empty() {
        return Err(ClinicalDistributionAssessmentError::InvalidDigest);
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
pub enum ClinicalDistributionAssessmentError {
    #[error("unsupported clinical distribution assessment schema version {found}; expected {expected}")]
    UnsupportedAssessmentVersion { found: u16, expected: u16 },
    #[error("unsupported clinical distribution policy schema version {found}; expected {expected}")]
    UnsupportedPolicyVersion { found: u16, expected: u16 },
    #[error("distribution assessment id is required")]
    MissingAssessmentId,
    #[error("distribution policy id is required")]
    MissingPolicyId,
    #[error("clinical subject binding is invalid")]
    InvalidSubject,
    #[error("artifact identity is incomplete")]
    IncompleteArtifactIdentity,
    #[error("content digest is invalid")]
    InvalidDigest,
    #[error("reference population is required")]
    MissingReferencePopulation,
    #[error("distribution assessment time is invalid")]
    InvalidAssessmentTime,
    #[error("current qualification time is invalid")]
    InvalidCurrentTime,
    #[error("numeric distribution boundary must contain finite values")]
    InvalidNumericBoundary,
    #[error("numeric distribution boundary contradicts the declared status")]
    NumericStatusMismatch,
    #[error("detector-produced distribution status requires evidence")]
    MissingAssessmentEvidence,
    #[error("clinical evidence capsule serialization failed")]
    SerializationFailure,
    #[error("model identity is required for distribution-qualified model promotion")]
    ModelIdentityRequired,
    #[error("distribution assessment model does not match capsule model")]
    ModelMismatch,
    #[error("distribution assessment subject does not match capsule subject")]
    SubjectMismatch,
    #[error("distribution assessment is not bound to this exact evidence capsule")]
    CapsuleBindingMismatch,
    #[error("distribution detector does not match deployment policy")]
    DetectorMismatch,
    #[error("distribution reference population does not match deployment policy")]
    ReferencePopulationMismatch,
    #[error("distribution reference domain does not match deployment policy")]
    ReferenceDomainMismatch,
    #[error("distribution assessment freshness policy is invalid")]
    InvalidFreshnessPolicy,
    #[error("distribution assessment is older than the allowed freshness window")]
    StaleAssessment,
    #[error("distribution assessment is too far in the future")]
    AssessmentFromFuture,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_evidence::{
        AssertionKind, AssertionUncertainty, AuthorityMode, ClinicalAuthority, EvidenceRole,
        ExecutionIdentity, FactEvidence, HumanReview, IntendedUse, QualificationLevel,
        ReviewStatus,
    };
    use mycelix_clinical_semantics::EvaluationState;

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
            statement: "Elevated short-term risk signal".into(),
            code: None,
            state: EvaluationState::Satisfied,
            reasons: vec!["model output".into()],
            fact_evidence: vec![FactEvidence {
                fact_id: "fact-1".into(),
                role: EvidenceRole::Supports,
                fact_digest: Some(digest("fact-1")),
            }],
            sources: vec![],
            missing_requirements: vec![],
            alternatives: vec![],
            uncertainty: Some(AssertionUncertainty {
                confidence: Some(0.72),
                interpretation: Some("calibrated probability".into()),
                calibration_reference: Some(digest("calibration")),
            }),
            execution: ExecutionIdentity {
                engine: artifact("symthaea", "1.0.0", "engine"),
                model: Some(artifact("risk-model", "1.0.0", "model")),
                knowledge_artifact: None,
                environment_digest: Some(digest("env")),
                operation: "risk_prediction".into(),
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
                    reviewed_at_micros: Some(20),
                    notes: None,
                },
            },
            issued_at_micros: 10,
            supersedes_capsule_id: None,
        }
    }

    fn policy() -> ClinicalDistributionPolicyV1 {
        ClinicalDistributionPolicyV1 {
            schema_version: CLINICAL_DISTRIBUTION_POLICY_VERSION,
            policy_id: "ood-policy-v1".into(),
            required_detector: artifact("mahalanobis-ood", "1.0.0", "detector"),
            required_reference_population: "adult-outpatient-v1".into(),
            required_reference_domain_digest: digest("reference-domain"),
            max_age_micros: 1_000,
            max_future_skew_micros: 10,
        }
    }

    fn assessment(capsule: &ClinicalEvidenceCapsule) -> ClinicalDistributionAssessmentV1 {
        ClinicalDistributionAssessmentV1 {
            schema_version: CLINICAL_DISTRIBUTION_ASSESSMENT_VERSION,
            assessment_id: "assessment-1".into(),
            subject: capsule.subject.clone(),
            model: capsule.execution.model.clone().unwrap(),
            detector: policy().required_detector,
            reference_population: "adult-outpatient-v1".into(),
            reference_domain_digest: digest("reference-domain"),
            capsule_binding_digest: capsule_binding_digest(capsule).unwrap(),
            status: ClinicalDistributionStatusV1::InDistribution,
            numeric_boundary: Some(NumericDistributionBoundaryV1 {
                score: 0.8,
                threshold: 0.5,
                direction: NumericDistributionDirectionV1::HigherMeansMoreInDistribution,
            }),
            assessment_evidence_digest: Some(digest("ood-evidence")),
            assessed_at_micros: 100,
        }
    }

    #[test]
    fn exact_in_distribution_assessment_qualifies() {
        let capsule = capsule();
        let validated =
            validate_distribution_for_capsule(&capsule, &assessment(&capsule), &policy(), 200)
                .unwrap();
        assert_eq!(
            validated.status(),
            ClinicalDistributionStatusV1::InDistribution
        );
    }

    #[test]
    fn model_substitution_fails_closed() {
        let capsule = capsule();
        let mut assessment = assessment(&capsule);
        assessment.model = artifact("other-model", "1.0.0", "other-model");
        assert_eq!(
            validate_distribution_for_capsule(&capsule, &assessment, &policy(), 200),
            Err(ClinicalDistributionAssessmentError::ModelMismatch)
        );
    }

    #[test]
    fn capsule_substitution_fails_closed() {
        let capsule = capsule();
        let assessment = assessment(&capsule);
        let mut changed = capsule.clone();
        changed.statement = "changed output".into();
        assert_eq!(
            validate_distribution_for_capsule(&changed, &assessment, &policy(), 200),
            Err(ClinicalDistributionAssessmentError::CapsuleBindingMismatch)
        );
    }

    #[test]
    fn detector_substitution_fails_closed() {
        let capsule = capsule();
        let mut assessment = assessment(&capsule);
        assessment.detector = artifact("other-detector", "1.0.0", "other-detector");
        assert_eq!(
            validate_distribution_for_capsule(&capsule, &assessment, &policy(), 200),
            Err(ClinicalDistributionAssessmentError::DetectorMismatch)
        );
    }

    #[test]
    fn stale_assessment_fails_closed() {
        let capsule = capsule();
        let mut stale_policy = policy();
        stale_policy.max_age_micros = 50;
        assert_eq!(
            validate_distribution_for_capsule(&capsule, &assessment(&capsule), &stale_policy, 200),
            Err(ClinicalDistributionAssessmentError::StaleAssessment)
        );
    }

    #[test]
    fn numeric_status_must_match_detector_boundary() {
        let capsule = capsule();
        let mut assessment = assessment(&capsule);
        assessment.status = ClinicalDistributionStatusV1::OutOfDistribution;
        assert_eq!(
            assessment.validate(),
            Err(ClinicalDistributionAssessmentError::NumericStatusMismatch)
        );
    }
}
