//! Insurance Claims and Coverage Verification Integrity Zome
//!
//! Defines entry types for insurance policies, claims, prior authorizations,
//! and coverage verification with X12 EDI alignment.

use hdi::prelude::*;

/// Insurance plan/policy
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct InsurancePlan {
    pub plan_id: String,
    pub patient_hash: ActionHash,
    /// Insurance company/payer
    pub payer_name: String,
    pub payer_id: String,
    /// Member ID
    pub member_id: String,
    pub group_number: Option<String>,
    /// Plan type
    pub plan_type: PlanType,
    /// Coverage type
    pub coverage_type: CoverageType,
    /// Relationship to subscriber
    pub relationship: SubscriberRelationship,
    /// Subscriber if not patient
    pub subscriber_name: Option<String>,
    pub subscriber_dob: Option<String>,
    pub subscriber_id: Option<String>,
    /// Effective dates
    pub effective_date: String,
    pub termination_date: Option<String>,
    pub status: PlanStatus,
    /// Benefits summary
    pub deductible: Option<f64>,
    pub deductible_met: Option<f64>,
    pub out_of_pocket_max: Option<f64>,
    pub out_of_pocket_met: Option<f64>,
    pub copay_primary: Option<f64>,
    pub copay_specialist: Option<f64>,
    pub copay_emergency: Option<f64>,
    pub coinsurance_percent: Option<f64>,
    /// Priority order (primary, secondary, etc.)
    pub coordination_order: u8,
    /// MATL trust score for claims processing
    pub matl_trust_score: f64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PlanType {
    HMO,
    PPO,
    EPO,
    POS,
    HDHP,
    Medicare,
    MedicareAdvantage,
    Medicaid,
    Tricare,
    WorkersComp,
    AutoInsurance,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CoverageType {
    Medical,
    Dental,
    Vision,
    Pharmacy,
    Mental,
    LongTermCare,
    Disability,
    Supplemental,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SubscriberRelationship {
    Self_,
    Spouse,
    Child,
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PlanStatus {
    Active,
    Terminated,
    Pending,
    Suspended,
}

/// Insurance claim
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Claim {
    pub claim_id: String,
    pub patient_hash: ActionHash,
    pub plan_hash: ActionHash,
    pub encounter_hash: ActionHash,
    /// Billing provider
    pub billing_provider_hash: ActionHash,
    /// Rendering provider (if different)
    pub rendering_provider_hash: Option<ActionHash>,
    /// Claim type
    pub claim_type: ClaimType,
    /// Place of service code
    pub place_of_service: String,
    /// Primary diagnosis (ICD-10)
    pub primary_diagnosis: String,
    /// Secondary diagnoses
    pub secondary_diagnoses: Vec<String>,
    /// Line items
    pub line_items: Vec<ClaimLineItem>,
    /// Totals
    pub total_charges: f64,
    pub total_allowed: Option<f64>,
    pub total_paid: Option<f64>,
    pub patient_responsibility: Option<f64>,
    /// Dates
    pub service_date_from: String,
    pub service_date_to: String,
    pub submitted_at: Timestamp,
    /// Status tracking
    pub status: ClaimStatus,
    pub adjudication_date: Option<Timestamp>,
    /// Payer claim number
    pub payer_claim_number: Option<String>,
    /// Remittance advice
    pub remittance_hash: Option<ActionHash>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ClaimType {
    Professional,  // CMS-1500
    Institutional, // UB-04
    Dental,
    Pharmacy,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClaimLineItem {
    pub line_number: u32,
    /// CPT/HCPCS code
    pub procedure_code: String,
    pub modifiers: Vec<String>,
    pub description: String,
    pub units: f64,
    pub charge_amount: f64,
    pub allowed_amount: Option<f64>,
    pub paid_amount: Option<f64>,
    pub denial_reason: Option<String>,
    pub service_date: String,
    /// National Drug Code (for pharmacy)
    pub ndc: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ClaimStatus {
    Draft,
    Submitted,
    Received,
    InProcess,
    Pending,
    Approved,
    PartiallyApproved,
    Denied,
    Appealed,
    Paid,
    Voided,
}

/// Prior authorization request
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PriorAuthorization {
    pub auth_id: String,
    pub patient_hash: ActionHash,
    pub plan_hash: ActionHash,
    pub requesting_provider: AgentPubKey,
    /// What's being requested
    pub auth_type: AuthorizationType,
    /// CPT/HCPCS codes
    pub procedure_codes: Vec<String>,
    /// ICD-10 diagnosis codes
    pub diagnosis_codes: Vec<String>,
    /// Clinical justification
    pub clinical_notes: String,
    /// Supporting documentation
    pub supporting_docs: Vec<EntryHash>,
    /// Quantity requested
    pub quantity_requested: Option<u32>,
    /// Duration requested (days)
    pub duration_days: Option<u32>,
    /// Status
    pub status: AuthStatus,
    pub submitted_at: Timestamp,
    pub decision_at: Option<Timestamp>,
    pub decision_by: Option<String>,
    /// Authorization number (if approved)
    pub auth_number: Option<String>,
    pub effective_date: Option<String>,
    pub expiration_date: Option<String>,
    /// Denial reason
    pub denial_reason: Option<String>,
    /// Appeal information
    pub appealed: bool,
    pub appeal_deadline: Option<Timestamp>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AuthorizationType {
    Procedure,
    Admission,
    Medication,
    DME,
    Therapy,
    Imaging,
    SpecialistReferral,
    OutOfNetwork,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AuthStatus {
    Draft,
    Submitted,
    PendingInfo,
    InReview,
    Approved,
    PartiallyApproved,
    Denied,
    Appealed,
    AppealApproved,
    AppealDenied,
    Expired,
    Cancelled,
}

/// Eligibility verification
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct EligibilityCheck {
    pub check_id: String,
    pub patient_hash: ActionHash,
    pub plan_hash: ActionHash,
    pub requested_by: AgentPubKey,
    pub checked_at: Timestamp,
    pub service_date: String,
    pub service_type: String,
    /// Response
    pub eligible: bool,
    pub coverage_active: bool,
    pub deductible_remaining: Option<f64>,
    pub oop_remaining: Option<f64>,
    pub copay_amount: Option<f64>,
    pub coinsurance_percent: Option<f64>,
    pub requires_auth: bool,
    pub in_network: Option<bool>,
    pub response_code: Option<String>,
    pub response_message: Option<String>,
}

/// Explanation of Benefits (EOB)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ExplanationOfBenefits {
    pub eob_id: String,
    pub claim_hash: ActionHash,
    pub patient_hash: ActionHash,
    pub plan_hash: ActionHash,
    pub issued_date: Timestamp,
    /// Summary
    pub total_charges: f64,
    pub plan_discount: f64,
    pub amount_paid: f64,
    pub patient_owes: f64,
    /// Applied to deductible
    pub applied_to_deductible: f64,
    /// Line item details
    pub line_details: Vec<EOBLineDetail>,
    /// Remarks/messages
    pub remarks: Vec<String>,
    /// Appeal rights
    pub appeal_deadline: Option<Timestamp>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EOBLineDetail {
    pub service_date: String,
    pub procedure_code: String,
    pub description: String,
    pub billed_amount: f64,
    pub allowed_amount: f64,
    pub paid_amount: f64,
    pub patient_responsibility: f64,
    pub adjustment_reason: Option<String>,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    InsurancePlan(InsurancePlan),
    Claim(Claim),
    PriorAuthorization(PriorAuthorization),
    EligibilityCheck(EligibilityCheck),
    ExplanationOfBenefits(ExplanationOfBenefits),
}

#[hdk_link_types]
pub enum LinkTypes {
    PatientToPlans,
    PatientToClaims,
    PatientToAuths,
    PlanToClaims,
    ClaimToEOB,
    EncounterToClaim,
    PendingAuths,
    PendingClaims,
    DeniedClaims,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::InsurancePlan(p) => validate_plan(&p),
                EntryTypes::Claim(c) => validate_claim(&c),
                EntryTypes::PriorAuthorization(a) => validate_auth(&a),
                EntryTypes::EligibilityCheck(_) => Ok(ValidateCallbackResult::Valid),
                EntryTypes::ExplanationOfBenefits(_) => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_plan(plan: &InsurancePlan) -> ExternResult<ValidateCallbackResult> {
    if plan.plan_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Plan ID is required".to_string(),
        ));
    }
    if plan.member_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Member ID is required".to_string(),
        ));
    }
    if plan.matl_trust_score < 0.0 || plan.matl_trust_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "MATL trust score must be between 0.0 and 1.0".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_claim(claim: &Claim) -> ExternResult<ValidateCallbackResult> {
    if claim.claim_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Claim ID is required".to_string(),
        ));
    }
    if claim.primary_diagnosis.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Primary diagnosis is required".to_string(),
        ));
    }
    if claim.line_items.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one line item is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_auth(auth: &PriorAuthorization) -> ExternResult<ValidateCallbackResult> {
    if auth.auth_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Authorization ID is required".to_string(),
        ));
    }
    if auth.clinical_notes.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Clinical justification is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
