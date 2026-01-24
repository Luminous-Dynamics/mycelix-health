//! SDOH Coordinator Zome
//!
//! Social Determinants of Health screening, resource management, and intervention tracking.

use hdk::prelude::*;
use sdoh_integrity::*;

/// Input for creating a screening
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateScreeningInput {
    pub patient_hash: ActionHash,
    pub instrument: ScreeningInstrument,
    pub responses: Vec<ScreeningResponse>,
    pub notes: Option<String>,
    pub consent_obtained: bool,
}

/// Create a new SDOH screening
#[hdk_extern]
pub fn create_screening(input: CreateScreeningInput) -> ExternResult<Record> {
    let caller = agent_info()?.agent_initial_pubkey;

    // Calculate risk levels from responses
    let (overall_risk, domains_at_risk, categories_at_risk) =
        calculate_risk_levels(&input.responses);

    let screening = SdohScreening {
        patient_hash: input.patient_hash.clone(),
        screener_hash: caller,
        instrument: input.instrument,
        screening_date: sys_time()?,
        responses: input.responses,
        overall_risk_level: overall_risk,
        domains_at_risk,
        categories_at_risk,
        notes: input.notes,
        consent_obtained: input.consent_obtained,
        created_at: sys_time()?,
    };

    let action_hash = create_entry(&EntryTypes::SdohScreening(screening))?;

    // Link patient to screening
    create_link(
        input.patient_hash,
        action_hash.clone(),
        LinkTypes::PatientToScreenings,
        (),
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get created screening".to_string())))
}

/// Calculate risk levels from screening responses
fn calculate_risk_levels(
    responses: &[ScreeningResponse],
) -> (RiskLevel, Vec<SdohDomain>, Vec<SdohCategory>) {
    let mut categories_at_risk: Vec<SdohCategory> = Vec::new();
    let mut risk_count = 0;
    let mut urgent_count = 0;

    for response in responses {
        if response.risk_indicated {
            risk_count += 1;
            if !categories_at_risk.contains(&response.category) {
                categories_at_risk.push(response.category.clone());
            }
            // Check for urgent indicators
            if matches!(
                response.category,
                SdohCategory::InterpersonalViolence
                    | SdohCategory::HousingInstability
                    | SdohCategory::FoodInsecurity
            ) {
                urgent_count += 1;
            }
        }
    }

    // Determine domains at risk
    let domains_at_risk: Vec<SdohDomain> = categories_at_risk
        .iter()
        .map(|cat| category_to_domain(cat))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Calculate overall risk
    let overall_risk = if urgent_count > 0 {
        RiskLevel::Urgent
    } else if risk_count >= 5 {
        RiskLevel::HighRisk
    } else if risk_count >= 3 {
        RiskLevel::ModerateRisk
    } else if risk_count >= 1 {
        RiskLevel::LowRisk
    } else {
        RiskLevel::NoRisk
    };

    (overall_risk, domains_at_risk, categories_at_risk)
}

/// Map category to domain
fn category_to_domain(category: &SdohCategory) -> SdohDomain {
    match category {
        SdohCategory::Employment
        | SdohCategory::FoodInsecurity
        | SdohCategory::HousingInstability
        | SdohCategory::Transportation
        | SdohCategory::Utilities => SdohDomain::EconomicStability,

        SdohCategory::Literacy
        | SdohCategory::LanguageBarrier
        | SdohCategory::EducationLevel => SdohDomain::EducationAccess,

        SdohCategory::HealthInsurance
        | SdohCategory::ProviderAvailability
        | SdohCategory::HealthLiteracy => SdohDomain::HealthcareAccess,

        SdohCategory::SafetyConcerns
        | SdohCategory::EnvironmentalHazards
        | SdohCategory::AccessToHealthyFood => SdohDomain::NeighborhoodEnvironment,

        SdohCategory::SocialIsolation
        | SdohCategory::InterpersonalViolence
        | SdohCategory::IncarceratedFamilyMember
        | SdohCategory::RefugeeImmigrantStatus
        | SdohCategory::Discrimination
        | SdohCategory::Stress => SdohDomain::SocialCommunity,
    }
}

/// Get a screening by hash
#[hdk_extern]
pub fn get_screening(hash: ActionHash) -> ExternResult<Option<Record>> {
    get(hash, GetOptions::default())
}

/// Get all screenings for a patient
#[hdk_extern]
pub fn get_patient_screenings(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToScreenings)?, GetStrategy::default(),
    )?;

    let mut screenings = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                screenings.push(record);
            }
        }
    }

    Ok(screenings)
}

