//! Decentralized IRB Coordinator Zome
//!
//! Provides functions for managing decentralized institutional review board
//! operations including protocol submissions, reviews, and approvals.

use hdk::prelude::*;
use irb_integrity::*;

/// Input for creating a protocol submission
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateProtocolInput {
    pub protocol_id: String,
    pub title: String,
    pub co_investigators: Vec<ActionHash>,
    pub institution: String,
    pub review_type: ReviewType,
    pub risk_level: RiskLevel,
    pub summary: String,
    pub objectives: Vec<String>,
    pub population: String,
    pub target_enrollment: u32,
    pub inclusion_criteria: Vec<String>,
    pub exclusion_criteria: Vec<String>,
    pub procedures: Vec<String>,
    pub risks: Vec<String>,
    pub risk_mitigations: Vec<String>,
    pub benefits: Vec<String>,
    pub data_safety_plan: Option<String>,
    pub consent_document_hash: Option<ActionHash>,
    pub protocol_document_hash: Option<ActionHash>,
    pub investigator_brochure_hash: Option<ActionHash>,
    pub funding_source: Option<String>,
    pub conflict_of_interest: Option<String>,
}

/// Input for submitting a protocol for review
#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitProtocolInput {
    pub protocol_hash: ActionHash,
}

/// Input for creating an IRB member
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateMemberInput {
    pub name: String,
    pub role: ReviewerRole,
    pub credentials: Vec<String>,
    pub expertise: Vec<String>,
    pub institution: String,
}

/// Input for creating a protocol review
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateReviewInput {
    pub review_id: String,
    pub protocol_hash: ActionHash,
    pub risk_assessment: RiskLevel,
    pub scientific_merit: Option<u32>,
    pub consent_assessment: Option<u32>,
    pub privacy_assessment: Option<u32>,
    pub comments: String,
    pub required_modifications: Vec<String>,
    pub suggested_modifications: Vec<String>,
    pub questions: Vec<String>,
    pub vote: VoteType,
    pub vote_rationale: String,
    pub conditions: Vec<String>,
}

/// Input for scheduling an IRB meeting
#[derive(Serialize, Deserialize, Debug)]
pub struct ScheduleMeetingInput {
    pub meeting_id: String,
    pub meeting_date: Timestamp,
    pub meeting_type: String,
    pub protocols_to_review: Vec<ActionHash>,
}

/// Input for recording meeting attendance
#[derive(Serialize, Deserialize, Debug)]
pub struct RecordAttendanceInput {
    pub meeting_hash: ActionHash,
    pub members_present: Vec<ActionHash>,
    pub members_absent: Vec<ActionHash>,
    pub quorum_present: bool,
}

/// Input for recording a decision
#[derive(Serialize, Deserialize, Debug)]
pub struct RecordDecisionInput {
    pub decision_id: String,
    pub protocol_hash: ActionHash,
    pub meeting_hash: Option<ActionHash>,
    pub review_type: ReviewType,
    pub decision: SubmissionStatus,
    pub votes_approve: u32,
    pub votes_disapprove: u32,
    pub votes_defer: u32,
    pub votes_abstain: u32,
    pub conditions: Vec<String>,
    pub approval_duration_days: Option<u32>,
    pub rationale: String,
    pub chair_signature_hash: Option<ActionHash>,
}

/// Input for submitting a continuing review
#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitContinuingReviewInput {
    pub review_id: String,
    pub protocol_hash: ActionHash,
    pub current_enrollment: u32,
    pub enrollment_since_last: u32,
    pub withdrawals: u32,
    pub adverse_events: u32,
    pub serious_adverse_events: u32,
    pub deviations: u32,
    pub progress_summary: String,
    pub amendments: Vec<String>,
    pub risk_benefit_update: String,
}

/// Create a new protocol submission (draft)
#[hdk_extern]
pub fn create_protocol_submission(input: CreateProtocolInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;

    // Create member entry to get action hash for PI
    let pi_hash = create_entry(EntryTypes::IrbMember(IrbMember {
        member_hash: ActionHash::from_raw_36(vec![0; 36]), // Placeholder
        name: String::new(),
        role: ReviewerRole::ScientificReviewer,
        credentials: vec![],
        expertise: vec![],
        institution: input.institution.clone(),
        is_active: true,
        joined_at: now,
        training_completed: vec![],
    }))?;

    let submission = ProtocolSubmission {
        protocol_id: input.protocol_id,
        title: input.title,
        principal_investigator_hash: pi_hash.clone(),
        co_investigators: input.co_investigators,
        institution: input.institution,
        review_type: input.review_type,
        status: SubmissionStatus::Draft,
        risk_level: input.risk_level,
        summary: input.summary,
        objectives: input.objectives,
        population: input.population,
        target_enrollment: input.target_enrollment,
        inclusion_criteria: input.inclusion_criteria,
        exclusion_criteria: input.exclusion_criteria,
        procedures: input.procedures,
        risks: input.risks,
        risk_mitigations: input.risk_mitigations,
        benefits: input.benefits,
        data_safety_plan: input.data_safety_plan,
        consent_document_hash: input.consent_document_hash,
        protocol_document_hash: input.protocol_document_hash,
        investigator_brochure_hash: input.investigator_brochure_hash,
        funding_source: input.funding_source,
        conflict_of_interest: input.conflict_of_interest,
        submitted_at: None,
        approved_at: None,
        approval_expires_at: None,
        created_at: now,
        updated_at: now,
    };

    let action_hash = create_entry(EntryTypes::ProtocolSubmission(submission))?;

    // Link from all submissions anchor
    let all_anchor = anchor_hash("all_submissions")?;
    create_link(
        all_anchor,
        action_hash.clone(),
        LinkTypes::AllSubmissions,
        (),
    )?;

    // Link from investigator
    create_link(
        pi_hash,
        action_hash.clone(),
        LinkTypes::InvestigatorToProtocols,
        (),
    )?;

    Ok(action_hash)
}

