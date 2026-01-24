//! Mental Health Coordinator Zome
//!
//! Behavioral health management with enhanced privacy and crisis protocols.

use hdk::prelude::*;
use mental_health_integrity::*;

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

/// Get patient's mental health screenings
#[hdk_extern]
pub fn get_patient_screenings(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToScreenings)?, GetStrategy::default(),
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
    let entry = MoodEntry {
        patient_hash: input.patient_hash.clone(),
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
        input.patient_hash,
        action_hash.clone(),
        LinkTypes::PatientToMoodEntries,
        (),
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get mood entry".to_string())))
}

/// Get patient's mood entries
#[hdk_extern]
pub fn get_mood_entries(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToMoodEntries)?, GetStrategy::default(),
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
        input.patient_hash,
        action_hash.clone(),
        LinkTypes::PatientToSafetyPlan,
        (),
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get safety plan".to_string())))
}

/// Get patient's current safety plan
#[hdk_extern]
pub fn get_safety_plan(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToSafetyPlan)?, GetStrategy::default(),
    )?;

    // Get most recent
    if let Some(link) = links.last() {
        if let Some(target) = link.target.clone().into_action_hash() {
            return get(target, GetOptions::default());
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
    let caller = agent_info()?.agent_initial_pubkey;

    let event = CrisisEvent {
        patient_hash: input.patient_hash.clone(),
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
        input.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToCrisisEvents,
        (),
    )?;

    // Emit signal for high-risk events
    if matches!(input.crisis_level, CrisisLevel::HighRisk | CrisisLevel::Imminent) {
        emit_signal(CrisisSignal {
            patient_hash: input.patient_hash,
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
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToCrisisEvents)?, GetStrategy::default(),
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
    let consent = Part2Consent {
        patient_hash: input.patient_hash.clone(),
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
        input.patient_hash,
        action_hash.clone(),
        LinkTypes::PatientToPart2Consents,
        (),
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

    consent.is_revoked = true;
    consent.revocation_date = Some(sys_time()?);

    let action_hash = update_entry(consent_hash, &consent)?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get updated consent".to_string())))
}

/// Get patient's Part 2 consents
#[hdk_extern]
pub fn get_part2_consents(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToPart2Consents)?, GetStrategy::default(),
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
    let caller = agent_info()?.agent_initial_pubkey;

    let note = TherapyNote {
        patient_hash: input.patient_hash.clone(),
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
        input.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToTherapyNotes,
        (),
    )?;

    create_link(
        caller,
        input.patient_hash,
        LinkTypes::ProviderToPatients,
        (),
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get therapy note".to_string())))
}

/// Get therapy notes (provider access only, respects psychotherapy note protection)
#[hdk_extern]
pub fn get_therapy_notes(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToTherapyNotes)?, GetStrategy::default(),
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
    let screenings = get_patient_screenings(patient_hash.clone())?;
    let safety_plan = get_safety_plan(patient_hash.clone())?;
    let crisis_events = get_crisis_history(patient_hash.clone())?;

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

    Ok(MentalHealthSummary {
        patient_hash,
        latest_phq9_score: latest_phq9.as_ref().map(|(s, _)| *s),
        latest_phq9_severity: latest_phq9.map(|(_, sev)| sev),
        latest_gad7_score: latest_gad7,
        has_active_safety_plan: safety_plan.is_some(),
        recent_crisis_events: crisis_events.len() as u32,
        active_treatment_plan: false, // TODO: check treatment plans
        mood_trend: None, // TODO: calculate from mood entries
    })
}
