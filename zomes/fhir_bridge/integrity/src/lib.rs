#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
// clippy::collapsible_match is allowed crate-wide here. Every instance is the
// standard HDK validation-dispatch shape:
//
//     FlatOp::RegisterUpdate(op) => match op {
//         OpUpdate::Entry { app_entry, action } => validate_update_entry(action, app_entry),
//         _ => Ok(ValidateCallbackResult::Valid),
//     }
//
// Collapsing it into the outer pattern would force a separate catch-all arm for
// the remaining OpUpdate variants, diverge from every sibling integrity zome, and
// restructure the exact RegisterUpdate dispatch the author-binding work depends on
// -- a real risk in security-critical validation for a style lint. This crate is
// validation dispatch end to end, so the allow is scoped to what it describes.
#![allow(clippy::collapsible_match)]

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
    resource
        .get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Helper to extract the resource type from FHIR JSON
pub fn get_resource_type(resource: &JsonValue) -> Option<String> {
    get_fhir_string(resource, "resourceType")
}

/// Helper to extract the resource ID from FHIR JSON
pub fn get_resource_id(resource: &JsonValue) -> Option<String> {
    get_fhir_string(resource, "id")
}

/// Extract every patient-bearing reference field present on a FHIR resource.
/// Returning all references lets the coordinator reject conflicting subject,
/// patient and context references instead of silently trusting only the first.
pub fn get_patient_references(resource: &JsonValue) -> Vec<String> {
    ["patient", "subject"]
        .iter()
        .filter_map(|field| {
            resource
                .get(*field)
                .and_then(|value| value.get("reference"))
                .and_then(|reference| reference.as_str())
                .map(str::to_string)
        })
        .collect()
}

/// Backwards-compatible convenience accessor. Security-sensitive ingestion
/// should use get_patient_references and validate every returned reference.
pub fn get_patient_reference(resource: &JsonValue) -> Option<String> {
    get_patient_references(resource).into_iter().next()
}

/// Identifier-only patient references cannot be tied to a Bundle's Patient
/// resource without a trusted source identifier registry lookup.
pub fn has_identifier_only_patient_reference(resource: &JsonValue) -> bool {
    ["patient", "subject", "context"].iter().any(|field| {
        resource
            .get(*field)
            .map(|value| value.get("identifier").is_some()
                && value.get("reference").and_then(|reference| reference.as_str()).is_none())
            .unwrap_or(false)
    })
}

/// Resolve a FHIR Patient reference to the Patient resource ID in this Bundle.
/// Supports relative/absolute Patient paths and fullUrl references (including
/// urn:uuid). Absolute URL origin validation belongs to the trusted gateway;
/// this zome independently verifies the Patient identity embedded in the path
/// or referenced Bundle entry.
pub fn resolve_fhir_patient_reference(
    reference: &str,
    bundle_entries: &[JsonValue],
) -> Option<String> {
    if let Some(entry) = bundle_entries.iter().find(|entry| {
        entry.get("fullUrl").and_then(|value| value.as_str()) == Some(reference)
    }) {
        let resource = entry.get("resource")?;
        if get_resource_type(resource).as_deref() != Some("Patient") {
            return None;
        }
        return get_resource_id(resource);
    }

    if reference.starts_with("urn:") {
        return None;
    }

    let path = reference.split(['?', '#']).next()?;
    let segments: Vec<&str> = path.split('/').filter(|segment| !segment.is_empty()).collect();
    segments
        .windows(2)
        .rev()
        .find(|pair| pair[0] == "Patient" && !pair[1].is_empty())
        .map(|pair| pair[1].to_string())
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

    #[test]
    fn all_patient_reference_fields_are_returned() {
        let resource: JsonValue = serde_json::json!({
            "resourceType": "Procedure",
            "patient": { "reference": "Patient/123" },
            "subject": { "reference": "https://ehr.example.test/fhir/Patient/123" },
            "context": { "reference": "Encounter/encounter-1" }
        });
        assert_eq!(
            get_patient_references(&resource),
            vec![
                "Patient/123".to_string(),
                "https://ehr.example.test/fhir/Patient/123".to_string(),
            ],
        );
    }

    #[test]
    fn resolve_relative_absolute_and_history_patient_references() {
        let entries: Vec<JsonValue> = vec![];
        assert_eq!(
            resolve_fhir_patient_reference("Patient/123", &entries),
            Some("123".to_string()),
        );
        assert_eq!(
            resolve_fhir_patient_reference(
                "https://ehr.example.test/fhir/Patient/123/_history/4",
                &entries,
            ),
            Some("123".to_string()),
        );
    }

    #[test]
    fn detect_identifier_only_patient_references() {
        let resource: JsonValue = serde_json::json!({
            "resourceType": "Observation",
            "subject": { "identifier": { "system": "urn:test", "value": "123" } }
        });
        assert!(has_identifier_only_patient_reference(&resource));
        let with_reference: JsonValue = serde_json::json!({
            "resourceType": "Observation",
            "subject": {
                "reference": "Patient/123",
                "identifier": { "system": "urn:test", "value": "123" }
            }
        });
        assert!(!has_identifier_only_patient_reference(&with_reference));
    }

    #[test]
    fn resolve_urn_uuid_only_through_a_patient_bundle_entry() {
        let entries: Vec<JsonValue> = vec![
            serde_json::json!({
                "fullUrl": "urn:uuid:patient-123",
                "resource": { "resourceType": "Patient", "id": "123" }
            }),
            serde_json::json!({
                "fullUrl": "urn:uuid:observation-1",
                "resource": { "resourceType": "Observation", "id": "observation-1" }
            }),
        ];
        assert_eq!(
            resolve_fhir_patient_reference("urn:uuid:patient-123", &entries),
            Some("123".to_string()),
        );
        assert_eq!(
            resolve_fhir_patient_reference("urn:uuid:observation-1", &entries),
            None,
        );
        assert_eq!(
            resolve_fhir_patient_reference("urn:uuid:unknown", &entries),
            None,
        );
    }

    #[test]
    fn reject_unknown_resource_reference() {
        let entries: Vec<JsonValue> = vec![];
        assert_eq!(
            resolve_fhir_patient_reference("Device/device-1", &entries),
            None,
        );
    }
}
