//! Health Credentials Coordinator Zome
//!
//! Verifiable health credentials with issuer verification and revocation.
//! Uses LinkQuery::try_new() for link queries.

use hdk::prelude::*;
use credentials_integrity::{
    Anchor as CredentialsAnchor, CredentialRevocation, CredentialType, EntryTypes,
    HealthCredential, LinkTypes,
};

// ============================================================================
// Anchor Helpers
// ============================================================================

/// Get or create an anchor entry and return its entry hash
fn get_or_create_anchor(anchor_value: &str) -> ExternResult<EntryHash> {
    let anchor = CredentialsAnchor::new(anchor_value);
    let entry_hash = hash_entry(&anchor)?;

    // Try to get existing anchor
    if get(entry_hash.clone(), GetOptions::default())?.is_none() {
        create_entry(&EntryTypes::Anchor(anchor))?;
    }

    Ok(entry_hash)
}

/// Get current agent's DID
fn get_my_did() -> ExternResult<String> {
    let agent_info = agent_info()?;
    Ok(format!("did:mycelix:{}", agent_info.agent_initial_pubkey))
}

// ============================================================================
// Credential CRUD
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueCredentialInput {
    pub holder_did: String,
    pub credential_type: CredentialType,
    pub claims: String,
    pub expires_in_days: Option<u32>,
}

#[hdk_extern]
pub fn issue_credential(input: IssueCredentialInput) -> ExternResult<Record> {
    let issuer_did = get_my_did()?;
    let now = sys_time()?;

    // Calculate expiration
    let expires = input.expires_in_days.map(|days| {
        let micros = (days as i64) * 24 * 60 * 60 * 1_000_000;
        Timestamp::from_micros(now.as_micros() + micros)
    });

    let credential = HealthCredential {
        holder_did: input.holder_did.clone(),
        credential_type: input.credential_type.clone(),
        issuer_did: issuer_did.clone(),
        claims: input.claims,
        issued: now,
        expires,
    };

    let action_hash = create_entry(&EntryTypes::HealthCredential(credential))?;

    // Link holder to credential
    let holder_anchor = get_or_create_anchor(&format!("holder:{}", input.holder_did))?;
    create_link(
        holder_anchor,
        action_hash.clone(),
        LinkTypes::HolderToCredentials,
        (),
    )?;

    // Link issuer to credential
    let issuer_anchor = get_or_create_anchor(&format!("issuer:{}", issuer_did))?;
    create_link(
        issuer_anchor,
        action_hash.clone(),
        LinkTypes::IssuerToCredentials,
        (),
    )?;

    // Link credential type to credential
    let type_anchor =
        get_or_create_anchor(&format!("credential_type:{:?}", input.credential_type))?;
    create_link(
        type_anchor,
        action_hash.clone(),
        LinkTypes::CredentialTypeToCredentials,
        (),
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Credential not found after creation".into()
        )))
}

#[hdk_extern]
pub fn get_credential(action_hash: ActionHash) -> ExternResult<Option<CredentialWithStatus>> {
    if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
        if let Some(credential) = record
            .entry()
            .to_app_option::<HealthCredential>()
            .ok()
            .flatten()
        {
            let is_revoked = is_credential_revoked(&action_hash)?;
            let now = sys_time()?;
            let is_expired = credential.expires.map_or(false, |exp| exp <= now);

            return Ok(Some(CredentialWithStatus {
                credential,
                action_hash,
                is_revoked,
                is_expired,
                is_valid: !is_revoked && !is_expired,
            }));
        }
    }
    Ok(None)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CredentialWithStatus {
    pub credential: HealthCredential,
    pub action_hash: ActionHash,
    pub is_revoked: bool,
    pub is_expired: bool,
    pub is_valid: bool,
}