/// Input for creating a community resource
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateResourceInput {
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
}

/// Create a community resource
#[hdk_extern]
pub fn create_resource(input: CreateResourceInput) -> ExternResult<Record> {
    let caller = agent_info()?.agent_initial_pubkey;

    let resource = CommunityResource {
        name: input.name,
        resource_type: input.resource_type,
        categories_served: input.categories_served.clone(),
        description: input.description,
        address: input.address,
        city: input.city,
        state: input.state,
        zip_code: input.zip_code.clone(),
        phone: input.phone,
        website: input.website,
        email: input.email,
        hours_of_operation: input.hours_of_operation,
        eligibility_requirements: input.eligibility_requirements,
        languages_available: input.languages_available,
        accepts_uninsured: input.accepts_uninsured,
        accepts_medicaid: input.accepts_medicaid,
        is_active: true,
        last_verified: sys_time()?,
        created_by: caller,
        created_at: sys_time()?,
    };

    let action_hash = create_entry(&EntryTypes::CommunityResource(resource))?;

    // Link by zip code for geographic search
    let zip_anchor = anchor_hash(&format!("zip:{}", input.zip_code))?;
    create_link(
        zip_anchor,
        action_hash.clone(),
        LinkTypes::ZipCodeToResources,
        (),
    )?;

    // Link by category
    for category in input.categories_served {
        let cat_anchor = anchor_hash(&format!("sdoh_category:{:?}", category))?;
        create_link(
            cat_anchor,
            action_hash.clone(),
            LinkTypes::CategoryToResources,
            (),
        )?;
    }

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get created resource".to_string())))
}

/// Create an anchor hash for linking using Path
fn anchor_hash(anchor: &str) -> ExternResult<EntryHash> {
    let path = Path::from(anchor);
    path.path_entry_hash()
}

/// Get a resource by hash
#[hdk_extern]
pub fn get_resource(hash: ActionHash) -> ExternResult<Option<Record>> {
    get(hash, GetOptions::default())
}

/// Search criteria for resources
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceSearchCriteria {
    pub zip_code: Option<String>,
    pub category: Option<SdohCategory>,
    pub accepts_uninsured: Option<bool>,
    pub accepts_medicaid: Option<bool>,
    pub language: Option<String>,
}

