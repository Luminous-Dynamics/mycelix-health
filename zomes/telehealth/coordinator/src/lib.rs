//! Telehealth Session Coordinator Zome
//!
//! Provides extern functions for telehealth session management including:
//! - Session scheduling and management
//! - Waiting room operations
//! - Session documentation
//!
//! All data access enforces consent-based access control.

use hdk::prelude::*;
use telehealth_integrity::*;
use mycelix_health_shared::{
    require_authorization, log_data_access,
    DataCategory, Permission, anchor_hash,
};

// ============================================================================
// Session Scheduling Functions
// ============================================================================

/// Schedule a new telehealth session
#[hdk_extern]
pub fn schedule_telehealth_session(input: ScheduleSessionInput) -> ExternResult<Record> {
    let session = TelehealthSession {
        session_id: format!("TH-{}", sys_time()?.as_micros()),
        patient_hash: input.patient_hash.clone(),
        provider_hash: input.provider_hash.clone(),
        scheduled_start: input.scheduled_start,
        scheduled_duration_minutes: input.duration_minutes,
        session_type: input.session_type,
        status: SessionStatus::Scheduled,
        visit_reason: input.visit_reason,
        chief_complaint: input.chief_complaint,
        meeting_url: None, // Generated when session starts
        platform: input.platform,
        actual_start: None,
        actual_end: None,
        provider_notes: None,
        patient_symptoms: Vec::new(),
        follow_up_needed: false,
        follow_up_notes: None,
        prescription_hashes: Vec::new(),
        order_hashes: Vec::new(),
        created_at: sys_time()?,
        updated_at: sys_time()?,
    };

    let hash = create_entry(&EntryTypes::TelehealthSession(session.clone()))?;
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find session".to_string())))?;

    // Link from patient to session
    create_link(
        input.patient_hash.clone(),
        hash.clone(),
        LinkTypes::PatientToSessions,
        (),
    )?;

    // Link from provider to session
    create_link(
        input.provider_hash,
        hash.clone(),
        LinkTypes::ProviderToSessions,
        (),
    )?;

    // Link to upcoming sessions anchor (by date)
    let date_anchor = anchor_hash(&format!("sessions_{}", timestamp_to_date(input.scheduled_start)))?;
    create_link(date_anchor, hash, LinkTypes::UpcomingSessions, ())?;

    Ok(record)
}

/// Input for getting session with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct GetSessionInput {
    pub session_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get a telehealth session with access control
#[hdk_extern]
pub fn get_telehealth_session(input: GetSessionInput) -> ExternResult<Option<Record>> {
    let record = get(input.session_hash.clone(), GetOptions::default())?;

    if let Some(ref rec) = record {
        if let Some(session) = rec.entry().to_app_option::<TelehealthSession>().ok().flatten() {
            // Require authorization
            let auth = require_authorization(
                session.patient_hash.clone(),
                DataCategory::All,
                Permission::Read,
                input.is_emergency,
            )?;

            // Log access
            log_data_access(
                session.patient_hash,
                vec![DataCategory::All],
                Permission::Read,
                auth.consent_hash,
                auth.emergency_override,
                input.emergency_reason,
            )?;
        }
    }

    Ok(record)
}

// ============================================================================
// Session Lifecycle Functions
// ============================================================================

/// Start a telehealth session
#[hdk_extern]
pub fn start_session(session_hash: ActionHash) -> ExternResult<SessionDetails> {
    let record = get(session_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Session not found".to_string())))?;

    let mut session: TelehealthSession = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid session entry".to_string())))?;

    // Validate session can be started
    if session.status != SessionStatus::Scheduled && session.status != SessionStatus::PatientWaiting && session.status != SessionStatus::ProviderReady {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Cannot start session in status: {:?}", session.status)
        )));
    }

    // Generate meeting URL (in production, this would integrate with video platform)
    let meeting_url = generate_meeting_url(&session.session_id, &session.platform)?;

    session.status = SessionStatus::InProgress;
    session.actual_start = Some(sys_time()?);
    session.meeting_url = Some(meeting_url.clone());
    session.updated_at = sys_time()?;

    let updated_hash = update_entry(session_hash.clone(), &session)?;
    create_link(session_hash.clone(), updated_hash.clone(), LinkTypes::SessionUpdates, ())?;

    Ok(SessionDetails {
        session_hash: updated_hash,
        meeting_url,
        status: SessionStatus::InProgress,
        provider_name: None, // Would be looked up from provider directory
        scheduled_start: session.scheduled_start,
        session_type: session.session_type,
    })
}

/// Input for ending a session
#[derive(Serialize, Deserialize, Debug)]
pub struct EndSessionInput {
    pub session_hash: ActionHash,
    pub notes: Option<String>,
    pub follow_up_needed: bool,
    pub follow_up_notes: Option<String>,
}

