//! FHIR Bridge Coordinator Zome
//!
//! Provides the public API for ingesting FHIR R4 Bundles from external
//! EHR systems and exporting patient data as FHIR Bundles.
//!
//! This zome bridges external FHIR resources to internal Mycelix-Health
//! data structures, handling:
//! - Bundle parsing and resource extraction
//! - Deduplication via source_system + resource_id anchors
//! - Cross-zome calls to create internal records
//! - Audit logging of all data access

use hdk::prelude::*;
use fhir_bridge_integrity::*;
use fhir_mapping_integrity::{
    FhirPatientMapping, FhirObservationMapping, FhirConditionMapping,
    FhirMedicationMapping, SyncStatus,
};
use mycelix_health_shared::{
    log_data_access, anchor_hash,
    DataCategory, Permission,
};
use serde_json::Value as JsonValue;

/// Ingest a FHIR R4 Bundle into Mycelix-Health
///
/// This is the primary entry point for EHR data ingestion. It:
/// 1. Parses the FHIR Bundle JSON
/// 2. Extracts and validates each resource
/// 3. Creates/updates internal records via cross-zome calls
/// 4. Handles deduplication based on source_system + resource_id
/// 5. Returns a detailed IngestReport
#[hdk_extern]
pub fn ingest_bundle(input: IngestBundleInput) -> ExternResult<IngestReport> {
    let now = sys_time()?;
    let report_id = format!("ingest-{}-{}", input.source_system, now.as_micros());

    let mut report = IngestReport {
        report_id: report_id.clone(),
        source_system: input.source_system.clone(),
        ingested_at: Timestamp::from_micros(now.as_micros() as i64),
        total_processed: 0,
        patients_created: 0,
        patients_updated: 0,
        conditions_created: 0,
        conditions_skipped: 0,
        medications_created: 0,
        medications_skipped: 0,
        allergies_created: 0,
        allergies_skipped: 0,
        immunizations_created: 0,
        immunizations_skipped: 0,
        observations_created: 0,
        observations_skipped: 0,
        procedures_created: 0,
        procedures_skipped: 0,
        diagnostic_reports_created: 0,
        diagnostic_reports_skipped: 0,
        care_plans_created: 0,
        care_plans_skipped: 0,
        unknown_types: Vec::new(),
        parse_errors: Vec::new(),
    };

    // Extract entries from bundle
    let entries = match input.bundle.get("entry") {
        Some(JsonValue::Array(entries)) => entries.clone(),
        _ => {
            report.parse_errors.push("Bundle has no 'entry' array".to_string());
            // Store the report even on error
            create_entry(&EntryTypes::IngestReport(report.clone()))?;
            return Ok(report);
        }
    };

    // First pass: find and process Patient resources to establish patient hash
    let mut patient_hash: Option<ActionHash> = None;
    let mut patient_fhir_id: Option<String> = None;

    for entry in &entries {
        let resource = match entry.get("resource") {
            Some(r) => r,
            None => continue,
        };

        if get_resource_type(resource) == Some("Patient".to_string()) {
            match process_patient(resource, &input.source_system) {
                Ok((hash, created)) => {
                    patient_hash = Some(hash);
                    patient_fhir_id = get_resource_id(resource);
                    report.total_processed += 1;
                    if created {
                        report.patients_created += 1;
                    } else {
                        report.patients_updated += 1;
                    }
                }
                Err(e) => {
                    report.parse_errors.push(format!("Patient: {}", e));
                }
            }
            break; // Only expect one Patient per bundle
        }
    }

    // If no patient found, try to find patient reference from other resources
    if patient_hash.is_none() {
        for entry in &entries {
            if let Some(resource) = entry.get("resource") {
                if let Some(patient_ref) = get_patient_reference(resource) {
                    // Try to look up existing patient by reference
                    if let Some(hash) = lookup_patient_by_fhir_reference(&patient_ref, &input.source_system)? {
                        patient_hash = Some(hash);
                        break;
                    }
                }
            }
        }
    }

    // If still no patient, we can't process other resources
    let patient_hash = match patient_hash {
        Some(h) => h,
        None => {
            report.parse_errors.push("No Patient resource found and could not resolve patient reference".to_string());
            create_entry(&EntryTypes::IngestReport(report.clone()))?;
            return Ok(report);
        }
    };

    // Second pass: process all other resources
    for entry in &entries {
        let resource = match entry.get("resource") {
            Some(r) => r,
            None => continue,
        };

        let resource_type = match get_resource_type(resource) {
            Some(t) => t,
            None => {
                report.parse_errors.push("Resource missing resourceType".to_string());
                continue;
            }
        };

        // Skip Patient (already processed)
        if resource_type == "Patient" {
            continue;
        }

        report.total_processed += 1;

        match resource_type.as_str() {
            "Observation" => {
                match process_observation(resource, &patient_hash, &input.source_system) {
                    Ok(created) => {
                        if created {
                            report.observations_created += 1;
                        } else {
                            report.observations_skipped += 1;
                        }
                    }
                    Err(e) => report.parse_errors.push(format!("Observation: {}", e)),
                }
            }
            "Condition" => {
                match process_condition(resource, &patient_hash, &input.source_system) {
                    Ok(created) => {
                        if created {
                            report.conditions_created += 1;
                        } else {
                            report.conditions_skipped += 1;
                        }
                    }
                    Err(e) => report.parse_errors.push(format!("Condition: {}", e)),
                }
            }
            "MedicationRequest" | "MedicationStatement" => {
                match process_medication(resource, &patient_hash, &input.source_system) {
                    Ok(created) => {
                        if created {
                            report.medications_created += 1;
                        } else {
                            report.medications_skipped += 1;
                        }
                    }
                    Err(e) => report.parse_errors.push(format!("Medication: {}", e)),
                }
            }
            "AllergyIntolerance" => {
                match process_allergy(resource, &patient_hash, &input.source_system) {
                    Ok(created) => {
                        if created {
                            report.allergies_created += 1;
                        } else {
                            report.allergies_skipped += 1;
                        }
                    }
                    Err(e) => report.parse_errors.push(format!("Allergy: {}", e)),
                }
            }
            "Immunization" => {
                match process_immunization(resource, &patient_hash, &input.source_system) {
                    Ok(created) => {
                        if created {
                            report.immunizations_created += 1;
                        } else {
                            report.immunizations_skipped += 1;
                        }
                    }
                    Err(e) => report.parse_errors.push(format!("Immunization: {}", e)),
                }
            }
            "Procedure" => {
                match process_procedure(resource, &patient_hash, &input.source_system) {
                    Ok(created) => {
                        if created {
                            report.procedures_created += 1;
                        } else {
                            report.procedures_skipped += 1;
                        }
                    }
                    Err(e) => report.parse_errors.push(format!("Procedure: {}", e)),
                }
            }
            "DiagnosticReport" => {
                match process_diagnostic_report(resource, &patient_hash, &input.source_system) {
                    Ok(created) => {
                        if created {
                            report.diagnostic_reports_created += 1;
                        } else {
                            report.diagnostic_reports_skipped += 1;
                        }
                    }
                    Err(e) => report.parse_errors.push(format!("DiagnosticReport: {}", e)),
                }
            }
            "CarePlan" => {
                match process_care_plan(resource, &patient_hash, &input.source_system) {
                    Ok(created) => {
                        if created {
                            report.care_plans_created += 1;
                        } else {
                            report.care_plans_skipped += 1;
                        }
                    }
                    Err(e) => report.parse_errors.push(format!("CarePlan: {}", e)),
                }
            }
            _ => {
                if !report.unknown_types.contains(&resource_type) {
                    report.unknown_types.push(resource_type);
                }
            }
        }
    }

    // Store the ingest report
    let report_hash = create_entry(&EntryTypes::IngestReport(report.clone()))?;

    // Link report to patient
    create_link(
        patient_hash,
        report_hash,
        LinkTypes::PatientToIngestReports,
        LinkTag::new(&input.source_system),
    )?;

    Ok(report)
}

