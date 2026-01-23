//! Medical Records Coordinator Zome
//!
//! Provides extern functions for encounters, diagnoses,
//! procedures, lab results, imaging, and vital signs.
//!
//! All data access functions enforce consent-based access control
//! per HIPAA requirements.
//!
//! ## Cross-Zome Integration
//!
//! When lab results or vital signs are recorded, this zome automatically
//! feeds the data to the patient's Health Twin (if one exists) for
//! continuous model updates and health predictions.

use hdk::prelude::*;
use mycelix_health_shared::{
    log_data_access, require_admin_authorization, require_authorization, DataCategory, Permission,
};
use records_integrity::*;

// ==================== HEALTH TWIN INTEGRATION ====================

/// Try to feed data to the patient's health twin (if one exists)
/// This is a best-effort operation - failures are logged but don't break the main operation
fn try_feed_to_health_twin(patient_hash: &ActionHash, data_point: TwinDataPointInput) {
    // Best effort - don't fail main operation if twin doesn't exist or has issues
    let _ = feed_to_health_twin_internal(patient_hash, data_point);
}

/// Internal function to feed data to health twin
fn feed_to_health_twin_internal(
    patient_hash: &ActionHash,
    data_point: TwinDataPointInput,
) -> ExternResult<()> {
    // First, check if patient has a twin
    let twin_response = call(
        CallTargetCell::Local,
        ZomeName::from("twin"),
        FunctionName::from("get_patient_twin"),
        None,
        patient_hash,
    )?;

    // Decode the response
    let twin_record: Option<Record> = match twin_response {
        ZomeCallResponse::Ok(io) => io.decode().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to decode twin response: {}",
                e
            )))
        })?,
        // Any non-success response means we can't reach the twin - that's ok
        _ => return Ok(()),
    };

    // If no twin exists, nothing to do
    let twin_record = match twin_record {
        Some(record) => record,
        None => return Ok(()),
    };

    let twin_hash = twin_record.action_address().clone();

    // Create the full data point with twin hash
    let full_data_point = TwinDataPointFull {
        data_point_id: format!("DP-{}", sys_time()?.as_micros()),
        twin_hash,
        data_type: data_point.data_type,
        value: data_point.value,
        unit: data_point.unit,
        measured_at: data_point.measured_at,
        source: data_point.source,
        quality: data_point.quality,
        triggered_update: true, // Always trigger model update for clinical data
        ingested_at: sys_time()?.as_micros() as i64,
    };

    // Call the twin zome to ingest the data point
    let ingest_response = call(
        CallTargetCell::Local,
        ZomeName::from("twin"),
        FunctionName::from("ingest_data_point"),
        None,
        &full_data_point,
    )?;

    // Check response but don't fail main operation
    match ingest_response {
        ZomeCallResponse::Ok(_) => Ok(()),
        _ => Ok(()), // Ignore errors - best effort
    }
}

/// Input for creating a twin data point (without twin hash, which we'll look up)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TwinDataPointInput {
    pub data_type: TwinDataType,
    pub value: String,
    pub unit: Option<String>,
    pub measured_at: i64,
    pub source: TwinDataSourceType,
    pub quality: TwinDataQuality,
}

/// Full twin data point structure (matches twin zome)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TwinDataPointFull {
    pub data_point_id: String,
    pub twin_hash: ActionHash,
    pub data_type: TwinDataType,
    pub value: String,
    pub unit: Option<String>,
    pub measured_at: i64,
    pub source: TwinDataSourceType,
    pub quality: TwinDataQuality,
    pub triggered_update: bool,
    pub ingested_at: i64,
}

/// Twin data types (mirrors twin_integrity)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TwinDataType {
    VitalSign(TwinVitalSignType),
    LabResult(String),
    Medication(String),
    Diagnosis(String),
    Procedure(String),
    Lifestyle(String),
    Symptom(String),
    BiometricReading(String),
    GeneticMarker(String),
    SocialDeterminant(String),
}

