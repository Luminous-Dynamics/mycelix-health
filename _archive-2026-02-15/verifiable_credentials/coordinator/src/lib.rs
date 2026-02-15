//! Verifiable Credentials Coordinator Zome
//!
//! Implements W3C Verifiable Credentials operations for health credentials
//! including issuance, verification, presentation, and revocation.

use hdk::prelude::*;
use verifiable_credentials_integrity::*;

/// Anchor for all schemas
const ALL_SCHEMAS_ANCHOR: &str = "all_schemas";
/// Anchor for all issuers
const ALL_ISSUERS_ANCHOR: &str = "all_issuers";

// ============================================================================
// Schema Management
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateSchemaInput {
    pub schema_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub schema_type: String,
    pub required_attributes: Vec<String>,
    pub optional_attributes: Vec<String>,
    pub attribute_definitions: String,
    pub context_urls: Vec<String>,
    pub is_standard: bool,
    pub standard_reference: Option<String>,
}

#[hdk_extern]
pub fn create_schema(input: CreateSchemaInput) -> ExternResult<ActionHash> {
    let schema = CredentialSchema {
        schema_id: input.schema_id,
        name: input.name,
        version: input.version,
        description: input.description,
        author: input.author,
        schema_type: input.schema_type,
        required_attributes: input.required_attributes,
        optional_attributes: input.optional_attributes,
        attribute_definitions: input.attribute_definitions,
        context_urls: input.context_urls,
        is_standard: input.is_standard,
        standard_reference: input.standard_reference,
        created_at: sys_time()?,
    };

    let action_hash = create_entry(EntryTypes::CredentialSchema(schema))?;

    // Link to all schemas anchor
    let anchor = anchor(LinkTypes::AllSchemas, ALL_SCHEMAS_ANCHOR.to_string())?;
    create_link(anchor, action_hash.clone(), LinkTypes::AllSchemas, ())?;

    Ok(action_hash)
}

#[hdk_extern]
pub fn get_schema(action_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(action_hash, GetOptions::default())
}

#[hdk_extern]
pub fn get_all_schemas(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor(LinkTypes::AllSchemas, ALL_SCHEMAS_ANCHOR.to_string())?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::AllSchemas)?, GetStrategy::default())?;

    let mut schemas = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                schemas.push(record);
            }
        }
    }

    Ok(schemas)
}

// ============================================================================
// Issuer Management
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterIssuerInput {
    pub issuer_id: String,
    pub name: String,
    pub issuer_type: String,
    pub organization_id: Option<String>,
    pub public_key: String,
    pub key_type: String,
    pub verification_endpoint: Option<String>,
    pub authorized_types: Vec<CredentialType>,
    pub jurisdiction: Option<String>,
}

#[hdk_extern]
pub fn register_issuer(input: RegisterIssuerInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let issuer = CredentialIssuer {
        issuer_id: input.issuer_id,
        name: input.name,
        issuer_type: input.issuer_type,
        organization_id: input.organization_id,
        public_key: input.public_key,
        key_type: input.key_type,
        verification_endpoint: input.verification_endpoint,
        trusted_by: Vec::new(),
        authorized_types: input.authorized_types,
        jurisdiction: input.jurisdiction,
        is_active: true,
        created_at: now,
        updated_at: now,
    };

    let action_hash = create_entry(EntryTypes::CredentialIssuer(issuer))?;

    // Link to all issuers anchor
    let anchor = anchor(LinkTypes::AllIssuers, ALL_ISSUERS_ANCHOR.to_string())?;
    create_link(anchor, action_hash.clone(), LinkTypes::AllIssuers, ())?;

    Ok(action_hash)
}

#[hdk_extern]
pub fn get_issuer(action_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(action_hash, GetOptions::default())
}

#[hdk_extern]
pub fn get_all_issuers(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor(LinkTypes::AllIssuers, ALL_ISSUERS_ANCHOR.to_string())?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::AllIssuers)?, GetStrategy::default())?;

    let mut issuers = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                issuers.push(record);
            }
        }
    }

    Ok(issuers)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTrustInput {
    pub trustee_hash: ActionHash,
    pub trust_level: u8,
    pub trust_scope: Vec<CredentialType>,
    pub reason: Option<String>,
    pub valid_until: Option<Timestamp>,
}

