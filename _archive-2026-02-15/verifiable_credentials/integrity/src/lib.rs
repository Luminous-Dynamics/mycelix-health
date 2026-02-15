//! Verifiable Credentials Integrity Zome
//!
//! Implements W3C Verifiable Credentials standard for health credentials
//! including vaccination records, professional licenses, lab results,
//! prescriptions, and identity verification.

use hdi::prelude::*;

/// Credential type categories
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CredentialType {
    /// Vaccination/immunization record
    VaccinationCredential,
    /// COVID-19 specific vaccination
    CovidVaccinationCredential,
    /// Medical professional license
    MedicalLicenseCredential,
    /// Nursing license
    NursingLicenseCredential,
    /// Lab test result
    LabResultCredential,
    /// Prescription credential
    PrescriptionCredential,
    /// Insurance verification
    InsuranceCredential,
    /// Patient identity
    PatientIdentityCredential,
    /// Health status attestation
    HealthStatusCredential,
    /// Allergy/condition alert
    MedicalAlertCredential,
    /// Organ donor status
    OrganDonorCredential,
    /// Emergency contact info
    EmergencyInfoCredential,
    /// Travel health certificate
    TravelHealthCredential,
    /// Disability verification
    DisabilityCredential,
    /// Custom credential type
    Custom(String),
}

/// Proof type for credentials
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ProofType {
    /// Ed25519 signature
    Ed25519Signature2020,
    /// JSON Web Signature
    JsonWebSignature2020,
    /// BBS+ for selective disclosure
    BbsBlsSignature2020,
    /// Holochain-native proof
    HolochainSignature,
}

/// Credential status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CredentialStatusType {
    /// Active and valid
    Active,
    /// Temporarily suspended
    Suspended,
    /// Permanently revoked
    Revoked,
    /// Expired
    Expired,
    /// Pending verification
    Pending,
}

/// Verification status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum VerificationStatus {
    /// Verified successfully
    Verified,
    /// Verification failed
    Failed,
    /// Signature invalid
    InvalidSignature,
    /// Credential revoked
    Revoked,
    /// Credential expired
    Expired,
    /// Issuer not trusted
    UntrustedIssuer,
    /// Schema mismatch
    SchemaMismatch,
}

/// Credential schema definition
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CredentialSchema {
    /// Schema ID (URI)
    pub schema_id: String,
    /// Schema name
    pub name: String,
    /// Schema version
    pub version: String,
    /// Description
    pub description: String,
    /// Author/organization
    pub author: String,
    /// Schema type (JSON-LD context)
    pub schema_type: String,
    /// Required attributes
    pub required_attributes: Vec<String>,
    /// Optional attributes
    pub optional_attributes: Vec<String>,
    /// Attribute definitions (JSON)
    pub attribute_definitions: String,
    /// JSON-LD context URLs
    pub context_urls: Vec<String>,
    /// Is this a standard schema (HL7, SMART, etc.)
    pub is_standard: bool,
    /// Standard reference (if applicable)
    pub standard_reference: Option<String>,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Issuer profile for credential issuance
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CredentialIssuer {
    /// Issuer ID (DID)
    pub issuer_id: String,
    /// Issuer name
    pub name: String,
    /// Issuer type (hospital, clinic, lab, authority)
    pub issuer_type: String,
    /// Organization identifier (NPI, license number)
    pub organization_id: Option<String>,
    /// Public key for verification
    pub public_key: String,
    /// Key type
    pub key_type: String,
    /// Verification endpoint
    pub verification_endpoint: Option<String>,
    /// Trusted by (parent authority hashes)
    pub trusted_by: Vec<ActionHash>,
    /// Credential types this issuer can issue
    pub authorized_types: Vec<CredentialType>,
    /// Geographic jurisdiction
    pub jurisdiction: Option<String>,
    /// Active status
    pub is_active: bool,
    /// Created timestamp
    pub created_at: Timestamp,
    /// Updated timestamp
    pub updated_at: Timestamp,
}

/// Verifiable Credential (W3C VC Data Model)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct VerifiableCredential {
    /// Credential ID
    pub credential_id: String,
    /// JSON-LD context
    pub context: Vec<String>,
    /// Credential type
    pub credential_type: CredentialType,
    /// Additional types (W3C type array)
    pub types: Vec<String>,
    /// Issuer DID or hash
    pub issuer: String,
    /// Issuer hash in Holochain
    pub issuer_hash: ActionHash,
    /// Issuance date
    pub issuance_date: Timestamp,
    /// Expiration date
    pub expiration_date: Option<Timestamp>,
    /// Credential subject (holder DID)
    pub credential_subject_id: String,
    /// Subject claims (JSON)
    pub credential_subject: String,
    /// Proof type
    pub proof_type: ProofType,
    /// Proof value (signature)
    pub proof_value: String,
    /// Proof created timestamp
    pub proof_created: Timestamp,
    /// Verification method
    pub verification_method: String,
    /// Proof purpose
    pub proof_purpose: String,
    /// Schema hash
    pub schema_hash: ActionHash,
    /// Status
    pub status: CredentialStatusType,
    /// Revocation registry reference
    pub revocation_registry: Option<ActionHash>,
    /// Revocation index (if applicable)
    pub revocation_index: Option<u32>,
    /// Evidence references (supporting documents)
    pub evidence: Vec<String>,
    /// Terms of use
    pub terms_of_use: Option<String>,
    /// Refresh service URL
    pub refresh_service: Option<String>,
}

