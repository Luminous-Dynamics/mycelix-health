// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Zero-knowledge health attestations.
//!
//! Enables patients to prove health properties WITHOUT revealing health data:
//! - "I qualify for life insurance" → proves all vitals in range, no diagnoses
//! - "I'm eligible for clinical trial X" → proves 7/7 criteria met
//! - "I passed employment physical" → proves clearance without showing records
//! - "I'm vaccinated against COVID-19" → proves status without revealing brand/date
//!
//! ## Architecture (DASTARK dual-backend)
//!
//! - Simple proofs (VitalsInRange, AgeRange, LabThreshold): **Winterfell STARK** AIR circuits
//! - Complex proofs (TrialEligibility, OrganDonorCompatibility): **RISC0 zkVM** guest programs
//! - All proofs wrapped in `AuthenticatedProof` with Dilithium5 PQ signatures
//! - Domain tag: `ZTML:Health:RecordAttest:v1`
//!
//! ## Proof flow
//!
//! 1. Client generates proof (Winterfell or RISC0, depending on complexity)
//! 2. Client signs with Dilithium5 → `AuthenticatedProof`
//! 3. Zome receives `HealthProof` containing `AuthenticatedProof`
//! 4. A production verifier calls `verify_proof()` and rejects unsupported or
//!    unbound proof systems by default
//! 5. Only a fully verified attestation may be stored or used for policy
//!
//! ## Security status
//!
//! The current Winterfell range AIR is cryptographic but still experimental:
//! it does not bind the hidden range value to `data_commitment`. Consequently,
//! default verification rejects it. The opt-in
//! `experimental-unbound-range-proofs` feature exists only for research and
//! benchmark continuity and must not gate clinical or financial decisions.

pub mod circuits;
pub mod prover;
pub mod verifier;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// Re-export from mycelix-zkp-core
pub use mycelix_zkp_core::domain::{tag_health_attest, DomainTag};
pub use mycelix_zkp_core::error::ZkpError;
pub use mycelix_zkp_core::types::{AuthenticatedProof, BackendId};

/// Types of health attestations that can be proven in zero knowledge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HealthProofType {
    /// All vitals within normal reference ranges.
    VitalsInRange,
    /// No active diagnosis matching ICD-10 codes in the exclusion set.
    ConditionAbsence { excluded_icd10: Vec<String> },
    /// Vaccination status for a specific disease.
    VaccinationStatus { disease: String },
    /// Lab value above/below a threshold (e.g., "A1C < 7.0").
    LabThreshold {
        loinc_code: String,
        threshold: f64,
        direction: ThresholdDirection,
    },
    /// Age within a range (e.g., "18-65").
    AgeRange { min_age: u32, max_age: Option<u32> },
    /// Clinical trial eligibility (meets N of M criteria).
    TrialEligibility {
        trial_id: String,
        criteria_met: u32,
        criteria_total: u32,
    },
    /// Employment physical clearance.
    EmploymentPhysical,
    /// Insurance qualification tier.
    InsuranceQualification { tier: String },
    /// Substance screening clear (42 CFR Part 2 protected).
    SubstanceScreening,
    /// Organ donor compatibility.
    OrganDonorCompatibility { recipient_hla_hash: [u8; 32] },
    /// Custom attestation with description.
    Custom { description: String },
}

impl HealthProofType {
    /// Determine which backend is best for this proof type.
    pub fn recommended_backend(&self) -> BackendId {
        match self {
            // Simple range/threshold proofs → Winterfell (3-10x faster)
            HealthProofType::VitalsInRange
            | HealthProofType::LabThreshold { .. }
            | HealthProofType::AgeRange { .. }
            | HealthProofType::VaccinationStatus { .. }
            | HealthProofType::EmploymentPhysical
            | HealthProofType::SubstanceScreening => BackendId::Winterfell,

            // Complex multi-condition proofs → RISC0 (arbitrary Rust logic)
            HealthProofType::TrialEligibility { .. }
            | HealthProofType::OrganDonorCompatibility { .. }
            | HealthProofType::ConditionAbsence { .. }
            | HealthProofType::InsuranceQualification { .. }
            | HealthProofType::Custom { .. } => BackendId::Risc0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ThresholdDirection {
    Below, // value < threshold
    Above, // value > threshold
}

/// Who attested to the underlying health data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AttestorRole {
    Physician,
    NursePractitioner,
    Laboratory,
    HealthcareOrganization,
    PublicHealthAuthority,
    ClinicalTrialSponsor,
    PatientSelf,
}

/// A zero-knowledge health proof.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthProof {
    /// What is being proven.
    pub proof_type: HealthProofType,
    /// The proof bytes (STARK proof or RISC0 receipt).
    pub proof_bytes: Vec<u8>,
    /// Public inputs (what the verifier can see).
    pub public_inputs: HealthPublicInputs,
    /// Proof metadata.
    pub metadata: ProofMetadata,
}

/// Public inputs visible to the verifier (no private health data).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthPublicInputs {
    /// Hash of the patient's identity (not the identity itself).
    pub patient_id_hash: [u8; 32],
    /// Commitment to the health data used (hash of the data).
    pub data_commitment: [u8; 32],
    /// Whether the criteria are met (the claim being proven).
    pub criteria_met: bool,
    /// Timestamp of the underlying health data.
    pub data_timestamp: i64,
    /// Attestor role (who verified the underlying data).
    pub attestor_role: AttestorRole,
    /// Minimum value of the proven range (public).
    pub min_value: u64,
    /// Maximum value of the proven range (public).
    pub max_value: u64,
}