/// Search for community resources
#[hdk_extern]
pub fn search_resources(criteria: ResourceSearchCriteria) -> ExternResult<Vec<Record>> {
    let mut results: Vec<Record> = Vec::new();

    // Search by zip code
    if let Some(zip) = &criteria.zip_code {
        let zip_anchor = anchor_hash(&format!("zip:{}", zip))?;
        let links = get_links(
            LinkQuery::try_new(zip_anchor, LinkTypes::ZipCodeToResources)?, GetStrategy::default(),
        )?;

        for link in links {
            if let Some(target) = link.target.into_action_hash() {
                if let Some(record) = get(target, GetOptions::default())? {
                    results.push(record);
                }
            }
        }
    }

    // Search by category
    if let Some(category) = &criteria.category {
        let cat_anchor = anchor_hash(&format!("sdoh_category:{:?}", category))?;
        let links = get_links(
            LinkQuery::try_new(cat_anchor, LinkTypes::CategoryToResources)?, GetStrategy::default(),
        )?;

        for link in links {
            if let Some(target) = link.target.into_action_hash() {
                if let Some(record) = get(target, GetOptions::default())? {
                    // Check if already in results
                    let hash = record.action_address().clone();
                    if !results.iter().any(|r| r.action_address() == &hash) {
                        results.push(record);
                    }
                }
            }
        }
    }

    // Filter results
    let filtered: Vec<Record> = results
        .into_iter()
        .filter(|record| {
            if let Ok(Some(resource)) = record
                .entry()
                .to_app_option::<CommunityResource>()
            {
                // Check uninsured filter
                if let Some(accepts) = criteria.accepts_uninsured {
                    if resource.accepts_uninsured != accepts {
                        return false;
                    }
                }

                // Check medicaid filter
                if let Some(accepts) = criteria.accepts_medicaid {
                    if resource.accepts_medicaid != accepts {
                        return false;
                    }
                }

                // Check language filter
                if let Some(lang) = &criteria.language {
                    if !resource.languages_available.contains(lang) {
                        return false;
                    }
                }

                // Must be active
                resource.is_active
            } else {
                false
            }
        })
        .collect();

    Ok(filtered)
}

/// Input for creating an intervention
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateInterventionInput {
    pub screening_hash: ActionHash,
    pub patient_hash: ActionHash,
    pub category: SdohCategory,
    pub resource_hash: Option<ActionHash>,
    pub resource_name: String,
    pub intervention_type: String,
    pub notes: Option<String>,
}

/// Create an SDOH intervention/referral
#[hdk_extern]
pub fn create_intervention(input: CreateInterventionInput) -> ExternResult<Record> {
    let caller = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let intervention = SdohIntervention {
        screening_hash: input.screening_hash.clone(),
        patient_hash: input.patient_hash,
        category: input.category,
        resource_hash: input.resource_hash.clone(),
        resource_name: input.resource_name,
        intervention_type: input.intervention_type,
        status: InterventionStatus::ReferralMade,
        referred_by: caller,
        referred_date: now,
        follow_up_date: None,
        outcome: None,
        barrier_to_completion: None,
        notes: input.notes,
        created_at: now,
        updated_at: now,
    };

    let action_hash = create_entry(&EntryTypes::SdohIntervention(intervention))?;

    // Link screening to intervention
    create_link(
        input.screening_hash,
        action_hash.clone(),
        LinkTypes::ScreeningToInterventions,
        (),
    )?;

    // Link resource to intervention if provided
    if let Some(resource_hash) = input.resource_hash {
        create_link(
            resource_hash,
            action_hash.clone(),
            LinkTypes::ResourceToInterventions,
            (),
        )?;
    }

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get created intervention".to_string())))
}

/// Get interventions for a screening
#[hdk_extern]
pub fn get_screening_interventions(screening_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(screening_hash, LinkTypes::ScreeningToInterventions)?, GetStrategy::default(),
    )?;

    let mut interventions = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                interventions.push(record);
            }
        }
    }

    Ok(interventions)
}

/// Input for updating intervention status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateInterventionInput {
    pub intervention_hash: ActionHash,
    pub status: InterventionStatus,
    pub outcome: Option<String>,
    pub barrier_to_completion: Option<String>,
    pub notes: Option<String>,
}

