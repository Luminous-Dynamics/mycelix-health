#![deny(unsafe_code)]
//! Structural qualification gate for clinician-facing Mycelix-Health outputs.
//!
//! The key design choice is that a clinician-facing presentation permit is an
//! opaque Rust value. Safe downstream code can require that permit instead of
//! trusting mutable metadata such as a string qualification label.
//!
//! Model-backed supervised outputs have an additional invariant: they cannot
//! obtain a clinician-presentation permit through the legacy capsule-only path.
//! They require a separately policy-qualified distribution/OOD assessment bound
//! to the exact capsule, model, subject, detector, and reference domain.

use mycelix_clinical_distribution_assessment::{
    validate_distribution_for_capsule, ClinicalDistributionAssessmentError,
    ClinicalDistributionAssessmentV1, ClinicalDistributionPolicyV1,
    ClinicalDistributionStatusV1,
};
use mycelix_clinical_evidence::{
    AuthorityMode, ClinicalEvidenceCapsule, ContentDigest, EvidenceCapsuleError,
    QualificationLevel, RequirementCriticality, ReviewStatus,
};
use mycelix_clinical_semantics::EvaluationState;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateDecision {
    ResearchOnly,
    OfflineValidationOnly,
    ShadowWorkflowOnly,
    BlockedIndeterminate,
    BlockedCriticalMissingData,
    BlockedModelRequiresDistributionAssessment,
    BlockedDistributionNotRun,
    BlockedDistributionUnavailable,
    BlockedDistributionIndeterminate,
    BlockedOutOfDistribution,
    HumanReviewRequired,
    RejectedByHuman,
    EligibleForClinicalPresentation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateOutcome {
    pub decision: GateDecision,
    permit: Option<ClinicalPresentationPermit>,
}

impl GateOutcome {
    pub fn permit(&self) -> Option<&ClinicalPresentationPermit> {
        self.permit.as_ref()
    }
}

/// Opaque proof that one exact evidence capsule passed the supervised-clinical
/// promotion boundary. This type deliberately does not implement serde traits;
/// it is not a wire credential and cannot be reconstructed from untrusted JSON.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClinicalPresentationPermit {
    capsule_id: String,
    subject_resource_type: String,
    subject_id: String,
    issued_at_micros: i64,
    distribution_assessment_digest: Option<ContentDigest>,
    distribution_policy_digest: Option<ContentDigest>,
}

impl ClinicalPresentationPermit {
    pub fn capsule_id(&self) -> &str {
        &self.capsule_id
    }

    pub fn subject_resource_type(&self) -> &str {
        &self.subject_resource_type
    }

    pub fn subject_id(&self) -> &str {
        &self.subject_id
    }

    pub fn issued_at_micros(&self) -> i64 {
        self.issued_at_micros
    }

    /// Present only for model-backed permits qualified through the distribution
    /// assurance path.
    pub fn distribution_assessment_digest(&self) -> Option<&ContentDigest> {
        self.distribution_assessment_digest.as_ref()
    }

    /// Present only for model-backed permits qualified through the distribution
    /// assurance path.
    pub fn distribution_policy_digest(&self) -> Option<&ContentDigest> {
        self.distribution_policy_digest.as_ref()
    }
}

/// Capsule-only presentation evaluation.
///
/// Deterministic/rule-backed supervised capsules may continue to use this path.
/// A supervised capsule carrying a model identity is deliberately blocked until
/// the caller supplies a separately qualified distribution assessment through
/// [`evaluate_model_for_clinical_presentation`].
pub fn evaluate_for_clinical_presentation(
    capsule: &ClinicalEvidenceCapsule,
) -> Result<GateOutcome, EvidenceCapsuleError> {
    capsule.validate()?;

    let decision = match capsule.intended_use.qualification {
        QualificationLevel::Experimental => GateDecision::ResearchOnly,
        QualificationLevel::ValidatedOffline => GateDecision::OfflineValidationOnly,
        QualificationLevel::ShadowClinical => GateDecision::ShadowWorkflowOnly,
        QualificationLevel::SupervisedClinical if capsule.execution.model.is_some() => {
            GateDecision::BlockedModelRequiresDistributionAssessment
        }
        QualificationLevel::SupervisedClinical => evaluate_supervised(capsule),
    };

    let permit = if decision == GateDecision::EligibleForClinicalPresentation {
        Some(make_permit(capsule, None, None))
    } else {
        None
    };

    Ok(GateOutcome { decision, permit })
}

