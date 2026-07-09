#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
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
                return Ok(ValidateCallbackResult::Invalid(
                    "Only the original entry author can update their entries".into(),
                ));
            }
            Ok(ValidateCallbackResult::Valid)
        }
        FlatOp::RegisterDelete(OpDelete { action, .. }) => {
            let original = must_get_action(action.deletes_address.clone())?;
            if *original.action().author() != action.author {
                return Ok(ValidateCallbackResult::Invalid(
                    "Only the original entry author can delete their entries".into(),
                ));
            }
            Ok(ValidateCallbackResult::Valid)
        }
    }
}

fn validate_create_entry(
    action: EntryCreationAction,
    app_entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match app_entry {
        EntryTypes::Anchor(anchor) => validate_anchor(anchor),
        EntryTypes::HealthCredential(credential) => validate_health_credential(action, credential),
        EntryTypes::CredentialRevocation(revocation) => {
            validate_credential_revocation(action, revocation)
        }
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

/// holder_did is deliberately NOT bound to the committer: it's a
/// third-party field by design (the issuer commits the entry but names a
/// DIFFERENT agent, the patient/practitioner, as holder). Reviewed
/// 2026-07-09 during the P0 author-binding pass; case (b).
fn validate_health_credential(
    action: EntryCreationAction,
    credential: HealthCredential,
) -> ExternResult<ValidateCallbackResult> {
    // Author-binding: the coordinator's issue_credential already derives
    // issuer_did from get_my_did() (agent_info()-based), so this is
    // belt-and-suspenders against a modified coordinator forging a victim
    // agent as issuer. Found + fixed 2026-07-09 during the P0
    // author-binding pass.
    let expected_issuer = format!("did:mycelix:{}", action.author());
    if credential.issuer_did != expected_issuer {
        return Ok(ValidateCallbackResult::Invalid(
            "issuer_did must correspond to the committing agent".into(),
        ));
    }

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

/// Author-binding + cross-entry authorization check. The coordinator's
/// revoke_credential already derives revoker_did from get_my_did() and
/// checks (client-side only, bypassable by a modified coordinator) that
/// the caller is the credential's issuer before creating this entry.
/// Found + fixed 2026-07-09 during the P0 author-binding pass: this is
/// the real DHT-level enforcement of BOTH checks -- revoker_did must
/// correspond to the committing agent, AND the committing agent must
/// actually be the ORIGINAL credential's issuer (fetched via
/// must_get_valid_record), not merely claim to be. Without the second
/// check, a modified coordinator could forge a valid-looking revocation
/// of someone else's credential from a non-issuer agent.
fn validate_credential_revocation(
    action: EntryCreationAction,
    revocation: CredentialRevocation,
) -> ExternResult<ValidateCallbackResult> {
    let expected_revoker = format!("did:mycelix:{}", action.author());
    if revocation.revoker_did != expected_revoker {
        return Ok(ValidateCallbackResult::Invalid(
            "revoker_did must correspond to the committing agent".into(),
        ));
    }

    let credential_record = must_get_valid_record(revocation.credential_hash.clone())?;
    let credential: HealthCredential = credential_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Referenced credential not found".into()
        )))?;
    if credential.issuer_did != revocation.revoker_did {
        return Ok(ValidateCallbackResult::Invalid(
            "Only the credential's issuer can revoke it".into(),
        ));
    }

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
        LinkTypes::CredentialToRevocation | LinkTypes::IssuerToRevocations => Ok(
            ValidateCallbackResult::Invalid("Revocation links cannot be deleted".into()),
        ),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

#[cfg(test)]
mod author_binding_tests {
    use super::*;

    fn create_action(author: AgentPubKey) -> EntryCreationAction {
        EntryCreationAction::Create(Create {
            author,
            timestamp: Timestamp::from_micros(0),
            action_seq: 0,
            prev_action: ActionHash::from_raw_36(vec![0u8; 36]),
            entry_type: EntryType::App(AppEntryDef::new(
                EntryDefIndex::from(0),
                0.into(),
                EntryVisibility::Public,
            )),
            entry_hash: EntryHash::from_raw_36(vec![0u8; 36]),
            weight: Default::default(),
        })
    }

    fn me() -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![0u8; 36])
    }

    fn other_agent() -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![1u8; 36])
    }

    fn valid_credential(issuer_did: String) -> HealthCredential {
        HealthCredential {
            holder_did: "did:mycelix:some-patient-agent-pubkey".into(),
            credential_type: CredentialType::VaccinationProof,
            issuer_did,
            claims: "{}".into(),
            issued: Timestamp::from_micros(0),
            expires: None,
        }
    }

    #[test]
    fn create_credential_valid_when_issuer_matches_committer() {
        let author = me();
        let c = valid_credential(format!("did:mycelix:{}", author));
        let result = validate_health_credential(create_action(author), c).unwrap();
        assert_eq!(result, ValidateCallbackResult::Valid);
    }

    #[test]
    fn create_credential_issuer_forgery_rejected() {
        let c = valid_credential(format!("did:mycelix:{}", me()));
        let result = validate_health_credential(create_action(other_agent()), c).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn create_credential_holder_third_party_allowed() {
        // holder_did naming a different agent than the committer/issuer is
        // the expected case, not forgery.
        let author = me();
        let c = valid_credential(format!("did:mycelix:{}", author));
        assert_ne!(c.holder_did, format!("did:mycelix:{}", author));
        let result = validate_health_credential(create_action(author), c).unwrap();
        assert_eq!(result, ValidateCallbackResult::Valid);
    }

    #[test]
    fn create_revocation_revoker_forgery_rejected_before_must_get() {
        // revoker_did must match the committing agent; this is checked
        // before must_get_valid_record, so it's testable without a live
        // HDI host.
        let revocation = CredentialRevocation {
            credential_hash: ActionHash::from_raw_36(vec![2u8; 36]),
            revoker_did: format!("did:mycelix:{}", me()),
            reason: "expired".into(),
            revoked_at: Timestamp::from_micros(0),
        };
        let result =
            validate_credential_revocation(create_action(other_agent()), revocation).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }
}
