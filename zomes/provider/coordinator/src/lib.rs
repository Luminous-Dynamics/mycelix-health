//! Healthcare Provider Coordinator Zome
//! 
//! Provides extern functions for provider management,
//! credential verification, and patient relationships.

use hdk::prelude::*;
use provider_integrity::*;

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
    
    Ok(record)
}

/// Get provider's patients
#[hdk_extern]
pub fn get_provider_patients(provider_hash: ActionHash) -> ExternResult<Vec<ActionHash>> {
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
