#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Provider Directory Integrity Zome
//!
//! Defines entry types for healthcare provider profiles including:
//! - Provider credentials and NPI registry
//! - Practice locations and contact information
//! - Specialties and services offered
//! - Telehealth capabilities
//!
//! Supports provider verification and discovery.

use hdi::prelude::*;

// ============================================================================
// Provider Profile Types
// ============================================================================

/// Healthcare provider profile
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ProviderProfile {
    /// National Provider Identifier (NPI)
    pub npi: String,
    /// Provider name
    pub name: PersonName,
    /// Professional credentials (MD, DO, NP, PA, etc.)
    pub credentials: Vec<String>,
    /// License information
    pub licenses: Vec<License>,
    /// Medical specialties
    pub specialties: Vec<Specialty>,
    /// Board certifications
    pub board_certifications: Vec<BoardCertification>,
    /// Practice locations
    pub practice_locations: Vec<PracticeLocation>,
    /// Contact information
    pub contact: ProviderContact,
    /// Languages spoken
    pub languages: Vec<String>,
    /// Whether provider offers telehealth
    pub telehealth_available: bool,
    /// Telehealth capabilities
    pub telehealth_capabilities: Option<TelehealthCapabilities>,
    /// Insurance networks accepted
    pub accepted_insurances: Vec<String>,
    /// Provider bio/about text
    pub bio: Option<String>,
    /// Profile photo hash (if stored in DHT)
    pub photo_hash: Option<EntryHash>,
    /// Accepting new patients
    pub accepting_new_patients: bool,
    /// Mycelix identity link (for verified providers)
    pub mycelix_identity_hash: Option<ActionHash>,
    /// Verification status
    pub verification_status: VerificationStatus,
    /// MATL trust score
    pub matl_trust_score: f64,
    /// Profile created at
    pub created_at: Timestamp,
    /// Profile last updated
    pub updated_at: Timestamp,
}

/// Person name structure
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PersonName {
    pub prefix: Option<String>,
    pub first_name: String,
    pub middle_name: Option<String>,
    pub last_name: String,
    pub suffix: Option<String>,
    pub display_name: Option<String>,
}

impl PersonName {
    pub fn full_name(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ref prefix) = self.prefix {
            parts.push(prefix.clone());
        }
        parts.push(self.first_name.clone());
        if let Some(ref middle) = self.middle_name {
            parts.push(middle.clone());
        }
        parts.push(self.last_name.clone());
        if let Some(ref suffix) = self.suffix {
            parts.push(suffix.clone());
        }
        parts.join(" ")
    }
}

/// Medical license information
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct License {
    /// License type (Medical, Nursing, Pharmacy, etc.)
    pub license_type: String,
    /// License number
    pub license_number: String,
    /// Issuing state/jurisdiction
    pub state: String,
    /// Expiration date (YYYY-MM-DD)
    pub expiration_date: String,
    /// Active status
    pub is_active: bool,
}

/// Medical specialty
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Specialty {
    /// Specialty name
    pub name: String,
    /// NUCC taxonomy code
    pub taxonomy_code: String,
    /// Is this the primary specialty
    pub is_primary: bool,
}

/// Board certification
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BoardCertification {
    /// Certifying board name
    pub board_name: String,
    /// Specialty certified in
    pub specialty: String,
    /// Initial certification date
    pub initial_date: String,
    /// Recertification date (if applicable)
    pub recertification_date: Option<String>,
    /// Expiration date
    pub expiration_date: Option<String>,
    /// Is current/active
    pub is_current: bool,
}

/// Practice location
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PracticeLocation {
    /// Location name (e.g., "Main Office", "Hospital")
    pub name: String,
    /// Location type (office, hospital, clinic, etc.)
    pub location_type: String,
    /// Full address
    pub address: Address,
    /// Phone number
    pub phone: String,
    /// Fax number
    pub fax: Option<String>,
    /// Office hours
    pub hours: Vec<OfficeHours>,
    /// Is primary location
    pub is_primary: bool,
    /// Accepts walk-ins
    pub accepts_walkins: bool,
    /// Parking available
    pub parking_available: bool,
    /// Wheelchair accessible
    pub wheelchair_accessible: bool,
}

