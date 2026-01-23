//! Patient Identity and Demographics Integrity Zome
//!
//! Defines entry types for patient profiles, health identifiers,
//! and demographic information with HIPAA-compliant validation.

use hdi::prelude::*;

/// Patient profile with demographics and health identifiers
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Patient {
    /// Unique patient identifier (internal)
    pub patient_id: String,
    /// Medical Record Number (MRN) - optional, provider-assigned
    pub mrn: Option<String>,
    /// First name (encrypted at rest)
    pub first_name: String,
    /// Last name (encrypted at rest)
    pub last_name: String,
    /// Date of birth (YYYY-MM-DD format)
    pub date_of_birth: String,
    /// Biological sex for medical purposes
    pub biological_sex: BiologicalSex,
    /// Gender identity (patient-reported)
    pub gender_identity: Option<String>,
    /// Blood type if known
    pub blood_type: Option<BloodType>,
    /// Primary contact information
    pub contact: ContactInfo,
    /// Emergency contact
    pub emergency_contact: Option<EmergencyContact>,
    /// Primary language
    pub primary_language: String,
    /// Known allergies (critical for safety)
    pub allergies: Vec<Allergy>,
    /// Active medical conditions
    pub conditions: Vec<String>,
    /// Current medications
    pub medications: Vec<String>,
    /// Mycelix identity link (for cross-hApp federation)
    pub mycelix_identity_hash: Option<ActionHash>,
    /// MATL trust score for this patient's data reliability
    pub matl_trust_score: f64,
    /// Timestamp of profile creation
    pub created_at: Timestamp,
    /// Timestamp of last update
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BiologicalSex {
    Male,
    Female,
    Intersex,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BloodType {
    APositive,
    ANegative,
    BPositive,
    BNegative,
    ABPositive,
    ABNegative,
    OPositive,
    ONegative,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ContactInfo {
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state_province: Option<String>,
    pub postal_code: Option<String>,
    pub country: String,
    pub phone_primary: Option<String>,
    pub phone_secondary: Option<String>,
    pub email: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EmergencyContact {
    pub name: String,
    pub relationship: String,
    pub phone: String,
    pub email: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Allergy {
    pub allergen: String,
    pub reaction: String,
    pub severity: AllergySeverity,
    pub verified: bool,
    pub verified_by: Option<AgentPubKey>,
    pub verified_at: Option<Timestamp>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AllergySeverity {
    Mild,
    Moderate,
    Severe,
    LifeThreatening,
}

/// Link between patient and their identity verification
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PatientIdentityLink {
    pub patient_hash: ActionHash,
    pub identity_provider: String,
    pub verified_at: Timestamp,
    pub verification_method: String,
    pub confidence_score: f64,
}

/// Patient health summary for quick reference
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PatientHealthSummary {
    pub patient_hash: ActionHash,
    pub summary_date: Timestamp,
    pub active_conditions: Vec<String>,
    pub current_medications: Vec<String>,
    pub recent_procedures: Vec<String>,
    pub upcoming_appointments: Vec<String>,
    pub care_team: Vec<AgentPubKey>,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Patient(Patient),
    PatientIdentityLink(PatientIdentityLink),
    PatientHealthSummary(PatientHealthSummary),
}

#[hdk_link_types]
pub enum LinkTypes {
    PatientToRecords,
    PatientToConsents,
    PatientToPrescriptions,
    PatientToProviders,
    PatientToTrials,
    PatientToInsurance,
    PatientUpdates,
    AllPatients,
}

/// Validation for Patient entries
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::Patient(patient) => validate_patient(&patient),
                EntryTypes::PatientIdentityLink(link) => validate_identity_link(&link),
                EntryTypes::PatientHealthSummary(summary) => validate_health_summary(&summary),
            },
            OpEntry::UpdateEntry { app_entry, .. } => match app_entry {
                EntryTypes::Patient(patient) => validate_patient(&patient),
                EntryTypes::PatientIdentityLink(link) => validate_identity_link(&link),
                EntryTypes::PatientHealthSummary(summary) => validate_health_summary(&summary),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterCreateLink { link_type, .. } => match link_type {
            LinkTypes::PatientToRecords => Ok(ValidateCallbackResult::Valid),
            LinkTypes::PatientToConsents => Ok(ValidateCallbackResult::Valid),
            LinkTypes::PatientToPrescriptions => Ok(ValidateCallbackResult::Valid),
            LinkTypes::PatientToProviders => Ok(ValidateCallbackResult::Valid),
            LinkTypes::PatientToTrials => Ok(ValidateCallbackResult::Valid),
            LinkTypes::PatientToInsurance => Ok(ValidateCallbackResult::Valid),
            LinkTypes::PatientUpdates => Ok(ValidateCallbackResult::Valid),
            LinkTypes::AllPatients => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_patient(patient: &Patient) -> ExternResult<ValidateCallbackResult> {
    // Validate patient_id is not empty
    if patient.patient_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Patient ID cannot be empty".to_string(),
        ));
    }

    // Validate names are not empty
    if patient.first_name.is_empty() || patient.last_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Patient first and last name are required".to_string(),
        ));
    }

    // Validate date of birth format (basic check)
    if !is_valid_date_format(&patient.date_of_birth) {
        return Ok(ValidateCallbackResult::Invalid(
            "Date of birth must be in YYYY-MM-DD format".to_string(),
        ));
    }

    // Validate MATL trust score is in valid range
    if patient.matl_trust_score < 0.0 || patient.matl_trust_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "MATL trust score must be between 0.0 and 1.0".to_string(),
        ));
    }

    // Validate allergies have required fields
    for allergy in &patient.allergies {
        if allergy.allergen.is_empty() || allergy.reaction.is_empty() {
            return Ok(ValidateCallbackResult::Invalid(
                "Allergies must have allergen and reaction specified".to_string(),
            ));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_identity_link(link: &PatientIdentityLink) -> ExternResult<ValidateCallbackResult> {
    if link.identity_provider.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Identity provider is required".to_string(),
        ));
    }

    if link.confidence_score < 0.0 || link.confidence_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Confidence score must be between 0.0 and 1.0".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_health_summary(
    _summary: &PatientHealthSummary,
) -> ExternResult<ValidateCallbackResult> {
    // Basic validation - summary must reference a patient
    // Additional validation would check if patient_hash exists
    Ok(ValidateCallbackResult::Valid)
}

fn is_valid_date_format(date: &str) -> bool {
    // Basic YYYY-MM-DD validation
    if date.len() != 10 {
        return false;
    }
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
}
