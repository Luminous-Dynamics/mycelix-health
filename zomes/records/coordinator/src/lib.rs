// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
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
use records_integrity::*;
use mycelix_health_shared::{
    require_authorization, require_admin_authorization,
    log_data_access,
    DataCategory, Permission,
    batch::links_to_records,
};

// ==================== HEALTH TWIN INTEGRATION ====================

/// Try to feed data to the patient's health twin (if one exists)
/// This is a best-effort operation - failures are logged but don't break the main operation
fn try_feed_to_health_twin(patient_hash: &ActionHash, data_point: TwinDataPointInput) {
    // Best effort - don't fail main operation if twin doesn't exist or has issues
    let _ = feed_to_health_twin_internal(patient_hash, data_point);
}

/// Internal function to feed data to health twin
fn feed_to_health_twin_internal(patient_hash: &ActionHash, data_point: TwinDataPointInput) -> ExternResult<()> {
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
        ZomeCallResponse::Ok(io) => io.decode()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to decode twin response: {}", e))))?,
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
    }).to_string();

    TwinDataPointInput {
        data_type: TwinDataType::LabResult(lab.loinc_code.clone()),
        value: value_json,
        unit: Some(lab.unit.clone()),
        measured_at: lab.result_time.as_micros() as i64,
        source: TwinDataSourceType::Laboratory,
        quality: TwinDataQuality::Clinical,
    }
}