#[hdk_extern]
pub fn get_my_credentials(_: ()) -> ExternResult<Vec<CredentialWithStatus>> {
    let my_did = get_my_did()?;
    let holder_anchor = get_or_create_anchor(&format!("holder:{}", my_did))?;

    let query = LinkQuery::try_new(holder_anchor, LinkTypes::HolderToCredentials)?;
    let links = get_links(query, GetStrategy::default())?;

    let now = sys_time()?;
    let mut credentials = Vec::new();

    for link in links {
        if let Ok(action_hash) = ActionHash::try_from(link.target) {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(credential) = record
                    .entry()
                    .to_app_option::<HealthCredential>()
                    .ok()
                    .flatten()
                {
                    let is_revoked = is_credential_revoked(&action_hash)?;
                    let is_expired = credential.expires.map_or(false, |exp| exp <= now);

                    credentials.push(CredentialWithStatus {
                        credential,
                        action_hash,
                        is_revoked,
                        is_expired,
                        is_valid: !is_revoked && !is_expired,
                    });
                }
            }
        }
    }

    Ok(credentials)
}

#[hdk_extern]
pub fn get_issued_credentials(_: ()) -> ExternResult<Vec<CredentialWithStatus>> {
    let my_did = get_my_did()?;
    let issuer_anchor = get_or_create_anchor(&format!("issuer:{}", my_did))?;

    let query = LinkQuery::try_new(issuer_anchor, LinkTypes::IssuerToCredentials)?;
    let links = get_links(query, GetStrategy::default())?;

    let now = sys_time()?;
    let mut credentials = Vec::new();

    for link in links {
        if let Ok(action_hash) = ActionHash::try_from(link.target) {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(credential) = record
                    .entry()
                    .to_app_option::<HealthCredential>()
                    .ok()
                    .flatten()
                {
                    let is_revoked = is_credential_revoked(&action_hash)?;
                    let is_expired = credential.expires.map_or(false, |exp| exp <= now);

                    credentials.push(CredentialWithStatus {
                        credential,
                        action_hash,
                        is_revoked,
                        is_expired,
                        is_valid: !is_revoked && !is_expired,
                    });
                }
            }
        }
    }

    Ok(credentials)
}

#[hdk_extern]
pub fn get_credentials_by_type(credential_type: CredentialType) -> ExternResult<Vec<CredentialWithStatus>> {
    let my_did = get_my_did()?;
    let type_anchor = get_or_create_anchor(&format!("credential_type:{:?}", credential_type))?;

    let query = LinkQuery::try_new(type_anchor, LinkTypes::CredentialTypeToCredentials)?;
    let links = get_links(query, GetStrategy::default())?;

    let now = sys_time()?;
    let mut credentials = Vec::new();

    for link in links {
        if let Ok(action_hash) = ActionHash::try_from(link.target) {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(credential) = record
                    .entry()
                    .to_app_option::<HealthCredential>()
                    .ok()
                    .flatten()
                {
                    // Only return credentials where caller is holder or issuer
                    if credential.holder_did == my_did || credential.issuer_did == my_did {
                        let is_revoked = is_credential_revoked(&action_hash)?;
                        let is_expired = credential.expires.map_or(false, |exp| exp <= now);

                        credentials.push(CredentialWithStatus {
                            credential,
                            action_hash,
                            is_revoked,
                            is_expired,
                            is_valid: !is_revoked && !is_expired,
                        });
                    }
                }
            }
        }
    }

    Ok(credentials)
}

// ============================================================================
// Revocation Operations
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeCredentialInput {
    pub credential_hash: ActionHash,
    pub reason: String,
}