/// Vital sign types for twin
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TwinVitalSignType {
    HeartRate,
    BloodPressure,
    Temperature,
    RespiratoryRate,
    SpO2,
    Weight,
    Height,
    BMI,
}

/// Data source types (mirrors twin_integrity)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TwinDataSourceType {
    EHR,
    Laboratory,
    Wearable,
    SelfReported,
    Imaging,
    Pharmacy,
    Genetic,
    SocialDeterminants,
}

/// Data quality levels (mirrors twin_integrity)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TwinDataQuality {
    Clinical,
    Consumer,
    SelfReported,
    Derived,
    Unknown,
}

/// Convert a lab result to twin data points
fn lab_result_to_twin_data_point(lab: &LabResult) -> TwinDataPointInput {
    // Serialize the lab value as JSON for flexibility
    let value_json = serde_json::json!({
        "value": lab.value,
        "reference_range": lab.reference_range,
        "interpretation": format!("{:?}", lab.interpretation),
        "loinc_code": lab.loinc_code,
        "test_name": lab.test_name,
    })
    .to_string();

    TwinDataPointInput {
        data_type: TwinDataType::LabResult(lab.loinc_code.clone()),
        value: value_json,
        unit: Some(lab.unit.clone()),
        measured_at: lab.result_time.as_micros(),
        source: TwinDataSourceType::Laboratory,
        quality: TwinDataQuality::Clinical,
    }
}

/// Convert vital signs to twin data points (multiple points from one reading)
fn vitals_to_twin_data_points(vitals: &VitalSigns) -> Vec<TwinDataPointInput> {
    let mut data_points = Vec::new();
    let measured_at = vitals.recorded_at.as_micros();

    // Heart rate
    if let Some(hr) = vitals.heart_rate_bpm {
        data_points.push(TwinDataPointInput {
            data_type: TwinDataType::VitalSign(TwinVitalSignType::HeartRate),
            value: hr.to_string(),
            unit: Some("bpm".to_string()),
            measured_at,
            source: TwinDataSourceType::EHR,
            quality: TwinDataQuality::Clinical,
        });
    }

    // Blood pressure (combined)
    if let (Some(sys), Some(dia)) = (
        vitals.blood_pressure_systolic,
        vitals.blood_pressure_diastolic,
    ) {
        data_points.push(TwinDataPointInput {
            data_type: TwinDataType::VitalSign(TwinVitalSignType::BloodPressure),
            value: serde_json::json!({"systolic": sys, "diastolic": dia}).to_string(),
            unit: Some("mmHg".to_string()),
            measured_at,
            source: TwinDataSourceType::EHR,
            quality: TwinDataQuality::Clinical,
        });
    }

    // Temperature
    if let Some(temp) = vitals.temperature_celsius {
        data_points.push(TwinDataPointInput {
            data_type: TwinDataType::VitalSign(TwinVitalSignType::Temperature),
            value: temp.to_string(),
            unit: Some("°C".to_string()),
            measured_at,
            source: TwinDataSourceType::EHR,
            quality: TwinDataQuality::Clinical,
        });
    }

    // Respiratory rate
    if let Some(rr) = vitals.respiratory_rate {
        data_points.push(TwinDataPointInput {
            data_type: TwinDataType::VitalSign(TwinVitalSignType::RespiratoryRate),
            value: rr.to_string(),
            unit: Some("breaths/min".to_string()),
            measured_at,
            source: TwinDataSourceType::EHR,
            quality: TwinDataQuality::Clinical,
        });
    }

    // Oxygen saturation
    if let Some(spo2) = vitals.oxygen_saturation {
        data_points.push(TwinDataPointInput {
            data_type: TwinDataType::VitalSign(TwinVitalSignType::SpO2),
            value: spo2.to_string(),
            unit: Some("%".to_string()),
            measured_at,
            source: TwinDataSourceType::EHR,
            quality: TwinDataQuality::Clinical,
        });
    }

    // Weight
    if let Some(weight) = vitals.weight_kg {
        data_points.push(TwinDataPointInput {
            data_type: TwinDataType::VitalSign(TwinVitalSignType::Weight),
            value: weight.to_string(),
            unit: Some("kg".to_string()),
            measured_at,
            source: TwinDataSourceType::EHR,
            quality: TwinDataQuality::Clinical,
        });
    }

    // Height
    if let Some(height) = vitals.height_cm {
        data_points.push(TwinDataPointInput {
            data_type: TwinDataType::VitalSign(TwinVitalSignType::Height),
            value: height.to_string(),
            unit: Some("cm".to_string()),
            measured_at,
            source: TwinDataSourceType::EHR,
            quality: TwinDataQuality::Clinical,
        });
    }

    // BMI
    if let Some(bmi) = vitals.bmi {
        data_points.push(TwinDataPointInput {
            data_type: TwinDataType::VitalSign(TwinVitalSignType::BMI),
            value: bmi.to_string(),
            unit: Some("kg/m²".to_string()),
            measured_at,
            source: TwinDataSourceType::EHR,
            quality: TwinDataQuality::Derived,
        });
    }

    data_points
}