/// Address structure
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Address {
    pub street_line1: String,
    pub street_line2: Option<String>,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Office hours
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OfficeHours {
    /// Day of week (Monday, Tuesday, etc.)
    pub day: String,
    /// Opening time (HH:MM in 24-hour format)
    pub open_time: String,
    /// Closing time (HH:MM in 24-hour format)
    pub close_time: String,
    /// Is closed this day
    pub is_closed: bool,
}

/// Provider contact information
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProviderContact {
    pub email: Option<String>,
    pub website: Option<String>,
    pub scheduling_url: Option<String>,
    pub patient_portal_url: Option<String>,
}

/// Telehealth capabilities
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TelehealthCapabilities {
    /// Supports video visits
    pub video_visits: bool,
    /// Supports phone visits
    pub phone_visits: bool,
    /// Supports async messaging
    pub async_messaging: bool,
    /// Platforms supported (e.g., "Zoom", "Doxy.me")
    pub platforms: Vec<String>,
    /// States where telehealth is offered
    pub licensed_states: Vec<String>,
}

/// Provider verification status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum VerificationStatus {
    Unverified,
    Pending,
    Verified,
    Suspended,
    Revoked,
}

// ============================================================================
// NPI Verification
// ============================================================================

/// NPI verification record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct NpiVerification {
    /// NPI being verified
    pub npi: String,
    /// Provider hash
    pub provider_hash: ActionHash,
    /// Verification source (e.g., "NPPES")
    pub source: String,
    /// Verification status
    pub status: NpiVerificationStatus,
    /// Data from NPI registry
    pub registry_data: Option<String>,
    /// Verification timestamp
    pub verified_at: Timestamp,
    /// Next verification due
    pub next_verification_due: Option<Timestamp>,
    /// Verification notes
    pub notes: Option<String>,
}

/// NPI verification status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum NpiVerificationStatus {
    Valid,
    Invalid,
    Deactivated,
    NotFound,
    Pending,
    Error,
}

/// NPI verification result (returned to caller)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NpiVerificationResult {
    pub npi: String,
    pub is_valid: bool,
    pub provider_name: Option<String>,
    pub provider_type: Option<String>,
    pub status: NpiVerificationStatus,
    pub message: String,
}

// ============================================================================
// Provider Search Types
// ============================================================================

/// Search criteria for finding providers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderSearchCriteria {
    /// Search by name (partial match)
    pub name: Option<String>,
    /// Search by specialty
    pub specialty: Option<String>,
    /// Search by location (city, state, or zip)
    pub location: Option<String>,
    /// Search by insurance accepted
    pub insurance: Option<String>,
    /// Filter to telehealth providers only
    pub telehealth_only: bool,
    /// Filter to accepting new patients only
    pub accepting_new_patients_only: bool,
    /// Maximum distance in miles (requires lat/long)
    pub max_distance_miles: Option<f64>,
    /// Caller's latitude for distance search
    pub latitude: Option<f64>,
    /// Caller's longitude for distance search
    pub longitude: Option<f64>,
}

// ============================================================================
// Provider Affiliation
// ============================================================================

/// Provider organization affiliation
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ProviderAffiliation {
    /// Provider hash
    pub provider_hash: ActionHash,
    /// Organization name
    pub organization_name: String,
    /// Organization NPI (if applicable)
    pub organization_npi: Option<String>,
    /// Role/position at organization
    pub role: String,
    /// Department
    pub department: Option<String>,
    /// Start date
    pub start_date: String,
    /// End date (if no longer affiliated)
    pub end_date: Option<String>,
    /// Is current affiliation
    pub is_current: bool,
}

// ============================================================================
// Entry and Link Type Enums
// ============================================================================

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    ProviderProfile(ProviderProfile),
    NpiVerification(NpiVerification),
    ProviderAffiliation(ProviderAffiliation),
}

#[hdk_link_types]
pub enum LinkTypes {
    /// NPI to provider profile
    NpiToProvider,
    /// Provider to their verifications
    ProviderToVerifications,
    /// Provider to their affiliations
    ProviderToAffiliations,
    /// Specialty anchor to providers
    SpecialtyToProviders,
    /// Location anchor to providers (by zip code)
    LocationToProviders,
    /// Insurance anchor to providers
    InsuranceToProviders,
    /// Telehealth providers anchor
    TelehealthProviders,
    /// All providers anchor
    AllProviders,
    /// Provider updates
    ProviderUpdates,
}

