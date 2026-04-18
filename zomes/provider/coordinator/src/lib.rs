#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Healthcare Provider Coordinator Zome
//! 
//! Provides extern functions for provider management,
//! credential verification, and patient relationships.

use hdk::prelude::*;
use provider_integrity::*;
use mycelix_health_shared::{require_authorization, log_data_access, DataCategory, Permission};

/// Create a new provider profile
#[hdk_extern]
pub fn create_provider(provider: Provider) -> ExternResult<Record> {
    let provider_hash = create_entry(&EntryTypes::Provider(provider.clone()))?;
    let record = get(provider_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find newly created provider".to_string())))?;
    
    // Link to global providers anchor
    let providers_anchor = anchor_hash("all_providers")?;
    create_link(
        providers_anchor,
        provider_hash.clone(),
        LinkTypes::AllProviders,
        (),
    )?;
    
    // Link by specialty
    let specialty_anchor = anchor_hash(&format!("specialty_{}", provider.specialty))?;
    create_link(
        specialty_anchor,
        provider_hash,
        LinkTypes::ProvidersBySpecialty,
        (),
    )?;
    
    Ok(record)
}

/// Get a provider by their action hash
#[hdk_extern]
pub fn get_provider(provider_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(provider_hash, GetOptions::default())
}

/// Update an existing provider
#[hdk_extern]
pub fn update_provider(input: UpdateProviderInput) -> ExternResult<Record> {
    let caller = agent_info()?.agent_initial_pubkey;
    let original_record = get(input.original_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Provider not found".to_string())))?;
    if original_record.action().author() != &caller {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the provider profile creator can update it".to_string()
        )));
    }

    let updated_hash = update_entry(input.original_hash.clone(), &input.updated_provider)?;
    let record = get(updated_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated provider".to_string())))?;
    
    create_link(
        input.original_hash,
        updated_hash,
        LinkTypes::ProviderUpdates,
        (),
    )?;
    
    Ok(record)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateProviderInput {
    pub original_hash: ActionHash,
    pub updated_provider: Provider,
}

/// Get all providers
#[hdk_extern]
pub fn get_all_providers(_: ()) -> ExternResult<Vec<Record>> {
    let providers_anchor = anchor_hash("all_providers")?;
    let links = get_links(LinkQuery::try_new(providers_anchor, LinkTypes::AllProviders)?, GetStrategy::default())?;
    
    let mut providers = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                providers.push(record);
            }
        }
    }
    
    Ok(providers)
}

/// Search providers by specialty
#[hdk_extern]
pub fn search_providers_by_specialty(specialty: String) -> ExternResult<Vec<Record>> {
    let specialty_anchor = anchor_hash(&format!("specialty_{}", specialty))?;
    let links = get_links(LinkQuery::try_new(specialty_anchor, LinkTypes::ProvidersBySpecialty)?, GetStrategy::default())?;
    
    let mut providers = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                providers.push(record);
            }
        }
    }
    
    Ok(providers)
}

/// Add a license to a provider
#[hdk_extern]
pub fn add_license(license: License) -> ExternResult<Record> {
    let caller = agent_info()?.agent_initial_pubkey;
    let provider_record = get(license.provider_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Provider not found".to_string())))?;
    if provider_record.action().author() != &caller {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the provider profile creator can add licenses".to_string()
        )));
    }

    let license_hash = create_entry(&EntryTypes::License(license.clone()))?;
    let record = get(license_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find license".to_string())))?;
    
    create_link(
        license.provider_hash,
        license_hash,
        LinkTypes::ProviderToLicenses,
        (),
    )?;
    
    Ok(record)
}

/// Get provider's licenses
#[hdk_extern]
pub fn get_provider_licenses(provider_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(provider_hash, LinkTypes::ProviderToLicenses)?, GetStrategy::default())?;
    
    let mut licenses = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                licenses.push(record);
            }
        }
    }
    
    Ok(licenses)
}

/// Add board certification
#[hdk_extern]
pub fn add_board_certification(cert: BoardCertification) -> ExternResult<Record> {
    let caller = agent_info()?.agent_initial_pubkey;
    let provider_record = get(cert.provider_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Provider not found".to_string())))?;
    if provider_record.action().author() != &caller {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the provider profile creator can add certifications".to_string()
        )));
    }

    let cert_hash = create_entry(&EntryTypes::BoardCertification(cert.clone()))?;
    let record = get(cert_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find certification".to_string())))?;
    
    create_link(
        cert.provider_hash,
        cert_hash,
        LinkTypes::ProviderToCertifications,
        (),
    )?;
    
    Ok(record)
}

/// Get provider's certifications
#[hdk_extern]
pub fn get_provider_certifications(provider_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(provider_hash, LinkTypes::ProviderToCertifications)?, GetStrategy::default())?;
    
    let mut certs = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                certs.push(record);
            }
        }
    }
    
    Ok(certs)
}