/// Submit a protocol for review
#[hdk_extern]
pub fn submit_protocol(input: SubmitProtocolInput) -> ExternResult<ActionHash> {
    let record = get(input.protocol_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Protocol not found".to_string())))?;

    let mut submission: ProtocolSubmission = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid protocol entry".to_string())))?;

    let now = sys_time()?;
    submission.status = SubmissionStatus::Submitted;
    submission.submitted_at = Some(now);
    submission.updated_at = now;

    let new_hash = update_entry(input.protocol_hash.clone(), submission)?;

    // Link by status
    let status_anchor = anchor_hash("status_submitted")?;
    create_link(
        status_anchor,
        new_hash.clone(),
        LinkTypes::SubmissionsByStatus,
        (),
    )?;

    Ok(new_hash)
}

/// Get all protocol submissions
#[hdk_extern]
pub fn get_all_submissions(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash("all_submissions")?;
    let links = get_links(
        GetLinksInputBuilder::try_new(anchor, LinkTypes::AllSubmissions)?.build(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Get submissions by status
#[hdk_extern]
pub fn get_submissions_by_status(status: String) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash(&format!("status_{}", status.to_lowercase()))?;
    let links = get_links(
        GetLinksInputBuilder::try_new(anchor, LinkTypes::SubmissionsByStatus)?.build(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Register an IRB member
#[hdk_extern]
pub fn register_irb_member(input: CreateMemberInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let member = IrbMember {
        member_hash: ActionHash::from_raw_36(vec![0; 36]), // Will be updated
        name: input.name,
        role: input.role,
        credentials: input.credentials,
        expertise: input.expertise,
        institution: input.institution,
        is_active: true,
        joined_at: now,
        training_completed: vec![],
    };

    let action_hash = create_entry(EntryTypes::IrbMember(member))?;

    // Link from all members anchor
    let all_anchor = anchor_hash("all_irb_members")?;
    create_link(
        all_anchor,
        action_hash.clone(),
        LinkTypes::AllMembers,
        (),
    )?;

    // Link from active members anchor
    let active_anchor = anchor_hash("active_irb_members")?;
    create_link(
        active_anchor,
        action_hash.clone(),
        LinkTypes::ActiveMembers,
        (),
    )?;

    Ok(action_hash)
}

/// Get all active IRB members
#[hdk_extern]
pub fn get_active_members(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash("active_irb_members")?;
    let links = get_links(
        GetLinksInputBuilder::try_new(anchor, LinkTypes::ActiveMembers)?.build(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Submit a protocol review
#[hdk_extern]
pub fn submit_review(input: CreateReviewInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;

    // Get reviewer's member hash (simplified - would lookup in production)
    let reviewer_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let review = ProtocolReview {
        review_id: input.review_id,
        protocol_hash: input.protocol_hash.clone(),
        reviewer_hash: reviewer_hash.clone(),
        reviewer_role: ReviewerRole::ScientificReviewer, // Would lookup in production
        risk_assessment: input.risk_assessment,
        scientific_merit: input.scientific_merit,
        consent_assessment: input.consent_assessment,
        privacy_assessment: input.privacy_assessment,
        comments: input.comments,
        required_modifications: input.required_modifications,
        suggested_modifications: input.suggested_modifications,
        questions: input.questions,
        vote: input.vote,
        vote_rationale: input.vote_rationale,
        conditions: input.conditions,
        reviewed_at: now,
    };

    let action_hash = create_entry(EntryTypes::ProtocolReview(review))?;

    // Link from protocol
    create_link(
        input.protocol_hash,
        action_hash.clone(),
        LinkTypes::ProtocolToReviews,
        (),
    )?;

    // Link from reviewer
    create_link(
        reviewer_hash,
        action_hash.clone(),
        LinkTypes::ReviewerToReviews,
        (),
    )?;

    Ok(action_hash)
}

/// Get reviews for a protocol
#[hdk_extern]
pub fn get_protocol_reviews(protocol_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(protocol_hash, LinkTypes::ProtocolToReviews)?.build(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Schedule an IRB meeting
#[hdk_extern]
pub fn schedule_meeting(input: ScheduleMeetingInput) -> ExternResult<ActionHash> {
    let meeting = IrbMeeting {
        meeting_id: input.meeting_id,
        meeting_date: input.meeting_date,
        meeting_type: input.meeting_type,
        quorum_present: false,
        members_present: vec![],
        members_absent: vec![],
        protocols_reviewed: input.protocols_to_review.clone(),
        minutes_hash: None,
        adjourned_at: None,
    };

    let action_hash = create_entry(EntryTypes::IrbMeeting(meeting))?;

    // Link protocols to meeting
    for protocol_hash in input.protocols_to_review {
        create_link(
            action_hash.clone(),
            protocol_hash,
            LinkTypes::MeetingToProtocols,
            (),
        )?;
    }

    Ok(action_hash)
}

/// Record meeting attendance
#[hdk_extern]
pub fn record_attendance(input: RecordAttendanceInput) -> ExternResult<ActionHash> {
    let record = get(input.meeting_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Meeting not found".to_string())))?;

    let mut meeting: IrbMeeting = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid meeting entry".to_string())))?;

    meeting.members_present = input.members_present;
    meeting.members_absent = input.members_absent;
    meeting.quorum_present = input.quorum_present;

    update_entry(input.meeting_hash, meeting)
}

/// Record an IRB decision
#[hdk_extern]
pub fn record_decision(input: RecordDecisionInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let continuing_review_due = if let Some(days) = input.approval_duration_days {
        // Calculate expiration timestamp
        let duration_micros = (days as i64) * 24 * 60 * 60 * 1_000_000;
        Some(Timestamp::from_micros(now.as_micros() + duration_micros))
    } else {
        None
    };

    let decision = IrbDecision {
        decision_id: input.decision_id,
        protocol_hash: input.protocol_hash.clone(),
        meeting_hash: input.meeting_hash,
        review_type: input.review_type,
        decision: input.decision.clone(),
        votes_approve: input.votes_approve,
        votes_disapprove: input.votes_disapprove,
        votes_defer: input.votes_defer,
        votes_abstain: input.votes_abstain,
        conditions: input.conditions,
        approval_duration_days: input.approval_duration_days,
        continuing_review_due,
        rationale: input.rationale,
        chair_signature_hash: input.chair_signature_hash,
        decided_at: now,
    };

    let action_hash = create_entry(EntryTypes::IrbDecision(decision))?;

    // Link from protocol
    create_link(
        input.protocol_hash.clone(),
        action_hash.clone(),
        LinkTypes::ProtocolToDecisions,
        (),
    )?;

    // Update protocol status
    let record = get(input.protocol_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Protocol not found".to_string())))?;

    let mut submission: ProtocolSubmission = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid protocol entry".to_string())))?;

    submission.status = input.decision;
    if matches!(submission.status, SubmissionStatus::Approved | SubmissionStatus::ApprovedWithConditions) {
        submission.approved_at = Some(now);
        submission.approval_expires_at = continuing_review_due;
    }
    submission.updated_at = now;

    update_entry(input.protocol_hash, submission)?;

    Ok(action_hash)
}

/// Submit a continuing review
#[hdk_extern]
pub fn submit_continuing_review(input: SubmitContinuingReviewInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let review = ContinuingReview {
        review_id: input.review_id,
        protocol_hash: input.protocol_hash.clone(),
        current_enrollment: input.current_enrollment,
        enrollment_since_last: input.enrollment_since_last,
        withdrawals: input.withdrawals,
        adverse_events: input.adverse_events,
        serious_adverse_events: input.serious_adverse_events,
        deviations: input.deviations,
        progress_summary: input.progress_summary,
        amendments: input.amendments,
        risk_benefit_update: input.risk_benefit_update,
        submitted_at: now,
    };

    let action_hash = create_entry(EntryTypes::ContinuingReview(review))?;

    // Link from protocol
    create_link(
        input.protocol_hash,
        action_hash.clone(),
        LinkTypes::ProtocolToContinuingReviews,
        (),
    )?;

    Ok(action_hash)
}

/// Get continuing reviews for a protocol
#[hdk_extern]
pub fn get_continuing_reviews(protocol_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(protocol_hash, LinkTypes::ProtocolToContinuingReviews)?.build(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Get decision for a protocol
#[hdk_extern]
pub fn get_protocol_decision(protocol_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(protocol_hash, LinkTypes::ProtocolToDecisions)?.build(),
    )?;

    // Return most recent decision
    if let Some(link) = links.last() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            return get(hash, GetOptions::default());
        }
    }

    Ok(None)
}

// Helper function to create anchor hash
fn anchor_hash(anchor: &str) -> ExternResult<AnyLinkableHash> {
    let anchor_bytes = anchor.as_bytes().to_vec();
    Ok(AnyLinkableHash::from(
        EntryHash::from_raw_36(
            hdk::hash::hash_keccak256(anchor_bytes)
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?[..36]
                .to_vec(),
        ),
    ))
}
