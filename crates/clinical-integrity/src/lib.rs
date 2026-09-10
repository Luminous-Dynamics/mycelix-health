#![deny(unsafe_code)]
//! Domain-separated integrity identities for Mycelix-Health clinical artifacts.
//!
//! Wire/storage digests are deliberately distinct from verified in-process digests.
//! A deserialized `StoredDigest` is only a claim. `VerifiedDigest` can be obtained
//! only by hashing canonical bytes locally or by re-verifying a stored claim against
//! the exact canonical bytes.

use serde::{Deserialize, Serialize};
use thiserror::Error;

const DERIVE_KEY_CONTEXT: &str = "mycelix.health.clinical-integrity.v1";
const FRAME_VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DigestAlgorithm {
    Blake3_256,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DigestDomain {
    ClinicalArtifact,
    EvidenceCapsule,
    MedicationOrder,
    MedicationRequestArtifact,
    MedicationSafetyContext,
    MedicationSafetyKnowledge,
    MedicationSafetyPolicy,
    MedicationSafetyEvaluation,
    MedicationSafetyAssessment,
    MedicationSafetyTrustPolicy,
    MedicationSafetyTrustReceipt,
    MedicationActivationReceipt,
    MedicationActivationAttestation,
    MedicationActivationState,
    EmergencyMedicationOverridePolicy,
    EmergencyMedicationOverrideReceipt,
    EmergencyMedicationOverrideAttestation,
    PharmacyRecord,
    PharmacyAffiliationEvidence,
    PharmacyStatusEvidence,
    MedicationDispensePolicy,
    MedicationDispenseRequest,
    MedicationDispenseLedgerSnapshot,
    MedicationDispenseReceipt,
    QuarantineEvent,
    QuarantineDecision,
    AuthorityPolicy,
    IssuerTrustPolicy,
    CredentialRecord,
    CredentialStatusEvidence,
    IssuerAuthorityEvidence,
    WorkflowPolicy,
}

impl DigestDomain {
    fn label(self) -> &'static [u8] {
        match self {
            Self::ClinicalArtifact => b"clinical-artifact",
            Self::EvidenceCapsule => b"evidence-capsule",
            Self::MedicationOrder => b"medication-order",
            Self::MedicationRequestArtifact => b"medication-request-artifact",
            Self::MedicationSafetyContext => b"medication-safety-context",
            Self::MedicationSafetyKnowledge => b"medication-safety-knowledge",
            Self::MedicationSafetyPolicy => b"medication-safety-policy",
            Self::MedicationSafetyEvaluation => b"medication-safety-evaluation",
            Self::MedicationSafetyAssessment => b"medication-safety-assessment",
            Self::MedicationSafetyTrustPolicy => b"medication-safety-trust-policy",
            Self::MedicationSafetyTrustReceipt => b"medication-safety-trust-receipt",
            Self::MedicationActivationReceipt => b"medication-activation-receipt",
            Self::MedicationActivationAttestation => b"medication-activation-attestation",
            Self::MedicationActivationState => b"medication-activation-state",
            Self::EmergencyMedicationOverridePolicy => b"emergency-medication-override-policy",
            Self::EmergencyMedicationOverrideReceipt => b"emergency-medication-override-receipt",
            Self::EmergencyMedicationOverrideAttestation => {
                b"emergency-medication-override-attestation"
            }
            Self::PharmacyRecord => b"pharmacy-record",
            Self::PharmacyAffiliationEvidence => b"pharmacy-affiliation-evidence",
            Self::PharmacyStatusEvidence => b"pharmacy-status-evidence",
            Self::MedicationDispensePolicy => b"medication-dispense-policy",
            Self::MedicationDispenseRequest => b"medication-dispense-request",
            Self::MedicationDispenseLedgerSnapshot => b"medication-dispense-ledger-snapshot",
            Self::MedicationDispenseReceipt => b"medication-dispense-receipt",
            Self::QuarantineEvent => b"quarantine-event",
            Self::QuarantineDecision => b"quarantine-decision",
            Self::AuthorityPolicy => b"authority-policy",
            Self::IssuerTrustPolicy => b"issuer-trust-policy",
            Self::CredentialRecord => b"credential-record",
            Self::CredentialStatusEvidence => b"credential-status-evidence",
            Self::IssuerAuthorityEvidence => b"issuer-authority-evidence",
            Self::WorkflowPolicy => b"workflow-policy",
        }
    }
}

/// Serializable digest claim. Deserialization does not confer verification.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct StoredDigest {
    pub algorithm: DigestAlgorithm,
    pub domain: DigestDomain,
    pub value: [u8; 32],
}

impl StoredDigest {
    pub fn validate_shape(&self) -> Result<(), IntegrityError> {
        if self.value == [0u8; 32] {
            return Err(IntegrityError::ZeroDigest);
        }
        Ok(())
    }
}

/// In-process proof that exact canonical bytes were hashed under the stated v1
/// algorithm/domain contract. Intentionally not serializable/deserializable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VerifiedDigest {
    stored: StoredDigest,
}

impl VerifiedDigest {
    pub fn stored(self) -> StoredDigest {
        self.stored
    }

    pub fn domain(self) -> DigestDomain {
        self.stored.domain
    }

    pub fn value(self) -> [u8; 32] {
        self.stored.value
    }

    pub fn require_domain(self, expected: DigestDomain) -> Result<Self, IntegrityError> {
        if self.domain() != expected {
            return Err(IntegrityError::WrongDomain {
                expected,
                actual: self.domain(),
            });
        }
        Ok(self)
    }
}

