//! ZK Health Proofs Integrity Zome
//!
//! Zero-knowledge proof system for healthcare attestations that lets patients
//! prove health status without revealing actual health information.
//!
//! Built on zkSTARK foundation from Mycelix SDK.
//!
//! Use Cases:
//! - "I am healthy enough for life insurance" (reveals: qualifies ✓)
//! - "I passed employment physical" (reveals: cleared ✓)
//! - "I am organ donor compatible" (reveals: compatible ✓)
//! - "I am eligible for clinical trial X" (reveals: eligible ✓)
//! - "I have no communicable diseases" (reveals: cleared ✓)

use hdi::prelude::*;

/// Define the entry types for the ZK health proofs zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    /// Zero-knowledge health proof
    HealthProof(HealthProof),
    /// Proof request from a verifier
    ProofRequest(ProofRequest),
    /// Proof verification result
    VerificationResult(VerificationResult),
    /// Trusted attestor registration
    TrustedAttestor(TrustedAttestor),
    /// Proof template (reusable proof types)
    ProofTemplate(ProofTemplate),
    /// Proof generation parameters (patient-controlled)
    ProofParameters(ProofParameters),
}

/// Link types for the ZK health proofs zome
#[hdk_link_types]
pub enum LinkTypes {
    PatientToProofs,
    PatientToRequests,
    PatientToParameters,
    VerifierToRequests,
    ProofToVerifications,
    TemplateToProofs,
    TrustedAttestors,
    ActiveRequests,
    CompletedProofs,
}

// ==================== HEALTH PROOFS ====================

/// Zero-knowledge health proof
///
/// The proof attests to a health claim without revealing underlying data.
/// Uses zkSTARK circuits for cryptographic guarantees.
#[hdk_entry_helper]
#[derive(Clone)]
pub struct HealthProof {
    /// Unique proof ID
    pub proof_id: String,
    /// Patient who generated the proof
    pub patient_hash: ActionHash,
    /// Type of proof (what is being attested)
    pub proof_type: HealthProofType,
    /// The claim being proven (human-readable)
    pub claim: String,
    /// Cryptographic proof bytes (zkSTARK proof)
    pub proof_bytes: Vec<u8>,
    /// Public inputs for verification
    pub public_inputs: PublicHealthInputs,
    /// Proof metadata
    pub metadata: ProofMetadata,
    /// Attestations from trusted parties (if any)
    pub attestations: Vec<HealthAttestation>,
    /// Validity period start
    pub valid_from: i64,
    /// Validity period end (expiration)
    pub valid_until: i64,
    /// Whether proof has been revoked
    pub revoked: bool,
    /// Revocation reason if revoked
    pub revocation_reason: Option<String>,
    /// Generation timestamp
    pub generated_at: i64,
}

/// Types of health proofs
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum HealthProofType {
    /// General health status (e.g., "healthy enough for X")
    GeneralHealth,
    /// Specific condition absence (e.g., "no diabetes")
    ConditionAbsence,
    /// Specific condition presence (e.g., "has condition for disability")
    ConditionPresence,
    /// Vaccination status
    VaccinationStatus,
    /// Allergy absence (e.g., "no penicillin allergy")
    AllergyStatus,
    /// Medication status (e.g., "not on controlled substances")
    MedicationStatus,
    /// Lab result threshold (e.g., "cholesterol below X")
    LabThreshold,
    /// Physical capability (e.g., "vision 20/20 or better")
    PhysicalCapability,
    /// Mental health clearance
    MentalHealthClearance,
    /// Drug/alcohol screening
    SubstanceScreening,
    /// Organ donor compatibility
    OrganDonorCompatibility,
    /// Clinical trial eligibility
    ClinicalTrialEligibility,
    /// Travel health clearance
    TravelHealthClearance,
    /// Employment physical
    EmploymentPhysical,
    /// Insurance qualification
    InsuranceQualification,
    /// Age verification (health-based)
    AgeVerification,
    /// Custom proof type
    Custom(String),
}