/// Create provider-patient relationship
#[hdk_extern]
pub fn create_provider_patient_relationship(relationship: ProviderPatientRelationship) -> ExternResult<Record> {
    let caller = agent_info()?.agent_initial_pubkey;
    let provider_record = get(relationship.provider_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Provider not found".to_string())))?;
    let provider_author = provider_record.action().author().clone();

    let patient_record = get(relationship.patient_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Patient not found".to_string())))?;
    let patient_author = patient_record.action().author().clone();

    if caller != provider_author && caller != patient_author {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the patient or provider can create this relationship".to_string()
        )));
    }

    let auth = require_authorization(
        relationship.patient_hash.clone(),
        DataCategory::All,
        Permission::Share,
        false,
    )?;

    let rel_hash = create_entry(&EntryTypes::ProviderPatientRelationship(relationship.clone()))?;
    let record = get(rel_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find relationship".to_string())))?;
    
    // Link provider to patients
    create_link(
        relationship.provider_hash.clone(),
        relationship.patient_hash.clone(),
        LinkTypes::ProviderToPatients,
        (),
    )?;

    log_data_access(
        relationship.patient_hash,
        vec![DataCategory::All],
        Permission::Share,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;
    
    Ok(record)
}

/// Get provider's patients
#[hdk_extern]
pub fn get_provider_patients(provider_hash: ActionHash) -> ExternResult<Vec<ActionHash>> {
    let caller = agent_info()?.agent_initial_pubkey;
    let provider_record = get(provider_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Provider not found".to_string())))?;
    if provider_record.action().author() != &caller {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the provider profile creator can view patients".to_string()
        )));
    }

    let links = get_links(LinkQuery::try_new(provider_hash, LinkTypes::ProviderToPatients)?, GetStrategy::default())?;
    
    Ok(links.into_iter()
        .filter_map(|link| link.target.into_action_hash())
        .collect())
}

