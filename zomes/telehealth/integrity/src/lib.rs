#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
// clippy::collapsible_match is allowed crate-wide here. Every instance is the
// standard HDK validation-dispatch shape:
//
//     FlatOp::RegisterUpdate(op) => match op {
//         OpUpdate::Entry { app_entry, action } => validate_update_entry(action, app_entry),
//         _ => Ok(ValidateCallbackResult::Valid),
//     }
//
// Collapsing it into the outer pattern would force a separate catch-all arm for
// the remaining OpUpdate variants, diverge from every sibling integrity zome, and
// restructure the exact RegisterUpdate dispatch the author-binding work depends on
// -- a real risk in security-critical validation for a style lint. This crate is
// validation dispatch end to end, so the allow is scoped to what it describes.
#![allow(clippy::collapsible_match)]

//! Telehealth Session Integrity Zome
//!
//! Defines entry types for telehealth capabilities including:
//! - Virtual visit sessions
//! - Appointment scheduling
//! - Session notes and documentation
//! - Waiting room management
//!
//! HIPAA compliant for remote patient care.

use hdi::prelude::*;

// ============================================================================
// Telehealth Session Types
// ============================================================================

/// Telehealth session record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TelehealthSession {
    /// Unique session identifier
    pub session_id: String,
    /// Patient participating in the session
    pub patient_hash: ActionHash,
    /// Provider conducting the session
    pub provider_hash: ActionHash,
    /// Scheduled start time
    pub scheduled_start: Timestamp,
    /// Scheduled duration in minutes
    pub scheduled_duration_minutes: u32,
    /// Type of session
    pub session_type: SessionType,
    /// Current session status
    pub status: SessionStatus,
    /// Reason for visit
    pub visit_reason: String,
    /// Chief complaint (patient-reported)
    pub chief_complaint: Option<String>,
    /// Meeting URL (video conference link)
    pub meeting_url: Option<String>,
    /// Platform used (Zoom, Doxy.me, etc.)
    pub platform: String,
    /// Actual start time (when session began)
    pub actual_start: Option<Timestamp>,
    /// Actual end time
    pub actual_end: Option<Timestamp>,
    /// Session notes (provider)
    pub provider_notes: Option<String>,
    /// Patient-reported symptoms
    pub patient_symptoms: Vec<String>,
    /// Follow-up needed
    pub follow_up_needed: bool,
    /// Follow-up notes
    pub follow_up_notes: Option<String>,
    /// Related prescriptions created during visit
    pub prescription_hashes: Vec<ActionHash>,
    /// Related orders created during visit
    pub order_hashes: Vec<ActionHash>,
    /// Session created at
    pub created_at: Timestamp,
    /// Last updated
    pub updated_at: Timestamp,
}

/// Type of telehealth session
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SessionType {
    /// Standard video consultation
    VideoConsult,
    /// Phone-only consultation
    PhoneConsult,
    /// Follow-up visit
    FollowUp,
    /// Urgent care visit
    UrgentCare,
    /// Mental health session
    MentalHealth,
    /// Medication review
    MedicationReview,
    /// Lab result review
    LabReview,
    /// Post-procedure check
    PostProcedure,
    /// Chronic care management
    ChronicCare,
    /// Other (specify)
    Other(String),
}

/// Session status tracking
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    /// Session is scheduled but not started
    Scheduled,
    /// Patient is in waiting room
    PatientWaiting,
    /// Provider is ready
    ProviderReady,
    /// Session is in progress
    InProgress,
    /// Session completed successfully
    Completed,
    /// Session was cancelled
    Cancelled,
    /// Patient did not show
    NoShow,
    /// Technical difficulties prevented session
    TechnicalIssue,
    /// Rescheduled to another time
    Rescheduled,
}

// ============================================================================
// Waiting Room Types
// ============================================================================

/// Waiting room entry for a session
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct WaitingRoomEntry {
    /// Session this entry is for
    pub session_hash: ActionHash,
    /// Patient hash
    pub patient_hash: ActionHash,
    /// When patient entered waiting room
    pub entered_at: Timestamp,
    /// When patient was called (if applicable)
    pub called_at: Option<Timestamp>,
    /// Current position in queue
    pub queue_position: u32,
    /// Estimated wait time in minutes
    pub estimated_wait_minutes: Option<u32>,
    /// Waiting room status
    pub status: WaitingRoomStatus,
    /// Notes (e.g., "running 10 minutes late")
    pub notes: Option<String>,
}

/// Waiting room status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum WaitingRoomStatus {
    Waiting,
    BeingCalled,
    InSession,
    LeftWaitingRoom,
}