/// Model-aware presentation evaluation with an exact distribution/OOD proof.
///
/// Structural mismatches (wrong model, wrong subject, wrong detector, stale
/// evidence, capsule substitution, etc.) are errors. A valid negative state such
/// as `OutOfDistribution` is not an error: it produces an explicit blocked gate
/// decision while preserving the assessment as evidence.
pub fn evaluate_model_for_clinical_presentation(
    capsule: &ClinicalEvidenceCapsule,
    assessment: &ClinicalDistributionAssessmentV1,
    policy: &ClinicalDistributionPolicyV1,
    now_micros: i64,
) -> Result<GateOutcome, ClinicalAiPromotionError> {
    capsule.validate()?;
    let validated = validate_distribution_for_capsule(capsule, assessment, policy, now_micros)?;

    let decision = match capsule.intended_use.qualification {
        QualificationLevel::Experimental => GateDecision::ResearchOnly,
        QualificationLevel::ValidatedOffline => GateDecision::OfflineValidationOnly,
        QualificationLevel::ShadowClinical => GateDecision::ShadowWorkflowOnly,
        QualificationLevel::SupervisedClinical => match validated.status() {
            ClinicalDistributionStatusV1::NotRun => GateDecision::BlockedDistributionNotRun,
            ClinicalDistributionStatusV1::Unavailable => {
                GateDecision::BlockedDistributionUnavailable
            }
            ClinicalDistributionStatusV1::Indeterminate => {
                GateDecision::BlockedDistributionIndeterminate
            }
            ClinicalDistributionStatusV1::OutOfDistribution => {
                GateDecision::BlockedOutOfDistribution
            }
            ClinicalDistributionStatusV1::InDistribution => evaluate_supervised(capsule),
        },
    };

    let permit = if decision == GateDecision::EligibleForClinicalPresentation {
        Some(make_permit(
            capsule,
            Some(validated.assessment_digest().clone()),
            Some(validated.policy_digest().clone()),
        ))
    } else {
        None
    };

    Ok(GateOutcome { decision, permit })
}

fn make_permit(
    capsule: &ClinicalEvidenceCapsule,
    distribution_assessment_digest: Option<ContentDigest>,
    distribution_policy_digest: Option<ContentDigest>,
) -> ClinicalPresentationPermit {
    ClinicalPresentationPermit {
        capsule_id: capsule.capsule_id.clone(),
        subject_resource_type: capsule.subject.resource_type.clone(),
        subject_id: capsule.subject.id.clone(),
        issued_at_micros: capsule.issued_at_micros,
        distribution_assessment_digest,
        distribution_policy_digest,
    }
}