/// Input for creating encounter with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateEncounterInput {
    pub encounter: Encounter,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Create a new encounter with access control
#[hdk_extern]
pub fn create_encounter(input: CreateEncounterInput) -> ExternResult<Record> {
    // Require Write authorization for Procedures category (encounters)
    let auth = require_authorization(
        input.encounter.patient_hash.clone(),
        DataCategory::Procedures,
        Permission::Write,
        input.is_emergency,
    )?;

    let encounter_hash = create_entry(&EntryTypes::Encounter(input.encounter.clone()))?;
    let record = get(encounter_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find encounter".to_string())
    ))?;

    // Link to patient
    create_link(
        input.encounter.patient_hash.clone(),
        encounter_hash.clone(),
        LinkTypes::PatientToEncounters,
        (),
    )?;

    // Link to provider
    create_link(
        input.encounter.provider_hash.clone(),
        encounter_hash.clone(),
        LinkTypes::ProviderToEncounters,
        (),
    )?;

    // Log the access for audit trail
    log_data_access(
        input.encounter.patient_hash,
        vec![DataCategory::Procedures],
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(record)
}

/// Input for getting encounter with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct GetEncounterInput {
    pub encounter_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Internal get without access control
fn get_encounter_internal(encounter_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(encounter_hash, GetOptions::default())
}

/// Get an encounter with access control
#[hdk_extern]
pub fn get_encounter(input: GetEncounterInput) -> ExternResult<Option<Record>> {
    // First get the encounter to find the patient_hash
    let record = get_encounter_internal(input.encounter_hash.clone())?;

    if let Some(ref rec) = record {
        let encounter: Encounter = rec
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Invalid encounter entry".to_string()
            )))?;

        // Require Read authorization
        let auth = require_authorization(
            encounter.patient_hash.clone(),
            DataCategory::Procedures,
            Permission::Read,
            input.is_emergency,
        )?;

        // Log the access
        log_data_access(
            encounter.patient_hash,
            vec![DataCategory::Procedures],
            auth.consent_hash,
            auth.emergency_override,
            input.emergency_reason,
        )?;
    }

    Ok(record)
}

/// Input for getting patient encounters with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPatientEncountersInput {
    pub patient_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get patient's encounters with access control
#[hdk_extern]
pub fn get_patient_encounters(input: GetPatientEncountersInput) -> ExternResult<Vec<Record>> {
    // Require Read authorization for Procedures category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Procedures,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToEncounters)?,
        GetStrategy::default(),
    )?;

    let mut encounters = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                encounters.push(record);
            }
        }
    }

    // Log the access
    if !encounters.is_empty() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::Procedures],
            auth.consent_hash,
            auth.emergency_override,
            input.emergency_reason,
        )?;
    }

    Ok(encounters)
}

