#![deny(unsafe_code)]
//! Serializable audit receipt for a qualified medication activation.
//!
//! A `MedicationActivationCapability` is intentionally non-cloneable and
//! non-serializable. This crate consumes that single-owner capability into an audit
//! receipt that may be persisted or attested. The receipt is evidence, not authority:
//! deserializing it does not recreate the capability and does not authorize another
//! activation.

use mycelix_clinical_authority::JurisdictionCode;
use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use mycelix_clinical_workflow::MedicationActivationCapability;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const RECEIPT_SCHEMA_TAG: &[u8] = b"mycelix-health/medication-activation-audit-receipt-v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MedicationActivationAuditReceiptV1 {
    pub schema_version: u16,
    pub order_id: String,
    pub medication_artifact_digest: StoredDigest,
    pub principal: [u8; 32],
    pub requester_resolution_evidence: [StoredDigest; 3],
    pub requester_resolved_at_micros: i64,
    pub safety_context_digest: StoredDigest,
    pub safety_policy_digest: StoredDigest,
    pub safety_check_evaluation_digests: Vec<StoredDigest>,
    pub safety_cleared_at_micros: i64,
    pub safety_trust_policy_digest: StoredDigest,
    pub safety_trust_receipt_digest: StoredDigest,
    pub safety_evaluator_admission_evidence: Vec<StoredDigest>,
    pub safety_knowledge_admission_evidence: Vec<StoredDigest>,
    pub safety_trust_evaluated_at_micros: i64,
    pub jurisdiction: JurisdictionCode,
    pub authority_policy_digest: StoredDigest,
    pub workflow_policy_digest: StoredDigest,
    pub authorized_at_micros: i64,
    pub supporting_authority_evidence: Vec<[u8; 32]>,
}

