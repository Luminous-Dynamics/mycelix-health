#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Healthcare Provider Credentials and Licensing Integrity Zome
//!
//! Defines provider profiles and legacy provider-owned license/certification
//! records. This zome enforces ownership and cross-entry attachment authority at
//! DHT validation so a modified coordinator cannot bypass those checks.
//!
//! IMPORTANT: a provider-authored `License`/`BoardCertification` record is not
//! proof of external licensure. High-assurance clinical authority must use the
//! separate issuer-trust + clinical-authority stack.

use hdi::prelude::*;

/// Healthcare provider profile
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Provider {
    /// National Provider Identifier (NPI) - US standard
    pub npi: Option<String>,
    /// Provider type
    pub provider_type: ProviderType,
    /// First name
    pub first_name: String,
    /// Last name
    pub last_name: String,
    /// Professional title (MD, DO, RN, etc.)
    pub title: String,
    /// Primary specialty
    pub specialty: String,
    /// Sub-specialties
    pub sub_specialties: Vec<String>,
    /// Practice/organization name
    pub organization: Option<String>,
    /// Practice locations
    pub locations: Vec<PracticeLocation>,
    /// Contact information
    pub contact: ProviderContact,
    /// Languages spoken
    pub languages: Vec<String>,
    /// Accepting new patients
    pub accepting_patients: bool,
    /// Telehealth capable
    pub telehealth_enabled: bool,
    /// Legacy Mycelix identity link. This is not an authenticated-principal proof.
    pub mycelix_identity_hash: Option<ActionHash>,
    /// MATL trust score based on patient outcomes and feedback
    pub matl_trust_score: f64,
    /// Epistemic classification for claims made
    pub epistemic_level: EpistemicLevel,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ProviderType {
    Physician,
    Nurse,
    NursePractitioner,
    PhysicianAssistant,
    Pharmacist,
    Therapist,
    Dentist,
    Optometrist,
    Chiropractor,
    Researcher,
    LabTechnician,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EpistemicLevel {
    /// E0: Unverified claims
    Unverified,
    /// E1: Peer-reviewed but not replicated
    PeerReviewed,
    /// E2: Replicated findings
    Replicated,
    /// E3: Established medical consensus
    Consensus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PracticeLocation {
    pub name: String,
    pub address_line1: String,
    pub address_line2: Option<String>,
    pub city: String,
    pub state_province: String,
    pub postal_code: String,
    pub country: String,
    pub phone: String,
    pub fax: Option<String>,
    pub hours: Option<String>,
    pub is_primary: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProviderContact {
    pub email: String,
    pub phone_office: String,
    pub phone_emergency: Option<String>,
    pub website: Option<String>,
}

/// Legacy provider-owned license record.
///
/// This records what the provider submitted. It is deliberately not treated as
/// external licensing-authority evidence by the high-assurance authority stack.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct License {
    pub provider_hash: ActionHash,
    pub license_type: LicenseType,
    pub license_number: String,
    pub issuing_authority: String,
    pub jurisdiction: String,
    pub issued_date: String,
    pub expiration_date: String,
    pub status: LicenseStatus,
    /// Legacy verification source metadata; not sufficient authority evidence.
    pub verification_source: Option<String>,
    pub verified_at: Option<Timestamp>,
    pub verified_by: Option<AgentPubKey>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum LicenseType {
    Medical,
    Nursing,
    Pharmacy,
    Dental,
    Psychology,
    Therapy,
    DEA,
    StateControlled,
    BoardCertification,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum LicenseStatus {
    Active,
    Expired,
    Suspended,
    Revoked,
    Pending,
    Restricted,
}

/// Legacy provider-owned board-certification record.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct BoardCertification {
    pub provider_hash: ActionHash,
    pub board_name: String,
    pub specialty: String,
    pub certification_number: Option<String>,
    pub initial_certification_date: String,
    pub expiration_date: String,
    pub status: CertificationStatus,
    pub verified: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CertificationStatus {
    Active,
    Expired,
    Pending,
    Revoked,
}

/// Provider-patient relationship.
///
/// v1 integrity proves that the author is one of the referenced record authors;
/// it does NOT prove bilateral consent. Coordinator-side consent checks remain a
/// separate layer until a consent-evidence-bearing relationship v2 is introduced.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ProviderPatientRelationship {
    pub provider_hash: ActionHash,
    pub patient_hash: ActionHash,
    pub relationship_type: RelationshipType,
    pub start_date: Timestamp,
    pub end_date: Option<Timestamp>,
    pub is_active: bool,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    PrimaryCare,
    Specialist,
    Consultant,
    EmergencyOnly,
    Research,
    Other(String),
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Provider(Provider),
    License(License),
    BoardCertification(BoardCertification),
    ProviderPatientRelationship(ProviderPatientRelationship),
}

#[hdk_link_types]
pub enum LinkTypes {
    ProviderToLicenses,
    ProviderToCertifications,
    ProviderToPatients,
    ProviderToRecords,
    ProviderToPrescriptions,
    ProviderToTrials,
    ProviderUpdates,
    AllProviders,
    ProvidersBySpecialty,
    ProvidersByLocation,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => {
                validate_create_entry(EntryCreationAction::Create(action), app_entry)
            }
            OpEntry::UpdateEntry {
                app_entry, action, ..
            } => validate_update_entry(action, app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            action,
            ..
        } => validate_create_link(link_type, base_address, target_address, action),
        FlatOp::RegisterDeleteLink {
            original_action,
            action,
            ..
        } => {
            if action.author != original_action.author {
                return invalid("Only the original link creator can delete a provider link");
            }
            Ok(ValidateCallbackResult::Valid)
        }
        FlatOp::RegisterUpdate(update) => {
            let action = match &update {
                OpUpdate::Entry { action, .. }
                | OpUpdate::PrivateEntry { action, .. }
                | OpUpdate::Agent { action, .. }
                | OpUpdate::CapClaim { action, .. }
                | OpUpdate::CapGrant { action, .. } => action,
            };
            let original = must_get_action(action.original_action_address.clone())?;
            if *original.action().author() != action.author {
                return invalid("Only the original entry author can update a provider entry");
            }
            match update {
                OpUpdate::Entry { app_entry, action } => validate_update_entry(action, app_entry),
                _ => Ok(ValidateCallbackResult::Valid),
            }
        }
        FlatOp::RegisterDelete(OpDelete { action }) => {
            let original = must_get_action(action.deletes_address.clone())?;
            if *original.action().author() != action.author {
                return invalid("Only the original entry author can delete a provider entry");
            }
            Ok(ValidateCallbackResult::Valid)
        }
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(
    action: EntryCreationAction,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::Provider(provider) => validate_provider(&provider),
        EntryTypes::License(license) => {
            let shape = validate_license(&license)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            require_provider_owner(action.author(), &license.provider_hash)
        }
        EntryTypes::BoardCertification(cert) => {
            let shape = validate_certification(&cert)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            require_provider_owner(action.author(), &cert.provider_hash)
        }
        EntryTypes::ProviderPatientRelationship(rel) => {
            let shape = validate_relationship(&rel)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            require_relationship_participant(action.author(), &rel)
        }
    }
}

fn validate_update_entry(
    action: Update,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    let original_action = must_get_action(action.original_action_address.clone())?;
    if *original_action.action().author() != action.author {
        return invalid("Only the original entry author can update a provider entry");
    }

    match entry {
        EntryTypes::Provider(provider) => {
            let original_record = must_get_valid_record(action.original_action_address.clone())?;
            let original: Provider = decode_entry(&original_record, "provider")?;

            // NPI and identity binding are identity-bearing fields. A profile may
            // evolve, but those identifiers cannot be rebound through an update.
            if original.npi != provider.npi {
                return invalid("Provider NPI is immutable; create a new provider identity");
            }
            if original.mycelix_identity_hash != provider.mycelix_identity_hash {
                return invalid(
                    "Provider legacy identity binding is immutable; use a verified binding lineage",
                );
            }
            if original.created_at != provider.created_at {
                return invalid("Provider created_at is immutable");
            }
            if provider.updated_at < original.updated_at || provider.updated_at < provider.created_at {
                return invalid("Provider updated_at must be monotonic and not predate created_at");
            }
            validate_provider(&provider)
        }
        EntryTypes::License(_) => invalid(
            "Legacy provider license records are immutable; create a new evidence record",
        ),
        EntryTypes::BoardCertification(_) => invalid(
            "Legacy board certification records are immutable; create a new evidence record",
        ),
        EntryTypes::ProviderPatientRelationship(_) => invalid(
            "Provider-patient relationship v1 records are immutable; create a new relationship event",
        ),
    }
}

fn validate_provider(provider: &Provider) -> ExternResult<ValidateCallbackResult> {
    if provider.first_name.trim().is_empty() || provider.last_name.trim().is_empty() {
        return invalid("Provider first and last name are required");
    }

    if provider.title.trim().is_empty() {
        return invalid("Provider title is required");
    }

    if provider.specialty.trim().is_empty() {
        return invalid("Provider specialty is required");
    }

    if !provider.matl_trust_score.is_finite()
        || provider.matl_trust_score < 0.0
        || provider.matl_trust_score > 1.0
    {
        return invalid("MATL trust score must be finite and between 0.0 and 1.0");
    }

    if provider.updated_at < provider.created_at {
        return invalid("Provider updated_at cannot predate created_at");
    }

    // Format validation only. NPI existence/ownership requires authoritative
    // registry evidence and is never inferred from this field alone.
    if let Some(ref npi) = provider.npi {
        if npi.len() != 10 || !npi.chars().all(|c| c.is_ascii_digit()) {
            return invalid("NPI must be a 10-digit number");
        }
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_license(license: &License) -> ExternResult<ValidateCallbackResult> {
    if license.license_number.trim().is_empty() {
        return invalid("License number is required");
    }
    if license.issuing_authority.trim().is_empty() {
        return invalid("Issuing authority is required");
    }
    if license.jurisdiction.trim().is_empty() {
        return invalid("License jurisdiction is required");
    }
    if license.issued_date.trim().is_empty() || license.expiration_date.trim().is_empty() {
        return invalid("License issue and expiration dates are required");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_certification(cert: &BoardCertification) -> ExternResult<ValidateCallbackResult> {
    if cert.board_name.trim().is_empty() {
        return invalid("Board name is required");
    }
    if cert.specialty.trim().is_empty() {
        return invalid("Specialty is required");
    }
    if cert.initial_certification_date.trim().is_empty() || cert.expiration_date.trim().is_empty() {
        return invalid("Certification issue and expiration dates are required");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_relationship(
    rel: &ProviderPatientRelationship,
) -> ExternResult<ValidateCallbackResult> {
    if rel.provider_hash == rel.patient_hash {
        return invalid("Provider and patient references cannot be the same action");
    }
    if let Some(end) = rel.end_date {
        if end < rel.start_date {
            return invalid("Provider-patient relationship end cannot predate start");
        }
        if rel.is_active {
            return invalid("Ended provider-patient relationship cannot remain active");
        }
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_provider_owner(
    author: &AgentPubKey,
    provider_hash: &ActionHash,
) -> ExternResult<ValidateCallbackResult> {
    let provider_record = must_get_valid_record(provider_hash.clone())?;
    let _: Provider = decode_entry(&provider_record, "provider")?;
    if provider_record.action().author() != author {
        return invalid("Only the provider profile author may attach this legacy record");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_relationship_participant(
    author: &AgentPubKey,
    rel: &ProviderPatientRelationship,
) -> ExternResult<ValidateCallbackResult> {
    let provider_record = must_get_valid_record(rel.provider_hash.clone())?;
    let _: Provider = decode_entry(&provider_record, "provider")?;
    let patient_record = must_get_valid_record(rel.patient_hash.clone())?;

    if provider_record.action().author() != author && patient_record.action().author() != author {
        return invalid("Only a referenced provider/patient record author may create the relationship");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_create_link(
    link_type: LinkTypes,
    base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    action: CreateLink,
) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        LinkTypes::ProviderToLicenses => {
            let provider_hash = require_action_hash(base_address, "provider link base")?;
            let license_hash = require_action_hash(target_address, "license link target")?;
            let provider_record = must_get_valid_record(provider_hash.clone())?;
            let _: Provider = decode_entry(&provider_record, "provider")?;
            let license_record = must_get_valid_record(license_hash)?;
            let license: License = decode_entry(&license_record, "license")?;

            if license.provider_hash != provider_hash {
                return invalid("License link target does not belong to provider link base");
            }
            if provider_record.action().author() != &action.author
                || license_record.action().author() != &action.author
            {
                return invalid("Only provider owner may create provider-to-license link");
            }
            Ok(ValidateCallbackResult::Valid)
        }
        LinkTypes::ProviderToCertifications => {
            let provider_hash = require_action_hash(base_address, "provider link base")?;
            let cert_hash = require_action_hash(target_address, "certification link target")?;
            let provider_record = must_get_valid_record(provider_hash.clone())?;
            let _: Provider = decode_entry(&provider_record, "provider")?;
            let cert_record = must_get_valid_record(cert_hash)?;
            let cert: BoardCertification = decode_entry(&cert_record, "board certification")?;

            if cert.provider_hash != provider_hash {
                return invalid("Certification link target does not belong to provider link base");
            }
            if provider_record.action().author() != &action.author
                || cert_record.action().author() != &action.author
            {
                return invalid("Only provider owner may create provider-to-certification link");
            }
            Ok(ValidateCallbackResult::Valid)
        }
        LinkTypes::ProviderToPatients => {
            let provider_hash = require_action_hash(base_address, "provider link base")?;
            let patient_hash = require_action_hash(target_address, "patient link target")?;
            let provider_record = must_get_valid_record(provider_hash)?;
            let _: Provider = decode_entry(&provider_record, "provider")?;
            let patient_record = must_get_valid_record(patient_hash)?;

            if provider_record.action().author() != &action.author
                && patient_record.action().author() != &action.author
            {
                return invalid("Only a referenced provider/patient record author may create patient link");
            }
            Ok(ValidateCallbackResult::Valid)
        }
        LinkTypes::ProviderUpdates => {
            let original_hash = require_action_hash(base_address, "provider update base")?;
            let updated_hash = require_action_hash(target_address, "provider update target")?;
            let original = must_get_valid_record(original_hash)?;
            let updated = must_get_valid_record(updated_hash)?;
            let _: Provider = decode_entry(&original, "provider update base")?;
            let _: Provider = decode_entry(&updated, "provider update target")?;
            if original.action().author() != &action.author
                || updated.action().author() != &action.author
            {
                return invalid("Provider update link must be authored by provider owner");
            }
            Ok(ValidateCallbackResult::Valid)
        }
        LinkTypes::AllProviders | LinkTypes::ProvidersBySpecialty | LinkTypes::ProvidersByLocation => {
            let target = require_action_hash(target_address, "provider index target")?;
            let provider_record = must_get_valid_record(target)?;
            let _: Provider = decode_entry(&provider_record, "provider index target")?;
            if provider_record.action().author() != &action.author {
                return invalid("Only provider owner may publish provider index links");
            }
            Ok(ValidateCallbackResult::Valid)
        }
        LinkTypes::ProviderToRecords
        | LinkTypes::ProviderToPrescriptions
        | LinkTypes::ProviderToTrials => invalid(
            "Reserved provider link type is disabled until an explicit authority contract exists",
        ),
    }
}

fn require_action_hash(
    hash: AnyLinkableHash,
    field: &'static str,
) -> ExternResult<ActionHash> {
    hash.into_action_hash().ok_or(wasm_error!(WasmErrorInner::Guest(
        format!("{field} must be an ActionHash")
    )))
}

fn decode_entry<T>(record: &Record, label: &'static str) -> ExternResult<T>
where
    T: TryFrom<SerializedBytes, Error = SerializedBytesError>,
{
    record
        .entry()
        .to_app_option::<T>()
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(format!(
            "Referenced {label} entry is missing or wrong type"
        ))))
}

fn invalid(message: impl Into<String>) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(byte: u8) -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![byte; 36])
    }

    fn action_hash(byte: u8) -> ActionHash {
        ActionHash::from_raw_36(vec![byte; 36])
    }

    fn provider() -> Provider {
        Provider {
            npi: Some("1234567890".into()),
            provider_type: ProviderType::Physician,
            first_name: "Ada".into(),
            last_name: "Example".into(),
            title: "MD".into(),
            specialty: "Internal Medicine".into(),
            sub_specialties: vec![],
            organization: None,
            locations: vec![],
            contact: ProviderContact {
                email: "a@example.invalid".into(),
                phone_office: "000".into(),
                phone_emergency: None,
                website: None,
            },
            languages: vec!["en".into()],
            accepting_patients: true,
            telehealth_enabled: false,
            mycelix_identity_hash: None,
            matl_trust_score: 0.5,
            epistemic_level: EpistemicLevel::Unverified,
            created_at: Timestamp::from_micros(10),
            updated_at: Timestamp::from_micros(10),
        }
    }

    #[test]
    fn non_finite_matl_score_is_rejected() {
        let mut p = provider();
        p.matl_trust_score = f64::NAN;
        let result = validate_provider(&p).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn relationship_cannot_self_reference_same_action() {
        let hash = action_hash(1);
        let rel = ProviderPatientRelationship {
            provider_hash: hash.clone(),
            patient_hash: hash,
            relationship_type: RelationshipType::PrimaryCare,
            start_date: Timestamp::from_micros(1),
            end_date: None,
            is_active: true,
            notes: None,
        };
        let result = validate_relationship(&rel).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn ended_relationship_cannot_be_active() {
        let rel = ProviderPatientRelationship {
            provider_hash: action_hash(1),
            patient_hash: action_hash(2),
            relationship_type: RelationshipType::PrimaryCare,
            start_date: Timestamp::from_micros(1),
            end_date: Some(Timestamp::from_micros(2)),
            is_active: true,
            notes: None,
        };
        let result = validate_relationship(&rel).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn provider_shape_accepts_finite_score_and_monotonic_time() {
        assert_eq!(validate_provider(&provider()).unwrap(), ValidateCallbackResult::Valid);
    }

    #[test]
    fn distinct_agents_are_not_equal() {
        assert_ne!(agent(1), agent(2));
    }
}
