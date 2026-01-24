//! SDOH Integrity Zome
//!
//! Social Determinants of Health screening and intervention tracking.
//! Supports PRAPARE, AHC-HRSN, and custom screening instruments.

use hdi::prelude::*;

/// SDOH screening instruments
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScreeningInstrument {
    /// Protocol for Responding to and Assessing Patients' Assets, Risks, and Experiences
    PRAPARE,
    /// Accountable Health Communities Health-Related Social Needs
    AHCHRSN,
    /// We Care (pediatric)
    WeCare,
    /// Custom organization-specific instrument
    Custom(String),
}

/// SDOH domains based on Healthy People 2030
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SdohDomain {
    EconomicStability,
    EducationAccess,
    HealthcareAccess,
    NeighborhoodEnvironment,
    SocialCommunity,
}

/// Specific SDOH categories
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SdohCategory {
    // Economic Stability
    Employment,
    FoodInsecurity,
    HousingInstability,
    Transportation,
    Utilities,

    // Education
    Literacy,
    LanguageBarrier,
    EducationLevel,

    // Healthcare Access
    HealthInsurance,
    ProviderAvailability,
    HealthLiteracy,

    // Neighborhood
    SafetyConcerns,
    EnvironmentalHazards,
    AccessToHealthyFood,

    // Social/Community
    SocialIsolation,
    InterpersonalViolence,
    IncarceratedFamilyMember,
    RefugeeImmigrantStatus,
    Discrimination,
    Stress,
}

/// Risk level for SDOH screening results
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RiskLevel {
    NoRisk,
    LowRisk,
    ModerateRisk,
    HighRisk,
    Urgent,
}

/// Intervention status
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InterventionStatus {
    Identified,
    ReferralMade,
    InProgress,
    Completed,
    Declined,
    UnableToComplete,
}

/// Resource type
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ResourceType {
    FoodPantry,
    HousingAssistance,
    TransportationService,
    UtilityAssistance,
    EmploymentServices,
    LegalAid,
    MentalHealthServices,
    SubstanceAbuseServices,
    DomesticViolenceServices,
    ChildcareServices,
    EducationProgram,
    LanguageServices,
    Other(String),
}

/// Individual screening response
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScreeningResponse {
    pub question_id: String,
    pub question_text: String,
    pub response: String,
    pub response_code: Option<String>,
    pub category: SdohCategory,
    pub risk_indicated: bool,
}

/// Complete SDOH screening
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SdohScreening {
    pub patient_hash: ActionHash,
    pub screener_hash: AgentPubKey,
    pub instrument: ScreeningInstrument,
    pub screening_date: Timestamp,
    pub responses: Vec<ScreeningResponse>,
    pub overall_risk_level: RiskLevel,
    pub domains_at_risk: Vec<SdohDomain>,
    pub categories_at_risk: Vec<SdohCategory>,
    pub notes: Option<String>,
    pub consent_obtained: bool,
    pub created_at: Timestamp,
}

/// Community resource for SDOH interventions
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CommunityResource {
    pub name: String,
    pub resource_type: ResourceType,
    pub categories_served: Vec<SdohCategory>,
    pub description: String,
    pub address: Option<String>,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub email: Option<String>,
    pub hours_of_operation: Option<String>,
    pub eligibility_requirements: Option<String>,
    pub languages_available: Vec<String>,
    pub accepts_uninsured: bool,
    pub accepts_medicaid: bool,
    pub is_active: bool,
    pub last_verified: Timestamp,
    pub created_by: AgentPubKey,
    pub created_at: Timestamp,
}

/// SDOH intervention/referral
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SdohIntervention {
    pub screening_hash: ActionHash,
    pub patient_hash: ActionHash,
    pub category: SdohCategory,
    pub resource_hash: Option<ActionHash>,
    pub resource_name: String,
    pub intervention_type: String,
    pub status: InterventionStatus,
    pub referred_by: AgentPubKey,
    pub referred_date: Timestamp,
    pub follow_up_date: Option<Timestamp>,
    pub outcome: Option<String>,
    pub barrier_to_completion: Option<String>,
    pub notes: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Follow-up on intervention
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct InterventionFollowUp {
    pub intervention_hash: ActionHash,
    pub follow_up_date: Timestamp,
    pub contact_method: String,
    pub contacted_by: AgentPubKey,
    pub patient_reached: bool,
    pub current_status: InterventionStatus,
    pub patient_feedback: Option<String>,
    pub need_resolved: bool,
    pub barriers_identified: Vec<String>,
    pub next_steps: Option<String>,
    pub next_follow_up_date: Option<Timestamp>,
    pub created_at: Timestamp,
}

/// Aggregate SDOH data for population health (de-identified)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SdohPopulationData {
    pub organization_id: String,
    pub reporting_period_start: Timestamp,
    pub reporting_period_end: Timestamp,
    pub total_screenings: u32,
    pub screenings_by_instrument: Vec<(ScreeningInstrument, u32)>,
    pub risk_by_domain: Vec<(SdohDomain, u32)>,
    pub risk_by_category: Vec<(SdohCategory, u32)>,
    pub interventions_made: u32,
    pub interventions_completed: u32,
    pub created_at: Timestamp,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    SdohScreening(SdohScreening),
    CommunityResource(CommunityResource),
    SdohIntervention(SdohIntervention),
    InterventionFollowUp(InterventionFollowUp),
    SdohPopulationData(SdohPopulationData),
}

#[hdk_link_types]
pub enum LinkTypes {
    PatientToScreenings,
    ScreeningToInterventions,
    InterventionToFollowUps,
    CategoryToResources,
    ResourceToInterventions,
    ZipCodeToResources,
}

/// Validate SDOH entries
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => validate_create_entry(app_entry),
            OpEntry::UpdateEntry { app_entry, .. } => validate_create_entry(app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(entry: EntryTypes) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::SdohScreening(screening) => validate_screening(&screening),
        EntryTypes::CommunityResource(resource) => validate_resource(&resource),
        EntryTypes::SdohIntervention(intervention) => validate_intervention(&intervention),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_screening(screening: &SdohScreening) -> ExternResult<ValidateCallbackResult> {
    // Must have at least one response
    if screening.responses.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Screening must have at least one response".to_string(),
        ));
    }

    // Must have consent
    if !screening.consent_obtained {
        return Ok(ValidateCallbackResult::Invalid(
            "Patient consent required for SDOH screening".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_resource(resource: &CommunityResource) -> ExternResult<ValidateCallbackResult> {
    // Must have a name
    if resource.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Resource must have a name".to_string(),
        ));
    }

    // Must serve at least one category
    if resource.categories_served.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Resource must serve at least one SDOH category".to_string(),
        ));
    }

    // Must have location
    if resource.city.is_empty() || resource.state.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Resource must have city and state".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_intervention(intervention: &SdohIntervention) -> ExternResult<ValidateCallbackResult> {
    // Must have resource name
    if intervention.resource_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Intervention must specify resource name".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}
