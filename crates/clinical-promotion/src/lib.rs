#![deny(unsafe_code)]
//! Structural qualification gate for clinician-facing Mycelix-Health outputs.
//!
//! The key design choice is that a clinician-facing presentation permit is an
//! opaque Rust value. Safe downstream code can require that permit instead of
//! trusting mutable metadata such as a string qualification label.

use mycelix_clinical_evidence::{
    AuthorityMode, ClinicalEvidenceCapsule, EvidenceCapsuleError, QualificationLevel,
    RequirementCriticality, ReviewStatus,
};
use mycelix_clinical_evidence::mycelix_clinical_semantics::EvaluationState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateDecision {
    ResearchOnly,
    OfflineValidationOnly,
    ShadowWorkflowOnly,
    BlockedIndeterminate,
    BlockedCriticalMissingData,
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
}

pub fn evaluate_for_clinical_presentation(
    capsule: &ClinicalEvidenceCapsule,
) -> Result<GateOutcome, EvidenceCapsuleError> {
    capsule.validate()?;

    let decision = match capsule.intended_use.qualification {
        QualificationLevel::Experimental => GateDecision::ResearchOnly,
        QualificationLevel::ValidatedOffline => GateDecision::OfflineValidationOnly,
        QualificationLevel::ShadowClinical => GateDecision::ShadowWorkflowOnly,
        QualificationLevel::SupervisedClinical => evaluate_supervised(capsule),
    };

    let permit = if decision == GateDecision::EligibleForClinicalPresentation {
        Some(ClinicalPresentationPermit {
            capsule_id: capsule.capsule_id.clone(),
            subject_resource_type: capsule.subject.resource_type.clone(),
            subject_id: capsule.subject.id.clone(),
            issued_at_micros: capsule.issued_at_micros,
        })
    } else {
        None
    };

    Ok(GateOutcome { decision, permit })
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

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_evidence::{
        Alternative, ArtifactIdentity, AssertionKind, AssertionUncertainty, ClinicalAuthority,
        ContentDigest, EvidenceRole, EvidenceSource, EvidenceSourceKind, ExecutionIdentity,
        FactEvidence, HumanReview, IntendedUse, MissingRequirement, RequirementCriticality,
    };
    use mycelix_clinical_evidence::mycelix_clinical_semantics::{
        CodeableConcept, Coding, EvaluationState, SubjectRef,
    };

    fn digest(value: &str) -> ContentDigest {
        ContentDigest {
            algorithm: "sha256".into(),
            value: value.into(),
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
                engine: ArtifactIdentity {
                    name: "symthaea-cds".into(),
                    version: "0.1.0".into(),
                    digest: digest("engine"),
                },
                model: None,
                knowledge_artifact: Some(ArtifactIdentity {
                    name: "guideline".into(),
                    version: "2026.1".into(),
                    digest: digest("guideline"),
                }),
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
    fn supervised_approved_capsule_gets_opaque_permit() {
        let capsule = capsule(QualificationLevel::SupervisedClinical);
        let outcome = evaluate_for_clinical_presentation(&capsule).unwrap();
        assert_eq!(outcome.decision, GateDecision::EligibleForClinicalPresentation);
        let permit = outcome.permit().expect("permit required");
        assert_eq!(permit.capsule_id(), "capsule-1");
        assert_eq!(permit.subject_id(), "patient-a");
    }

    #[test]
    fn indeterminate_supervised_capsule_is_blocked() {
        let mut capsule = capsule(QualificationLevel::SupervisedClinical);
        capsule.state = EvaluationState::Indeterminate;
        capsule.missing_requirements = vec![MissingRequirement {
            requirement_id: "renal-function".into(),
            description: "renal function required".into(),
            concept: None,
            criticality: RequirementCriticality::Critical,
        }];
        let outcome = evaluate_for_clinical_presentation(&capsule).unwrap();
        assert_eq!(outcome.decision, GateDecision::BlockedIndeterminate);
        assert!(outcome.permit().is_none());
    }

    #[test]
    fn critical_missing_data_blocks_permit() {
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
