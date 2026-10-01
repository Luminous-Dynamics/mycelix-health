#![deny(unsafe_code)]
//! Exact-target capability composition for Mycelix-Health.
//!
//! A valid credential, requester identity, qualification result, medication-safety
//! result, or source-trust result is not sufficient on its own. This crate composes
//! verifier-owned evidence into exact-purpose, exact-target workflow capabilities.
//!
//! Resulting workflow capabilities intentionally implement no Clone/Copy/serde/Debug.

use mycelix_clinical_authority::{
    AuthorityPermit, AuthorityPurpose, EvidenceDigest, JurisdictionCode, PrincipalBinding,
};
use mycelix_clinical_evidence::ClinicalEvidenceCapsule;
use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use mycelix_clinical_promotion::{evaluate_for_clinical_presentation, GateDecision};
use mycelix_clinical_quarantine::{
    ArtifactDigest, DigestAlgorithm as QuarantineDigestAlgorithm, OpaqueCommitment, QuarantineEvent,
    QuarantineState, ResolutionDisposition,
};
use mycelix_fhir_medication_semantics::MedicationRequestArtifact;
use mycelix_medication_safety::MedicationSafetyClearance;
use mycelix_medication_safety_trust::SafetySourceTrustReceipt;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

const MAX_EVIDENCE_AGE_MICROS: i64 = 86_400_000_000;
const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPolicyV1 {
    pub schema_version: u16,
    /// Maximum age of the professional authority decision.
    pub max_authority_age_micros: i64,
    /// Maximum age of the external-practitioner -> Mycelix-principal resolution.
    pub max_requester_resolution_age_micros: i64,
    /// Maximum time between medication safety clearance and activation.
    pub max_safety_clearance_age_micros: i64,
    /// Maximum time between safety-source trust admission and activation.
    pub max_safety_source_trust_age_micros: i64,
    pub max_future_skew_micros: i64,
}