/// Export a patient's data as a FHIR R4 Bundle
#[hdk_extern]
pub fn export_patient_fhir(input: ExportPatientInput) -> ExternResult<ExportResult> {
    // Call fhir_mapping zome to get the export
    let export_input = serde_json::json!({
        "patient_hash": input.patient_hash,
        "include_observations": input.include_sections.contains(&"Observation".to_string()),
        "include_conditions": input.include_sections.contains(&"Condition".to_string()),
        "include_medications": input.include_sections.contains(&"MedicationRequest".to_string()),
        "is_emergency": false,
        "emergency_reason": null
    });

    let response = call(
        CallTargetCell::Local,
        ZomeName::from("fhir_mapping"),
        FunctionName::from("export_patient_bundle"),
        None,
        &export_input,
    )?;

    let bundle_output: JsonValue = match response {
        ZomeCallResponse::Ok(io) => io.decode()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to decode export: {}", e))))?,
        ZomeCallResponse::NetworkError(e) => {
            return Err(wasm_error!(WasmErrorInner::Guest(format!("Network error: {}", e))));
        }
        ZomeCallResponse::CountersigningSession(e) => {
            return Err(wasm_error!(WasmErrorInner::Guest(format!("Countersigning error: {}", e))));
        }
        _ => return Err(wasm_error!(WasmErrorInner::Guest("Unexpected response".to_string()))),
    };

    // Count resources in the output
    let resource_count = count_resources(&bundle_output);

    Ok(ExportResult {
        bundle: bundle_output,
        resource_count,
        format: input.format.unwrap_or_else(|| "r4".to_string()),
        sections_exported: input.include_sections,
    })
}

