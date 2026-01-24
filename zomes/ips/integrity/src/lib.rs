//! International Patient Summary (IPS) Integrity Zome
//!
//! Defines entry types for the HL7 IPS standard, enabling cross-border
//! healthcare information exchange with standardized patient summaries.

use hdi::prelude::*;

/// IPS section types per HL7 standard
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum IpsSection {
    /// Allergies and intolerances (required)
    AllergiesIntolerances,
    /// Medication summary (required)
    MedicationSummary,
    /// Problem list (required)
    ProblemList,
    /// Immunizations
    Immunizations,
    /// History of procedures
    HistoryOfProcedures,
    /// Medical devices
    MedicalDevices,
    /// Diagnostic results
    DiagnosticResults,
    /// Vital signs
    VitalSigns,
    /// History of past illness
    PastIllnessHistory,
    /// Functional status
    FunctionalStatus,
    /// Plan of care
    PlanOfCare,
    /// Social history
    SocialHistory,
    /// Pregnancy history
    PregnancyHistory,
    /// Advance directives
    AdvanceDirectives,
}

/// Status of an IPS document
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum IpsStatus {
    /// Draft, not yet finalized
    Draft,
    /// Current and active
    Current,
    /// Superseded by newer version
    Superseded,
    /// Entered in error
    EnteredInError,
}

/// Criticality level for allergies
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AllergyCategory {
    Food,
    Medication,
    Environment,
    Biologic,
}

/// Severity of allergy reaction
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AllergySeverity {
    Mild,
    Moderate,
    Severe,
    LifeThreatening,
}

/// Medication status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MedicationStatus {
    Active,
    Completed,
    EnteredInError,
    Intended,
    Stopped,
    OnHold,
    Unknown,
    NotTaken,
}

/// Condition clinical status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ConditionClinicalStatus {
    Active,
    Recurrence,
    Relapse,
    Inactive,
    Remission,
    Resolved,
}

/// Complete International Patient Summary document
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct InternationalPatientSummary {
    /// IPS document ID (UUID)
    pub ips_id: String,
    /// Patient hash
    pub patient_hash: ActionHash,
    /// Document status
    pub status: IpsStatus,
    /// Version number
    pub version: u32,
    /// Language code (ISO 639-1)
    pub language: String,
    /// Country of origin (ISO 3166-1 alpha-2)
    pub country_of_origin: String,
    /// Generating organization
    pub author_organization: String,
    /// Author practitioner hash
    pub author_hash: Option<ActionHash>,
    /// Date of generation
    pub generated_at: Timestamp,
    /// Custodian organization
    pub custodian: String,
    /// Sections included
    pub sections_included: Vec<IpsSection>,
    /// FHIR Bundle serialization (JSON)
    pub fhir_bundle: String,
    /// Digital signature
    pub signature: Option<Vec<u8>>,
    /// Previous version hash
    pub previous_version_hash: Option<ActionHash>,
    /// Expiration timestamp
    pub expires_at: Option<Timestamp>,
}

/// IPS allergy entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IpsAllergy {
    /// Entry ID
    pub allergy_id: String,
    /// IPS document hash
    pub ips_hash: ActionHash,
    /// Category
    pub category: AllergyCategory,
    /// Causative agent code (SNOMED CT or RxNorm)
    pub agent_code: String,
    /// Coding system (SNOMED, RxNorm, etc.)
    pub coding_system: String,
    /// Agent display name
    pub agent_display: String,
    /// Severity
    pub severity: AllergySeverity,
    /// Criticality (high, low, unable-to-assess)
    pub criticality: String,
    /// Reaction manifestations (SNOMED codes)
    pub reactions: Vec<String>,
    /// Onset date if known
    pub onset_date: Option<Timestamp>,
    /// Verification status
    pub verification_status: String,
    /// Notes
    pub notes: Option<String>,
}

/// IPS medication entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IpsMedication {
    /// Entry ID
    pub medication_id: String,
    /// IPS document hash
    pub ips_hash: ActionHash,
    /// Medication code (RxNorm, ATC, or SNOMED)
    pub medication_code: String,
    /// Coding system
    pub coding_system: String,
    /// Medication display name
    pub medication_display: String,
    /// Status
    pub status: MedicationStatus,
    /// Dosage instructions (FHIR Dosage JSON)
    pub dosage: String,
    /// Route of administration (SNOMED)
    pub route_code: Option<String>,
    /// Form (tablet, capsule, etc.)
    pub form: Option<String>,
    /// Strength
    pub strength: Option<String>,
    /// Start date
    pub start_date: Option<Timestamp>,
    /// End date
    pub end_date: Option<Timestamp>,
    /// Reason for medication (condition code)
    pub reason_code: Option<String>,
    /// Prescriber reference
    pub prescriber_hash: Option<ActionHash>,
}