// ============================================================================
// Session Documentation Types
// ============================================================================

/// Clinical documentation for a telehealth session
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SessionDocumentation {
    /// Session this documentation belongs to
    pub session_hash: ActionHash,
    /// Patient hash
    pub patient_hash: ActionHash,
    /// Provider hash
    pub provider_hash: ActionHash,
    /// Subjective (patient history)
    pub subjective: Option<String>,
    /// Objective (exam findings - limited for telehealth)
    pub objective: Option<String>,
    /// Assessment (diagnosis/impressions)
    pub assessment: Option<String>,
    /// Plan (treatment plan)
    pub plan: Option<String>,
    /// ICD-10 diagnosis codes
    pub diagnosis_codes: Vec<String>,
    /// CPT procedure codes
    pub procedure_codes: Vec<String>,
    /// Medications prescribed
    pub medications_prescribed: Vec<MedicationPrescribed>,
    /// Labs ordered
    pub labs_ordered: Vec<String>,
    /// Imaging ordered
    pub imaging_ordered: Vec<String>,
    /// Referrals made
    pub referrals: Vec<Referral>,
    /// Patient education provided
    pub patient_education: Vec<String>,
    /// Documentation created at
    pub created_at: Timestamp,
    /// Signed by provider
    pub signed: bool,
    /// Signed at timestamp
    pub signed_at: Option<Timestamp>,
}

/// Medication prescribed during session
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MedicationPrescribed {
    pub medication_name: String,
    pub rxnorm_code: Option<String>,
    pub dosage: String,
    pub frequency: String,
    pub duration: String,
    pub quantity: u32,
    pub refills: u32,
    pub instructions: String,
}

/// Referral made during session
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Referral {
    pub specialty: String,
    pub reason: String,
    pub urgency: ReferralUrgency,
    pub provider_name: Option<String>,
    pub notes: Option<String>,
}

/// Referral urgency levels
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ReferralUrgency {
    Routine,
    Urgent,
    Emergent,
}

// ============================================================================
// Appointment Scheduling Types
// ============================================================================

/// Available time slot for scheduling
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AvailableSlot {
    /// Provider offering this slot
    pub provider_hash: ActionHash,
    /// Start time of slot
    pub start_time: Timestamp,
    /// Duration in minutes
    pub duration_minutes: u32,
    /// Types of visits available in this slot
    pub available_session_types: Vec<SessionType>,
    /// Whether slot is still available
    pub is_available: bool,
    /// If booked, which session
    pub booked_session_hash: Option<ActionHash>,
    /// Slot created at
    pub created_at: Timestamp,
}

/// Scheduling request from patient
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SchedulingRequest {
    /// Request identifier
    pub request_id: String,
    /// Patient making the request
    pub patient_hash: ActionHash,
    /// Preferred provider (optional)
    pub preferred_provider_hash: Option<ActionHash>,
    /// Requested session type
    pub session_type: SessionType,
    /// Preferred dates (YYYY-MM-DD)
    pub preferred_dates: Vec<String>,
    /// Preferred time of day
    pub preferred_time: PreferredTime,
    /// Reason for visit
    pub reason: String,
    /// Urgency level
    pub urgency: SchedulingUrgency,
    /// Request status
    pub status: SchedulingRequestStatus,
    /// Assigned slot (if scheduled)
    pub assigned_slot_hash: Option<ActionHash>,
    /// Created at
    pub created_at: Timestamp,
    /// Updated at
    pub updated_at: Timestamp,
}

/// Preferred time of day for appointment
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PreferredTime {
    Morning,
    Afternoon,
    Evening,
    AnyTime,
}

/// Scheduling urgency
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SchedulingUrgency {
    Routine,
    Soon,
    Urgent,
    SameDay,
}

/// Scheduling request status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SchedulingRequestStatus {
    Pending,
    Scheduled,
    NoAvailability,
    Cancelled,
    Expired,
}

// ============================================================================
// Session Input/Output Types
// ============================================================================

/// Input for scheduling a new session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduleSessionInput {
    pub patient_hash: ActionHash,
    pub provider_hash: ActionHash,
    pub scheduled_start: Timestamp,
    pub duration_minutes: u32,
    pub session_type: SessionType,
    pub visit_reason: String,
    pub chief_complaint: Option<String>,
    pub platform: String,
}

/// Details returned when starting a session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionDetails {
    pub session_hash: ActionHash,
    pub meeting_url: String,
    pub status: SessionStatus,
    pub provider_name: Option<String>,
    pub scheduled_start: Timestamp,
    pub session_type: SessionType,
}

