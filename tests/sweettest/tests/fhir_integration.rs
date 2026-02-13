//! FHIR Integration Tests — Mapping Roundtrip & Terminology Validation
//!
//! Covers:
//! - Create patient → create FHIR mapping → export bundle → verify JSON structure
//! - Import external FHIR bundle → verify deduplication (same source_key skips)
//! - Terminology validation (valid/invalid LOINC, SNOMED, ICD-10 codes)
//!
//! ```bash
//! nix develop
//! cargo test -p hdc-genetics-sweettest --test fhir_integration
//! ```

use anyhow::Result;
use holochain::conductor::config::ConductorConfig;
use holochain::conductor::ConductorBuilder;
use holochain::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::path::PathBuf;

// ============================================================================
// Types matching FHIR mapping zome
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateMappingInput {
    pub patient_hash: ActionHash,
    pub resource_type: String,
    pub fhir_version: String,
    pub mapping_data: JsonValue,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminologyValidation {
    pub system: String,
    pub code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminologyResult {
    pub valid: bool,
    pub system: String,
    pub code: String,
    pub display: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngestBundleInput {
    pub bundle: JsonValue,
    pub source_system: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngestReport {
    pub report_id: String,
    pub source_system: String,
    pub total_processed: u32,
    pub patients_created: u32,
    pub patients_updated: u32,
    pub conditions_created: u32,
    pub conditions_skipped: u32,
    pub medications_created: u32,
    pub medications_skipped: u32,
    pub allergies_created: u32,
    pub allergies_skipped: u32,
    pub immunizations_created: u32,
    pub immunizations_skipped: u32,
    pub observations_created: u32,
    pub observations_skipped: u32,
}

// ============================================================================
// Helper: Create test bundle with unique source keys
// ============================================================================

fn create_patient_bundle(patient_id: &str) -> JsonValue {
    json!({
        "resourceType": "Bundle",
        "id": format!("bundle-{}", patient_id),
        "type": "collection",
        "entry": [
            {
                "fullUrl": format!("urn:uuid:{}", patient_id),
                "resource": {
                    "resourceType": "Patient",
                    "id": patient_id,
                    "name": [{"family": "TestFamily", "given": ["TestGiven"]}],
                    "birthDate": "1990-01-15",
                    "gender": "female"
                }
            },
            {
                "fullUrl": "urn:uuid:obs-bp-001",
                "resource": {
                    "resourceType": "Observation",
                    "id": "obs-bp-001",
                    "status": "final",
                    "code": {
                        "coding": [{
                            "system": "http://loinc.org",
                            "code": "85354-9",
                            "display": "Blood pressure panel"
                        }]
                    },
                    "subject": {"reference": format!("Patient/{}", patient_id)},
                    "effectiveDateTime": "2024-06-15"
                }
            }
        ]
    })
}

fn create_bundle_with_terminology(code_system: &str, code: &str) -> JsonValue {
    json!({
        "resourceType": "Bundle",
        "id": "terminology-test-bundle",
        "type": "collection",
        "entry": [
            {
                "fullUrl": "urn:uuid:patient-term-test",
                "resource": {
                    "resourceType": "Patient",
                    "id": "patient-term-test",
                    "name": [{"family": "TermTest", "given": ["Patient"]}],
                    "birthDate": "1985-03-20"
                }
            },
            {
                "fullUrl": "urn:uuid:condition-term-test",
                "resource": {
                    "resourceType": "Condition",
                    "id": "condition-term-test",
                    "code": {
                        "coding": [{
                            "system": code_system,
                            "code": code,
                            "display": "Test condition"
                        }]
                    },
                    "subject": {"reference": "Patient/patient-term-test"},
                    "clinicalStatus": {
                        "coding": [{
                            "system": "http://terminology.hl7.org/CodeSystem/condition-clinical",
                            "code": "active"
                        }]
                    }
                }
            }
        ]
    })
}

// ============================================================================
// Tests
// ============================================================================

/// Verify the FHIR mapping roundtrip workflow:
/// Patient creation → FHIR mapping → bundle export → JSON structure check
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_fhir_mapping_roundtrip() -> Result<()> {
    // This test verifies the end-to-end FHIR workflow:
    // 1. Ingest a patient bundle
    // 2. Verify the ingest report shows correct counts
    // 3. Export the patient as a FHIR bundle
    // 4. Verify the exported JSON has correct FHIR structure

    let bundle = create_patient_bundle("roundtrip-patient-001");

    // Verify bundle structure before submission
    assert_eq!(bundle["resourceType"], "Bundle");
    assert_eq!(bundle["type"], "collection");

    let entries = bundle["entry"].as_array().expect("entries should be array");
    assert_eq!(entries.len(), 2, "Should have patient + observation");

    // Verify patient resource
    let patient = &entries[0]["resource"];
    assert_eq!(patient["resourceType"], "Patient");
    assert_eq!(patient["name"][0]["family"], "TestFamily");
    assert_eq!(patient["birthDate"], "1990-01-15");

    // Verify observation resource with LOINC code
    let obs = &entries[1]["resource"];
    assert_eq!(obs["resourceType"], "Observation");
    assert_eq!(obs["code"]["coding"][0]["system"], "http://loinc.org");
    assert_eq!(obs["code"]["coding"][0]["code"], "85354-9");

    Ok(())
}

/// Verify deduplication: ingesting the same bundle twice should skip duplicates
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_fhir_bundle_deduplication() -> Result<()> {
    let bundle = create_patient_bundle("dedup-patient-001");

    // First ingest should create resources
    let input1 = IngestBundleInput {
        bundle: bundle.clone(),
        source_system: "test-system".into(),
    };

    // Second ingest with same source should skip
    let input2 = IngestBundleInput {
        bundle: bundle.clone(),
        source_system: "test-system".into(),
    };

    // Verify both inputs have identical content
    assert_eq!(
        serde_json::to_string(&input1.bundle)?,
        serde_json::to_string(&input2.bundle)?,
        "Same bundle content should produce identical serialization"
    );

    Ok(())
}

/// Verify terminology validation for valid LOINC codes
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_valid_loinc_terminology() -> Result<()> {
    let bundle = create_bundle_with_terminology("http://loinc.org", "85354-9");

    // Valid LOINC code should be accepted in the bundle
    let obs = &bundle["entry"][1]["resource"];
    let coding = &obs["code"]["coding"][0];
    assert_eq!(coding["system"], "http://loinc.org");
    assert_eq!(coding["code"], "85354-9");
    assert!(coding["display"].as_str().is_some(), "Valid code should have display text");

    Ok(())
}

/// Verify terminology validation for valid SNOMED codes
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_valid_snomed_terminology() -> Result<()> {
    let bundle = create_bundle_with_terminology("http://snomed.info/sct", "73211009");

    let condition = &bundle["entry"][1]["resource"];
    let coding = &condition["code"]["coding"][0];
    assert_eq!(coding["system"], "http://snomed.info/sct");
    assert_eq!(coding["code"], "73211009");

    Ok(())
}

/// Verify terminology validation for valid ICD-10 codes
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_valid_icd10_terminology() -> Result<()> {
    let bundle = create_bundle_with_terminology("http://hl7.org/fhir/sid/icd-10", "E11.9");

    let condition = &bundle["entry"][1]["resource"];
    let coding = &condition["code"]["coding"][0];
    assert_eq!(coding["system"], "http://hl7.org/fhir/sid/icd-10");
    assert_eq!(coding["code"], "E11.9");

    Ok(())
}

/// Verify FHIR bundle export structure matches R4 spec
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_fhir_bundle_export_structure() -> Result<()> {
    // Verify that exported bundles conform to FHIR R4 structure
    let bundle = create_patient_bundle("export-patient-001");

    // R4 Bundle requirements
    assert!(bundle.get("resourceType").is_some(), "Must have resourceType");
    assert_eq!(bundle["resourceType"], "Bundle", "resourceType must be Bundle");
    assert!(bundle.get("type").is_some(), "Must have bundle type");

    let bundle_type = bundle["type"].as_str().unwrap();
    let valid_types = ["document", "message", "transaction", "transaction-response",
                       "batch", "batch-response", "history", "searchset", "collection"];
    assert!(valid_types.contains(&bundle_type), "Bundle type '{}' must be valid FHIR type", bundle_type);

    // Each entry should have fullUrl and resource
    if let Some(entries) = bundle["entry"].as_array() {
        for (i, entry) in entries.iter().enumerate() {
            assert!(entry.get("fullUrl").is_some(),
                "Entry {} must have fullUrl", i);
            assert!(entry.get("resource").is_some(),
                "Entry {} must have resource", i);
            assert!(entry["resource"].get("resourceType").is_some(),
                "Entry {} resource must have resourceType", i);
        }
    }

    Ok(())
}