/// IPS problem/condition entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IpsProblem {
    /// Entry ID
    pub problem_id: String,
    /// IPS document hash
    pub ips_hash: ActionHash,
    /// Condition code (ICD-10 or SNOMED CT)
    pub condition_code: String,
    /// Coding system
    pub coding_system: String,
    /// Condition display name
    pub condition_display: String,
    /// Clinical status
    pub clinical_status: ConditionClinicalStatus,
    /// Verification status
    pub verification_status: String,
    /// Severity (SNOMED)
    pub severity_code: Option<String>,
    /// Body site (SNOMED)
    pub body_site: Option<String>,
    /// Onset date
    pub onset_date: Option<Timestamp>,
    /// Abatement date (if resolved)
    pub abatement_date: Option<Timestamp>,
    /// Recorded date
    pub recorded_date: Timestamp,
    /// Notes
    pub notes: Option<String>,
}

/// IPS immunization entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IpsImmunization {
    /// Entry ID
    pub immunization_id: String,
    /// IPS document hash
    pub ips_hash: ActionHash,
    /// Vaccine code (CVX)
    pub vaccine_code: String,
    /// Coding system
    pub coding_system: String,
    /// Vaccine display name
    pub vaccine_display: String,
    /// Administration date
    pub occurrence_date: Timestamp,
    /// Lot number
    pub lot_number: Option<String>,
    /// Expiration date
    pub expiration_date: Option<Timestamp>,
    /// Dose number
    pub dose_number: Option<u32>,
    /// Series doses
    pub series_doses: Option<u32>,
    /// Route (SNOMED)
    pub route_code: Option<String>,
    /// Site (SNOMED)
    pub site_code: Option<String>,
    /// Performer organization
    pub performer: Option<String>,
    /// Target disease (SNOMED)
    pub target_disease: Option<String>,
}

/// IPS procedure entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IpsProcedure {
    /// Entry ID
    pub procedure_id: String,
    /// IPS document hash
    pub ips_hash: ActionHash,
    /// Procedure code (SNOMED, CPT, or ICD-10-PCS)
    pub procedure_code: String,
    /// Coding system
    pub coding_system: String,
    /// Procedure display name
    pub procedure_display: String,
    /// Status (completed, in-progress, etc.)
    pub status: String,
    /// Performed date
    pub performed_date: Timestamp,
    /// Body site (SNOMED)
    pub body_site: Option<String>,
    /// Outcome
    pub outcome: Option<String>,
    /// Performer organization
    pub performer: Option<String>,
    /// Reason code
    pub reason_code: Option<String>,
    /// Notes
    pub notes: Option<String>,
}

/// IPS medical device entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IpsDevice {
    /// Entry ID
    pub device_id: String,
    /// IPS document hash
    pub ips_hash: ActionHash,
    /// Device type code (SNOMED)
    pub device_code: String,
    /// Coding system
    pub coding_system: String,
    /// Device display name
    pub device_display: String,
    /// Status (active, inactive, entered-in-error)
    pub status: String,
    /// Unique Device Identifier (UDI)
    pub udi: Option<String>,
    /// Manufacturer
    pub manufacturer: Option<String>,
    /// Model number
    pub model_number: Option<String>,
    /// Serial number
    pub serial_number: Option<String>,
    /// Implant date
    pub implant_date: Option<Timestamp>,
    /// Body site (SNOMED)
    pub body_site: Option<String>,
    /// Notes
    pub notes: Option<String>,
}

/// IPS result (lab/vital sign) entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IpsResult {
    /// Entry ID
    pub result_id: String,
    /// IPS document hash
    pub ips_hash: ActionHash,
    /// Result type (laboratory, vital-signs)
    pub result_type: String,
    /// Test code (LOINC)
    pub test_code: String,
    /// Coding system
    pub coding_system: String,
    /// Test display name
    pub test_display: String,
    /// Value (quantity, string, or coded)
    pub value: String,
    /// Value type (quantity, string, codeableConcept)
    pub value_type: String,
    /// Unit (UCUM)
    pub unit: Option<String>,
    /// Reference range
    pub reference_range: Option<String>,
    /// Interpretation (normal, abnormal, critical)
    pub interpretation: Option<String>,
    /// Effective date
    pub effective_date: Timestamp,
    /// Status
    pub status: String,
}

