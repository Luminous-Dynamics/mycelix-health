//! Mycelix Health Bridge Coordinator Zome
//! 
//! Provides extern functions for cross-hApp communication,
//! data federation, and reputation integration.

use hdk::prelude::*;
use bridge_integrity::*;

/// Register this hApp with the Mycelix bridge
#[hdk_extern]
pub fn register_with_bridge(input: RegisterBridgeInput) -> ExternResult<Record> {
    let registration = HealthBridgeRegistration {
        registration_id: input.registration_id,
        mycelix_identity_hash: input.mycelix_identity_hash,
        happ_id: "mycelix-health".to_string(),
        capabilities: input.capabilities,
        federated_data: input.federated_data,
        minimum_trust_score: input.minimum_trust_score,
        registered_at: sys_time()?,
        status: BridgeStatus::Active,
    };
    
    let reg_hash = create_entry(&EntryTypes::HealthBridgeRegistration(registration.clone()))?;
    let record = get(reg_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find registration".to_string())))?;
    
    // Link to identity
    create_link(
        registration.mycelix_identity_hash,
        reg_hash.clone(),
        LinkTypes::IdentityToRegistrations,
        (),
    )?;
    
    // Add to active registrations
    let active_anchor = anchor_hash("active_registrations")?;
    create_link(
        active_anchor,
        reg_hash,
        LinkTypes::ActiveRegistrations,
        (),
    )?;
    
    Ok(record)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterBridgeInput {
    pub registration_id: String,
    pub mycelix_identity_hash: ActionHash,
    pub capabilities: Vec<HealthCapability>,
    pub federated_data: Vec<FederatedDataType>,
    pub minimum_trust_score: f64,
}

/// Query health data from another hApp
#[hdk_extern]
pub fn query_federated_data(query: HealthDataQuery) -> ExternResult<Record> {
    let query_hash = create_entry(&EntryTypes::HealthDataQuery(query.clone()))?;
    let record = get(query_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find query".to_string())))?;
    
    let pending_anchor = anchor_hash("pending_queries")?;
    create_link(
        pending_anchor,
        query_hash,
        LinkTypes::PendingQueries,
        (),
    )?;
    
    Ok(record)
}

/// Respond to a health data query
#[hdk_extern]
pub fn respond_to_query(response: HealthDataResponse) -> ExternResult<Record> {
    let response_hash = create_entry(&EntryTypes::HealthDataResponse(response.clone()))?;
    let record = get(response_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find response".to_string())))?;
    
    create_link(
        response.query_hash,
        response_hash,
        LinkTypes::QueryToResponses,
        (),
    )?;
    
    Ok(record)
}

/// Get responses for a query
#[hdk_extern]
pub fn get_query_responses(query_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(query_hash, LinkTypes::QueryToResponses)?, GetStrategy::default())?;
    
    let mut responses = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                responses.push(record);
            }
        }
    }
    
    Ok(responses)
}

/// Request provider verification
#[hdk_extern]
pub fn request_provider_verification(request: ProviderVerificationRequest) -> ExternResult<Record> {
    let request_hash = create_entry(&EntryTypes::ProviderVerificationRequest(request.clone()))?;
    let record = get(request_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find request".to_string())))?;
    
    create_link(
        request.provider_hash,
        request_hash,
        LinkTypes::ProviderToVerifications,
        (),
    )?;
    
    Ok(record)
}

/// Submit verification result
#[hdk_extern]
pub fn submit_verification_result(result: ProviderVerificationResult) -> ExternResult<Record> {
    let result_hash = create_entry(&EntryTypes::ProviderVerificationResult(result.clone()))?;
    let record = get(result_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find result".to_string())))?;
    
    // Update the request with result
    if let Some(request_record) = get(result.request_hash.clone(), GetOptions::default())? {
        if let Some(mut request) = request_record.entry().to_app_option::<ProviderVerificationRequest>().ok().flatten() {
            request.status = VerificationStatus::Verified;
            request.result_hash = Some(result_hash.clone());
            update_entry(result.request_hash, &request)?;
        }
    }
    
    Ok(record)
}

/// Create health epistemic claim
#[hdk_extern]
pub fn create_epistemic_claim(claim: HealthEpistemicClaim) -> ExternResult<Record> {
    let claim_hash = create_entry(&EntryTypes::HealthEpistemicClaim(claim.clone()))?;
    let record = get(claim_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find claim".to_string())))?;
    
    create_link(
        claim.subject_hash,
        claim_hash.clone(),
        LinkTypes::EntityToClaims,
        (),
    )?;
    
    // Link by claim type
    let type_anchor = anchor_hash(&format!("claims_{:?}", claim.claim_type))?;
    create_link(
        type_anchor,
        claim_hash,
        LinkTypes::ClaimsByType,
        (),
    )?;
    
    Ok(record)
}

/// Get claims for an entity (patient, provider, trial, etc.)
#[hdk_extern]
pub fn get_entity_claims(entity_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(entity_hash, LinkTypes::EntityToClaims)?, GetStrategy::default())?;
    
    let mut claims = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                claims.push(record);
            }
        }
    }
    
    Ok(claims)
}