/// Input for creating diagnosis with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateDiagnosisInput {
    pub diagnosis: Diagnosis,
    pub patient_hash: ActionHash, // Must provide patient for authorization
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Create a diagnosis with access control
#[hdk_extern]
pub fn create_diagnosis(input: CreateDiagnosisInput) -> ExternResult<Record> {
    // Require Write authorization for Diagnoses category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Write,
        input.is_emergency,
    )?;

    let diagnosis_hash = create_entry(&EntryTypes::Diagnosis(input.diagnosis.clone()))?;
    let record = get(diagnosis_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find diagnosis".to_string())
    ))?;

    // Link to encounter if provided
    if let Some(encounter_hash) = input.diagnosis.encounter_hash {
        create_link(
            encounter_hash,
            diagnosis_hash,
            LinkTypes::EncounterToDiagnoses,
            (),
        )?;
    }

    // Log the access
    log_data_access(
        input.patient_hash,
        vec![DataCategory::Diagnoses],
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(record)
}

/// Input for getting encounter diagnoses with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct GetEncounterDiagnosesInput {
    pub encounter_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get diagnoses for an encounter with access control
#[hdk_extern]
pub fn get_encounter_diagnoses(input: GetEncounterDiagnosesInput) -> ExternResult<Vec<Record>> {
    // First get the encounter to find the patient_hash
    let encounter_record = get_encounter_internal(input.encounter_hash.clone())?.ok_or(
        wasm_error!(WasmErrorInner::Guest("Encounter not found".to_string())),
    )?;

    let encounter: Encounter = encounter_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid encounter entry".to_string()
        )))?;

    // Require Read authorization for Diagnoses category
    let auth = require_authorization(
        encounter.patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(
        LinkQuery::try_new(input.encounter_hash, LinkTypes::EncounterToDiagnoses)?,
        GetStrategy::default(),
    )?;

    let mut diagnoses = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                diagnoses.push(record);
            }
        }
    }

    // Log the access
    if !diagnoses.is_empty() {
        log_data_access(
            encounter.patient_hash,
            vec![DataCategory::Diagnoses],
            auth.consent_hash,
            auth.emergency_override,
            input.emergency_reason,
        )?;
    }

    Ok(diagnoses)
}

/// Input for creating procedure with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateProcedureInput {
    pub procedure: ProcedurePerformed,
    pub patient_hash: ActionHash, // Must provide patient for authorization
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Create a procedure record with access control
#[hdk_extern]
pub fn create_procedure(input: CreateProcedureInput) -> ExternResult<Record> {
    // Require Write authorization for Procedures category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Procedures,
        Permission::Write,
        input.is_emergency,
    )?;

    let procedure_hash = create_entry(&EntryTypes::ProcedurePerformed(input.procedure.clone()))?;
    let record = get(procedure_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find procedure".to_string())
    ))?;

    create_link(
        input.procedure.encounter_hash,
        procedure_hash,
        LinkTypes::EncounterToProcedures,
        (),
    )?;

    // Log the access
    log_data_access(
        input.patient_hash,
        vec![DataCategory::Procedures],
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(record)
}

/// Input for creating lab result with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateLabResultInput {
    pub lab_result: LabResult,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Create a lab result with access control