/// Hash bytes that are already in the owning artifact's canonical serialization.
pub fn hash_canonical_bytes(
    domain: DigestDomain,
    canonical_bytes: &[u8],
) -> Result<VerifiedDigest, IntegrityError> {
    if canonical_bytes.is_empty() {
        return Err(IntegrityError::EmptyCanonicalBytes);
    }

    let label = domain.label();
    let mut hasher = blake3::Hasher::new_derive_key(DERIVE_KEY_CONTEXT);
    hasher.update(&[FRAME_VERSION]);
    hasher.update(&(label.len() as u16).to_be_bytes());
    hasher.update(label);
    hasher.update(&(canonical_bytes.len() as u64).to_be_bytes());
    hasher.update(canonical_bytes);

    let value = *hasher.finalize().as_bytes();
    if value == [0u8; 32] {
        return Err(IntegrityError::ZeroDigest);
    }

    Ok(VerifiedDigest {
        stored: StoredDigest {
            algorithm: DigestAlgorithm::Blake3_256,
            domain,
            value,
        },
    })
}

pub fn verify_stored_digest(
    claimed: StoredDigest,
    canonical_bytes: &[u8],
) -> Result<VerifiedDigest, IntegrityError> {
    claimed.validate_shape()?;
    if claimed.algorithm != DigestAlgorithm::Blake3_256 {
        return Err(IntegrityError::UnsupportedAlgorithm);
    }
    let computed = hash_canonical_bytes(claimed.domain, canonical_bytes)?;
    if computed.stored != claimed {
        return Err(IntegrityError::DigestMismatch);
    }
    Ok(computed)
}

pub fn require_same_digest(
    left: VerifiedDigest,
    right: VerifiedDigest,
    domain: DigestDomain,
) -> Result<(), IntegrityError> {
    left.require_domain(domain)?;
    right.require_domain(domain)?;
    if left != right {
        return Err(IntegrityError::DigestMismatch);
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IntegrityError {
    #[error("canonical artifact bytes cannot be empty")]
    EmptyCanonicalBytes,
    #[error("digest value cannot be all zero")]
    ZeroDigest,
    #[error("unsupported digest algorithm")]
    UnsupportedAlgorithm,
    #[error("digest domain mismatch: expected {expected:?}, got {actual:?}")]
    WrongDomain {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("digest does not match canonical artifact bytes")]
    DigestMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_domain_and_bytes_are_deterministic() {
        let a = hash_canonical_bytes(DigestDomain::MedicationOrder, b"canonical-order-v1").unwrap();
        let b = hash_canonical_bytes(DigestDomain::MedicationOrder, b"canonical-order-v1").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn identical_bytes_in_different_domains_do_not_collide_by_construction() {
        let payload = b"same-canonical-bytes";
        let domains = [
            DigestDomain::MedicationOrder,
            DigestDomain::MedicationRequestArtifact,
            DigestDomain::MedicationSafetyContext,
            DigestDomain::MedicationSafetyPolicy,
            DigestDomain::MedicationSafetyKnowledge,
            DigestDomain::MedicationSafetyEvaluation,
            DigestDomain::MedicationSafetyAssessment,
            DigestDomain::MedicationSafetyTrustPolicy,
            DigestDomain::MedicationSafetyTrustReceipt,
            DigestDomain::MedicationActivationReceipt,
            DigestDomain::MedicationActivationAttestation,
            DigestDomain::MedicationActivationState,
            DigestDomain::EmergencyMedicationOverridePolicy,
            DigestDomain::EmergencyMedicationOverrideReceipt,
            DigestDomain::EmergencyMedicationOverrideAttestation,
            DigestDomain::PharmacyRecord,
            DigestDomain::PharmacyAffiliationEvidence,
            DigestDomain::PharmacyStatusEvidence,
            DigestDomain::MedicationDispensePolicy,
            DigestDomain::MedicationDispenseRequest,
            DigestDomain::MedicationDispenseLedgerSnapshot,
            DigestDomain::MedicationDispenseReceipt,
            DigestDomain::AuthorityPolicy,
        ];
        let values: Vec<[u8; 32]> = domains
            .iter()
            .map(|domain| hash_canonical_bytes(*domain, payload).unwrap().value())
            .collect();
        for (index, left) in values.iter().enumerate() {
            for right in values.iter().skip(index + 1) {
                assert_ne!(left, right);
            }
        }
    }

    #[test]
    fn framing_prevents_simple_concatenation_ambiguity() {
        let a = hash_canonical_bytes(DigestDomain::ClinicalArtifact, b"ab").unwrap();
        let b = hash_canonical_bytes(DigestDomain::ClinicalArtifact, b"a\0b").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn stored_claim_must_be_reverified() {
        let verified = hash_canonical_bytes(DigestDomain::EvidenceCapsule, b"capsule-v1").unwrap();
        assert_eq!(
            verify_stored_digest(verified.stored(), b"capsule-v1").unwrap(),
            verified
        );
        assert_eq!(
            verify_stored_digest(verified.stored(), b"different"),
            Err(IntegrityError::DigestMismatch)
        );
    }

    #[test]
    fn domain_requirement_fails_closed() {
        let digest = hash_canonical_bytes(DigestDomain::QuarantineEvent, b"event").unwrap();
        assert!(matches!(
            digest.require_domain(DigestDomain::MedicationOrder),
            Err(IntegrityError::WrongDomain { .. })
        ));
    }

    #[test]
    fn empty_input_is_not_a_valid_artifact_identity() {
        assert_eq!(
            hash_canonical_bytes(DigestDomain::ClinicalArtifact, b""),
            Err(IntegrityError::EmptyCanonicalBytes)
        );
    }
}
