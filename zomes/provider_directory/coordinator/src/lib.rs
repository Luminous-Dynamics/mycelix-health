//! Provider Directory Coordinator Zome
//!
//! Provides extern functions for provider management including:
//! - Provider registration and profile updates
//! - NPI verification
//! - Provider search and discovery
//!
//! Supports healthcare interoperability and patient-provider matching.

use hdk::prelude::*;
use provider_directory_integrity::*;
use mycelix_health_shared::anchor_hash;

// ============================================================================
// Provider Registration Functions
// ============================================================================

/// Register a new provider profile
#[hdk_extern]
pub fn register_provider(profile: ProviderProfile) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::ProviderProfile(profile.clone()))?;
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find provider profile".to_string())))?;

    // Link from NPI to provider
    let npi_anchor = anchor_hash(&format!("npi_{}", profile.npi))?;
    create_link(npi_anchor, hash.clone(), LinkTypes::NpiToProvider, ())?;

    // Link to all providers anchor
    let all_anchor = anchor_hash("all_providers")?;
    create_link(all_anchor, hash.clone(), LinkTypes::AllProviders, ())?;

    // Link from each specialty to provider
    for specialty in &profile.specialties {
        let specialty_anchor = anchor_hash(&format!("specialty_{}", specialty.taxonomy_code))?;
        create_link(specialty_anchor, hash.clone(), LinkTypes::SpecialtyToProviders, ())?;
    }

    // Link from practice location zip codes to provider
    for location in &profile.practice_locations {
        let location_anchor = anchor_hash(&format!("zip_{}", location.address.postal_code))?;
        create_link(location_anchor, hash.clone(), LinkTypes::LocationToProviders, ())?;
    }

    // Link from accepted insurances to provider
    for insurance in &profile.accepted_insurances {
        let insurance_anchor = anchor_hash(&format!("insurance_{}", insurance.to_lowercase().replace(' ', "_")))?;
        create_link(insurance_anchor, hash.clone(), LinkTypes::InsuranceToProviders, ())?;
    }

    // If telehealth available, link to telehealth anchor
    if profile.telehealth_available {
        let telehealth_anchor = anchor_hash("telehealth_providers")?;
        create_link(telehealth_anchor, hash, LinkTypes::TelehealthProviders, ())?;
    }

    Ok(record)
}

/// Get a provider by their profile hash
#[hdk_extern]
pub fn get_provider(provider_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(provider_hash, GetOptions::default())
}

/// Input for updating provider profile
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateProviderInput {
    pub original_hash: ActionHash,
    pub updated_profile: ProviderProfile,
}

