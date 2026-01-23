//! Clinical Trials Coordinator Zome
//!
//! Provides extern functions for trial management,
//! participant enrollment, data collection, and adverse events.
//!
//! ## Cross-Zome Integration
//!
//! This zome integrates with the Data Dividends zome to:
//! - Create data contributions when patients enroll in trials
//! - Track data usage when trial data is collected
//! - Enable fair compensation for research participation

use hdk::prelude::*;
use trials_integrity::*;

// ==================== DATA DIVIDENDS INTEGRATION ====================

/// Create a data contribution record when patient enrolls in a trial
fn try_create_trial_contribution(participant: &TrialParticipant, trial: &ClinicalTrial) {
    let _ = create_trial_contribution_internal(participant, trial);
}

/// Internal function to create data contribution for trial participation
fn create_trial_contribution_internal(
    participant: &TrialParticipant,
    trial: &ClinicalTrial,
) -> ExternResult<()> {
    // Determine data categories based on trial type
    let data_categories = data_categories_for_trial(trial);

    // Get NCT number or use trial_id as fallback
    let nct = trial
        .nct_number
        .clone()
        .unwrap_or_else(|| trial.trial_id.clone());

    // Create contribution input
    let contribution = TrialDataContributionInput {
        contribution_id: format!("TRIAL-{}-{}", nct, participant.participant_id),
        patient_hash: participant.patient_hash.clone(),
        data_type: "TreatmentOutcomes".to_string(),
        data_categories,
        consent_hash: participant.consent_hash.clone(),
        trial_hash: participant.trial_hash.clone(),
        trial_nct: nct,
        trial_title: trial.title.clone(),
        trial_phase: format!("{:?}", trial.phase),
        contributed_at: sys_time()?.as_micros() as i64,
        // Trial data is typically used for specific permitted uses
        permitted_uses: vec![
            "AcademicResearch".to_string(),
            "DrugDevelopment".to_string(),
            "ClinicalDecisionSupport".to_string(),
        ],
        // Standard prohibited uses for trial data
        prohibited_uses: vec![
            "Marketing".to_string(),
            "InsuranceUnderwriting".to_string(),
            "EmploymentDecisions".to_string(),
        ],
    };

    // Call dividends zome to create contribution
    let response = call(
        CallTargetCell::Local,
        ZomeName::from("dividends"),
        FunctionName::from("create_trial_contribution"),
        None,
        &contribution,
    )?;

    match response {
        ZomeCallResponse::Ok(_) => Ok(()),
        _ => Ok(()), // Best effort - don't fail enrollment if dividends integration fails
    }
}

/// Determine which data categories a trial will collect
fn data_categories_for_trial(trial: &ClinicalTrial) -> Vec<String> {
    let mut categories = vec![
        "Demographics".to_string(),
        "Medications".to_string(),
        "Outcomes".to_string(),
    ];

    // Add categories based on trial type/phase
    // Phase 1 trials often collect more safety data
    match trial.phase {
        TrialPhase::EarlyPhase1 | TrialPhase::Phase1 | TrialPhase::Phase1Phase2 => {
            categories.push("VitalSigns".to_string());
            categories.push("LabResults".to_string());
        }
        TrialPhase::Phase2 | TrialPhase::Phase2Phase3 => {
            categories.push("LabResults".to_string());
            categories.push("Diagnoses".to_string());
        }
        TrialPhase::Phase3 | TrialPhase::Phase4 => {
            categories.push("Diagnoses".to_string());
            categories.push("Procedures".to_string());
        }
        TrialPhase::NotApplicable => {}
    }

    categories
}

/// Track data collection from a trial visit for dividends
fn try_track_visit_data(visit: &TrialVisit, participant: &TrialParticipant) {
    let _ = track_visit_data_internal(visit, participant);
}

/// Internal function to track trial visit data usage
fn track_visit_data_internal(
    visit: &TrialVisit,
    participant: &TrialParticipant,
) -> ExternResult<()> {
    // Calculate data volume from visit
    let data_points_count = visit.data_points.len() as u64;

    if data_points_count == 0 {
        return Ok(()); // No data to track
    }

    let usage = TrialVisitDataUsageInput {
        usage_id: format!("VISIT-{}-{}", visit.visit_id, sys_time()?.as_micros()),
        patient_hash: participant.patient_hash.clone(),
        trial_hash: visit.trial_hash.clone(),
        visit_id: visit.visit_id.clone(),
        visit_number: visit.visit_number,
        data_points_count,
        collected_at: visit
            .actual_date
            .map(|ts| ts.as_micros())
            .unwrap_or_else(|| sys_time().map(|t| t.as_micros()).unwrap_or(0)),
    };

    // Call dividends zome to record usage
    let response = call(
        CallTargetCell::Local,
        ZomeName::from("dividends"),
        FunctionName::from("track_trial_data_usage"),
        None,
        &usage,
    )?;

    match response {
        ZomeCallResponse::Ok(_) => Ok(()),
        _ => Ok(()), // Best effort
    }
}

