//! AI Health Advocate Coordinator Zome
//!
//! Provides extern functions for the AI Health Advocate system.

use advocate_integrity::*;
use hdk::prelude::*;

// ==================== APPOINTMENT PREPARATION ====================

/// Create appointment preparation
#[hdk_extern]
pub fn create_appointment_prep(prep: AppointmentPrep) -> ExternResult<Record> {
    validate_appointment_prep(&prep)?;

    let prep_hash = create_entry(&EntryTypes::AppointmentPrep(prep.clone()))?;
    let record = get(prep_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find prep".to_string())
    ))?;

    // Link to patient
    create_link(
        prep.patient_hash.clone(),
        prep_hash.clone(),
        LinkTypes::PatientToPreps,
        (),
    )?;

    Ok(record)
}

/// Get patient's appointment preps
#[hdk_extern]
pub fn get_patient_preps(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToPreps)?,
        GetStrategy::default(),
    )?;

    let mut preps = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                preps.push(record);
            }
        }
    }

    Ok(preps)
}

/// Mark prep as reviewed
#[hdk_extern]
pub fn mark_prep_reviewed(input: MarkPrepReviewedInput) -> ExternResult<Record> {
    let record = get(input.prep_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Prep not found".to_string())
    ))?;

    let mut prep: AppointmentPrep = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid prep".to_string()
        )))?;

    prep.reviewed = true;
    prep.patient_notes = input.patient_notes;

    let updated_hash = update_entry(input.prep_hash, &prep)?;

    get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Could not find updated prep".to_string()
    )))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MarkPrepReviewedInput {
    pub prep_hash: ActionHash,
    pub patient_notes: Option<String>,
}

// ==================== HEALTH INSIGHTS ====================

/// Create health insight
#[hdk_extern]
pub fn create_health_insight(insight: HealthInsight) -> ExternResult<Record> {
    validate_health_insight(&insight)?;

    let insight_hash = create_entry(&EntryTypes::HealthInsight(insight.clone()))?;
    let record = get(insight_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find insight".to_string())
    ))?;

    // Link to patient
    create_link(
        insight.patient_hash.clone(),
        insight_hash,
        LinkTypes::PatientToInsights,
        (),
    )?;

    Ok(record)
}

/// Get patient's insights
#[hdk_extern]
pub fn get_patient_insights(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToInsights)?,
        GetStrategy::default(),
    )?;

    let mut insights = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                insights.push(record);
            }
        }
    }

    Ok(insights)
}

/// Acknowledge an insight
#[hdk_extern]
pub fn acknowledge_insight(input: AcknowledgeInsightInput) -> ExternResult<Record> {
    let record = get(input.insight_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Insight not found".to_string())
    ))?;

    let mut insight: HealthInsight = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid insight".to_string()
        )))?;

    insight.acknowledged = true;
    insight.action_taken = input.action_taken;

    let updated_hash = update_entry(input.insight_hash, &insight)?;

    get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Could not find updated insight".to_string()
    )))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AcknowledgeInsightInput {
    pub insight_hash: ActionHash,
    pub action_taken: Option<String>,
}

// ==================== PROVIDER REVIEWS ====================

/// Create provider review
#[hdk_extern]
pub fn create_provider_review(review: ProviderReview) -> ExternResult<Record> {
    validate_provider_review(&review)?;

    let review_hash = create_entry(&EntryTypes::ProviderReview(review.clone()))?;
    let record = get(review_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find review".to_string())
    ))?;

    // Link to patient
    create_link(
        review.patient_hash.clone(),
        review_hash.clone(),
        LinkTypes::PatientToReviews,
        (),
    )?;

    // Link to provider
    create_link(
        review.provider_hash.clone(),
        review_hash,
        LinkTypes::ProviderToReviews,
        (),
    )?;

    Ok(record)
}

/// Get patient's reviews
#[hdk_extern]
pub fn get_patient_reviews(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToReviews)?,
        GetStrategy::default(),
    )?;

    let mut reviews = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                reviews.push(record);
            }
        }
    }

    Ok(reviews)
}

