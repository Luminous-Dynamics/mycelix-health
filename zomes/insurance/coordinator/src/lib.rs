#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Insurance Coordinator Zome
//! 
//! Provides extern functions for insurance plan management,
//! claims processing, and prior authorization workflows.

use hdk::prelude::*;
use insurance_integrity::*;
use mycelix_health_shared::{
    require_authorization, require_admin_authorization,
    log_data_access,
    DataCategory, Permission,
};

/// Register an insurance plan for a patient
#[hdk_extern]
pub fn register_insurance_plan(plan: InsurancePlan) -> ExternResult<Record> {
    let auth = require_authorization(
        plan.patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Write,
        false,
    )?;
    let plan_hash = create_entry(&EntryTypes::InsurancePlan(plan.clone()))?;
    let record = get(plan_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find plan".to_string())))?;
    
    create_link(
        plan.patient_hash.clone(),
        plan_hash,
        LinkTypes::PatientToPlans,
        (),
    )?;

    log_data_access(
        plan.patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;
    
    Ok(record)
}

/// Get patient's insurance plans
#[hdk_extern]
pub fn get_patient_insurance(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Read,
        false,
    )?;
    let links = get_links(LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToPlans)?, GetStrategy::default())?;
    
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
        let a_order = a.entry().to_app_option::<InsurancePlan>().ok().flatten()
            .map(|p| p.coordination_order).unwrap_or(255);
        let b_order = b.entry().to_app_option::<InsurancePlan>().ok().flatten()
            .map(|p| p.coordination_order).unwrap_or(255);
        a_order.cmp(&b_order)
    });

    if !plans.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::FinancialData],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }
    
    Ok(plans)
}

/// Update insurance plan
#[hdk_extern]
pub fn update_insurance_plan(input: UpdatePlanInput) -> ExternResult<Record> {
    let record = get(input.original_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Plan not found".to_string())))?;

    let existing: InsurancePlan = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid plan entry".to_string())))?;

    if existing.patient_hash != input.updated_plan.patient_hash {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Cannot change patient_hash on an insurance plan".to_string()
        )));
    }

    let auth = require_authorization(
        input.updated_plan.patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Amend,
        false,
    )?;

    let updated_hash = update_entry(input.original_hash, &input.updated_plan)?;

    let updated_record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated plan".to_string())))?;

    log_data_access(
        input.updated_plan.patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Amend,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(updated_record)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdatePlanInput {
    pub original_hash: ActionHash,
    pub updated_plan: InsurancePlan,
}

/// Submit a claim
#[hdk_extern]
pub fn submit_claim(claim: Claim) -> ExternResult<Record> {
    let auth = require_authorization(
        claim.patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Write,
        false,
    )?;
    let claim_hash = create_entry(&EntryTypes::Claim(claim.clone()))?;
    let record = get(claim_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find claim".to_string())))?;
    
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
    create_link(
        pending_anchor,
        claim_hash,
        LinkTypes::PendingClaims,
        (),
    )?;

    log_data_access(
        claim.patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;
    
    Ok(record)
}

/// Get patient's claims
#[hdk_extern]
pub fn get_patient_claims(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Read,
        false,
    )?;
    let links = get_links(LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToClaims)?, GetStrategy::default())?;
    
    let mut claims = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                claims.push(record);
            }
        }
    }

    if !claims.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::FinancialData],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }
    
    Ok(claims)
}

/// Update claim status (adjudication result)
#[hdk_extern]
pub fn update_claim_status(input: UpdateClaimInput) -> ExternResult<Record> {
    let record = get(input.claim_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Claim not found".to_string())))?;
    
    let mut claim: Claim = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid claim".to_string())))?;

    let authz = require_authorization(
        claim.patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Amend,
        false,
    )?;
    
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
    
    let updated_record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated claim".to_string())))?;

    log_data_access(
        claim.patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Amend,
        authz.consent_hash,
        authz.emergency_override,
        None,
    )?;

    Ok(updated_record)
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
    let patient_hash = auth.patient_hash.clone();
    let authz = require_authorization(
        patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Write,
        false,
    )?;
    let auth_hash = create_entry(&EntryTypes::PriorAuthorization(auth.clone()))?;
    let record = get(auth_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find authorization".to_string())))?;
    
    create_link(
        patient_hash.clone(),
        auth_hash.clone(),
        LinkTypes::PatientToAuths,
        (),
    )?;
    
    let pending_anchor = anchor_hash("pending_auths")?;
    create_link(
        pending_anchor,
        auth_hash,
        LinkTypes::PendingAuths,
        (),
    )?;

    log_data_access(
        patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Write,
        authz.consent_hash,
        authz.emergency_override,
        None,
    )?;
    
    Ok(record)
}

/// Get patient's prior authorizations
#[hdk_extern]
pub fn get_patient_authorizations(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Read,
        false,
    )?;
    let links = get_links(LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToAuths)?, GetStrategy::default())?;
    
    let mut auths = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                auths.push(record);
            }
        }
    }

    if !auths.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::FinancialData],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }
    
    Ok(auths)
}

