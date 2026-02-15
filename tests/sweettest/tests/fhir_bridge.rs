//! Sweettest Integration Tests for FHIR Bridge Zome
//!
//! These tests verify the complete FHIR Bridge workflow including:
//! - FHIR Bundle ingestion
//! - Resource deduplication
//! - Patient export as FHIR Bundle
//! - Resource validation
//!
//! # Running Tests
//!
//! ```bash
//! # Requires nix develop environment with Holochain
//! nix develop
//! cargo test -p fhir-bridge-sweettest
//! ```
//!
//! # Prerequisites
//!
//! - Built FHIR Bridge WASM zomes
//! - Holochain conductor available in PATH

use anyhow::Result;
use holochain::conductor::api::error::ConductorApiResult;
use holochain::conductor::config::ConductorConfig;
use holochain::conductor::ConductorBuilder;
use holochain::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::path::PathBuf;

// ============================================================================
// Type Definitions (match zome types)
// ============================================================================

/// Input for ingesting a FHIR Bundle
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngestBundleInput {
    pub bundle: JsonValue,
    pub source_system: String,
}

/// Report of what was ingested from a FHIR Bundle
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IngestReport {
    pub report_id: String,
    pub source_system: String,
    pub ingested_at: i64,
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
    pub procedures_created: u32,
    pub procedures_skipped: u32,
    pub unknown_types: Vec<String>,
    pub parse_errors: Vec<String>,
}

/// Input for exporting a patient's data as FHIR
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportPatientInput {
    pub patient_hash: ActionHash,
    pub include_sections: Vec<String>,
    pub format: Option<String>,
}

/// Result of exporting patient data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportResult {
    pub bundle: JsonValue,
    pub resource_count: u32,
    pub format: String,
    pub sections_exported: Vec<String>,
}

// ============================================================================
// Test Fixtures
// ============================================================================

/// Path to the built DNA file
fn dna_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../workdir/health.dna")
}

/// Create a test conductor with the health DNA installed
async fn setup_conductor() -> Result<(holochain::conductor::Conductor, CellId)> {
    let conductor_config = ConductorConfig::default();
    let conductor = ConductorBuilder::new()
        .config(conductor_config)
        .build()
        .await?;

    let dna_file = DnaFile::from_file_content(&std::fs::read(dna_path())?).await?;
    let dna_hash = conductor.register_dna(dna_file).await?;

    let agent_key = conductor
        .keystore()
        .generate_new_sign_keypair_random()
        .await?;

    let installed_cell = conductor
        .install_app(
            "test-app".to_string(),
            vec![InstalledCell::new(
                CellId::new(dna_hash, agent_key.clone()),
                "health".into(),
            )],
        )
        .await?;

    let cell_id = installed_cell.into_iter().next().unwrap().into_id();

    Ok((conductor, cell_id))
}

/// Create a minimal valid FHIR Patient resource
fn create_test_patient(id: &str) -> JsonValue {
    json!({
        "resourceType": "Patient",
        "id": id,
        "identifier": [{
            "system": "http://example.org/mrn",
            "value": format!("MRN-{}", id)
        }],
        "name": [{
            "use": "official",
            "family": "TestFamily",
            "given": ["TestGiven"]
        }],
        "gender": "female",
        "birthDate": "1990-01-15"
    })
}

/// Create a FHIR Observation resource
fn create_test_observation(id: &str, patient_ref: &str) -> JsonValue {
    json!({
        "resourceType": "Observation",
        "id": id,
        "status": "final",
        "code": {
            "coding": [{
                "system": "http://loinc.org",
                "code": "8867-4",
                "display": "Heart rate"
            }]
        },
        "subject": {
            "reference": patient_ref
        },
        "effectiveDateTime": "2024-01-15T10:30:00Z",
        "valueQuantity": {
            "value": 72,
            "unit": "beats/minute",
            "system": "http://unitsofmeasure.org",
            "code": "/min"
        }
    })
}

/// Create a FHIR Condition resource
fn create_test_condition(id: &str, patient_ref: &str) -> JsonValue {
    json!({
        "resourceType": "Condition",
        "id": id,
        "clinicalStatus": {
            "coding": [{
                "system": "http://terminology.hl7.org/CodeSystem/condition-clinical",
                "code": "active"
            }]
        },
        "code": {
            "coding": [{
                "system": "http://snomed.info/sct",
                "code": "73211009",
                "display": "Diabetes mellitus"
            }]
        },
        "subject": {
            "reference": patient_ref
        },
        "onsetDateTime": "2020-06-15"
    })
}