fn evaluate_supervised(capsule: &ClinicalEvidenceCapsule) -> GateDecision {
    if capsule.state == EvaluationState::Indeterminate {
        return GateDecision::BlockedIndeterminate;
    }

    if capsule
        .missing_requirements
        .iter()
        .any(|requirement| requirement.criticality == RequirementCriticality::Critical)
    {
        return GateDecision::BlockedCriticalMissingData;
    }

    if capsule.authority.review.status == ReviewStatus::Rejected {
        return GateDecision::RejectedByHuman;
    }

    if capsule.authority.mode == AuthorityMode::ClinicianReviewRequired
        && capsule.authority.review.status != ReviewStatus::Approved
    {
        return GateDecision::HumanReviewRequired;
    }

    GateDecision::EligibleForClinicalPresentation
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClinicalAiPromotionError {
    #[error(transparent)]
    EvidenceCapsule(#[from] EvidenceCapsuleError),
    #[error(transparent)]
    Distribution(#[from] ClinicalDistributionAssessmentError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_distribution_assessment::{
        capsule_binding_digest, ClinicalDistributionAssessmentV1,
        ClinicalDistributionPolicyV1, NumericDistributionBoundaryV1,
        NumericDistributionDirectionV1, CLINICAL_DISTRIBUTION_ASSESSMENT_VERSION,
        CLINICAL_DISTRIBUTION_POLICY_VERSION,
    };
    use mycelix_clinical_evidence::{
        Alternative, ArtifactIdentity, AssertionKind, AssertionUncertainty, ClinicalAuthority,
        ContentDigest, EvidenceRole, EvidenceSource, EvidenceSourceKind, ExecutionIdentity,
        FactEvidence, HumanReview, IntendedUse, MissingRequirement, RequirementCriticality,
    };
    use mycelix_clinical_semantics::{CodeableConcept, Coding, EvaluationState, SubjectRef};

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

    fn capsule(qualification: QualificationLevel) -> ClinicalEvidenceCapsule {
        ClinicalEvidenceCapsule {
            capsule_id: "capsule-1".into(),
            subject: SubjectRef {
                resource_type: "Patient".into(),
                id: "patient-a".into(),
            },
            kind: AssertionKind::Recommendation,
            statement: "Review finding with clinician".into(),
            code: Some(CodeableConcept {
                coding: vec![Coding {
                    system: "http://loinc.org".into(),
                    code: "14771-0".into(),
                    display: Some("Glucose".into()),
                    version: None,
                }],
                text: Some("Glucose".into()),
            }),
            state: EvaluationState::Satisfied,
            reasons: vec!["rule matched".into()],
            fact_evidence: vec![FactEvidence {
                fact_id: "fact-1".into(),
                role: EvidenceRole::Supports,
                fact_digest: Some(digest("fact")),
            }],
            sources: vec![EvidenceSource {
                source_id: "guideline-1".into(),
                kind: EvidenceSourceKind::Guideline,
                title: "Example guideline".into(),
                version: Some("2026.1".into()),
                canonical_uri: None,
                content_digest: digest("guideline"),
                retrieved_at_micros: 1,
            }],
            missing_requirements: vec![],
            alternatives: vec![Alternative {
                description: "Repeat measurement".into(),
                code: None,
                rationale: "Confirm unexpected result".into(),
            }],
            uncertainty: Some(AssertionUncertainty {
                confidence: None,
                interpretation: Some("deterministic rule".into()),
                calibration_reference: None,
            }),
            execution: ExecutionIdentity {
                engine: artifact("symthaea-cds", "0.1.0", "engine"),
                model: None,
                knowledge_artifact: Some(artifact("guideline", "2026.1", "guideline")),
                environment_digest: Some(digest("env")),
                operation: "evaluate".into(),
            },
            intended_use: IntendedUse {
                use_case: "decision support".into(),
                intended_user: "licensed clinician".into(),
                population: "adults".into(),
                care_setting: "outpatient".into(),
                qualification,
            },
            authority: ClinicalAuthority {
                mode: AuthorityMode::ClinicianReviewRequired,
                review: HumanReview {
                    status: ReviewStatus::Approved,
                    reviewer: Some("Practitioner/123".into()),
                    reviewed_at_micros: Some(2),
                    notes: None,
                },
            },
            issued_at_micros: 3,
            supersedes_capsule_id: None,
        }
    }

    fn model_capsule() -> ClinicalEvidenceCapsule {
        let mut capsule = capsule(QualificationLevel::SupervisedClinical);
        capsule.kind = AssertionKind::RiskPrediction;
        capsule.execution.model = Some(artifact("risk-model", "1.0.0", "risk-model"));
        capsule.execution.knowledge_artifact = None;
        capsule
    }

    fn distribution_policy() -> ClinicalDistributionPolicyV1 {
        ClinicalDistributionPolicyV1 {
            schema_version: CLINICAL_DISTRIBUTION_POLICY_VERSION,
            policy_id: "distribution-policy-v1".into(),
            required_detector: artifact("ood-detector", "1.0.0", "ood-detector"),
            required_reference_population: "adult-outpatient-v1".into(),
            required_reference_domain_digest: digest("reference-domain"),
            max_age_micros: 1_000,
            max_future_skew_micros: 10,
        }
    }

    fn distribution_assessment(
        capsule: &ClinicalEvidenceCapsule,
        status: ClinicalDistributionStatusV1,
    ) -> ClinicalDistributionAssessmentV1 {
        let numeric_boundary = match status {
            ClinicalDistributionStatusV1::InDistribution => {
                Some(NumericDistributionBoundaryV1 {
                    score: 0.8,
                    threshold: 0.5,
                    direction: NumericDistributionDirectionV1::HigherMeansMoreInDistribution,
                })
            }
            ClinicalDistributionStatusV1::OutOfDistribution => {
                Some(NumericDistributionBoundaryV1 {
                    score: 0.2,
                    threshold: 0.5,
                    direction: NumericDistributionDirectionV1::HigherMeansMoreInDistribution,
                })
            }
            _ => None,
        };
        ClinicalDistributionAssessmentV1 {
            schema_version: CLINICAL_DISTRIBUTION_ASSESSMENT_VERSION,
            assessment_id: "assessment-1".into(),
            subject: capsule.subject.clone(),
            model: capsule.execution.model.clone().expect("model required"),
            detector: distribution_policy().required_detector,
            reference_population: "adult-outpatient-v1".into(),
            reference_domain_digest: digest("reference-domain"),
            capsule_binding_digest: capsule_binding_digest(capsule).unwrap(),
            status,
            numeric_boundary,
            assessment_evidence_digest: if status == ClinicalDistributionStatusV1::NotRun
                || status == ClinicalDistributionStatusV1::Unavailable
            {
                None
            } else {
                Some(digest("distribution-evidence"))
            },
            assessed_at_micros: 100,
        }
    }

    #[test]
    fn experimental_cannot_obtain_clinical_permit() {
        let outcome = evaluate_for_clinical_presentation(&capsule(QualificationLevel::Experimental))
            .unwrap();
        assert_eq!(outcome.decision, GateDecision::ResearchOnly);
        assert!(outcome.permit().is_none());
    }

    #[test]
    fn offline_validated_cannot_obtain_clinical_permit() {
        let outcome = evaluate_for_clinical_presentation(&capsule(QualificationLevel::ValidatedOffline))
            .unwrap();
        assert_eq!(outcome.decision, GateDecision::OfflineValidationOnly);
        assert!(outcome.permit().is_none());
    }

    #[test]
    fn shadow_cannot_obtain_clinical_permit() {
        let outcome = evaluate_for_clinical_presentation(&capsule(QualificationLevel::ShadowClinical))
            .unwrap();
        assert_eq!(outcome.decision, GateDecision::ShadowWorkflowOnly);
        assert!(outcome.permit().is_none());
    }

    #[test]
    fn supervised_approved_rule_capsule_gets_opaque_permit() {
        let capsule = capsule(QualificationLevel::SupervisedClinical);
        let outcome = evaluate_for_clinical_presentation(&capsule).unwrap();
        assert_eq!(outcome.decision, GateDecision::EligibleForClinicalPresentation);
        let permit = outcome.permit().expect("permit required");
        assert_eq!(permit.capsule_id(), "capsule-1");
        assert_eq!(permit.subject_id(), "patient-a");
        assert!(permit.distribution_assessment_digest().is_none());
    }

    #[test]
    fn model_cannot_use_capsule_only_supervised_path() {
        let outcome = evaluate_for_clinical_presentation(&model_capsule()).unwrap();
        assert_eq!(
            outcome.decision,
            GateDecision::BlockedModelRequiresDistributionAssessment
        );
        assert!(outcome.permit().is_none());
    }

    #[test]
    fn in_distribution_model_can_reach_existing_supervised_gate() {
        let capsule = model_capsule();
        let outcome = evaluate_model_for_clinical_presentation(
            &capsule,
            &distribution_assessment(&capsule, ClinicalDistributionStatusV1::InDistribution),
            &distribution_policy(),
            200,
        )
        .unwrap();
        assert_eq!(outcome.decision, GateDecision::EligibleForClinicalPresentation);
        let permit = outcome.permit().expect("permit required");
        assert!(permit.distribution_assessment_digest().is_some());
        assert!(permit.distribution_policy_digest().is_some());
    }

    #[test]
    fn out_of_distribution_model_is_explicitly_blocked() {
        let capsule = model_capsule();
        let outcome = evaluate_model_for_clinical_presentation(
            &capsule,
            &distribution_assessment(
                &capsule,
                ClinicalDistributionStatusV1::OutOfDistribution,
            ),
            &distribution_policy(),
            200,
        )
        .unwrap();
        assert_eq!(outcome.decision, GateDecision::BlockedOutOfDistribution);
        assert!(outcome.permit().is_none());
    }

    #[test]
    fn detector_not_run_is_distinct_from_in_distribution() {
        let capsule = model_capsule();
        let outcome = evaluate_model_for_clinical_presentation(
            &capsule,
            &distribution_assessment(&capsule, ClinicalDistributionStatusV1::NotRun),
            &distribution_policy(),
            200,
        )
        .unwrap();
        assert_eq!(outcome.decision, GateDecision::BlockedDistributionNotRun);
        assert!(outcome.permit().is_none());
    }

    #[test]
    fn indeterminate_supervised_capsule_is_blocked_after_distribution_pass() {
        let mut capsule = model_capsule();
        capsule.state = EvaluationState::Indeterminate;
        capsule.missing_requirements = vec![MissingRequirement {
            requirement_id: "renal-function".into(),
            description: "renal function required".into(),
            concept: None,
            criticality: RequirementCriticality::Critical,
        }];
        let assessment = distribution_assessment(&capsule, ClinicalDistributionStatusV1::InDistribution);
        let outcome = evaluate_model_for_clinical_presentation(
            &capsule,
            &assessment,
            &distribution_policy(),
            200,
        )
        .unwrap();
        assert_eq!(outcome.decision, GateDecision::BlockedIndeterminate);
        assert!(outcome.permit().is_none());
    }

    #[test]
    fn stale_distribution_assessment_is_not_a_gate_decision() {
        let capsule = model_capsule();
        let mut policy = distribution_policy();
        policy.max_age_micros = 50;
        let result = evaluate_model_for_clinical_presentation(
            &capsule,
            &distribution_assessment(&capsule, ClinicalDistributionStatusV1::InDistribution),
            &policy,
            200,
        );
        assert_eq!(
            result,
            Err(ClinicalAiPromotionError::Distribution(
                ClinicalDistributionAssessmentError::StaleAssessment
            ))
        );
    }

    #[test]
    fn critical_missing_data_blocks_rule_permit() {
        let mut capsule = capsule(QualificationLevel::SupervisedClinical);
        capsule.missing_requirements = vec![MissingRequirement {
            requirement_id: "pregnancy-status".into(),
            description: "pregnancy status required".into(),
            concept: None,
            criticality: RequirementCriticality::Critical,
        }];
        let outcome = evaluate_for_clinical_presentation(&capsule).unwrap();
        assert_eq!(outcome.decision, GateDecision::BlockedCriticalMissingData);
        assert!(outcome.permit().is_none());
    }

    #[test]
    fn pending_required_review_blocks_permit() {
        let mut capsule = capsule(QualificationLevel::SupervisedClinical);
        capsule.authority.review = HumanReview {
            status: ReviewStatus::Pending,
            reviewer: None,
            reviewed_at_micros: None,
            notes: None,
        };
        let outcome = evaluate_for_clinical_presentation(&capsule).unwrap();
        assert_eq!(outcome.decision, GateDecision::HumanReviewRequired);
        assert!(outcome.permit().is_none());
    }

    #[test]
    fn rejected_review_blocks_permit() {
        let mut capsule = capsule(QualificationLevel::SupervisedClinical);
        capsule.authority.review = HumanReview {
            status: ReviewStatus::Rejected,
            reviewer: Some("Practitioner/123".into()),
            reviewed_at_micros: Some(2),
            notes: Some("not applicable".into()),
        };
        let outcome = evaluate_for_clinical_presentation(&capsule).unwrap();
        assert_eq!(outcome.decision, GateDecision::RejectedByHuman);
        assert!(outcome.permit().is_none());
    }
}
