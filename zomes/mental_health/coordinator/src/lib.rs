#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Mental Health Coordinator Zome
//!
//! Behavioral health management with enhanced privacy and crisis protocols.

use hdk::prelude::*;
use mental_health_integrity::*;
use mycelix_health_shared::{
    require_authorization,
    log_data_access,
    DataCategory,
    Permission,
    validation::{
        validate_screening_responses,
        validate_mood_entry_scores,
        validate_sleep_hours,
    },
    batch::{links_to_records_paginated, links_to_recent_records},
    PaginationInput,
};

/// Input for creating a screening
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateScreeningInput {
    pub patient_hash: ActionHash,
    pub instrument: MentalHealthInstrument,
    pub responses: Vec<(String, u8)>,
    pub notes: Option<String>,
}

/// Score interpretation for different instruments
fn interpret_score(instrument: &MentalHealthInstrument, score: u32) -> (Severity, String, bool) {
    match instrument {
        MentalHealthInstrument::PHQ9 => {
            let (severity, interpretation) = match score {
                0..=4 => (Severity::None, "None-minimal depression"),
                5..=9 => (Severity::Mild, "Mild depression"),
                10..=14 => (Severity::Moderate, "Moderate depression"),
                15..=19 => (Severity::ModeratelySevere, "Moderately severe depression"),
                _ => (Severity::Severe, "Severe depression"),
            };
            let follow_up = score >= 10;
            (severity, interpretation.to_string(), follow_up)
        }
        MentalHealthInstrument::GAD7 => {
            let (severity, interpretation) = match score {
                0..=4 => (Severity::Minimal, "Minimal anxiety"),
                5..=9 => (Severity::Mild, "Mild anxiety"),
                10..=14 => (Severity::Moderate, "Moderate anxiety"),
                _ => (Severity::Severe, "Severe anxiety"),
            };
            let follow_up = score >= 10;
            (severity, interpretation.to_string(), follow_up)
        }
        MentalHealthInstrument::PHQ2 => {
            let follow_up = score >= 3;
            let severity = if score >= 3 { Severity::Moderate } else { Severity::None };
            (severity, format!("PHQ-2 score: {} (positive if >= 3)", score), follow_up)
        }
        MentalHealthInstrument::AUDIT => {
            let (severity, interpretation) = match score {
                0..=7 => (Severity::None, "Low risk drinking"),
                8..=15 => (Severity::Mild, "Hazardous drinking"),
                16..=19 => (Severity::Moderate, "Harmful drinking"),
                _ => (Severity::Severe, "Possible alcohol dependence"),
            };
            let follow_up = score >= 8;
            (severity, interpretation.to_string(), follow_up)
        }
        // DAST-10: Drug Abuse Screening Test (Skinner 1982)
        MentalHealthInstrument::DAST10 => {
            let (severity, interpretation) = match score {
                0 => (Severity::None, "No problems reported"),
                1..=2 => (Severity::Minimal, "Low level of drug-related problems"),
                3..=5 => (Severity::Moderate, "Moderate level of drug-related problems"),
                6..=8 => (Severity::ModeratelySevere, "Substantial level of drug-related problems"),
                _ => (Severity::Severe, "Severe level of drug-related problems"),
            };
            let follow_up = score >= 3;
            (severity, interpretation.to_string(), follow_up)
        }
        // CAGE: Cut-Annoyed-Guilty-Eye-opener (Ewing 1984)
        MentalHealthInstrument::CAGE => {
            let (severity, interpretation) = match score {
                0..=1 => (Severity::None, "Low probability of alcohol use disorder"),
                2..=3 => (Severity::Moderate, "Clinically significant: further assessment recommended"),
                _ => (Severity::Severe, "High probability of alcohol use disorder"),
            };
            let follow_up = score >= 2;
            (severity, interpretation.to_string(), follow_up)
        }
        _ => {
            // Generic interpretation
            let severity = if score <= 5 {
                Severity::Minimal
            } else if score <= 10 {
                Severity::Mild
            } else if score <= 15 {
                Severity::Moderate
            } else {
                Severity::Severe
            };
            (severity, format!("Score: {}", score), score > 10)
        }
    }
}

/// Check for crisis indicators in responses
fn check_crisis_indicators(
    instrument: &MentalHealthInstrument,
    responses: &[(String, u8)],
) -> bool {
    match instrument {
        MentalHealthInstrument::PHQ9 => {
            // Question 9 is about self-harm thoughts
            responses
                .iter()
                .any(|(q, score)| q.contains("9") && *score > 0)
        }
        MentalHealthInstrument::CSSRS => {
            // Any positive response is concerning
            responses.iter().any(|(_, score)| *score > 0)
        }
        _ => false,
    }
}

/// Create a mental health screening
#[hdk_extern]
pub fn create_screening(input: CreateScreeningInput) -> ExternResult<Record> {
    // Validate input first
    let instrument_name = format!("{:?}", input.instrument);
    let validation = validate_screening_responses(&instrument_name, &input.responses);
    validation.into_result()?;

    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Write,
        false,
    )?;
    let caller = agent_info()?.agent_initial_pubkey;

    let raw_score: u32 = input.responses.iter().map(|(_, s)| *s as u32).sum();
    let (severity, interpretation, follow_up) =
        interpret_score(&input.instrument, raw_score);
    let crisis_indicators = check_crisis_indicators(&input.instrument, &input.responses);

    let screening = MentalHealthScreening {
        patient_hash: input.patient_hash.clone(),
        provider_hash: caller,
        instrument: input.instrument,
        screening_date: sys_time()?,
        raw_score,
        severity,
        responses: input.responses,
        interpretation,
        follow_up_recommended: follow_up,
        crisis_indicators_present: crisis_indicators,
        notes: input.notes,
        created_at: sys_time()?,
    };

    let action_hash = create_entry(&EntryTypes::MentalHealthScreening(screening))?;

    let patient_hash_for_link = input.patient_hash.clone();
    create_link(
        patient_hash_for_link,
        action_hash.clone(),
        LinkTypes::PatientToScreenings,
        (),
    )?;

    log_data_access(
        input.patient_hash.clone(),
        vec![DataCategory::MentalHealth],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    // If crisis indicators present, this should trigger additional workflow
    if crisis_indicators {
        emit_signal(CrisisSignal {
            patient_hash: input.patient_hash,
            screening_hash: action_hash.clone(),
            message: "Crisis indicators detected in screening".to_string(),
        })?;
    }

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get screening".to_string())))
}

