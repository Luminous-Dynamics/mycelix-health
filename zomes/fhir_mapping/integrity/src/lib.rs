//! FHIR R4 Resource Mapping Integrity Zome
//!
//! Defines entry types for FHIR R4 resource mappings, enabling
//! interoperability with external healthcare systems through
//! standardized clinical data exchange.
//!
//! Supports:
//! - Patient resource mapping
//! - Observation resource mapping (vital signs, lab results)
//! - Condition resource mapping (diagnoses)
//! - Medication resource mapping
//! - Bundle operations for bulk data exchange

use hdi::prelude::*;

// ============================================================================
// FHIR Common Types
// ============================================================================

/// FHIR Identifier structure (common across resources)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FhirIdentifier {
    /// Identifier system URI (e.g., "http://hl7.org/fhir/sid/us-npi")
    pub system: String,
    /// Identifier value
    pub value: String,
    /// Identifier use (usual, official, temp, secondary, old)
    pub use_code: Option<String>,
    /// Human-readable type description
    pub type_display: Option<String>,
}

/// FHIR Coding structure for coded values
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FhirCoding {
    /// Code system URI (e.g., "http://loinc.org", "http://snomed.info/sct")
    pub system: String,
    /// Code value
    pub code: String,
    /// Human-readable display
    pub display: Option<String>,
    /// Code version
    pub version: Option<String>,
}

/// FHIR CodeableConcept for multiple codings
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FhirCodeableConcept {
    /// List of codings
    pub coding: Vec<FhirCoding>,
    /// Plain text representation
    pub text: Option<String>,
}

/// FHIR Quantity for measurements
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FhirQuantity {
    /// Numeric value
    pub value: f64,
    /// Unit display string
    pub unit: String,
    /// UCUM system for units
    pub system: Option<String>,
    /// UCUM code
    pub code: Option<String>,
    /// Comparator (<, <=, >=, >)
    pub comparator: Option<String>,
}

/// FHIR Period for time ranges
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FhirPeriod {
    /// Start datetime (ISO 8601)
    pub start: Option<String>,
    /// End datetime (ISO 8601)
    pub end: Option<String>,
}

/// FHIR Reference to another resource
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FhirReference {
    /// Reference string (e.g., "Patient/123")
    pub reference: Option<String>,
    /// Resource type
    pub type_name: Option<String>,
    /// Logical identifier
    pub identifier: Option<FhirIdentifier>,
    /// Display text
    pub display: Option<String>,
}

/// FHIR HumanName for patient/practitioner names
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FhirHumanName {
    /// Name use (usual, official, temp, nickname, anonymous, old, maiden)
    pub use_code: Option<String>,
    /// Full text representation
    pub text: Option<String>,
    /// Family name (surname)
    pub family: Option<String>,
    /// Given names (first, middle)
    pub given: Vec<String>,
    /// Name prefix (Mr., Dr., etc.)
    pub prefix: Vec<String>,
    /// Name suffix (Jr., III, etc.)
    pub suffix: Vec<String>,
}

/// FHIR Address structure
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FhirAddress {
    /// Address use (home, work, temp, old, billing)
    pub use_code: Option<String>,
    /// Address type (postal, physical, both)
    pub type_code: Option<String>,
    /// Full text representation
    pub text: Option<String>,
    /// Street address lines
    pub line: Vec<String>,
    /// City
    pub city: Option<String>,
    /// State/Province
    pub state: Option<String>,
    /// Postal/ZIP code
    pub postal_code: Option<String>,
    /// Country
    pub country: Option<String>,
}

/// FHIR ContactPoint for phone/email
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FhirContactPoint {
    /// Contact system (phone, fax, email, pager, url, sms, other)
    pub system: Option<String>,
    /// Contact value
    pub value: Option<String>,
    /// Contact use (home, work, temp, old, mobile)
    pub use_code: Option<String>,
    /// Ranking preference (1 = highest)
    pub rank: Option<u32>,
}

