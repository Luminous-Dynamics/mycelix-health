//! Health Credentials Integrity Zome
//!
//! Verifiable health credentials with issuer verification and revocation support.
//! Uses Anchor pattern for link bases and FlatOp validation.

use hdi::prelude::*;

// ============================================================================
// Anchor Entry Type
// ============================================================================

/// Anchor entry for creating deterministic link bases from strings
#[hdk_entry_helper]
#[derive(Clone, PartialEq, Eq)]
pub struct Anchor(pub String);

impl Anchor {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

// ============================================================================
// Credential Types
// ============================================================================

/// Type of health credential
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum CredentialType {
    /// Proof of vaccination (e.g., COVID-19, flu, etc.)
    VaccinationProof,
    /// Healthcare practitioner license
    PractitionerLicense,
    /// Insurance coverage verification
    InsuranceCoverage,
}

/// Verifiable health credential
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HealthCredential {
    /// DID of the credential holder (patient/practitioner)
    pub holder_did: String,
    /// Type of credential
    pub credential_type: CredentialType,
    /// DID of the issuing authority
    pub issuer_did: String,
    /// JSON-encoded claims (can be encrypted)
    pub claims: String,
    /// When the credential was issued
    pub issued: Timestamp,
    /// When the credential expires (None = no expiration)
    pub expires: Option<Timestamp>,
}

/// Credential revocation entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CredentialRevocation {
    /// Hash of the credential being revoked
    pub credential_hash: ActionHash,
    /// DID of the revoker (must be issuer)
    pub revoker_did: String,
    /// Reason for revocation
    pub reason: String,
    /// When the revocation occurred
    pub revoked_at: Timestamp,
}

// ============================================================================
// Entry Types and Link Types
// ============================================================================

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Anchor(Anchor),
    HealthCredential(HealthCredential),
    CredentialRevocation(CredentialRevocation),
}

#[hdk_link_types]
pub enum LinkTypes {
    /// Anchor to anchor (for path-like structures)
    AnchorToAnchor,
    /// Holder DID anchor to their credentials
    HolderToCredentials,
    /// Issuer DID anchor to credentials they issued
    IssuerToCredentials,
    /// Credential type anchor to credentials
    CredentialTypeToCredentials,
    /// Credential to its revocation (if any)
    CredentialToRevocation,
    /// Issuer to revocations they created
    IssuerToRevocations,
}

// ============================================================================
// DID Validation Helper
// ============================================================================

/// Validates a Mycelix DID format: did:mycelix:<identifier>
fn validate_did(did: &str, field_name: &str) -> ExternResult<ValidateCallbackResult> {
    if !did.starts_with("did:mycelix:") {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "{} must be a valid Mycelix DID (did:mycelix:...)",
            field_name
        )));
    }
    if did.len() < 20 {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "{} DID is too short",
            field_name
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

// ============================================================================
// Validation
// ============================================================================

#[hdk_extern]
pub fn genesis_self_check(_data: GenesisSelfCheckData) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
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
            } => validate_create_entry(EntryCreationAction::Update(action), app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            tag,
            action,
        } => validate_create_link(link_type, base_address, target_address, tag, action),
        FlatOp::RegisterDeleteLink {
            link_type,
            original_action,
            action,
            ..
        } => validate_delete_link(link_type, original_action, action),
        FlatOp::StoreRecord(_) => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterAgentActivity(_) => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterUpdate(_) => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterDelete(_) => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(
    _action: EntryCreationAction,
    app_entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match app_entry {
        EntryTypes::Anchor(anchor) => validate_anchor(anchor),
        EntryTypes::HealthCredential(credential) => validate_health_credential(credential),
        EntryTypes::CredentialRevocation(revocation) => validate_credential_revocation(revocation),
    }
}

fn validate_anchor(anchor: Anchor) -> ExternResult<ValidateCallbackResult> {
    if anchor.0.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Anchor value cannot be empty".into(),
        ));
    }
    if anchor.0.len() > 1024 {
        return Ok(ValidateCallbackResult::Invalid(
            "Anchor value too long (max 1024 bytes)".into(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_health_credential(credential: HealthCredential) -> ExternResult<ValidateCallbackResult> {
    // Validate holder DID
    let result = validate_did(&credential.holder_did, "holder_did")?;
    if let ValidateCallbackResult::Invalid(_) = result {
        return Ok(result);
    }

    // Validate issuer DID
    let result = validate_did(&credential.issuer_did, "issuer_did")?;
    if let ValidateCallbackResult::Invalid(_) = result {
        return Ok(result);
    }

    // Claims cannot be empty
    if credential.claims.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "claims cannot be empty".into(),
        ));
    }

    // Expiration must be after issuance
    if let Some(expires) = credential.expires {
        if expires <= credential.issued {
            return Ok(ValidateCallbackResult::Invalid(
                "expires must be after issued timestamp".into(),
            ));
        }
    }

    // Validate claims is valid JSON
    if serde_json::from_str::<serde_json::Value>(&credential.claims).is_err() {
        return Ok(ValidateCallbackResult::Invalid(
            "claims must be valid JSON".into(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_credential_revocation(
    revocation: CredentialRevocation,
) -> ExternResult<ValidateCallbackResult> {
    // Validate revoker DID
    let result = validate_did(&revocation.revoker_did, "revoker_did")?;
    if let ValidateCallbackResult::Invalid(_) = result {
        return Ok(result);
    }

    // Reason cannot be empty
    if revocation.reason.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "revocation reason cannot be empty".into(),
        ));
    }

    if revocation.reason.len() > 1024 {
        return Ok(ValidateCallbackResult::Invalid(
            "revocation reason too long (max 1024 characters)".into(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_create_link(
    link_type: LinkTypes,
    _base_address: AnyLinkableHash,
    _target_address: AnyLinkableHash,
    _tag: LinkTag,
    _action: CreateLink,
) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        LinkTypes::AnchorToAnchor => Ok(ValidateCallbackResult::Valid),
        LinkTypes::HolderToCredentials => Ok(ValidateCallbackResult::Valid),
        LinkTypes::IssuerToCredentials => Ok(ValidateCallbackResult::Valid),
        LinkTypes::CredentialTypeToCredentials => Ok(ValidateCallbackResult::Valid),
        LinkTypes::CredentialToRevocation => Ok(ValidateCallbackResult::Valid),
        LinkTypes::IssuerToRevocations => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_delete_link(
    link_type: LinkTypes,
    _original_action: CreateLink,
    _action: DeleteLink,
) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        // Revocation links cannot be deleted (immutable audit trail)
        LinkTypes::CredentialToRevocation | LinkTypes::IssuerToRevocations => {
            Ok(ValidateCallbackResult::Invalid(
                "Revocation links cannot be deleted".into(),
            ))
        }
        _ => Ok(ValidateCallbackResult::Valid),
    }
}