impl MedicationActivationAuditReceiptV1 {
    /// Validate the serialized receipt as a well-formed audit artifact. This does
    /// not recreate workflow authority; it only establishes internal shape/domain
    /// consistency before hashing or persistence.
    pub fn validate(&self) -> Result<(), ActivationReceiptError> {
        if self.schema_version != 1 {
            return Err(ActivationReceiptError::UnsupportedVersion(self.schema_version));
        }
        if self.order_id.trim().is_empty() {
            return Err(ActivationReceiptError::MissingOrderId);
        }
        if self.principal == [0u8; 32] {
            return Err(ActivationReceiptError::ZeroPrincipal);
        }
        if self.jurisdiction.system.trim().is_empty() || self.jurisdiction.code.trim().is_empty() {
            return Err(ActivationReceiptError::InvalidJurisdiction);
        }

        require_domain(
            self.medication_artifact_digest,
            DigestDomain::MedicationRequestArtifact,
        )?;
        for digest in self.requester_resolution_evidence {
            require_domain(digest, DigestDomain::ClinicalArtifact)?;
        }
        require_domain(
            self.safety_context_digest,
            DigestDomain::MedicationSafetyContext,
        )?;
        require_domain(
            self.safety_policy_digest,
            DigestDomain::MedicationSafetyPolicy,
        )?;
        require_nonempty_domains(
            &self.safety_check_evaluation_digests,
            DigestDomain::MedicationSafetyEvaluation,
            ActivationReceiptError::MissingSafetyEvaluations,
        )?;
        require_domain(
            self.safety_trust_policy_digest,
            DigestDomain::MedicationSafetyTrustPolicy,
        )?;
        require_domain(
            self.safety_trust_receipt_digest,
            DigestDomain::MedicationSafetyTrustReceipt,
        )?;
        require_nonempty_domains(
            &self.safety_evaluator_admission_evidence,
            DigestDomain::ClinicalArtifact,
            ActivationReceiptError::MissingEvaluatorAdmissionEvidence,
        )?;
        require_nonempty_domains(
            &self.safety_knowledge_admission_evidence,
            DigestDomain::ClinicalArtifact,
            ActivationReceiptError::MissingKnowledgeAdmissionEvidence,
        )?;
        require_domain(self.authority_policy_digest, DigestDomain::AuthorityPolicy)?;
        require_domain(self.workflow_policy_digest, DigestDomain::WorkflowPolicy)?;

        if self.supporting_authority_evidence.is_empty()
            || self
                .supporting_authority_evidence
                .iter()
                .any(|digest| *digest == [0u8; 32])
        {
            return Err(ActivationReceiptError::MissingAuthorityEvidence);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, ActivationReceiptError> {
        self.validate()?;
        let encoded = serde_json::to_vec(self)
            .map_err(|error| ActivationReceiptError::Serialization(error.to_string()))?;
        let mut framed = Vec::with_capacity(RECEIPT_SCHEMA_TAG.len() + 1 + encoded.len());
        framed.extend_from_slice(RECEIPT_SCHEMA_TAG);
        framed.push(0);
        framed.extend_from_slice(&encoded);
        Ok(hash_canonical_bytes(
            DigestDomain::MedicationActivationReceipt,
            &framed,
        )?)
    }
}

/// Consume the single-owner workflow capability into a replay-safe audit artifact.
/// The resulting receipt may be copied for audit, but it cannot be converted back
/// into `MedicationActivationCapability` through this API.
pub fn consume_activation_capability(
    capability: MedicationActivationCapability,
) -> Result<MedicationActivationAuditReceiptV1, ActivationReceiptError> {
    let receipt = MedicationActivationAuditReceiptV1 {
        schema_version: 1,
        order_id: capability.order_id().to_string(),
        medication_artifact_digest: capability.medication_artifact_digest(),
        principal: capability.principal().0,
        requester_resolution_evidence: *capability.requester_resolution_evidence(),
        requester_resolved_at_micros: capability.requester_resolved_at_micros(),
        safety_context_digest: capability.safety_context_digest(),
        safety_policy_digest: capability.safety_policy_digest(),
        safety_check_evaluation_digests: capability.safety_check_evaluation_digests().to_vec(),
        safety_cleared_at_micros: capability.safety_cleared_at_micros(),
        safety_trust_policy_digest: capability.safety_trust_policy_digest(),
        safety_trust_receipt_digest: capability.safety_trust_receipt_digest(),
        safety_evaluator_admission_evidence: capability
            .safety_evaluator_admission_evidence()
            .to_vec(),
        safety_knowledge_admission_evidence: capability
            .safety_knowledge_admission_evidence()
            .to_vec(),
        safety_trust_evaluated_at_micros: capability.safety_trust_evaluated_at_micros(),
        jurisdiction: capability.jurisdiction().clone(),
        authority_policy_digest: capability.authority_policy_digest(),
        workflow_policy_digest: capability.workflow_policy_digest(),
        authorized_at_micros: capability.authorized_at_micros(),
        supporting_authority_evidence: capability.supporting_authority_evidence().to_vec(),
    };
    receipt.validate()?;
    Ok(receipt)
}

fn require_domain(
    digest: StoredDigest,
    expected: DigestDomain,
) -> Result<(), ActivationReceiptError> {
    digest.validate_shape()?;
    if digest.domain != expected {
        return Err(ActivationReceiptError::DigestDomainMismatch {
            expected,
            actual: digest.domain,
        });
    }
    Ok(())
}

fn require_nonempty_domains(
    digests: &[StoredDigest],
    expected: DigestDomain,
    empty_error: ActivationReceiptError,
) -> Result<(), ActivationReceiptError> {
    if digests.is_empty() {
        return Err(empty_error);
    }
    for digest in digests {
        require_domain(*digest, expected)?;
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum ActivationReceiptError {
    #[error("unsupported medication activation receipt version {0}")]
    UnsupportedVersion(u16),
    #[error("medication activation receipt requires an order id")]
    MissingOrderId,
    #[error("medication activation receipt principal cannot be zero")]
    ZeroPrincipal,
    #[error("medication activation receipt jurisdiction is invalid")]
    InvalidJurisdiction,
    #[error("medication activation receipt has no safety evaluations")]
    MissingSafetyEvaluations,
    #[error("medication activation receipt has no evaluator admission evidence")]
    MissingEvaluatorAdmissionEvidence,
    #[error("medication activation receipt has no knowledge admission evidence")]
    MissingKnowledgeAdmissionEvidence,
    #[error("medication activation receipt has no professional authority evidence")]
    MissingAuthorityEvidence,
    #[error("digest domain mismatch: expected {expected:?}, got {actual:?}")]
    DigestDomainMismatch {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("failed to serialize medication activation receipt: {0}")]
    Serialization(String),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_integrity::{hash_canonical_bytes, DigestDomain};

    fn stored(domain: DigestDomain, seed: u8) -> StoredDigest {
        hash_canonical_bytes(domain, &[seed]).unwrap().stored()
    }

    fn receipt() -> MedicationActivationAuditReceiptV1 {
        MedicationActivationAuditReceiptV1 {
            schema_version: 1,
            order_id: "order-a".into(),
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            principal: [2; 32],
            requester_resolution_evidence: [
                stored(DigestDomain::ClinicalArtifact, 3),
                stored(DigestDomain::ClinicalArtifact, 4),
                stored(DigestDomain::ClinicalArtifact, 5),
            ],
            requester_resolved_at_micros: 100,
            safety_context_digest: stored(DigestDomain::MedicationSafetyContext, 6),
            safety_policy_digest: stored(DigestDomain::MedicationSafetyPolicy, 7),
            safety_check_evaluation_digests: vec![stored(
                DigestDomain::MedicationSafetyEvaluation,
                8,
            )],
            safety_cleared_at_micros: 120,
            safety_trust_policy_digest: stored(DigestDomain::MedicationSafetyTrustPolicy, 9),
            safety_trust_receipt_digest: stored(DigestDomain::MedicationSafetyTrustReceipt, 10),
            safety_evaluator_admission_evidence: vec![stored(DigestDomain::ClinicalArtifact, 11)],
            safety_knowledge_admission_evidence: vec![stored(DigestDomain::ClinicalArtifact, 12)],
            safety_trust_evaluated_at_micros: 125,
            jurisdiction: JurisdictionCode {
                system: "urn:iso:std:iso:3166-2".into(),
                code: "US-TX".into(),
            },
            authority_policy_digest: stored(DigestDomain::AuthorityPolicy, 13),
            workflow_policy_digest: stored(DigestDomain::WorkflowPolicy, 14),
            authorized_at_micros: 130,
            supporting_authority_evidence: vec![[15; 32]],
        }
    }

    #[test]
    fn well_formed_receipt_has_activation_specific_identity() {
        let receipt = receipt();
        let digest = receipt.verified_digest().unwrap();
        assert_eq!(digest.domain(), DigestDomain::MedicationActivationReceipt);
    }

    #[test]
    fn wrong_safety_policy_domain_fails_closed() {
        let mut receipt = receipt();
        receipt.safety_policy_digest = stored(DigestDomain::AuthorityPolicy, 7);
        assert!(matches!(
            receipt.validate(),
            Err(ActivationReceiptError::DigestDomainMismatch { .. })
        ));
    }

    #[test]
    fn missing_source_admission_evidence_is_invalid() {
        let mut receipt = receipt();
        receipt.safety_knowledge_admission_evidence.clear();
        assert!(matches!(
            receipt.validate(),
            Err(ActivationReceiptError::MissingKnowledgeAdmissionEvidence)
        ));
    }
}