/// FHIR Dosage for medication instructions
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FhirDosage {
    /// Dosage sequence number
    pub sequence: Option<i32>,
    /// Free text dosage instructions
    pub text: Option<String>,
    /// Additional patient instructions
    pub patient_instruction: Option<String>,
    /// Timing (e.g., "twice daily")
    pub timing_text: Option<String>,
    /// Route of administration
    pub route: Option<FhirCodeableConcept>,
    /// Dose quantity
    pub dose_quantity: Option<FhirQuantity>,
    /// Maximum dose per period
    pub max_dose_per_period: Option<String>,
}

// ============================================================================
// FHIR Resource Mapping Entry Types
// ============================================================================

/// Mapping between internal patient and FHIR Patient resource
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FhirPatientMapping {
    /// Internal Mycelix patient record hash
    pub internal_patient_hash: ActionHash,
    /// FHIR Patient resource ID (from external system)
    pub fhir_patient_id: String,
    /// Source system identifier (e.g., "epic", "cerner", "allscripts")
    pub source_system: String,
    /// FHIR identifiers (MRN, SSN, etc.)
    pub fhir_identifiers: Vec<FhirIdentifier>,
    /// Patient name(s)
    pub name: Vec<FhirHumanName>,
    /// Telecom contacts
    pub telecom: Vec<FhirContactPoint>,
    /// Patient gender (male, female, other, unknown)
    pub gender: Option<String>,
    /// Birth date (YYYY-MM-DD)
    pub birth_date: Option<String>,
    /// Deceased indicator or datetime
    pub deceased: Option<String>,
    /// Patient addresses
    pub address: Vec<FhirAddress>,
    /// Marital status
    pub marital_status: Option<FhirCodeableConcept>,
    /// Communication preferences/languages
    pub communication: Vec<FhirCoding>,
    /// FHIR resource version ID
    pub fhir_version_id: Option<String>,
    /// Last modification timestamp in source system
    pub fhir_last_updated: Option<String>,
    /// Mapping version for schema evolution
    pub mapping_version: String,
    /// Last synced with external system
    pub last_synced: Timestamp,
    /// Sync status (synced, pending, conflict, error)
    pub sync_status: SyncStatus,
    /// Any sync error messages
    pub sync_errors: Vec<String>,
}

/// Mapping between internal medical record and FHIR Observation resource
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FhirObservationMapping {
    /// Internal Mycelix record hash
    pub internal_record_hash: ActionHash,
    /// Patient this observation belongs to
    pub patient_hash: ActionHash,
    /// FHIR Observation resource ID
    pub fhir_observation_id: String,
    /// Source system identifier
    pub source_system: String,
    /// Observation status (registered, preliminary, final, amended, etc.)
    pub status: String,
    /// Category (vital-signs, laboratory, imaging, etc.)
    pub category: Vec<FhirCodeableConcept>,
    /// Observation code (LOINC)
    pub code: FhirCodeableConcept,
    /// LOINC code for quick lookup
    pub loinc_code: String,
    /// SNOMED code if available
    pub snomed_code: Option<String>,
    /// Observation value (quantity)
    pub value_quantity: Option<FhirQuantity>,
    /// Observation value (codeable concept)
    pub value_codeable_concept: Option<FhirCodeableConcept>,
    /// Observation value (string)
    pub value_string: Option<String>,
    /// Observation value (boolean)
    pub value_boolean: Option<bool>,
    /// Clinically relevant time
    pub effective_datetime: Timestamp,
    /// When the observation was recorded
    pub issued: Option<Timestamp>,
    /// Reference range for the observation
    pub reference_range: Option<ObservationReferenceRange>,
    /// Interpretation codes (normal, abnormal, critical, etc.)
    pub interpretation: Vec<FhirCodeableConcept>,
    /// Notes/comments
    pub note: Vec<String>,
    /// Mapping version
    pub mapping_version: String,
    /// Last synced
    pub last_synced: Timestamp,
}

/// Reference range for observations
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ObservationReferenceRange {
    /// Low bound
    pub low: Option<FhirQuantity>,
    /// High bound
    pub high: Option<FhirQuantity>,
    /// Reference range type
    pub type_code: Option<FhirCodeableConcept>,
    /// Applicable age range
    pub age: Option<String>,
    /// Text description
    pub text: Option<String>,
}