#[hdk_extern]
pub fn revoke_credential(input: RevokeCredentialInput) -> ExternResult<Record> {
    let revoker_did = get_my_did()?;
    let now = sys_time()?;

    // Get the credential to verify issuer
    let credential_record = get(input.credential_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Credential not found".into()
        )))?;

    let credential = credential_record
        .entry()
        .to_app_option::<HealthCredential>()
        .ok()
        .flatten()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid credential format".into()
        )))?;

    // Only issuer can revoke
    if credential.issuer_did != revoker_did {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the issuer can revoke a credential".into()
        )));
    }

    // Check if already revoked
    if is_credential_revoked(&input.credential_hash)? {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Credential is already revoked".into()
        )));
    }

    let revocation = CredentialRevocation {
        credential_hash: input.credential_hash.clone(),
        revoker_did: revoker_did.clone(),
        reason: input.reason,
        revoked_at: now,
    };

    let action_hash = create_entry(&EntryTypes::CredentialRevocation(revocation))?;

    // Link credential to revocation
    create_link(
        input.credential_hash,
        action_hash.clone(),
        LinkTypes::CredentialToRevocation,
        (),
    )?;

    // Link issuer to revocation
    let issuer_anchor = get_or_create_anchor(&format!("issuer:{}", revoker_did))?;
    create_link(
        issuer_anchor,
        action_hash.clone(),
        LinkTypes::IssuerToRevocations,
        (),
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Revocation not found after creation".into()
        )))
}

#[hdk_extern]
pub fn get_revocation(credential_hash: ActionHash) -> ExternResult<Option<Record>> {
    let query = LinkQuery::try_new(credential_hash, LinkTypes::CredentialToRevocation)?;
    let links = get_links(query, GetStrategy::default())?;

    for link in links {
        if let Ok(action_hash) = ActionHash::try_from(link.target) {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                return Ok(Some(record));
            }
        }
    }

    Ok(None)
}

fn is_credential_revoked(credential_hash: &ActionHash) -> ExternResult<bool> {
    let query = LinkQuery::try_new(credential_hash.clone(), LinkTypes::CredentialToRevocation)?;
    let links = get_links(query, GetStrategy::default())?;
    Ok(!links.is_empty())
}

// ============================================================================
// Verification Operations
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyCredentialInput {
    pub credential_hash: ActionHash,
    pub expected_issuer_did: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub credential: Option<HealthCredential>,
    pub errors: Vec<String>,
}

#[hdk_extern]
pub fn verify_credential(input: VerifyCredentialInput) -> ExternResult<VerificationResult> {
    let mut errors = Vec::new();

    // Get the credential
    let credential_record = match get(input.credential_hash.clone(), GetOptions::default())? {
        Some(record) => record,
        None => {
            return Ok(VerificationResult {
                is_valid: false,
                credential: None,
                errors: vec!["Credential not found".into()],
            });
        }
    };

    let credential = match credential_record
        .entry()
        .to_app_option::<HealthCredential>()
        .ok()
        .flatten()
    {
        Some(c) => c,
        None => {
            return Ok(VerificationResult {
                is_valid: false,
                credential: None,
                errors: vec!["Invalid credential format".into()],
            });
        }
    };

    // Check if revoked
    if is_credential_revoked(&input.credential_hash)? {
        errors.push("Credential has been revoked".into());
    }

    // Check if expired
    let now = sys_time()?;
    if let Some(expires) = credential.expires {
        if expires <= now {
            errors.push("Credential has expired".into());
        }
    }

    // Check expected issuer if provided
    if let Some(expected_issuer) = input.expected_issuer_did {
        if credential.issuer_did != expected_issuer {
            errors.push(format!(
                "Issuer mismatch: expected {}, got {}",
                expected_issuer, credential.issuer_did
            ));
        }
    }

    Ok(VerificationResult {
        is_valid: errors.is_empty(),
        credential: Some(credential),
        errors,
    })
}

#[hdk_extern]
pub fn get_my_revocations(_: ()) -> ExternResult<Vec<Record>> {
    let my_did = get_my_did()?;
    let issuer_anchor = get_or_create_anchor(&format!("issuer:{}", my_did))?;

    let query = LinkQuery::try_new(issuer_anchor, LinkTypes::IssuerToRevocations)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut revocations = Vec::new();
    for link in links {
        if let Ok(action_hash) = ActionHash::try_from(link.target) {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                revocations.push(record);
            }
        }
    }

    Ok(revocations)
}
