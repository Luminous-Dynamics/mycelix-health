#![deny(unsafe_code)]
//! Evidence-bearing receipts for every medication safety decision.
//!
//! `MedicationSafetyClearance` intentionally exists only for fully cleared outcomes.
//! Emergency/manual care also needs a truthful artifact for non-cleared outcomes so a
//! later override can identify exactly which medication, patient-context snapshot,
//! safety policy, supplied checks, and reasons produced `Indeterminate` or
//! `RequiresReview`.
//!
//! A serialized receipt is evidence, not authority. `VerifiedMedicationSafetyAssessment`
//! is intentionally non-serializable and can only be produced by rerunning the v1
//! medication-safety evaluator against in-process verified inputs.

use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use mycelix_fhir_medication_semantics::MedicationRequestArtifact;
use mycelix_medication_safety::{
    evaluate_medication_safety, MedicationSafetyContextSnapshot, MedicationSafetyDecision,
    MedicationSafetyError, MedicationSafetyPolicyV1, SafetyCheckKind, SafetyContextKind,
    SafetyDecisionReason, VerifiedSafetyCheck,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const SCHEMA_TAG: &[u8] = b"mycelix-health/medication-safety-assessment-v1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssessmentDecision {
    Cleared,
    RequiresReview,
    Blocked,
    Indeterminate,
}