/// Public inputs for verification (revealed with proof)
/// These are the ONLY things revealed - everything else is hidden
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicHealthInputs {
    /// Hash of patient's identity (not the actual identity)
    pub patient_identity_hash: [u8; 32],
    /// Hash of the underlying health data (not the data itself)
    pub data_commitment: [u8; 32],
    /// The threshold or criteria being met (public)
    pub criteria_met: bool,
    /// Timestamp of data used (to prove recency)
    pub data_timestamp: i64,
    /// Attestor commitment (hash of attestor identity)
    pub attestor_commitment: Option<[u8; 32]>,
    /// Proof schema version
    pub schema_version: String,
}

/// Proof metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProofMetadata {
    /// Proof generation time (milliseconds)
    pub generation_time_ms: u64,
    /// Size of proof in bytes
    pub proof_size_bytes: u64,
    /// Circuit used for proof
    pub circuit_id: String,
    /// Prover version
    pub prover_version: String,
    /// Security level (bits)
    pub security_bits: u32,
    /// Whether proof is post-quantum secure
    pub post_quantum: bool,
}

/// Healthcare provider attestation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealthAttestation {
    /// Attestor (provider) identifier hash
    pub attestor_hash: [u8; 32],
    /// Attestor's credential type
    pub credential_type: AttestorCredentialType,
    /// Signature over the health claim
    pub signature: Vec<u8>,
    /// Timestamp of attestation
    pub attested_at: i64,
    /// Attestation expiry
    pub expires_at: Option<i64>,
}

/// Types of attestor credentials
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AttestorCredentialType {
    /// Licensed physician (MD/DO)
    Physician,
    /// Nurse practitioner
    NursePractitioner,
    /// Physician assistant
    PhysicianAssistant,
    /// Licensed laboratory
    Laboratory,
    /// Hospital/clinic
    HealthcareOrganization,
    /// Insurance company medical reviewer
    InsuranceMedicalReviewer,
    /// Public health authority
    PublicHealthAuthority,
    /// Clinical trial sponsor
    ClinicalTrialSponsor,
}

// ==================== PROOF REQUESTS ====================

/// Request for a health proof from a verifier
#[hdk_entry_helper]
#[derive(Clone)]
pub struct ProofRequest {
    /// Unique request ID
    pub request_id: String,
    /// Verifier requesting the proof
    pub verifier: AgentPubKey,
    /// Verifier name/organization
    pub verifier_name: String,
    /// Patient being asked for proof
    pub patient_hash: ActionHash,
    /// Type of proof requested
    pub requested_proof_type: HealthProofType,
    /// Human-readable description of what's needed
    pub description: String,
    /// Purpose of the proof (why they need it)
    pub purpose: ProofPurpose,
    /// Minimum validity period required
    pub required_validity_days: Option<u32>,
    /// Whether attestation is required
    pub requires_attestation: bool,
    /// Acceptable attestor types
    pub acceptable_attestors: Vec<AttestorCredentialType>,
    /// Request status
    pub status: ProofRequestStatus,
    /// When request was made
    pub requested_at: i64,
    /// When request expires
    pub expires_at: i64,
    /// Patient response
    pub patient_response: Option<PatientResponse>,
}

/// Purpose for requesting a health proof
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ProofPurpose {
    /// Employment requirement
    Employment,
    /// Insurance application
    Insurance,
    /// Clinical trial enrollment
    ClinicalTrial,
    /// Travel requirement
    Travel,
    /// Organ donation
    OrganDonation,
    /// Sports/athletic participation
    Athletic,
    /// Military service
    Military,
    /// Educational enrollment
    Education,
    /// Legal requirement
    Legal,
    /// Other (specified)
    Other(String),
}

/// Status of a proof request
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ProofRequestStatus {
    /// Waiting for patient response
    Pending,
    /// Patient accepted, generating proof
    Accepted,
    /// Proof submitted
    ProofSubmitted,
    /// Proof verified successfully
    Verified,
    /// Patient declined
    Declined,
    /// Request expired
    Expired,
    /// Verification failed
    VerificationFailed,
}

/// Patient's response to a proof request
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PatientResponse {
    /// Whether patient consents
    pub consents: bool,
    /// Reason if declined
    pub decline_reason: Option<String>,
    /// Proof hash if generated
    pub proof_hash: Option<ActionHash>,
    /// Response timestamp
    pub responded_at: i64,
}

// ==================== VERIFICATION ====================