/// Cross-border sharing record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IpsShareRecord {
    /// Share ID
    pub share_id: String,
    /// IPS document hash
    pub ips_hash: ActionHash,
    /// Patient hash
    pub patient_hash: ActionHash,
    /// Recipient country
    pub recipient_country: String,
    /// Recipient organization
    pub recipient_organization: String,
    /// Recipient identifier
    pub recipient_identifier: Option<String>,
    /// Purpose of sharing
    pub purpose: String,
    /// Shared timestamp
    pub shared_at: Timestamp,
    /// Expiration timestamp
    pub expires_at: Option<Timestamp>,
    /// Access count
    pub access_count: u32,
    /// Last accessed
    pub last_accessed_at: Option<Timestamp>,
    /// Consent reference
    pub consent_hash: Option<ActionHash>,
    /// Was translated
    pub was_translated: bool,
    /// Translation languages
    pub translation_languages: Vec<String>,
}

/// IPS translation record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IpsTranslation {
    /// Translation ID
    pub translation_id: String,
    /// Original IPS hash
    pub original_ips_hash: ActionHash,
    /// Source language
    pub source_language: String,
    /// Target language
    pub target_language: String,
    /// Translation method (human, machine, hybrid)
    pub translation_method: String,
    /// Translator organization (if human/hybrid)
    pub translator: Option<String>,
    /// Translated FHIR bundle
    pub translated_bundle: String,
    /// Quality score (0-100)
    pub quality_score: Option<u32>,
    /// Verified by human
    pub human_verified: bool,
    /// Verification date
    pub verified_at: Option<Timestamp>,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Entry types for the IPS zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    InternationalPatientSummary(InternationalPatientSummary),
    IpsAllergy(IpsAllergy),
    IpsMedication(IpsMedication),
    IpsProblem(IpsProblem),
    IpsImmunization(IpsImmunization),
    IpsProcedure(IpsProcedure),
    IpsDevice(IpsDevice),
    IpsResult(IpsResult),
    IpsShareRecord(IpsShareRecord),
    IpsTranslation(IpsTranslation),
}

/// Link types for the IPS zome
#[hdk_link_types]
pub enum LinkTypes {
    /// Patient to their IPS documents
    PatientToIps,
    /// IPS to allergies
    IpsToAllergies,
    /// IPS to medications
    IpsToMedications,
    /// IPS to problems
    IpsToProblems,
    /// IPS to immunizations
    IpsToImmunizations,
    /// IPS to procedures
    IpsToProcedures,
    /// IPS to devices
    IpsToDevices,
    /// IPS to results
    IpsToResults,
    /// IPS to share records
    IpsToShares,
    /// IPS to translations
    IpsToTranslations,
    /// All IPS documents
    AllIpsDocuments,
    /// IPS by country
    IpsByCountry,
}

/// Validation callbacks
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::InternationalPatientSummary(ips) => validate_ips(&ips),
                EntryTypes::IpsAllergy(allergy) => validate_allergy(&allergy),
                EntryTypes::IpsMedication(med) => validate_medication(&med),
                _ => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_ips(ips: &InternationalPatientSummary) -> ExternResult<ValidateCallbackResult> {
    if ips.ips_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("IPS ID is required".to_string()));
    }
    if ips.language.len() != 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "Language must be ISO 639-1 (2 characters)".to_string(),
        ));
    }
    if ips.country_of_origin.len() != 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "Country must be ISO 3166-1 alpha-2 (2 characters)".to_string(),
        ));
    }
    // IPS requires at least 3 sections: allergies, medications, problems
    let required_sections = [
        IpsSection::AllergiesIntolerances,
        IpsSection::MedicationSummary,
        IpsSection::ProblemList,
    ];
    for section in required_sections.iter() {
        if !ips.sections_included.contains(section) {
            return Ok(ValidateCallbackResult::Invalid(format!(
                "IPS requires {:?} section",
                section
            )));
        }
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_allergy(allergy: &IpsAllergy) -> ExternResult<ValidateCallbackResult> {
    if allergy.allergy_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Allergy ID is required".to_string()));
    }
    if allergy.agent_code.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Agent code is required".to_string()));
    }
    if allergy.agent_display.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Agent display name is required".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_medication(med: &IpsMedication) -> ExternResult<ValidateCallbackResult> {
    if med.medication_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Medication ID is required".to_string()));
    }
    if med.medication_code.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Medication code is required".to_string()));
    }
    if med.medication_display.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Medication display name is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