/// Update a provider profile
#[hdk_extern]
pub fn update_provider(input: UpdateProviderInput) -> ExternResult<Record> {
    let mut profile = input.updated_profile;
    profile.updated_at = sys_time()?;

    let updated_hash = update_entry(input.original_hash.clone(), &profile)?;
    let record = get(updated_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated provider".to_string())))?;

    create_link(input.original_hash, updated_hash, LinkTypes::ProviderUpdates, ())?;

    Ok(record)
}

// ============================================================================
// Provider Search Functions
// ============================================================================

/// Search for providers based on criteria
#[hdk_extern]
pub fn search_providers(criteria: ProviderSearchCriteria) -> ExternResult<Vec<Record>> {
    let mut results = Vec::new();
    let mut found_hashes: Vec<ActionHash> = Vec::new();

    // If searching by specialty
    if let Some(ref specialty) = criteria.specialty {
        let specialty_anchor = anchor_hash(&format!("specialty_{}", specialty))?;
        let links = get_links(
            LinkQuery::try_new(specialty_anchor, LinkTypes::SpecialtyToProviders)?, GetStrategy::default())?;
        for link in links {
            if let Some(hash) = link.target.into_action_hash() {
                if !found_hashes.contains(&hash) {
                    found_hashes.push(hash);
                }
            }
        }
    }

    // If searching by location (zip code)
    if let Some(ref location) = criteria.location {
        let location_anchor = anchor_hash(&format!("zip_{}", location))?;
        let links = get_links(
            LinkQuery::try_new(location_anchor, LinkTypes::LocationToProviders)?, GetStrategy::default())?;
        for link in links {
            if let Some(hash) = link.target.into_action_hash() {
                if criteria.specialty.is_some() {
                    // If specialty was also specified, filter to intersection
                    if found_hashes.contains(&hash) {
                        // Already included
                    }
                } else if !found_hashes.contains(&hash) {
                    found_hashes.push(hash);
                }
            }
        }
    }

    // If searching by insurance
    if let Some(ref insurance) = criteria.insurance {
        let insurance_anchor = anchor_hash(&format!("insurance_{}", insurance.to_lowercase().replace(' ', "_")))?;
        let links = get_links(
            LinkQuery::try_new(insurance_anchor, LinkTypes::InsuranceToProviders)?, GetStrategy::default())?;
        for link in links {
            if let Some(hash) = link.target.into_action_hash() {
                if !found_hashes.contains(&hash) {
                    found_hashes.push(hash);
                }
            }
        }
    }

    // If telehealth only
    if criteria.telehealth_only {
        let telehealth_anchor = anchor_hash("telehealth_providers")?;
        let links = get_links(
            LinkQuery::try_new(telehealth_anchor, LinkTypes::TelehealthProviders)?, GetStrategy::default())?;
        let telehealth_hashes: Vec<ActionHash> = links
            .into_iter()
            .filter_map(|l| l.target.into_action_hash())
            .collect();

        if !found_hashes.is_empty() {
            found_hashes.retain(|h| telehealth_hashes.contains(h));
        } else {
            found_hashes = telehealth_hashes;
        }
    }

    // If no specific criteria, get all providers
    if criteria.specialty.is_none()
        && criteria.location.is_none()
        && criteria.insurance.is_none()
        && !criteria.telehealth_only
    {
        let all_anchor = anchor_hash("all_providers")?;
        let links = get_links(
            LinkQuery::try_new(all_anchor, LinkTypes::AllProviders)?, GetStrategy::default())?;
        for link in links {
            if let Some(hash) = link.target.into_action_hash() {
                found_hashes.push(hash);
            }
        }
    }

    // Fetch and filter records
    for hash in found_hashes {
        if let Some(record) = get(hash, GetOptions::default())? {
            if let Some(profile) = record.entry().to_app_option::<ProviderProfile>().ok().flatten() {
                // Apply additional filters
                let mut include = true;

                // Name filter
                if let Some(ref name_query) = criteria.name {
                    let full_name = profile.name.full_name().to_lowercase();
                    if !full_name.contains(&name_query.to_lowercase()) {
                        include = false;
                    }
                }

                // Accepting new patients filter
                if criteria.accepting_new_patients_only && !profile.accepting_new_patients {
                    include = false;
                }

                if include {
                    results.push(record);
                }
            }
        }
    }

    Ok(results)
}

/// Get provider by NPI
#[hdk_extern]
pub fn get_provider_by_npi(npi: String) -> ExternResult<Option<Record>> {
    let npi_anchor = anchor_hash(&format!("npi_{}", npi))?;
    let links = get_links(
        LinkQuery::try_new(npi_anchor, LinkTypes::NpiToProvider)?, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            return get(hash, GetOptions::default());
        }
    }

    Ok(None)
}

// ============================================================================
// NPI Verification Functions
// ============================================================================