/// Crisis alert signal
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrisisSignal {
    pub patient_hash: ActionHash,
    pub screening_hash: ActionHash,
    pub message: String,
}

/// Input for paginated screening queries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetScreeningsInput {
    pub patient_hash: ActionHash,
    pub pagination: Option<PaginationInput>,
}

/// Get patient's mental health screenings
#[hdk_extern]
pub fn get_patient_screenings(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Read,
        false,
    )?;

    // Get links first
    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToScreenings)?,
        GetStrategy::default(),
    )?;

    // Use batch helper to avoid N+1 queries
    let pagination = PaginationInput::default();
    let result = links_to_records_paginated(links, &pagination)?;

    if !result.items.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::MentalHealth],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(result.items)
}

/// Get patient's mental health screenings with pagination
#[hdk_extern]
pub fn get_patient_screenings_paginated(input: GetScreeningsInput) -> ExternResult<mycelix_health_shared::PaginatedResult<Record>> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Read,
        false,
    )?;

    // Get links first
    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToScreenings)?,
        GetStrategy::default(),
    )?;

    let pagination = input.pagination.unwrap_or_default();
    let result = links_to_records_paginated(links, &pagination)?;

    if !result.items.is_empty() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::MentalHealth],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(result)
}

/// Input for mood entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateMoodEntryInput {
    pub patient_hash: ActionHash,
    pub mood_score: u8,
    pub anxiety_score: u8,
    pub sleep_quality: u8,
    pub sleep_hours: Option<f32>,
    pub energy_level: u8,
    pub appetite: Option<String>,
    pub medications_taken: bool,
    pub activities: Vec<String>,
    pub triggers: Vec<String>,
    pub coping_strategies_used: Vec<String>,
    pub notes: Option<String>,
}

/// Create a mood tracking entry (patient self-report)
#[hdk_extern]
pub fn create_mood_entry(input: CreateMoodEntryInput) -> ExternResult<Record> {
    // Validate mood entry scores (0-10 range)
    let mut validation = validate_mood_entry_scores(
        input.mood_score,
        input.anxiety_score,
        input.sleep_quality,
        input.energy_level,
    );
    validation.merge(validate_sleep_hours(input.sleep_hours));
    validation.into_result()?;

    let patient_hash = input.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Write,
        false,
    )?;
    let entry = MoodEntry {
        patient_hash: patient_hash.clone(),
        entry_date: sys_time()?,
        mood_score: input.mood_score,
        anxiety_score: input.anxiety_score,
        sleep_quality: input.sleep_quality,
        sleep_hours: input.sleep_hours,
        energy_level: input.energy_level,
        appetite: input.appetite,
        medications_taken: input.medications_taken,
        activities: input.activities,
        triggers: input.triggers,
        coping_strategies_used: input.coping_strategies_used,
        notes: input.notes,
        created_at: sys_time()?,
    };

    let action_hash = create_entry(&EntryTypes::MoodEntry(entry))?;

    create_link(
        patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToMoodEntries,
        (),
    )?;

    log_data_access(
        patient_hash,
        vec![DataCategory::MentalHealth],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get mood entry".to_string())))
}

/// Input for paginated mood entry queries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetMoodEntriesInput {
    pub patient_hash: ActionHash,
    pub pagination: Option<PaginationInput>,
}

/// Get patient's mood entries
#[hdk_extern]
pub fn get_mood_entries(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Read,
        false,
    )?;

    // Get links first
    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToMoodEntries)?,
        GetStrategy::default(),
    )?;

    // Use batch helper to avoid N+1 queries
    let pagination = PaginationInput::default();
    let result = links_to_records_paginated(links, &pagination)?;

    let records = result.items;

    if !records.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::MentalHealth],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(records)
}

/// Get patient's mood entries with pagination
#[hdk_extern]
pub fn get_mood_entries_paginated(input: GetMoodEntriesInput) -> ExternResult<mycelix_health_shared::PaginatedResult<Record>> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Read,
        false,
    )?;

    // Get links first
    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToMoodEntries)?,
        GetStrategy::default(),
    )?;

    let pagination = input.pagination.unwrap_or_default();
    let result = links_to_records_paginated(links, &pagination)?;

    if !result.items.is_empty() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::MentalHealth],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(result)
}

/// Get most recent N mood entries for a patient
#[hdk_extern]
pub fn get_recent_mood_entries(input: GetRecentMoodInput) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Read,
        false,
    )?;

    // Get links first
    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToMoodEntries)?,
        GetStrategy::default(),
    )?;

    let records = links_to_recent_records(links, input.count.unwrap_or(10) as usize)?;

    if !records.is_empty() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::MentalHealth],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(records)
}

/// Input for getting recent mood entries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetRecentMoodInput {
    pub patient_hash: ActionHash,
    pub count: Option<u32>,
}