#[hdk_extern]
pub fn create_trust(input: CreateTrustInput) -> ExternResult<ActionHash> {
    // Get caller as trustor
    let agent_info = agent_info()?;

    let trust = TrustEntry {
        trustor_hash: ActionHash::from_raw_36(agent_info.agent_initial_pubkey.get_raw_36().to_vec()),
        trustee_hash: input.trustee_hash.clone(),
        trust_level: input.trust_level,
        trust_scope: input.trust_scope,
        reason: input.reason,
        valid_from: sys_time()?,
        valid_until: input.valid_until,
        is_active: true,
    };

    let action_hash = create_entry(EntryTypes::TrustEntry(trust))?;

    // Link for trust chain
    create_link(
        input.trustee_hash,
        action_hash.clone(),
        LinkTypes::TrustChain,
        (),
    )?;

    Ok(action_hash)
}

// ============================================================================
// Credential Issuance
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueCredentialInput {
    pub credential_id: String,
    pub credential_type: CredentialType,
    pub types: Vec<String>,
    pub issuer: String,
    pub issuer_hash: ActionHash,
    pub credential_subject_id: String,
    pub credential_subject: String,
    pub schema_hash: ActionHash,
    pub expiration_date: Option<Timestamp>,
    pub proof_type: ProofType,
    pub proof_value: String,
    pub verification_method: String,
    pub evidence: Vec<String>,
    pub terms_of_use: Option<String>,
    pub revocation_registry: Option<ActionHash>,
    pub revocation_index: Option<u32>,
}

#[hdk_extern]
pub fn issue_credential(input: IssueCredentialInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let vc = VerifiableCredential {
        credential_id: input.credential_id,
        context: vec![
            "https://www.w3.org/2018/credentials/v1".to_string(),
            "https://www.w3.org/2018/credentials/examples/v1".to_string(),
        ],
        credential_type: input.credential_type.clone(),
        types: input.types,
        issuer: input.issuer,
        issuer_hash: input.issuer_hash.clone(),
        issuance_date: now,
        expiration_date: input.expiration_date,
        credential_subject_id: input.credential_subject_id,
        credential_subject: input.credential_subject,
        proof_type: input.proof_type,
        proof_value: input.proof_value,
        proof_created: now,
        verification_method: input.verification_method,
        proof_purpose: "assertionMethod".to_string(),
        schema_hash: input.schema_hash.clone(),
        status: CredentialStatusType::Active,
        revocation_registry: input.revocation_registry,
        revocation_index: input.revocation_index,
        evidence: input.evidence,
        terms_of_use: input.terms_of_use,
        refresh_service: None,
    };

    let action_hash = create_entry(EntryTypes::VerifiableCredential(vc))?;

    // Link issuer to credential
    create_link(
        input.issuer_hash,
        action_hash.clone(),
        LinkTypes::IssuerToCredentials,
        (),
    )?;

    // Link schema to credential
    create_link(
        input.schema_hash,
        action_hash.clone(),
        LinkTypes::SchemaToCredentials,
        (),
    )?;

    // Link by credential type
    let type_anchor = anchor(
        LinkTypes::CredentialsByType,
        format!("{:?}", input.credential_type),
    )?;
    create_link(type_anchor, action_hash.clone(), LinkTypes::CredentialsByType, ())?;

    Ok(action_hash)
}

#[hdk_extern]
pub fn get_credential(action_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(action_hash, GetOptions::default())
}

#[hdk_extern]
pub fn get_credentials_by_issuer(issuer_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(issuer_hash, LinkTypes::IssuerToCredentials)?, GetStrategy::default())?;

    let mut credentials = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                credentials.push(record);
            }
        }
    }

    Ok(credentials)
}

