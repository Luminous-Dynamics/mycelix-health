//! Decentralized IRB Integrity Zome
//!
//! Defines entry types for decentralized institutional review board
//! functionality including protocol submissions, reviews, and approvals.

use hdi::prelude::*;

/// Type of IRB review required
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ReviewType {
    /// Full board review
    FullBoard,
    /// Expedited review (minimal risk)
    Expedited,
    /// Exempt from review
    Exempt,
    /// Continuing review
    Continuing,
    /// Amendment review
    Amendment,
    /// Adverse event review
    AdverseEvent,
    /// Protocol deviation review
    Deviation,
}

/// Status of an IRB submission
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SubmissionStatus {
    Draft,
    Submitted,
    UnderReview,
    RequestingChanges,
    Approved,
    ApprovedWithConditions,
    Deferred,
    Disapproved,
    Withdrawn,
    Suspended,
    Terminated,
    Closed,
}

/// Risk level assessment
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RiskLevel {
    MinimalRisk,
    GreaterThanMinimal,
    HighRisk,
}

/// Reviewer role on the IRB
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ReviewerRole {
    Chair,
    ViceChair,
    ScientificReviewer,
    NonScientificReviewer,
    CommunityMember,
    PatientAdvocate,
    Ethicist,
    LegalCounsel,
    ExternalExpert,
}

/// Vote on a submission
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum VoteType {
    Approve,
    ApproveWithConditions,
    Defer,
    Disapprove,
    Abstain,
    Recuse,
}

/// A research protocol submitted for IRB review
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ProtocolSubmission {
    /// Unique protocol ID
    pub protocol_id: String,
    /// Protocol title
    pub title: String,
    /// Principal investigator
    pub principal_investigator_hash: ActionHash,
    /// Co-investigators
    pub co_investigators: Vec<ActionHash>,
    /// Institution/sponsor
    pub institution: String,
    /// Review type requested
    pub review_type: ReviewType,
    /// Current status
    pub status: SubmissionStatus,
    /// Risk level assessment
    pub risk_level: RiskLevel,
    /// Study summary
    pub summary: String,
    /// Research objectives
    pub objectives: Vec<String>,
    /// Study population description
    pub population: String,
    /// Number of participants
    pub target_enrollment: u32,
    /// Inclusion criteria summary
    pub inclusion_criteria: Vec<String>,
    /// Exclusion criteria summary
    pub exclusion_criteria: Vec<String>,
    /// Study procedures summary
    pub procedures: Vec<String>,
    /// Potential risks
    pub risks: Vec<String>,
    /// Risk mitigation measures
    pub risk_mitigations: Vec<String>,
    /// Potential benefits
    pub benefits: Vec<String>,
    /// Data safety monitoring plan
    pub data_safety_plan: Option<String>,
    /// Informed consent document hash
    pub consent_document_hash: Option<ActionHash>,
    /// Protocol document hash
    pub protocol_document_hash: Option<ActionHash>,
    /// Investigator brochure hash (if applicable)
    pub investigator_brochure_hash: Option<ActionHash>,
    /// Funding source
    pub funding_source: Option<String>,
    /// Conflict of interest declaration
    pub conflict_of_interest: Option<String>,
    /// Submission date
    pub submitted_at: Option<Timestamp>,
    /// Approval date (if approved)
    pub approved_at: Option<Timestamp>,
    /// Approval expiration date
    pub approval_expires_at: Option<Timestamp>,
    /// Created timestamp
    pub created_at: Timestamp,
    /// Last updated
    pub updated_at: Timestamp,
}

/// An IRB member/reviewer
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IrbMember {
    /// Member hash
    pub member_hash: ActionHash,
    /// Name
    pub name: String,
    /// Role on IRB
    pub role: ReviewerRole,
    /// Credentials/qualifications
    pub credentials: Vec<String>,
    /// Areas of expertise
    pub expertise: Vec<String>,
    /// Institutional affiliation
    pub institution: String,
    /// Whether currently active
    pub is_active: bool,
    /// Date joined IRB
    pub joined_at: Timestamp,
    /// Training completion dates
    pub training_completed: Vec<(String, Timestamp)>,
}

/// A review of a protocol
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ProtocolReview {
    /// Review ID
    pub review_id: String,
    /// Protocol being reviewed
    pub protocol_hash: ActionHash,
    /// Reviewer
    pub reviewer_hash: ActionHash,
    /// Reviewer's role
    pub reviewer_role: ReviewerRole,
    /// Risk assessment
    pub risk_assessment: RiskLevel,
    /// Scientific merit assessment (1-5)
    pub scientific_merit: Option<u32>,
    /// Consent process assessment (1-5)
    pub consent_assessment: Option<u32>,
    /// Privacy/confidentiality assessment (1-5)
    pub privacy_assessment: Option<u32>,
    /// Comments
    pub comments: String,
    /// Required modifications
    pub required_modifications: Vec<String>,
    /// Suggested modifications (optional)
    pub suggested_modifications: Vec<String>,
    /// Questions for investigators
    pub questions: Vec<String>,
    /// Vote
    pub vote: VoteType,
    /// Vote rationale
    pub vote_rationale: String,
    /// Conditions if approved with conditions
    pub conditions: Vec<String>,
    /// Review date
    pub reviewed_at: Timestamp,
}