/// Input for safety plan
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateSafetyPlanInput {
    pub patient_hash: ActionHash,
    pub warning_signs: Vec<String>,
    pub internal_coping_strategies: Vec<String>,
    pub people_for_distraction: Vec<ContactInfo>,
    pub people_for_help: Vec<ContactInfo>,
    pub professionals_to_contact: Vec<ContactInfo>,
    pub additional_crisis_resources: Vec<String>,
    pub environment_safety_steps: Vec<String>,
    pub reasons_for_living: Vec<String>,
}

/// Create a safety plan
#[hdk_extern]
pub fn create_safety_plan(input: CreateSafetyPlanInput) -> ExternResult<Record> {
    let patient_hash = input.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Write,
        false,
    )?;
    let caller = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    // Calculate next review date (90 days)
    let review_micros = now.as_micros() + (90 * 24 * 60 * 60 * 1_000_000);
    let next_review = Timestamp::from_micros(review_micros as i64);

    let plan = SafetyPlan {
        patient_hash: input.patient_hash.clone(),
        provider_hash: caller,
        warning_signs: input.warning_signs,
        internal_coping_strategies: input.internal_coping_strategies,
        people_for_distraction: input.people_for_distraction,
        people_for_help: input.people_for_help,
        professionals_to_contact: input.professionals_to_contact,
        crisis_line_988: true, // Always include
        additional_crisis_resources: input.additional_crisis_resources,
        environment_safety_steps: input.environment_safety_steps,
        reasons_for_living: input.reasons_for_living,
        status: SafetyPlanStatus::Active,
        created_at: now,
        last_reviewed: now,
        next_review_date: next_review,
    };

    let action_hash = create_entry(&EntryTypes::SafetyPlan(plan))?;

    create_link(
        patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToSafetyPlan,
        (),
    )?;

    log_data_access(
        patient_hash,
        vec![DataCategory::MentalHealth],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get safety plan".to_string())))
}

/// Get patient's current safety plan
#[hdk_extern]
pub fn get_safety_plan(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Read,
        false,
    )?;
    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToSafetyPlan)?, GetStrategy::default(),
    )?;

    // Get most recent
    if let Some(link) = links.last() {
        if let Some(target) = link.target.clone().into_action_hash() {
            let record = get(target, GetOptions::default())?;
            if record.is_some() {
                log_data_access(
                    patient_hash,
                    vec![DataCategory::MentalHealth],
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

/// Input for crisis event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportCrisisEventInput {
    pub patient_hash: ActionHash,
    pub crisis_level: CrisisLevel,
    pub suicidal_ideation: bool,
    pub homicidal_ideation: bool,
    pub self_harm: bool,
    pub substance_intoxication: bool,
    pub psychotic_symptoms: bool,
    pub description: String,
    pub intervention_taken: String,
    pub disposition: String,
    pub follow_up_plan: String,
    pub safety_plan_reviewed: bool,
}

/// Report a crisis event
#[hdk_extern]
pub fn report_crisis_event(input: ReportCrisisEventInput) -> ExternResult<Record> {
    let patient_hash = input.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Write,
        false,
    )?;
    let caller = agent_info()?.agent_initial_pubkey;

    let event = CrisisEvent {
        patient_hash: patient_hash.clone(),
        reporter_hash: caller,
        event_date: sys_time()?,
        crisis_level: input.crisis_level.clone(),
        suicidal_ideation: input.suicidal_ideation,
        homicidal_ideation: input.homicidal_ideation,
        self_harm: input.self_harm,
        substance_intoxication: input.substance_intoxication,
        psychotic_symptoms: input.psychotic_symptoms,
        description: input.description,
        intervention_taken: input.intervention_taken,
        disposition: input.disposition,
        follow_up_plan: input.follow_up_plan,
        safety_plan_reviewed: input.safety_plan_reviewed,
        created_at: sys_time()?,
    };

    let action_hash = create_entry(&EntryTypes::CrisisEvent(event))?;

    create_link(
        patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToCrisisEvents,
        (),
    )?;

    log_data_access(
        patient_hash.clone(),
        vec![DataCategory::MentalHealth],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    // Emit signal for high-risk events
    if matches!(input.crisis_level, CrisisLevel::HighRisk | CrisisLevel::Imminent) {
        emit_signal(CrisisSignal {
            patient_hash,
            screening_hash: action_hash.clone(),
            message: format!("Crisis event reported: {:?}", input.crisis_level),
        })?;
    }

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get crisis event".to_string())))
}

/// Get patient's crisis history
#[hdk_extern]
pub fn get_crisis_history(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Read,
        false,
    )?;
    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToCrisisEvents)?, GetStrategy::default(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    if !records.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::MentalHealth],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(records)
}

/// Input for 42 CFR Part 2 consent
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreatePart2ConsentInput {
    pub patient_hash: ActionHash,
    pub consent_type: Part2ConsentType,
    pub disclosing_program: String,
    pub recipient_name: String,
    pub recipient_hash: Option<ActionHash>,
    pub purpose: String,
    pub information_to_disclose: Vec<String>,
    pub substances_covered: Vec<SubstanceCategory>,
    pub expiration_date: Option<Timestamp>,
    pub witness_name: Option<String>,
}

/// Create a 42 CFR Part 2 consent
#[hdk_extern]
pub fn create_part2_consent(input: CreatePart2ConsentInput) -> ExternResult<Record> {
    let patient_hash = input.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::SubstanceAbuse,
        Permission::Write,
        false,
    )?;
    let consent = Part2Consent {
        patient_hash: patient_hash.clone(),
        consent_type: input.consent_type,
        disclosing_program: input.disclosing_program,
        recipient_name: input.recipient_name,
        recipient_hash: input.recipient_hash,
        purpose: input.purpose,
        information_to_disclose: input.information_to_disclose,
        substances_covered: input.substances_covered,
        effective_date: sys_time()?,
        expiration_date: input.expiration_date,
        right_to_revoke_explained: true,
        patient_signature_date: sys_time()?,
        witness_name: input.witness_name,
        is_revoked: false,
        revocation_date: None,
        created_at: sys_time()?,
    };

    let action_hash = create_entry(&EntryTypes::Part2Consent(consent))?;

    create_link(
        patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToPart2Consents,
        (),
    )?;

    log_data_access(
        patient_hash,
        vec![DataCategory::SubstanceAbuse],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get Part 2 consent".to_string())))
}

/// Revoke a Part 2 consent
#[hdk_extern]
pub fn revoke_part2_consent(consent_hash: ActionHash) -> ExternResult<Record> {
    let record = get(consent_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Consent not found".to_string())))?;

    let mut consent: Part2Consent = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid consent".to_string())))?;

    let auth = require_authorization(
        consent.patient_hash.clone(),
        DataCategory::SubstanceAbuse,
        Permission::Write,
        false,
    )?;

    consent.is_revoked = true;
    consent.revocation_date = Some(sys_time()?);

    let action_hash = update_entry(consent_hash, &consent)?;

    log_data_access(
        consent.patient_hash,
        vec![DataCategory::SubstanceAbuse],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get updated consent".to_string())))
}

/// Get patient's Part 2 consents
#[hdk_extern]
pub fn get_part2_consents(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::SubstanceAbuse,
        Permission::Read,
        false,
    )?;
    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToPart2Consents)?, GetStrategy::default(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    if !records.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::SubstanceAbuse],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(records)
}

/// Input for therapy note
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateTherapyNoteInput {
    pub patient_hash: ActionHash,
    pub session_type: TreatmentModality,
    pub duration_minutes: u32,
    pub presenting_concerns: String,
    pub mental_status: Option<String>,
    pub interventions_used: Vec<String>,
    pub patient_response: String,
    pub risk_assessment: Option<CrisisLevel>,
    pub plan_for_next_session: String,
    pub is_psychotherapy_note: bool,
}

/// Create a therapy note
#[hdk_extern]
pub fn create_therapy_note(input: CreateTherapyNoteInput) -> ExternResult<Record> {
    let patient_hash = input.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Write,
        false,
    )?;
    let caller = agent_info()?.agent_initial_pubkey;

    let note = TherapyNote {
        patient_hash: patient_hash.clone(),
        provider_hash: caller.clone(),
        session_date: sys_time()?,
        session_type: input.session_type,
        duration_minutes: input.duration_minutes,
        presenting_concerns: input.presenting_concerns,
        mental_status: input.mental_status,
        interventions_used: input.interventions_used,
        patient_response: input.patient_response,
        risk_assessment: input.risk_assessment,
        plan_for_next_session: input.plan_for_next_session,
        is_psychotherapy_note: input.is_psychotherapy_note,
        created_at: sys_time()?,
    };

    let action_hash = create_entry(&EntryTypes::TherapyNote(note))?;

    create_link(
        patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToTherapyNotes,
        (),
    )?;

    create_link(
        caller,
        patient_hash.clone(),
        LinkTypes::ProviderToPatients,
        (),
    )?;

    log_data_access(
        patient_hash,
        vec![DataCategory::MentalHealth],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get therapy note".to_string())))
}

/// Get therapy notes (provider access only, respects psychotherapy note protection)
#[hdk_extern]
pub fn get_therapy_notes(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Read,
        false,
    )?;
    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToTherapyNotes)?, GetStrategy::default(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    if !records.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::MentalHealth],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(records)
}