///
/// This function automatically feeds the lab result to the patient's Health Twin
/// (if one exists) for model updates and health predictions.
#[hdk_extern]
pub fn create_lab_result(input: CreateLabResultInput) -> ExternResult<Record> {
    // Require Write authorization for LabResults category
    let auth = require_authorization(
        input.lab_result.patient_hash.clone(),
        DataCategory::LabResults,
        Permission::Write,
        input.is_emergency,
    )?;

    let result_hash = create_entry(&EntryTypes::LabResult(input.lab_result.clone()))?;
    let record = get(result_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find lab result".to_string())
    ))?;

    // Link to patient
    create_link(
        input.lab_result.patient_hash.clone(),
        result_hash.clone(),
        LinkTypes::PatientToLabResults,
        (),
    )?;

    // If critical, add to critical results
    if input.lab_result.is_critical {
        let critical_anchor = anchor_hash("critical_results")?;
        create_link(critical_anchor, result_hash, LinkTypes::CriticalResults, ())?;
    }

    // ==================== HEALTH TWIN INTEGRATION ====================
    // Feed the lab result to the patient's Health Twin for model updates
    let twin_data_point = lab_result_to_twin_data_point(&input.lab_result);
    try_feed_to_health_twin(&input.lab_result.patient_hash, twin_data_point);
    // ================================================================

    // Log the access
    log_data_access(
        input.lab_result.patient_hash,
        vec![DataCategory::LabResults],
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(record)
}

/// Input for getting patient lab results with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPatientLabResultsInput {
    pub patient_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get patient's lab results with access control
#[hdk_extern]
pub fn get_patient_lab_results(input: GetPatientLabResultsInput) -> ExternResult<Vec<Record>> {
    // Require Read authorization for LabResults category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::LabResults,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToLabResults)?,
        GetStrategy::default(),
    )?;

    let mut results = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                results.push(record);
            }
        }
    }

    // Log the access
    if !results.is_empty() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::LabResults],
            auth.consent_hash,
            auth.emergency_override,
            input.emergency_reason,
        )?;
    }

    Ok(results)
}

/// Input for acknowledging critical result with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct AcknowledgeInput {
    pub result_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Acknowledge critical lab result with access control
#[hdk_extern]
pub fn acknowledge_critical_result(input: AcknowledgeInput) -> ExternResult<Record> {
    let record = get(input.result_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Lab result not found".to_string())
    ))?;

    let mut lab_result: LabResult = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid lab result".to_string()
        )))?;

    // Require Write authorization (Amend would be more specific but Write is sufficient)
    let auth = require_authorization(
        lab_result.patient_hash.clone(),
        DataCategory::LabResults,
        Permission::Write,
        input.is_emergency,
    )?;

    lab_result.acknowledged_by = Some(agent_info()?.agent_initial_pubkey);
    lab_result.acknowledged_at = Some(sys_time()?);

    let updated_hash = update_entry(input.result_hash, &lab_result)?;
    let updated_record = get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find updated result".to_string())
    ))?;

    // Log the access
    log_data_access(
        lab_result.patient_hash,
        vec![DataCategory::LabResults],
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(updated_record)
}

/// Input for creating imaging study with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateImagingStudyInput {
    pub imaging: ImagingStudy,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Create imaging study with access control
#[hdk_extern]
pub fn create_imaging_study(input: CreateImagingStudyInput) -> ExternResult<Record> {
    // Require Write authorization for ImagingStudies category
    let auth = require_authorization(
        input.imaging.patient_hash.clone(),
        DataCategory::ImagingStudies,
        Permission::Write,
        input.is_emergency,
    )?;

    let imaging_hash = create_entry(&EntryTypes::ImagingStudy(input.imaging.clone()))?;
    let record = get(imaging_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find imaging study".to_string())
    ))?;

    create_link(
        input.imaging.patient_hash.clone(),
        imaging_hash.clone(),
        LinkTypes::PatientToImaging,
        (),
    )?;

    if input.imaging.is_critical {
        let critical_anchor = anchor_hash("critical_results")?;
        create_link(
            critical_anchor,
            imaging_hash,
            LinkTypes::CriticalResults,
            (),
        )?;
    }

    // Log the access
    log_data_access(
        input.imaging.patient_hash,
        vec![DataCategory::ImagingStudies],
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(record)
}

/// Input for getting patient imaging with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPatientImagingInput {
    pub patient_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get patient's imaging studies with access control