/// Update an intervention's status
#[hdk_extern]
pub fn update_intervention(input: UpdateInterventionInput) -> ExternResult<Record> {
    let record = get(input.intervention_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Intervention not found".to_string())))?;

    let mut intervention: SdohIntervention = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid intervention entry".to_string())))?;

    intervention.status = input.status;
    intervention.outcome = input.outcome.or(intervention.outcome);
    intervention.barrier_to_completion = input.barrier_to_completion.or(intervention.barrier_to_completion);
    intervention.notes = input.notes.or(intervention.notes);
    intervention.updated_at = sys_time()?;

    let action_hash = update_entry(input.intervention_hash, &intervention)?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get updated intervention".to_string())))
}

/// Input for creating a follow-up
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateFollowUpInput {
    pub intervention_hash: ActionHash,
    pub contact_method: String,
    pub patient_reached: bool,
    pub current_status: InterventionStatus,
    pub patient_feedback: Option<String>,
    pub need_resolved: bool,
    pub barriers_identified: Vec<String>,
    pub next_steps: Option<String>,
    pub next_follow_up_date: Option<Timestamp>,
}

/// Create a follow-up record
#[hdk_extern]
pub fn create_follow_up(input: CreateFollowUpInput) -> ExternResult<Record> {
    let caller = agent_info()?.agent_initial_pubkey;

    let follow_up = InterventionFollowUp {
        intervention_hash: input.intervention_hash.clone(),
        follow_up_date: sys_time()?,
        contact_method: input.contact_method,
        contacted_by: caller,
        patient_reached: input.patient_reached,
        current_status: input.current_status,
        patient_feedback: input.patient_feedback,
        need_resolved: input.need_resolved,
        barriers_identified: input.barriers_identified,
        next_steps: input.next_steps,
        next_follow_up_date: input.next_follow_up_date,
        created_at: sys_time()?,
    };

    let action_hash = create_entry(&EntryTypes::InterventionFollowUp(follow_up))?;

    // Link intervention to follow-up
    create_link(
        input.intervention_hash,
        action_hash.clone(),
        LinkTypes::InterventionToFollowUps,
        (),
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get created follow-up".to_string())))
}

/// Get follow-ups for an intervention
#[hdk_extern]
pub fn get_intervention_follow_ups(intervention_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(intervention_hash, LinkTypes::InterventionToFollowUps)?, GetStrategy::default(),
    )?;

    let mut follow_ups = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                follow_ups.push(record);
            }
        }
    }

    Ok(follow_ups)
}

/// Summary of patient's SDOH status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatientSdohSummary {
    pub patient_hash: ActionHash,
    pub latest_screening_date: Option<Timestamp>,
    pub overall_risk_level: Option<RiskLevel>,
    pub active_needs: Vec<SdohCategory>,
    pub pending_interventions: u32,
    pub completed_interventions: u32,
    pub needs_follow_up: bool,
}

/// Get SDOH summary for a patient
#[hdk_extern]
pub fn get_patient_sdoh_summary(patient_hash: ActionHash) -> ExternResult<PatientSdohSummary> {
    let screenings = get_patient_screenings(patient_hash.clone())?;

    let mut latest_date: Option<Timestamp> = None;
    let mut overall_risk: Option<RiskLevel> = None;
    let mut active_needs: Vec<SdohCategory> = Vec::new();
    let mut pending = 0u32;
    let mut completed = 0u32;

    // Find latest screening
    for record in &screenings {
        if let Some(screening) = record.entry().to_app_option::<SdohScreening>().ok().flatten() {
            if latest_date.is_none() || screening.screening_date > latest_date.unwrap() {
                latest_date = Some(screening.screening_date);
                overall_risk = Some(screening.overall_risk_level.clone());
                active_needs = screening.categories_at_risk.clone();
            }

            // Count interventions
            let interventions = get_screening_interventions(record.action_address().clone())?;
            for int_record in interventions {
                if let Some(intervention) = int_record.entry().to_app_option::<SdohIntervention>().ok().flatten() {
                    match intervention.status {
                        InterventionStatus::Completed => completed += 1,
                        InterventionStatus::Identified
                        | InterventionStatus::ReferralMade
                        | InterventionStatus::InProgress => pending += 1,
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(PatientSdohSummary {
        patient_hash,
        latest_screening_date: latest_date,
        overall_risk_level: overall_risk,
        active_needs,
        pending_interventions: pending,
        completed_interventions: completed,
        needs_follow_up: pending > 0,
    })
}