/// Verify an epistemic claim
#[hdk_extern]
pub fn verify_claim(input: VerifyClaimInput) -> ExternResult<Record> {
    let record = get(input.claim_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Claim not found".to_string())))?;
    
    let mut claim: HealthEpistemicClaim = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid claim".to_string())))?;
    
    claim.verified = true;
    claim.verified_by.push(agent_info()?.agent_initial_pubkey);
    
    let updated_hash = update_entry(input.claim_hash, &claim)?;
    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated claim".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyClaimInput {
    pub claim_hash: ActionHash,
}

/// Update federated reputation
#[hdk_extern]
pub fn update_federated_reputation(federation: HealthReputationFederation) -> ExternResult<Record> {
    let fed_hash = create_entry(&EntryTypes::HealthReputationFederation(federation.clone()))?;
    let record = get(fed_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find federation".to_string())))?;
    
    create_link(
        federation.entity_hash,
        fed_hash,
        LinkTypes::EntityToReputation,
        (),
    )?;
    
    Ok(record)
}

/// Get entity's federated reputation
#[hdk_extern]
pub fn get_federated_reputation(entity_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(LinkQuery::try_new(entity_hash, LinkTypes::EntityToReputation)?, GetStrategy::default())?;
    
    // Get the most recent federation record
    let mut latest: Option<(Timestamp, ActionHash)> = None;
    
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash.clone(), GetOptions::default())? {
                if let Some(fed) = record.entry().to_app_option::<HealthReputationFederation>().ok().flatten() {
                    match &latest {
                        None => latest = Some((fed.aggregated_at, hash)),
                        Some((ts, _)) if fed.aggregated_at > *ts => {
                            latest = Some((fed.aggregated_at, hash))
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    
    if let Some((_, hash)) = latest {
        return get(hash, GetOptions::default());
    }
    
    Ok(None)
}

/// Aggregate reputation from multiple sources
#[hdk_extern]
pub fn aggregate_reputation(input: AggregateReputationInput) -> ExternResult<HealthReputationFederation> {
    let total_weight: f64 = input.scores.iter().map(|s| s.weight).sum();
    
    let weighted_sum: f64 = input.scores
        .iter()
        .map(|s| s.score * s.weight)
        .sum();
    
    let aggregated_score = if total_weight > 0.0 {
        weighted_sum / total_weight
    } else {
        0.5 // Default neutral score
    };
    
    let federation = HealthReputationFederation {
        federation_id: input.federation_id,
        entity_hash: input.entity_hash,
        entity_type: input.entity_type,
        scores: input.scores,
        aggregated_score,
        aggregated_at: sys_time()?,
    };
    
    Ok(federation)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AggregateReputationInput {
    pub federation_id: String,
    pub entity_hash: ActionHash,
    pub entity_type: HealthEntityType,
    pub scores: Vec<FederatedScore>,
}

/// Get active bridge registrations
#[hdk_extern]
pub fn get_active_registrations(_: ()) -> ExternResult<Vec<Record>> {
    let active_anchor = anchor_hash("active_registrations")?;
    let links = get_links(LinkQuery::try_new(active_anchor, LinkTypes::ActiveRegistrations)?, GetStrategy::default())?;
    
    let mut registrations = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                registrations.push(record);
            }
        }
    }
    
    Ok(registrations)
}

/// Anchor entry for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}