// ============================================================================
// Entry and Link Type Enums
// ============================================================================

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    TelehealthSession(TelehealthSession),
    WaitingRoomEntry(WaitingRoomEntry),
    SessionDocumentation(SessionDocumentation),
    AvailableSlot(AvailableSlot),
    SchedulingRequest(SchedulingRequest),
}

#[hdk_link_types]
pub enum LinkTypes {
    /// Patient to their telehealth sessions
    PatientToSessions,
    /// Provider to their telehealth sessions
    ProviderToSessions,
    /// Session to its documentation
    SessionToDocumentation,
    /// Session to waiting room entry
    SessionToWaitingRoom,
    /// Provider to their available slots
    ProviderToAvailableSlots,
    /// Patient to their scheduling requests
    PatientToSchedulingRequests,
    /// All upcoming sessions (by date anchor)
    UpcomingSessions,
    /// Session updates tracking
    SessionUpdates,
}

// ============================================================================
// Validation Functions
// ============================================================================

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => validate_create_entry(app_entry),
            OpEntry::UpdateEntry {
                app_entry, action, ..
            } => validate_update_entry(action, app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterCreateLink { link_type, .. } => validate_link(link_type),
        // Previously left fully permissive via the catch-all `_` arm --
        // the 19th confirmed instance of the wide-open RegisterUpdate/
        // RegisterDelete bug this pass. Found + fixed 2026-07-09 during
        // the P0 author-binding pass. This coordinator never calls
        // delete_link/delete_entry (confirmed via grep), so
        // RegisterDeleteLink/RegisterDelete hardening below is pure
        // defense-in-depth.
        FlatOp::RegisterDeleteLink {
            original_action,
            action,
            ..
        } => {
            if action.author != original_action.author {
                return Ok(ValidateCallbackResult::Invalid(
                    "Only the original link creator can delete a link".into(),
                ));
            }
            Ok(ValidateCallbackResult::Valid)
        }
        FlatOp::RegisterUpdate(op_update) => match op_update {
            OpUpdate::Entry { app_entry, action } => validate_update_entry(action, app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDelete(OpDelete { action }) => {
            let original = must_get_action(action.deletes_address.clone())?;
            if action.author != *original.action().author() {
                return Ok(ValidateCallbackResult::Invalid(
                    "Only the original entry author can delete an entry".into(),
                ));
            }
            Ok(ValidateCallbackResult::Valid)
        }
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

/// No entry type in this zome has a self-declared agent-identity field
/// -- patient_hash/provider_hash are ActionHash references into the
/// patient/provider_directory zomes, not directly comparable
/// AgentPubKeys. Reviewed 2026-07-09 during the P0 author-binding pass;
/// case (a) across the board.
fn validate_create_entry(entry: EntryTypes) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::TelehealthSession(session) => validate_telehealth_session(&session),
        EntryTypes::WaitingRoomEntry(entry) => validate_waiting_room_entry(&entry),
        EntryTypes::SessionDocumentation(doc) => validate_session_documentation(&doc),
        EntryTypes::AvailableSlot(slot) => validate_available_slot(&slot),
        EntryTypes::SchedulingRequest(request) => validate_scheduling_request(&request),
    }
}

/// TelehealthSession/WaitingRoomEntry/SessionDocumentation each have a
/// live, narrow update flow (status transitions). AvailableSlot and
/// SchedulingRequest have no live coordinator update call at all
/// (confirmed via grep for `update_entry` -- SchedulingRequest has no
/// live coordinator function referencing it at all beyond its entry-type
/// declaration) and are made immutable. Reviewed 2026-07-09 during the
/// P0 author-binding pass: not an author-forgery finding (no identity
/// field exists anywhere in this zome), but the previous unconditional
/// `Ok(Valid)` meant a modified coordinator could silently rewrite
/// patient_hash/provider_hash on a live session -- a real
/// patient/provider-record-mismatch vector.
fn validate_update_entry(
    action: Update,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::TelehealthSession(session) => {
            let original_record = must_get_valid_record(action.original_action_address.clone())?;
            let original: TelehealthSession = original_record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "Original session not found".into()
                )))?;
            // Union of fields changed across start_session/end_session/
            // cancel_session/join_waiting_room: status, actual_start,
            // actual_end, meeting_url, provider_notes, follow_up_needed,
            // follow_up_notes, updated_at. Everything else is immutable.
            if session.session_id != original.session_id
                || session.patient_hash != original.patient_hash
                || session.provider_hash != original.provider_hash
                || session.scheduled_start != original.scheduled_start
                || session.scheduled_duration_minutes != original.scheduled_duration_minutes
                || session.session_type != original.session_type
                || session.visit_reason != original.visit_reason
                || session.chief_complaint != original.chief_complaint
                || session.platform != original.platform
                || session.patient_symptoms != original.patient_symptoms
                || session.prescription_hashes != original.prescription_hashes
                || session.order_hashes != original.order_hashes
                || session.created_at != original.created_at
            {
                return Ok(ValidateCallbackResult::Invalid(
                    "Only status/timing/notes fields can change on a session update".into(),
                ));
            }
            validate_telehealth_session(&session)
        }
        EntryTypes::WaitingRoomEntry(entry) => {
            let original_record = must_get_valid_record(action.original_action_address.clone())?;
            let original: WaitingRoomEntry = original_record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "Original waiting room entry not found".into()
                )))?;
            // Only call_patient updates this entry: status/called_at.
            if entry.session_hash != original.session_hash
                || entry.patient_hash != original.patient_hash
                || entry.entered_at != original.entered_at
                || entry.queue_position != original.queue_position
                || entry.estimated_wait_minutes != original.estimated_wait_minutes
                || entry.notes != original.notes
            {
                return Ok(ValidateCallbackResult::Invalid(
                    "Only status/called_at can change on a waiting room entry update".into(),
                ));
            }
            validate_waiting_room_entry(&entry)
        }
        EntryTypes::SessionDocumentation(doc) => {
            let original_record = must_get_valid_record(action.original_action_address.clone())?;
            let original: SessionDocumentation = original_record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "Original documentation not found".into()
                )))?;
            // Only sign_documentation updates this entry: signed/signed_at.
            if doc.session_hash != original.session_hash
                || doc.patient_hash != original.patient_hash
                || doc.provider_hash != original.provider_hash
                || doc.subjective != original.subjective
                || doc.objective != original.objective
                || doc.assessment != original.assessment
                || doc.plan != original.plan
                || doc.diagnosis_codes != original.diagnosis_codes
                || doc.procedure_codes != original.procedure_codes
                || doc.medications_prescribed != original.medications_prescribed
                || doc.labs_ordered != original.labs_ordered
                || doc.imaging_ordered != original.imaging_ordered
                || doc.referrals != original.referrals
                || doc.patient_education != original.patient_education
                || doc.created_at != original.created_at
            {
                return Ok(ValidateCallbackResult::Invalid(
                    "Only signed/signed_at can change on a documentation update".into(),
                ));
            }
            Ok(ValidateCallbackResult::Valid)
        }
        EntryTypes::AvailableSlot(_) => Ok(ValidateCallbackResult::Invalid(
            "Available slots are immutable; create a new one".into(),
        )),
        EntryTypes::SchedulingRequest(_) => Ok(ValidateCallbackResult::Invalid(
            "Scheduling requests are immutable".into(),
        )),
    }
}