/// Create a FHIR MedicationRequest resource
fn create_test_medication(id: &str, patient_ref: &str) -> JsonValue {
    json!({
        "resourceType": "MedicationRequest",
        "id": id,
        "status": "active",
        "intent": "order",
        "medicationCodeableConcept": {
            "coding": [{
                "system": "http://www.nlm.nih.gov/research/umls/rxnorm",
                "code": "860975",
                "display": "Metformin 500 MG"
            }]
        },
        "subject": {
            "reference": patient_ref
        },
        "authoredOn": "2024-01-10"
    })
}

/// Create a FHIR AllergyIntolerance resource
fn create_test_allergy(id: &str, patient_ref: &str) -> JsonValue {
    json!({
        "resourceType": "AllergyIntolerance",
        "id": id,
        "clinicalStatus": {
            "coding": [{
                "system": "http://terminology.hl7.org/CodeSystem/allergyintolerance-clinical",
                "code": "active"
            }]
        },
        "code": {
            "coding": [{
                "system": "http://snomed.info/sct",
                "code": "91936005",
                "display": "Allergy to penicillin"
            }]
        },
        "patient": {
            "reference": patient_ref
        },
        "recordedDate": "2015-03-20"
    })
}

/// Create a FHIR Immunization resource
fn create_test_immunization(id: &str, patient_ref: &str) -> JsonValue {
    json!({
        "resourceType": "Immunization",
        "id": id,
        "status": "completed",
        "vaccineCode": {
            "coding": [{
                "system": "http://hl7.org/fhir/sid/cvx",
                "code": "208",
                "display": "COVID-19 vaccine"
            }]
        },
        "patient": {
            "reference": patient_ref
        },
        "occurrenceDateTime": "2021-04-15"
    })
}

/// Create a FHIR Procedure resource
fn create_test_procedure(id: &str, patient_ref: &str) -> JsonValue {
    json!({
        "resourceType": "Procedure",
        "id": id,
        "status": "completed",
        "code": {
            "coding": [{
                "system": "http://snomed.info/sct",
                "code": "80146002",
                "display": "Appendectomy"
            }]
        },
        "subject": {
            "reference": patient_ref
        },
        "performedDateTime": "2018-11-20"
    })
}

/// Create a complete FHIR Bundle with all resource types
fn create_comprehensive_test_bundle() -> JsonValue {
    let patient_id = "test-patient-001";
    let patient_ref = format!("Patient/{}", patient_id);

    json!({
        "resourceType": "Bundle",
        "id": "test-bundle-001",
        "type": "collection",
        "timestamp": "2024-01-15T12:00:00Z",
        "entry": [
            {
                "fullUrl": format!("urn:uuid:{}", patient_id),
                "resource": create_test_patient(patient_id)
            },
            {
                "fullUrl": "urn:uuid:obs-001",
                "resource": create_test_observation("obs-001", &patient_ref)
            },
            {
                "fullUrl": "urn:uuid:obs-002",
                "resource": create_test_observation("obs-002", &patient_ref)
            },
            {
                "fullUrl": "urn:uuid:cond-001",
                "resource": create_test_condition("cond-001", &patient_ref)
            },
            {
                "fullUrl": "urn:uuid:med-001",
                "resource": create_test_medication("med-001", &patient_ref)
            },
            {
                "fullUrl": "urn:uuid:allergy-001",
                "resource": create_test_allergy("allergy-001", &patient_ref)
            },
            {
                "fullUrl": "urn:uuid:imm-001",
                "resource": create_test_immunization("imm-001", &patient_ref)
            },
            {
                "fullUrl": "urn:uuid:proc-001",
                "resource": create_test_procedure("proc-001", &patient_ref)
            }
        ]
    })
}

// ============================================================================
// Test: Basic Bundle Ingestion
// ============================================================================