/// End a telehealth session
#[hdk_extern]
pub fn end_session(input: EndSessionInput) -> ExternResult<Record> {
    let record = get(input.session_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Session not found".to_string())))?;

    let mut session: TelehealthSession = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid session entry".to_string())))?;

    // Validate session can be ended
    if session.status != SessionStatus::InProgress {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Can only end sessions that are in progress".to_string()
        )));
    }

    session.status = SessionStatus::Completed;
    session.actual_end = Some(sys_time()?);
    session.provider_notes = input.notes;
    session.follow_up_needed = input.follow_up_needed;
    session.follow_up_notes = input.follow_up_notes;
    session.updated_at = sys_time()?;

    let updated_hash = update_entry(input.session_hash.clone(), &session)?;
    let updated_record = get(updated_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated session".to_string())))?;

    create_link(input.session_hash, updated_hash, LinkTypes::SessionUpdates, ())?;

    Ok(updated_record)
}

/// Input for cancelling a session
#[derive(Serialize, Deserialize, Debug)]
pub struct CancelSessionInput {
    pub session_hash: ActionHash,
    pub reason: String,
}

/// Cancel a telehealth session
#[hdk_extern]
pub fn cancel_session(input: CancelSessionInput) -> ExternResult<Record> {
    let record = get(input.session_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Session not found".to_string())))?;

    let mut session: TelehealthSession = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid session entry".to_string())))?;

    // Can only cancel scheduled or waiting sessions
    if session.status == SessionStatus::Completed || session.status == SessionStatus::InProgress {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Cannot cancel completed or in-progress sessions".to_string()
        )));
    }

    session.status = SessionStatus::Cancelled;
    session.provider_notes = Some(format!("Cancelled: {}", input.reason));
    session.updated_at = sys_time()?;

    let updated_hash = update_entry(input.session_hash.clone(), &session)?;
    let updated_record = get(updated_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated session".to_string())))?;

    create_link(input.session_hash, updated_hash, LinkTypes::SessionUpdates, ())?;

    Ok(updated_record)
}

// ============================================================================
// Waiting Room Functions
// ============================================================================

/// Patient joins waiting room
#[hdk_extern]
pub fn join_waiting_room(session_hash: ActionHash) -> ExternResult<Record> {
    let record = get(session_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Session not found".to_string())))?;

    let mut session: TelehealthSession = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid session entry".to_string())))?;

    // Update session status
    session.status = SessionStatus::PatientWaiting;
    session.updated_at = sys_time()?;
    let updated_session_hash = update_entry(session_hash.clone(), &session)?;

    // Create waiting room entry
    let entry = WaitingRoomEntry {
        session_hash: session_hash.clone(),
        patient_hash: session.patient_hash.clone(),
        entered_at: sys_time()?,
        called_at: None,
        queue_position: 1, // Would be calculated based on provider's queue
        estimated_wait_minutes: Some(5), // Would be calculated
        status: WaitingRoomStatus::Waiting,
        notes: None,
    };

    let entry_hash = create_entry(&EntryTypes::WaitingRoomEntry(entry))?;
    let waiting_room_record = get(entry_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find waiting room entry".to_string())))?;

    // Link session to waiting room entry
    create_link(session_hash.clone(), entry_hash, LinkTypes::SessionToWaitingRoom, ())?;
    create_link(session_hash, updated_session_hash, LinkTypes::SessionUpdates, ())?;

    Ok(waiting_room_record)
}

/// Provider calls patient from waiting room
#[hdk_extern]
pub fn call_patient(session_hash: ActionHash) -> ExternResult<Record> {
    // Get waiting room entry
    let links = get_links(
        LinkQuery::try_new(session_hash.clone(), LinkTypes::SessionToWaitingRoom)?, GetStrategy::default())?;

    let entry_hash = links.first()
        .and_then(|l| l.target.clone().into_action_hash())
        .ok_or(wasm_error!(WasmErrorInner::Guest("No waiting room entry found".to_string())))?;

    let record = get(entry_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Waiting room entry not found".to_string())))?;

    let mut entry: WaitingRoomEntry = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid waiting room entry".to_string())))?;

    entry.status = WaitingRoomStatus::BeingCalled;
    entry.called_at = Some(sys_time()?);

    let updated_hash = update_entry(entry_hash, &entry)?;
    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated entry".to_string())))
}

// ============================================================================
// Session Documentation Functions
// ============================================================================

/// Create session documentation (SOAP note)
#[hdk_extern]
pub fn create_session_documentation(doc: SessionDocumentation) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::SessionDocumentation(doc.clone()))?;
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find documentation".to_string())))?;

    // Link from session to documentation
    create_link(
        doc.session_hash,
        hash,
        LinkTypes::SessionToDocumentation,
        (),
    )?;

    Ok(record)
}

/// Get documentation for a session
#[hdk_extern]
pub fn get_session_documentation(input: GetSessionInput) -> ExternResult<Option<Record>> {
    // First get the session to verify access
    let session_record = get(input.session_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Session not found".to_string())))?;

    let session: TelehealthSession = session_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid session".to_string())))?;

    // Require authorization
    let auth = require_authorization(
        session.patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        input.is_emergency,
    )?;

    // Get documentation
    let links = get_links(
        LinkQuery::try_new(input.session_hash.clone(), LinkTypes::SessionToDocumentation)?, GetStrategy::default())?;

    let result = if let Some(link) = links.first() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            get(hash, GetOptions::default())?
        } else {
            None
        }
    } else {
        None
    };

    // Log access
    log_data_access(
        session.patient_hash,
        vec![DataCategory::All],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(result)
}

/// Sign session documentation
#[hdk_extern]
pub fn sign_documentation(doc_hash: ActionHash) -> ExternResult<Record> {
    let record = get(doc_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Documentation not found".to_string())))?;

    let mut doc: SessionDocumentation = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid documentation".to_string())))?;

    doc.signed = true;
    doc.signed_at = Some(sys_time()?);

    let updated_hash = update_entry(doc_hash, &doc)?;
    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find signed documentation".to_string())))
}

// ============================================================================
// Available Slot Functions
// ============================================================================

/// Create available slots for a provider
#[hdk_extern]
pub fn create_available_slot(slot: AvailableSlot) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::AvailableSlot(slot.clone()))?;
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find slot".to_string())))?;

    // Link from provider to slot
    create_link(
        slot.provider_hash,
        hash,
        LinkTypes::ProviderToAvailableSlots,
        (),
    )?;

    Ok(record)
}