/// Get provider's reviews
#[hdk_extern]
pub fn get_provider_reviews(provider_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(provider_hash, LinkTypes::ProviderToReviews)?,
        GetStrategy::default(),
    )?;

    let mut reviews = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                reviews.push(record);
            }
        }
    }

    Ok(reviews)
}

/// Calculate provider's aggregate rating
#[hdk_extern]
pub fn get_provider_aggregate_rating(provider_hash: ActionHash) -> ExternResult<AggregateRating> {
    let reviews = get_provider_reviews(provider_hash)?;

    if reviews.is_empty() {
        return Ok(AggregateRating {
            overall_rating: 0.0,
            total_reviews: 0,
            would_recommend_percentage: 0.0,
            average_wait_time: None,
        });
    }

    let mut total_rating: u32 = 0;
    let mut would_recommend_count: u32 = 0;
    let mut wait_times: Vec<u32> = Vec::new();

    for record in &reviews {
        if let Some(review) = record
            .entry()
            .to_app_option::<ProviderReview>()
            .ok()
            .flatten()
        {
            total_rating += review.overall_rating as u32;
            if review.would_recommend {
                would_recommend_count += 1;
            }
            if let Some(wait) = review.wait_time_minutes {
                wait_times.push(wait);
            }
        }
    }

    let count = reviews.len() as f32;
    let average_wait = if wait_times.is_empty() {
        None
    } else {
        Some(wait_times.iter().sum::<u32>() as f32 / wait_times.len() as f32)
    };

    Ok(AggregateRating {
        overall_rating: total_rating as f32 / count,
        total_reviews: reviews.len() as u32,
        would_recommend_percentage: (would_recommend_count as f32 / count) * 100.0,
        average_wait_time: average_wait,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AggregateRating {
    pub overall_rating: f32,
    pub total_reviews: u32,
    pub would_recommend_percentage: f32,
    pub average_wait_time: Option<f32>,
}

// ==================== HEALTH ALERTS ====================

/// Create health alert
#[hdk_extern]
pub fn create_health_alert(alert: HealthAlert) -> ExternResult<Record> {
    validate_health_alert(&alert)?;

    let alert_hash = create_entry(&EntryTypes::HealthAlert(alert.clone()))?;
    let record = get(alert_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find alert".to_string())
    ))?;

    // Link to patient
    create_link(
        alert.patient_hash.clone(),
        alert_hash.clone(),
        LinkTypes::PatientToAlerts,
        (),
    )?;

    // Link to active alerts if not resolved
    if !matches!(alert.status, AlertStatus::Resolved | AlertStatus::Dismissed) {
        let active_anchor = anchor_hash("active_alerts")?;
        create_link(active_anchor, alert_hash, LinkTypes::ActiveAlerts, ())?;
    }

    Ok(record)
}

/// Get patient's alerts
#[hdk_extern]
pub fn get_patient_alerts(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToAlerts)?,
        GetStrategy::default(),
    )?;

    let mut alerts = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                alerts.push(record);
            }
        }
    }

    Ok(alerts)
}

/// Get patient's active (unresolved) alerts
#[hdk_extern]
pub fn get_active_alerts(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let all_alerts = get_patient_alerts(patient_hash)?;

    let active: Vec<Record> = all_alerts
        .into_iter()
        .filter(|record| {
            if let Some(alert) = record.entry().to_app_option::<HealthAlert>().ok().flatten() {
                !matches!(alert.status, AlertStatus::Resolved | AlertStatus::Dismissed)
            } else {
                false
            }
        })
        .collect();

    Ok(active)
}