/// Proof metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofMetadata {
    /// Proof system used.
    pub system: ProofSystem,
    /// Security level in bits.
    pub security_bits: u32,
    /// Whether this proof is post-quantum secure.
    pub post_quantum: bool,
    /// When the proof was generated.
    pub generated_at: i64,
    /// When the proof expires (health data may change).
    pub expires_at: i64,
    /// Size of the proof in bytes.
    pub proof_size: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofSystem {
    /// Winterfell STARK (simple range/threshold proofs).
    WinterfellStark,
    /// RISC0 zkVM (complex multi-condition proofs).
    Risc0ZkVm,
    /// SHA-256 commitment (legacy, not true ZK — backward compat only).
    Sha256Commitment,
}

impl ProofSystem {
    /// Map to the mycelix-zkp-core BackendId.
    pub fn backend_id(&self) -> Option<BackendId> {
        match self {
            ProofSystem::WinterfellStark => Some(BackendId::Winterfell),
            ProofSystem::Risc0ZkVm => Some(BackendId::Risc0),
            ProofSystem::Sha256Commitment => None, // Legacy, no real backend
        }
    }
}

/// Generate a health proof.
///
/// For simple proofs, uses Winterfell STARK AIR circuits.
/// For complex proofs, uses RISC0 zkVM guest programs.
/// Emits a domain-tagged SHA-256 commitment placeholder when a proof backend
/// is not available. Commitment placeholders are never accepted by
/// [`verify_proof`].
///
/// In production, proof generation happens CLIENT-SIDE (portal or mobile app).
/// This function is the reference implementation.
pub fn generate_proof(
    proof_type: HealthProofType,
    private_health_data: &[u8],
    patient_id: &[u8],
    attestor: AttestorRole,
    timestamp: i64,
    expiry: i64,
    value: u64,
    min_value: u64,
    max_value: u64,
) -> HealthProof {
    // Commitment to private data (verifier can check this matches)
    let data_commitment = {
        let mut hasher = Sha256::new();
        hasher.update(private_health_data);
        hasher.update(timestamp.to_le_bytes());
        let result = hasher.finalize();
        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&result);
        commitment
    };

    // Patient identity hash
    let patient_id_hash = {
        let hash = Sha256::digest(patient_id);
        let mut id_hash = [0u8; 32];
        id_hash.copy_from_slice(&hash);
        id_hash
    };

    let recommended = proof_type.recommended_backend();

    // Determine proof system: try an experimental backend, otherwise emit a
    // non-verifiable commitment placeholder for migration/debugging only.
    let (proof_bytes, system) = generate_proof_bytes(
        recommended,
        &data_commitment,
        &patient_id_hash,
        &proof_type,
        value,
        min_value,
        max_value,
    );

    HealthProof {
        proof_type,
        proof_bytes: proof_bytes.clone(),
        public_inputs: HealthPublicInputs {
            patient_id_hash,
            data_commitment,
            criteria_met: value >= min_value && value <= max_value,
            data_timestamp: timestamp,
            attestor_role: attestor,
            min_value,
            max_value,
        },
        metadata: ProofMetadata {
            system,
            security_bits: match system {
                ProofSystem::WinterfellStark => 96,
                ProofSystem::Risc0ZkVm => 0, // No health guest verifier is wired.
                ProofSystem::Sha256Commitment => 0, // Commitment is not a proof.
            },
            post_quantum: matches!(system, ProofSystem::WinterfellStark),
            generated_at: timestamp,
            expires_at: expiry,
            proof_size: proof_bytes.len(),
        },
    }
}