// ============================================================================
// Validation Functions
// ============================================================================

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => validate_create_entry(app_entry),
            OpEntry::UpdateEntry {
                app_entry, action, ..
            } => validate_update_entry(action, app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterCreateLink { link_type, .. } => validate_link(link_type),
        // Previously left fully permissive via the catch-all `_` arm --
        // the 18th confirmed instance of the wide-open RegisterUpdate/
        // RegisterDelete bug this pass. Found + fixed 2026-07-09 during
        // the P0 author-binding pass. This coordinator never calls
        // delete_link/delete_entry (confirmed via grep), so
        // RegisterDeleteLink/RegisterDelete hardening below is pure
        // defense-in-depth.
        FlatOp::RegisterDeleteLink {
            original_action,
            action,
            ..
        } => {
            if action.author != original_action.author {
                return Ok(ValidateCallbackResult::Invalid(
                    "Only the original link creator can delete a link".into(),
                ));
            }
            Ok(ValidateCallbackResult::Valid)
        }
        FlatOp::RegisterUpdate(op_update) => match op_update {
            OpUpdate::Entry { app_entry, action } => validate_update_entry(action, app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDelete(OpDelete { action }) => {
            let original = must_get_action(action.deletes_address.clone())?;
            if action.author != *original.action().author() {
                return Ok(ValidateCallbackResult::Invalid(
                    "Only the original entry author can delete an entry".into(),
                ));
            }
            Ok(ValidateCallbackResult::Valid)
        }
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

/// None of this zome's entry types have a self-declared agent-identity
/// field (ProviderProfile describes the provider themselves but has no
/// "owner" AgentPubKey field at all; mycelix_identity_hash/provider_hash
/// are ActionHash references to other records). Reviewed 2026-07-09
/// during the P0 author-binding pass; case (a) across the board.
fn validate_create_entry(entry: EntryTypes) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::ProviderProfile(profile) => validate_provider_profile(&profile),
        EntryTypes::NpiVerification(verification) => validate_npi_verification(&verification),
        EntryTypes::ProviderAffiliation(affiliation) => validate_provider_affiliation(&affiliation),
    }
}

/// Only `ProviderProfile` has a live coordinator update path
/// (update_provider). Reviewed 2026-07-09 during the P0 author-binding
/// pass: this IS a real authorization gap, just not a per-field
/// author-binding one -- the coordinator's own check
/// (`original_record.action().author() != &caller`) uses the REAL DHT
/// author of the original create action (there being no self-declared
/// owner field to forge), but only client-side. A modified coordinator
/// could previously rewrite ANY other provider's profile. Fixed by
/// comparing the committing agent to the ORIGINAL create action's real
/// author via must_get_action -- the same "universal same-author-update"
/// idiom used in several mycelix-identity zomes (did_registry,
/// credential_schema, revocation). NpiVerification/ProviderAffiliation
/// have no live update call at all (confirmed via grep) and are made
/// immutable.
fn validate_update_entry(
    action: Update,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::ProviderProfile(profile) => {
            let original = must_get_action(action.original_action_address.clone())?;
            if *original.action().author() != action.author {
                return Ok(ValidateCallbackResult::Invalid(
                    "Only the original provider profile creator can update it".into(),
                ));
            }
            validate_provider_profile(&profile)
        }
        EntryTypes::NpiVerification(_) => Ok(ValidateCallbackResult::Invalid(
            "NPI verification records are immutable; create a new one".into(),
        )),
        EntryTypes::ProviderAffiliation(_) => Ok(ValidateCallbackResult::Invalid(
            "Provider affiliations are immutable; create a new one".into(),
        )),
    }
}