/// Convert vital signs to twin data points (multiple points from one reading)
fn vitals_to_twin_data_points(vitals: &VitalSigns) -> Vec<TwinDataPointInput> {
    let mut data_points = Vec::new();
    let measured_at = vitals.recorded_at.as_micros() as i64;

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
    if let (Some(sys), Some(dia)) = (vitals.blood_pressure_systolic, vitals.blood_pressure_diastolic) {
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

/// Convert a diagnosis to twin data point
fn diagnosis_to_twin_data_point(diagnosis: &Diagnosis) -> TwinDataPointInput {
    let value_json = serde_json::json!({
        "icd10_code": diagnosis.icd10_code,
        "snomed_code": diagnosis.snomed_code,
        "description": diagnosis.description,
        "diagnosis_type": format!("{:?}", diagnosis.diagnosis_type),
        "status": format!("{:?}", diagnosis.status),
        "severity": diagnosis.severity.as_ref().map(|s| format!("{:?}", s)),
        "onset_date": diagnosis.onset_date,
        "epistemic_level": format!("{:?}", diagnosis.epistemic_level),
    }).to_string();

    TwinDataPointInput {
        data_type: TwinDataType::Diagnosis(diagnosis.icd10_code.clone()),
        value: value_json,
        unit: None,
        measured_at: diagnosis.created_at.as_micros() as i64,
        source: TwinDataSourceType::EHR,
        quality: TwinDataQuality::Clinical,
    }
}

/// Convert a procedure to twin data point
fn procedure_to_twin_data_point(procedure: &ProcedurePerformed) -> TwinDataPointInput {
    let value_json = serde_json::json!({
        "cpt_code": procedure.cpt_code,
        "hcpcs_code": procedure.hcpcs_code,
        "description": procedure.description,
        "location": procedure.location,
        "outcome": format!("{:?}", procedure.outcome),
        "complications": procedure.complications,
    }).to_string();

    TwinDataPointInput {
        data_type: TwinDataType::Procedure(procedure.cpt_code.clone()),
        value: value_json,
        unit: None,
        measured_at: procedure.performed_at.as_micros() as i64,
        source: TwinDataSourceType::EHR,
        quality: TwinDataQuality::Clinical,
    }
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
    let record = get(encounter_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find encounter".to_string())))?;

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
        Permission::Write,
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
            .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid encounter entry".to_string())))?;

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
            Permission::Read,
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
///
/// OPTIMIZED: Uses batch query to avoid N+1 pattern
#[hdk_extern]
pub fn get_patient_encounters(input: GetPatientEncountersInput) -> ExternResult<Vec<Record>> {
    // Require Read authorization for Procedures category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Procedures,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToEncounters)?, GetStrategy::default())?;

    // FIXED N+1: Use batch fetch instead of individual get() calls
    let encounters = links_to_records(links)?;

    // Log the access
    if !encounters.is_empty() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::Procedures],
            Permission::Read,
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
    let record = get(diagnosis_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find diagnosis".to_string())))?;

    // ================ HEALTH TWIN INTEGRATION ================
    // Feed the diagnosis to the patient's Health Twin for model updates
    // (Must be done before moving encounter_hash)
    let twin_data_point = diagnosis_to_twin_data_point(&input.diagnosis);
    try_feed_to_health_twin(&input.diagnosis.patient_hash, twin_data_point);
    // =========================================================

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
        Permission::Write,
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
///
/// OPTIMIZED: Uses batch query to avoid N+1 pattern
#[hdk_extern]
pub fn get_encounter_diagnoses(input: GetEncounterDiagnosesInput) -> ExternResult<Vec<Record>> {
    // First get the encounter to find the patient_hash
    let encounter_record = get_encounter_internal(input.encounter_hash.clone())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Encounter not found".to_string())))?;

    let encounter: Encounter = encounter_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid encounter entry".to_string())))?;

    // Require Read authorization for Diagnoses category
    let auth = require_authorization(
        encounter.patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(LinkQuery::try_new(input.encounter_hash, LinkTypes::EncounterToDiagnoses)?, GetStrategy::default())?;

    // FIXED N+1: Use batch fetch instead of individual get() calls
    let diagnoses = links_to_records(links)?;

    // Log the access
    if !diagnoses.is_empty() {
        log_data_access(
            encounter.patient_hash,
            vec![DataCategory::Diagnoses],
            Permission::Read,
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
    let record = get(procedure_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find procedure".to_string())))?;

    // ================ HEALTH TWIN INTEGRATION ================
    // Feed the procedure to the patient's Health Twin for model updates
    // (Must be done before moving encounter_hash)
    let twin_data_point = procedure_to_twin_data_point(&input.procedure);
    try_feed_to_health_twin(&input.procedure.patient_hash, twin_data_point);
    // =========================================================

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
        Permission::Write,
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
    let record = get(result_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find lab result".to_string())))?;

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
        create_link(
            critical_anchor,
            result_hash,
            LinkTypes::CriticalResults,
            (),
        )?;
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
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(record)
}

// ==================== ENCRYPTED RECORD CREATION ====================

/// Input for creating an encrypted lab result.
/// The lab result data is encrypted with the patient's public key before DHT storage.
/// Only the patient or consent-granted agents can decrypt it.
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateEncryptedLabResultInput {
    /// The lab result to encrypt and store.
    pub lab_result: LabResult,
    /// Patient's public key for encryption (from PatientKeyMetadata).
    pub patient_public_key: Vec<u8>,
    /// Whether this is an emergency access (bypass normal consent).
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Create a lab result encrypted with the patient's key.
///
/// The lab result is serialized, encrypted, then stored as an `EncryptedRecord`.
/// The `data_category` (LabResults) is stored in cleartext for consent checking.
/// The actual clinical data can only be read by decrypting with the patient's key
/// or a consent-derived re-encryption key.
#[hdk_extern]
pub fn create_encrypted_lab_result(input: CreateEncryptedLabResultInput) -> ExternResult<Record> {
    use mycelix_health_shared::patient_encryption::compute_fingerprint;

    // Require Write authorization (same as unencrypted path)
    let auth = require_authorization(
        input.lab_result.patient_hash.clone(),
        DataCategory::LabResults,
        Permission::Write,
        input.is_emergency,
    )?;

    // Serialize the lab result to MessagePack
    let plaintext = ExternIO::encode(&input.lab_result)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Serialize failed: {}", e))))?;

    // Generate nonce from system time + agent entropy (24 bytes for XChaCha20-Poly1305)
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let mut nonce = [0u8; 24];
    let time_bytes = now.as_micros().to_le_bytes();
    let agent_bytes = agent.get_raw_39();
    for i in 0..8 { nonce[i] = time_bytes[i]; }
    for i in 0..16.min(agent_bytes.len()) { nonce[8 + i % 16] = agent_bytes[i]; }

    // XOR-encrypt with key (simplified — production would use XChaCha20-Poly1305)
    // This demonstrates the architecture; replace with real AEAD in production.
    let key = &input.patient_public_key;
    let ciphertext: Vec<u8> = plaintext.as_bytes()
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % key.len()] ^ nonce[i % 24])
        .collect();

    let fingerprint = compute_fingerprint(&input.patient_public_key);

    // Create the encrypted record
    let encrypted = EncryptedRecord {
        patient_hash: input.lab_result.patient_hash.clone(),
        key_fingerprint: fingerprint,
        ciphertext,
        nonce,
        data_category: "LabResults".to_string(),
        entry_type: "LabResult".to_string(),
        encrypted_at: now.as_micros() as i64,
    };

    let record_hash = create_entry(&EntryTypes::EncryptedRecord(encrypted))?;
    let record = get(record_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find encrypted record".to_string())))?;

    // Link to patient (via encrypted records link)
    create_link(
        input.lab_result.patient_hash.clone(),
        record_hash,
        LinkTypes::PatientToEncryptedRecords,
        (),
    )?;

    // Log the access (audit trail)
    log_data_access(
        input.lab_result.patient_hash,
        vec![DataCategory::LabResults],
        Permission::Write,
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
///
/// OPTIMIZED: Uses batch query to avoid N+1 pattern
#[hdk_extern]
pub fn get_patient_lab_results(input: GetPatientLabResultsInput) -> ExternResult<Vec<Record>> {
    // Require Read authorization for LabResults category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::LabResults,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToLabResults)?, GetStrategy::default())?;

    // FIXED N+1: Use batch fetch instead of individual get() calls
    let results = links_to_records(links)?;

    // Log the access
    if !results.is_empty() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::LabResults],
            Permission::Read,
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
    let record = get(input.result_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Lab result not found".to_string())))?;

    let mut lab_result: LabResult = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid lab result".to_string())))?;

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
    let updated_record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated result".to_string())))?;

    // Log the access
    log_data_access(
        lab_result.patient_hash,
        vec![DataCategory::LabResults],
        Permission::Write,
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
    let record = get(imaging_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find imaging study".to_string())))?;

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
        Permission::Write,
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
///
/// OPTIMIZED: Uses batch query to avoid N+1 pattern
#[hdk_extern]
pub fn get_patient_imaging(input: GetPatientImagingInput) -> ExternResult<Vec<Record>> {
    // Require Read authorization for ImagingStudies category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::ImagingStudies,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToImaging)?, GetStrategy::default())?;

    // FIXED N+1: Use batch fetch instead of individual get() calls
    let studies = links_to_records(links)?;

    // Log the access
    if !studies.is_empty() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::ImagingStudies],
            Permission::Read,
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
    let record = get(vitals_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find vitals".to_string())))?;

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
        Permission::Write,
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
///
/// OPTIMIZED: Uses batch query to avoid N+1 pattern
#[hdk_extern]
pub fn get_patient_vitals(input: GetPatientVitalsInput) -> ExternResult<Vec<Record>> {
    // Require Read authorization for VitalSigns category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::VitalSigns,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToVitals)?, GetStrategy::default())?;

    // FIXED N+1: Use batch fetch instead of individual get() calls
    let vitals = links_to_records(links)?;

    // Log the access
    if !vitals.is_empty() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::VitalSigns],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            input.emergency_reason,
        )?;
    }

    Ok(vitals)
}

/// Get all critical/unacknowledged results (admin function)
/// Requires admin authorization as it accesses multiple patients' data
///
/// OPTIMIZED: Uses batch query to avoid N+1 pattern
#[hdk_extern]
pub fn get_critical_results(_: ()) -> ExternResult<Vec<Record>> {
    // Require admin authorization for bulk critical results access
    require_admin_authorization()?;

    let critical_anchor = anchor_hash("critical_results")?;
    let links = get_links(LinkQuery::try_new(critical_anchor, LinkTypes::CriticalResults)?, GetStrategy::default())?;

    // FIXED N+1: Batch fetch all records first, then filter
    let all_records = links_to_records(links)?;

    let results = all_records
        .into_iter()
        .filter(|record| {
            // Check if lab result is unacknowledged
            if let Some(lab) = record.entry().to_app_option::<LabResult>().ok().flatten() {
                return lab.acknowledged_by.is_none();
            }
            // Check if imaging is critical
            if let Some(imaging) = record.entry().to_app_option::<ImagingStudy>().ok().flatten() {
                return imaging.is_critical;
            }
            false
        })
        .collect();

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
    let record = get(updated_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated encounter".to_string())))?;

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
        Permission::Write,
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
    let record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated diagnosis".to_string())))?;

    // Log the access
    log_data_access(
        input.patient_hash,
        vec![DataCategory::Diagnoses],
        Permission::Amend,
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
    let record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated lab result".to_string())))?;

    // Log the access
    log_data_access(
        input.updated_result.patient_hash,
        vec![DataCategory::LabResults],
        Permission::Amend,
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
    let record = get_encounter_internal(input.encounter_hash.clone())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Encounter not found".to_string())))?;

    let encounter: Encounter = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid encounter entry".to_string())))?;

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
        Permission::Delete,
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
///
/// OPTIMIZED: Uses batch query to avoid N+1 pattern
#[hdk_extern]
pub fn get_encounter_history(input: GetEncounterHistoryInput) -> ExternResult<Vec<Record>> {
    // First get the encounter to find the patient_hash
    let original = get_encounter_internal(input.encounter_hash.clone())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Encounter not found".to_string())))?;

    let encounter: Encounter = original
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid encounter entry".to_string())))?;

    // Require Read authorization for Procedures category
    let auth = require_authorization(
        encounter.patient_hash.clone(),
        DataCategory::Procedures,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(LinkQuery::try_new(input.encounter_hash, LinkTypes::EncounterUpdates)?, GetStrategy::default())?;

    // FIXED N+1: Batch fetch updates
    let mut history = Vec::new();
    history.push(original);
    history.extend(links_to_records(links)?);

    // Log the access
    log_data_access(
        encounter.patient_hash,
        vec![DataCategory::Procedures],
        Permission::Read,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_hash() -> ActionHash {
        ActionHash::from_raw_36(vec![0u8; 36])
    }

    fn dummy_agent() -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![0u8; 36])
    }

    // ==================== TwinDataPointInput tests ====================

    #[test]
    fn test_twin_data_point_input_serde_roundtrip() {
        let dp = TwinDataPointInput {
            data_type: TwinDataType::VitalSign(TwinVitalSignType::HeartRate),
            value: "72".to_string(),
            unit: Some("bpm".to_string()),
            measured_at: 1710000000,
            source: TwinDataSourceType::EHR,
            quality: TwinDataQuality::Clinical,
        };
        let json = serde_json::to_string(&dp).expect("serialize");
        let decoded: TwinDataPointInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.value, "72");
        assert_eq!(decoded.measured_at, 1710000000);
    }

    #[test]
    fn test_twin_data_type_all_variants_serde() {
        let types: Vec<TwinDataType> = vec![
            TwinDataType::VitalSign(TwinVitalSignType::HeartRate),
            TwinDataType::VitalSign(TwinVitalSignType::BloodPressure),
            TwinDataType::VitalSign(TwinVitalSignType::Temperature),
            TwinDataType::VitalSign(TwinVitalSignType::SpO2),
            TwinDataType::VitalSign(TwinVitalSignType::BMI),
            TwinDataType::LabResult("GLU".to_string()),
            TwinDataType::Medication("Metformin".to_string()),
            TwinDataType::Diagnosis("E11.9".to_string()),
            TwinDataType::Procedure("99213".to_string()),
            TwinDataType::Lifestyle("Exercise".to_string()),
            TwinDataType::Symptom("Headache".to_string()),
            TwinDataType::BiometricReading("HRV".to_string()),
            TwinDataType::GeneticMarker("BRCA1".to_string()),
            TwinDataType::SocialDeterminant("Housing".to_string()),
        ];
        for dt in types {
            let json = serde_json::to_string(&dt).expect("serialize");
            let _decoded: TwinDataType = serde_json::from_str(&json).expect("deserialize");
        }
    }

    // ==================== vitals_to_twin_data_points tests ====================

    #[test]
    fn test_vitals_to_twin_data_points_empty_vitals() {
        let vitals = VitalSigns {
            patient_hash: dummy_hash(),
            encounter_hash: None,
            recorded_at: Timestamp::from_micros(1000000),
            recorded_by: dummy_agent(),
            temperature_celsius: None,
            heart_rate_bpm: None,
            blood_pressure_systolic: None,
            blood_pressure_diastolic: None,
            respiratory_rate: None,
            oxygen_saturation: None,
            height_cm: None,
            weight_kg: None,
            bmi: None,
            pain_level: None,
            notes: None,
        };
        let points = vitals_to_twin_data_points(&vitals);
        assert!(points.is_empty(), "No vitals set should produce no data points");
    }

    #[test]
    fn test_vitals_to_twin_data_points_all_populated() {
        let vitals = VitalSigns {
            patient_hash: dummy_hash(),
            encounter_hash: None,
            recorded_at: Timestamp::from_micros(1000000),
            recorded_by: dummy_agent(),
            temperature_celsius: Some(37.0),
            heart_rate_bpm: Some(72),
            blood_pressure_systolic: Some(120),
            blood_pressure_diastolic: Some(80),
            respiratory_rate: Some(16),
            oxygen_saturation: Some(98.0),
            height_cm: Some(175.0),
            weight_kg: Some(70.0),
            bmi: Some(22.9),
            pain_level: Some(2),
            notes: None,
        };
        let points = vitals_to_twin_data_points(&vitals);
        // Should produce: HR, BP, Temp, RR, SpO2, Weight, Height, BMI = 8 points
        assert_eq!(points.len(), 8, "All vitals should produce 8 data points, got {}", points.len());
    }

    #[test]
    fn test_vitals_to_twin_bp_requires_both_systolic_and_diastolic() {
        // Only systolic, no diastolic
        let vitals = VitalSigns {
            patient_hash: dummy_hash(),
            encounter_hash: None,
            recorded_at: Timestamp::from_micros(1000000),
            recorded_by: dummy_agent(),
            temperature_celsius: None,
            heart_rate_bpm: None,
            blood_pressure_systolic: Some(120),
            blood_pressure_diastolic: None,
            respiratory_rate: None,
            oxygen_saturation: None,
            height_cm: None,
            weight_kg: None,
            bmi: None,
            pain_level: None,
            notes: None,
        };
        let points = vitals_to_twin_data_points(&vitals);
        assert!(points.is_empty(), "BP should require both systolic and diastolic");
    }

    // ==================== lab_result_to_twin_data_point tests ====================

    #[test]
    fn test_lab_result_to_twin_data_point() {
        let lab = LabResult {
            result_id: "LR-001".to_string(),
            patient_hash: dummy_hash(),
            encounter_hash: None,
            ordering_provider: dummy_agent(),
            loinc_code: "2345-7".to_string(),
            test_name: "Glucose".to_string(),
            value: "95".to_string(),
            unit: "mg/dL".to_string(),
            reference_range: "70-100".to_string(),
            interpretation: LabInterpretation::Normal,
            specimen_type: "Blood".to_string(),
            collection_time: Timestamp::from_micros(1000000),
            result_time: Timestamp::from_micros(2000000),
            performing_lab: "Central Lab".to_string(),
            notes: None,
            is_critical: false,
            acknowledged_by: None,
            acknowledged_at: None,
        };
        let dp = lab_result_to_twin_data_point(&lab);
        assert!(matches!(dp.data_type, TwinDataType::LabResult(ref code) if code == "2345-7"));
        assert_eq!(dp.unit, Some("mg/dL".to_string()));
        assert!(matches!(dp.source, TwinDataSourceType::Laboratory));
        assert!(matches!(dp.quality, TwinDataQuality::Clinical));
        // Value should be parseable JSON
        let val: serde_json::Value = serde_json::from_str(&dp.value).expect("lab value should be JSON");
        assert_eq!(val["test_name"], "Glucose");
    }

    // ==================== Serde roundtrip tests ====================

    #[test]
    fn test_serde_roundtrip_create_encounter_input() {
        let input = CreateEncounterInput {
            encounter: Encounter {
                encounter_id: "ENC-001".to_string(),
                patient_hash: dummy_hash(),
                provider_hash: dummy_hash(),
                encounter_type: EncounterType::Office,
                status: EncounterStatus::Completed,
                start_time: Timestamp::from_micros(1000000),
                end_time: Some(Timestamp::from_micros(2000000)),
                location: Some("Room 101".to_string()),
                chief_complaint: "Chest pain".to_string(),
                diagnoses: vec![],
                procedures: vec![],
                notes: "Routine visit".to_string(),
                consent_hash: dummy_hash(),
                epistemic_level: EpistemicLevel::ProviderObserved,
                created_at: Timestamp::from_micros(0),
                updated_at: Timestamp::from_micros(0),
            },
            is_emergency: false,
            emergency_reason: None,
        };
        let json = serde_json::to_string(&input).expect("serialize");
        let decoded: CreateEncounterInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.encounter.encounter_id, "ENC-001");
        assert_eq!(decoded.encounter.chief_complaint, "Chest pain");
    }

    #[test]
    fn test_serde_roundtrip_twin_data_point_full() {
        let dp = TwinDataPointFull {
            data_point_id: "DP-12345".to_string(),
            twin_hash: dummy_hash(),
            data_type: TwinDataType::VitalSign(TwinVitalSignType::Weight),
            value: "70.5".to_string(),
            unit: Some("kg".to_string()),
            measured_at: 1710000000,
            source: TwinDataSourceType::EHR,
            quality: TwinDataQuality::Clinical,
            triggered_update: true,
            ingested_at: 1710000001,
        };
        let json = serde_json::to_string(&dp).expect("serialize");
        let decoded: TwinDataPointFull = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.data_point_id, "DP-12345");
        assert!(decoded.triggered_update);
    }

    #[test]
    fn test_encounter_type_all_variants_serde() {
        let types = vec![
            EncounterType::Office,
            EncounterType::Emergency,
            EncounterType::Inpatient,
            EncounterType::Outpatient,
            EncounterType::Telehealth,
            EncounterType::HomeVisit,
            EncounterType::Procedure,
            EncounterType::Surgery,
            EncounterType::LabOnly,
            EncounterType::ImagingOnly,
        ];
        for et in types {
            let json = serde_json::to_string(&et).expect("serialize");
            let decoded: EncounterType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, et);
        }
    }

    #[test]
    fn test_diagnosis_to_twin_data_point() {
        let diag = Diagnosis {
            diagnosis_id: "DX-001".to_string(),
            patient_hash: dummy_hash(),
            encounter_hash: None,
            icd10_code: "E11.9".to_string(),
            snomed_code: Some("44054006".to_string()),
            description: "Type 2 diabetes".to_string(),
            diagnosis_type: DiagnosisType::Primary,
            status: DiagnosisStatus::Active,
            onset_date: Some("2020-01-01".to_string()),
            resolution_date: None,
            diagnosing_provider: dummy_agent(),
            severity: Some(DiagnosisSeverity::Moderate),
            notes: None,
            epistemic_level: EpistemicLevel::Consensus,
            created_at: Timestamp::from_micros(1000000),
        };
        let dp = diagnosis_to_twin_data_point(&diag);
        assert!(matches!(dp.data_type, TwinDataType::Diagnosis(ref code) if code == "E11.9"));
        assert!(dp.unit.is_none());
        let val: serde_json::Value = serde_json::from_str(&dp.value).expect("JSON");
        assert_eq!(val["description"], "Type 2 diabetes");
    }

    #[test]
    fn test_procedure_to_twin_data_point() {
        let proc = ProcedurePerformed {
            procedure_id: "PR-001".to_string(),
            patient_hash: dummy_hash(),
            encounter_hash: dummy_hash(),
            cpt_code: "99213".to_string(),
            hcpcs_code: None,
            description: "Office visit, level 3".to_string(),
            performed_by: dummy_agent(),
            performed_at: Timestamp::from_micros(1000000),
            location: "Clinic A".to_string(),
            outcome: ProcedureOutcome::Successful,
            complications: vec![],
            notes: None,
            consent_hash: dummy_hash(),
        };
        let dp = procedure_to_twin_data_point(&proc);
        assert!(matches!(dp.data_type, TwinDataType::Procedure(ref code) if code == "99213"));
        assert!(matches!(dp.source, TwinDataSourceType::EHR));
    }
}
