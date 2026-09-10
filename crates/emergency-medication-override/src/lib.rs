#![deny(unsafe_code)]
//! Evidence-bearing emergency medication override capability.
//!
//! This path is deliberately distinct from qualified medication activation.
//! It exists for time-critical care where the canonical medication-safety evaluator
//! produced `Indeterminate` (or an explicitly policy-permitted `RequiresReview`)
//! because required evidence could not be completed in time.
//!
//! It does **not** convert uncertainty into safety. The output permanently records
//! that care proceeded under an emergency override, with exact medication, safety
//! assessment, professional authority, policy, reason, and rationale commitments.
//! `Blocked` safety evidence is outside this v1 path.

use mycelix_clinical_authority::{
    AuthorityPermit, AuthorityPurpose, EvidenceDigest, JurisdictionCode, PrincipalBinding,
};
use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use mycelix_fhir_medication_semantics::MedicationRequestArtifact;
use mycelix_medication_safety_assessment::{
    AssessmentDecision, AssessmentReason, VerifiedMedicationSafetyAssessment,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const POLICY_SCHEMA_TAG: &[u8] = b"mycelix-health/emergency-medication-override-policy-v1";
const RECEIPT_SCHEMA_TAG: &[u8] = b"mycelix-health/emergency-medication-override-receipt-v1";
const MAX_EVIDENCE_AGE_MICROS: i64 = 86_400_000_000;
const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EmergencyMedicationOverridePolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub max_authority_age_micros: i64,
    pub max_requester_resolution_age_micros: i64,
    pub max_safety_assessment_age_micros: i64,
    pub max_future_skew_micros: i64,
    /// `RequiresReview` means safety evidence was complete enough to find a warning,
    /// but normal review is required. Deployments must opt in explicitly if they
    /// want the emergency path to accept that state. `Indeterminate` is the primary
    /// reason this v1 pathway exists.
    pub allow_requires_review: bool,
}