impl From<MedicationSafetyDecision> for AssessmentDecision {
    fn from(value: MedicationSafetyDecision) -> Self {
        match value {
            MedicationSafetyDecision::Cleared => Self::Cleared,
            MedicationSafetyDecision::RequiresReview => Self::RequiresReview,
            MedicationSafetyDecision::Blocked => Self::Blocked,
            MedicationSafetyDecision::Indeterminate => Self::Indeterminate,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssessmentReason {
    MissingContext(SafetyContextKind),
    ContextFromFuture(SafetyContextKind),
    ContextStale(SafetyContextKind),
    ContextExpired(SafetyContextKind),
    ContextStateNotAllowed(SafetyContextKind),
    MissingCheck(SafetyCheckKind),
    CheckFromFuture(SafetyCheckKind),
    CheckStale(SafetyCheckKind),
    KnowledgeFromFuture(SafetyCheckKind),
    KnowledgeStale(SafetyCheckKind),
    IncompleteKnowledgeCoverage(SafetyCheckKind),
    CheckWarning(SafetyCheckKind),
    CheckBlocked(SafetyCheckKind),
    CheckIndeterminate(SafetyCheckKind),
}

impl From<&SafetyDecisionReason> for AssessmentReason {
    fn from(value: &SafetyDecisionReason) -> Self {
        match value {
            SafetyDecisionReason::MissingContext(kind) => Self::MissingContext(*kind),
            SafetyDecisionReason::ContextFromFuture(kind) => Self::ContextFromFuture(*kind),
            SafetyDecisionReason::ContextStale(kind) => Self::ContextStale(*kind),
            SafetyDecisionReason::ContextExpired(kind) => Self::ContextExpired(*kind),
            SafetyDecisionReason::ContextStateNotAllowed(kind) => {
                Self::ContextStateNotAllowed(*kind)
            }
            SafetyDecisionReason::MissingCheck(kind) => Self::MissingCheck(*kind),
            SafetyDecisionReason::CheckFromFuture(kind) => Self::CheckFromFuture(*kind),
            SafetyDecisionReason::CheckStale(kind) => Self::CheckStale(*kind),
            SafetyDecisionReason::KnowledgeFromFuture(kind) => Self::KnowledgeFromFuture(*kind),
            SafetyDecisionReason::KnowledgeStale(kind) => Self::KnowledgeStale(*kind),
            SafetyDecisionReason::IncompleteKnowledgeCoverage(kind) => {
                Self::IncompleteKnowledgeCoverage(*kind)
            }
            SafetyDecisionReason::CheckWarning(kind) => Self::CheckWarning(*kind),
            SafetyDecisionReason::CheckBlocked(kind) => Self::CheckBlocked(*kind),
            SafetyDecisionReason::CheckIndeterminate(kind) => Self::CheckIndeterminate(*kind),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuppliedSafetyCheckRef {
    pub kind: SafetyCheckKind,
    pub evaluation_digest: StoredDigest,
}

/// Serializable audit representation of one exact medication safety assessment.
///
/// Deserializing this type does not establish that the assessment was actually run.
/// High-assurance workflow code consumes `VerifiedMedicationSafetyAssessment` instead.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MedicationSafetyAssessmentReceiptV1 {
    pub schema_version: u16,
    pub medication_artifact_digest: StoredDigest,
    pub context_digest: StoredDigest,
    pub policy_digest: StoredDigest,
    pub supplied_checks: Vec<SuppliedSafetyCheckRef>,
    pub decision: AssessmentDecision,
    pub reasons: Vec<AssessmentReason>,
    pub assessed_at_micros: i64,
}

impl MedicationSafetyAssessmentReceiptV1 {
    pub fn validate_shape(&self) -> Result<(), AssessmentError> {
        if self.schema_version != 1 {
            return Err(AssessmentError::UnsupportedSchemaVersion(self.schema_version));
        }
        require_domain(
            self.medication_artifact_digest,
            DigestDomain::MedicationRequestArtifact,
        )?;
        require_domain(self.context_digest, DigestDomain::MedicationSafetyContext)?;
        require_domain(self.policy_digest, DigestDomain::MedicationSafetyPolicy)?;

        let mut seen_kinds = std::collections::HashSet::new();
        for check in &self.supplied_checks {
            if !seen_kinds.insert(check.kind) {
                return Err(AssessmentError::DuplicateCheckReference(check.kind));
            }
            require_domain(
                check.evaluation_digest,
                DigestDomain::MedicationSafetyEvaluation,
            )?;
        }

        if self.decision == AssessmentDecision::Cleared && !self.reasons.is_empty() {
            return Err(AssessmentError::ClearedAssessmentHasReasons);
        }
        if self.decision != AssessmentDecision::Cleared && self.reasons.is_empty() {
            return Err(AssessmentError::NonClearedAssessmentHasNoReasons);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, AssessmentError> {
        self.validate_shape()?;
        let encoded = serde_json::to_vec(self)
            .map_err(|error| AssessmentError::Serialization(error.to_string()))?;
        let mut framed = Vec::with_capacity(SCHEMA_TAG.len() + 1 + encoded.len());
        framed.extend_from_slice(SCHEMA_TAG);
        framed.push(0);
        framed.extend_from_slice(&encoded);
        Ok(hash_canonical_bytes(
            DigestDomain::MedicationSafetyAssessment,
            &framed,
        )?)
    }
}

/// In-process proof that the exact v1 safety evaluator produced the accompanying
/// receipt from the supplied verified inputs. No Clone/serde implementation.
pub struct VerifiedMedicationSafetyAssessment {
    receipt: MedicationSafetyAssessmentReceiptV1,
    digest: VerifiedDigest,
}

impl VerifiedMedicationSafetyAssessment {
    pub fn receipt(&self) -> &MedicationSafetyAssessmentReceiptV1 {
        &self.receipt
    }

    pub fn digest(&self) -> StoredDigest {
        self.digest.stored()
    }

    pub fn decision(&self) -> AssessmentDecision {
        self.receipt.decision
    }

    pub fn into_receipt(self) -> MedicationSafetyAssessmentReceiptV1 {
        self.receipt
    }
}

/// Run the canonical v1 medication-safety evaluator and preserve an audit receipt
/// regardless of whether the outcome is cleared, review-required, blocked, or
/// indeterminate.
pub fn evaluate_with_assessment_receipt(
    medication: &MedicationRequestArtifact,
    context: &MedicationSafetyContextSnapshot,
    policy: &MedicationSafetyPolicyV1,
    checks: &[VerifiedSafetyCheck],
    at_micros: i64,
) -> Result<VerifiedMedicationSafetyAssessment, AssessmentError> {
    let evaluation = evaluate_medication_safety(medication, context, policy, checks, at_micros)?;

    let medication_artifact_digest = medication.verified_digest()?.stored();
    let context_digest = context.verified_digest()?.stored();
    let policy_digest = policy.verified_digest()?.stored();

    // Preserve supplied check identity in deterministic policy order. The safety
    // evaluator has already rejected duplicate and unexpected check kinds.
    let supplied_checks = policy
        .required_checks
        .iter()
        .filter_map(|requirement| {
            checks
                .iter()
                .find(|check| check.kind() == requirement.kind)
                .map(|check| SuppliedSafetyCheckRef {
                    kind: requirement.kind,
                    evaluation_digest: check.evaluation_digest(),
                })
        })
        .collect();

    let receipt = MedicationSafetyAssessmentReceiptV1 {
        schema_version: 1,
        medication_artifact_digest,
        context_digest,
        policy_digest,
        supplied_checks,
        decision: evaluation.decision.into(),
        reasons: evaluation.reasons.iter().map(AssessmentReason::from).collect(),
        assessed_at_micros: at_micros,
    };
    let digest = receipt.verified_digest()?;

    Ok(VerifiedMedicationSafetyAssessment { receipt, digest })
}

fn require_domain(digest: StoredDigest, expected: DigestDomain) -> Result<(), AssessmentError> {
    digest.validate_shape()?;
    if digest.domain != expected {
        return Err(AssessmentError::WrongDigestDomain {
            expected,
            actual: digest.domain,
        });
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum AssessmentError {
    #[error("unsupported medication safety assessment schema version {0}")]
    UnsupportedSchemaVersion(u16),
    #[error("duplicate supplied safety-check reference: {0:?}")]
    DuplicateCheckReference(SafetyCheckKind),
    #[error("cleared medication safety assessment cannot contain failure/warning reasons")]
    ClearedAssessmentHasReasons,
    #[error("non-cleared medication safety assessment must explain why it did not clear")]
    NonClearedAssessmentHasNoReasons,
    #[error("medication safety assessment digest domain mismatch: expected {expected:?}, got {actual:?}")]
    WrongDigestDomain {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("medication safety assessment serialization failed: {0}")]
    Serialization(String),
    #[error(transparent)]
    MedicationSafety(#[from] MedicationSafetyError),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
    #[error("FHIR medication artifact failure: {0}")]
    MedicationArtifact(String),
}

impl From<mycelix_fhir_medication_semantics::FhirMedicationError> for AssessmentError {
    fn from(error: mycelix_fhir_medication_semantics::FhirMedicationError) -> Self {
        Self::MedicationArtifact(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(domain: DigestDomain, seed: u8) -> StoredDigest {
        StoredDigest {
            algorithm: mycelix_clinical_integrity::DigestAlgorithm::Blake3_256,
            domain,
            value: [seed; 32],
        }
    }

    #[test]
    fn non_cleared_receipt_requires_typed_reason() {
        let receipt = MedicationSafetyAssessmentReceiptV1 {
            schema_version: 1,
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            context_digest: stored(DigestDomain::MedicationSafetyContext, 2),
            policy_digest: stored(DigestDomain::MedicationSafetyPolicy, 3),
            supplied_checks: vec![],
            decision: AssessmentDecision::Indeterminate,
            reasons: vec![],
            assessed_at_micros: 100,
        };
        assert!(matches!(
            receipt.validate_shape(),
            Err(AssessmentError::NonClearedAssessmentHasNoReasons)
        ));
    }

    #[test]
    fn cleared_receipt_cannot_hide_warning_reason() {
        let receipt = MedicationSafetyAssessmentReceiptV1 {
            schema_version: 1,
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            context_digest: stored(DigestDomain::MedicationSafetyContext, 2),
            policy_digest: stored(DigestDomain::MedicationSafetyPolicy, 3),
            supplied_checks: vec![],
            decision: AssessmentDecision::Cleared,
            reasons: vec![AssessmentReason::MissingContext(
                SafetyContextKind::AllergiesIntolerances,
            )],
            assessed_at_micros: 100,
        };
        assert!(matches!(
            receipt.validate_shape(),
            Err(AssessmentError::ClearedAssessmentHasReasons)
        ));
    }

    #[test]
    fn assessment_has_distinct_digest_domain() {
        let receipt = MedicationSafetyAssessmentReceiptV1 {
            schema_version: 1,
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            context_digest: stored(DigestDomain::MedicationSafetyContext, 2),
            policy_digest: stored(DigestDomain::MedicationSafetyPolicy, 3),
            supplied_checks: vec![SuppliedSafetyCheckRef {
                kind: SafetyCheckKind::DrugDrugInteraction,
                evaluation_digest: stored(DigestDomain::MedicationSafetyEvaluation, 4),
            }],
            decision: AssessmentDecision::Indeterminate,
            reasons: vec![AssessmentReason::MissingContext(
                SafetyContextKind::AllergiesIntolerances,
            )],
            assessed_at_micros: 100,
        };
        assert_eq!(
            receipt.verified_digest().unwrap().domain(),
            DigestDomain::MedicationSafetyAssessment
        );
    }
}