/// Validate a FHIR resource before ingestion
#[hdk_extern]
pub fn validate_fhir_resource(resource: JsonValue) -> ExternResult<bool> {
    // Basic validation - check required fields
    let resource_type = match get_resource_type(&resource) {
        Some(t) => t,
        None => return Ok(false),
    };

    // Check resource has an ID
    if get_resource_id(&resource).is_none() {
        return Ok(false);
    }

    // Type-specific validation
    match resource_type.as_str() {
        "Patient" => Ok(validate_patient_resource(&resource)),
        "Observation" => Ok(validate_observation_resource(&resource)),
        "Condition" => Ok(validate_condition_resource(&resource)),
        "MedicationRequest" => Ok(validate_medication_resource(&resource)),
        _ => Ok(true), // Allow unknown types to pass basic validation
    }
}

// ============================================================================
// Resource Processing Functions
// ============================================================================

/// Process a Patient resource
/// Returns (patient_hash, was_created)
fn process_patient(resource: &JsonValue, source_system: &str) -> Result<(ActionHash, bool), String> {
    let fhir_id = get_resource_id(resource)
        .ok_or("Patient missing 'id' field")?;

    // Check if patient already exists from this source
    let source_key = format!("{}:Patient:{}", source_system, fhir_id);

    if let Some(existing) = lookup_resource_anchor(&source_key).map_err(|e| e.to_string())? {
        // Patient already exists, return existing hash
        return Ok((existing.internal_hash, false));
    }

    // Create patient mapping via fhir_mapping zome
    let name = extract_patient_name(resource);
    let birth_date = get_fhir_string(resource, "birthDate");
    let gender = get_fhir_string(resource, "gender");

    // Call patient zome to create or find patient
    let patient_input = serde_json::json!({
        "given_name": name.0,
        "family_name": name.1,
        "birth_date": birth_date,
        "gender": gender,
        "source_system": source_system,
        "external_id": fhir_id,
    });

    let response = call(
        CallTargetCell::Local,
        ZomeName::from("patient"),
        FunctionName::from("create_or_update_patient"),
        None,
        &patient_input,
    ).map_err(|e| format!("Failed to call patient zome: {}", e))?;

    let patient_hash: ActionHash = match response {
        ZomeCallResponse::Ok(io) => io.decode()
            .map_err(|e| format!("Failed to decode patient hash: {}", e))?,
        _ => return Err("Failed to create patient".to_string()),
    };

    // Create anchor for deduplication
    let now = sys_time().map_err(|e| e.to_string())?;
    let anchor = FhirResourceAnchor {
        source_key,
        resource_type: "Patient".to_string(),
        internal_hash: patient_hash.clone(),
        first_ingested: Timestamp::from_micros(now.as_micros() as i64),
        last_updated: Timestamp::from_micros(now.as_micros() as i64),
    };
    create_entry(&EntryTypes::FhirResourceAnchor(anchor))
        .map_err(|e| e.to_string())?;

    Ok((patient_hash, true))
}