/// Update alert status
#[hdk_extern]
pub fn update_alert_status(input: UpdateAlertStatusInput) -> ExternResult<Record> {
    let record = get(input.alert_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Alert not found".to_string())
    ))?;

    let mut alert: HealthAlert = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid alert".to_string()
        )))?;

    alert.status = input.new_status.clone();

    match &input.new_status {
        AlertStatus::Acknowledged => {
            alert.acknowledged_at = Some(sys_time()?.as_micros() as i64);
        }
        AlertStatus::Resolved | AlertStatus::Dismissed => {
            alert.resolved_at = Some(sys_time()?.as_micros() as i64);
            alert.resolution_notes = input.notes;
        }
        _ => {}
    }

    let updated_hash = update_entry(input.alert_hash, &alert)?;

    get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Could not find updated alert".to_string()
    )))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateAlertStatusInput {
    pub alert_hash: ActionHash,
    pub new_status: AlertStatus,
    pub notes: Option<String>,
}

// ==================== ADVOCATE SESSIONS ====================

/// Create advocate session
#[hdk_extern]
pub fn create_advocate_session(session: AdvocateSession) -> ExternResult<Record> {
    let session_hash = create_entry(&EntryTypes::AdvocateSession(session.clone()))?;
    let record = get(session_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find session".to_string())
    ))?;

    // Link to patient
    create_link(
        session.patient_hash.clone(),
        session_hash,
        LinkTypes::PatientToSessions,
        (),
    )?;

    Ok(record)
}

/// Add message to session
#[hdk_extern]
pub fn add_session_message(input: AddMessageInput) -> ExternResult<Record> {
    let record = get(input.session_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Session not found".to_string())
    ))?;

    let mut session: AdvocateSession = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid session".to_string()
        )))?;

    session.messages.push(input.message);
    session.last_message_at = sys_time()?.as_micros() as i64;

    let updated_hash = update_entry(input.session_hash, &session)?;

    get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Could not find updated session".to_string()
    )))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddMessageInput {
    pub session_hash: ActionHash,
    pub message: ConversationMessage,
}

/// End session with summary
#[hdk_extern]
pub fn end_session(input: EndSessionInput) -> ExternResult<Record> {
    let record = get(input.session_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Session not found".to_string())
    ))?;

    let mut session: AdvocateSession = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid session".to_string()
        )))?;

    session.ended = true;
    session.summary = Some(input.summary);
    session.action_items = input.action_items;
    session.satisfaction_rating = input.satisfaction_rating;

    let updated_hash = update_entry(input.session_hash, &session)?;

    get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Could not find updated session".to_string()
    )))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EndSessionInput {
    pub session_hash: ActionHash,
    pub summary: String,
    pub action_items: Vec<String>,
    pub satisfaction_rating: Option<u8>,
}

/// Get patient's sessions
#[hdk_extern]
pub fn get_patient_sessions(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToSessions)?,
        GetStrategy::default(),
    )?;

    let mut sessions = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                sessions.push(record);
            }
        }
    }

    Ok(sessions)
}

// ==================== ADVOCATE PREFERENCES ====================

/// Set advocate preferences
#[hdk_extern]
pub fn set_advocate_preferences(prefs: AdvocatePreferences) -> ExternResult<Record> {
    validate_advocate_preferences(&prefs)?;

    let prefs_hash = create_entry(&EntryTypes::AdvocatePreferences(prefs.clone()))?;
    let record = get(prefs_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find preferences".to_string())
    ))?;

    // Link to patient
    create_link(
        prefs.patient_hash.clone(),
        prefs_hash,
        LinkTypes::PatientToPreferences,
        (),
    )?;

    Ok(record)
}

/// Get patient's advocate preferences
#[hdk_extern]
pub fn get_advocate_preferences(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToPreferences)?,
        GetStrategy::default(),
    )?;

    // Get the most recent preferences
    if let Some(link) = links.last() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            return get(hash, GetOptions::default());
        }
    }

    Ok(None)
}

// ==================== SECOND OPINION ====================