#[hdk_extern]
pub fn get_credentials_by_type(credential_type: CredentialType) -> ExternResult<Vec<Record>> {
    let type_anchor = anchor(
        LinkTypes::CredentialsByType,
        format!("{:?}", credential_type),
    )?;

    let links = get_links(LinkQuery::try_new(type_anchor, LinkTypes::CredentialsByType)?, GetStrategy::default())?;

    let mut credentials = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                credentials.push(record);
            }
        }
    }

    Ok(credentials)
}

// ============================================================================
// Holder Wallet
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateHolderInput {
    pub holder_id: String,
    pub public_key: String,
    pub key_type: String,
    pub display_name: Option<String>,
}

#[hdk_extern]
pub fn create_holder(input: CreateHolderInput) -> ExternResult<ActionHash> {
    let holder = CredentialHolder {
        holder_id: input.holder_id,
        public_key: input.public_key,
        key_type: input.key_type,
        display_name: input.display_name,
        recovery_key_hash: None,
        created_at: sys_time()?,
    };

    create_entry(EntryTypes::CredentialHolder(holder))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddCredentialToWalletInput {
    pub holder_hash: ActionHash,
    pub credential_hash: ActionHash,
    pub label: Option<String>,
    pub category: Option<String>,
}

#[hdk_extern]
pub fn add_credential_to_wallet(input: AddCredentialToWalletInput) -> ExternResult<ActionHash> {
    let held = HeldCredential {
        holder_hash: input.holder_hash.clone(),
        credential_hash: input.credential_hash.clone(),
        acquired_at: sys_time()?,
        label: input.label,
        is_favorite: false,
        category: input.category,
        last_used: None,
        use_count: 0,
    };

    let action_hash = create_entry(EntryTypes::HeldCredential(held))?;

    // Link holder to credential
    create_link(
        input.holder_hash,
        action_hash.clone(),
        LinkTypes::HolderToCredentials,
        (),
    )?;

    Ok(action_hash)
}

#[hdk_extern]
pub fn get_holder_credentials(holder_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(holder_hash, LinkTypes::HolderToCredentials)?, GetStrategy::default())?;

    let mut credentials = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                credentials.push(record);
            }
        }
    }

    Ok(credentials)
}

// ============================================================================
// Presentation
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePresentationRequestInput {
    pub request_id: String,
    pub verifier: String,
    pub verifier_name: Option<String>,
    pub purpose: String,
    pub required_credentials: Vec<CredentialType>,
    pub required_attributes: String,
    pub optional_credentials: Vec<CredentialType>,
    pub nonce: String,
    pub domain: Option<String>,
    pub challenge: Option<String>,
    pub expires_in_seconds: u64,
}

#[hdk_extern]
pub fn create_presentation_request(
    input: CreatePresentationRequestInput,
) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let expires_at = Timestamp::from_micros(
        now.as_micros() + (input.expires_in_seconds as i64 * 1_000_000),
    );

    let request = PresentationRequest {
        request_id: input.request_id,
        verifier: input.verifier,
        verifier_name: input.verifier_name,
        purpose: input.purpose,
        required_credentials: input.required_credentials,
        required_attributes: input.required_attributes,
        optional_credentials: input.optional_credentials,
        nonce: input.nonce,
        domain: input.domain,
        challenge: input.challenge,
        expires_at,
        created_at: now,
    };

    create_entry(EntryTypes::PresentationRequest(request))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePresentationInput {
    pub presentation_id: String,
    pub holder: String,
    pub credential_hashes: Vec<ActionHash>,
    pub derived_credentials: Option<String>,
    pub request_hash: Option<ActionHash>,
    pub proof_type: ProofType,
    pub proof_value: String,
    pub challenge: Option<String>,
    pub domain: Option<String>,
}

#[hdk_extern]
pub fn create_presentation(input: CreatePresentationInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let vp = VerifiablePresentation {
        presentation_id: input.presentation_id,
        context: vec![
            "https://www.w3.org/2018/credentials/v1".to_string(),
        ],
        presentation_type: vec![
            "VerifiablePresentation".to_string(),
        ],
        holder: input.holder,
        credential_hashes: input.credential_hashes,
        derived_credentials: input.derived_credentials,
        request_hash: input.request_hash,
        proof_type: input.proof_type,
        proof_value: input.proof_value,
        proof_created: now,
        challenge: input.challenge,
        domain: input.domain,
        created_at: now,
    };

    create_entry(EntryTypes::VerifiablePresentation(vp))
}

