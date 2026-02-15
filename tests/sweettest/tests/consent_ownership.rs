//! Sweettest Integration Tests for Consent Ownership
//!
//! Validates that only the patient owner can create consent entries
//! for their patient record.

use anyhow::Result;
use holochain::conductor::config::ConductorConfig;
use holochain::conductor::ConductorBuilder;
use holochain::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================//
// Type Definitions (match zome types)
// ============================================================================//

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BiologicalSex {
    Male,
    Female,
    Intersex,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BloodType {
    APositive,
    ANegative,
    BPositive,
    BNegative,
    ABPositive,
    ABNegative,
    OPositive,
    ONegative,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ContactInfo {
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state_province: Option<String>,
    pub postal_code: Option<String>,
    pub country: String,
    pub phone_primary: Option<String>,
    pub phone_secondary: Option<String>,
    pub email: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EmergencyContact {
    pub name: String,
    pub relationship: String,
    pub phone: String,
    pub email: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AllergySeverity {
    Mild,
    Moderate,
    Severe,
    LifeThreatening,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Allergy {
    pub allergen: String,
    pub reaction: String,
    pub severity: AllergySeverity,
    pub verified: bool,
    pub verified_by: Option<AgentPubKey>,
    pub verified_at: Option<Timestamp>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Patient {
    pub patient_id: String,
    pub mrn: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub biological_sex: BiologicalSex,
    pub gender_identity: Option<String>,
    pub blood_type: Option<BloodType>,
    pub contact: ContactInfo,
    pub emergency_contact: Option<EmergencyContact>,
    pub primary_language: String,
    pub allergies: Vec<Allergy>,
    pub conditions: Vec<String>,
    pub medications: Vec<String>,
    pub mycelix_identity_hash: Option<ActionHash>,
    pub matl_trust_score: f64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DataCategory {
    Demographics,
    Allergies,
    Medications,
    Diagnoses,
    Procedures,
    LabResults,
    ImagingStudies,
    VitalSigns,
    Immunizations,
    MentalHealth,
    SubstanceAbuse,
    SexualHealth,
    GeneticData,
    FinancialData,
    All,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DataPermission {
    Read,
    Write,
    Share,
    Export,
    Delete,
    Amend,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ConsentPurpose {
    Treatment,
    Payment,
    HealthcareOperations,
    Research,
    PublicHealth,
    LegalProceeding,
    Marketing,
    FamilyNotification,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ConsentStatus {
    Active,
    Revoked,
    Expired,
    Pending,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ConsentGrantee {
    Provider(ActionHash),
    Organization(String),
    Agent(AgentPubKey),
    ResearchStudy(ActionHash),
    InsuranceCompany(ActionHash),
    EmergencyAccess,
    Public,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DateRange {
    pub start: Timestamp,
    pub end: Option<Timestamp>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ConsentScope {
    pub data_categories: Vec<DataCategory>,
    pub date_range: Option<DateRange>,
    pub encounter_hashes: Option<Vec<ActionHash>>,
    pub exclusions: Vec<DataCategory>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Consent {
    pub consent_id: String,
    pub patient_hash: ActionHash,
    pub grantee: ConsentGrantee,
    pub scope: ConsentScope,
    pub permissions: Vec<DataPermission>,
    pub purpose: ConsentPurpose,
    pub status: ConsentStatus,
    pub granted_at: Timestamp,
    pub expires_at: Option<Timestamp>,
    pub revoked_at: Option<Timestamp>,
    pub revocation_reason: Option<String>,
    pub document_hash: Option<EntryHash>,
    pub witness: Option<AgentPubKey>,
    pub legal_representative: Option<AgentPubKey>,
    pub notes: Option<String>,
}

// ============================================================================//
// Test Fixtures
// ============================================================================//

fn dna_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../workdir/health.dna")
}

async fn setup_two_agents() -> Result<(holochain::conductor::Conductor, CellId, CellId)> {
    let conductor = ConductorBuilder::new()
        .config(ConductorConfig::default())
        .build()
        .await?;

    let dna_file = DnaFile::from_file_content(&std::fs::read(dna_path())?).await?;
    let dna_hash = conductor.register_dna(dna_file).await?;

    let alice_key = conductor
        .keystore()
        .generate_new_sign_keypair_random()
        .await?;
    let bob_key = conductor
        .keystore()
        .generate_new_sign_keypair_random()
        .await?;

    let alice_cell = conductor
        .install_app(
            "consent-test-alice".to_string(),
            vec![InstalledCell::new(
                CellId::new(dna_hash.clone(), alice_key),
                "health".into(),
            )],
        )
        .await?
        .into_iter()
        .next()
        .unwrap()
        .into_id();

    let bob_cell = conductor
        .install_app(
            "consent-test-bob".to_string(),
            vec![InstalledCell::new(
                CellId::new(dna_hash, bob_key),
                "health".into(),
            )],
        )
        .await?
        .into_iter()
        .next()
        .unwrap()
        .into_id();

    Ok((conductor, alice_cell, bob_cell))
}

fn test_patient() -> Patient {
    Patient {
        patient_id: "PAT-ALICE-001".to_string(),
        mrn: None,
        first_name: "Alice".to_string(),
        last_name: "Owner".to_string(),
        date_of_birth: "1990-01-01".to_string(),
        biological_sex: BiologicalSex::Female,
        gender_identity: None,
        blood_type: Some(BloodType::APositive),
        contact: ContactInfo {
            address_line1: None,
            address_line2: None,
            city: None,
            state_province: None,
            postal_code: None,
            country: "US".to_string(),
            phone_primary: None,
            phone_secondary: None,
            email: Some("alice@example.com".to_string()),
        },
        emergency_contact: Some(EmergencyContact {
            name: "Bob Owner".to_string(),
            relationship: "Spouse".to_string(),
            phone: "+1-555-0101".to_string(),
            email: None,
        }),
        primary_language: "en".to_string(),
        allergies: vec![],
        conditions: vec![],
        medications: vec![],
        mycelix_identity_hash: None,
        matl_trust_score: 0.9,
        created_at: Timestamp::from_micros(0),
        updated_at: Timestamp::from_micros(0),
    }
}

// ============================================================================//
// Test: Consent Ownership Enforcement
// ============================================================================//

#[tokio::test]
#[ignore = "Requires running Holochain conductor"]
async fn test_non_owner_cannot_create_consent() -> Result<()> {
    let (conductor, alice_cell, bob_cell) = setup_two_agents().await?;

    let patient_record: Record = conductor
        .call_zome(&alice_cell, "patient", "create_patient", test_patient())
        .await?;

    let patient_hash = patient_record.action_address().clone();

    let consent = Consent {
        consent_id: "CONSENT-OWNERSHIP-001".to_string(),
        patient_hash,
        grantee: ConsentGrantee::Agent(bob_cell.agent_pubkey().clone()),
        scope: ConsentScope {
            data_categories: vec![DataCategory::Demographics],
            date_range: None,
            encounter_hashes: None,
            exclusions: Vec::new(),
        },
        permissions: vec![DataPermission::Read],
        purpose: ConsentPurpose::Treatment,
        status: ConsentStatus::Active,
        granted_at: Timestamp::from_micros(0),
        expires_at: None,
        revoked_at: None,
        revocation_reason: None,
        document_hash: None,
        witness: None,
        legal_representative: None,
        notes: None,
    };

    let result: Result<Record, _> = conductor
        .call_zome(&bob_cell, "consent", "create_consent", consent)
        .await;

    assert!(result.is_err(), "Non-owner should not be able to create consent");

    Ok(())
}