/// Update authorization decision
#[hdk_extern]
pub fn update_authorization(input: UpdateAuthInput) -> ExternResult<Record> {
    let record = get(input.auth_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Authorization not found".to_string())))?;
    
    let mut auth: PriorAuthorization = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid authorization".to_string())))?;

    let authz = require_authorization(
        auth.patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Amend,
        false,
    )?;
    
    auth.status = input.new_status;
    auth.decision_at = Some(sys_time()?);
    auth.decision_by = input.decision_by;
    auth.auth_number = input.auth_number;
    auth.effective_date = input.effective_date;
    auth.expiration_date = input.expiration_date;
    auth.denial_reason = input.denial_reason;
    
    let updated_hash = update_entry(input.auth_hash, &auth)?;

    let updated_record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated authorization".to_string())))?;

    log_data_access(
        auth.patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Amend,
        authz.consent_hash,
        authz.emergency_override,
        None,
    )?;

    Ok(updated_record)
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
    let patient_hash = check.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Read,
        false,
    )?;
    let check_hash = create_entry(&EntryTypes::EligibilityCheck(check))?;
    let record = get(check_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find eligibility check".to_string())))?;

    log_data_access(
        patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Create explanation of benefits
#[hdk_extern]
pub fn create_eob(eob: ExplanationOfBenefits) -> ExternResult<Record> {
    let auth = require_authorization(
        eob.patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Write,
        false,
    )?;
    let eob_hash = create_entry(&EntryTypes::ExplanationOfBenefits(eob.clone()))?;
    let record = get(eob_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find EOB".to_string())))?;
    
    create_link(
        eob.claim_hash,
        eob_hash,
        LinkTypes::ClaimToEOB,
        (),
    )?;

    log_data_access(
        eob.patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;
    
    Ok(record)
}

/// Get EOB for a claim
#[hdk_extern]
pub fn get_claim_eob(claim_hash: ActionHash) -> ExternResult<Option<Record>> {
    let claim_record = get(claim_hash.clone(), GetOptions::default())?;
    let rec = match claim_record {
        Some(record) => record,
        None => return Ok(None),
    };

    let claim: Claim = rec
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid claim".to_string())))?;

    let auth = require_authorization(
        claim.patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(claim_hash, LinkTypes::ClaimToEOB)?,
        GetStrategy::default(),
    )?;

    if let Some(link) = links.first() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            let record = get(hash, GetOptions::default())?;
            if record.is_some() {
                log_data_access(
                    claim.patient_hash,
                    vec![DataCategory::FinancialData],
                    Permission::Read,
                    auth.consent_hash,
                    auth.emergency_override,
                    None,
                )?;
            }
            return Ok(record);
        }
    }

    Ok(None)
}

/// Get pending claims
#[hdk_extern]
pub fn get_pending_claims(_: ()) -> ExternResult<Vec<Record>> {
    // Bulk cross-patient claim access is restricted to admins.
    require_admin_authorization()?;
    let pending_anchor = anchor_hash("pending_claims")?;
    let links = get_links(LinkQuery::try_new(pending_anchor, LinkTypes::PendingClaims)?, GetStrategy::default())?;
    
    let mut claims = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(claim) = record.entry().to_app_option::<Claim>().ok().flatten() {
                    if matches!(claim.status, ClaimStatus::Submitted | ClaimStatus::Pending | ClaimStatus::InProcess) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_hash() -> ActionHash {
        ActionHash::from_raw_36(vec![0u8; 36])
    }

    // ==================== Serde roundtrip tests ====================

    #[test]
    fn test_serde_roundtrip_update_plan_input() {
        let input = UpdatePlanInput {
            original_hash: dummy_hash(),
            updated_plan: InsurancePlan {
                plan_id: "PLAN-001".to_string(),
                patient_hash: dummy_hash(),
                payer_name: "BlueCross".to_string(),
                payer_id: "BC-001".to_string(),
                member_id: "MEM-12345".to_string(),
                group_number: Some("GRP-100".to_string()),
                plan_type: PlanType::PPO,
                coverage_type: CoverageType::Medical,
                relationship: SubscriberRelationship::Self_,
                subscriber_name: None,
                subscriber_dob: None,
                subscriber_id: None,
                effective_date: "2025-01-01".to_string(),
                termination_date: None,
                status: PlanStatus::Active,
                deductible: Some(1500.0),
                deductible_met: Some(500.0),
                out_of_pocket_max: Some(6000.0),
                out_of_pocket_met: Some(800.0),
                copay_primary: Some(25.0),
                copay_specialist: Some(50.0),
                copay_emergency: Some(250.0),
                coinsurance_percent: Some(20.0),
                coordination_order: 1,
                matl_trust_score: 0.9,
                created_at: Timestamp::from_micros(0),
                updated_at: Timestamp::from_micros(0),
            },
        };
        let json = serde_json::to_string(&input).expect("serialize");
        let decoded: UpdatePlanInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.updated_plan.plan_id, "PLAN-001");
        assert_eq!(decoded.updated_plan.payer_name, "BlueCross");
        assert_eq!(decoded.updated_plan.coordination_order, 1);
    }

    #[test]
    fn test_serde_roundtrip_update_claim_input() {
        let input = UpdateClaimInput {
            claim_hash: dummy_hash(),
            new_status: ClaimStatus::Approved,
            allowed_amount: Some(1200.0),
            paid_amount: Some(960.0),
            patient_responsibility: Some(240.0),
            payer_claim_number: Some("PCN-99999".to_string()),
        };
        let json = serde_json::to_string(&input).expect("serialize");
        let decoded: UpdateClaimInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.new_status, ClaimStatus::Approved);
        assert!((decoded.allowed_amount.unwrap() - 1200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_serde_roundtrip_update_auth_input() {
        let input = UpdateAuthInput {
            auth_hash: dummy_hash(),
            new_status: AuthStatus::Approved,
            decision_by: Some("Dr. Smith".to_string()),
            auth_number: Some("AUTH-001".to_string()),
            effective_date: Some("2025-03-01".to_string()),
            expiration_date: Some("2025-06-01".to_string()),
            denial_reason: None,
        };
        let json = serde_json::to_string(&input).expect("serialize");
        let decoded: UpdateAuthInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.new_status, AuthStatus::Approved);
        assert_eq!(decoded.auth_number, Some("AUTH-001".to_string()));
    }

    // ==================== Plan type variants ====================

    #[test]
    fn test_plan_type_all_variants_serde() {
        let types = vec![
            PlanType::HMO,
            PlanType::PPO,
            PlanType::EPO,
            PlanType::POS,
            PlanType::HDHP,
            PlanType::Medicare,
            PlanType::MedicareAdvantage,
            PlanType::Medicaid,
            PlanType::Tricare,
            PlanType::WorkersComp,
            PlanType::AutoInsurance,
            PlanType::Other("CHIP".to_string()),
        ];
        for pt in types {
            let json = serde_json::to_string(&pt).expect("serialize");
            let decoded: PlanType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, pt);
        }
    }

    #[test]
    fn test_claim_status_all_variants_serde() {
        let statuses = vec![
            ClaimStatus::Draft,
            ClaimStatus::Submitted,
            ClaimStatus::Received,
            ClaimStatus::InProcess,
            ClaimStatus::Pending,
            ClaimStatus::Approved,
            ClaimStatus::PartiallyApproved,
            ClaimStatus::Denied,
            ClaimStatus::Appealed,
            ClaimStatus::Paid,
            ClaimStatus::Voided,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).expect("serialize");
            let decoded: ClaimStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, s);
        }
    }

    #[test]
    fn test_auth_status_all_variants_serde() {
        let statuses = vec![
            AuthStatus::Draft,
            AuthStatus::Submitted,
            AuthStatus::PendingInfo,
            AuthStatus::InReview,
            AuthStatus::Approved,
            AuthStatus::PartiallyApproved,
            AuthStatus::Denied,
            AuthStatus::Appealed,
            AuthStatus::AppealApproved,
            AuthStatus::AppealDenied,
            AuthStatus::Expired,
            AuthStatus::Cancelled,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).expect("serialize");
            let decoded: AuthStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, s);
        }
    }

    #[test]
    fn test_coverage_type_all_variants_serde() {
        let types = vec![
            CoverageType::Medical,
            CoverageType::Dental,
            CoverageType::Vision,
            CoverageType::Pharmacy,
            CoverageType::Mental,
            CoverageType::LongTermCare,
            CoverageType::Disability,
            CoverageType::Supplemental,
        ];
        for ct in types {
            let json = serde_json::to_string(&ct).expect("serialize");
            let decoded: CoverageType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, ct);
        }
    }

    // ==================== Claim line item tests ====================

    #[test]
    fn test_claim_line_item_serde_roundtrip() {
        let item = ClaimLineItem {
            line_number: 1,
            procedure_code: "99213".to_string(),
            modifiers: vec!["25".to_string()],
            description: "Office visit, level 3".to_string(),
            units: 1.0,
            charge_amount: 150.0,
            allowed_amount: Some(120.0),
            paid_amount: Some(96.0),
            denial_reason: None,
            service_date: "2025-03-15".to_string(),
            ndc: None,
        };
        let json = serde_json::to_string(&item).expect("serialize");
        let decoded: ClaimLineItem = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.line_number, 1);
        assert_eq!(decoded.procedure_code, "99213");
        assert!((decoded.charge_amount - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_claim_line_item_with_pharmacy_ndc() {
        let item = ClaimLineItem {
            line_number: 1,
            procedure_code: "J7030".to_string(),
            modifiers: vec![],
            description: "Normal saline IV".to_string(),
            units: 1.0,
            charge_amount: 25.0,
            allowed_amount: None,
            paid_amount: None,
            denial_reason: None,
            service_date: "2025-03-15".to_string(),
            ndc: Some("00409488802".to_string()),
        };
        let json = serde_json::to_string(&item).expect("serialize");
        let decoded: ClaimLineItem = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.ndc, Some("00409488802".to_string()));
    }

    // ==================== EOB tests ====================

    #[test]
    fn test_eob_line_detail_serde_roundtrip() {
        let detail = EOBLineDetail {
            service_date: "2025-03-15".to_string(),
            procedure_code: "99213".to_string(),
            description: "Office visit".to_string(),
            billed_amount: 150.0,
            allowed_amount: 120.0,
            paid_amount: 96.0,
            patient_responsibility: 24.0,
            adjustment_reason: Some("Contractual adjustment".to_string()),
        };
        let json = serde_json::to_string(&detail).expect("serialize");
        let decoded: EOBLineDetail = serde_json::from_str(&json).expect("deserialize");
        assert!((decoded.billed_amount - 150.0).abs() < f64::EPSILON);
        assert!((decoded.patient_responsibility - 24.0).abs() < f64::EPSILON);
    }

    // ==================== Edge cases ====================

    #[test]
    fn test_update_claim_denied_with_no_amounts() {
        let input = UpdateClaimInput {
            claim_hash: dummy_hash(),
            new_status: ClaimStatus::Denied,
            allowed_amount: None,
            paid_amount: None,
            patient_responsibility: None,
            payer_claim_number: None,
        };
        let json = serde_json::to_string(&input).expect("serialize");
        let decoded: UpdateClaimInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.new_status, ClaimStatus::Denied);
        assert!(decoded.allowed_amount.is_none());
        assert!(decoded.paid_amount.is_none());
    }

    #[test]
    fn test_authorization_type_all_variants_serde() {
        let types = vec![
            AuthorizationType::Procedure,
            AuthorizationType::Admission,
            AuthorizationType::Medication,
            AuthorizationType::DME,
            AuthorizationType::Therapy,
            AuthorizationType::Imaging,
            AuthorizationType::SpecialistReferral,
            AuthorizationType::OutOfNetwork,
        ];
        for at in types {
            let json = serde_json::to_string(&at).expect("serialize");
            let decoded: AuthorizationType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, at);
        }
    }

    #[test]
    fn test_subscriber_relationship_all_variants_serde() {
        let rels = vec![
            SubscriberRelationship::Self_,
            SubscriberRelationship::Spouse,
            SubscriberRelationship::Child,
            SubscriberRelationship::Other,
        ];
        for r in rels {
            let json = serde_json::to_string(&r).expect("serialize");
            let decoded: SubscriberRelationship = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, r);
        }
    }
}
