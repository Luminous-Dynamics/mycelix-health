//! Insurance Coordinator Zome
//!
//! Provides extern functions for insurance plan management,
//! claims processing, and prior authorization workflows.

use hdk::prelude::*;
use insurance_integrity::*;

/// Register an insurance plan for a patient
#[hdk_extern]
pub fn register_insurance_plan(plan: InsurancePlan) -> ExternResult<Record> {
    let plan_hash = create_entry(&EntryTypes::InsurancePlan(plan.clone()))?;
    let record = get(plan_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find plan".to_string())
    ))?;

    create_link(plan.patient_hash, plan_hash, LinkTypes::PatientToPlans, ())?;

    Ok(record)
}

/// Get patient's insurance plans
#[hdk_extern]
pub fn get_patient_insurance(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToPlans)?,
        GetStrategy::default(),
    )?;

    let mut plans = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                plans.push(record);
            }
        }
    }

    // Sort by coordination order
    plans.sort_by(|a, b| {
        let a_order = a
            .entry()
            .to_app_option::<InsurancePlan>()
            .ok()
            .flatten()
            .map(|p| p.coordination_order)
            .unwrap_or(255);
        let b_order = b
            .entry()
            .to_app_option::<InsurancePlan>()
            .ok()
            .flatten()
            .map(|p| p.coordination_order)
            .unwrap_or(255);
        a_order.cmp(&b_order)
    });

    Ok(plans)
}

/// Update insurance plan
#[hdk_extern]
pub fn update_insurance_plan(input: UpdatePlanInput) -> ExternResult<Record> {
    let updated_hash = update_entry(input.original_hash, &input.updated_plan)?;
    get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Could not find updated plan".to_string()
    )))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdatePlanInput {
    pub original_hash: ActionHash,
    pub updated_plan: InsurancePlan,
}

/// Submit a claim
#[hdk_extern]
pub fn submit_claim(claim: Claim) -> ExternResult<Record> {
    let claim_hash = create_entry(&EntryTypes::Claim(claim.clone()))?;
    let record = get(claim_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find claim".to_string())
    ))?;

    // Link to patient
    create_link(
        claim.patient_hash.clone(),
        claim_hash.clone(),
        LinkTypes::PatientToClaims,
        (),
    )?;

    // Link to plan
    create_link(
        claim.plan_hash.clone(),
        claim_hash.clone(),
        LinkTypes::PlanToClaims,
        (),
    )?;

    // Link to encounter
    create_link(
        claim.encounter_hash,
        claim_hash.clone(),
        LinkTypes::EncounterToClaim,
        (),
    )?;

    // Track pending claims
    let pending_anchor = anchor_hash("pending_claims")?;
    create_link(pending_anchor, claim_hash, LinkTypes::PendingClaims, ())?;

    Ok(record)
}

/// Get patient's claims
#[hdk_extern]
pub fn get_patient_claims(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToClaims)?,
        GetStrategy::default(),
    )?;

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

/// Update claim status (adjudication result)
#[hdk_extern]
pub fn update_claim_status(input: UpdateClaimInput) -> ExternResult<Record> {
    let record = get(input.claim_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Claim not found".to_string())
    ))?;

    let mut claim: Claim = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid claim".to_string()
        )))?;

    claim.status = input.new_status.clone();
    claim.adjudication_date = Some(sys_time()?);
    claim.total_allowed = input.allowed_amount;
    claim.total_paid = input.paid_amount;
    claim.patient_responsibility = input.patient_responsibility;
    claim.payer_claim_number = input.payer_claim_number;

    let updated_hash = update_entry(input.claim_hash.clone(), &claim)?;

    // Track denied claims
    if matches!(input.new_status, ClaimStatus::Denied) {
        let denied_anchor = anchor_hash("denied_claims")?;
        create_link(
            denied_anchor,
            updated_hash.clone(),
            LinkTypes::DeniedClaims,
            (),
        )?;
    }

    get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Could not find updated claim".to_string()
    )))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateClaimInput {
    pub claim_hash: ActionHash,
    pub new_status: ClaimStatus,
    pub allowed_amount: Option<f64>,
    pub paid_amount: Option<f64>,
    pub patient_responsibility: Option<f64>,
    pub payer_claim_number: Option<String>,
}