/// Result of verifying a health proof
#[hdk_entry_helper]
#[derive(Clone)]
pub struct VerificationResult {
    /// Unique verification ID
    pub verification_id: String,
    /// Proof that was verified
    pub proof_hash: ActionHash,
    /// Verifier who checked
    pub verifier: AgentPubKey,
    /// Whether verification succeeded
    pub verified: bool,
    /// Verification details
    pub details: VerificationDetails,
    /// Verification timestamp
    pub verified_at: i64,
}

/// Details of verification
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerificationDetails {
    /// Cryptographic verification passed
    pub crypto_valid: bool,
    /// Proof is within validity period
    pub within_validity: bool,
    /// Attestations verified
    pub attestations_verified: bool,
    /// Proof not revoked
    pub not_revoked: bool,
    /// Data recency acceptable
    pub data_recency_ok: bool,
    /// Failure reason if any
    pub failure_reason: Option<String>,
    /// Verification time (milliseconds)
    pub verification_time_ms: u64,
}

// ==================== TRUSTED ATTESTORS ====================

/// Registered trusted attestor
#[hdk_entry_helper]
#[derive(Clone)]
pub struct TrustedAttestor {
    /// Unique attestor ID
    pub attestor_id: String,
    /// Attestor's public key
    pub public_key: AgentPubKey,
    /// Attestor name/organization
    pub name: String,
    /// Credential type
    pub credential_type: AttestorCredentialType,
    /// Credentials (license numbers, etc.) - hashed for privacy
    pub credential_hashes: Vec<[u8; 32]>,
    /// Whether currently active
    pub active: bool,
    /// Registration timestamp
    pub registered_at: i64,
    /// Trust score (from MATL)
    pub trust_score: f32,
    /// Number of attestations made
    pub attestation_count: u64,
}

// ==================== PROOF TEMPLATES ====================

/// Reusable proof template
#[hdk_entry_helper]
#[derive(Clone)]
pub struct ProofTemplate {
    /// Unique template ID
    pub template_id: String,
    /// Template name
    pub name: String,
    /// Description
    pub description: String,
    /// Proof type this template is for
    pub proof_type: HealthProofType,
    /// Required data categories
    pub required_data: Vec<RequiredDataCategory>,
    /// Circuit ID for this proof type
    pub circuit_id: String,
    /// Whether attestation is typically required
    pub typically_requires_attestation: bool,
    /// Default validity period (days)
    pub default_validity_days: u32,
    /// Example use cases
    pub use_cases: Vec<String>,
    /// Created by
    pub created_by: Option<AgentPubKey>,
    /// System template (vs user-created)
    pub system_template: bool,
}

/// Data categories needed for a proof
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RequiredDataCategory {
    /// Lab results
    LabResults(Vec<String>), // specific test types
    /// Vital signs
    VitalSigns,
    /// Diagnoses
    Diagnoses,
    /// Medications
    Medications,
    /// Allergies
    Allergies,
    /// Immunizations
    Immunizations,
    /// Physical exam
    PhysicalExam,
    /// Mental health assessment
    MentalHealthAssessment,
    /// Imaging
    Imaging,
    /// Procedures
    Procedures,
}

// ==================== PROOF PARAMETERS ====================

/// Patient's proof generation parameters
#[hdk_entry_helper]
#[derive(Clone)]
pub struct ProofParameters {
    /// Patient
    pub patient_hash: ActionHash,
    /// Default validity period (days)
    pub default_validity_days: u32,
    /// Auto-approve requests from certain verifiers
    pub auto_approve_verifiers: Vec<AgentPubKey>,
    /// Proof types patient is willing to generate
    pub allowed_proof_types: Vec<HealthProofType>,
    /// Proof types patient refuses to generate
    pub refused_proof_types: Vec<HealthProofType>,
    /// Require attestation for certain proof types
    pub require_attestation_for: Vec<HealthProofType>,
    /// Maximum proofs to generate per day
    pub daily_proof_limit: Option<u32>,
    /// Updated at
    pub updated_at: i64,
}

// ==================== VALIDATION ====================