/// An IRB meeting record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IrbMeeting {
    /// Meeting ID
    pub meeting_id: String,
    /// Meeting date
    pub meeting_date: Timestamp,
    /// Meeting type (regular, special, emergency)
    pub meeting_type: String,
    /// Quorum present
    pub quorum_present: bool,
    /// Members present
    pub members_present: Vec<ActionHash>,
    /// Members absent
    pub members_absent: Vec<ActionHash>,
    /// Protocols reviewed
    pub protocols_reviewed: Vec<ActionHash>,
    /// Meeting minutes hash
    pub minutes_hash: Option<ActionHash>,
    /// Meeting adjourned
    pub adjourned_at: Option<Timestamp>,
}

/// Final decision on a protocol
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IrbDecision {
    /// Decision ID
    pub decision_id: String,
    /// Protocol hash
    pub protocol_hash: ActionHash,
    /// Meeting where decision was made (if full board)
    pub meeting_hash: Option<ActionHash>,
    /// Review type used
    pub review_type: ReviewType,
    /// Final status
    pub decision: SubmissionStatus,
    /// Vote counts
    pub votes_approve: u32,
    pub votes_disapprove: u32,
    pub votes_defer: u32,
    pub votes_abstain: u32,
    /// Required conditions
    pub conditions: Vec<String>,
    /// Approval valid for (days)
    pub approval_duration_days: Option<u32>,
    /// Continuing review required date
    pub continuing_review_due: Option<Timestamp>,
    /// Decision rationale
    pub rationale: String,
    /// Chair signature hash
    pub chair_signature_hash: Option<ActionHash>,
    /// Decision date
    pub decided_at: Timestamp,
}

/// A continuing review submission
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ContinuingReview {
    /// Review ID
    pub review_id: String,
    /// Original protocol
    pub protocol_hash: ActionHash,
    /// Current enrollment
    pub current_enrollment: u32,
    /// Enrollment since last review
    pub enrollment_since_last: u32,
    /// Withdrawals since last review
    pub withdrawals: u32,
    /// Adverse events since last review
    pub adverse_events: u32,
    /// Serious adverse events
    pub serious_adverse_events: u32,
    /// Protocol deviations
    pub deviations: u32,
    /// Summary of progress
    pub progress_summary: String,
    /// Any protocol amendments
    pub amendments: Vec<String>,
    /// Risk benefit assessment update
    pub risk_benefit_update: String,
    /// Submitted date
    pub submitted_at: Timestamp,
}

/// Entry types for the IRB zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    ProtocolSubmission(ProtocolSubmission),
    IrbMember(IrbMember),
    ProtocolReview(ProtocolReview),
    IrbMeeting(IrbMeeting),
    IrbDecision(IrbDecision),
    ContinuingReview(ContinuingReview),
}

/// Link types for the IRB zome
#[hdk_link_types]
pub enum LinkTypes {
    /// All submissions
    AllSubmissions,
    /// Submissions by status
    SubmissionsByStatus,
    /// Protocol to reviews
    ProtocolToReviews,
    /// Protocol to decisions
    ProtocolToDecisions,
    /// Investigator to protocols
    InvestigatorToProtocols,
    /// Reviewer to reviews
    ReviewerToReviews,
    /// Meeting to protocols
    MeetingToProtocols,
    /// All IRB members
    AllMembers,
    /// Active members
    ActiveMembers,
    /// Protocol to continuing reviews
    ProtocolToContinuingReviews,
}

/// Validation callbacks
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::ProtocolSubmission(submission) => validate_submission(&submission),
                EntryTypes::ProtocolReview(review) => validate_review(&review),
                EntryTypes::IrbDecision(decision) => validate_decision(&decision),
                _ => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_submission(submission: &ProtocolSubmission) -> ExternResult<ValidateCallbackResult> {
    if submission.protocol_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Protocol ID is required".to_string()));
    }
    if submission.title.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Protocol title is required".to_string()));
    }
    if submission.summary.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Study summary is required".to_string()));
    }
    if submission.risks.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Risk disclosure is required".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_review(review: &ProtocolReview) -> ExternResult<ValidateCallbackResult> {
    if review.review_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Review ID is required".to_string()));
    }
    if review.comments.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Review comments are required".to_string()));
    }
    if review.vote_rationale.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Vote rationale is required".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_decision(decision: &IrbDecision) -> ExternResult<ValidateCallbackResult> {
    if decision.decision_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Decision ID is required".to_string()));
    }
    if decision.rationale.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Decision rationale is required".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}