/// Credential holder's wallet entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CredentialHolder {
    /// Holder ID (DID)
    pub holder_id: String,
    /// Holder's public key
    pub public_key: String,
    /// Key type
    pub key_type: String,
    /// Display name (optional)
    pub display_name: Option<String>,
    /// Recovery key hash
    pub recovery_key_hash: Option<String>,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Held credential (credential in holder's wallet)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HeldCredential {
    /// Holder hash
    pub holder_hash: ActionHash,
    /// Credential hash
    pub credential_hash: ActionHash,
    /// Date acquired
    pub acquired_at: Timestamp,
    /// Nickname/label
    pub label: Option<String>,
    /// Is favorite/pinned
    pub is_favorite: bool,
    /// Category for organization
    pub category: Option<String>,
    /// Last used timestamp
    pub last_used: Option<Timestamp>,
    /// Use count
    pub use_count: u32,
}

/// Presentation request (verifier asking for credentials)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PresentationRequest {
    /// Request ID
    pub request_id: String,
    /// Verifier DID
    pub verifier: String,
    /// Verifier name
    pub verifier_name: Option<String>,
    /// Purpose of request
    pub purpose: String,
    /// Required credential types
    pub required_credentials: Vec<CredentialType>,
    /// Required attributes per credential type (JSON)
    pub required_attributes: String,
    /// Optional credentials
    pub optional_credentials: Vec<CredentialType>,
    /// Nonce for replay protection
    pub nonce: String,
    /// Domain/origin
    pub domain: Option<String>,
    /// Challenge (for proof)
    pub challenge: Option<String>,
    /// Expiration of request
    pub expires_at: Timestamp,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Verifiable Presentation (holder's response to request)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct VerifiablePresentation {
    /// Presentation ID
    pub presentation_id: String,
    /// JSON-LD context
    pub context: Vec<String>,
    /// Presentation type
    pub presentation_type: Vec<String>,
    /// Holder DID
    pub holder: String,
    /// Credential hashes included
    pub credential_hashes: Vec<ActionHash>,
    /// Derived credentials (selective disclosure)
    pub derived_credentials: Option<String>,
    /// Request hash (what prompted this)
    pub request_hash: Option<ActionHash>,
    /// Proof type
    pub proof_type: ProofType,
    /// Proof value
    pub proof_value: String,
    /// Proof created
    pub proof_created: Timestamp,
    /// Challenge response
    pub challenge: Option<String>,
    /// Domain
    pub domain: Option<String>,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Verification record (result of verifying a credential/presentation)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct VerificationRecord {
    /// Verification ID
    pub verification_id: String,
    /// What was verified (credential or presentation hash)
    pub verified_item_hash: ActionHash,
    /// Item type (credential or presentation)
    pub item_type: String,
    /// Verifier (who performed verification)
    pub verifier: String,
    /// Verification status
    pub status: VerificationStatus,
    /// Detailed checks performed (JSON)
    pub checks_performed: String,
    /// Error details if failed
    pub error_details: Option<String>,
    /// Verified timestamp
    pub verified_at: Timestamp,
}

/// Revocation registry for batch revocation
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RevocationRegistry {
    /// Registry ID
    pub registry_id: String,
    /// Issuer hash
    pub issuer_hash: ActionHash,
    /// Credential type this registry covers
    pub credential_type: CredentialType,
    /// Maximum credentials in registry
    pub max_credentials: u32,
    /// Current index
    pub current_index: u32,
    /// Accumulator value (for zero-knowledge revocation)
    pub accumulator: Option<String>,
    /// Revocation bitmap (for simple revocation)
    pub revocation_bitmap: Option<String>,
    /// Created timestamp
    pub created_at: Timestamp,
    /// Updated timestamp
    pub updated_at: Timestamp,
}

/// Individual revocation entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RevocationEntry {
    /// Registry hash
    pub registry_hash: ActionHash,
    /// Credential hash being revoked
    pub credential_hash: ActionHash,
    /// Revocation index in registry
    pub revocation_index: u32,
    /// Reason for revocation
    pub reason: String,
    /// Revoked by
    pub revoked_by: ActionHash,
    /// Revocation timestamp
    pub revoked_at: Timestamp,
}