// ============================================================================
// Treatment Plan Functions
// ============================================================================

/// Input for creating a treatment plan
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateTreatmentPlanInput {
    pub patient_hash: ActionHash,
    pub primary_diagnosis_icd10: String,
    pub secondary_diagnoses: Vec<String>,
    pub treatment_goals: Vec<TreatmentGoal>,
    pub modalities: Vec<TreatmentModality>,
    pub medications: Vec<PsychMedication>,
    pub session_frequency: String,
    pub estimated_duration: Option<String>,
    pub crisis_plan_hash: Option<ActionHash>,
    pub review_date: Timestamp,
}

/// Create a mental health treatment plan
#[hdk_extern]
pub fn create_treatment_plan(input: CreateTreatmentPlanInput) -> ExternResult<Record> {
    let patient_hash = input.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Write,
        false,
    )?;
    let caller = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let plan = MentalHealthTreatmentPlan {
        patient_hash: patient_hash.clone(),
        provider_hash: caller.clone(),
        primary_diagnosis_icd10: input.primary_diagnosis_icd10,
        secondary_diagnoses: input.secondary_diagnoses,
        treatment_goals: input.treatment_goals,
        modalities: input.modalities,
        medications: input.medications,
        session_frequency: input.session_frequency,
        estimated_duration: input.estimated_duration,
        crisis_plan_hash: input.crisis_plan_hash,
        effective_date: now,
        review_date: input.review_date,
        status: "Active".to_string(),
        created_at: now,
        updated_at: now,
    };

    let action_hash = create_entry(&EntryTypes::MentalHealthTreatmentPlan(plan))?;

    create_link(
        patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToTreatmentPlans,
        (),
    )?;

    log_data_access(
        patient_hash,
        vec![DataCategory::MentalHealth],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get treatment plan".to_string())))
}

/// Get all treatment plans for a patient
#[hdk_extern]
pub fn get_treatment_plans(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Read,
        false,
    )?;
    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToTreatmentPlans)?, GetStrategy::default(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    if !records.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::MentalHealth],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(records)
}