/// Internal: generate proof bytes using the recommended backend.
///
/// Default builds emit only a non-verifiable commitment placeholder. The
/// explicit research feature enables the current unbound Winterfell range AIR.
/// Complex RISC0 proof types remain unsupported.
fn generate_proof_bytes(
    recommended: BackendId,
    data_commitment: &[u8; 32],
    patient_id_hash: &[u8; 32],
    proof_type: &HealthProofType,
    value: u64,
    min_value: u64,
    max_value: u64,
) -> (Vec<u8>, ProofSystem) {
    // The current range AIR is available only as an explicit research opt-in.
    // Default builds do not even emit an unbound STARK that another component
    // might accidentally treat as production evidence.
    #[cfg(feature = "experimental-unbound-range-proofs")]
    if recommended == BackendId::Winterfell && value >= min_value && value <= max_value {
        if let Ok(proof) =
            circuits::range_proof::prove_range(value, min_value, max_value, *data_commitment)
        {
            return (proof.to_bytes(), ProofSystem::WinterfellStark);
        }
    }

    #[cfg(not(feature = "experimental-unbound-range-proofs"))]
    let _ = (recommended, value, min_value, max_value);

    // Fallback: domain-tagged commitment placeholder. This is retained only so
    // callers can migrate without silently receiving an accepted proof.
    // Verification always rejects this system.
    let domain_tag = tag_health_attest();
    let mut proof_hasher = Sha256::new();
    proof_hasher.update(domain_tag.as_bytes());
    proof_hasher.update(data_commitment);
    proof_hasher.update(patient_id_hash);
    proof_hasher.update(format!("{:?}", proof_type).as_bytes());
    let proof_hash = proof_hasher.finalize();

    (proof_hash.to_vec(), ProofSystem::Sha256Commitment)
}

/// Verify a health proof using the current wall-clock time.
///
/// Verification is fail-closed:
/// - expired or internally inconsistent metadata is rejected;
/// - commitment placeholders are rejected;
/// - RISC0 receipts are rejected until a health guest image and journal binding
///   are implemented;
/// - the current unbound Winterfell range AIR is rejected unless the explicitly
///   experimental feature is enabled.
pub fn verify_proof(proof: &HealthProof) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    verify_proof_at(proof, now)
}

/// Deterministic verification entry point for tests and policy engines that
/// already have a trusted timestamp.
pub fn verify_proof_at(proof: &HealthProof, now: i64) -> bool {
    if proof.metadata.proof_size != proof.proof_bytes.len()
        || proof.metadata.generated_at <= 0
        || proof.metadata.expires_at <= proof.metadata.generated_at
        || now < proof.metadata.generated_at
        || now > proof.metadata.expires_at
        || proof.public_inputs.data_timestamp > proof.metadata.generated_at
        || proof.public_inputs.data_timestamp > now
    {
        return false;
    }

    match proof.metadata.system {
        ProofSystem::WinterfellStark => {
            if !proof.metadata.post_quantum || proof.metadata.security_bits != 96 {
                return false;
            }
            verify_winterfell_proof(proof)
        }
        ProofSystem::Risc0ZkVm => {
            if proof.metadata.post_quantum || proof.metadata.security_bits != 0 {
                return false;
            }
            verify_risc0_proof(proof)
        }
        ProofSystem::Sha256Commitment => {
            if proof.metadata.post_quantum || proof.metadata.security_bits != 0 {
                return false;
            }
            verify_commitment_proof(proof)
        }
    }
}

/// Verify the experimental Winterfell range proof.
///
/// The actual Winterfell proof is always deserialized and cryptographically
/// checked when the research feature is enabled. Without that feature this
/// returns false rather than degrading to metadata inspection.
fn verify_winterfell_proof(proof: &HealthProof) -> bool {
    #[cfg(not(feature = "experimental-unbound-range-proofs"))]
    {
        let _ = proof;
        return false;
    }

    #[cfg(feature = "experimental-unbound-range-proofs")]
    {
        use winterfell::Proof;

        if proof.proof_bytes.is_empty()
            || !proof.public_inputs.criteria_met
            || proof.metadata.security_bits != 96
        {
            return false;
        }

        let stark_proof = match Proof::from_bytes(&proof.proof_bytes) {
            Ok(proof) => proof,
            Err(_) => return false,
        };

        circuits::range_proof::verify_range(
            stark_proof,
            proof.public_inputs.min_value,
            proof.public_inputs.max_value,
            proof.public_inputs.data_commitment,
        )
        .is_ok()
    }
}