impl WorkflowPolicyV1 {
    pub fn validate(&self) -> Result<(), WorkflowError> {
        if self.schema_version != 1 {
            return Err(WorkflowError::UnsupportedWorkflowPolicyVersion(
                self.schema_version,
            ));
        }
        validate_age_policy(
            self.max_authority_age_micros,
            WorkflowError::InvalidAuthorityAgePolicy,
        )?;
        validate_age_policy(
            self.max_requester_resolution_age_micros,
            WorkflowError::InvalidRequesterResolutionAgePolicy,
        )?;
        validate_age_policy(
            self.max_safety_clearance_age_micros,
            WorkflowError::InvalidSafetyClearanceAgePolicy,
        )?;
        validate_age_policy(
            self.max_safety_source_trust_age_micros,
            WorkflowError::InvalidSafetySourceTrustAgePolicy,
        )?;
        if self.max_future_skew_micros < 0
            || self.max_future_skew_micros > MAX_FUTURE_SKEW_MICROS
        {
            return Err(WorkflowError::InvalidFutureSkewPolicy);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, WorkflowError> {
        self.validate()?;
        hash_json(
            DigestDomain::WorkflowPolicy,
            b"mycelix-health/workflow-policy-v1",
            self,
        )
    }
}

fn validate_age_policy(age: i64, error: WorkflowError) -> Result<(), WorkflowError> {
    if age <= 0 || age > MAX_EVIDENCE_AGE_MICROS {
        return Err(error);
    }
    Ok(())
}

pub struct ClinicalPresentationCapability {
    capsule_id: String,
    capsule_digest: StoredDigest,
    principal: PrincipalBinding,
    jurisdiction: JurisdictionCode,
    authority_policy_digest: StoredDigest,
    workflow_policy_digest: StoredDigest,
    authorized_at_micros: i64,
    supporting_authority_evidence: Vec<[u8; 32]>,
}

impl ClinicalPresentationCapability {
    pub fn capsule_id(&self) -> &str {
        &self.capsule_id
    }

    pub fn capsule_digest(&self) -> StoredDigest {
        self.capsule_digest
    }

    pub fn principal(&self) -> PrincipalBinding {
        self.principal
    }

    pub fn jurisdiction(&self) -> &JurisdictionCode {
        &self.jurisdiction
    }

    pub fn authority_policy_digest(&self) -> StoredDigest {
        self.authority_policy_digest
    }

    pub fn workflow_policy_digest(&self) -> StoredDigest {
        self.workflow_policy_digest
    }

    pub fn authorized_at_micros(&self) -> i64 {
        self.authorized_at_micros
    }

    pub fn supporting_authority_evidence(&self) -> &[[u8; 32]] {
        &self.supporting_authority_evidence
    }
}

/// Exact medication activation capability.
///
/// It preserves four independent proof lineages:
/// 1. requester -> authenticated principal resolution evidence;
/// 2. professional authority evidence supporting the prescribing action;
/// 3. patient-context/check evidence supporting medication safety;
/// 4. deployment trust evidence admitting the exact evaluator/knowledge sources.
///
/// The capability is intentionally non-cloneable/non-serializable.
pub struct MedicationActivationCapability {
    order_id: String,
    medication_artifact_digest: StoredDigest,
    principal: PrincipalBinding,
    requester_resolution_evidence: [StoredDigest; 3],
    requester_resolved_at_micros: i64,
    safety_context_digest: StoredDigest,
    safety_policy_digest: StoredDigest,
    safety_check_evaluation_digests: Vec<StoredDigest>,
    safety_cleared_at_micros: i64,
    safety_trust_policy_digest: StoredDigest,
    safety_trust_receipt_digest: StoredDigest,
    safety_evaluator_admission_evidence: Vec<StoredDigest>,
    safety_knowledge_admission_evidence: Vec<StoredDigest>,
    safety_trust_evaluated_at_micros: i64,
    jurisdiction: JurisdictionCode,
    authority_policy_digest: StoredDigest,
    workflow_policy_digest: StoredDigest,
    authorized_at_micros: i64,
    supporting_authority_evidence: Vec<[u8; 32]>,
}

impl MedicationActivationCapability {
    pub fn order_id(&self) -> &str {
        &self.order_id
    }

    pub fn medication_artifact_digest(&self) -> StoredDigest {
        self.medication_artifact_digest
    }

    pub fn principal(&self) -> PrincipalBinding {
        self.principal
    }

    pub fn requester_resolution_evidence(&self) -> &[StoredDigest; 3] {
        &self.requester_resolution_evidence
    }

    pub fn requester_resolved_at_micros(&self) -> i64 {
        self.requester_resolved_at_micros
    }

    pub fn safety_context_digest(&self) -> StoredDigest {
        self.safety_context_digest
    }

    pub fn safety_policy_digest(&self) -> StoredDigest {
        self.safety_policy_digest
    }

    pub fn safety_check_evaluation_digests(&self) -> &[StoredDigest] {
        &self.safety_check_evaluation_digests
    }

    pub fn safety_cleared_at_micros(&self) -> i64 {
        self.safety_cleared_at_micros
    }

    pub fn safety_trust_policy_digest(&self) -> StoredDigest {
        self.safety_trust_policy_digest
    }

    pub fn safety_trust_receipt_digest(&self) -> StoredDigest {
        self.safety_trust_receipt_digest
    }

    pub fn safety_evaluator_admission_evidence(&self) -> &[StoredDigest] {
        &self.safety_evaluator_admission_evidence
    }

    pub fn safety_knowledge_admission_evidence(&self) -> &[StoredDigest] {
        &self.safety_knowledge_admission_evidence
    }

    pub fn safety_trust_evaluated_at_micros(&self) -> i64 {
        self.safety_trust_evaluated_at_micros
    }

    pub fn jurisdiction(&self) -> &JurisdictionCode {
        &self.jurisdiction
    }

    pub fn authority_policy_digest(&self) -> StoredDigest {
        self.authority_policy_digest
    }

    pub fn workflow_policy_digest(&self) -> StoredDigest {
        self.workflow_policy_digest
    }

    pub fn authorized_at_micros(&self) -> i64 {
        self.authorized_at_micros
    }

    pub fn supporting_authority_evidence(&self) -> &[[u8; 32]] {
        &self.supporting_authority_evidence
    }
}

pub struct QuarantineResolutionCapability {
    case_nonce: [u8; 16],
    sequence: u32,
    event_digest: VerifiedDigest,
    disposition: ResolutionDisposition,
    principal: PrincipalBinding,
    authority_policy_digest: StoredDigest,
    workflow_policy_digest: StoredDigest,
    authorized_at_micros: i64,
}

impl QuarantineResolutionCapability {
    pub fn case_nonce(&self) -> [u8; 16] {
        self.case_nonce
    }

    pub fn sequence(&self) -> u32 {
        self.sequence
    }

    pub fn event_digest(&self) -> StoredDigest {
        self.event_digest.stored()
    }

    pub fn disposition(&self) -> ResolutionDisposition {
        self.disposition
    }

    pub fn principal(&self) -> PrincipalBinding {
        self.principal
    }

    pub fn authority_policy_digest(&self) -> StoredDigest {
        self.authority_policy_digest
    }

    pub fn workflow_policy_digest(&self) -> StoredDigest {
        self.workflow_policy_digest
    }

    pub fn authorized_at_micros(&self) -> i64 {
        self.authorized_at_micros
    }

    #[allow(clippy::too_many_arguments)]
    pub fn resolve(
        self,
        previous: &QuarantineEvent,
        producer_artifact_digest: ArtifactDigest,
        reviewer_authority_commitment: OpaqueCommitment,
        decision_artifact_digest: ArtifactDigest,
        promoted_artifact_digest: Option<ArtifactDigest>,
        recorded_at_micros: i64,
    ) -> Result<QuarantineEvent, WorkflowError> {
        if previous.case_nonce != self.case_nonce || previous.sequence != self.sequence {
            return Err(WorkflowError::QuarantineTargetChanged);
        }
        let actual = hash_quarantine_event(previous)?;
        if actual != self.event_digest {
            return Err(WorkflowError::TargetDigestMismatch);
        }

        QuarantineEvent::resolve(
            previous,
            ArtifactDigest {
                algorithm: QuarantineDigestAlgorithm::Blake3,
                value: self.event_digest.value(),
            },
            producer_artifact_digest,
            reviewer_authority_commitment,
            self.disposition,
            decision_artifact_digest,
            promoted_artifact_digest,
            recorded_at_micros,
        )
        .map_err(|error| WorkflowError::Quarantine(error.to_string()))
    }
}

pub fn authorize_clinical_presentation(
    capsule: &ClinicalEvidenceCapsule,
    authority: AuthorityPermit,
    authority_policy_digest: VerifiedDigest,
    workflow_policy: &WorkflowPolicyV1,
    authenticated_principal: PrincipalBinding,
    jurisdiction: &JurisdictionCode,
    execution_at_micros: i64,
) -> Result<ClinicalPresentationCapability, WorkflowError> {
    let capsule_digest = hash_evidence_capsule(capsule)?;
    let workflow_policy_digest = workflow_policy.verified_digest()?;
    let evidence = verify_authority_binding(
        &authority,
        capsule_digest,
        authority_policy_digest,
        AuthorityPurpose::ClinicalReview,
        authenticated_principal,
        jurisdiction,
        workflow_policy,
        execution_at_micros,
    )?;

    let gate = evaluate_for_clinical_presentation(capsule)
        .map_err(|error| WorkflowError::Qualification(error.to_string()))?;
    if gate.decision != GateDecision::EligibleForClinicalPresentation {
        return Err(WorkflowError::ClinicalQualificationDenied(gate.decision));
    }

    Ok(ClinicalPresentationCapability {
        capsule_id: capsule.capsule_id.clone(),
        capsule_digest: capsule_digest.stored(),
        principal: authenticated_principal,
        jurisdiction: jurisdiction.clone(),
        authority_policy_digest: authority_policy_digest.stored(),
        workflow_policy_digest: workflow_policy_digest.stored(),
        authorized_at_micros: execution_at_micros,
        supporting_authority_evidence: evidence,
    })
}

/// Convert one strict FHIR MedicationRequest artifact into an exact activation
/// capability. Both the safety clearance and source-trust receipt are consumed.
#[allow(clippy::too_many_arguments)]
pub fn authorize_resolved_medication_activation(
    artifact: &MedicationRequestArtifact,
    safety_clearance: MedicationSafetyClearance,
    safety_source_trust: SafetySourceTrustReceipt,
    expected_safety_policy_digest: VerifiedDigest,
    expected_safety_trust_policy_digest: VerifiedDigest,
    authority: AuthorityPermit,
    authority_policy_digest: VerifiedDigest,
    workflow_policy: &WorkflowPolicyV1,
    authenticated_principal: PrincipalBinding,
    jurisdiction: &JurisdictionCode,
    execution_at_micros: i64,
) -> Result<MedicationActivationCapability, WorkflowError> {
    workflow_policy.validate()?;
    artifact
        .validate_for_activation(execution_at_micros)
        .map_err(|error| WorkflowError::MedicationArtifact(error.to_string()))?;

    if artifact.requester_principal() != authenticated_principal {
        return Err(WorkflowError::RequesterPrincipalMismatch);
    }
    verify_freshness(
        artifact.requester_resolved_at_micros(),
        execution_at_micros,
        workflow_policy.max_requester_resolution_age_micros,
        workflow_policy.max_future_skew_micros,
        WorkflowError::RequesterResolutionFromFuture,
        WorkflowError::RequesterResolutionStale,
    )?;

    let artifact_digest = artifact
        .verified_digest()
        .map_err(|error| WorkflowError::MedicationArtifact(error.to_string()))?;
    artifact_digest.require_domain(DigestDomain::MedicationRequestArtifact)?;

    expected_safety_policy_digest.require_domain(DigestDomain::MedicationSafetyPolicy)?;
    if safety_clearance.medication_artifact_digest() != artifact_digest.stored() {
        return Err(WorkflowError::SafetyClearanceMedicationMismatch);
    }
    if safety_clearance.policy_digest() != expected_safety_policy_digest.stored() {
        return Err(WorkflowError::SafetyPolicyDigestMismatch);
    }

    let safety_context_digest = safety_clearance.context_digest();
    validate_stored_domain(safety_context_digest, DigestDomain::MedicationSafetyContext)?;
    let safety_check_evaluation_digests = safety_clearance.check_evaluation_digests().to_vec();
    if safety_check_evaluation_digests.is_empty() {
        return Err(WorkflowError::SafetyClearanceHasNoChecks);
    }
    for digest in &safety_check_evaluation_digests {
        validate_stored_domain(*digest, DigestDomain::MedicationSafetyEvaluation)?;
    }
    verify_freshness(
        safety_clearance.cleared_at_micros(),
        execution_at_micros,
        workflow_policy.max_safety_clearance_age_micros,
        workflow_policy.max_future_skew_micros,
        WorkflowError::SafetyClearanceFromFuture,
        WorkflowError::SafetyClearanceStale,
    )?;

    expected_safety_trust_policy_digest
        .require_domain(DigestDomain::MedicationSafetyTrustPolicy)?;
    if safety_source_trust.safety_policy_digest() != expected_safety_policy_digest.stored() {
        return Err(WorkflowError::SafetyTrustSafetyPolicyMismatch);
    }
    if safety_source_trust.trust_policy_digest()
        != expected_safety_trust_policy_digest.stored()
    {
        return Err(WorkflowError::SafetyTrustPolicyDigestMismatch);
    }
    if !same_digest_set(
        &safety_check_evaluation_digests,
        safety_source_trust.check_evaluation_digests(),
    ) {
        return Err(WorkflowError::SafetyTrustEvaluationSetMismatch);
    }

    let safety_trust_receipt_digest = safety_source_trust.receipt_digest();
    validate_stored_domain(
        safety_trust_receipt_digest,
        DigestDomain::MedicationSafetyTrustReceipt,
    )?;
    let safety_evaluator_admission_evidence =
        safety_source_trust.evaluator_admission_evidence_digests().to_vec();
    let safety_knowledge_admission_evidence =
        safety_source_trust.knowledge_admission_evidence_digests().to_vec();
    if safety_evaluator_admission_evidence.is_empty()
        || safety_knowledge_admission_evidence.is_empty()
    {
        return Err(WorkflowError::SafetyTrustHasNoAdmissionEvidence);
    }
    for digest in safety_evaluator_admission_evidence
        .iter()
        .chain(safety_knowledge_admission_evidence.iter())
    {
        validate_stored_domain(*digest, DigestDomain::ClinicalArtifact)?;
    }
    verify_freshness(
        safety_source_trust.evaluated_at_micros(),
        execution_at_micros,
        workflow_policy.max_safety_source_trust_age_micros,
        workflow_policy.max_future_skew_micros,
        WorkflowError::SafetySourceTrustFromFuture,
        WorkflowError::SafetySourceTrustStale,
    )?;

    let workflow_policy_digest = workflow_policy.verified_digest()?;
    let evidence = verify_authority_binding(
        &authority,
        artifact_digest,
        authority_policy_digest,
        AuthorityPurpose::Prescribe,
        authenticated_principal,
        jurisdiction,
        workflow_policy,
        execution_at_micros,
    )?;

    Ok(MedicationActivationCapability {
        order_id: artifact.order().order_id.clone(),
        medication_artifact_digest: artifact_digest.stored(),
        principal: authenticated_principal,
        requester_resolution_evidence: [
            artifact.requester_provider_record_digest(),
            artifact.requester_author_binding_evidence_digest(),
            artifact.requester_status_evidence_digest(),
        ],
        requester_resolved_at_micros: artifact.requester_resolved_at_micros(),
        safety_context_digest,
        safety_policy_digest: expected_safety_policy_digest.stored(),
        safety_check_evaluation_digests,
        safety_cleared_at_micros: safety_clearance.cleared_at_micros(),
        safety_trust_policy_digest: expected_safety_trust_policy_digest.stored(),
        safety_trust_receipt_digest,
        safety_evaluator_admission_evidence,
        safety_knowledge_admission_evidence,
        safety_trust_evaluated_at_micros: safety_source_trust.evaluated_at_micros(),
        jurisdiction: jurisdiction.clone(),
        authority_policy_digest: authority_policy_digest.stored(),
        workflow_policy_digest: workflow_policy_digest.stored(),
        authorized_at_micros: execution_at_micros,
        supporting_authority_evidence: evidence,
    })
}

pub fn authorize_quarantine_resolution(
    event: &QuarantineEvent,
    disposition: ResolutionDisposition,
    authority: AuthorityPermit,
    authority_policy_digest: VerifiedDigest,
    workflow_policy: &WorkflowPolicyV1,
    authenticated_principal: PrincipalBinding,
    jurisdiction: &JurisdictionCode,
    execution_at_micros: i64,
) -> Result<QuarantineResolutionCapability, WorkflowError> {
    if event.state != QuarantineState::UnderReview {
        return Err(WorkflowError::QuarantineNotUnderReview);
    }
    let event_digest = hash_quarantine_event(event)?;
    let workflow_policy_digest = workflow_policy.verified_digest()?;
    verify_authority_binding(
        &authority,
        event_digest,
        authority_policy_digest,
        AuthorityPurpose::ClinicalReview,
        authenticated_principal,
        jurisdiction,
        workflow_policy,
        execution_at_micros,
    )?;

    Ok(QuarantineResolutionCapability {
        case_nonce: event.case_nonce,
        sequence: event.sequence,
        event_digest,
        disposition,
        principal: authenticated_principal,
        authority_policy_digest: authority_policy_digest.stored(),
        workflow_policy_digest: workflow_policy_digest.stored(),
        authorized_at_micros: execution_at_micros,
    })
}

pub fn hash_evidence_capsule(
    capsule: &ClinicalEvidenceCapsule,
) -> Result<VerifiedDigest, WorkflowError> {
    hash_json(
        DigestDomain::EvidenceCapsule,
        b"mycelix-health/clinical-evidence-capsule-v1",
        capsule,
    )
}

pub fn hash_quarantine_event(event: &QuarantineEvent) -> Result<VerifiedDigest, WorkflowError> {
    hash_json(
        DigestDomain::QuarantineEvent,
        b"mycelix-health/quarantine-event-v1",
        event,
    )
}

fn hash_json<T: Serialize>(
    domain: DigestDomain,
    schema_tag: &[u8],
    value: &T,
) -> Result<VerifiedDigest, WorkflowError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| WorkflowError::CanonicalSerialization(error.to_string()))?;
    let mut framed = Vec::with_capacity(schema_tag.len() + 1 + encoded.len());
    framed.extend_from_slice(schema_tag);
    framed.push(0);
    framed.extend_from_slice(&encoded);
    Ok(hash_canonical_bytes(domain, &framed)?)
}