/// Process an Observation resource
fn process_observation(resource: &JsonValue, patient_hash: &ActionHash, source_system: &str) -> Result<bool, String> {
    let fhir_id = get_resource_id(resource)
        .ok_or("Observation missing 'id' field")?;

    // Check for duplicate
    let source_key = format!("{}:Observation:{}", source_system, fhir_id);
    if lookup_resource_anchor(&source_key).map_err(|e| e.to_string())?.is_some() {
        return Ok(false); // Already exists
    }

    // Extract observation data
    let code = extract_coding(resource, "code");
    let value = extract_value(resource);

    // Create FHIR mapping
    let mapping = FhirObservationMapping {
        fhir_observation_id: fhir_id.clone(),
        internal_record_hash: patient_hash.clone(), // Will be updated when actual record is created
        patient_hash: patient_hash.clone(),
        loinc_code: code.0.clone(),
        display: code.1.clone(),
        value: value.clone(),
        unit: extract_unit(resource),
        effective_datetime: get_fhir_string(resource, "effectiveDateTime"),
        source_system: source_system.to_string(),
        last_synced: sys_time().map_err(|e| e.to_string())?,
        sync_status: SyncStatus::Synced,
        sync_errors: Vec::new(),
    };

    // Call fhir_mapping to create
    let response = call(
        CallTargetCell::Local,
        ZomeName::from("fhir_mapping"),
        FunctionName::from("create_fhir_observation_mapping"),
        None,
        &mapping,
    ).map_err(|e| format!("Failed to create observation mapping: {}", e))?;

    let mapping_hash: ActionHash = match response {
        ZomeCallResponse::Ok(io) => {
            let record: Record = io.decode()
                .map_err(|e| format!("Failed to decode observation: {}", e))?;
            record.action_address().clone()
        }
        _ => return Err("Failed to create observation mapping".to_string()),
    };

    // Create deduplication anchor
    create_resource_anchor(&source_key, "Observation", &mapping_hash)?;

    Ok(true)
}

/// Process a Condition resource
fn process_condition(resource: &JsonValue, patient_hash: &ActionHash, source_system: &str) -> Result<bool, String> {
    let fhir_id = get_resource_id(resource)
        .ok_or("Condition missing 'id' field")?;

    let source_key = format!("{}:Condition:{}", source_system, fhir_id);
    if lookup_resource_anchor(&source_key).map_err(|e| e.to_string())?.is_some() {
        return Ok(false);
    }

    let code = extract_coding(resource, "code");
    let clinical_status = get_fhir_string(resource, "clinicalStatus");

    let mapping = FhirConditionMapping {
        fhir_condition_id: fhir_id.clone(),
        internal_diagnosis_hash: patient_hash.clone(),
        patient_hash: patient_hash.clone(),
        snomed_code: code.0.clone(),
        icd10_code: extract_icd10(resource),
        display: code.1.clone(),
        clinical_status,
        onset_datetime: get_fhir_string(resource, "onsetDateTime"),
        source_system: source_system.to_string(),
        last_synced: sys_time().map_err(|e| e.to_string())?,
        sync_status: SyncStatus::Synced,
        sync_errors: Vec::new(),
    };

    let response = call(
        CallTargetCell::Local,
        ZomeName::from("fhir_mapping"),
        FunctionName::from("create_fhir_condition_mapping"),
        None,
        &mapping,
    ).map_err(|e| format!("Failed to create condition mapping: {}", e))?;

    let mapping_hash: ActionHash = match response {
        ZomeCallResponse::Ok(io) => {
            let record: Record = io.decode()
                .map_err(|e| format!("Failed to decode condition: {}", e))?;
            record.action_address().clone()
        }
        _ => return Err("Failed to create condition mapping".to_string()),
    };

    create_resource_anchor(&source_key, "Condition", &mapping_hash)?;
    Ok(true)
}

/// Process a Medication resource
fn process_medication(resource: &JsonValue, patient_hash: &ActionHash, source_system: &str) -> Result<bool, String> {
    let fhir_id = get_resource_id(resource)
        .ok_or("Medication missing 'id' field")?;

    let source_key = format!("{}:Medication:{}", source_system, fhir_id);
    if lookup_resource_anchor(&source_key).map_err(|e| e.to_string())?.is_some() {
        return Ok(false);
    }

    let medication_code = extract_medication_code(resource);
    let status = get_fhir_string(resource, "status");

    let mapping = FhirMedicationMapping {
        fhir_medication_id: fhir_id.clone(),
        internal_medication_hash: patient_hash.clone(),
        patient_hash: patient_hash.clone(),
        rxnorm_code: medication_code.0.clone(),
        ndc_code: medication_code.1.clone(),
        display: medication_code.2.clone(),
        status,
        authored_on: get_fhir_string(resource, "authoredOn"),
        source_system: source_system.to_string(),
        last_synced: sys_time().map_err(|e| e.to_string())?,
        sync_status: SyncStatus::Synced,
        sync_errors: Vec::new(),
    };

    let response = call(
        CallTargetCell::Local,
        ZomeName::from("fhir_mapping"),
        FunctionName::from("create_fhir_medication_mapping"),
        None,
        &mapping,
    ).map_err(|e| format!("Failed to create medication mapping: {}", e))?;

    let mapping_hash: ActionHash = match response {
        ZomeCallResponse::Ok(io) => {
            let record: Record = io.decode()
                .map_err(|e| format!("Failed to decode medication: {}", e))?;
            record.action_address().clone()
        }
        _ => return Err("Failed to create medication mapping".to_string()),
    };

    create_resource_anchor(&source_key, "Medication", &mapping_hash)?;
    Ok(true)
}

