// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Sweettest Integration Tests for Consent Ownership
//!
//! Validates that only the patient owner can create consent entries
//! for their patient record, and characterizes the current peer-visible
//! retrieval behavior tracked by the Tier-1 PHI privacy P0.

use anyhow::Result;
use holochain::conductor::config::ConductorConfig;
use holochain::conductor::ConductorBuilder;
use holochain::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

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

// ============================================================================//
// Characterization: Current Consent Confidentiality Boundary
// ============================================================================//

/// Characterizes the current P0 tracked by #157/#164.
///
/// Alice authors a valid MentalHealth consent whose grantee is Alice herself;
/// Bob is neither patient nor grantee. Bob then calls `get_patient_consents` from
/// Bob's own cell. That coordinator function performs `get_links` + `get` without
/// an authorization check. If the full Alice-authored consent arrives in Bob's
/// cell, the canary demonstrates peer-visible policy detail through the currently
/// packaged public-DHT-backed path.
///
/// This is intentionally an ignored *current-behavior* test. After the protected
/// capability migration, the expected behavior should be inverted: Bob must not
/// obtain the human-readable consent/category/note detail.
#[tokio::test]
#[ignore = "P0 characterization: requires packaged health.dna + multi-agent conductor; expected current behavior is sensitive peer retrieval"]
async fn characterize_ungranted_peer_can_read_full_consent_via_public_getter() -> Result<()> {
    const CANARY: &str = "PRIVACY-CANARY: psychotherapy-consent-detail-must-not-be-peer-visible";

    let (conductor, alice_cell, bob_cell) = setup_two_agents().await?;

    let patient_record: Record = conductor
        .call_zome(&alice_cell, "patient", "create_patient", test_patient())
        .await?;
    let patient_hash = patient_record.action_address().clone();

    let consent = Consent {
        consent_id: "CONSENT-PRIVACY-CHAR-001".to_string(),
        patient_hash: patient_hash.clone(),
        // Bob is deliberately NOT the grantee.
        grantee: ConsentGrantee::Agent(alice_cell.agent_pubkey().clone()),
        scope: ConsentScope {
            data_categories: vec![DataCategory::MentalHealth],
            date_range: None,
            encounter_hashes: None,
            exclusions: vec![DataCategory::SubstanceAbuse],
        },
        permissions: vec![DataPermission::Read],
        purpose: ConsentPurpose::Treatment,
        status: ConsentStatus::Active,
        granted_at: Timestamp::from_micros(1),
        expires_at: None,
        revoked_at: None,
        revocation_reason: None,
        document_hash: None,
        witness: None,
        legal_representative: None,
        notes: Some(CANARY.to_string()),
    };

    let _: Record = conductor
        .call_zome(&alice_cell, "consent", "create_consent", consent)
        .await?;

    // Public DHT integration is asynchronous. Retry only the read; do not mutate
    // any policy or grant Bob access while waiting for the authored entry/link to
    // become visible from Bob's cell.
    let mut observed: Option<Consent> = None;
    for _ in 0..50 {
        let records: Vec<Record> = conductor
            .call_zome(
                &bob_cell,
                "consent",
                "get_patient_consents",
                patient_hash.clone(),
            )
            .await?;

        observed = records.into_iter().find_map(|record| {
            record
                .entry()
                .to_app_option::<Consent>()
                .ok()
                .flatten()
                .filter(|entry| entry.notes.as_deref() == Some(CANARY))
        });

        if observed.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let leaked = observed.expect(
        "characterization canary was not visible from Bob's cell; either DHT integration did not complete or current privacy behavior changed",
    );

    assert_eq!(leaked.patient_hash, patient_hash);
    assert_eq!(leaked.scope.data_categories, vec![DataCategory::MentalHealth]);
    assert_eq!(leaked.scope.exclusions, vec![DataCategory::SubstanceAbuse]);
    assert_eq!(leaked.purpose, ConsentPurpose::Treatment);
    assert_eq!(leaked.notes.as_deref(), Some(CANARY));

    // The characterization is meaningful only if Bob is not accidentally the
    // authorized grantee represented by the consent.
    assert_ne!(
        leaked.grantee,
        ConsentGrantee::Agent(bob_cell.agent_pubkey().clone()),
        "test fixture accidentally granted Bob the consent"
    );

    Ok(())
}