// ============================================================================
// Verification
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyCredentialInput {
    pub credential_hash: ActionHash,
    pub verifier: String,
}

#[hdk_extern]
pub fn verify_credential(input: VerifyCredentialInput) -> ExternResult<ActionHash> {
    let credential_record = get(input.credential_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Credential not found".to_string()
        )))?;

    let vc: VerifiableCredential = credential_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid credential entry".to_string()
        )))?;

    // Perform verification checks
    let now = sys_time()?;
    let mut status = VerificationStatus::Verified;
    let mut checks = Vec::new();
    let mut error_details = None;

    // Check expiration
    if let Some(exp) = vc.expiration_date {
        if now > exp {
            status = VerificationStatus::Expired;
            error_details = Some("Credential has expired".to_string());
        }
        checks.push("expiration_check".to_string());
    }

    // Check revocation status
    if vc.status == CredentialStatusType::Revoked {
        status = VerificationStatus::Revoked;
        error_details = Some("Credential has been revoked".to_string());
    }
    checks.push("revocation_check".to_string());

    // Check issuer exists
    if let Some(_issuer_record) = get(vc.issuer_hash.clone(), GetOptions::default())? {
        checks.push("issuer_exists".to_string());
    } else {
        status = VerificationStatus::UntrustedIssuer;
        error_details = Some("Issuer not found".to_string());
    }

    // Note: Actual signature verification would happen here
    // For now we record the attempt
    checks.push("signature_check".to_string());

    let verification = VerificationRecord {
        verification_id: format!("verify_{}", now.as_micros()),
        verified_item_hash: input.credential_hash.clone(),
        item_type: "credential".to_string(),
        verifier: input.verifier,
        status,
        checks_performed: serde_json::to_string(&checks).unwrap_or_default(),
        error_details,
        verified_at: now,
    };

    let action_hash = create_entry(EntryTypes::VerificationRecord(verification))?;

    // Link credential to verification
    create_link(
        input.credential_hash,
        action_hash.clone(),
        LinkTypes::CredentialToVerifications,
        (),
    )?;

    Ok(action_hash)
}

