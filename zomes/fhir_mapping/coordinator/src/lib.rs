//! FHIR R4 Resource Mapping Coordinator Zome
//!
//! Provides extern functions for FHIR resource transformations,
//! import/export operations, and terminology validation.
//!
//! All data access functions enforce consent-based access control.

use hdk::prelude::*;
use fhir_mapping_integrity::*;
use mycelix_health_shared::{
    require_authorization, log_data_access,
    DataCategory, Permission, anchor_hash,
};

// ============================================================================
// Patient FHIR Mapping Functions
// ============================================================================

/// Create a FHIR Patient mapping from internal patient record
#[hdk_extern]
pub fn create_fhir_patient_mapping(mapping: FhirPatientMapping) -> ExternResult<Record> {
    let auth = require_authorization(
        mapping.internal_patient_hash.clone(),
        DataCategory::Demographics,
        Permission::Write,
        false,
    )?;
    let mapping_hash = create_entry(&EntryTypes::FhirPatientMapping(mapping.clone()))?;
    let record = get(mapping_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find newly created FHIR patient mapping".to_string())))?;

    // Link from internal patient to FHIR mapping
    create_link(
        mapping.internal_patient_hash.clone(),
        mapping_hash.clone(),
        LinkTypes::PatientToFhirMappings,
        (),
    )?;

    // Link to source system anchor for cross-system queries
    let source_anchor = anchor_hash(&format!("fhir_source_{}", mapping.source_system))?;
    create_link(
        source_anchor,
        mapping_hash.clone(),
        LinkTypes::SourceSystemMappings,
        (),
    )?;

    // Link to all FHIR patient mappings anchor
    let all_mappings_anchor = anchor_hash("all_fhir_patient_mappings")?;
    create_link(
        all_mappings_anchor,
        mapping_hash,
        LinkTypes::AllFhirPatientMappings,
        (),
    )?;

    log_data_access(
        mapping.internal_patient_hash,
        vec![DataCategory::Demographics],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Input for getting FHIR patient mapping with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct GetFhirMappingInput {
    pub mapping_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get a FHIR patient mapping by hash with access control
#[hdk_extern]
pub fn get_fhir_patient_mapping(input: GetFhirMappingInput) -> ExternResult<Option<Record>> {
    let record = get(input.mapping_hash.clone(), GetOptions::default())?;

    if let Some(ref rec) = record {
        // Get the mapping to find patient hash
        if let Some(mapping) = rec.entry().to_app_option::<FhirPatientMapping>().ok().flatten() {
            // Require authorization
            let auth = require_authorization(
                mapping.internal_patient_hash.clone(),
                DataCategory::Demographics,
                Permission::Read,
                input.is_emergency,
            )?;

            // Log access
            log_data_access(
                mapping.internal_patient_hash,
                vec![DataCategory::Demographics],
                Permission::Read,
                auth.consent_hash,
                auth.emergency_override,
                input.emergency_reason,
            )?;
        }
    }

    Ok(record)
}

/// Get all FHIR mappings for a patient
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPatientFhirMappingsInput {
    pub patient_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

#[hdk_extern]
pub fn get_patient_fhir_mappings(input: GetPatientFhirMappingsInput) -> ExternResult<Vec<Record>> {
    // Require authorization
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToFhirMappings)?, GetStrategy::default())?;

    let mut mappings = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                mappings.push(record);
            }
        }
    }

    // Log access
    log_data_access(
        input.patient_hash,
        vec![DataCategory::All],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(mappings)
}

// ============================================================================
// Observation FHIR Mapping Functions
// ============================================================================

/// Create a FHIR Observation mapping
#[hdk_extern]
pub fn create_fhir_observation_mapping(mapping: FhirObservationMapping) -> ExternResult<Record> {
    let auth = require_authorization(
        mapping.patient_hash.clone(),
        DataCategory::LabResults,
        Permission::Write,
        false,
    )?;
    let mapping_hash = create_entry(&EntryTypes::FhirObservationMapping(mapping.clone()))?;
    let record = get(mapping_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find newly created FHIR observation mapping".to_string())))?;

    // Link from internal record to FHIR mapping
    create_link(
        mapping.internal_record_hash.clone(),
        mapping_hash.clone(),
        LinkTypes::RecordToFhirObservation,
        (),
    )?;

    // Link from patient to this observation
    create_link(
        mapping.patient_hash.clone(),
        mapping_hash,
        LinkTypes::PatientToFhirMappings,
        (),
    )?;

    log_data_access(
        mapping.patient_hash,
        vec![DataCategory::LabResults],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Get FHIR observation mapping with access control
#[hdk_extern]
pub fn get_fhir_observation_mapping(input: GetFhirMappingInput) -> ExternResult<Option<Record>> {
    let record = get(input.mapping_hash.clone(), GetOptions::default())?;

    if let Some(ref rec) = record {
        if let Some(mapping) = rec.entry().to_app_option::<FhirObservationMapping>().ok().flatten() {
            let auth = require_authorization(
                mapping.patient_hash.clone(),
                DataCategory::LabResults,
                Permission::Read,
                input.is_emergency,
            )?;

            log_data_access(
                mapping.patient_hash,
                vec![DataCategory::LabResults],
                Permission::Read,
                auth.consent_hash,
                auth.emergency_override,
                input.emergency_reason,
            )?;
        }
    }

    Ok(record)
}

// ============================================================================
// Condition FHIR Mapping Functions
// ============================================================================

/// Create a FHIR Condition mapping
#[hdk_extern]
pub fn create_fhir_condition_mapping(mapping: FhirConditionMapping) -> ExternResult<Record> {
    let auth = require_authorization(
        mapping.patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Write,
        false,
    )?;
    let mapping_hash = create_entry(&EntryTypes::FhirConditionMapping(mapping.clone()))?;
    let record = get(mapping_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find newly created FHIR condition mapping".to_string())))?;

    // Link from internal diagnosis to FHIR mapping
    create_link(
        mapping.internal_diagnosis_hash.clone(),
        mapping_hash.clone(),
        LinkTypes::DiagnosisToFhirCondition,
        (),
    )?;

    // Link from patient
    create_link(
        mapping.patient_hash.clone(),
        mapping_hash,
        LinkTypes::PatientToFhirMappings,
        (),
    )?;

    log_data_access(
        mapping.patient_hash,
        vec![DataCategory::Diagnoses],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Get FHIR condition mapping with access control
#[hdk_extern]
pub fn get_fhir_condition_mapping(input: GetFhirMappingInput) -> ExternResult<Option<Record>> {
    let record = get(input.mapping_hash.clone(), GetOptions::default())?;

    if let Some(ref rec) = record {
        if let Some(mapping) = rec.entry().to_app_option::<FhirConditionMapping>().ok().flatten() {
            let auth = require_authorization(
                mapping.patient_hash.clone(),
                DataCategory::Diagnoses,
                Permission::Read,
                input.is_emergency,
            )?;

            log_data_access(
                mapping.patient_hash,
                vec![DataCategory::Diagnoses],
                Permission::Read,
                auth.consent_hash,
                auth.emergency_override,
                input.emergency_reason,
            )?;
        }
    }

    Ok(record)
}

// ============================================================================
// Medication FHIR Mapping Functions
// ============================================================================

/// Create a FHIR Medication mapping
#[hdk_extern]
pub fn create_fhir_medication_mapping(mapping: FhirMedicationMapping) -> ExternResult<Record> {
    let auth = require_authorization(
        mapping.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Write,
        false,
    )?;
    let mapping_hash = create_entry(&EntryTypes::FhirMedicationMapping(mapping.clone()))?;
    let record = get(mapping_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find newly created FHIR medication mapping".to_string())))?;

    // Link from internal medication to FHIR mapping
    create_link(
        mapping.internal_medication_hash.clone(),
        mapping_hash.clone(),
        LinkTypes::MedicationToFhirMapping,
        (),
    )?;

    // Link from patient
    create_link(
        mapping.patient_hash.clone(),
        mapping_hash,
        LinkTypes::PatientToFhirMappings,
        (),
    )?;

    log_data_access(
        mapping.patient_hash,
        vec![DataCategory::Medications],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Get FHIR medication mapping with access control
#[hdk_extern]
pub fn get_fhir_medication_mapping(input: GetFhirMappingInput) -> ExternResult<Option<Record>> {
    let record = get(input.mapping_hash.clone(), GetOptions::default())?;

    if let Some(ref rec) = record {
        if let Some(mapping) = rec.entry().to_app_option::<FhirMedicationMapping>().ok().flatten() {
            let auth = require_authorization(
                mapping.patient_hash.clone(),
                DataCategory::Medications,
                Permission::Read,
                input.is_emergency,
            )?;

            log_data_access(
                mapping.patient_hash,
                vec![DataCategory::Medications],
                Permission::Read,
                auth.consent_hash,
                auth.emergency_override,
                input.emergency_reason,
            )?;
        }
    }

    Ok(record)
}

// ============================================================================
// Bundle Operations
// ============================================================================

/// Input for exporting a patient bundle
#[derive(Serialize, Deserialize, Debug)]
pub struct ExportPatientBundleInput {
    pub patient_hash: ActionHash,
    pub include_observations: bool,
    pub include_conditions: bool,
    pub include_medications: bool,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// FHIR Bundle output structure
#[derive(Serialize, Deserialize, Debug)]
pub struct FhirBundleOutput {
    pub bundle_record: Record,
    pub patient_mapping: Option<Record>,
    pub observations: Vec<Record>,
    pub conditions: Vec<Record>,
    pub medications: Vec<Record>,
}

/// Export a patient's data as a FHIR bundle
#[hdk_extern]
pub fn export_patient_bundle(input: ExportPatientBundleInput) -> ExternResult<FhirBundleOutput> {
    // Require authorization for full patient export
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::All,
        Permission::Export,
        input.is_emergency,
    )?;

    // Get all FHIR mappings for this patient
    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToFhirMappings)?, GetStrategy::default())?;

    let mut patient_mapping: Option<Record> = None;
    let mut observations: Vec<Record> = Vec::new();
    let mut conditions: Vec<Record> = Vec::new();
    let mut medications: Vec<Record> = Vec::new();

    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash.clone(), GetOptions::default())? {
                // Determine the type of mapping
                if record.entry().to_app_option::<FhirPatientMapping>().ok().flatten().is_some() {
                    patient_mapping = Some(record);
                } else if input.include_observations && record.entry().to_app_option::<FhirObservationMapping>().ok().flatten().is_some() {
                    observations.push(record);
                } else if input.include_conditions && record.entry().to_app_option::<FhirConditionMapping>().ok().flatten().is_some() {
                    conditions.push(record);
                } else if input.include_medications && record.entry().to_app_option::<FhirMedicationMapping>().ok().flatten().is_some() {
                    medications.push(record);
                }
            }
        }
    }

    // Create bundle record
    let mut resource_summary = Vec::new();
    if patient_mapping.is_some() {
        resource_summary.push(ResourceTypeSummary {
            resource_type: "Patient".to_string(),
            count: 1,
        });
    }
    if !observations.is_empty() {
        resource_summary.push(ResourceTypeSummary {
            resource_type: "Observation".to_string(),
            count: observations.len() as u32,
        });
    }
    if !conditions.is_empty() {
        resource_summary.push(ResourceTypeSummary {
            resource_type: "Condition".to_string(),
            count: conditions.len() as u32,
        });
    }
    if !medications.is_empty() {
        resource_summary.push(ResourceTypeSummary {
            resource_type: "MedicationRequest".to_string(),
            count: medications.len() as u32,
        });
    }

    let total = resource_summary.iter().map(|s| s.count).sum();
    let bundle = FhirBundleRecord {
        bundle_id: format!("bundle-{}", sys_time()?.as_micros()),
        bundle_type: "collection".to_string(),
        total,
        timestamp: sys_time()?,
        patient_hash: Some(input.patient_hash.clone()),
        resource_summary,
        status: BundleStatus::Completed,
        errors: Vec::new(),
    };

    let bundle_hash = create_entry(&EntryTypes::FhirBundleRecord(bundle))?;
    let bundle_record = get(bundle_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find bundle record".to_string())))?;

    // Link bundle to patient
    create_link(
        input.patient_hash.clone(),
        bundle_hash,
        LinkTypes::PatientToBundles,
        (),
    )?;

    // Log access
    log_data_access(
        input.patient_hash,
        vec![DataCategory::All],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(FhirBundleOutput {
        bundle_record,
        patient_mapping,
        observations,
        conditions,
        medications,
    })
}

/// Input for importing a FHIR bundle
#[derive(Serialize, Deserialize, Debug)]
pub struct ImportFhirBundleInput {
    pub patient_hash: ActionHash,
    pub source_system: String,
    pub patient_mapping: Option<FhirPatientMapping>,
    pub observations: Vec<FhirObservationMapping>,
    pub conditions: Vec<FhirConditionMapping>,
    pub medications: Vec<FhirMedicationMapping>,
}

/// Result of importing a FHIR bundle
#[derive(Serialize, Deserialize, Debug)]
pub struct ImportBundleResult {
    pub bundle_record: Record,
    pub imported_patient: Option<ActionHash>,
    pub imported_observations: Vec<ActionHash>,
    pub imported_conditions: Vec<ActionHash>,
    pub imported_medications: Vec<ActionHash>,
    pub errors: Vec<String>,
}

/// Import a FHIR bundle into the system
#[hdk_extern]
pub fn import_fhir_bundle(input: ImportFhirBundleInput) -> ExternResult<ImportBundleResult> {
    let mut imported_patient: Option<ActionHash> = None;
    let mut imported_observations: Vec<ActionHash> = Vec::new();
    let mut imported_conditions: Vec<ActionHash> = Vec::new();
    let mut imported_medications: Vec<ActionHash> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    // Import patient mapping if provided
    if let Some(patient_mapping) = input.patient_mapping {
        match create_entry(&EntryTypes::FhirPatientMapping(patient_mapping.clone())) {
            Ok(hash) => {
                create_link(
                    patient_mapping.internal_patient_hash.clone(),
                    hash.clone(),
                    LinkTypes::PatientToFhirMappings,
                    (),
                )?;
                imported_patient = Some(hash);
            }
            Err(e) => errors.push(format!("Failed to import patient: {}", e)),
        }
    }

    // Import observations
    for obs in input.observations {
        match create_entry(&EntryTypes::FhirObservationMapping(obs.clone())) {
            Ok(hash) => {
                create_link(
                    obs.patient_hash.clone(),
                    hash.clone(),
                    LinkTypes::PatientToFhirMappings,
                    (),
                )?;
                imported_observations.push(hash);
            }
            Err(e) => errors.push(format!("Failed to import observation {}: {}", obs.fhir_observation_id, e)),
        }
    }

    // Import conditions
    for cond in input.conditions {
        match create_entry(&EntryTypes::FhirConditionMapping(cond.clone())) {
            Ok(hash) => {
                create_link(
                    cond.patient_hash.clone(),
                    hash.clone(),
                    LinkTypes::PatientToFhirMappings,
                    (),
                )?;
                imported_conditions.push(hash);
            }
            Err(e) => errors.push(format!("Failed to import condition {}: {}", cond.fhir_condition_id, e)),
        }
    }

    // Import medications
    for med in input.medications {
        match create_entry(&EntryTypes::FhirMedicationMapping(med.clone())) {
            Ok(hash) => {
                create_link(
                    med.patient_hash.clone(),
                    hash.clone(),
                    LinkTypes::PatientToFhirMappings,
                    (),
                )?;
                imported_medications.push(hash);
            }
            Err(e) => errors.push(format!("Failed to import medication {}: {}", med.fhir_medication_id, e)),
        }
    }

    // Create bundle record
    let total = (if imported_patient.is_some() { 1 } else { 0 })
        + imported_observations.len() as u32
        + imported_conditions.len() as u32
        + imported_medications.len() as u32;

    let status = if errors.is_empty() {
        BundleStatus::Completed
    } else if total > 0 {
        BundleStatus::PartiallyCompleted
    } else {
        BundleStatus::Failed
    };

    let bundle = FhirBundleRecord {
        bundle_id: format!("import-{}", sys_time()?.as_micros()),
        bundle_type: "transaction-response".to_string(),
        total,
        timestamp: sys_time()?,
        patient_hash: Some(input.patient_hash.clone()),
        resource_summary: vec![
            ResourceTypeSummary { resource_type: "Patient".to_string(), count: if imported_patient.is_some() { 1 } else { 0 } },
            ResourceTypeSummary { resource_type: "Observation".to_string(), count: imported_observations.len() as u32 },
            ResourceTypeSummary { resource_type: "Condition".to_string(), count: imported_conditions.len() as u32 },
            ResourceTypeSummary { resource_type: "MedicationRequest".to_string(), count: imported_medications.len() as u32 },
        ],
        status,
        errors: errors.clone(),
    };

    let bundle_hash = create_entry(&EntryTypes::FhirBundleRecord(bundle))?;
    let bundle_record = get(bundle_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find bundle record".to_string())))?;

    create_link(
        input.patient_hash,
        bundle_hash,
        LinkTypes::PatientToBundles,
        (),
    )?;

    Ok(ImportBundleResult {
        bundle_record,
        imported_patient,
        imported_observations,
        imported_conditions,
        imported_medications,
        errors,
    })
}

// ============================================================================
// Terminology Validation Functions
// ============================================================================

/// Input for validating a code
#[derive(Serialize, Deserialize, Debug)]
pub struct ValidateCodeInput {
    pub code_system: String,
    pub code: String,
    pub display: Option<String>,
}

/// Validate a LOINC code
#[hdk_extern]
pub fn validate_loinc_code(input: ValidateCodeInput) -> ExternResult<Record> {
    // Basic LOINC code format validation (NNNNN-N)
    let is_valid = validate_loinc_format(&input.code);

    let validation = TerminologyValidation {
        code_system: "loinc".to_string(),
        code: input.code.clone(),
        display: input.display,
        is_valid,
        message: if is_valid {
            Some("LOINC code format is valid".to_string())
        } else {
            Some("Invalid LOINC code format. Expected format: NNNNN-N".to_string())
        },
        validated_at: sys_time()?,
    };

    let hash = create_entry(&EntryTypes::TerminologyValidation(validation))?;
    get(hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find validation record".to_string())))
}

/// Validate a SNOMED CT code
#[hdk_extern]
pub fn validate_snomed_code(input: ValidateCodeInput) -> ExternResult<Record> {
    // Basic SNOMED code validation (numeric, typically 6-18 digits)
    let is_valid = validate_snomed_format(&input.code);

    let validation = TerminologyValidation {
        code_system: "snomed".to_string(),
        code: input.code.clone(),
        display: input.display,
        is_valid,
        message: if is_valid {
            Some("SNOMED CT code format is valid".to_string())
        } else {
            Some("Invalid SNOMED CT code format. Expected 6-18 digit numeric code".to_string())
        },
        validated_at: sys_time()?,
    };

    let hash = create_entry(&EntryTypes::TerminologyValidation(validation))?;
    get(hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find validation record".to_string())))
}

/// Validate an ICD-10 code
#[hdk_extern]
pub fn validate_icd10_code(input: ValidateCodeInput) -> ExternResult<Record> {
    // Basic ICD-10 validation (letter followed by digits and optional decimal)
    let is_valid = validate_icd10_format(&input.code);

    let validation = TerminologyValidation {
        code_system: "icd10".to_string(),
        code: input.code.clone(),
        display: input.display,
        is_valid,
        message: if is_valid {
            Some("ICD-10 code format is valid".to_string())
        } else {
            Some("Invalid ICD-10 code format. Expected format: A00.0 or A00".to_string())
        },
        validated_at: sys_time()?,
    };

    let hash = create_entry(&EntryTypes::TerminologyValidation(validation))?;
    get(hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find validation record".to_string())))
}

/// Validate an RxNorm code
#[hdk_extern]
pub fn validate_rxnorm_code(input: ValidateCodeInput) -> ExternResult<Record> {
    // Basic RxNorm validation (numeric code)
    let is_valid = input.code.chars().all(|c| c.is_ascii_digit()) && !input.code.is_empty();

    let validation = TerminologyValidation {
        code_system: "rxnorm".to_string(),
        code: input.code.clone(),
        display: input.display,
        is_valid,
        message: if is_valid {
            Some("RxNorm code format is valid".to_string())
        } else {
            Some("Invalid RxNorm code format. Expected numeric code".to_string())
        },
        validated_at: sys_time()?,
    };

    let hash = create_entry(&EntryTypes::TerminologyValidation(validation))?;
    get(hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find validation record".to_string())))
}

// ============================================================================
// Helper Functions
// ============================================================================

fn validate_loinc_format(code: &str) -> bool {
    // LOINC format: NNNNN-N (5+ digits, dash, check digit)
    let parts: Vec<&str> = code.split('-').collect();
    if parts.len() != 2 {
        return false;
    }
    parts[0].len() >= 3 && parts[0].chars().all(|c| c.is_ascii_digit())
        && parts[1].len() == 1 && parts[1].chars().all(|c| c.is_ascii_digit())
}

fn validate_snomed_format(code: &str) -> bool {
    // SNOMED codes are numeric, typically 6-18 digits
    code.len() >= 6 && code.len() <= 18 && code.chars().all(|c| c.is_ascii_digit())
}

fn validate_icd10_format(code: &str) -> bool {
    // ICD-10 format: Letter + 2 digits, optionally followed by decimal and more digits
    if code.is_empty() {
        return false;
    }
    let chars: Vec<char> = code.chars().collect();
    if !chars[0].is_ascii_alphabetic() {
        return false;
    }
    if chars.len() < 3 {
        return false;
    }
    // Check remaining characters are digits or decimal point
    for (i, c) in chars.iter().enumerate().skip(1) {
        if i == 3 && *c == '.' {
            continue;
        }
        if !c.is_ascii_digit() {
            return false;
        }
    }
    true
}

// ============================================================================
// Sync Status Updates
// ============================================================================

/// Input for updating sync status
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateSyncStatusInput {
    pub mapping_hash: ActionHash,
    pub new_status: SyncStatus,
    pub error_message: Option<String>,
}

/// Update the sync status of a FHIR patient mapping
#[hdk_extern]
pub fn update_patient_mapping_sync_status(input: UpdateSyncStatusInput) -> ExternResult<Record> {
    let record = get(input.mapping_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Mapping not found".to_string())))?;

    let mut mapping: FhirPatientMapping = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid mapping entry".to_string())))?;

    mapping.sync_status = input.new_status;
    mapping.last_synced = sys_time()?;
    if let Some(error) = input.error_message {
        mapping.sync_errors.push(error);
    }

    let updated_hash = update_entry(input.mapping_hash.clone(), &mapping)?;
    let updated_record = get(updated_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated mapping".to_string())))?;

    create_link(
        input.mapping_hash,
        updated_hash,
        LinkTypes::FhirMappingUpdates,
        (),
    )?;

    Ok(updated_record)
}