fn validate_provider_profile(profile: &ProviderProfile) -> ExternResult<ValidateCallbackResult> {
    // Validate NPI format (10 digits)
    if !validate_npi_format(&profile.npi) {
        return Ok(ValidateCallbackResult::Invalid(
            "Invalid NPI format. NPI must be 10 digits".to_string(),
        ));
    }

    // Validate name
    if profile.name.first_name.is_empty() || profile.name.last_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Provider first and last name are required".to_string(),
        ));
    }

    // Validate at least one credential
    if profile.credentials.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one credential is required".to_string(),
        ));
    }

    // Validate MATL trust score
    if profile.matl_trust_score < 0.0 || profile.matl_trust_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "MATL trust score must be between 0.0 and 1.0".to_string(),
        ));
    }

    // Validate at least one practice location
    if profile.practice_locations.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one practice location is required".to_string(),
        ));
    }

    // If telehealth is offered, validate capabilities are provided
    if profile.telehealth_available && profile.telehealth_capabilities.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "Telehealth capabilities must be specified when telehealth is available".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_npi_verification(
    verification: &NpiVerification,
) -> ExternResult<ValidateCallbackResult> {
    // Validate NPI format
    if !validate_npi_format(&verification.npi) {
        return Ok(ValidateCallbackResult::Invalid(
            "Invalid NPI format".to_string(),
        ));
    }

    // Validate source is provided
    if verification.source.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Verification source is required".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_provider_affiliation(
    affiliation: &ProviderAffiliation,
) -> ExternResult<ValidateCallbackResult> {
    // Validate organization name
    if affiliation.organization_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Organization name is required".to_string(),
        ));
    }

    // Validate role
    if affiliation.role.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Role is required".to_string(),
        ));
    }

    // Validate start date format
    if !is_valid_date_format(&affiliation.start_date) {
        return Ok(ValidateCallbackResult::Invalid(
            "Invalid start date format. Use YYYY-MM-DD".to_string(),
        ));
    }

    // Validate end date format if provided
    if let Some(ref end_date) = affiliation.end_date {
        if !is_valid_date_format(end_date) {
            return Ok(ValidateCallbackResult::Invalid(
                "Invalid end date format. Use YYYY-MM-DD".to_string(),
            ));
        }
    }

    // If end date is provided, is_current should be false
    if affiliation.end_date.is_some() && affiliation.is_current {
        return Ok(ValidateCallbackResult::Invalid(
            "Cannot be current affiliation with an end date".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_link(link_type: LinkTypes) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        LinkTypes::NpiToProvider => Ok(ValidateCallbackResult::Valid),
        LinkTypes::ProviderToVerifications => Ok(ValidateCallbackResult::Valid),
        LinkTypes::ProviderToAffiliations => Ok(ValidateCallbackResult::Valid),
        LinkTypes::SpecialtyToProviders => Ok(ValidateCallbackResult::Valid),
        LinkTypes::LocationToProviders => Ok(ValidateCallbackResult::Valid),
        LinkTypes::InsuranceToProviders => Ok(ValidateCallbackResult::Valid),
        LinkTypes::TelehealthProviders => Ok(ValidateCallbackResult::Valid),
        LinkTypes::AllProviders => Ok(ValidateCallbackResult::Valid),
        LinkTypes::ProviderUpdates => Ok(ValidateCallbackResult::Valid),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate NPI format (10 digits with Luhn check digit)
fn validate_npi_format(npi: &str) -> bool {
    if npi.len() != 10 {
        return false;
    }
    if !npi.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    // NPI uses the Luhn algorithm with prefix "80840" for validation
    // For simplicity, we just check length and digits here
    // Full Luhn validation would be implemented in coordinator
    true
}

/// Validate date format (YYYY-MM-DD)
fn is_valid_date_format(date: &str) -> bool {
    if date.len() != 10 {
        return false;
    }
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod content_integrity_tests {
    use super::*;

    fn update_action(author: AgentPubKey) -> Update {
        Update {
            author,
            timestamp: Timestamp::from_micros(1),
            action_seq: 1,
            prev_action: ActionHash::from_raw_36(vec![0u8; 36]),
            original_action_address: ActionHash::from_raw_36(vec![9u8; 36]),
            original_entry_address: EntryHash::from_raw_36(vec![0u8; 36]),
            entry_type: EntryType::App(AppEntryDef::new(
                EntryDefIndex::from(0),
                0.into(),
                EntryVisibility::Public,
            )),
            entry_hash: EntryHash::from_raw_36(vec![0u8; 36]),
            weight: Default::default(),
        }
    }

    fn me() -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![0u8; 36])
    }

    #[test]
    fn update_entry_rejects_npi_verification_update() {
        // No live update_entry call exists for NpiVerification --
        // dead-path immutability, testable without must_get_action.
        let v = NpiVerification {
            npi: "1234567890".into(),
            provider_hash: ActionHash::from_raw_36(vec![2u8; 36]),
            source: "NPPES".into(),
            status: NpiVerificationStatus::Valid,
            registry_data: None,
            verified_at: Timestamp::from_micros(0),
            next_verification_due: None,
            notes: None,
        };
        let result =
            validate_update_entry(update_action(me()), EntryTypes::NpiVerification(v)).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn update_entry_rejects_provider_affiliation_update() {
        let a = ProviderAffiliation {
            provider_hash: ActionHash::from_raw_36(vec![2u8; 36]),
            organization_name: "General Hospital".into(),
            organization_npi: None,
            role: "Attending".into(),
            department: None,
            start_date: "2020-01-01".into(),
            end_date: None,
            is_current: true,
        };
        let result =
            validate_update_entry(update_action(me()), EntryTypes::ProviderAffiliation(a)).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }
}