#[hdk_extern]
pub fn get_patient_imaging(input: GetPatientImagingInput) -> ExternResult<Vec<Record>> {
    // Require Read authorization for ImagingStudies category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::ImagingStudies,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToImaging)?,
        GetStrategy::default(),
    )?;

    let mut studies = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                studies.push(record);
            }
        }
    }

    // Log the access
    if !studies.is_empty() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::ImagingStudies],
            auth.consent_hash,
            auth.emergency_override,
            input.emergency_reason,
        )?;
    }

    Ok(studies)
}

/// Input for recording vital signs with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct RecordVitalSignsInput {
    pub vitals: VitalSigns,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Record vital signs with access control
///
/// This function automatically feeds vital sign data to the patient's Health Twin
/// (if one exists) for model updates and health predictions.
#[hdk_extern]
pub fn record_vital_signs(input: RecordVitalSignsInput) -> ExternResult<Record> {
    // Require Write authorization for VitalSigns category
    let auth = require_authorization(
        input.vitals.patient_hash.clone(),
        DataCategory::VitalSigns,
        Permission::Write,
        input.is_emergency,
    )?;

    let vitals_hash = create_entry(&EntryTypes::VitalSigns(input.vitals.clone()))?;
    let record = get(vitals_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find vitals".to_string())
    ))?;

    create_link(
        input.vitals.patient_hash.clone(),
        vitals_hash,
        LinkTypes::PatientToVitals,
        (),
    )?;

    // ==================== HEALTH TWIN INTEGRATION ====================
    // Feed vital signs to the patient's Health Twin for model updates
    // Vitals create multiple data points (one per measurement)
    let twin_data_points = vitals_to_twin_data_points(&input.vitals);
    for data_point in twin_data_points {
        try_feed_to_health_twin(&input.vitals.patient_hash, data_point);
    }
    // ================================================================

    // Log the access
    log_data_access(
        input.vitals.patient_hash,
        vec![DataCategory::VitalSigns],
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(record)
}

/// Input for getting patient vitals with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPatientVitalsInput {
    pub patient_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get patient's recent vital signs with access control
#[hdk_extern]
pub fn get_patient_vitals(input: GetPatientVitalsInput) -> ExternResult<Vec<Record>> {
    // Require Read authorization for VitalSigns category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::VitalSigns,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToVitals)?,
        GetStrategy::default(),
    )?;

    let mut vitals = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                vitals.push(record);
            }
        }
    }

    // Log the access
    if !vitals.is_empty() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::VitalSigns],
            auth.consent_hash,
            auth.emergency_override,
            input.emergency_reason,
        )?;
    }

    Ok(vitals)
}