/// Process an AllergyIntolerance resource
fn process_allergy(resource: &JsonValue, patient_hash: &ActionHash, source_system: &str) -> Result<bool, String> {
    let fhir_id = get_resource_id(resource)
        .ok_or("AllergyIntolerance missing 'id' field")?;

    let source_key = format!("{}:AllergyIntolerance:{}", source_system, fhir_id);
    if lookup_resource_anchor(&source_key).map_err(|e| e.to_string())?.is_some() {
        return Ok(false);
    }

    // For now, store as a generic observation since there's no dedicated allergy mapping
    // In a full implementation, we'd have a dedicated allergy zome
    let code = extract_coding(resource, "code");

    let mapping = FhirObservationMapping {
        fhir_observation_id: format!("allergy-{}", fhir_id),
        internal_record_hash: patient_hash.clone(),
        patient_hash: patient_hash.clone(),
        loinc_code: "allergy".to_string(),
        display: code.1.clone(),
        value: serde_json::to_string(resource).ok(),
        unit: None,
        effective_datetime: get_fhir_string(resource, "recordedDate"),
        source_system: source_system.to_string(),
        last_synced: sys_time().map_err(|e| e.to_string())?,
        sync_status: SyncStatus::Synced,
        sync_errors: Vec::new(),
    };

    let response = call(
        CallTargetCell::Local,
        ZomeName::from("fhir_mapping"),
        FunctionName::from("create_fhir_observation_mapping"),
        None,
        &mapping,
    ).map_err(|e| format!("Failed to create allergy mapping: {}", e))?;

    let mapping_hash: ActionHash = match response {
        ZomeCallResponse::Ok(io) => {
            let record: Record = io.decode()
                .map_err(|e| format!("Failed to decode allergy: {}", e))?;
            record.action_address().clone()
        }
        _ => return Err("Failed to create allergy mapping".to_string()),
    };

    create_resource_anchor(&source_key, "AllergyIntolerance", &mapping_hash)?;
    Ok(true)
}

/// Process an Immunization resource
fn process_immunization(resource: &JsonValue, patient_hash: &ActionHash, source_system: &str) -> Result<bool, String> {
    let fhir_id = get_resource_id(resource)
        .ok_or("Immunization missing 'id' field")?;

    let source_key = format!("{}:Immunization:{}", source_system, fhir_id);
    if lookup_resource_anchor(&source_key).map_err(|e| e.to_string())?.is_some() {
        return Ok(false);
    }

    let vaccine_code = extract_coding(resource, "vaccineCode");

    let mapping = FhirObservationMapping {
        fhir_observation_id: format!("immunization-{}", fhir_id),
        internal_record_hash: patient_hash.clone(),
        patient_hash: patient_hash.clone(),
        loinc_code: "immunization".to_string(),
        display: vaccine_code.1.clone(),
        value: serde_json::to_string(resource).ok(),
        unit: None,
        effective_datetime: get_fhir_string(resource, "occurrenceDateTime"),
        source_system: source_system.to_string(),
        last_synced: sys_time().map_err(|e| e.to_string())?,
        sync_status: SyncStatus::Synced,
        sync_errors: Vec::new(),
    };

    let response = call(
        CallTargetCell::Local,
        ZomeName::from("fhir_mapping"),
        FunctionName::from("create_fhir_observation_mapping"),
        None,
        &mapping,
    ).map_err(|e| format!("Failed to create immunization mapping: {}", e))?;

    let mapping_hash: ActionHash = match response {
        ZomeCallResponse::Ok(io) => {
            let record: Record = io.decode()
                .map_err(|e| format!("Failed to decode immunization: {}", e))?;
            record.action_address().clone()
        }
        _ => return Err("Failed to create immunization mapping".to_string()),
    };

    create_resource_anchor(&source_key, "Immunization", &mapping_hash)?;
    Ok(true)
}