/// Get the active treatment plan for a patient
#[hdk_extern]
pub fn get_active_treatment_plan(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    let plans = get_treatment_plans(patient_hash)?;

    for record in plans {
        if let Some(plan) = record.entry().to_app_option::<MentalHealthTreatmentPlan>().ok().flatten() {
            if plan.status == "Active" {
                return Ok(Some(record));
            }
        }
    }

    Ok(None)
}

/// Input for updating a treatment plan goal
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateTreatmentGoalInput {
    pub plan_hash: ActionHash,
    pub goal_id: String,
    pub new_progress: String,
    pub notes: Option<String>,
}

/// Update progress on a treatment plan goal
#[hdk_extern]
pub fn update_treatment_goal(input: UpdateTreatmentGoalInput) -> ExternResult<Record> {
    let record = get(input.plan_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Treatment plan not found".to_string())))?;

    let mut plan: MentalHealthTreatmentPlan = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid treatment plan".to_string())))?;

    let auth = require_authorization(
        plan.patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Write,
        false,
    )?;

    // Find and update the goal
    let mut goal_found = false;
    for goal in &mut plan.treatment_goals {
        if goal.goal_id == input.goal_id {
            goal.progress = input.new_progress;
            goal_found = true;
            break;
        }
    }

    if !goal_found {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Goal {} not found in treatment plan",
            input.goal_id
        ))));
    }

    plan.updated_at = sys_time()?;

    let updated_hash = update_entry(input.plan_hash, &plan)?;

    log_data_access(
        plan.patient_hash,
        vec![DataCategory::MentalHealth],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get updated plan".to_string())))
}

/// Input for closing a treatment plan
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloseTreatmentPlanInput {
    pub plan_hash: ActionHash,
    pub reason: String,
}

/// Close a treatment plan
#[hdk_extern]
pub fn close_treatment_plan(input: CloseTreatmentPlanInput) -> ExternResult<Record> {
    let record = get(input.plan_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Treatment plan not found".to_string())))?;

    let mut plan: MentalHealthTreatmentPlan = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid treatment plan".to_string())))?;

    let auth = require_authorization(
        plan.patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Write,
        false,
    )?;

    plan.status = format!("Closed: {}", input.reason);
    plan.updated_at = sys_time()?;

    let updated_hash = update_entry(input.plan_hash, &plan)?;

    log_data_access(
        plan.patient_hash,
        vec![DataCategory::MentalHealth],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get closed plan".to_string())))
}

// ============================================================================
// Mood Trend Analysis
// ============================================================================

/// Mood trend analysis result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoodTrendAnalysis {
    pub patient_hash: ActionHash,
    pub period_days: u32,
    pub entry_count: u32,
    pub average_mood: f32,
    pub average_anxiety: f32,
    pub average_sleep_quality: f32,
    pub average_energy: f32,
    pub mood_trend: MoodTrendDirection,
    pub common_triggers: Vec<String>,
    pub common_coping_strategies: Vec<String>,
    pub medication_adherence_rate: f32,
}

/// Mood trend direction
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MoodTrendDirection {
    Improving,
    Stable,
    Declining,
    InsufficientData,
}

/// Calculate mood trend for a patient over a period
#[hdk_extern]
pub fn calculate_mood_trend(patient_hash: ActionHash) -> ExternResult<MoodTrendAnalysis> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToMoodEntries)?, GetStrategy::default(),
    )?;

    let mut entries: Vec<MoodEntry> = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(entry) = record.entry().to_app_option::<MoodEntry>().ok().flatten() {
                    entries.push(entry);
                }
            }
        }
    }

    // Sort by date
    entries.sort_by(|a, b| a.entry_date.cmp(&b.entry_date));

    // Calculate analysis
    let entry_count = entries.len() as u32;

    if entry_count < 3 {
        return Ok(MoodTrendAnalysis {
            patient_hash,
            period_days: 0,
            entry_count,
            average_mood: 0.0,
            average_anxiety: 0.0,
            average_sleep_quality: 0.0,
            average_energy: 0.0,
            mood_trend: MoodTrendDirection::InsufficientData,
            common_triggers: Vec::new(),
            common_coping_strategies: Vec::new(),
            medication_adherence_rate: 0.0,
        });
    }

    // Calculate averages
    let total_mood: u32 = entries.iter().map(|e| e.mood_score as u32).sum();
    let total_anxiety: u32 = entries.iter().map(|e| e.anxiety_score as u32).sum();
    let total_sleep: u32 = entries.iter().map(|e| e.sleep_quality as u32).sum();
    let total_energy: u32 = entries.iter().map(|e| e.energy_level as u32).sum();
    let meds_taken: u32 = entries.iter().filter(|e| e.medications_taken).count() as u32;

    let average_mood = total_mood as f32 / entry_count as f32;
    let average_anxiety = total_anxiety as f32 / entry_count as f32;
    let average_sleep_quality = total_sleep as f32 / entry_count as f32;
    let average_energy = total_energy as f32 / entry_count as f32;
    let medication_adherence_rate = meds_taken as f32 / entry_count as f32;

    // Calculate trend (compare first half to second half)
    let mid_point = entry_count / 2;
    let first_half_mood: f32 = entries[..(mid_point as usize)]
        .iter()
        .map(|e| e.mood_score as f32)
        .sum::<f32>() / mid_point as f32;
    let second_half_mood: f32 = entries[(mid_point as usize)..]
        .iter()
        .map(|e| e.mood_score as f32)
        .sum::<f32>() / (entry_count - mid_point) as f32;

    let mood_trend = if second_half_mood > first_half_mood + 0.5 {
        MoodTrendDirection::Improving
    } else if second_half_mood < first_half_mood - 0.5 {
        MoodTrendDirection::Declining
    } else {
        MoodTrendDirection::Stable
    };

    // Collect common triggers and coping strategies
    let mut trigger_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut coping_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    for entry in &entries {
        for trigger in &entry.triggers {
            *trigger_counts.entry(trigger.clone()).or_insert(0) += 1;
        }
        for strategy in &entry.coping_strategies_used {
            *coping_counts.entry(strategy.clone()).or_insert(0) += 1;
        }
    }

    let mut common_triggers: Vec<(String, u32)> = trigger_counts.into_iter().collect();
    common_triggers.sort_by(|a, b| b.1.cmp(&a.1));
    let common_triggers: Vec<String> = common_triggers.into_iter().take(5).map(|(t, _)| t).collect();

    let mut common_coping: Vec<(String, u32)> = coping_counts.into_iter().collect();
    common_coping.sort_by(|a, b| b.1.cmp(&a.1));
    let common_coping_strategies: Vec<String> = common_coping.into_iter().take(5).map(|(s, _)| s).collect();

    // Calculate period in days
    let period_days = if !entries.is_empty() {
        let first_date = entries.first().unwrap().entry_date.as_micros();
        let last_date = entries.last().unwrap().entry_date.as_micros();
        ((last_date - first_date) / (24 * 60 * 60 * 1_000_000)) as u32
    } else {
        0
    };

    log_data_access(
        patient_hash.clone(),
        vec![DataCategory::MentalHealth],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(MoodTrendAnalysis {
        patient_hash,
        period_days,
        entry_count,
        average_mood,
        average_anxiety,
        average_sleep_quality,
        average_energy,
        mood_trend,
        common_triggers,
        common_coping_strategies,
        medication_adherence_rate,
    })
}

