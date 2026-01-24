//! Healthcare Provider Credentials and Licensing Integrity Zome
//! 
//! Defines entry types for healthcare providers, credentials,
//! licenses, and specializations with verification tracking.

use hdi::prelude::*;

/// Healthcare provider profile
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Provider {
    /// National Provider Identifier (NPI) - US standard
    pub npi: Option<String>,
    /// Provider type
    pub provider_type: ProviderType,
    /// First name
    pub first_name: String,
    /// Last name
    pub last_name: String,
    /// Professional title (MD, DO, RN, etc.)
    pub title: String,
    /// Primary specialty
    pub specialty: String,
    /// Sub-specialties
    pub sub_specialties: Vec<String>,
    /// Practice/organization name
    pub organization: Option<String>,
    /// Practice locations
    pub locations: Vec<PracticeLocation>,
    /// Contact information
    pub contact: ProviderContact,
    /// Languages spoken
    pub languages: Vec<String>,
    /// Accepting new patients
    pub accepting_patients: bool,
    /// Telehealth capable
    pub telehealth_enabled: bool,
    /// Mycelix identity link
    pub mycelix_identity_hash: Option<ActionHash>,
    /// MATL trust score based on patient outcomes and feedback
    pub matl_trust_score: f64,
    /// Epistemic classification for claims made
    pub epistemic_level: EpistemicLevel,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ProviderType {
    Physician,
    Nurse,
    NursePractitioner,
    PhysicianAssistant,
    Pharmacist,
    Therapist,
    Dentist,
    Optometrist,
    Chiropractor,
    Researcher,
    LabTechnician,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EpistemicLevel {
    /// E0: Unverified claims
    Unverified,
    /// E1: Peer-reviewed but not replicated
    PeerReviewed,
    /// E2: Replicated findings
    Replicated,
    /// E3: Established medical consensus
    Consensus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PracticeLocation {
    pub name: String,
    pub address_line1: String,
    pub address_line2: Option<String>,
    pub city: String,
    pub state_province: String,
    pub postal_code: String,
    pub country: String,
    pub phone: String,
    pub fax: Option<String>,
    pub hours: Option<String>,
    pub is_primary: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProviderContact {
    pub email: String,
    pub phone_office: String,
    pub phone_emergency: Option<String>,
    pub website: Option<String>,
}

/// Medical license credential
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct License {
    pub provider_hash: ActionHash,
    pub license_type: LicenseType,
    pub license_number: String,
    pub issuing_authority: String,
    pub jurisdiction: String,
    pub issued_date: String,
    pub expiration_date: String,
    pub status: LicenseStatus,
    /// Verification from external source
    pub verification_source: Option<String>,
    pub verified_at: Option<Timestamp>,
    pub verified_by: Option<AgentPubKey>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum LicenseType {
    Medical,
    Nursing,
    Pharmacy,
    Dental,
    Psychology,
    Therapy,
    DEA,
    StateControlled,
    BoardCertification,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum LicenseStatus {
    Active,
    Expired,
    Suspended,
    Revoked,
    Pending,
    Restricted,
}

/// Board certification
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct BoardCertification {
    pub provider_hash: ActionHash,
    pub board_name: String,
    pub specialty: String,
    pub certification_number: Option<String>,
    pub initial_certification_date: String,
    pub expiration_date: String,
    pub status: CertificationStatus,
    pub verified: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CertificationStatus {
    Active,
    Expired,
    Pending,
    Revoked,
}

/// Provider-patient relationship
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ProviderPatientRelationship {
    pub provider_hash: ActionHash,
    pub patient_hash: ActionHash,
    pub relationship_type: RelationshipType,
    pub start_date: Timestamp,
    pub end_date: Option<Timestamp>,
    pub is_active: bool,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    PrimaryCare,
    Specialist,
    Consultant,
    EmergencyOnly,
    Research,
    Other(String),
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Provider(Provider),
    License(License),
    BoardCertification(BoardCertification),
    ProviderPatientRelationship(ProviderPatientRelationship),
}

#[hdk_link_types]
pub enum LinkTypes {
    ProviderToLicenses,
    ProviderToCertifications,
    ProviderToPatients,
    ProviderToRecords,
    ProviderToPrescriptions,
    ProviderToTrials,
    ProviderUpdates,
    AllProviders,
    ProvidersBySpecialty,
    ProvidersByLocation,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::Provider(provider) => validate_provider(&provider),
                EntryTypes::License(license) => validate_license(&license),
                EntryTypes::BoardCertification(cert) => validate_certification(&cert),
                EntryTypes::ProviderPatientRelationship(rel) => validate_relationship(&rel),
            },
            OpEntry::UpdateEntry { app_entry, .. } => match app_entry {
                EntryTypes::Provider(provider) => validate_provider(&provider),
                EntryTypes::License(license) => validate_license(&license),
                EntryTypes::BoardCertification(cert) => validate_certification(&cert),
                EntryTypes::ProviderPatientRelationship(rel) => validate_relationship(&rel),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_provider(provider: &Provider) -> ExternResult<ValidateCallbackResult> {
    if provider.first_name.is_empty() || provider.last_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Provider first and last name are required".to_string(),
        ));
    }

    if provider.title.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Provider title is required".to_string(),
        ));
    }

    if provider.specialty.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Provider specialty is required".to_string(),
        ));
    }

    if provider.matl_trust_score < 0.0 || provider.matl_trust_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "MATL trust score must be between 0.0 and 1.0".to_string(),
        ));
    }

    // Validate NPI format if provided (US 10-digit standard)
    if let Some(ref npi) = provider.npi {
        if npi.len() != 10 || !npi.chars().all(|c| c.is_ascii_digit()) {
            return Ok(ValidateCallbackResult::Invalid(
                "NPI must be a 10-digit number".to_string(),
            ));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_license(license: &License) -> ExternResult<ValidateCallbackResult> {
    if license.license_number.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "License number is required".to_string(),
        ));
    }

    if license.issuing_authority.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Issuing authority is required".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_certification(cert: &BoardCertification) -> ExternResult<ValidateCallbackResult> {
    if cert.board_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Board name is required".to_string(),
        ));
    }

    if cert.specialty.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Specialty is required".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_relationship(_rel: &ProviderPatientRelationship) -> ExternResult<ValidateCallbackResult> {
    // Relationship validation - hashes must exist (checked at runtime)
    Ok(ValidateCallbackResult::Valid)
}