/// Input for creating trial data contribution
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrialDataContributionInput {
    pub contribution_id: String,
    pub patient_hash: ActionHash,
    pub data_type: String,
    pub data_categories: Vec<String>,
    pub consent_hash: ActionHash,
    pub trial_hash: ActionHash,
    pub trial_nct: String,
    pub trial_title: String,
    pub trial_phase: String,
    pub contributed_at: i64,
    pub permitted_uses: Vec<String>,
    pub prohibited_uses: Vec<String>,
}

/// Input for tracking trial visit data usage
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrialVisitDataUsageInput {
    pub usage_id: String,
    pub patient_hash: ActionHash,
    pub trial_hash: ActionHash,
    pub visit_id: String,
    pub visit_number: u32,
    pub data_points_count: u64,
    pub collected_at: i64,
}

/// Create a new clinical trial
#[hdk_extern]
pub fn create_trial(trial: ClinicalTrial) -> ExternResult<Record> {
    let trial_hash = create_entry(&EntryTypes::ClinicalTrial(trial.clone()))?;
    let record = get(trial_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find trial".to_string())
    ))?;

    // Link by status
    let status_anchor = match trial.status {
        TrialStatus::Recruiting | TrialStatus::EnrollingByInvitation => {
            anchor_hash("recruiting_trials")?
        }
        TrialStatus::Completed | TrialStatus::Terminated => anchor_hash("completed_trials")?,
        _ => anchor_hash("active_trials")?,
    };

    create_link(
        status_anchor,
        trial_hash.clone(),
        LinkTypes::ActiveTrials,
        (),
    )?;

    // Link by phase
    let phase_anchor = anchor_hash(&format!("phase_{:?}", trial.phase))?;
    create_link(phase_anchor, trial_hash, LinkTypes::TrialsByPhase, ())?;

    Ok(record)
}

/// Get a clinical trial
#[hdk_extern]
pub fn get_trial(trial_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(trial_hash, GetOptions::default())
}

/// Get recruiting trials
#[hdk_extern]
pub fn get_recruiting_trials(_: ()) -> ExternResult<Vec<Record>> {
    let recruiting_anchor = anchor_hash("recruiting_trials")?;
    let links = get_links(
        LinkQuery::try_new(recruiting_anchor, LinkTypes::RecruitingTrials)?,
        GetStrategy::default(),
    )?;

    let mut trials = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                trials.push(record);
            }
        }
    }

    Ok(trials)
}

/// Enroll participant in trial
#[hdk_extern]
pub fn enroll_participant(participant: TrialParticipant) -> ExternResult<Record> {
    let participant_hash = create_entry(&EntryTypes::TrialParticipant(participant.clone()))?;
    let record = get(participant_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find participant".to_string())
    ))?;

    // Link to trial
    create_link(
        participant.trial_hash.clone(),
        participant_hash.clone(),
        LinkTypes::TrialToParticipants,
        (),
    )?;

    // Link to patient
    create_link(
        participant.patient_hash.clone(),
        participant.trial_hash.clone(),
        LinkTypes::PatientToTrials,
        (),
    )?;

    // Update trial enrollment count and create data contribution
    if let Some(trial_record) = get(participant.trial_hash.clone(), GetOptions::default())? {
        if let Some(mut trial) = trial_record
            .entry()
            .to_app_option::<ClinicalTrial>()
            .ok()
            .flatten()
        {
            trial.current_enrollment += 1;
            trial.updated_at = sys_time()?;
            update_entry(participant.trial_hash.clone(), &trial)?;

            // Create data contribution in dividends zome (best effort)
            try_create_trial_contribution(&participant, &trial);
        }
    }

    Ok(record)
}

/// Get trial participants
#[hdk_extern]
pub fn get_trial_participants(trial_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(trial_hash, LinkTypes::TrialToParticipants)?,
        GetStrategy::default(),
    )?;

    let mut participants = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                participants.push(record);
            }
        }
    }

    Ok(participants)
}

/// Withdraw participant from trial
#[hdk_extern]
pub fn withdraw_participant(input: WithdrawInput) -> ExternResult<Record> {
    let record = get(input.participant_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Participant not found".to_string())
    ))?;

    let mut participant: TrialParticipant = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid participant".to_string()
        )))?;

    participant.status = ParticipantStatus::Withdrawn;
    participant.withdrawal_date = Some(sys_time()?);
    participant.withdrawal_reason = Some(input.reason);

    let updated_hash = update_entry(input.participant_hash, &participant)?;

    get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Could not find updated participant".to_string()
    )))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WithdrawInput {
    pub participant_hash: ActionHash,
    pub reason: String,
}