/// Mapping between internal diagnosis and FHIR Condition resource
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FhirConditionMapping {
    /// Internal Mycelix diagnosis hash
    pub internal_diagnosis_hash: ActionHash,
    /// Patient this condition belongs to
    pub patient_hash: ActionHash,
    /// FHIR Condition resource ID
    pub fhir_condition_id: String,
    /// Source system identifier
    pub source_system: String,
    /// Clinical status (active, recurrence, relapse, inactive, remission, resolved)
    pub clinical_status: String,
    /// Verification status (unconfirmed, provisional, differential, confirmed, refuted, entered-in-error)
    pub verification_status: String,
    /// Condition category (problem-list-item, encounter-diagnosis, health-concern)
    pub category: Vec<FhirCodeableConcept>,
    /// Severity (mild, moderate, severe)
    pub severity: Option<FhirCodeableConcept>,
    /// Condition code
    pub code: FhirCodeableConcept,
    /// ICD-10 code for quick lookup
    pub icd10_code: String,
    /// SNOMED code if available
    pub snomed_code: Option<String>,
    /// Body site affected
    pub body_site: Vec<FhirCodeableConcept>,
    /// Onset datetime or age
    pub onset_datetime: Option<Timestamp>,
    /// Abatement (resolution) datetime
    pub abatement_datetime: Option<Timestamp>,
    /// When condition was recorded
    pub recorded_date: Option<Timestamp>,
    /// Who recorded the condition
    pub recorder_reference: Option<FhirReference>,
    /// Who asserted the condition
    pub asserter_reference: Option<FhirReference>,
    /// Clinical notes
    pub note: Vec<String>,
    /// Mapping version
    pub mapping_version: String,
    /// Last synced
    pub last_synced: Timestamp,
}

/// Mapping between internal medication and FHIR MedicationRequest resource
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FhirMedicationMapping {
    /// Internal Mycelix medication hash
    pub internal_medication_hash: ActionHash,
    /// Patient this medication is for
    pub patient_hash: ActionHash,
    /// FHIR MedicationRequest resource ID
    pub fhir_medication_id: String,
    /// Source system identifier
    pub source_system: String,
    /// Request status (active, on-hold, cancelled, completed, entered-in-error, stopped, draft, unknown)
    pub status: String,
    /// Intent (proposal, plan, order, original-order, reflex-order, filler-order, instance-order, option)
    pub intent: String,
    /// Medication code
    pub medication_codeable_concept: FhirCodeableConcept,
    /// RxNorm code for quick lookup
    pub rxnorm_code: String,
    /// NDC code if available
    pub ndc_code: Option<String>,
    /// Prescriber reference
    pub requester_reference: Option<FhirReference>,
    /// Reason for medication
    pub reason_code: Vec<FhirCodeableConcept>,
    /// Dosage instructions
    pub dosage_instruction: Vec<FhirDosage>,
    /// Dispense request details
    pub dispense_quantity: Option<FhirQuantity>,
    /// Number of refills authorized
    pub dispense_refills: Option<u32>,
    /// Validity period
    pub validity_period: Option<FhirPeriod>,
    /// Date prescription was written
    pub authored_on: Option<Timestamp>,
    /// Notes
    pub note: Vec<String>,
    /// Mapping version
    pub mapping_version: String,
    /// Last synced
    pub last_synced: Timestamp,
}

/// FHIR Bundle for bulk data operations
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FhirBundleRecord {
    /// Bundle ID
    pub bundle_id: String,
    /// Bundle type (document, message, transaction, transaction-response, batch, batch-response, history, searchset, collection)
    pub bundle_type: String,
    /// Total count of resources
    pub total: u32,
    /// Timestamp when bundle was created
    pub timestamp: Timestamp,
    /// Patient this bundle is for (if applicable)
    pub patient_hash: Option<ActionHash>,
    /// Resource type summary (count per type)
    pub resource_summary: Vec<ResourceTypeSummary>,
    /// Bundle status
    pub status: BundleStatus,
    /// Processing errors
    pub errors: Vec<String>,
}

/// Summary of resource types in a bundle
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ResourceTypeSummary {
    pub resource_type: String,
    pub count: u32,
}