fn validate_stored_domain(
    digest: StoredDigest,
    expected: DigestDomain,
) -> Result<(), WorkflowError> {
    digest.validate_shape()?;
    if digest.domain != expected {
        return Err(WorkflowError::StoredEvidenceDomainMismatch {
            expected,
            actual: digest.domain,
        });
    }
    Ok(())
}

fn same_digest_set(left: &[StoredDigest], right: &[StoredDigest]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let left: HashSet<StoredDigest> = left.iter().copied().collect();
    let right: HashSet<StoredDigest> = right.iter().copied().collect();
    left.len() == right.len() && left == right
}

#[allow(clippy::too_many_arguments)]
fn verify_authority_binding(
    authority: &AuthorityPermit,
    target_digest: VerifiedDigest,
    authority_policy_digest: VerifiedDigest,
    expected_purpose: AuthorityPurpose,
    authenticated_principal: PrincipalBinding,
    jurisdiction: &JurisdictionCode,
    workflow_policy: &WorkflowPolicyV1,
    execution_at_micros: i64,
) -> Result<Vec<[u8; 32]>, WorkflowError> {
    authority_policy_digest.require_domain(DigestDomain::AuthorityPolicy)?;
    workflow_policy.validate()?;

    if authority.target_artifact_digest().0 != target_digest.value() {
        return Err(WorkflowError::TargetDigestMismatch);
    }
    if authority.policy_digest().0 != authority_policy_digest.value() {
        return Err(WorkflowError::AuthorityPolicyDigestMismatch);
    }
    if authority.purpose() != expected_purpose {
        return Err(WorkflowError::WrongAuthorityPurpose {
            expected: expected_purpose,
            actual: authority.purpose(),
        });
    }
    if authority.principal() != authenticated_principal {
        return Err(WorkflowError::PrincipalMismatch);
    }
    if authority.jurisdiction() != jurisdiction {
        return Err(WorkflowError::JurisdictionMismatch);
    }

    verify_freshness(
        authority.evaluated_at_micros(),
        execution_at_micros,
        workflow_policy.max_authority_age_micros,
        workflow_policy.max_future_skew_micros,
        WorkflowError::AuthorityEvaluationFromFuture,
        WorkflowError::AuthorityEvaluationStale,
    )?;

    Ok(authority
        .supporting_evidence_digests()
        .iter()
        .map(|digest: &EvidenceDigest| digest.0)
        .collect())
}

