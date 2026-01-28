//! FHIR Bridge Integrity Zome
//!
//! Provides entry types and validation for FHIR Bundle ingestion and export.
//! This zome acts as the bridge between external EHR systems (via FHIR R4)
//! and Mycelix-Health's internal data structures.

use hdi::prelude::*;
use serde_json::Value as JsonValue;

/// Input for ingesting a FHIR Bundle
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngestBundleInput {
    /// The FHIR R4 Bundle as JSON
    pub bundle: JsonValue,
    /// Source EHR system identifier (e.g., "epic-sandbox", "cerner-prod")
    pub source_system: String,
}

/// Report of what was ingested from a FHIR Bundle
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IngestReport {
    /// Unique report ID
    pub report_id: String,
    /// Source system that provided the data
    pub source_system: String,
    /// Timestamp of ingestion
    pub ingested_at: Timestamp,
    /// Total resources processed
    pub total_processed: u32,
    /// Patients created
    pub patients_created: u32,
    /// Patients updated (already existed)
    pub patients_updated: u32,
    /// Conditions created
    pub conditions_created: u32,
    /// Conditions skipped (duplicates)
    pub conditions_skipped: u32,
    /// Medications created
    pub medications_created: u32,
    /// Medications skipped
    pub medications_skipped: u32,
    /// Allergies created
    pub allergies_created: u32,
    /// Allergies skipped
    pub allergies_skipped: u32,
    /// Immunizations created
    pub immunizations_created: u32,
    /// Immunizations skipped
    pub immunizations_skipped: u32,
    /// Observations created
    pub observations_created: u32,
    /// Observations skipped
    pub observations_skipped: u32,
    /// Procedures created
    pub procedures_created: u32,
    /// Procedures skipped
    pub procedures_skipped: u32,
    /// DiagnosticReports created
    pub diagnostic_reports_created: u32,
    /// DiagnosticReports skipped
    pub diagnostic_reports_skipped: u32,
    /// CarePlans created
    pub care_plans_created: u32,
    /// CarePlans skipped
    pub care_plans_skipped: u32,
    /// Resource types that were not recognized
    pub unknown_types: Vec<String>,
    /// Errors encountered during parsing
    pub parse_errors: Vec<String>,
}

/// Input for exporting a patient's data as FHIR
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportPatientInput {
    /// Patient hash to export
    pub patient_hash: ActionHash,
    /// Which sections to include
    pub include_sections: Vec<String>,
    /// Format: "r4" (default), "us-core", "ips"
    pub format: Option<String>,
}

/// Result of exporting patient data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportResult {
    /// The FHIR Bundle as JSON
    pub bundle: JsonValue,
    /// Number of resources included
    pub resource_count: u32,
    /// Export format used
    pub format: String,
    /// Sections that were exported
    pub sections_exported: Vec<String>,
}

/// A deduplication anchor for tracking ingested resources
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FhirResourceAnchor {
    /// Source system + resource ID combination
    pub source_key: String,
    /// Type of FHIR resource
    pub resource_type: String,
    /// Internal Holochain hash of the created record
    pub internal_hash: ActionHash,
    /// When this was first ingested
    pub first_ingested: Timestamp,
    /// Last time this resource was updated from source
    pub last_updated: Timestamp,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    IngestReport(IngestReport),
    FhirResourceAnchor(FhirResourceAnchor),
}

#[hdk_link_types]
pub enum LinkTypes {
    /// Source system to its ingested resources
    SourceToResources,
    /// Patient to their ingest reports
    PatientToIngestReports,
    /// Resource type index
    ResourceTypeIndex,
    /// Deduplication anchor by source key
    SourceKeyToAnchor,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::IngestReport(r) => validate_ingest_report(&r),
                EntryTypes::FhirResourceAnchor(a) => validate_resource_anchor(&a),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_ingest_report(report: &IngestReport) -> ExternResult<ValidateCallbackResult> {
    if report.report_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Report ID is required".to_string(),
        ));
    }
    if report.source_system.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Source system is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_resource_anchor(anchor: &FhirResourceAnchor) -> ExternResult<ValidateCallbackResult> {
    if anchor.source_key.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Source key is required".to_string(),
        ));
    }
    if anchor.resource_type.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Resource type is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

/// Helper to extract a string field from FHIR JSON
pub fn get_fhir_string(resource: &JsonValue, field: &str) -> Option<String> {
    resource.get(field).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Helper to extract the resource type from FHIR JSON
pub fn get_resource_type(resource: &JsonValue) -> Option<String> {
    get_fhir_string(resource, "resourceType")
}

/// Helper to extract the resource ID from FHIR JSON
pub fn get_resource_id(resource: &JsonValue) -> Option<String> {
    get_fhir_string(resource, "id")
}

/// Helper to extract patient reference from a FHIR resource
pub fn get_patient_reference(resource: &JsonValue) -> Option<String> {
    // Try direct "patient" field
    if let Some(patient) = resource.get("patient") {
        if let Some(reference) = patient.get("reference").and_then(|r| r.as_str()) {
            return Some(reference.to_string());
        }
    }
    // Try "subject" field (used in Observation, Condition, etc.)
    if let Some(subject) = resource.get("subject") {
        if let Some(reference) = subject.get("reference").and_then(|r| r.as_str()) {
            return Some(reference.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_resource_type() {
        let json: JsonValue = serde_json::json!({
            "resourceType": "Patient",
            "id": "123"
        });
        assert_eq!(get_resource_type(&json), Some("Patient".to_string()));
    }

    #[test]
    fn test_get_patient_reference() {
        let obs: JsonValue = serde_json::json!({
            "resourceType": "Observation",
            "subject": {
                "reference": "Patient/123"
            }
        });
        assert_eq!(get_patient_reference(&obs), Some("Patient/123".to_string()));
    }
}