impl EmergencyMedicationOverridePolicyV1 {
    pub fn validate(&self) -> Result<(), EmergencyOverrideError> {
        if self.schema_version != 1 {
            return Err(EmergencyOverrideError::UnsupportedPolicyVersion(
                self.schema_version,
            ));
        }
        if self.policy_id.trim().is_empty() {
            return Err(EmergencyOverrideError::MissingPolicyId);
        }
        for (value, field) in [
            (self.max_authority_age_micros, "max_authority_age_micros"),
            (
                self.max_requester_resolution_age_micros,
                "max_requester_resolution_age_micros",
            ),
            (
                self.max_safety_assessment_age_micros,
                "max_safety_assessment_age_micros",
            ),
        ] {
            if value <= 0 || value > MAX_EVIDENCE_AGE_MICROS {
                return Err(EmergencyOverrideError::InvalidAgePolicy(field));
            }
        }
        if self.max_future_skew_micros < 0
            || self.max_future_skew_micros > MAX_FUTURE_SKEW_MICROS
        {
            return Err(EmergencyOverrideError::InvalidFutureSkewPolicy);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, EmergencyOverrideError> {
        self.validate()?;
        hash_json(
            DigestDomain::EmergencyMedicationOverridePolicy,
            POLICY_SCHEMA_TAG,
            self,
        )
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EmergencyMedicationReason {
    ImmediateThreatToLife,
    SeriousHarmIfDelayed,
    TimeCriticalTherapy,
    CriticalClinicalInfrastructureUnavailable,
    Other,
}

/// Non-cloneable, non-serializable one-use authorization to proceed under an
/// explicitly emergency/uncertain medication pathway.
pub struct EmergencyMedicationOverrideCapability {
    order_id: String,
    medication_artifact_digest: StoredDigest,
    principal: PrincipalBinding,
    requester_resolution_evidence: [StoredDigest; 3],
    requester_resolved_at_micros: i64,
    safety_assessment_digest: StoredDigest,
    safety_context_digest: StoredDigest,
    safety_policy_digest: StoredDigest,
    safety_decision: AssessmentDecision,
    safety_reasons: Vec<AssessmentReason>,
    safety_assessed_at_micros: i64,
    authority_policy_digest: StoredDigest,
    emergency_policy_digest: StoredDigest,
    jurisdiction: JurisdictionCode,
    emergency_reason: EmergencyMedicationReason,
    rationale_commitment: [u8; 32],
    authorized_at_micros: i64,
    supporting_authority_evidence: Vec<[u8; 32]>,
}

impl EmergencyMedicationOverrideCapability {
    pub fn order_id(&self) -> &str {
        &self.order_id
    }

    pub fn medication_artifact_digest(&self) -> StoredDigest {
        self.medication_artifact_digest
    }

    pub fn principal(&self) -> PrincipalBinding {
        self.principal
    }

    pub fn safety_assessment_digest(&self) -> StoredDigest {
        self.safety_assessment_digest
    }

    pub fn safety_decision(&self) -> AssessmentDecision {
        self.safety_decision
    }

    pub fn emergency_reason(&self) -> EmergencyMedicationReason {
        self.emergency_reason
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EmergencyMedicationOverrideReceiptV1 {
    pub schema_version: u16,
    pub order_id: String,
    pub medication_artifact_digest: StoredDigest,
    pub principal_binding: [u8; 32],
    pub requester_resolution_evidence: [StoredDigest; 3],
    pub requester_resolved_at_micros: i64,
    pub safety_assessment_digest: StoredDigest,
    pub safety_context_digest: StoredDigest,
    pub safety_policy_digest: StoredDigest,
    pub safety_decision: AssessmentDecision,
    pub safety_reasons: Vec<AssessmentReason>,
    pub safety_assessed_at_micros: i64,
    pub authority_policy_digest: StoredDigest,
    pub emergency_policy_digest: StoredDigest,
    pub jurisdiction: JurisdictionCode,
    pub emergency_reason: EmergencyMedicationReason,
    /// Keyed/content commitment to the sensitive human rationale. Raw rationale
    /// belongs in an appropriately protected clinical record, not this portable
    /// receipt or a public DHT entry.
    pub rationale_commitment: [u8; 32],
    pub authorized_at_micros: i64,
    pub supporting_authority_evidence: Vec<[u8; 32]>,
}

impl EmergencyMedicationOverrideReceiptV1 {
    pub fn validate_shape(&self) -> Result<(), EmergencyOverrideError> {
        if self.schema_version != 1 {
            return Err(EmergencyOverrideError::UnsupportedReceiptVersion(
                self.schema_version,
            ));
        }
        if self.order_id.trim().is_empty() {
            return Err(EmergencyOverrideError::MissingOrderId);
        }
        if self.principal_binding == [0u8; 32] {
            return Err(EmergencyOverrideError::ZeroPrincipal);
        }
        if self.rationale_commitment == [0u8; 32] {
            return Err(EmergencyOverrideError::ZeroRationaleCommitment);
        }
        if self.supporting_authority_evidence.is_empty()
            || self
                .supporting_authority_evidence
                .iter()
                .any(|digest| *digest == [0u8; 32])
        {
            return Err(EmergencyOverrideError::InvalidAuthorityEvidence);
        }
        require_domain(
            self.medication_artifact_digest,
            DigestDomain::MedicationRequestArtifact,
        )?;
        for digest in self.requester_resolution_evidence {
            digest.validate_shape()?;
        }
        require_domain(
            self.safety_assessment_digest,
            DigestDomain::MedicationSafetyAssessment,
        )?;
        require_domain(self.safety_context_digest, DigestDomain::MedicationSafetyContext)?;
        require_domain(self.safety_policy_digest, DigestDomain::MedicationSafetyPolicy)?;
        require_domain(self.authority_policy_digest, DigestDomain::AuthorityPolicy)?;
        require_domain(
            self.emergency_policy_digest,
            DigestDomain::EmergencyMedicationOverridePolicy,
        )?;
        if matches!(self.safety_decision, AssessmentDecision::Cleared | AssessmentDecision::Blocked)
        {
            return Err(EmergencyOverrideError::AssessmentNotEligible(
                self.safety_decision,
            ));
        }
        if self.safety_reasons.is_empty() {
            return Err(EmergencyOverrideError::MissingSafetyReasons);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, EmergencyOverrideError> {
        self.validate_shape()?;
        hash_json(
            DigestDomain::EmergencyMedicationOverrideReceipt,
            RECEIPT_SCHEMA_TAG,
            self,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub fn authorize_emergency_medication_override(
    artifact: &MedicationRequestArtifact,
    safety_assessment: VerifiedMedicationSafetyAssessment,
    expected_safety_policy_digest: VerifiedDigest,
    authority: AuthorityPermit,
    authority_policy_digest: VerifiedDigest,
    emergency_policy: &EmergencyMedicationOverridePolicyV1,
    authenticated_principal: PrincipalBinding,
    jurisdiction: &JurisdictionCode,
    emergency_reason: EmergencyMedicationReason,
    rationale_commitment: [u8; 32],
    execution_at_micros: i64,
) -> Result<EmergencyMedicationOverrideCapability, EmergencyOverrideError> {
    emergency_policy.validate()?;
    if rationale_commitment == [0u8; 32] {
        return Err(EmergencyOverrideError::ZeroRationaleCommitment);
    }

    artifact
        .validate_for_activation(execution_at_micros)
        .map_err(|error| EmergencyOverrideError::MedicationArtifact(error.to_string()))?;
    if artifact.requester_principal() != authenticated_principal {
        return Err(EmergencyOverrideError::RequesterPrincipalMismatch);
    }
    verify_freshness(
        artifact.requester_resolved_at_micros(),
        execution_at_micros,
        emergency_policy.max_requester_resolution_age_micros,
        emergency_policy.max_future_skew_micros,
        EmergencyOverrideError::RequesterResolutionFromFuture,
        EmergencyOverrideError::RequesterResolutionStale,
    )?;

    let medication_digest = artifact
        .verified_digest()
        .map_err(|error| EmergencyOverrideError::MedicationArtifact(error.to_string()))?;
    medication_digest.require_domain(DigestDomain::MedicationRequestArtifact)?;

    expected_safety_policy_digest.require_domain(DigestDomain::MedicationSafetyPolicy)?;
    let assessment_receipt = safety_assessment.receipt();
    if assessment_receipt.medication_artifact_digest != medication_digest.stored() {
        return Err(EmergencyOverrideError::AssessmentMedicationMismatch);
    }
    if assessment_receipt.policy_digest != expected_safety_policy_digest.stored() {
        return Err(EmergencyOverrideError::SafetyPolicyMismatch);
    }
    match assessment_receipt.decision {
        AssessmentDecision::Indeterminate => {}
        AssessmentDecision::RequiresReview if emergency_policy.allow_requires_review => {}
        AssessmentDecision::RequiresReview => {
            return Err(EmergencyOverrideError::RequiresReviewNotAllowed)
        }
        AssessmentDecision::Cleared | AssessmentDecision::Blocked => {
            return Err(EmergencyOverrideError::AssessmentNotEligible(
                assessment_receipt.decision,
            ))
        }
    }
    verify_freshness(
        assessment_receipt.assessed_at_micros,
        execution_at_micros,
        emergency_policy.max_safety_assessment_age_micros,
        emergency_policy.max_future_skew_micros,
        EmergencyOverrideError::SafetyAssessmentFromFuture,
        EmergencyOverrideError::SafetyAssessmentStale,
    )?;

    authority_policy_digest.require_domain(DigestDomain::AuthorityPolicy)?;
    if authority.purpose() != AuthorityPurpose::Prescribe {
        return Err(EmergencyOverrideError::WrongAuthorityPurpose);
    }
    if authority.principal() != authenticated_principal {
        return Err(EmergencyOverrideError::AuthorityPrincipalMismatch);
    }
    if authority.target_artifact_digest().0 != medication_digest.value() {
        return Err(EmergencyOverrideError::AuthorityTargetMismatch);
    }
    if authority.policy_digest().0 != authority_policy_digest.value() {
        return Err(EmergencyOverrideError::AuthorityPolicyMismatch);
    }
    if authority.jurisdiction() != jurisdiction {
        return Err(EmergencyOverrideError::AuthorityJurisdictionMismatch);
    }
    verify_freshness(
        authority.evaluated_at_micros(),
        execution_at_micros,
        emergency_policy.max_authority_age_micros,
        emergency_policy.max_future_skew_micros,
        EmergencyOverrideError::AuthorityFromFuture,
        EmergencyOverrideError::AuthorityStale,
    )?;
    if authority.supporting_evidence_digests().is_empty() {
        return Err(EmergencyOverrideError::InvalidAuthorityEvidence);
    }

    let emergency_policy_digest = emergency_policy.verified_digest()?;
    let requester_resolution_evidence = [
        artifact.requester_provider_record_digest(),
        artifact.requester_author_binding_evidence_digest(),
        artifact.requester_status_evidence_digest(),
    ];
    for digest in requester_resolution_evidence {
        digest.validate_shape()?;
    }

    Ok(EmergencyMedicationOverrideCapability {
        order_id: artifact.order().order_id.clone(),
        medication_artifact_digest: medication_digest.stored(),
        principal: authenticated_principal,
        requester_resolution_evidence,
        requester_resolved_at_micros: artifact.requester_resolved_at_micros(),
        safety_assessment_digest: safety_assessment.digest(),
        safety_context_digest: assessment_receipt.context_digest,
        safety_policy_digest: assessment_receipt.policy_digest,
        safety_decision: assessment_receipt.decision,
        safety_reasons: assessment_receipt.reasons.clone(),
        safety_assessed_at_micros: assessment_receipt.assessed_at_micros,
        authority_policy_digest: authority_policy_digest.stored(),
        emergency_policy_digest: emergency_policy_digest.stored(),
        jurisdiction: jurisdiction.clone(),
        emergency_reason,
        rationale_commitment,
        authorized_at_micros: execution_at_micros,
        supporting_authority_evidence: authority
            .supporting_evidence_digests()
            .iter()
            .map(|digest| digest.0)
            .collect(),
    })
}

/// Consume the single-owner emergency capability into an audit receipt. The receipt
/// remains evidence, not authority; serializing/deserializing it cannot recreate the
/// capability that was consumed.
pub fn consume_emergency_override(
    capability: EmergencyMedicationOverrideCapability,
) -> Result<(EmergencyMedicationOverrideReceiptV1, VerifiedDigest), EmergencyOverrideError> {
    let receipt = EmergencyMedicationOverrideReceiptV1 {
        schema_version: 1,
        order_id: capability.order_id,
        medication_artifact_digest: capability.medication_artifact_digest,
        principal_binding: capability.principal.0,
        requester_resolution_evidence: capability.requester_resolution_evidence,
        requester_resolved_at_micros: capability.requester_resolved_at_micros,
        safety_assessment_digest: capability.safety_assessment_digest,
        safety_context_digest: capability.safety_context_digest,
        safety_policy_digest: capability.safety_policy_digest,
        safety_decision: capability.safety_decision,
        safety_reasons: capability.safety_reasons,
        safety_assessed_at_micros: capability.safety_assessed_at_micros,
        authority_policy_digest: capability.authority_policy_digest,
        emergency_policy_digest: capability.emergency_policy_digest,
        jurisdiction: capability.jurisdiction,
        emergency_reason: capability.emergency_reason,
        rationale_commitment: capability.rationale_commitment,
        authorized_at_micros: capability.authorized_at_micros,
        supporting_authority_evidence: capability.supporting_authority_evidence,
    };
    let digest = receipt.verified_digest()?;
    Ok((receipt, digest))
}

fn verify_freshness(
    evidence_at: i64,
    now: i64,
    max_age: i64,
    max_future_skew: i64,
    future_error: EmergencyOverrideError,
    stale_error: EmergencyOverrideError,
) -> Result<(), EmergencyOverrideError> {
    let delta = now as i128 - evidence_at as i128;
    if delta < -(max_future_skew as i128) {
        return Err(future_error);
    }
    if delta.max(0) > max_age as i128 {
        return Err(stale_error);
    }
    Ok(())
}

fn require_domain(digest: StoredDigest, expected: DigestDomain) -> Result<(), EmergencyOverrideError> {
    digest.validate_shape()?;
    if digest.domain != expected {
        return Err(EmergencyOverrideError::WrongDigestDomain {
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
) -> Result<VerifiedDigest, EmergencyOverrideError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| EmergencyOverrideError::Serialization(error.to_string()))?;
    let mut framed = Vec::with_capacity(schema_tag.len() + 1 + encoded.len());
    framed.extend_from_slice(schema_tag);
    framed.push(0);
    framed.extend_from_slice(&encoded);
    Ok(hash_canonical_bytes(domain, &framed)?)
}

#[derive(Debug, Error)]
pub enum EmergencyOverrideError {
    #[error("unsupported emergency medication override policy version {0}")]
    UnsupportedPolicyVersion(u16),
    #[error("unsupported emergency medication override receipt version {0}")]
    UnsupportedReceiptVersion(u16),
    #[error("emergency medication override policy id is required")]
    MissingPolicyId,
    #[error("emergency medication override receipt order id is required")]
    MissingOrderId,
    #[error("invalid emergency evidence-age policy: {0}")]
    InvalidAgePolicy(&'static str),
    #[error("emergency future-skew allowance must be between zero and five minutes")]
    InvalidFutureSkewPolicy,
    #[error("emergency rationale commitment cannot be zero")]
    ZeroRationaleCommitment,
    #[error("emergency principal binding cannot be zero")]
    ZeroPrincipal,
    #[error("emergency override requires non-zero professional authority evidence")]
    InvalidAuthorityEvidence,
    #[error("requester principal does not match authenticated principal")]
    RequesterPrincipalMismatch,
    #[error("requester identity resolution is from the future")]
    RequesterResolutionFromFuture,
    #[error("requester identity resolution is stale")]
    RequesterResolutionStale,
    #[error("medication safety assessment targets another medication artifact")]
    AssessmentMedicationMismatch,
    #[error("medication safety assessment used a different safety policy")]
    SafetyPolicyMismatch,
    #[error("safety assessment outcome is not eligible for emergency uncertainty override: {0:?}")]
    AssessmentNotEligible(AssessmentDecision),
    #[error("RequiresReview is not enabled by this emergency override policy")]
    RequiresReviewNotAllowed,
    #[error("emergency override safety assessment is from the future")]
    SafetyAssessmentFromFuture,
    #[error("emergency override safety assessment is stale")]
    SafetyAssessmentStale,
    #[error("professional authority purpose must be Prescribe")]
    WrongAuthorityPurpose,
    #[error("professional authority principal does not match authenticated principal")]
    AuthorityPrincipalMismatch,
    #[error("professional authority targets another medication artifact")]
    AuthorityTargetMismatch,
    #[error("professional authority used a different authority policy")]
    AuthorityPolicyMismatch,
    #[error("professional authority jurisdiction does not match emergency execution jurisdiction")]
    AuthorityJurisdictionMismatch,
    #[error("professional authority decision is from the future")]
    AuthorityFromFuture,
    #[error("professional authority decision is stale")]
    AuthorityStale,
    #[error("emergency safety assessment must contain typed reasons")]
    MissingSafetyReasons,
    #[error("emergency medication digest domain mismatch: expected {expected:?}, got {actual:?}")]
    WrongDigestDomain {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("emergency medication artifact failed validation: {0}")]
    MedicationArtifact(String),
    #[error("emergency override serialization failed: {0}")]
    Serialization(String),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_integrity::DigestAlgorithm;

    fn stored(domain: DigestDomain, seed: u8) -> StoredDigest {
        StoredDigest {
            algorithm: DigestAlgorithm::Blake3_256,
            domain,
            value: [seed; 32],
        }
    }

    fn jurisdiction() -> JurisdictionCode {
        JurisdictionCode {
            system: "urn:iso:std:iso:3166-2".into(),
            code: "US-TX".into(),
        }
    }

    fn receipt(decision: AssessmentDecision) -> EmergencyMedicationOverrideReceiptV1 {
        EmergencyMedicationOverrideReceiptV1 {
            schema_version: 1,
            order_id: "order-1".into(),
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            principal_binding: [2; 32],
            requester_resolution_evidence: [
                stored(DigestDomain::ClinicalArtifact, 3),
                stored(DigestDomain::ClinicalArtifact, 4),
                stored(DigestDomain::ClinicalArtifact, 5),
            ],
            requester_resolved_at_micros: 10,
            safety_assessment_digest: stored(DigestDomain::MedicationSafetyAssessment, 6),
            safety_context_digest: stored(DigestDomain::MedicationSafetyContext, 7),
            safety_policy_digest: stored(DigestDomain::MedicationSafetyPolicy, 8),
            safety_decision: decision,
            safety_reasons: vec![AssessmentReason::MissingContext(
                mycelix_medication_safety_assessment::SafetyContextKind::AllergiesIntolerances,
            )],
            safety_assessed_at_micros: 20,
            authority_policy_digest: stored(DigestDomain::AuthorityPolicy, 9),
            emergency_policy_digest: stored(DigestDomain::EmergencyMedicationOverridePolicy, 10),
            jurisdiction: jurisdiction(),
            emergency_reason: EmergencyMedicationReason::SeriousHarmIfDelayed,
            rationale_commitment: [11; 32],
            authorized_at_micros: 30,
            supporting_authority_evidence: vec![[12; 32]],
        }
    }

    #[test]
    fn indeterminate_receipt_is_valid_shape() {
        assert!(receipt(AssessmentDecision::Indeterminate)
            .validate_shape()
            .is_ok());
    }

    #[test]
    fn blocked_receipt_cannot_masquerade_as_emergency_uncertainty_override() {
        assert!(matches!(
            receipt(AssessmentDecision::Blocked).validate_shape(),
            Err(EmergencyOverrideError::AssessmentNotEligible(
                AssessmentDecision::Blocked
            ))
        ));
    }

    #[test]
    fn cleared_receipt_belongs_on_normal_activation_path() {
        assert!(matches!(
            receipt(AssessmentDecision::Cleared).validate_shape(),
            Err(EmergencyOverrideError::AssessmentNotEligible(
                AssessmentDecision::Cleared
            ))
        ));
    }

    #[test]
    fn zero_rationale_commitment_is_rejected() {
        let mut value = receipt(AssessmentDecision::Indeterminate);
        value.rationale_commitment = [0; 32];
        assert!(matches!(
            value.validate_shape(),
            Err(EmergencyOverrideError::ZeroRationaleCommitment)
        ));
    }

    #[test]
    fn emergency_receipt_has_dedicated_digest_domain() {
        assert_eq!(
            receipt(AssessmentDecision::Indeterminate)
                .verified_digest()
                .unwrap()
                .domain(),
            DigestDomain::EmergencyMedicationOverrideReceipt
        );
    }
}