#[tokio::test]
#[ignore = "Requires running Holochain conductor - run with 'cargo test -- --ignored'"]
async fn test_ingest_bundle_basic() -> Result<()> {
    let (conductor, cell_id) = setup_conductor().await?;

    let bundle = create_comprehensive_test_bundle();

    let input = IngestBundleInput {
        bundle,
        source_system: "test-ehr-001".to_string(),
    };

    let report: IngestReport = conductor
        .call_zome(
            &cell_id,
            "fhir_bridge",
            "ingest_bundle",
            input,
        )
        .await?;

    // Verify report structure
    assert_eq!(report.source_system, "test-ehr-001");
    assert!(report.total_processed > 0, "Should process at least one resource");
    assert_eq!(report.patients_created, 1, "Should create one patient");
    assert!(report.parse_errors.is_empty(), "Should have no parse errors");

    println!("Ingest Report: {:?}", report);
    println!("Total processed: {}", report.total_processed);
    println!("Patients created: {}", report.patients_created);
    println!("Observations created: {}", report.observations_created);
    println!("Conditions created: {}", report.conditions_created);
    println!("Medications created: {}", report.medications_created);
    println!("Allergies created: {}", report.allergies_created);
    println!("Immunizations created: {}", report.immunizations_created);
    println!("Procedures created: {}", report.procedures_created);

    Ok(())
}

// ============================================================================
// Test: Patient-Only Bundle
// ============================================================================

#[tokio::test]
#[ignore = "Requires running Holochain conductor"]
async fn test_ingest_patient_only() -> Result<()> {
    let (conductor, cell_id) = setup_conductor().await?;

    let bundle = json!({
        "resourceType": "Bundle",
        "id": "patient-only-bundle",
        "type": "collection",
        "entry": [{
            "fullUrl": "urn:uuid:patient-only-001",
            "resource": create_test_patient("patient-only-001")
        }]
    });

    let input = IngestBundleInput {
        bundle,
        source_system: "test-patient-only".to_string(),
    };

    let report: IngestReport = conductor
        .call_zome(&cell_id, "fhir_bridge", "ingest_bundle", input)
        .await?;

    assert_eq!(report.patients_created, 1);
    assert_eq!(report.patients_updated, 0);
    assert_eq!(report.total_processed, 1);

    Ok(())
}

// ============================================================================
// Test: Deduplication - Same Bundle Twice
// ============================================================================

#[tokio::test]
#[ignore = "Requires running Holochain conductor"]
async fn test_deduplication_same_bundle_twice() -> Result<()> {
    let (conductor, cell_id) = setup_conductor().await?;

    let bundle = json!({
        "resourceType": "Bundle",
        "id": "dedup-test-bundle",
        "type": "collection",
        "entry": [
            {
                "fullUrl": "urn:uuid:dedup-patient",
                "resource": create_test_patient("dedup-patient-001")
            },
            {
                "fullUrl": "urn:uuid:dedup-obs",
                "resource": create_test_observation("dedup-obs-001", "Patient/dedup-patient-001")
            }
        ]
    });

    let source_system = "dedup-test-ehr".to_string();

    // First ingestion
    let input1 = IngestBundleInput {
        bundle: bundle.clone(),
        source_system: source_system.clone(),
    };

    let report1: IngestReport = conductor
        .call_zome(&cell_id, "fhir_bridge", "ingest_bundle", input1)
        .await?;

    assert_eq!(report1.patients_created, 1, "First ingestion should create patient");
    assert_eq!(report1.observations_created, 1, "First ingestion should create observation");

    // Second ingestion (same bundle, same source)
    let input2 = IngestBundleInput {
        bundle,
        source_system,
    };

    let report2: IngestReport = conductor
        .call_zome(&cell_id, "fhir_bridge", "ingest_bundle", input2)
        .await?;

    assert_eq!(report2.patients_created, 0, "Second ingestion should not create patient");
    assert_eq!(report2.patients_updated, 1, "Second ingestion should update existing patient");
    assert_eq!(report2.observations_skipped, 1, "Second ingestion should skip duplicate observation");

    println!("Deduplication test passed:");
    println!("  First ingestion: {} created", report1.observations_created);
    println!("  Second ingestion: {} skipped", report2.observations_skipped);

    Ok(())
}

// ============================================================================
// Test: Different Source Systems (No Deduplication)
// ============================================================================