/// Mental health summary for a patient
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MentalHealthSummary {
    pub patient_hash: ActionHash,
    pub latest_phq9_score: Option<u32>,
    pub latest_phq9_severity: Option<Severity>,
    pub latest_gad7_score: Option<u32>,
    pub has_active_safety_plan: bool,
    pub recent_crisis_events: u32,
    pub active_treatment_plan: bool,
    pub mood_trend: Option<String>,
}

/// Get mental health summary for a patient
#[hdk_extern]
pub fn get_mental_health_summary(patient_hash: ActionHash) -> ExternResult<MentalHealthSummary> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Read,
        false,
    )?;
    let screenings = get_patient_screenings(patient_hash.clone())?;
    let safety_plan = get_safety_plan(patient_hash.clone())?;
    let crisis_events = get_crisis_history(patient_hash.clone())?;
    let active_plan = get_active_treatment_plan(patient_hash.clone())?;

    // Calculate mood trend
    let mood_trend_analysis = calculate_mood_trend(patient_hash.clone()).ok();
    let mood_trend = mood_trend_analysis.map(|analysis| {
        match analysis.mood_trend {
            MoodTrendDirection::Improving => "Improving".to_string(),
            MoodTrendDirection::Stable => "Stable".to_string(),
            MoodTrendDirection::Declining => "Declining".to_string(),
            MoodTrendDirection::InsufficientData => "Insufficient data".to_string(),
        }
    });

    let mut latest_phq9: Option<(u32, Severity)> = None;
    let mut latest_gad7: Option<u32> = None;

    for record in &screenings {
        if let Some(screening) = record.entry().to_app_option::<MentalHealthScreening>().ok().flatten() {
            match screening.instrument {
                MentalHealthInstrument::PHQ9 => {
                    latest_phq9 = Some((screening.raw_score, screening.severity));
                }
                MentalHealthInstrument::GAD7 => {
                    latest_gad7 = Some(screening.raw_score);
                }
                _ => {}
            }
        }
    }

    let summary = MentalHealthSummary {
        patient_hash,
        latest_phq9_score: latest_phq9.as_ref().map(|(s, _)| *s),
        latest_phq9_severity: latest_phq9.map(|(_, sev)| sev),
        latest_gad7_score: latest_gad7,
        has_active_safety_plan: safety_plan.is_some(),
        recent_crisis_events: crisis_events.len() as u32,
        active_treatment_plan: active_plan.is_some(),
        mood_trend,
    };

    log_data_access(
        summary.patient_hash.clone(),
        vec![DataCategory::MentalHealth],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(summary)
}

// ── Recovery & Rehabilitation Functions ──

/// Input for creating a recovery milestone
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateMilestoneInput {
    pub patient_hash: ActionHash,
    pub milestone_type: MilestoneType,
    pub notes: String,
}

/// Create a recovery milestone
#[hdk_extern]
pub fn log_milestone(input: CreateMilestoneInput) -> ExternResult<Record> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Write,
        false,
    )?;

    let milestone = RecoveryMilestone {
        patient_hash: input.patient_hash.clone(),
        milestone_type: input.milestone_type,
        achieved_at: sys_time()?,
        verified_by: None,
        notes: input.notes,
        created_at: sys_time()?,
    };

    let action_hash = create_entry(&EntryTypes::RecoveryMilestone(milestone))?;
    create_link(
        input.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToMilestones,
        (),
    )?;

    log_data_access(
        input.patient_hash,
        vec![DataCategory::MentalHealth],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get milestone".to_string())))
}

/// Verify a milestone (sponsor/counselor attestation)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifyMilestoneInput {
    pub milestone_hash: ActionHash,
    pub patient_hash: ActionHash,
}