/// Bundle processing status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BundleStatus {
    Pending,
    Processing,
    Completed,
    PartiallyCompleted,
    Failed,
}

/// Sync status for mappings
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    Synced,
    Pending,
    Conflict,
    Error,
}

/// Terminology validation result
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TerminologyValidation {
    /// Code system (loinc, snomed, icd10, rxnorm)
    pub code_system: String,
    /// Code being validated
    pub code: String,
    /// Display text
    pub display: Option<String>,
    /// Whether the code is valid
    pub is_valid: bool,
    /// Validation message
    pub message: Option<String>,
    /// When validation was performed
    pub validated_at: Timestamp,
}

// ============================================================================
// Entry and Link Type Enums
// ============================================================================

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    FhirPatientMapping(FhirPatientMapping),
    FhirObservationMapping(FhirObservationMapping),
    FhirConditionMapping(FhirConditionMapping),
    FhirMedicationMapping(FhirMedicationMapping),
    FhirBundleRecord(FhirBundleRecord),
    TerminologyValidation(TerminologyValidation),
}

#[hdk_link_types]
pub enum LinkTypes {
    /// Patient to their FHIR mappings
    PatientToFhirMappings,
    /// Internal record to FHIR observation
    RecordToFhirObservation,
    /// Internal diagnosis to FHIR condition
    DiagnosisToFhirCondition,
    /// Internal medication to FHIR medication request
    MedicationToFhirMapping,
    /// Patient to their FHIR bundles
    PatientToBundles,
    /// Source system to all its mappings
    SourceSystemMappings,
    /// All FHIR patient mappings
    AllFhirPatientMappings,
    /// Bundle to its entries
    BundleToEntries,
    /// Updates tracking
    FhirMappingUpdates,
}

// ============================================================================
// Validation Functions
// ============================================================================

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => validate_create_entry(app_entry),
            OpEntry::UpdateEntry { app_entry, .. } => validate_create_entry(app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterCreateLink { link_type, .. } => validate_link(link_type),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(entry: EntryTypes) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::FhirPatientMapping(mapping) => validate_fhir_patient_mapping(&mapping),
        EntryTypes::FhirObservationMapping(mapping) => validate_fhir_observation_mapping(&mapping),
        EntryTypes::FhirConditionMapping(mapping) => validate_fhir_condition_mapping(&mapping),
        EntryTypes::FhirMedicationMapping(mapping) => validate_fhir_medication_mapping(&mapping),
        EntryTypes::FhirBundleRecord(bundle) => validate_fhir_bundle(&bundle),
        EntryTypes::TerminologyValidation(validation) => validate_terminology_validation(&validation),
    }
}