/// Trust registry entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TrustEntry {
    /// Trustor (who is trusting)
    pub trustor_hash: ActionHash,
    /// Trustee (who is being trusted)
    pub trustee_hash: ActionHash,
    /// Trust level (0-100)
    pub trust_level: u8,
    /// Scope of trust (credential types)
    pub trust_scope: Vec<CredentialType>,
    /// Reason for trust
    pub reason: Option<String>,
    /// Valid from
    pub valid_from: Timestamp,
    /// Valid until
    pub valid_until: Option<Timestamp>,
    /// Is active
    pub is_active: bool,
}

/// Health-specific: Vaccination credential claims
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct VaccinationClaims {
    /// Vaccine type/name
    pub vaccine_name: String,
    /// Vaccine manufacturer
    pub manufacturer: String,
    /// Lot number
    pub lot_number: String,
    /// Dose number
    pub dose_number: u8,
    /// Total doses in series
    pub total_doses: u8,
    /// Administration date
    pub administration_date: Timestamp,
    /// Administering provider
    pub provider_name: String,
    /// Provider NPI
    pub provider_npi: Option<String>,
    /// Location administered
    pub location: String,
    /// CVX code
    pub cvx_code: String,
    /// MVX code (manufacturer)
    pub mvx_code: Option<String>,
    /// Disease target
    pub target_disease: String,
    /// SNOMED code
    pub snomed_code: Option<String>,
}

/// Health-specific: Lab result credential claims
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct LabResultClaims {
    /// Test name
    pub test_name: String,
    /// LOINC code
    pub loinc_code: String,
    /// Result value
    pub result_value: String,
    /// Result unit
    pub result_unit: Option<String>,
    /// Reference range
    pub reference_range: Option<String>,
    /// Interpretation (normal, abnormal, etc.)
    pub interpretation: String,
    /// Specimen type
    pub specimen_type: String,
    /// Collection date
    pub collection_date: Timestamp,
    /// Result date
    pub result_date: Timestamp,
    /// Lab name
    pub lab_name: String,
    /// Lab CLIA number
    pub lab_clia: Option<String>,
    /// Ordering provider
    pub ordering_provider: Option<String>,
}