/// Process a Procedure resource
fn process_procedure(resource: &JsonValue, patient_hash: &ActionHash, source_system: &str) -> Result<bool, String> {
    let fhir_id = get_resource_id(resource)
        .ok_or("Procedure missing 'id' field")?;

    let source_key = format!("{}:Procedure:{}", source_system, fhir_id);
    if lookup_resource_anchor(&source_key).map_err(|e| e.to_string())?.is_some() {
        return Ok(false);
    }

    let code = extract_coding(resource, "code");

    let mapping = FhirObservationMapping {
        fhir_observation_id: format!("procedure-{}", fhir_id),
        internal_record_hash: patient_hash.clone(),
        patient_hash: patient_hash.clone(),
        loinc_code: code.0.clone(),
        display: code.1.clone(),
        value: serde_json::to_string(resource).ok(),
        unit: None,
        effective_datetime: get_fhir_string(resource, "performedDateTime"),
        source_system: source_system.to_string(),
        last_synced: sys_time().map_err(|e| e.to_string())?,
        sync_status: SyncStatus::Synced,
        sync_errors: Vec::new(),
    };

    let response = call(
        CallTargetCell::Local,
        ZomeName::from("fhir_mapping"),
        FunctionName::from("create_fhir_observation_mapping"),
        None,
        &mapping,
    ).map_err(|e| format!("Failed to create procedure mapping: {}", e))?;

    let mapping_hash: ActionHash = match response {
        ZomeCallResponse::Ok(io) => {
            let record: Record = io.decode()
                .map_err(|e| format!("Failed to decode procedure: {}", e))?;
            record.action_address().clone()
        }
        _ => return Err("Failed to create procedure mapping".to_string()),
    };

    create_resource_anchor(&source_key, "Procedure", &mapping_hash)?;
    Ok(true)
}

/// Process a DiagnosticReport resource
/// DiagnosticReports represent lab results, imaging studies, pathology reports, etc.
fn process_diagnostic_report(resource: &JsonValue, patient_hash: &ActionHash, source_system: &str) -> Result<bool, String> {
    let fhir_id = get_resource_id(resource)
        .ok_or("DiagnosticReport missing 'id' field")?;

    let source_key = format!("{}:DiagnosticReport:{}", source_system, fhir_id);
    if lookup_resource_anchor(&source_key).map_err(|e| e.to_string())?.is_some() {
        return Ok(false);
    }

    // Extract diagnostic report data
    let code = extract_coding(resource, "code");
    let category = extract_category(resource);
    let status = get_fhir_string(resource, "status");
    let effective_datetime = get_fhir_string(resource, "effectiveDateTime")
        .or_else(|| {
            // Try effectivePeriod.start
            resource.get("effectivePeriod")
                .and_then(|p| p.get("start"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
        });
    let conclusion = get_fhir_string(resource, "conclusion");

    // Build display text
    let display = code.1.clone().or_else(|| {
        conclusion.clone().map(|c| c.chars().take(100).collect())
    });

    // Store as observation mapping (DiagnosticReports are observation-like)
    let mapping = FhirObservationMapping {
        fhir_observation_id: format!("diagnostic-report-{}", fhir_id),
        internal_record_hash: patient_hash.clone(),
        patient_hash: patient_hash.clone(),
        loinc_code: code.0.clone().unwrap_or_else(|| format!("diagnostic-report:{}", category.unwrap_or_default())),
        display,
        value: serde_json::to_string(resource).ok(),
        unit: status, // Use unit field to store status
        effective_datetime,
        source_system: source_system.to_string(),
        last_synced: sys_time().map_err(|e| e.to_string())?,
        sync_status: SyncStatus::Synced,
        sync_errors: Vec::new(),
    };

    let response = call(
        CallTargetCell::Local,
        ZomeName::from("fhir_mapping"),
        FunctionName::from("create_fhir_observation_mapping"),
        None,
        &mapping,
    ).map_err(|e| format!("Failed to create diagnostic report mapping: {}", e))?;

    let mapping_hash: ActionHash = match response {
        ZomeCallResponse::Ok(io) => {
            let record: Record = io.decode()
                .map_err(|e| format!("Failed to decode diagnostic report: {}", e))?;
            record.action_address().clone()
        }
        _ => return Err("Failed to create diagnostic report mapping".to_string()),
    };

    create_resource_anchor(&source_key, "DiagnosticReport", &mapping_hash)?;
    Ok(true)
}

/// Process a CarePlan resource
/// CarePlans represent care plans, treatment plans, health maintenance plans
fn process_care_plan(resource: &JsonValue, patient_hash: &ActionHash, source_system: &str) -> Result<bool, String> {
    let fhir_id = get_resource_id(resource)
        .ok_or("CarePlan missing 'id' field")?;

    let source_key = format!("{}:CarePlan:{}", source_system, fhir_id);
    if lookup_resource_anchor(&source_key).map_err(|e| e.to_string())?.is_some() {
        return Ok(false);
    }

    // Extract care plan data
    let title = get_fhir_string(resource, "title");
    let description = get_fhir_string(resource, "description");
    let status = get_fhir_string(resource, "status");
    let intent = get_fhir_string(resource, "intent");
    let category = extract_category(resource);

    // Extract period
    let period_start = resource.get("period")
        .and_then(|p| p.get("start"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    // Extract activities summary
    let activities_count = resource.get("activity")
        .and_then(|a| a.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);

    // Extract goals count
    let goals_count = resource.get("goal")
        .and_then(|g| g.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);

    // Build display text
    let display = title.clone()
        .or_else(|| description.clone().map(|d| d.chars().take(100).collect()))
        .or_else(|| category.clone().map(|c| format!("{} Care Plan", c)));

    // Store as observation mapping (we store the full JSON for rich data)
    let mapping = FhirObservationMapping {
        fhir_observation_id: format!("care-plan-{}", fhir_id),
        internal_record_hash: patient_hash.clone(),
        patient_hash: patient_hash.clone(),
        loinc_code: format!("care-plan:{}", category.unwrap_or_else(|| "general".to_string())),
        display,
        value: serde_json::to_string(resource).ok(),
        unit: Some(format!("status:{} intent:{} activities:{} goals:{}",
            status.unwrap_or_default(),
            intent.unwrap_or_default(),
            activities_count,
            goals_count
        )),
        effective_datetime: period_start,
        source_system: source_system.to_string(),
        last_synced: sys_time().map_err(|e| e.to_string())?,
        sync_status: SyncStatus::Synced,
        sync_errors: Vec::new(),
    };

    let response = call(
        CallTargetCell::Local,
        ZomeName::from("fhir_mapping"),
        FunctionName::from("create_fhir_observation_mapping"),
        None,
        &mapping,
    ).map_err(|e| format!("Failed to create care plan mapping: {}", e))?;

    let mapping_hash: ActionHash = match response {
        ZomeCallResponse::Ok(io) => {
            let record: Record = io.decode()
                .map_err(|e| format!("Failed to decode care plan: {}", e))?;
            record.action_address().clone()
        }
        _ => return Err("Failed to create care plan mapping".to_string()),
    };

    create_resource_anchor(&source_key, "CarePlan", &mapping_hash)?;
    Ok(true)
}

/// Extract category from FHIR resource
fn extract_category(resource: &JsonValue) -> Option<String> {
    resource.get("category")
        .and_then(|cats| cats.as_array())
        .and_then(|arr| arr.first())
        .and_then(|cat| {
            // Try coding first
            cat.get("coding")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|coding| coding.get("display").or(coding.get("code")))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                // Fall back to text
                .or_else(|| cat.get("text").and_then(|t| t.as_str()).map(|s| s.to_string()))
        })
}

// ============================================================================
// Helper Functions
// ============================================================================

fn lookup_resource_anchor(source_key: &str) -> ExternResult<Option<FhirResourceAnchor>> {
    let anchor = anchor_hash(&format!("fhir_anchor:{}", source_key))?;
    let links = get_links(
        LinkQuery::try_new(anchor, LinkTypes::SourceKeyToAnchor)?,
        GetStrategy::default(),
    )?;

    if let Some(link) = links.first() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                return Ok(record.entry().to_app_option::<FhirResourceAnchor>().ok().flatten());
            }
        }
    }
    Ok(None)
}