/// Get all critical/unacknowledged results (admin function)
/// Requires admin authorization as it accesses multiple patients' data
#[hdk_extern]
pub fn get_critical_results(_: ()) -> ExternResult<Vec<Record>> {
    // Require admin authorization for bulk critical results access
    require_admin_authorization()?;

    let critical_anchor = anchor_hash("critical_results")?;
    let links = get_links(
        LinkQuery::try_new(critical_anchor, LinkTypes::CriticalResults)?,
        GetStrategy::default(),
    )?;

    let mut results = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                // Check if lab result is unacknowledged
                if let Some(lab) = record.entry().to_app_option::<LabResult>().ok().flatten() {
                    if lab.acknowledged_by.is_none() {
                        results.push(record);
                    }
                } else if let Some(imaging) = record
                    .entry()
                    .to_app_option::<ImagingStudy>()
                    .ok()
                    .flatten()
                {
                    if imaging.is_critical {
                        results.push(record);
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Input for updating an encounter with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateEncounterInput {
    pub original_hash: ActionHash,
    pub updated_encounter: Encounter,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Update an encounter status with access control
#[hdk_extern]
pub fn update_encounter(input: UpdateEncounterInput) -> ExternResult<Record> {
    // Require Write authorization for Procedures category
    let auth = require_authorization(
        input.updated_encounter.patient_hash.clone(),
        DataCategory::Procedures,
        Permission::Write,
        input.is_emergency,
    )?;

    let updated_hash = update_entry(input.original_hash.clone(), &input.updated_encounter)?;
    let record = get(updated_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find updated encounter".to_string())
    ))?;

    create_link(
        input.original_hash,
        updated_hash,
        LinkTypes::EncounterUpdates,
        (),
    )?;

    // Log the access
    log_data_access(
        input.updated_encounter.patient_hash,
        vec![DataCategory::Procedures],
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(record)
}

/// Input for updating diagnosis with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateDiagnosisInput {
    pub original_hash: ActionHash,
    pub updated_diagnosis: Diagnosis,
    pub patient_hash: ActionHash, // Must provide patient for authorization
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Update diagnosis status (e.g., resolve, correct) with access control
#[hdk_extern]
pub fn update_diagnosis(input: UpdateDiagnosisInput) -> ExternResult<Record> {
    // Require Amend authorization for Diagnoses category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Amend,
        input.is_emergency,
    )?;

    let updated_hash = update_entry(input.original_hash.clone(), &input.updated_diagnosis)?;
    let record = get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find updated diagnosis".to_string())
    ))?;

    // Log the access
    log_data_access(
        input.patient_hash,
        vec![DataCategory::Diagnoses],
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(record)
}

/// Input for updating lab result with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateLabResultInput {
    pub original_hash: ActionHash,
    pub updated_result: LabResult,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Update lab result (e.g., amended results) with access control
#[hdk_extern]
pub fn update_lab_result(input: UpdateLabResultInput) -> ExternResult<Record> {
    // Require Amend authorization for LabResults category
    let auth = require_authorization(
        input.updated_result.patient_hash.clone(),
        DataCategory::LabResults,
        Permission::Amend,
        input.is_emergency,
    )?;

    let updated_hash = update_entry(input.original_hash.clone(), &input.updated_result)?;
    let record = get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find updated lab result".to_string())
    ))?;

    // Log the access
    log_data_access(
        input.updated_result.patient_hash,
        vec![DataCategory::LabResults],
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(record)
}

/// Input for deleting encounter with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteEncounterInput {
    pub encounter_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Delete encounter (soft delete - mark as cancelled) with access control
#[hdk_extern]
pub fn delete_encounter(input: DeleteEncounterInput) -> ExternResult<ActionHash> {
    // First get the encounter to find the patient_hash
    let record = get_encounter_internal(input.encounter_hash.clone())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Encounter not found".to_string())
    ))?;

    let encounter: Encounter = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid encounter entry".to_string()
        )))?;

    // Require Delete authorization
    let auth = require_authorization(
        encounter.patient_hash.clone(),
        DataCategory::Procedures,
        Permission::Delete,
        input.is_emergency,
    )?;

    let result = delete_entry(input.encounter_hash)?;

    // Log the deletion for audit trail
    log_data_access(
        encounter.patient_hash,
        vec![DataCategory::Procedures],
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(result)
}

/// Input for getting encounter history with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct GetEncounterHistoryInput {
    pub encounter_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get encounter history (all versions) with access control
#[hdk_extern]
pub fn get_encounter_history(input: GetEncounterHistoryInput) -> ExternResult<Vec<Record>> {
    // First get the encounter to find the patient_hash
    let original = get_encounter_internal(input.encounter_hash.clone())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Encounter not found".to_string())
    ))?;

    let encounter: Encounter = original
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid encounter entry".to_string()
        )))?;

    // Require Read authorization for Procedures category
    let auth = require_authorization(
        encounter.patient_hash.clone(),
        DataCategory::Procedures,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(
        LinkQuery::try_new(input.encounter_hash, LinkTypes::EncounterUpdates)?,
        GetStrategy::default(),
    )?;

    let mut history = Vec::new();
    history.push(original);

    // Add updates
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                history.push(record);
            }
        }
    }

    // Log the access
    log_data_access(
        encounter.patient_hash,
        vec![DataCategory::Procedures],
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(history)
}

/// Anchor entry for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}