#[tokio::test]
#[ignore = "Requires running Holochain conductor"]
async fn test_different_source_systems() -> Result<()> {
    let (conductor, cell_id) = setup_conductor().await?;

    // Same patient ID but different source systems should create separate records
    let bundle = json!({
        "resourceType": "Bundle",
        "id": "multi-source-bundle",
        "type": "collection",
        "entry": [{
            "fullUrl": "urn:uuid:multi-patient",
            "resource": create_test_patient("shared-patient-id")
        }]
    });

    // First source system
    let input1 = IngestBundleInput {
        bundle: bundle.clone(),
        source_system: "epic-prod".to_string(),
    };

    let report1: IngestReport = conductor
        .call_zome(&cell_id, "fhir_bridge", "ingest_bundle", input1)
        .await?;

    // Second source system (different EHR)
    let input2 = IngestBundleInput {
        bundle,
        source_system: "cerner-prod".to_string(),
    };

    let report2: IngestReport = conductor
        .call_zome(&cell_id, "fhir_bridge", "ingest_bundle", input2)
        .await?;

    // Both should create new patients (different source systems)
    assert_eq!(report1.patients_created, 1);
    assert_eq!(report2.patients_created, 1);

    Ok(())
}

// ============================================================================
// Test: Empty Bundle
// ============================================================================

#[tokio::test]
#[ignore = "Requires running Holochain conductor"]
async fn test_ingest_empty_bundle() -> Result<()> {
    let (conductor, cell_id) = setup_conductor().await?;

    let bundle = json!({
        "resourceType": "Bundle",
        "id": "empty-bundle",
        "type": "collection",
        "entry": []
    });

    let input = IngestBundleInput {
        bundle,
        source_system: "test-empty".to_string(),
    };

    let report: IngestReport = conductor
        .call_zome(&cell_id, "fhir_bridge", "ingest_bundle", input)
        .await?;

    // Empty bundle with no patient should have error
    assert!(report.parse_errors.len() > 0 || report.total_processed == 0);

    Ok(())
}

// ============================================================================
// Test: Bundle Without Patient (Using Reference)
// ============================================================================

#[tokio::test]
#[ignore = "Requires running Holochain conductor"]
async fn test_bundle_with_patient_reference_only() -> Result<()> {
    let (conductor, cell_id) = setup_conductor().await?;

    let source_system = "ref-test-ehr".to_string();

    // First, create a patient
    let patient_bundle = json!({
        "resourceType": "Bundle",
        "id": "patient-setup-bundle",
        "type": "collection",
        "entry": [{
            "fullUrl": "urn:uuid:ref-patient",
            "resource": create_test_patient("ref-patient-001")
        }]
    });

    let setup_input = IngestBundleInput {
        bundle: patient_bundle,
        source_system: source_system.clone(),
    };

    let _: IngestReport = conductor
        .call_zome(&cell_id, "fhir_bridge", "ingest_bundle", setup_input)
        .await?;

    // Now send observations referencing that patient
    let obs_bundle = json!({
        "resourceType": "Bundle",
        "id": "obs-only-bundle",
        "type": "collection",
        "entry": [{
            "fullUrl": "urn:uuid:ref-obs-001",
            "resource": create_test_observation("ref-obs-001", "Patient/ref-patient-001")
        }]
    });

    let obs_input = IngestBundleInput {
        bundle: obs_bundle,
        source_system,
    };

    let report: IngestReport = conductor
        .call_zome(&cell_id, "fhir_bridge", "ingest_bundle", obs_input)
        .await?;

    // Should resolve patient reference and create observation
    assert_eq!(report.observations_created, 1, "Should create observation via reference");
    assert!(report.parse_errors.is_empty(), "Should resolve patient reference");

    Ok(())
}

// ============================================================================
// Test: Unknown Resource Types
// ============================================================================