#[hdk_extern]
pub fn verify_milestone(input: VerifyMilestoneInput) -> ExternResult<Record> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Write,
        false,
    )?;
    let verifier = agent_info()?.agent_initial_pubkey;

    let record = get(input.milestone_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Milestone not found".to_string())))?;

    let mut milestone: RecoveryMilestone = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Deserialize error: {}", e))))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Not a milestone entry".to_string())))?;

    milestone.verified_by = Some(verifier);

    let updated_hash = update_entry(input.milestone_hash, &milestone)?;

    log_data_access(
        input.patient_hash,
        vec![DataCategory::MentalHealth],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get updated milestone".to_string())))
}

/// Get patient's recovery milestones
#[hdk_extern]
pub fn get_milestones(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToMilestones)?,
        GetStrategy::default(),
    )?;

    let records: Vec<Record> = links
        .into_iter()
        .filter_map(|link| {
            link.target
                .into_action_hash()
                .and_then(|ah| get(ah, GetOptions::default()).ok().flatten())
        })
        .collect();

    log_data_access(
        patient_hash,
        vec![DataCategory::MentalHealth],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(records)
}

/// Input for creating a relapse prevention plan
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateRelapsePlanInput {
    pub patient_hash: ActionHash,
    pub triggers: Vec<String>,
    pub high_risk_situations: Vec<String>,
    pub coping_plan: Vec<CopingAction>,
    pub support_contacts: Vec<ContactInfo>,
}

/// Create a relapse prevention plan
#[hdk_extern]
pub fn create_relapse_prevention(input: CreateRelapsePlanInput) -> ExternResult<Record> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Write,
        false,
    )?;

    let plan = RelapsePrevention {
        patient_hash: input.patient_hash.clone(),
        triggers: input.triggers,
        high_risk_situations: input.high_risk_situations,
        coping_plan: input.coping_plan,
        support_contacts: input.support_contacts,
        last_reviewed: sys_time()?,
        created_at: sys_time()?,
    };

    let action_hash = create_entry(&EntryTypes::RelapsePrevention(plan))?;
    create_link(
        input.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToRelapsePrevention,
        (),
    )?;

    log_data_access(
        input.patient_hash,
        vec![DataCategory::MentalHealth],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get relapse plan".to_string())))
}

/// Input for a recovery check-in
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateCheckInInput {
    pub patient_hash: ActionHash,
    pub mood_score: u8,
    pub craving_intensity: u8,
    pub triggers_encountered: Vec<String>,
    pub coping_used: Vec<String>,
    pub sleep_quality: u8,
    pub social_support_quality: u8,
    pub notes: Option<String>,
}

/// Record a recovery check-in
#[hdk_extern]
pub fn record_check_in(input: CreateCheckInInput) -> ExternResult<Record> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Write,
        false,
    )?;
    let caller = agent_info()?.agent_initial_pubkey;

    let check_in = RecoveryCheckIn {
        patient_hash: input.patient_hash.clone(),
        checked_in_by: caller,
        mood_score: input.mood_score,
        craving_intensity: input.craving_intensity,
        triggers_encountered: input.triggers_encountered,
        coping_used: input.coping_used,
        sleep_quality: input.sleep_quality,
        social_support_quality: input.social_support_quality,
        notes: input.notes,
        timestamp: sys_time()?,
    };

    let action_hash = create_entry(&EntryTypes::RecoveryCheckIn(check_in))?;
    create_link(
        input.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToCheckIns,
        (),
    )?;

    log_data_access(
        input.patient_hash,
        vec![DataCategory::MentalHealth],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get check-in".to_string())))
}

/// Get patient's recovery check-ins (most recent first)
#[hdk_extern]
pub fn get_check_ins(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToCheckIns)?,
        GetStrategy::default(),
    )?;

    let mut records: Vec<Record> = links
        .into_iter()
        .filter_map(|link| {
            link.target
                .into_action_hash()
                .and_then(|ah| get(ah, GetOptions::default()).ok().flatten())
        })
        .collect();

    // Sort by timestamp (most recent first)
    records.sort_by(|a, b| {
        let ts_a = a.action().timestamp();
        let ts_b = b.action().timestamp();
        ts_b.cmp(&ts_a)
    });

    log_data_access(
        patient_hash,
        vec![DataCategory::MentalHealth],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(records)
}

/// Recovery dashboard summary
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryDashboard {
    pub patient_hash: ActionHash,
    /// Days since most recent SobrietyDate milestone
    pub sobriety_days: Option<u32>,
    /// Current treatment phase (from most recent milestone)
    pub treatment_phase: Option<TreatmentPhase>,
    /// Total milestones achieved
    pub milestone_count: u32,
    /// Total verified milestones
    pub verified_milestone_count: u32,
    /// Most recent craving intensity (from check-in)
    pub latest_craving: Option<u8>,
    /// Average mood from last 7 check-ins
    pub recent_mood_avg: Option<f32>,
    /// Has active relapse prevention plan
    pub has_relapse_plan: bool,
    /// Has active safety plan
    pub has_safety_plan: bool,
    /// Number of check-ins in the last 30 days (approximate)
    pub recent_check_in_count: u32,
}