/// Health-specific: Medical license claims
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MedicalLicenseClaims {
    /// License number
    pub license_number: String,
    /// License type (MD, DO, RN, NP, etc.)
    pub license_type: String,
    /// Specialty
    pub specialty: Option<String>,
    /// Issuing state/jurisdiction
    pub issuing_jurisdiction: String,
    /// Issue date
    pub issue_date: Timestamp,
    /// Expiration date
    pub expiration_date: Timestamp,
    /// NPI number
    pub npi: Option<String>,
    /// DEA number (if applicable)
    pub dea_number: Option<String>,
    /// License status
    pub status: String,
    /// Disciplinary actions
    pub disciplinary_actions: bool,
}

/// Entry types for the verifiable credentials zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    CredentialSchema(CredentialSchema),
    CredentialIssuer(CredentialIssuer),
    VerifiableCredential(VerifiableCredential),
    CredentialHolder(CredentialHolder),
    HeldCredential(HeldCredential),
    PresentationRequest(PresentationRequest),
    VerifiablePresentation(VerifiablePresentation),
    VerificationRecord(VerificationRecord),
    RevocationRegistry(RevocationRegistry),
    RevocationEntry(RevocationEntry),
    TrustEntry(TrustEntry),
    VaccinationClaims(VaccinationClaims),
    LabResultClaims(LabResultClaims),
    MedicalLicenseClaims(MedicalLicenseClaims),
}

/// Link types for the verifiable credentials zome
#[hdk_link_types]
pub enum LinkTypes {
    /// All schemas
    AllSchemas,
    /// All issuers
    AllIssuers,
    /// Issuer to credentials issued
    IssuerToCredentials,
    /// Schema to credentials using it
    SchemaToCredentials,
    /// Holder to held credentials
    HolderToCredentials,
    /// Credential to verification records
    CredentialToVerifications,
    /// Credential to revocation
    CredentialToRevocation,
    /// Issuer to revocation registries
    IssuerToRegistries,
    /// Issuer trust chain
    TrustChain,
    /// Presentation requests by verifier
    VerifierToRequests,
    /// Credentials by type
    CredentialsByType,
}

/// Validation callbacks
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::VerifiableCredential(vc) => validate_credential(&vc),
                EntryTypes::CredentialSchema(schema) => validate_schema(&schema),
                EntryTypes::CredentialIssuer(issuer) => validate_issuer(&issuer),
                EntryTypes::VerifiablePresentation(vp) => validate_presentation(&vp),
                _ => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_credential(vc: &VerifiableCredential) -> ExternResult<ValidateCallbackResult> {
    if vc.credential_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Credential ID is required".to_string(),
        ));
    }
    if vc.context.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "JSON-LD context is required".to_string(),
        ));
    }
    if vc.issuer.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Issuer is required".to_string(),
        ));
    }
    if vc.credential_subject.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Credential subject is required".to_string(),
        ));
    }
    if vc.proof_value.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Proof value is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_schema(schema: &CredentialSchema) -> ExternResult<ValidateCallbackResult> {
    if schema.schema_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Schema ID is required".to_string(),
        ));
    }
    if schema.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Schema name is required".to_string(),
        ));
    }
    if schema.version.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Schema version is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_issuer(issuer: &CredentialIssuer) -> ExternResult<ValidateCallbackResult> {
    if issuer.issuer_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Issuer ID is required".to_string(),
        ));
    }
    if issuer.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Issuer name is required".to_string(),
        ));
    }
    if issuer.public_key.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Public key is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_presentation(vp: &VerifiablePresentation) -> ExternResult<ValidateCallbackResult> {
    if vp.presentation_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Presentation ID is required".to_string(),
        ));
    }
    if vp.holder.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Holder is required".to_string(),
        ));
    }
    if vp.credential_hashes.is_empty() && vp.derived_credentials.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one credential is required".to_string(),
        ));
    }
    if vp.proof_value.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Proof value is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