/// Create second opinion request
#[hdk_extern]
pub fn create_second_opinion_request(request: SecondOpinionRequest) -> ExternResult<Record> {
    let request_hash = create_entry(&EntryTypes::SecondOpinionRequest(request.clone()))?;
    let record = get(request_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find request".to_string())
    ))?;

    // Link to active requests
    let anchor = anchor_hash("second_opinion_requests")?;
    create_link(anchor, request_hash, LinkTypes::SecondOpinionRequests, ())?;

    Ok(record)
}

/// Update second opinion with AI analysis
#[hdk_extern]
pub fn add_ai_analysis(input: AddAIAnalysisInput) -> ExternResult<Record> {
    let record = get(input.request_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Request not found".to_string())
    ))?;

    let mut request: SecondOpinionRequest = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid request".to_string()
        )))?;

    request.ai_analysis = Some(input.analysis);
    request.status = SecondOpinionStatus::Completed;
    request.updated_at = sys_time()?.as_micros() as i64;

    let updated_hash = update_entry(input.request_hash, &request)?;

    get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Could not find updated request".to_string()
    )))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddAIAnalysisInput {
    pub request_hash: ActionHash,
    pub analysis: AISecondOpinionAnalysis,
}

// ==================== MEDICATION CHECK ====================

/// Create medication check
#[hdk_extern]
pub fn create_medication_check(check: MedicationCheck) -> ExternResult<Record> {
    validate_medication_check(&check)?;

    let check_hash = create_entry(&EntryTypes::MedicationCheck(check.clone()))?;
    let record = get(check_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find check".to_string())
    ))?;

    // Link to patient
    create_link(
        check.patient_hash.clone(),
        check_hash,
        LinkTypes::MedicationChecks,
        (),
    )?;

    Ok(record)
}

/// Get patient's medication checks
#[hdk_extern]
pub fn get_patient_medication_checks(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::MedicationChecks)?,
        GetStrategy::default(),
    )?;

    let mut checks = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                checks.push(record);
            }
        }
    }

    Ok(checks)
}

/// Get latest medication check for patient
#[hdk_extern]
pub fn get_latest_medication_check(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    let checks = get_patient_medication_checks(patient_hash)?;

    // Find the most recent by generated_at
    let latest = checks.into_iter().max_by(|a, b| {
        let a_time = a
            .entry()
            .to_app_option::<MedicationCheck>()
            .ok()
            .flatten()
            .map(|c| c.generated_at)
            .unwrap_or(0);
        let b_time = b
            .entry()
            .to_app_option::<MedicationCheck>()
            .ok()
            .flatten()
            .map(|c| c.generated_at)
            .unwrap_or(0);
        a_time.cmp(&b_time)
    });

    Ok(latest)
}

// ==================== RECOMMENDED QUESTIONS ====================

/// Create recommended question
#[hdk_extern]
pub fn create_recommended_question(question: RecommendedQuestion) -> ExternResult<Record> {
    let question_hash = create_entry(&EntryTypes::RecommendedQuestion(question.clone()))?;
    let record = get(question_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find question".to_string())
    ))?;

    // Link to patient
    create_link(
        question.patient_hash.clone(),
        question_hash,
        LinkTypes::PatientToQuestions,
        (),
    )?;

    Ok(record)
}

/// Get patient's recommended questions
#[hdk_extern]
pub fn get_patient_questions(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToQuestions)?,
        GetStrategy::default(),
    )?;

    let mut questions = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                questions.push(record);
            }
        }
    }

    Ok(questions)
}

/// Mark question as asked
#[hdk_extern]
pub fn mark_question_asked(input: MarkQuestionAskedInput) -> ExternResult<Record> {
    let record = get(input.question_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Question not found".to_string())
    ))?;

    let mut question: RecommendedQuestion = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid question".to_string()
        )))?;

    question.asked = true;
    question.answer_received = input.answer_received;

    let updated_hash = update_entry(input.question_hash, &question)?;

    get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Could not find updated question".to_string()
    )))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MarkQuestionAskedInput {
    pub question_hash: ActionHash,
    pub answer_received: Option<String>,
}

// ==================== ANCHOR SUPPORT ====================

/// Anchor entry for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}