fn validate_fhir_patient_mapping(mapping: &FhirPatientMapping) -> ExternResult<ValidateCallbackResult> {
    // Validate FHIR patient ID is not empty
    if mapping.fhir_patient_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "FHIR patient ID cannot be empty".to_string(),
        ));
    }

    // Validate source system is specified
    if mapping.source_system.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Source system must be specified".to_string(),
        ));
    }

    // Validate mapping version
    if mapping.mapping_version.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Mapping version is required".to_string(),
        ));
    }

    // Validate at least one name is provided
    if mapping.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one name must be provided for FHIR patient mapping".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_fhir_observation_mapping(mapping: &FhirObservationMapping) -> ExternResult<ValidateCallbackResult> {
    // Validate FHIR observation ID
    if mapping.fhir_observation_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "FHIR observation ID cannot be empty".to_string(),
        ));
    }

    // Validate LOINC code is provided
    if mapping.loinc_code.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "LOINC code is required for observations".to_string(),
        ));
    }

    // Validate status is valid FHIR status
    let valid_statuses = ["registered", "preliminary", "final", "amended", "corrected", "cancelled", "entered-in-error", "unknown"];
    if !valid_statuses.contains(&mapping.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid observation status: {}. Must be one of: {:?}", mapping.status, valid_statuses),
        ));
    }

    // Validate at least one value is provided
    if mapping.value_quantity.is_none()
        && mapping.value_codeable_concept.is_none()
        && mapping.value_string.is_none()
        && mapping.value_boolean.is_none()
    {
        return Ok(ValidateCallbackResult::Invalid(
            "Observation must have at least one value".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_fhir_condition_mapping(mapping: &FhirConditionMapping) -> ExternResult<ValidateCallbackResult> {
    // Validate FHIR condition ID
    if mapping.fhir_condition_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "FHIR condition ID cannot be empty".to_string(),
        ));
    }

    // Validate ICD-10 code is provided
    if mapping.icd10_code.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ICD-10 code is required for conditions".to_string(),
        ));
    }

    // Validate clinical status
    let valid_clinical = ["active", "recurrence", "relapse", "inactive", "remission", "resolved"];
    if !valid_clinical.contains(&mapping.clinical_status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid clinical status: {}. Must be one of: {:?}", mapping.clinical_status, valid_clinical),
        ));
    }

    // Validate verification status
    let valid_verification = ["unconfirmed", "provisional", "differential", "confirmed", "refuted", "entered-in-error"];
    if !valid_verification.contains(&mapping.verification_status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid verification status: {}. Must be one of: {:?}", mapping.verification_status, valid_verification),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_fhir_medication_mapping(mapping: &FhirMedicationMapping) -> ExternResult<ValidateCallbackResult> {
    // Validate FHIR medication ID
    if mapping.fhir_medication_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "FHIR medication ID cannot be empty".to_string(),
        ));
    }

    // Validate RxNorm code is provided
    if mapping.rxnorm_code.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RxNorm code is required for medications".to_string(),
        ));
    }

    // Validate status
    let valid_statuses = ["active", "on-hold", "cancelled", "completed", "entered-in-error", "stopped", "draft", "unknown"];
    if !valid_statuses.contains(&mapping.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid medication status: {}. Must be one of: {:?}", mapping.status, valid_statuses),
        ));
    }

    // Validate intent
    let valid_intents = ["proposal", "plan", "order", "original-order", "reflex-order", "filler-order", "instance-order", "option"];
    if !valid_intents.contains(&mapping.intent.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid medication intent: {}. Must be one of: {:?}", mapping.intent, valid_intents),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_fhir_bundle(bundle: &FhirBundleRecord) -> ExternResult<ValidateCallbackResult> {
    // Validate bundle ID
    if bundle.bundle_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Bundle ID cannot be empty".to_string(),
        ));
    }

    // Validate bundle type
    let valid_types = ["document", "message", "transaction", "transaction-response", "batch", "batch-response", "history", "searchset", "collection"];
    if !valid_types.contains(&bundle.bundle_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid bundle type: {}. Must be one of: {:?}", bundle.bundle_type, valid_types),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_terminology_validation(validation: &TerminologyValidation) -> ExternResult<ValidateCallbackResult> {
    // Validate code system is supported
    let supported_systems = ["loinc", "snomed", "icd10", "icd10-cm", "rxnorm", "ndc", "cpt"];
    if !supported_systems.contains(&validation.code_system.to_lowercase().as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Unsupported code system: {}. Supported: {:?}", validation.code_system, supported_systems),
        ));
    }

    // Validate code is not empty
    if validation.code.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Code cannot be empty".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_link(link_type: LinkTypes) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        LinkTypes::PatientToFhirMappings => Ok(ValidateCallbackResult::Valid),
        LinkTypes::RecordToFhirObservation => Ok(ValidateCallbackResult::Valid),
        LinkTypes::DiagnosisToFhirCondition => Ok(ValidateCallbackResult::Valid),
        LinkTypes::MedicationToFhirMapping => Ok(ValidateCallbackResult::Valid),
        LinkTypes::PatientToBundles => Ok(ValidateCallbackResult::Valid),
        LinkTypes::SourceSystemMappings => Ok(ValidateCallbackResult::Valid),
        LinkTypes::AllFhirPatientMappings => Ok(ValidateCallbackResult::Valid),
        LinkTypes::BundleToEntries => Ok(ValidateCallbackResult::Valid),
        LinkTypes::FhirMappingUpdates => Ok(ValidateCallbackResult::Valid),
    }
}