/// RISC0 health verification is not implemented. Merely having receipt bytes,
/// claimed security bits, or `criteria_met = true` is never authorization.
fn verify_risc0_proof(proof: &HealthProof) -> bool {
    let _ = proof;
    false
}

/// SHA-256 commitments provide data integrity, not zero-knowledge statement
/// verification. They remain parseable for migration but are never accepted.
fn verify_commitment_proof(proof: &HealthProof) -> bool {
    let _ = proof;
    false
}

/// Verify a proof's domain tag matches the health attestation tag.
pub fn verify_domain_tag(proof: &HealthProof, expected_domain: &DomainTag) -> bool {
    // The proof bytes should be generated with the correct domain tag.
    // Re-compute the expected commitment and compare.
    let domain_tag = expected_domain;
    let mut hasher = Sha256::new();
    hasher.update(domain_tag.as_bytes());
    hasher.update(&proof.public_inputs.data_commitment);
    hasher.update(&proof.public_inputs.patient_id_hash);
    hasher.update(format!("{:?}", proof.proof_type).as_bytes());
    let expected = hasher.finalize();

    proof.proof_bytes == expected.as_slice()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_verify_insurance_proof() {
        let proof = generate_proof(
            HealthProofType::InsuranceQualification {
                tier: "preferred".into(),
            },
            b"vitals:normal,a1c:5.4,bmi:22,no_tobacco",
            b"patient-001",
            AttestorRole::Physician,
            1000000,
            2000000,
            0,
            0,
            0, // Insurance: complex type, no range
        );

        assert!(proof.public_inputs.criteria_met);
        assert!(!proof.metadata.post_quantum);
        assert!(!proof.proof_bytes.is_empty());
        assert_eq!(proof.metadata.system, ProofSystem::Sha256Commitment);
        assert!(!verify_proof_at(&proof, 1_500_000));
    }

    #[test]
    fn generate_trial_eligibility_proof() {
        let proof = generate_proof(
            HealthProofType::TrialEligibility {
                trial_id: "NCT-12345678".into(),
                criteria_met: 7,
                criteria_total: 7,
            },
            b"age:45,no_cardiac,no_renal,a1c>6.5,bmi<35,no_insulin,consent",
            b"patient-002",
            AttestorRole::ClinicalTrialSponsor,
            1000000,
            1500000,
            0,
            0,
            0, // Trial: complex type, no range
        );

        assert_eq!(proof.metadata.proof_size, 32);
        assert_eq!(proof.metadata.security_bits, 0);
        assert!(!verify_proof_at(&proof, 1_250_000));
    }

    #[test]
    fn proof_types_cover_all_use_cases() {
        let types = vec![
            HealthProofType::VitalsInRange,
            HealthProofType::ConditionAbsence {
                excluded_icd10: vec!["E11".into()],
            },
            HealthProofType::VaccinationStatus {
                disease: "COVID-19".into(),
            },
            HealthProofType::LabThreshold {
                loinc_code: "2345-7".into(),
                threshold: 100.0,
                direction: ThresholdDirection::Below,
            },
            HealthProofType::AgeRange {
                min_age: 18,
                max_age: Some(65),
            },
            HealthProofType::TrialEligibility {
                trial_id: "NCT-1".into(),
                criteria_met: 5,
                criteria_total: 5,
            },
            HealthProofType::EmploymentPhysical,
            HealthProofType::InsuranceQualification {
                tier: "standard".into(),
            },
            HealthProofType::SubstanceScreening,
            HealthProofType::OrganDonorCompatibility {
                recipient_hla_hash: [0u8; 32],
            },
            HealthProofType::Custom {
                description: "Travel clearance".into(),
            },
        ];
        assert_eq!(types.len(), 11);
    }

    #[test]
    fn recommended_backend_selection() {
        assert_eq!(
            HealthProofType::VitalsInRange.recommended_backend(),
            BackendId::Winterfell
        );
        assert_eq!(
            HealthProofType::AgeRange {
                min_age: 18,
                max_age: None
            }
            .recommended_backend(),
            BackendId::Winterfell
        );
        assert_eq!(
            HealthProofType::TrialEligibility {
                trial_id: "T".into(),
                criteria_met: 1,
                criteria_total: 1
            }
            .recommended_backend(),
            BackendId::Risc0
        );
        assert_eq!(
            HealthProofType::OrganDonorCompatibility {
                recipient_hla_hash: [0; 32]
            }
            .recommended_backend(),
            BackendId::Risc0
        );
    }

    #[cfg(feature = "experimental-unbound-range-proofs")]
    #[test]
    fn experimental_winterfell_proof_generation_and_verification() {
        let proof = generate_proof(
            HealthProofType::VitalsInRange,
            b"bp:120/80,hr:72,temp:98.6",
            b"patient-003",
            AttestorRole::Physician,
            1000000,
            2000000,
            120,
            90,
            180, // BP systolic: 120 in [90, 180]
        );

        // Should generate a REAL Winterfell STARK proof (not SHA-256 commitment)
        assert_eq!(
            proof.metadata.system,
            ProofSystem::WinterfellStark,
            "VitalsInRange should use Winterfell STARK, not SHA-256 fallback"
        );

        // Proof should be significantly larger than a SHA-256 hash (32 bytes)
        assert!(
            proof.proof_bytes.len() > 1000,
            "STARK proof should be >1KB, got {} bytes",
            proof.proof_bytes.len()
        );

        // REAL verification should pass
        assert!(
            verify_proof_at(&proof, 1_500_000),
            "experimental Winterfell STARK proof must verify cryptographically"
        );
    }

    #[test]
    fn empty_proof_fails_verification() {
        let proof = HealthProof {
            proof_type: HealthProofType::VitalsInRange,
            proof_bytes: vec![],
            public_inputs: HealthPublicInputs {
                patient_id_hash: [0; 32],
                data_commitment: [0; 32],
                criteria_met: true,
                data_timestamp: 0,
                attestor_role: AttestorRole::PatientSelf,
                min_value: 0,
                max_value: 0,
            },
            metadata: ProofMetadata {
                system: ProofSystem::Sha256Commitment,
                security_bits: 128,
                post_quantum: true,
                generated_at: 0,
                expires_at: 0,
                proof_size: 0,
            },
        };
        assert!(!verify_proof(&proof), "empty proof should fail");
    }

    #[cfg(feature = "experimental-unbound-range-proofs")]
    #[test]
    fn tampered_stark_proof_fails() {
        let mut proof = generate_proof(
            HealthProofType::VitalsInRange,
            b"data",
            b"patient",
            AttestorRole::Physician,
            1000000,
            2000000,
            100,
            50,
            200,
        );
        // Tamper with the STARK proof bytes
        if !proof.proof_bytes.is_empty() {
            proof.proof_bytes[0] ^= 0xFF; // Flip first byte
        }
        assert!(
            !verify_proof_at(&proof, 1_500_000),
            "tampered STARK proof should fail verification"
        );
    }

    #[test]
    fn expired_proof_is_rejected_against_trusted_time() {
        let proof = generate_proof(
            HealthProofType::VitalsInRange,
            b"data",
            b"patient",
            AttestorRole::Physician,
            1_000,
            2_000,
            100,
            50,
            200,
        );
        assert!(!verify_proof_at(&proof, 2_001));
    }

    #[test]
    fn future_issued_proof_is_rejected() {
        let proof = generate_proof(
            HealthProofType::Custom {
                description: "future".into(),
            },
            b"data",
            b"patient",
            AttestorRole::PatientSelf,
            2_000,
            3_000,
            1,
            0,
            2,
        );
        assert!(!verify_proof_at(&proof, 1_999));
    }

    #[test]
    fn commitment_placeholder_is_never_treated_as_a_proof() {
        let proof = generate_proof(
            HealthProofType::Custom {
                description: "placeholder".into(),
            },
            b"data",
            b"patient",
            AttestorRole::PatientSelf,
            1_000,
            2_000,
            0,
            0,
            0,
        );
        assert_eq!(proof.metadata.system, ProofSystem::Sha256Commitment);
        assert!(!verify_proof_at(&proof, 1_500));
    }

    #[test]
    fn proof_system_backend_mapping() {
        assert_eq!(
            ProofSystem::WinterfellStark.backend_id(),
            Some(BackendId::Winterfell)
        );
        assert_eq!(ProofSystem::Risc0ZkVm.backend_id(), Some(BackendId::Risc0));
        assert_eq!(ProofSystem::Sha256Commitment.backend_id(), None);
    }
}