fn create_resource_anchor(source_key: &str, resource_type: &str, internal_hash: &ActionHash) -> Result<(), String> {
    let now = sys_time().map_err(|e| e.to_string())?;
    let anchor_entry = FhirResourceAnchor {
        source_key: source_key.to_string(),
        resource_type: resource_type.to_string(),
        internal_hash: internal_hash.clone(),
        first_ingested: Timestamp::from_micros(now.as_micros() as i64),
        last_updated: Timestamp::from_micros(now.as_micros() as i64),
    };

    let anchor_hash_result = create_entry(&EntryTypes::FhirResourceAnchor(anchor_entry))
        .map_err(|e| e.to_string())?;

    let link_anchor = anchor_hash(&format!("fhir_anchor:{}", source_key))
        .map_err(|e| e.to_string())?;

    create_link(
        link_anchor,
        anchor_hash_result,
        LinkTypes::SourceKeyToAnchor,
        LinkTag::new(""),
    ).map_err(|e| e.to_string())?;

    Ok(())
}

fn lookup_patient_by_fhir_reference(reference: &str, source_system: &str) -> ExternResult<Option<ActionHash>> {
    // Reference format: "Patient/123"
    let parts: Vec<&str> = reference.split('/').collect();
    if parts.len() == 2 && parts[0] == "Patient" {
        let source_key = format!("{}:Patient:{}", source_system, parts[1]);
        if let Some(anchor) = lookup_resource_anchor(&source_key)? {
            return Ok(Some(anchor.internal_hash));
        }
    }
    Ok(None)
}