/// Verify provider credentials (checks license status, board certs)
#[hdk_extern]
pub fn verify_provider_credentials(provider_hash: ActionHash) -> ExternResult<CredentialVerificationResult> {
    let provider_record = get(provider_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Provider not found".to_string())))?;
    
    let provider: Provider = provider_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid provider entry".to_string())))?;
    
    let licenses = get_provider_licenses(provider_hash.clone())?;
    let certs = get_provider_certifications(provider_hash)?;
    
    // Check for active license
    let has_active_license = licenses.iter().any(|record| {
        if let Some(license) = record.entry().to_app_option::<License>().ok().flatten() {
            matches!(license.status, LicenseStatus::Active)
        } else {
            false
        }
    });
    
    // Check for active board certification
    let has_board_cert = certs.iter().any(|record| {
        if let Some(cert) = record.entry().to_app_option::<BoardCertification>().ok().flatten() {
            matches!(cert.status, CertificationStatus::Active)
        } else {
            false
        }
    });
    
    Ok(CredentialVerificationResult {
        provider_name: format!("{} {} {}", provider.title, provider.first_name, provider.last_name),
        specialty: provider.specialty,
        has_active_license,
        has_board_certification: has_board_cert,
        licenses_count: licenses.len(),
        certifications_count: certs.len(),
        matl_trust_score: provider.matl_trust_score,
        verified_at: sys_time()?,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CredentialVerificationResult {
    pub provider_name: String,
    pub specialty: String,
    pub has_active_license: bool,
    pub has_board_certification: bool,
    pub licenses_count: usize,
    pub certifications_count: usize,
    pub matl_trust_score: f64,
    pub verified_at: Timestamp,
}

/// Get provider by NPI
#[hdk_extern]
pub fn get_provider_by_npi(npi: String) -> ExternResult<Option<Record>> {
    let all_providers = get_all_providers(())?;
    
    for record in all_providers {
        if let Some(provider) = record.entry().to_app_option::<Provider>().ok().flatten() {
            if provider.npi == Some(npi.clone()) {
                return Ok(Some(record));
            }
        }
    }
    
    Ok(None)
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

    fn make_provider(first: &str, last: &str, title: &str, specialty: &str, trust: f64) -> Provider {
        Provider {
            npi: None,
            provider_type: ProviderType::Physician,
            first_name: first.to_string(),
            last_name: last.to_string(),
            title: title.to_string(),
            specialty: specialty.to_string(),
            sub_specialties: vec![],
            organization: None,
            locations: vec![],
            contact: ProviderContact {
                email: "doc@hospital.com".to_string(),
                phone_office: "555-0100".to_string(),
                phone_emergency: None,
                website: None,
            },
            languages: vec!["en".to_string()],
            accepting_patients: true,
            telehealth_enabled: false,
            mycelix_identity_hash: None,
            matl_trust_score: trust,
            epistemic_level: EpistemicLevel::PeerReviewed,
            created_at: Timestamp::from_micros(0),
            updated_at: Timestamp::from_micros(0),
        }
    }

    #[test]
    fn test_provider_construction_and_field_access() {
        let p = make_provider("Jane", "Doe", "MD", "Cardiology", 0.9);
        assert_eq!(p.first_name, "Jane");
        assert_eq!(p.last_name, "Doe");
        assert_eq!(p.title, "MD");
        assert_eq!(p.specialty, "Cardiology");
        assert!((p.matl_trust_score - 0.9).abs() < f64::EPSILON);
        assert!(p.accepting_patients);
        assert!(!p.telehealth_enabled);
    }

    #[test]
    fn test_serde_roundtrip_provider() {
        let p = make_provider("Jane", "Doe", "MD", "Cardiology", 0.9);
        let json = serde_json::to_string(&p).expect("serialize provider");
        let decoded: Provider = serde_json::from_str(&json).expect("deserialize provider");
        assert_eq!(decoded.first_name, "Jane");
        assert_eq!(decoded.specialty, "Cardiology");
        assert_eq!(decoded.provider_type, ProviderType::Physician);
    }

    #[test]
    fn test_serde_roundtrip_credential_verification_result() {
        let result = CredentialVerificationResult {
            provider_name: "Dr. Jane Doe".to_string(),
            specialty: "Cardiology".to_string(),
            has_active_license: true,
            has_board_certification: true,
            licenses_count: 2,
            certifications_count: 1,
            matl_trust_score: 0.95,
            verified_at: Timestamp::from_micros(1000000),
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let decoded: CredentialVerificationResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.provider_name, "Dr. Jane Doe");
        assert!(decoded.has_active_license);
        assert_eq!(decoded.licenses_count, 2);
    }

    #[test]
    fn test_serde_roundtrip_update_provider_input() {
        let input = UpdateProviderInput {
            original_hash: ActionHash::from_raw_36(vec![0u8; 36]),
            updated_provider: make_provider("Bob", "Smith", "DO", "Family Medicine", 0.8),
        };
        let json = serde_json::to_string(&input).expect("serialize");
        let decoded: UpdateProviderInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.updated_provider.first_name, "Bob");
        assert_eq!(decoded.updated_provider.title, "DO");
    }

    #[test]
    fn test_provider_type_variants_serde() {
        let types = vec![
            ProviderType::Physician,
            ProviderType::Nurse,
            ProviderType::NursePractitioner,
            ProviderType::PhysicianAssistant,
            ProviderType::Pharmacist,
            ProviderType::Therapist,
            ProviderType::Dentist,
            ProviderType::Other("Midwife".to_string()),
        ];
        for pt in types {
            let json = serde_json::to_string(&pt).expect("serialize");
            let decoded: ProviderType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, pt);
        }
    }

    #[test]
    fn test_license_status_variants_serde() {
        let statuses = vec![
            LicenseStatus::Active,
            LicenseStatus::Expired,
            LicenseStatus::Suspended,
            LicenseStatus::Revoked,
            LicenseStatus::Pending,
            LicenseStatus::Restricted,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).expect("serialize");
            let decoded: LicenseStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, s);
        }
    }

    #[test]
    fn test_certification_status_variants_serde() {
        let statuses = vec![
            CertificationStatus::Active,
            CertificationStatus::Expired,
            CertificationStatus::Pending,
            CertificationStatus::Revoked,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).expect("serialize");
            let decoded: CertificationStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, s);
        }
    }

    #[test]
    fn test_relationship_type_variants_serde() {
        let types = vec![
            RelationshipType::PrimaryCare,
            RelationshipType::Specialist,
            RelationshipType::Consultant,
            RelationshipType::EmergencyOnly,
            RelationshipType::Research,
            RelationshipType::Other("Telehealth".to_string()),
        ];
        for rt in types {
            let json = serde_json::to_string(&rt).expect("serialize");
            let decoded: RelationshipType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, rt);
        }
    }

    #[test]
    fn test_practice_location_serde() {
        let loc = PracticeLocation {
            name: "Main Office".to_string(),
            address_line1: "123 Medical Dr".to_string(),
            address_line2: Some("Suite 200".to_string()),
            city: "Dallas".to_string(),
            state_province: "TX".to_string(),
            postal_code: "75001".to_string(),
            country: "US".to_string(),
            phone: "555-0100".to_string(),
            fax: Some("555-0101".to_string()),
            hours: Some("M-F 8am-5pm".to_string()),
            is_primary: true,
        };
        let json = serde_json::to_string(&loc).expect("serialize");
        let decoded: PracticeLocation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.name, "Main Office");
        assert!(decoded.is_primary);
    }

    #[test]
    fn test_epistemic_level_serde() {
        let levels = vec![
            EpistemicLevel::Unverified,
            EpistemicLevel::PeerReviewed,
            EpistemicLevel::Replicated,
            EpistemicLevel::Consensus,
        ];
        for el in levels {
            let json = serde_json::to_string(&el).expect("serialize");
            let decoded: EpistemicLevel = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, el);
        }
    }
}