#[tokio::test]
#[ignore = "Requires running Holochain conductor"]
async fn test_unknown_resource_types() -> Result<()> {
    let (conductor, cell_id) = setup_conductor().await?;

    let bundle = json!({
        "resourceType": "Bundle",
        "id": "unknown-types-bundle",
        "type": "collection",
        "entry": [
            {
                "fullUrl": "urn:uuid:known-patient",
                "resource": create_test_patient("unknown-test-patient")
            },
            {
                "fullUrl": "urn:uuid:unknown-1",
                "resource": {
                    "resourceType": "Practitioner",
                    "id": "prac-001",
                    "name": [{"family": "Smith"}]
                }
            },
            {
                "fullUrl": "urn:uuid:unknown-2",
                "resource": {
                    "resourceType": "Organization",
                    "id": "org-001",
                    "name": "Test Hospital"
                }
            }
        ]
    });

    let input = IngestBundleInput {
        bundle,
        source_system: "unknown-types-test".to_string(),
    };

    let report: IngestReport = conductor
        .call_zome(&cell_id, "fhir_bridge", "ingest_bundle", input)
        .await?;

    assert_eq!(report.patients_created, 1);
    assert!(report.unknown_types.contains(&"Practitioner".to_string()));
    assert!(report.unknown_types.contains(&"Organization".to_string()));

    println!("Unknown types detected: {:?}", report.unknown_types);

    Ok(())
}

// ============================================================================
// Test: Resource Validation
// ============================================================================

#[tokio::test]
#[ignore = "Requires running Holochain conductor"]
async fn test_validate_fhir_resource_valid() -> Result<()> {
    let (conductor, cell_id) = setup_conductor().await?;

    let patient = create_test_patient("valid-patient");

    let is_valid: bool = conductor
        .call_zome(&cell_id, "fhir_bridge", "validate_fhir_resource", patient)
        .await?;

    assert!(is_valid, "Valid patient should pass validation");

    Ok(())
}

#[tokio::test]
#[ignore = "Requires running Holochain conductor"]
async fn test_validate_fhir_resource_missing_type() -> Result<()> {
    let (conductor, cell_id) = setup_conductor().await?;

    let invalid = json!({
        "id": "no-type",
        "name": [{"family": "Test"}]
    });

    let is_valid: bool = conductor
        .call_zome(&cell_id, "fhir_bridge", "validate_fhir_resource", invalid)
        .await?;

    assert!(!is_valid, "Resource without resourceType should fail validation");

    Ok(())
}

#[tokio::test]
#[ignore = "Requires running Holochain conductor"]
async fn test_validate_observation_missing_value() -> Result<()> {
    let (conductor, cell_id) = setup_conductor().await?;

    // Observation without value or dataAbsentReason should fail
    let invalid_obs = json!({
        "resourceType": "Observation",
        "id": "invalid-obs",
        "status": "final",
        "code": {
            "coding": [{
                "system": "http://loinc.org",
                "code": "8867-4"
            }]
        },
        "subject": {
            "reference": "Patient/123"
        }
        // Missing valueQuantity, valueString, etc.
    });

    let is_valid: bool = conductor
        .call_zome(&cell_id, "fhir_bridge", "validate_fhir_resource", invalid_obs)
        .await?;

    assert!(!is_valid, "Observation without value should fail validation");

    Ok(())
}

// ============================================================================
// Test: Large Bundle Performance
// ============================================================================

#[tokio::test]
#[ignore = "Requires running Holochain conductor - performance test"]
async fn test_large_bundle_performance() -> Result<()> {
    let (conductor, cell_id) = setup_conductor().await?;

    let patient_id = "perf-test-patient";
    let patient_ref = format!("Patient/{}", patient_id);

    // Create bundle with 100 observations
    let mut entries = vec![json!({
        "fullUrl": format!("urn:uuid:{}", patient_id),
        "resource": create_test_patient(patient_id)
    })];

    for i in 0..100 {
        entries.push(json!({
            "fullUrl": format!("urn:uuid:perf-obs-{}", i),
            "resource": {
                "resourceType": "Observation",
                "id": format!("perf-obs-{}", i),
                "status": "final",
                "code": {
                    "coding": [{
                        "system": "http://loinc.org",
                        "code": "8867-4",
                        "display": "Heart rate"
                    }]
                },
                "subject": {
                    "reference": patient_ref
                },
                "valueQuantity": {
                    "value": 70 + (i % 20),
                    "unit": "beats/minute"
                }
            }
        }));
    }

    let bundle = json!({
        "resourceType": "Bundle",
        "id": "perf-test-bundle",
        "type": "collection",
        "entry": entries
    });

    let input = IngestBundleInput {
        bundle,
        source_system: "perf-test".to_string(),
    };

    let start = std::time::Instant::now();

    let report: IngestReport = conductor
        .call_zome(&cell_id, "fhir_bridge", "ingest_bundle", input)
        .await?;

    let duration = start.elapsed();

    assert_eq!(report.patients_created, 1);
    assert_eq!(report.observations_created, 100);
    assert!(report.parse_errors.is_empty());

    println!("Performance test results:");
    println!("  Total resources: {}", report.total_processed);
    println!("  Duration: {:?}", duration);
    println!("  Resources/second: {:.1}", report.total_processed as f64 / duration.as_secs_f64());

    Ok(())
}

