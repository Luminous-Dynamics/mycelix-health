#![deny(unsafe_code)]
//! Evidence-bearing clinical assertion capsules for Mycelix-Health.
//!
//! The purpose of this crate is not to make clinical decisions. It defines the
//! minimum evidence, provenance, uncertainty, intended-use, and human-authority
//! information that must accompany a clinical assertion before it is promoted
//! into a care, trial, or research workflow.

use health_validation::validate_confidence_score;
use mycelix_clinical_semantics::{CodeableConcept, EvaluationState, SubjectRef};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssertionKind {
    ObservationInterpretation,
    Recommendation,
    SafetyAlert,
    EligibilityAssessment,
    RiskPrediction,
    GuidelineApplicability,
    ResearchFinding,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvidenceRole {
    Supports,
    Opposes,
    Context,
    Contraindication,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FactEvidence {
    /// ID of a separately stored, validated ClinicalFact.
    pub fact_id: String,
    pub role: EvidenceRole,
    /// Optional digest of the exact serialized fact snapshot used by the
    /// computation. This prevents a later mutable view from being mistaken for
    /// the historical input.
    pub fact_digest: Option<ContentDigest>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvidenceSourceKind {
    Guideline,
    Protocol,
    SystematicReview,
    ClinicalStudy,
    KnowledgeBase,
    RegulatoryDocument,
    LocalPolicy,
    ModelCard,
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceSource {
    pub source_id: String,
    pub kind: EvidenceSourceKind,
    pub title: String,
    pub version: Option<String>,
    pub canonical_uri: Option<String>,
    /// Digest of the exact source artifact or normalized extract relied upon.
    pub content_digest: ContentDigest,
    pub retrieved_at_micros: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentDigest {
    /// Algorithm name, e.g. "sha256" or "blake3".
    pub algorithm: String,
    /// Lower/upper case is intentionally not prescribed here; adapters may
    /// normalize it. The invariant is that it is non-empty and bound to the
    /// named algorithm.
    pub value: String,
}

impl ContentDigest {
    fn validate(&self) -> Result<(), EvidenceCapsuleError> {
        if self.algorithm.trim().is_empty() || self.value.trim().is_empty() {
            return Err(EvidenceCapsuleError::InvalidDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RequirementCriticality {
    Informational,
    Important,
    Critical,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MissingRequirement {
    pub requirement_id: String,
    pub description: String,
    pub concept: Option<CodeableConcept>,
    pub criticality: RequirementCriticality,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Alternative {
    pub description: String,
    pub code: Option<CodeableConcept>,
    pub rationale: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactIdentity {
    pub name: String,
    pub version: String,
    pub digest: ContentDigest,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionIdentity {
    /// Program/rules engine that produced the assertion.
    pub engine: ArtifactIdentity,
    /// Optional model identity. None is preferred for deterministic rules.
    pub model: Option<ArtifactIdentity>,
    /// Exact computable rule/guideline/protocol artifact when applicable.
    pub knowledge_artifact: Option<ArtifactIdentity>,
    /// Optional execution environment/capsule digest.
    pub environment_digest: Option<ContentDigest>,
    /// Name of the operation, e.g. "evaluate_hba1c_trial_criterion".
    pub operation: String,
}

impl ExecutionIdentity {
    fn validate(&self) -> Result<(), EvidenceCapsuleError> {
        validate_artifact(&self.engine)?;
        if let Some(model) = &self.model {
            validate_artifact(model)?;
        }
        if let Some(knowledge_artifact) = &self.knowledge_artifact {
            validate_artifact(knowledge_artifact)?;
        }
        if let Some(environment) = &self.environment_digest {
            environment.validate()?;
        }
        if self.operation.trim().is_empty() {
            return Err(EvidenceCapsuleError::IncompleteExecutionIdentity);
        }
        Ok(())
    }
}

fn validate_artifact(artifact: &ArtifactIdentity) -> Result<(), EvidenceCapsuleError> {
    if artifact.name.trim().is_empty() || artifact.version.trim().is_empty() {
        return Err(EvidenceCapsuleError::IncompleteExecutionIdentity);
    }
    artifact.digest.validate()
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum QualificationLevel {
    Experimental,
    ValidatedOffline,
    ShadowClinical,
    SupervisedClinical,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntendedUse {
    pub use_case: String,
    pub intended_user: String,
    pub population: String,
    pub care_setting: String,
    pub qualification: QualificationLevel,
}

impl IntendedUse {
    fn validate(&self) -> Result<(), EvidenceCapsuleError> {
        if self.use_case.trim().is_empty()
            || self.intended_user.trim().is_empty()
            || self.population.trim().is_empty()
            || self.care_setting.trim().is_empty()
        {
            return Err(EvidenceCapsuleError::IncompleteIntendedUse);
        }
        Ok(())
    }
}

/// v1 intentionally has no autonomous therapeutic authority. A future mode may
/// be added only with a separate qualification and safety case.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthorityMode {
    InformationalOnly,
    Advisory,
    ClinicianReviewRequired,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReviewStatus {
    NotRequired,
    Pending,
    Approved,
    Rejected,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HumanReview {
    pub status: ReviewStatus,
    pub reviewer: Option<String>,
    pub reviewed_at_micros: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClinicalAuthority {
    pub mode: AuthorityMode,
    pub review: HumanReview,
}

impl ClinicalAuthority {
    fn validate(&self) -> Result<(), EvidenceCapsuleError> {
        match self.mode {
            AuthorityMode::ClinicianReviewRequired => match self.review.status {
                ReviewStatus::Pending => Ok(()),
                ReviewStatus::Approved | ReviewStatus::Rejected => {
                    if self.review.reviewer.as_deref().unwrap_or("").trim().is_empty()
                        || self.review.reviewed_at_micros.is_none()
                    {
                        Err(EvidenceCapsuleError::IncompleteHumanReview)
                    } else {
                        Ok(())
                    }
                }
                ReviewStatus::NotRequired => Err(EvidenceCapsuleError::ReviewRequirementContradiction),
            },
            AuthorityMode::InformationalOnly | AuthorityMode::Advisory => {
                if matches!(self.review.status, ReviewStatus::Approved | ReviewStatus::Rejected)
                    && (self.review.reviewer.as_deref().unwrap_or("").trim().is_empty()
                        || self.review.reviewed_at_micros.is_none())
                {
                    return Err(EvidenceCapsuleError::IncompleteHumanReview);
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AssertionUncertainty {
    /// Calibrated probability/confidence only when such calibration is actually
    /// established. A missing value is preferable to invented precision.
    pub confidence: Option<f64>,
    pub interpretation: Option<String>,
    pub calibration_reference: Option<ContentDigest>,
}

impl AssertionUncertainty {
    fn validate(&self) -> Result<(), EvidenceCapsuleError> {
        if let Some(confidence) = self.confidence {
            if !validate_confidence_score(confidence, "assertion.confidence").is_valid() {
                return Err(EvidenceCapsuleError::InvalidConfidence);
            }
        }
        if let Some(reference) = &self.calibration_reference {
            reference.validate()?;
        }
        Ok(())
    }
}

/// An evidence-bearing clinical assertion. This is intentionally a container for
/// provenance and review state, not an execution API.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClinicalEvidenceCapsule {
    pub capsule_id: String,
    pub subject: SubjectRef,
    pub kind: AssertionKind,
    /// Human-readable claim/recommendation text.
    pub statement: String,
    /// Optional machine-readable concept for the assertion/recommendation.
    pub code: Option<CodeableConcept>,
    /// Four-state result inherited from the clinical semantics kernel.
    pub state: EvaluationState,
    pub reasons: Vec<String>,
    pub fact_evidence: Vec<FactEvidence>,
    pub sources: Vec<EvidenceSource>,
    pub missing_requirements: Vec<MissingRequirement>,
    pub alternatives: Vec<Alternative>,
    pub uncertainty: Option<AssertionUncertainty>,
    pub execution: ExecutionIdentity,
    pub intended_use: IntendedUse,
    pub authority: ClinicalAuthority,
    pub issued_at_micros: i64,
    pub supersedes_capsule_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PromotionState {
    ResearchOnly,
    BlockedIndeterminate,
    BlockedCriticalMissingData,
    HumanReviewRequired,
    RejectedByHuman,
    EligibleForClinicalPresentation,
}

impl ClinicalEvidenceCapsule {
    pub fn validate(&self) -> Result<(), EvidenceCapsuleError> {
        if self.capsule_id.trim().is_empty() {
            return Err(EvidenceCapsuleError::MissingCapsuleId);
        }
        if self.subject.resource_type.trim().is_empty() || self.subject.id.trim().is_empty() {
            return Err(EvidenceCapsuleError::InvalidSubject);
        }
        if self.statement.trim().is_empty() {
            return Err(EvidenceCapsuleError::MissingStatement);
        }
        if let Some(code) = &self.code {
            code.validate_machine_actionable()
                .map_err(|_| EvidenceCapsuleError::InvalidMachineCode)?;
        }
        self.execution.validate()?;
        self.intended_use.validate()?;
        self.authority.validate()?;
        if let Some(uncertainty) = &self.uncertainty {
            uncertainty.validate()?;
        }

        let mut fact_ids = HashSet::new();
        for evidence in &self.fact_evidence {
            if evidence.fact_id.trim().is_empty() {
                return Err(EvidenceCapsuleError::MissingFactEvidenceId);
            }
            if !fact_ids.insert(evidence.fact_id.as_str()) {
                return Err(EvidenceCapsuleError::DuplicateFactEvidence);
            }
            if let Some(digest) = &evidence.fact_digest {
                digest.validate()?;
            }
        }

        let mut source_ids = HashSet::new();
        for source in &self.sources {
            if source.source_id.trim().is_empty() || source.title.trim().is_empty() {
                return Err(EvidenceCapsuleError::IncompleteEvidenceSource);
            }
            source.content_digest.validate()?;
            if !source_ids.insert(source.source_id.as_str()) {
                return Err(EvidenceCapsuleError::DuplicateEvidenceSource);
            }
        }

        for requirement in &self.missing_requirements {
            if requirement.requirement_id.trim().is_empty() || requirement.description.trim().is_empty() {
                return Err(EvidenceCapsuleError::IncompleteMissingRequirement);
            }
            if let Some(concept) = &requirement.concept {
                concept
                    .validate_machine_actionable()
                    .map_err(|_| EvidenceCapsuleError::InvalidMachineCode)?;
            }
        }

        for alternative in &self.alternatives {
            if alternative.description.trim().is_empty() || alternative.rationale.trim().is_empty() {
                return Err(EvidenceCapsuleError::IncompleteAlternative);
            }
            if let Some(code) = &alternative.code {
                code.validate_machine_actionable()
                    .map_err(|_| EvidenceCapsuleError::InvalidMachineCode)?;
            }
        }

        if self.state == EvaluationState::Indeterminate && self.missing_requirements.is_empty() {
            return Err(EvidenceCapsuleError::IndeterminateWithoutMissingRequirement);
        }

        if matches!(
            self.kind,
            AssertionKind::Recommendation
                | AssertionKind::SafetyAlert
                | AssertionKind::EligibilityAssessment
                | AssertionKind::RiskPrediction
                | AssertionKind::GuidelineApplicability
        ) && matches!(self.state, EvaluationState::Satisfied | EvaluationState::NotSatisfied)
            && self.fact_evidence.is_empty()
        {
            return Err(EvidenceCapsuleError::ConclusionWithoutFactEvidence);
        }

        if matches!(self.kind, AssertionKind::Recommendation | AssertionKind::SafetyAlert)
            && self.sources.is_empty()
        {
            return Err(EvidenceCapsuleError::ClinicalActionWithoutEvidenceSource);
        }

        Ok(())
    }

    /// Determine whether the capsule can advance beyond the evidence boundary.
    /// This does not execute a clinical action; it only states the next allowed
    /// workflow stage.
    pub fn promotion_state(&self) -> Result<PromotionState, EvidenceCapsuleError> {
        self.validate()?;

        if self.intended_use.qualification == QualificationLevel::Experimental {
            return Ok(PromotionState::ResearchOnly);
        }
        if self.state == EvaluationState::Indeterminate {
            return Ok(PromotionState::BlockedIndeterminate);
        }
        if self
            .missing_requirements
            .iter()
            .any(|requirement| requirement.criticality == RequirementCriticality::Critical)
        {
            return Ok(PromotionState::BlockedCriticalMissingData);
        }
        if self.authority.review.status == ReviewStatus::Rejected {
            return Ok(PromotionState::RejectedByHuman);
        }
        if self.authority.mode == AuthorityMode::ClinicianReviewRequired
            && self.authority.review.status != ReviewStatus::Approved
        {
            return Ok(PromotionState::HumanReviewRequired);
        }
        Ok(PromotionState::EligibleForClinicalPresentation)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EvidenceCapsuleError {
    #[error("clinical evidence capsule id is required")]
    MissingCapsuleId,
    #[error("clinical subject must have resource type and id")]
    InvalidSubject,
    #[error("clinical assertion statement is required")]
    MissingStatement,
    #[error("machine-readable assertion code is invalid")]
    InvalidMachineCode,
    #[error("content digest requires algorithm and value")]
    InvalidDigest,
    #[error("execution identity is incomplete")]
    IncompleteExecutionIdentity,
    #[error("intended use is incomplete")]
    IncompleteIntendedUse,
    #[error("human review is incomplete")]
    IncompleteHumanReview,
    #[error("authority mode requires clinician review but review is marked not required")]
    ReviewRequirementContradiction,
    #[error("assertion confidence is invalid")]
    InvalidConfidence,
    #[error("fact evidence id is required")]
    MissingFactEvidenceId,
    #[error("duplicate fact evidence id")]
    DuplicateFactEvidence,
    #[error("evidence source id/title is incomplete")]
    IncompleteEvidenceSource,
    #[error("duplicate evidence source id")]
    DuplicateEvidenceSource,
    #[error("missing requirement id/description is incomplete")]
    IncompleteMissingRequirement,
    #[error("alternative description/rationale is incomplete")]
    IncompleteAlternative,
    #[error("indeterminate assertions must identify at least one missing requirement")]
    IndeterminateWithoutMissingRequirement,
    #[error("determinate clinical conclusion requires fact evidence")]
    ConclusionWithoutFactEvidence,
    #[error("recommendations and safety alerts require an evidence source")]
    ClinicalActionWithoutEvidenceSource,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_semantics::{Coding, EvaluationState};

    fn digest(value: &str) -> ContentDigest {
        ContentDigest {
            algorithm: "sha256".into(),
            value: value.into(),
        }
    }

    fn code() -> CodeableConcept {
        CodeableConcept {
            coding: vec![Coding {
                system: "http://loinc.org".into(),
                code: "14771-0".into(),
                display: Some("Glucose [Moles/volume] in Serum or Plasma".into()),
                version: None,
            }],
            text: Some("glucose".into()),
        }
    }

    fn execution() -> ExecutionIdentity {
        ExecutionIdentity {
            engine: ArtifactIdentity {
                name: "symthaea-cds".into(),
                version: "0.1.0".into(),
                digest: digest("engine"),
            },
            model: None,
            knowledge_artifact: Some(ArtifactIdentity {
                name: "example-guideline".into(),
                version: "2026.1".into(),
                digest: digest("guideline"),
            }),
            environment_digest: Some(digest("environment")),
            operation: "evaluate_example".into(),
        }
    }

    fn source() -> EvidenceSource {
        EvidenceSource {
            source_id: "guideline-1".into(),
            kind: EvidenceSourceKind::Guideline,
            title: "Example guideline".into(),
            version: Some("2026.1".into()),
            canonical_uri: Some("https://example.invalid/guideline".into()),
            content_digest: digest("guideline"),
            retrieved_at_micros: 1_700_000_000_000_000,
        }
    }

    fn base_capsule() -> ClinicalEvidenceCapsule {
        ClinicalEvidenceCapsule {
            capsule_id: "capsule-1".into(),
            subject: SubjectRef {
                resource_type: "Patient".into(),
                id: "patient-a".into(),
            },
            kind: AssertionKind::Recommendation,
            statement: "Review abnormal glucose result with the treating clinician".into(),
            code: Some(code()),
            state: EvaluationState::Satisfied,
            reasons: vec!["example threshold met".into()],
            fact_evidence: vec![FactEvidence {
                fact_id: "fact-1".into(),
                role: EvidenceRole::Supports,
                fact_digest: Some(digest("fact")),
            }],
            sources: vec![source()],
            missing_requirements: vec![],
            alternatives: vec![Alternative {
                description: "Repeat measurement if clinically appropriate".into(),
                code: None,
                rationale: "Confirm unexpected result before escalation".into(),
            }],
            uncertainty: Some(AssertionUncertainty {
                confidence: None,
                interpretation: Some("Rule-based recommendation; no calibrated probability claimed".into()),
                calibration_reference: None,
            }),
            execution: execution(),
            intended_use: IntendedUse {
                use_case: "clinical decision support".into(),
                intended_user: "licensed clinician".into(),
                population: "adults".into(),
                care_setting: "outpatient".into(),
                qualification: QualificationLevel::SupervisedClinical,
            },
            authority: ClinicalAuthority {
                mode: AuthorityMode::ClinicianReviewRequired,
                review: HumanReview {
                    status: ReviewStatus::Pending,
                    reviewer: None,
                    reviewed_at_micros: None,
                    notes: None,
                },
            },
            issued_at_micros: 1_700_000_000_100_000,
            supersedes_capsule_id: None,
        }
    }

    #[test]
    fn valid_capsule_waits_for_required_human_review() {
        let capsule = base_capsule();
        assert_eq!(capsule.validate(), Ok(()));
        assert_eq!(
            capsule.promotion_state(),
            Ok(PromotionState::HumanReviewRequired)
        );
    }

    #[test]
    fn approved_review_allows_clinical_presentation() {
        let mut capsule = base_capsule();
        capsule.authority.review = HumanReview {
            status: ReviewStatus::Approved,
            reviewer: Some("Practitioner/123".into()),
            reviewed_at_micros: Some(1_700_000_000_200_000),
            notes: None,
        };
        assert_eq!(
            capsule.promotion_state(),
            Ok(PromotionState::EligibleForClinicalPresentation)
        );
    }

    #[test]
    fn experimental_capsule_never_promotes_into_clinical_workflow() {
        let mut capsule = base_capsule();
        capsule.intended_use.qualification = QualificationLevel::Experimental;
        capsule.authority.review.status = ReviewStatus::Approved;
        capsule.authority.review.reviewer = Some("Practitioner/123".into());
        capsule.authority.review.reviewed_at_micros = Some(1_700_000_000_200_000);
        assert_eq!(capsule.promotion_state(), Ok(PromotionState::ResearchOnly));
    }

    #[test]
    fn indeterminate_requires_explicit_missing_information() {
        let mut capsule = base_capsule();
        capsule.state = EvaluationState::Indeterminate;
        capsule.missing_requirements.clear();
        assert_eq!(
            capsule.validate(),
            Err(EvidenceCapsuleError::IndeterminateWithoutMissingRequirement)
        );
    }

    #[test]
    fn indeterminate_with_requirement_is_blocked() {
        let mut capsule = base_capsule();
        capsule.state = EvaluationState::Indeterminate;
        capsule.missing_requirements = vec![MissingRequirement {
            requirement_id: "renal-function".into(),
            description: "Current renal function is required before applying this rule".into(),
            concept: None,
            criticality: RequirementCriticality::Critical,
        }];
        assert_eq!(
            capsule.promotion_state(),
            Ok(PromotionState::BlockedIndeterminate)
        );
    }

    #[test]
    fn critical_missing_data_blocks_even_determinate_assertion() {
        let mut capsule = base_capsule();
        capsule.missing_requirements = vec![MissingRequirement {
            requirement_id: "pregnancy-status".into(),
            description: "Pregnancy status is required for this recommendation".into(),
            concept: None,
            criticality: RequirementCriticality::Critical,
        }];
        assert_eq!(
            capsule.promotion_state(),
            Ok(PromotionState::BlockedCriticalMissingData)
        );
    }

    #[test]
    fn recommendation_cannot_exist_without_evidence_source() {
        let mut capsule = base_capsule();
        capsule.sources.clear();
        assert_eq!(
            capsule.validate(),
            Err(EvidenceCapsuleError::ClinicalActionWithoutEvidenceSource)
        );
    }

    #[test]
    fn determinate_assessment_requires_fact_evidence() {
        let mut capsule = base_capsule();
        capsule.kind = AssertionKind::EligibilityAssessment;
        capsule.fact_evidence.clear();
        assert_eq!(
            capsule.validate(),
            Err(EvidenceCapsuleError::ConclusionWithoutFactEvidence)
        );
    }

    #[test]
    fn duplicate_fact_evidence_is_rejected() {
        let mut capsule = base_capsule();
        capsule.fact_evidence.push(capsule.fact_evidence[0].clone());
        assert_eq!(
            capsule.validate(),
            Err(EvidenceCapsuleError::DuplicateFactEvidence)
        );
    }

    #[test]
    fn invalid_confidence_is_rejected_via_existing_health_validator() {
        let mut capsule = base_capsule();
        capsule.uncertainty = Some(AssertionUncertainty {
            confidence: Some(f64::NAN),
            interpretation: None,
            calibration_reference: None,
        });
        assert_eq!(
            capsule.validate(),
            Err(EvidenceCapsuleError::InvalidConfidence)
        );
    }

    #[test]
    fn clinician_review_mode_cannot_claim_review_not_required() {
        let mut capsule = base_capsule();
        capsule.authority.review.status = ReviewStatus::NotRequired;
        assert_eq!(
            capsule.validate(),
            Err(EvidenceCapsuleError::ReviewRequirementContradiction)
        );
    }

    #[test]
    fn rejected_human_review_blocks_promotion() {
        let mut capsule = base_capsule();
        capsule.authority.review = HumanReview {
            status: ReviewStatus::Rejected,
            reviewer: Some("Practitioner/123".into()),
            reviewed_at_micros: Some(1_700_000_000_200_000),
            notes: Some("Evidence does not apply to this patient".into()),
        };
        assert_eq!(
            capsule.promotion_state(),
            Ok(PromotionState::RejectedByHuman)
        );
    }
}