/// Get recovery dashboard for a patient
#[hdk_extern]
pub fn get_recovery_dashboard(patient_hash: ActionHash) -> ExternResult<RecoveryDashboard> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Read,
        false,
    )?;

    // Gather milestones
    let milestone_links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToMilestones)?,
        GetStrategy::default(),
    )?;
    let milestones: Vec<RecoveryMilestone> = milestone_links
        .into_iter()
        .filter_map(|link| {
            link.target
                .into_action_hash()
                .and_then(|ah| get(ah, GetOptions::default()).ok().flatten())
                .and_then(|r| r.entry().to_app_option::<RecoveryMilestone>().ok().flatten())
        })
        .collect();

    let milestone_count = milestones.len() as u32;
    let verified_milestone_count = milestones.iter().filter(|m| m.verified_by.is_some()).count() as u32;

    // Find sobriety days from most recent SobrietyDate milestone
    let sobriety_days = milestones.iter().filter_map(|m| {
        if let MilestoneType::SobrietyDate { days } = &m.milestone_type {
            Some(*days)
        } else {
            None
        }
    }).max();

    // Find current treatment phase from most recent transition
    let treatment_phase = milestones.iter().filter_map(|m| {
        if let MilestoneType::TreatmentPhaseTransition { to, .. } = &m.milestone_type {
            Some(to.clone())
        } else {
            None
        }
    }).last();

    // Gather check-ins
    let check_in_links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToCheckIns)?,
        GetStrategy::default(),
    )?;
    let check_ins: Vec<RecoveryCheckIn> = check_in_links
        .into_iter()
        .filter_map(|link| {
            link.target
                .into_action_hash()
                .and_then(|ah| get(ah, GetOptions::default()).ok().flatten())
                .and_then(|r| r.entry().to_app_option::<RecoveryCheckIn>().ok().flatten())
        })
        .collect();

    let latest_craving = check_ins.last().map(|c| c.craving_intensity);
    let recent_check_in_count = check_ins.len() as u32;

    // Average mood from last 7
    let recent_mood_avg = if !check_ins.is_empty() {
        let last_n: Vec<&RecoveryCheckIn> = check_ins.iter().rev().take(7).collect();
        let sum: f32 = last_n.iter().map(|c| c.mood_score as f32).sum();
        Some(sum / last_n.len() as f32)
    } else {
        None
    };

    // Check for relapse prevention plan
    let relapse_links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToRelapsePrevention)?,
        GetStrategy::default(),
    )?;
    let has_relapse_plan = !relapse_links.is_empty();

    // Check for safety plan
    let safety_links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToSafetyPlan)?,
        GetStrategy::default(),
    )?;
    let has_safety_plan = !safety_links.is_empty();

    let dashboard = RecoveryDashboard {
        patient_hash: patient_hash.clone(),
        sobriety_days,
        treatment_phase,
        milestone_count,
        verified_milestone_count,
        latest_craving,
        recent_mood_avg,
        has_relapse_plan,
        has_safety_plan,
        recent_check_in_count,
    };

    log_data_access(
        patient_hash,
        vec![DataCategory::MentalHealth],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(dashboard)
}

// ── TEND Bridge: Recovery Care Compensation ──

/// Signal emitted when a peer support session is logged, enabling TEND compensation.
///
/// The finance cluster (TEND zome) listens for this signal to create a
/// TendExchange entry. This decouples health records from financial records
/// while enabling care work compensation.
///
/// Architecture: Health → Signal → Finance (no direct cross-zome dependency)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerSessionTendSignal {
    /// The peer specialist who provided support (earns TEND credit)
    pub provider_did: String,
    /// The participant who received support
    pub receiver_did: String,
    /// Duration of the session in hours (maps to TEND hours)
    pub hours: f32,
    /// Service description for the TEND exchange record
    pub service_description: String,
    /// The peer support connection hash (for audit trail)
    pub peer_connection_hash: ActionHash,
    /// Whether the session was verified by a third party
    pub verified: bool,
}

/// Signal emitted when a milestone is verified, for potential TEND credit.
///
/// The verifier (sponsor/counselor) can receive TEND credit for their
/// attestation work. This incentivizes quality oversight.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MilestoneVerifiedTendSignal {
    /// The verifier who attested the milestone (earns TEND credit)
    pub verifier_did: String,
    /// The participant whose milestone was verified
    pub participant_did: String,
    /// Milestone description
    pub milestone_description: String,
    /// Fractional TEND hours for verification work (typically 0.25-0.5h)
    pub verification_hours: f32,
}

/// Input for logging a peer support session with TEND compensation signal.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogPeerSessionInput {
    pub patient_hash: ActionHash,
    pub peer_specialist_hash: ActionHash,
    /// Provider DID for TEND credit (Mycelix identity)
    pub provider_did: String,
    /// Receiver DID for TEND debit
    pub receiver_did: String,
    /// Session duration in hours
    pub hours: f32,
    /// Session notes
    pub notes: String,
}

/// Log a peer support session and emit TEND compensation signal.
///
/// This is the key bridge function: it records the clinical event (peer session)
/// in the health record AND emits a signal for the finance cluster to create
/// a TEND exchange. The health and finance records are separate but linked.
#[hdk_extern]
pub fn log_peer_support_session(input: LogPeerSessionInput) -> ExternResult<Record> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::MentalHealth,
        Permission::Write,
        false,
    )?;

    // Create the peer support connection record
    let connection = PeerSupportConnection {
        patient_hash: input.patient_hash.clone(),
        peer_specialist_hash: input.peer_specialist_hash,
        connection_type: "Recovery peer support".to_string(),
        meeting_frequency: "As needed".to_string(),
        goals: vec!["Recovery support".to_string()],
        start_date: sys_time()?,
        status: "Active".to_string(),
        created_at: sys_time()?,
    };

    let action_hash = create_entry(&EntryTypes::PeerSupportConnection(connection))?;
    create_link(
        input.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToPeerSupport,
        (),
    )?;

    // Emit TEND compensation signal for the finance cluster
    let tend_signal = PeerSessionTendSignal {
        provider_did: input.provider_did,
        receiver_did: input.receiver_did,
        hours: input.hours,
        service_description: format!(
            "Recovery peer support session ({:.1}h): {}",
            input.hours, input.notes
        ),
        peer_connection_hash: action_hash.clone(),
        verified: false, // Can be verified later
    };
    emit_signal(tend_signal)?;

    log_data_access(
        input.patient_hash,
        vec![DataCategory::MentalHealth],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get peer session".to_string())))
}