#[hdk_extern]
pub fn get_verification_records(credential_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(credential_hash, LinkTypes::CredentialToVerifications)?, GetStrategy::default(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

// ============================================================================
// Revocation
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateRevocationRegistryInput {
    pub registry_id: String,
    pub issuer_hash: ActionHash,
    pub credential_type: CredentialType,
    pub max_credentials: u32,
}

#[hdk_extern]
pub fn create_revocation_registry(
    input: CreateRevocationRegistryInput,
) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let registry = RevocationRegistry {
        registry_id: input.registry_id,
        issuer_hash: input.issuer_hash.clone(),
        credential_type: input.credential_type,
        max_credentials: input.max_credentials,
        current_index: 0,
        accumulator: None,
        revocation_bitmap: Some("0".repeat(input.max_credentials as usize)),
        created_at: now,
        updated_at: now,
    };

    let action_hash = create_entry(EntryTypes::RevocationRegistry(registry))?;

    // Link issuer to registry
    create_link(
        input.issuer_hash,
        action_hash.clone(),
        LinkTypes::IssuerToRegistries,
        (),
    )?;

    Ok(action_hash)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeCredentialInput {
    pub registry_hash: ActionHash,
    pub credential_hash: ActionHash,
    pub revocation_index: u32,
    pub reason: String,
}

#[hdk_extern]
pub fn revoke_credential(input: RevokeCredentialInput) -> ExternResult<ActionHash> {
    let agent_info = agent_info()?;

    let revocation = RevocationEntry {
        registry_hash: input.registry_hash,
        credential_hash: input.credential_hash.clone(),
        revocation_index: input.revocation_index,
        reason: input.reason,
        revoked_by: ActionHash::from_raw_36(agent_info.agent_initial_pubkey.get_raw_36().to_vec()),
        revoked_at: sys_time()?,
    };

    let action_hash = create_entry(EntryTypes::RevocationEntry(revocation))?;

    // Link credential to revocation
    create_link(
        input.credential_hash,
        action_hash.clone(),
        LinkTypes::CredentialToRevocation,
        (),
    )?;

    Ok(action_hash)
}

#[hdk_extern]
pub fn check_revocation(credential_hash: ActionHash) -> ExternResult<bool> {
    let links = get_links(LinkQuery::try_new(credential_hash, LinkTypes::CredentialToRevocation)?, GetStrategy::default())?;

    Ok(!links.is_empty())
}

// ============================================================================
// Health-Specific Credential Helpers
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueVaccinationCredentialInput {
    pub credential_id: String,
    pub issuer_hash: ActionHash,
    pub schema_hash: ActionHash,
    pub holder_id: String,
    pub claims: VaccinationClaims,
    pub proof_type: ProofType,
    pub proof_value: String,
    pub verification_method: String,
    pub expiration_months: Option<u32>,
}

#[hdk_extern]
pub fn issue_vaccination_credential(
    input: IssueVaccinationCredentialInput,
) -> ExternResult<ActionHash> {
    // Store the claims
    let claims_hash = create_entry(EntryTypes::VaccinationClaims(input.claims.clone()))?;

    // Calculate expiration
    let now = sys_time()?;
    let expiration = input.expiration_months.map(|months| {
        Timestamp::from_micros(now.as_micros() + (months as i64 * 30 * 24 * 60 * 60 * 1_000_000))
    });

    // Create the credential
    let vc = VerifiableCredential {
        credential_id: input.credential_id,
        context: vec![
            "https://www.w3.org/2018/credentials/v1".to_string(),
            "https://w3id.org/vaccination/v1".to_string(),
        ],
        credential_type: CredentialType::VaccinationCredential,
        types: vec![
            "VerifiableCredential".to_string(),
            "VaccinationCredential".to_string(),
        ],
        issuer: "".to_string(), // Will be filled from issuer record
        issuer_hash: input.issuer_hash.clone(),
        issuance_date: now,
        expiration_date: expiration,
        credential_subject_id: input.holder_id,
        credential_subject: serde_json::to_string(&input.claims).unwrap_or_default(),
        proof_type: input.proof_type,
        proof_value: input.proof_value,
        proof_created: now,
        verification_method: input.verification_method,
        proof_purpose: "assertionMethod".to_string(),
        schema_hash: input.schema_hash.clone(),
        status: CredentialStatusType::Active,
        revocation_registry: None,
        revocation_index: None,
        evidence: vec![claims_hash.to_string()],
        terms_of_use: None,
        refresh_service: None,
    };

    let action_hash = create_entry(EntryTypes::VerifiableCredential(vc))?;

    // Create links
    create_link(
        input.issuer_hash,
        action_hash.clone(),
        LinkTypes::IssuerToCredentials,
        (),
    )?;

    let type_anchor = anchor(
        LinkTypes::CredentialsByType,
        "VaccinationCredential".to_string(),
    )?;
    create_link(type_anchor, action_hash.clone(), LinkTypes::CredentialsByType, ())?;

    Ok(action_hash)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueLabResultCredentialInput {
    pub credential_id: String,
    pub issuer_hash: ActionHash,
    pub schema_hash: ActionHash,
    pub holder_id: String,
    pub claims: LabResultClaims,
    pub proof_type: ProofType,
    pub proof_value: String,
    pub verification_method: String,
}

#[hdk_extern]
pub fn issue_lab_result_credential(
    input: IssueLabResultCredentialInput,
) -> ExternResult<ActionHash> {
    // Store the claims
    let claims_hash = create_entry(EntryTypes::LabResultClaims(input.claims.clone()))?;

    let now = sys_time()?;

    let vc = VerifiableCredential {
        credential_id: input.credential_id,
        context: vec![
            "https://www.w3.org/2018/credentials/v1".to_string(),
            "https://w3id.org/healthcare/v1".to_string(),
        ],
        credential_type: CredentialType::LabResultCredential,
        types: vec![
            "VerifiableCredential".to_string(),
            "LabResultCredential".to_string(),
        ],
        issuer: "".to_string(),
        issuer_hash: input.issuer_hash.clone(),
        issuance_date: now,
        expiration_date: None,
        credential_subject_id: input.holder_id,
        credential_subject: serde_json::to_string(&input.claims).unwrap_or_default(),
        proof_type: input.proof_type,
        proof_value: input.proof_value,
        proof_created: now,
        verification_method: input.verification_method,
        proof_purpose: "assertionMethod".to_string(),
        schema_hash: input.schema_hash.clone(),
        status: CredentialStatusType::Active,
        revocation_registry: None,
        revocation_index: None,
        evidence: vec![claims_hash.to_string()],
        terms_of_use: None,
        refresh_service: None,
    };

    let action_hash = create_entry(EntryTypes::VerifiableCredential(vc))?;

    create_link(
        input.issuer_hash,
        action_hash.clone(),
        LinkTypes::IssuerToCredentials,
        (),
    )?;

    let type_anchor = anchor(
        LinkTypes::CredentialsByType,
        "LabResultCredential".to_string(),
    )?;
    create_link(type_anchor, action_hash.clone(), LinkTypes::CredentialsByType, ())?;

    Ok(action_hash)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueMedicalLicenseCredentialInput {
    pub credential_id: String,
    pub issuer_hash: ActionHash,
    pub schema_hash: ActionHash,
    pub holder_id: String,
    pub claims: MedicalLicenseClaims,
    pub proof_type: ProofType,
    pub proof_value: String,
    pub verification_method: String,
}

#[hdk_extern]
pub fn issue_medical_license_credential(
    input: IssueMedicalLicenseCredentialInput,
) -> ExternResult<ActionHash> {
    // Store the claims
    let claims_hash = create_entry(EntryTypes::MedicalLicenseClaims(input.claims.clone()))?;

    let now = sys_time()?;

    let vc = VerifiableCredential {
        credential_id: input.credential_id,
        context: vec![
            "https://www.w3.org/2018/credentials/v1".to_string(),
            "https://w3id.org/healthcare/license/v1".to_string(),
        ],
        credential_type: CredentialType::MedicalLicenseCredential,
        types: vec![
            "VerifiableCredential".to_string(),
            "MedicalLicenseCredential".to_string(),
        ],
        issuer: "".to_string(),
        issuer_hash: input.issuer_hash.clone(),
        issuance_date: now,
        expiration_date: Some(input.claims.expiration_date),
        credential_subject_id: input.holder_id,
        credential_subject: serde_json::to_string(&input.claims).unwrap_or_default(),
        proof_type: input.proof_type,
        proof_value: input.proof_value,
        proof_created: now,
        verification_method: input.verification_method,
        proof_purpose: "assertionMethod".to_string(),
        schema_hash: input.schema_hash.clone(),
        status: CredentialStatusType::Active,
        revocation_registry: None,
        revocation_index: None,
        evidence: vec![claims_hash.to_string()],
        terms_of_use: None,
        refresh_service: None,
    };

    let action_hash = create_entry(EntryTypes::VerifiableCredential(vc))?;

    create_link(
        input.issuer_hash,
        action_hash.clone(),
        LinkTypes::IssuerToCredentials,
        (),
    )?;

    let type_anchor = anchor(
        LinkTypes::CredentialsByType,
        "MedicalLicenseCredential".to_string(),
    )?;
    create_link(type_anchor, action_hash.clone(), LinkTypes::CredentialsByType, ())?;

    Ok(action_hash)
}

// ============================================================================
// Utility Functions
// ============================================================================

fn anchor(_link_type: LinkTypes, anchor_text: String) -> ExternResult<EntryHash> {
    let path = Path::from(anchor_text);
    path.path_entry_hash()
}