/// Record trial visit
#[hdk_extern]
pub fn record_visit(visit: TrialVisit) -> ExternResult<Record> {
    let visit_hash = create_entry(&EntryTypes::TrialVisit(visit.clone()))?;
    let record = get(visit_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find visit".to_string())
    ))?;

    create_link(
        visit.trial_hash.clone(),
        visit_hash,
        LinkTypes::TrialToVisits,
        (),
    )?;

    // Track data usage in dividends zome (best effort)
    // Need to get the participant record to link the data to the patient
    if let Some(participant_record) = get(visit.participant_hash.clone(), GetOptions::default())? {
        if let Some(participant) = participant_record
            .entry()
            .to_app_option::<TrialParticipant>()
            .ok()
            .flatten()
        {
            try_track_visit_data(&visit, &participant);
        }
    }

    Ok(record)
}

/// Get participant's visits
#[hdk_extern]
pub fn get_participant_visits(participant_hash: ActionHash) -> ExternResult<Vec<Record>> {
    // Get participant to find trial
    let record = get(participant_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Participant not found".to_string())
    ))?;

    let participant: TrialParticipant = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid participant".to_string()
        )))?;

    let links = get_links(
        LinkQuery::try_new(participant.trial_hash, LinkTypes::TrialToVisits)?,
        GetStrategy::default(),
    )?;

    let mut visits = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(visit_record) = get(hash, GetOptions::default())? {
                if let Some(visit) = visit_record
                    .entry()
                    .to_app_option::<TrialVisit>()
                    .ok()
                    .flatten()
                {
                    if visit.participant_hash == participant_hash {
                        visits.push(visit_record);
                    }
                }
            }
        }
    }

    Ok(visits)
}

/// Report adverse event
#[hdk_extern]
pub fn report_adverse_event(event: AdverseEvent) -> ExternResult<Record> {
    let event_hash = create_entry(&EntryTypes::AdverseEvent(event.clone()))?;
    let record = get(event_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find adverse event".to_string())
    ))?;

    create_link(
        event.trial_hash,
        event_hash,
        LinkTypes::TrialToAdverseEvents,
        (),
    )?;

    Ok(record)
}

/// Get trial adverse events
#[hdk_extern]
pub fn get_trial_adverse_events(trial_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(trial_hash, LinkTypes::TrialToAdverseEvents)?,
        GetStrategy::default(),
    )?;

    let mut events = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                events.push(record);
            }
        }
    }

    Ok(events)
}

/// Get serious adverse events for a trial
#[hdk_extern]
pub fn get_serious_adverse_events(trial_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let all_events = get_trial_adverse_events(trial_hash)?;

    let serious: Vec<Record> = all_events
        .into_iter()
        .filter(|record| {
            if let Some(event) = record
                .entry()
                .to_app_option::<AdverseEvent>()
                .ok()
                .flatten()
            {
                event.is_serious
            } else {
                false
            }
        })
        .collect();

    Ok(serious)
}

/// Check patient eligibility for a trial
#[hdk_extern]
pub fn check_eligibility(input: EligibilityCheckInput) -> ExternResult<EligibilityResult> {
    let trial_record = get(input.trial_hash, GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Trial not found".to_string())
    ))?;

    let trial: ClinicalTrial = trial_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid trial".to_string()
        )))?;

    let mut eligible = true;
    let mut reasons = Vec::new();

    // Check age
    if let Some(min_age) = trial.eligibility.min_age {
        if input.patient_age < min_age {
            eligible = false;
            reasons.push(format!(
                "Patient age {} is below minimum {}",
                input.patient_age, min_age
            ));
        }
    }

    if let Some(max_age) = trial.eligibility.max_age {
        if input.patient_age > max_age {
            eligible = false;
            reasons.push(format!(
                "Patient age {} is above maximum {}",
                input.patient_age, max_age
            ));
        }
    }

    // Check if trial is recruiting
    if !matches!(
        trial.status,
        TrialStatus::Recruiting | TrialStatus::EnrollingByInvitation
    ) {
        eligible = false;
        reasons.push("Trial is not currently recruiting".to_string());
    }

    // Check enrollment capacity
    if trial.current_enrollment >= trial.target_enrollment {
        eligible = false;
        reasons.push("Trial has reached target enrollment".to_string());
    }

    Ok(EligibilityResult {
        eligible,
        reasons,
        trial_title: trial.title,
        trial_phase: format!("{:?}", trial.phase),
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EligibilityCheckInput {
    pub trial_hash: ActionHash,
    pub patient_hash: ActionHash,
    pub patient_age: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EligibilityResult {
    pub eligible: bool,
    pub reasons: Vec<String>,
    pub trial_title: String,
    pub trial_phase: String,
}

/// Anchor entry for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}
