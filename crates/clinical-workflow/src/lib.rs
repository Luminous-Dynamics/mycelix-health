#![deny(unsafe_code)]
//! Exact-target capability composition for Mycelix-Health.
//!
//! A valid credential or qualification result is not sufficient on its own. This
//! crate consumes a verifier-owned `AuthorityPermit` and rebinds it to one exact
//! clinical artifact, purpose, policy, principal, jurisdiction, and execution time.
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
use mycelix_medication_semantics::MedicationOrder;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_AUTHORITY_AGE_MICROS: i64 = 86_400_000_000;
const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPolicyV1 {
    pub schema_version: u16,
    pub max_authority_age_micros: i64,
    pub max_future_skew_micros: i64,
}

impl WorkflowPolicyV1 {
    pub fn validate(&self) -> Result<(), WorkflowError> {
        if self.schema_version != 1 {
            return Err(WorkflowError::UnsupportedWorkflowPolicyVersion(
                self.schema_version,
            ));
        }
        if self.max_authority_age_micros <= 0
            || self.max_authority_age_micros > MAX_AUTHORITY_AGE_MICROS
        {
            return Err(WorkflowError::InvalidAuthorityAgePolicy);
        }
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

pub struct MedicationActivationCapability {
    order_id: String,
    order_digest: StoredDigest,
    principal: PrincipalBinding,
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

    pub fn order_digest(&self) -> StoredDigest {
        self.order_digest
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

pub fn authorize_medication_activation(
    order: &MedicationOrder,
    authority: AuthorityPermit,
    authority_policy_digest: VerifiedDigest,
    workflow_policy: &WorkflowPolicyV1,
    authenticated_principal: PrincipalBinding,
    jurisdiction: &JurisdictionCode,
    execution_at_micros: i64,
) -> Result<MedicationActivationCapability, WorkflowError> {
    order
        .validate_active_order_candidate()
        .map_err(|error| WorkflowError::Medication(error.to_string()))?;

    let order_digest = hash_medication_order(order)?;
    let workflow_policy_digest = workflow_policy.verified_digest()?;
    let evidence = verify_authority_binding(
        &authority,
        order_digest,
        authority_policy_digest,
        AuthorityPurpose::Prescribe,
        authenticated_principal,
        jurisdiction,
        workflow_policy,
        execution_at_micros,
    )?;

    Ok(MedicationActivationCapability {
        order_id: order.order_id.clone(),
        order_digest: order_digest.stored(),
        principal: authenticated_principal,
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

pub fn hash_medication_order(order: &MedicationOrder) -> Result<VerifiedDigest, WorkflowError> {
    hash_json(
        DigestDomain::MedicationOrder,
        b"mycelix-health/medication-order-v1",
        order,
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

    let delta = execution_at_micros as i128 - authority.evaluated_at_micros() as i128;
    if delta < -(workflow_policy.max_future_skew_micros as i128) {
        return Err(WorkflowError::AuthorityEvaluationFromFuture);
    }
    if delta > workflow_policy.max_authority_age_micros as i128 {
        return Err(WorkflowError::AuthorityEvaluationStale);
    }

    Ok(authority
        .supporting_evidence_digests()
        .iter()
        .map(|digest: &EvidenceDigest| digest.0)
        .collect())
}

#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("unsupported workflow policy version {0}")]
    UnsupportedWorkflowPolicyVersion(u16),
    #[error("workflow authority age must be > 0 and <= 24 hours")]
    InvalidAuthorityAgePolicy,
    #[error("workflow future-skew allowance must be between 0 and 5 minutes")]
    InvalidFutureSkewPolicy,
    #[error("failed to serialize exact artifact under its v1 canonical JSON contract: {0}")]
    CanonicalSerialization(String),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
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
    #[error("authority jurisdiction does not match workflow jurisdiction")]
    JurisdictionMismatch,
    #[error("authority evaluation is too far in the future for workflow policy")]
    AuthorityEvaluationFromFuture,
    #[error("authority evaluation is stale for workflow policy")]
    AuthorityEvaluationStale,
    #[error("clinical qualification gate denied presentation: {0:?}")]
    ClinicalQualificationDenied(GateDecision),
    #[error("clinical qualification validation failed: {0}")]
    Qualification(String),
    #[error("medication semantics rejected activation: {0}")]
    Medication(String),
    #[error("quarantine item must be under review before terminal resolution")]
    QuarantineNotUnderReview,
    #[error("quarantine target changed after capability issuance")]
    QuarantineTargetChanged,
    #[error("quarantine transition failed: {0}")]
    Quarantine(String),
}