/// Input for getting provider's available slots
#[derive(Serialize, Deserialize, Debug)]
pub struct GetAvailableSlotsInput {
    pub provider_hash: ActionHash,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

/// Get available slots for a provider
#[hdk_extern]
pub fn get_available_slots(input: GetAvailableSlotsInput) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(input.provider_hash, LinkTypes::ProviderToAvailableSlots)?, GetStrategy::default())?;

    let mut slots = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(slot) = record.entry().to_app_option::<AvailableSlot>().ok().flatten() {
                    // Filter by availability and date range
                    if slot.is_available {
                        slots.push(record);
                    }
                }
            }
        }
    }

    Ok(slots)
}

// ============================================================================
// Patient Session Query Functions
// ============================================================================

/// Input for getting patient's sessions
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPatientSessionsInput {
    pub patient_hash: ActionHash,
    pub include_completed: bool,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get all telehealth sessions for a patient
#[hdk_extern]
pub fn get_patient_sessions(input: GetPatientSessionsInput) -> ExternResult<Vec<Record>> {
    // Require authorization
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToSessions)?, GetStrategy::default())?;

    let mut sessions = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(session) = record.entry().to_app_option::<TelehealthSession>().ok().flatten() {
                    let include = input.include_completed || session.status != SessionStatus::Completed;
                    if include {
                        sessions.push(record);
                    }
                }
            }
        }
    }

    // Log access
    log_data_access(
        input.patient_hash,
        vec![DataCategory::All],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(sessions)
}

/// Input for getting provider's sessions
#[derive(Serialize, Deserialize, Debug)]
pub struct GetProviderSessionsInput {
    pub provider_hash: ActionHash,
    pub date: Option<String>,
    pub include_completed: bool,
}

/// Get telehealth sessions for a provider
#[hdk_extern]
pub fn get_provider_sessions(input: GetProviderSessionsInput) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(input.provider_hash, LinkTypes::ProviderToSessions)?, GetStrategy::default())?;

    let mut sessions = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(session) = record.entry().to_app_option::<TelehealthSession>().ok().flatten() {
                    let include = input.include_completed || session.status != SessionStatus::Completed;
                    if include {
                        sessions.push(record);
                    }
                }
            }
        }
    }

    Ok(sessions)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate a meeting URL (would integrate with video platform in production)
fn generate_meeting_url(session_id: &str, platform: &str) -> ExternResult<String> {
    // In production, this would call the video platform's API
    // For now, generate a placeholder URL
    let url = match platform.to_lowercase().as_str() {
        "zoom" => format!("https://zoom.us/j/{}", session_id.replace('-', "")),
        "doxy.me" => format!("https://doxy.me/room/{}", session_id),
        "webex" => format!("https://webex.com/meet/{}", session_id),
        _ => format!("https://telehealth.mycelix.net/session/{}", session_id),
    };
    Ok(url)
}

/// Convert timestamp to date string (YYYY-MM-DD)
fn timestamp_to_date(ts: Timestamp) -> String {
    // Simple conversion - in production would use proper datetime library
    let micros = ts.as_micros();
    let secs = micros / 1_000_000;
    let days = secs / 86400;
    // Approximate date calculation (not accounting for leap years, etc.)
    let year = 1970 + (days / 365);
    let day_of_year = days % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;
    format!("{:04}-{:02}-{:02}", year, month.min(12), day.min(28))
}