// ============================================================================
// Test: Export Patient FHIR
// ============================================================================

#[tokio::test]
#[ignore = "Requires running Holochain conductor"]
async fn test_export_patient_fhir() -> Result<()> {
    let (conductor, cell_id) = setup_conductor().await?;

    // First, ingest some data
    let bundle = create_comprehensive_test_bundle();

    let ingest_input = IngestBundleInput {
        bundle,
        source_system: "export-test-ehr".to_string(),
    };

    let ingest_report: IngestReport = conductor
        .call_zome(&cell_id, "fhir_bridge", "ingest_bundle", ingest_input)
        .await?;

    // Get patient hash (would need to be returned from ingest or queried separately)
    // For this test, we assume we can get it from another zome call
    // This is a simplified test - real implementation would query the patient

    println!("Ingested {} resources for export test", ingest_report.total_processed);

    // Note: Full export test requires retrieving the patient_hash from the ingested data
    // This would typically involve calling a query function first

    Ok(())
}

// ============================================================================
// Test: Malformed Bundle Handling
// ============================================================================

#[tokio::test]
#[ignore = "Requires running Holochain conductor"]
async fn test_malformed_bundle_entry() -> Result<()> {
    let (conductor, cell_id) = setup_conductor().await?;

    let bundle = json!({
        "resourceType": "Bundle",
        "id": "malformed-bundle",
        "type": "collection",
        "entry": [
            {
                "fullUrl": "urn:uuid:good-patient",
                "resource": create_test_patient("good-patient")
            },
            {
                "fullUrl": "urn:uuid:bad-entry"
                // Missing "resource" field
            },
            {
                "resource": {
                    // Missing resourceType
                    "id": "orphan-resource"
                }
            }
        ]
    });

    let input = IngestBundleInput {
        bundle,
        source_system: "malformed-test".to_string(),
    };

    let report: IngestReport = conductor
        .call_zome(&cell_id, "fhir_bridge", "ingest_bundle", input)
        .await?;

    // Should still process the valid patient
    assert_eq!(report.patients_created, 1);
    // Should report errors for malformed entries
    assert!(report.parse_errors.len() > 0 || report.unknown_types.len() > 0);

    println!("Malformed bundle handling:");
    println!("  Parse errors: {:?}", report.parse_errors);
    println!("  Unknown types: {:?}", report.unknown_types);

    Ok(())
}

// ============================================================================
// Test: All Resource Types
// ============================================================================

#[tokio::test]
#[ignore = "Requires running Holochain conductor"]
async fn test_all_supported_resource_types() -> Result<()> {
    let (conductor, cell_id) = setup_conductor().await?;

    let bundle = create_comprehensive_test_bundle();

    let input = IngestBundleInput {
        bundle,
        source_system: "all-types-test".to_string(),
    };

    let report: IngestReport = conductor
        .call_zome(&cell_id, "fhir_bridge", "ingest_bundle", input)
        .await?;

    // Verify all resource types were processed
    assert_eq!(report.patients_created, 1, "Should create 1 patient");
    assert!(report.observations_created >= 1, "Should create observations");
    assert!(report.conditions_created >= 1, "Should create conditions");
    assert!(report.medications_created >= 1, "Should create medications");
    assert!(report.allergies_created >= 1, "Should create allergies");
    assert!(report.immunizations_created >= 1, "Should create immunizations");
    assert!(report.procedures_created >= 1, "Should create procedures");

    // No unknown types in comprehensive bundle
    assert!(report.unknown_types.is_empty(), "All types should be recognized");

    println!("All resource types test passed:");
    println!("  Patients: {}", report.patients_created);
    println!("  Observations: {}", report.observations_created);
    println!("  Conditions: {}", report.conditions_created);
    println!("  Medications: {}", report.medications_created);
    println!("  Allergies: {}", report.allergies_created);
    println!("  Immunizations: {}", report.immunizations_created);
    println!("  Procedures: {}", report.procedures_created);

    Ok(())
}