/// Submit prior authorization request
#[hdk_extern]
pub fn submit_prior_auth(auth: PriorAuthorization) -> ExternResult<Record> {
    let auth_hash = create_entry(&EntryTypes::PriorAuthorization(auth.clone()))?;
    let record = get(auth_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find authorization".to_string())
    ))?;

    create_link(
        auth.patient_hash,
        auth_hash.clone(),
        LinkTypes::PatientToAuths,
        (),
    )?;

    let pending_anchor = anchor_hash("pending_auths")?;
    create_link(pending_anchor, auth_hash, LinkTypes::PendingAuths, ())?;

    Ok(record)
}

/// Get patient's prior authorizations
#[hdk_extern]
pub fn get_patient_authorizations(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToAuths)?,
        GetStrategy::default(),
    )?;

    let mut auths = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                auths.push(record);
            }
        }
    }

    Ok(auths)
}

/// Update authorization decision
#[hdk_extern]
pub fn update_authorization(input: UpdateAuthInput) -> ExternResult<Record> {
    let record = get(input.auth_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Authorization not found".to_string())
    ))?;

    let mut auth: PriorAuthorization = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid authorization".to_string()
        )))?;

    auth.status = input.new_status;
    auth.decision_at = Some(sys_time()?);
    auth.decision_by = input.decision_by;
    auth.auth_number = input.auth_number;
    auth.effective_date = input.effective_date;
    auth.expiration_date = input.expiration_date;
    auth.denial_reason = input.denial_reason;

    let updated_hash = update_entry(input.auth_hash, &auth)?;
    get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Could not find updated authorization".to_string()
    )))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateAuthInput {
    pub auth_hash: ActionHash,
    pub new_status: AuthStatus,
    pub decision_by: Option<String>,
    pub auth_number: Option<String>,
    pub effective_date: Option<String>,
    pub expiration_date: Option<String>,
    pub denial_reason: Option<String>,
}

/// Check eligibility
#[hdk_extern]
pub fn check_eligibility(check: EligibilityCheck) -> ExternResult<Record> {
    let check_hash = create_entry(&EntryTypes::EligibilityCheck(check))?;
    get(check_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Could not find eligibility check".to_string()
    )))
}

/// Create explanation of benefits
#[hdk_extern]
pub fn create_eob(eob: ExplanationOfBenefits) -> ExternResult<Record> {
    let eob_hash = create_entry(&EntryTypes::ExplanationOfBenefits(eob.clone()))?;
    let record = get(eob_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find EOB".to_string())
    ))?;

    create_link(eob.claim_hash, eob_hash, LinkTypes::ClaimToEOB, ())?;

    Ok(record)
}

/// Get EOB for a claim
#[hdk_extern]
pub fn get_claim_eob(claim_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(
        LinkQuery::try_new(claim_hash, LinkTypes::ClaimToEOB)?,
        GetStrategy::default(),
    )?;

    if let Some(link) = links.first() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            return get(hash, GetOptions::default());
        }
    }

    Ok(None)
}

/// Get pending claims
#[hdk_extern]
pub fn get_pending_claims(_: ()) -> ExternResult<Vec<Record>> {
    let pending_anchor = anchor_hash("pending_claims")?;
    let links = get_links(
        LinkQuery::try_new(pending_anchor, LinkTypes::PendingClaims)?,
        GetStrategy::default(),
    )?;

    let mut claims = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(claim) = record.entry().to_app_option::<Claim>().ok().flatten() {
                    if matches!(
                        claim.status,
                        ClaimStatus::Submitted | ClaimStatus::Pending | ClaimStatus::InProcess
                    ) {
                        claims.push(record);
                    }
                }
            }
        }
    }

    Ok(claims)
}

/// Anchor entry for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}