fn verify_freshness(
    evidence_at_micros: i64,
    execution_at_micros: i64,
    max_age_micros: i64,
    max_future_skew_micros: i64,
    future_error: WorkflowError,
    stale_error: WorkflowError,
) -> Result<(), WorkflowError> {
    let delta = execution_at_micros as i128 - evidence_at_micros as i128;
    if delta < -(max_future_skew_micros as i128) {
        return Err(future_error);
    }
    if delta > max_age_micros as i128 {
        return Err(stale_error);
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("unsupported workflow policy version {0}")]
    UnsupportedWorkflowPolicyVersion(u16),
    #[error("workflow authority age must be > 0 and <= 24 hours")]
    InvalidAuthorityAgePolicy,
    #[error("workflow requester-resolution age must be > 0 and <= 24 hours")]
    InvalidRequesterResolutionAgePolicy,
    #[error("workflow safety-clearance age must be > 0 and <= 24 hours")]
    InvalidSafetyClearanceAgePolicy,
    #[error("workflow safety-source-trust age must be > 0 and <= 24 hours")]
    InvalidSafetySourceTrustAgePolicy,
    #[error("workflow future-skew allowance must be between 0 and 5 minutes")]
    InvalidFutureSkewPolicy,
    #[error("failed to serialize exact artifact under its v1 canonical JSON contract: {0}")]
    CanonicalSerialization(String),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
    #[error("stored evidence digest domain mismatch: expected {expected:?}, got {actual:?}")]
    StoredEvidenceDomainMismatch {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("authority target digest does not match exact workflow artifact")]
    TargetDigestMismatch,
    #[error("authority policy digest does not match the verified policy identity")]
    AuthorityPolicyDigestMismatch,
    #[error("authority purpose mismatch: expected {expected:?}, got {actual:?}")]
    WrongAuthorityPurpose {
        expected: AuthorityPurpose,
        actual: AuthorityPurpose,
    },
    #[error("authority principal does not match authenticated workflow principal")]
    PrincipalMismatch,
    #[error("resolved MedicationRequest requester does not match authenticated principal")]
    RequesterPrincipalMismatch,
    #[error("authority jurisdiction does not match workflow jurisdiction")]
    JurisdictionMismatch,
    #[error("authority evaluation is too far in the future for workflow policy")]
    AuthorityEvaluationFromFuture,
    #[error("authority evaluation is stale for workflow policy")]
    AuthorityEvaluationStale,
    #[error("requester identity resolution is too far in the future for workflow policy")]
    RequesterResolutionFromFuture,
    #[error("requester identity resolution is stale for workflow policy")]
    RequesterResolutionStale,
    #[error("medication safety clearance targets a different medication artifact")]
    SafetyClearanceMedicationMismatch,
    #[error("medication safety clearance was produced under a different safety policy")]
    SafetyPolicyDigestMismatch,
    #[error("medication safety clearance contains no safety-check evidence")]
    SafetyClearanceHasNoChecks,
    #[error("medication safety clearance is too far in the future for workflow policy")]
    SafetyClearanceFromFuture,
    #[error("medication safety clearance is stale for workflow policy")]
    SafetyClearanceStale,
    #[error("safety-source trust receipt qualifies a different medication safety policy")]
    SafetyTrustSafetyPolicyMismatch,
    #[error("safety-source trust receipt was produced under a different trust policy")]
    SafetyTrustPolicyDigestMismatch,
    #[error("safety-source trust receipt does not cover the exact safety evaluation set")]
    SafetyTrustEvaluationSetMismatch,
    #[error("safety-source trust receipt contains no evaluator/knowledge admission evidence")]
    SafetyTrustHasNoAdmissionEvidence,
    #[error("safety-source trust receipt is too far in the future for workflow policy")]
    SafetySourceTrustFromFuture,
    #[error("safety-source trust receipt is stale for workflow policy")]
    SafetySourceTrustStale,
    #[error("clinical qualification gate denied presentation: {0:?}")]
    ClinicalQualificationDenied(GateDecision),
    #[error("clinical qualification validation failed: {0}")]
    Qualification(String),
    #[error("resolved medication artifact rejected activation: {0}")]
    MedicationArtifact(String),
    #[error("quarantine item must be under review before terminal resolution")]
    QuarantineNotUnderReview,
    #[error("quarantine target changed after capability issuance")]
    QuarantineTargetChanged,
    #[error("quarantine transition failed: {0}")]
    Quarantine(String),
}