/// Validate a health proof
pub fn validate_health_proof(proof: &HealthProof) -> ExternResult<ValidateCallbackResult> {
    if proof.proof_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Proof ID required".to_string()));
    }

    if proof.claim.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Claim required".to_string()));
    }

    if proof.proof_bytes.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Proof bytes required".to_string()));
    }

    if proof.valid_until <= proof.valid_from {
        return Ok(ValidateCallbackResult::Invalid("Valid until must be after valid from".to_string()));
    }

    // Verify proof size is reasonable (typical zkSTARK ~75KB)
    if proof.proof_bytes.len() > 500_000 {
        return Ok(ValidateCallbackResult::Invalid("Proof size exceeds maximum".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate a proof request
pub fn validate_proof_request(request: &ProofRequest) -> ExternResult<ValidateCallbackResult> {
    if request.request_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Request ID required".to_string()));
    }

    if request.verifier_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Verifier name required".to_string()));
    }

    if request.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Description required".to_string()));
    }

    if request.expires_at <= request.requested_at {
        return Ok(ValidateCallbackResult::Invalid("Expiry must be after request time".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate a trusted attestor
pub fn validate_trusted_attestor(attestor: &TrustedAttestor) -> ExternResult<ValidateCallbackResult> {
    if attestor.attestor_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Attestor ID required".to_string()));
    }

    if attestor.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Attestor name required".to_string()));
    }

    if attestor.trust_score < 0.0 || attestor.trust_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid("Trust score must be between 0 and 1".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate a proof template
pub fn validate_proof_template(template: &ProofTemplate) -> ExternResult<ValidateCallbackResult> {
    if template.template_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Template ID required".to_string()));
    }

    if template.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Template name required".to_string()));
    }

    if template.circuit_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Circuit ID required".to_string()));
    }

    if template.default_validity_days == 0 {
        return Ok(ValidateCallbackResult::Invalid("Validity days must be positive".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate proof parameters
pub fn validate_proof_parameters(params: &ProofParameters) -> ExternResult<ValidateCallbackResult> {
    if params.default_validity_days == 0 {
        return Ok(ValidateCallbackResult::Invalid("Default validity days must be positive".to_string()));
    }

    // Cannot refuse and allow same proof type
    for refused in &params.refused_proof_types {
        if params.allowed_proof_types.contains(refused) {
            return Ok(ValidateCallbackResult::Invalid(
                "Cannot both allow and refuse same proof type".to_string()
            ));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}


// ==================== ZKSTARK INTEGRATION TYPES ====================

/// Interface for zkSTARK proof generation (simulation mode compatible)
/// These types mirror the Mycelix SDK zkproof types for integration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkStarkProofConfig {
    /// Circuit identifier
    pub circuit_id: String,
    /// Security parameter (bits)
    pub security_bits: u32,
    /// Whether to use post-quantum security
    pub post_quantum: bool,
    /// Maximum proof generation time (ms)
    pub max_generation_time_ms: Option<u64>,
}

impl Default for ZkStarkProofConfig {
    fn default() -> Self {
        Self {
            circuit_id: "health-attestation-v1".to_string(),
            security_bits: 128,
            post_quantum: false,
            max_generation_time_ms: Some(30_000), // 30 seconds
        }
    }
}

/// Proof statement for health attestation
/// This is the logical claim being proven in zero-knowledge
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealthProofStatement {
    /// What is being claimed
    pub claim_type: HealthProofType,
    /// The specific claim text
    pub claim_text: String,
    /// Public threshold/criteria (what's revealed)
    pub public_criteria: String,
    /// Private witness data hash (proves knowledge without revealing)
    pub private_data_commitment: [u8; 32],
    /// Timestamp of underlying data
    pub data_timestamp: i64,
}

/// Properties of zkSTARK health proofs
///
/// - **Completeness**: Honest patient can always prove true health claims
/// - **Soundness**: Dishonest patient cannot prove false claims
/// - **Zero-Knowledge**: Proof reveals NOTHING about actual health data
/// - **Succinctness**: Proof size O(log n), verification O(1)
pub const PROOF_PROPERTIES: &str = r#"
    I claim that my health data D satisfies criteria C,
    where commitment(D) = public_data_commitment,
    and the criteria check function f(D) = true.

    This proof reveals:
    - That criteria C is met (true/false)
    - The timestamp of data D
    - The identity commitment of the attestor (if any)

    This proof hides:
    - The actual health data D
    - Any specific values, conditions, or results
    - Patient identity beyond the commitment
"#;