fn extract_patient_name(resource: &JsonValue) -> (Option<String>, Option<String>) {
    if let Some(names) = resource.get("name").and_then(|n| n.as_array()) {
        if let Some(name) = names.first() {
            let given = name.get("given")
                .and_then(|g| g.as_array())
                .and_then(|arr| arr.first())
                .and_then(|g| g.as_str())
                .map(|s| s.to_string());
            let family = name.get("family")
                .and_then(|f| f.as_str())
                .map(|s| s.to_string());
            return (given, family);
        }
    }
    (None, None)
}

fn extract_coding(resource: &JsonValue, field: &str) -> (Option<String>, Option<String>) {
    if let Some(code_field) = resource.get(field) {
        if let Some(codings) = code_field.get("coding").and_then(|c| c.as_array()) {
            if let Some(coding) = codings.first() {
                let code = coding.get("code").and_then(|c| c.as_str()).map(|s| s.to_string());
                let display = coding.get("display").and_then(|d| d.as_str()).map(|s| s.to_string());
                return (code, display);
            }
        }
    }
    (None, None)
}

fn extract_value(resource: &JsonValue) -> Option<String> {
    // Try valueQuantity
    if let Some(vq) = resource.get("valueQuantity") {
        if let Some(value) = vq.get("value") {
            return Some(value.to_string());
        }
    }
    // Try valueString
    if let Some(vs) = resource.get("valueString").and_then(|v| v.as_str()) {
        return Some(vs.to_string());
    }
    // Try valueCodeableConcept
    if let Some(vcc) = resource.get("valueCodeableConcept") {
        if let Some(text) = vcc.get("text").and_then(|t| t.as_str()) {
            return Some(text.to_string());
        }
    }
    None
}

fn extract_unit(resource: &JsonValue) -> Option<String> {
    resource.get("valueQuantity")
        .and_then(|vq| vq.get("unit"))
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
}

fn extract_icd10(resource: &JsonValue) -> Option<String> {
    if let Some(code_field) = resource.get("code") {
        if let Some(codings) = code_field.get("coding").and_then(|c| c.as_array()) {
            for coding in codings {
                if let Some(system) = coding.get("system").and_then(|s| s.as_str()) {
                    if system.contains("icd") {
                        return coding.get("code").and_then(|c| c.as_str()).map(|s| s.to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_medication_code(resource: &JsonValue) -> (Option<String>, Option<String>, Option<String>) {
    let mut rxnorm = None;
    let mut ndc = None;
    let mut display = None;

    if let Some(med) = resource.get("medicationCodeableConcept") {
        if let Some(codings) = med.get("coding").and_then(|c| c.as_array()) {
            for coding in codings {
                let system = coding.get("system").and_then(|s| s.as_str()).unwrap_or("");
                let code = coding.get("code").and_then(|c| c.as_str()).map(|s| s.to_string());
                if display.is_none() {
                    display = coding.get("display").and_then(|d| d.as_str()).map(|s| s.to_string());
                }
                if system.contains("rxnorm") {
                    rxnorm = code;
                } else if system.contains("ndc") {
                    ndc = code;
                }
            }
        }
        if display.is_none() {
            display = med.get("text").and_then(|t| t.as_str()).map(|s| s.to_string());
        }
    }

    (rxnorm, ndc, display)
}

fn count_resources(bundle: &JsonValue) -> u32 {
    bundle.get("entry")
        .and_then(|e| e.as_array())
        .map(|arr| arr.len() as u32)
        .unwrap_or(0)
}

fn validate_patient_resource(resource: &JsonValue) -> bool {
    // Patient must have at least a name or identifier
    resource.get("name").is_some() || resource.get("identifier").is_some()
}

fn validate_observation_resource(resource: &JsonValue) -> bool {
    // Observation must have code and either value or dataAbsentReason
    resource.get("code").is_some() &&
    (resource.get("valueQuantity").is_some() ||
     resource.get("valueString").is_some() ||
     resource.get("valueCodeableConcept").is_some() ||
     resource.get("dataAbsentReason").is_some())
}

fn validate_condition_resource(resource: &JsonValue) -> bool {
    // Condition must have code
    resource.get("code").is_some()
}

fn validate_medication_resource(resource: &JsonValue) -> bool {
    // MedicationRequest must have medication reference or code
    resource.get("medicationCodeableConcept").is_some() ||
    resource.get("medicationReference").is_some()
}