fn validate_telehealth_session(
    session: &TelehealthSession,
) -> ExternResult<ValidateCallbackResult> {
    // Validate session ID
    if session.session_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Session ID cannot be empty".to_string(),
        ));
    }

    // Validate duration
    if session.scheduled_duration_minutes == 0 || session.scheduled_duration_minutes > 480 {
        return Ok(ValidateCallbackResult::Invalid(
            "Session duration must be between 1 and 480 minutes".to_string(),
        ));
    }

    // Validate visit reason
    if session.visit_reason.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Visit reason is required".to_string(),
        ));
    }

    // Validate platform
    if session.platform.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Platform must be specified".to_string(),
        ));
    }

    // Validate status transitions
    if session.status == SessionStatus::Completed && session.actual_end.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "Completed session must have an end time".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_waiting_room_entry(entry: &WaitingRoomEntry) -> ExternResult<ValidateCallbackResult> {
    // Validate queue position
    if entry.queue_position == 0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Queue position must be at least 1".to_string(),
        ));
    }

    // Validate status consistency
    if entry.status == WaitingRoomStatus::InSession && entry.called_at.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "Patient in session must have a called_at timestamp".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_session_documentation(
    doc: &SessionDocumentation,
) -> ExternResult<ValidateCallbackResult> {
    // Must have at least one of SOAP components
    if doc.subjective.is_none()
        && doc.objective.is_none()
        && doc.assessment.is_none()
        && doc.plan.is_none()
    {
        return Ok(ValidateCallbackResult::Invalid(
            "Documentation must include at least one SOAP component".to_string(),
        ));
    }

    // Validate signed consistency
    if doc.signed && doc.signed_at.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "Signed documentation must have a signed_at timestamp".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_available_slot(slot: &AvailableSlot) -> ExternResult<ValidateCallbackResult> {
    // Validate duration
    if slot.duration_minutes == 0 || slot.duration_minutes > 480 {
        return Ok(ValidateCallbackResult::Invalid(
            "Slot duration must be between 1 and 480 minutes".to_string(),
        ));
    }

    // Validate session types
    if slot.available_session_types.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one session type must be available".to_string(),
        ));
    }

    // Validate booking consistency
    if !slot.is_available && slot.booked_session_hash.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "Unavailable slot must have a booking reference".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_scheduling_request(
    request: &SchedulingRequest,
) -> ExternResult<ValidateCallbackResult> {
    // Validate request ID
    if request.request_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Request ID cannot be empty".to_string(),
        ));
    }

    // Validate reason
    if request.reason.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Reason for visit is required".to_string(),
        ));
    }

    // Validate preferred dates format
    for date in &request.preferred_dates {
        if !is_valid_date_format(date) {
            return Ok(ValidateCallbackResult::Invalid(format!(
                "Invalid date format: {}. Use YYYY-MM-DD",
                date
            )));
        }
    }

    // Validate status consistency
    if request.status == SchedulingRequestStatus::Scheduled && request.assigned_slot_hash.is_none()
    {
        return Ok(ValidateCallbackResult::Invalid(
            "Scheduled request must have an assigned slot".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_link(link_type: LinkTypes) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        LinkTypes::PatientToSessions => Ok(ValidateCallbackResult::Valid),
        LinkTypes::ProviderToSessions => Ok(ValidateCallbackResult::Valid),
        LinkTypes::SessionToDocumentation => Ok(ValidateCallbackResult::Valid),
        LinkTypes::SessionToWaitingRoom => Ok(ValidateCallbackResult::Valid),
        LinkTypes::ProviderToAvailableSlots => Ok(ValidateCallbackResult::Valid),
        LinkTypes::PatientToSchedulingRequests => Ok(ValidateCallbackResult::Valid),
        LinkTypes::UpcomingSessions => Ok(ValidateCallbackResult::Valid),
        LinkTypes::SessionUpdates => Ok(ValidateCallbackResult::Valid),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn is_valid_date_format(date: &str) -> bool {
    if date.len() != 10 {
        return false;
    }
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod content_integrity_tests {
    use super::*;

    fn update_action(author: AgentPubKey) -> Update {
        Update {
            author,
            timestamp: Timestamp::from_micros(1),
            action_seq: 1,
            prev_action: ActionHash::from_raw_36(vec![0u8; 36]),
            original_action_address: ActionHash::from_raw_36(vec![9u8; 36]),
            original_entry_address: EntryHash::from_raw_36(vec![0u8; 36]),
            entry_type: EntryType::App(AppEntryDef::new(
                EntryDefIndex::from(0),
                0.into(),
                EntryVisibility::Public,
            )),
            entry_hash: EntryHash::from_raw_36(vec![0u8; 36]),
            weight: Default::default(),
        }
    }

    fn me() -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![0u8; 36])
    }

    #[test]
    fn update_entry_rejects_available_slot_update() {
        // No live update_entry call exists for AvailableSlot -- dead-path
        // immutability, testable without must_get_valid_record.
        let slot = AvailableSlot {
            provider_hash: ActionHash::from_raw_36(vec![2u8; 36]),
            start_time: Timestamp::from_micros(0),
            duration_minutes: 30,
            available_session_types: vec![SessionType::VideoConsult],
            is_available: true,
            booked_session_hash: None,
            created_at: Timestamp::from_micros(0),
        };
        let result =
            validate_update_entry(update_action(me()), EntryTypes::AvailableSlot(slot)).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn update_entry_rejects_scheduling_request_update() {
        let request = SchedulingRequest {
            request_id: "r-1".into(),
            patient_hash: ActionHash::from_raw_36(vec![2u8; 36]),
            preferred_provider_hash: None,
            session_type: SessionType::VideoConsult,
            preferred_dates: vec![],
            preferred_time: PreferredTime::AnyTime,
            reason: "checkup".into(),
            urgency: SchedulingUrgency::Routine,
            status: SchedulingRequestStatus::Pending,
            assigned_slot_hash: None,
            created_at: Timestamp::from_micros(0),
            updated_at: Timestamp::from_micros(0),
        };
        let result =
            validate_update_entry(update_action(me()), EntryTypes::SchedulingRequest(request))
                .unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }
}