/// Verify an NPI number
#[hdk_extern]
pub fn verify_npi(npi: String) -> ExternResult<NpiVerificationResult> {
    // Validate NPI format first
    if npi.len() != 10 || !npi.chars().all(|c| c.is_ascii_digit()) {
        return Ok(NpiVerificationResult {
            npi: npi.clone(),
            is_valid: false,
            provider_name: None,
            provider_type: None,
            status: NpiVerificationStatus::Invalid,
            message: "Invalid NPI format. NPI must be exactly 10 digits.".to_string(),
        });
    }

    // Perform Luhn check digit validation
    if !validate_npi_luhn(&npi) {
        return Ok(NpiVerificationResult {
            npi: npi.clone(),
            is_valid: false,
            provider_name: None,
            provider_type: None,
            status: NpiVerificationStatus::Invalid,
            message: "NPI failed Luhn check digit validation.".to_string(),
        });
    }

    // Check if we have this NPI registered
    let npi_anchor = anchor_hash(&format!("npi_{}", npi))?;
    let links = get_links(
        LinkQuery::try_new(npi_anchor.clone(), LinkTypes::NpiToProvider)?, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(hash.clone(), GetOptions::default())? {
                if let Some(profile) = record.entry().to_app_option::<ProviderProfile>().ok().flatten() {
                    // Create verification record
                    let verification = NpiVerification {
                        npi: npi.clone(),
                        provider_hash: hash,
                        source: "internal_registry".to_string(),
                        status: NpiVerificationStatus::Valid,
                        registry_data: Some(serde_json::to_string(&profile.name).unwrap_or_default()),
                        verified_at: sys_time()?,
                        next_verification_due: None,
                        notes: None,
                    };

                    create_entry(&EntryTypes::NpiVerification(verification))?;

                    return Ok(NpiVerificationResult {
                        npi,
                        is_valid: true,
                        provider_name: Some(profile.name.full_name()),
                        provider_type: profile.specialties.first().map(|s| s.name.clone()),
                        status: NpiVerificationStatus::Valid,
                        message: "NPI verified successfully.".to_string(),
                    });
                }
            }
        }
    }

    // NPI format is valid but not found in our registry
    Ok(NpiVerificationResult {
        npi,
        is_valid: true,
        provider_name: None,
        provider_type: None,
        status: NpiVerificationStatus::NotFound,
        message: "NPI format is valid but provider not found in registry.".to_string(),
    })
}

/// Create an NPI verification record
#[hdk_extern]
pub fn create_npi_verification(verification: NpiVerification) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::NpiVerification(verification.clone()))?;
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find verification".to_string())))?;

    // Link from provider to verification
    create_link(
        verification.provider_hash,
        hash,
        LinkTypes::ProviderToVerifications,
        (),
    )?;

    Ok(record)
}

// ============================================================================
// Provider Affiliation Functions
// ============================================================================

/// Add a provider affiliation
#[hdk_extern]
pub fn add_provider_affiliation(affiliation: ProviderAffiliation) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::ProviderAffiliation(affiliation.clone()))?;
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find affiliation".to_string())))?;

    // Link from provider to affiliation
    create_link(
        affiliation.provider_hash,
        hash,
        LinkTypes::ProviderToAffiliations,
        (),
    )?;

    Ok(record)
}

/// Get affiliations for a provider
#[hdk_extern]
pub fn get_provider_affiliations(provider_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(provider_hash, LinkTypes::ProviderToAffiliations)?, GetStrategy::default())?;

    let mut affiliations = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                affiliations.push(record);
            }
        }
    }

    Ok(affiliations)
}

// ============================================================================
// Telehealth Provider Functions
// ============================================================================

/// Get all telehealth-enabled providers
#[hdk_extern]
pub fn get_telehealth_providers(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash("telehealth_providers")?;
    let links = get_links(
        LinkQuery::try_new(anchor, LinkTypes::TelehealthProviders)?, GetStrategy::default())?;

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

/// Input for searching telehealth providers by state
#[derive(Serialize, Deserialize, Debug)]
pub struct TelehealthByStateInput {
    pub state: String,
}

/// Get telehealth providers licensed in a specific state
#[hdk_extern]
pub fn get_telehealth_providers_by_state(input: TelehealthByStateInput) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash("telehealth_providers")?;
    let links = get_links(
        LinkQuery::try_new(anchor, LinkTypes::TelehealthProviders)?, GetStrategy::default())?;

    let mut providers = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(profile) = record.entry().to_app_option::<ProviderProfile>().ok().flatten() {
                    if let Some(ref capabilities) = profile.telehealth_capabilities {
                        if capabilities.licensed_states.iter().any(|s| s.to_uppercase() == input.state.to_uppercase()) {
                            providers.push(record);
                        }
                    }
                }
            }
        }
    }

    Ok(providers)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate NPI using Luhn algorithm
fn validate_npi_luhn(npi: &str) -> bool {
    // NPI uses Luhn algorithm with prefix "80840" prepended
    let prefixed = format!("80840{}", npi);
    let digits: Vec<u32> = prefixed.chars().filter_map(|c| c.to_digit(10)).collect();

    if digits.len() != 15 {
        return false;
    }

    let mut sum = 0;
    for (i, digit) in digits.iter().rev().enumerate() {
        let mut d = *digit;
        if i % 2 == 1 {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
    }

    sum % 10 == 0
}
