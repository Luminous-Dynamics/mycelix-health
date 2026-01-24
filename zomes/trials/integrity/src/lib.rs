//! Clinical Trials and Research Study Management Integrity Zome
//! 
//! Defines entry types for clinical trials, participant enrollment,
//! data collection, and adverse event reporting with FDA 21 CFR Part 11 alignment.

use hdi::prelude::*;

/// Clinical trial/study definition
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ClinicalTrial {
    pub trial_id: String,
    /// ClinicalTrials.gov NCT number
    pub nct_number: Option<String>,
    pub title: String,
    pub short_title: Option<String>,
    pub description: String,
    pub phase: TrialPhase,
    pub study_type: StudyType,
    pub status: TrialStatus,
    /// Principal investigator
    pub principal_investigator: AgentPubKey,
    /// Sponsor organization
    pub sponsor: String,
    /// Collaborating institutions
    pub collaborators: Vec<String>,
    /// Target enrollment
    pub target_enrollment: u32,
    /// Current enrollment
    pub current_enrollment: u32,
    /// Start date
    pub start_date: Timestamp,
    /// Estimated end date
    pub estimated_end_date: Option<Timestamp>,
    /// Actual end date
    pub actual_end_date: Option<Timestamp>,
    /// Eligibility criteria
    pub eligibility: EligibilityCriteria,
    /// Interventions being studied
    pub interventions: Vec<Intervention>,
    /// Primary outcomes
    pub primary_outcomes: Vec<Outcome>,
    /// Secondary outcomes
    pub secondary_outcomes: Vec<Outcome>,
    /// IRB approval
    pub irb_approved: bool,
    pub irb_approval_date: Option<Timestamp>,
    pub irb_expiration_date: Option<Timestamp>,
    /// Link to DeSci publication (if applicable)
    pub desci_publication_hash: Option<ActionHash>,
    /// Epistemic level of trial findings
    pub epistemic_level: EpistemicLevel,
    /// MATL trust score for this trial
    pub matl_trust_score: f64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TrialPhase {
    EarlyPhase1,
    Phase1,
    Phase1Phase2,
    Phase2,
    Phase2Phase3,
    Phase3,
    Phase4,
    NotApplicable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum StudyType {
    Interventional,
    Observational,
    ExpandedAccess,
    Registry,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TrialStatus {
    NotYetRecruiting,
    Recruiting,
    EnrollingByInvitation,
    ActiveNotRecruiting,
    Suspended,
    Terminated,
    Completed,
    Withdrawn,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EpistemicLevel {
    E0Preliminary,
    E1PeerReviewed,
    E2Replicated,
    E3Consensus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EligibilityCriteria {
    pub min_age: Option<u32>,
    pub max_age: Option<u32>,
    pub sex: EligibleSex,
    pub healthy_volunteers: bool,
    pub inclusion_criteria: Vec<String>,
    pub exclusion_criteria: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EligibleSex {
    All,
    Female,
    Male,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Intervention {
    pub intervention_type: InterventionType,
    pub name: String,
    pub description: String,
    pub arm_group: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum InterventionType {
    Drug,
    Device,
    Biological,
    Procedure,
    Radiation,
    Behavioral,
    Genetic,
    DietarySupplement,
    Diagnostic,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Outcome {
    pub measure: String,
    pub time_frame: String,
    pub description: String,
}

/// Trial participant enrollment
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TrialParticipant {
    pub participant_id: String,
    pub trial_hash: ActionHash,
    pub patient_hash: ActionHash,
    /// Consent specific to this trial
    pub consent_hash: ActionHash,
    pub enrollment_date: Timestamp,
    pub withdrawal_date: Option<Timestamp>,
    pub withdrawal_reason: Option<String>,
    pub arm_assignment: Option<String>,
    pub status: ParticipantStatus,
    /// Blinding
    pub blinded: bool,
    /// Screening results
    pub screening_passed: bool,
    pub screening_date: Option<Timestamp>,
    /// Site where enrolled
    pub enrollment_site: String,
    pub primary_contact: AgentPubKey,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ParticipantStatus {
    Screening,
    Enrolled,
    Active,
    FollowUp,
    Completed,
    Withdrawn,
    ScreenFail,
    LostToFollowUp,
}

/// Trial visit/data collection record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TrialVisit {
    pub visit_id: String,
    pub participant_hash: ActionHash,
    pub trial_hash: ActionHash,
    pub visit_number: u32,
    pub visit_name: String,
    pub scheduled_date: Timestamp,
    pub actual_date: Option<Timestamp>,
    pub status: VisitStatus,
    pub performed_by: Option<AgentPubKey>,
    /// Collected data points
    pub data_points: Vec<DataPoint>,
    /// Protocol deviations during this visit
    pub protocol_deviations: Vec<ProtocolDeviation>,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum VisitStatus {
    Scheduled,
    Completed,
    Missed,
    Rescheduled,
    OutOfWindow,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DataPoint {
    pub name: String,
    pub value: String,
    pub unit: Option<String>,
    pub collected_at: Timestamp,
    pub collected_by: AgentPubKey,
    pub source: DataSource,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DataSource {
    DirectEntry,
    LabResult,
    DeviceImport,
    PatientReported,
    EHRImport,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProtocolDeviation {
    pub deviation_type: DeviationType,
    pub description: String,
    pub occurred_at: Timestamp,
    pub reported_at: Timestamp,
    pub severity: DeviationSeverity,
    pub corrective_action: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DeviationType {
    InclusionExclusionViolation,
    MissedAssessment,
    WrongDose,
    WrongProcedure,
    TimingDeviation,
    ConsentIssue,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DeviationSeverity {
    Minor,
    Major,
    Critical,
}

/// Adverse event report
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AdverseEvent {
    pub event_id: String,
    pub participant_hash: ActionHash,
    pub trial_hash: ActionHash,
    pub event_term: String,
    pub description: String,
    pub onset_date: Timestamp,
    pub resolution_date: Option<Timestamp>,
    pub ongoing: bool,
    pub severity: AESeverity,
    pub seriousness: Vec<SeriousnessCriteria>,
    pub is_serious: bool,
    pub is_unexpected: bool,
    pub causality: Causality,
    pub outcome: AEOutcome,
    pub action_taken: Vec<ActionTaken>,
    pub reported_by: AgentPubKey,
    pub reported_at: Timestamp,
    /// FDA MedWatch report filed
    pub medwatch_submitted: bool,
    pub medwatch_date: Option<Timestamp>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AESeverity {
    Mild,
    Moderate,
    Severe,
    LifeThreatening,
    Death,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SeriousnessCriteria {
    Death,
    LifeThreatening,
    Hospitalization,
    Disability,
    CongenitalAnomaly,
    ImportantMedicalEvent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Causality {
    DefinitelyRelated,
    ProbablyRelated,
    PossiblyRelated,
    UnlikelyRelated,
    NotRelated,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AEOutcome {
    Recovered,
    Recovering,
    NotRecovered,
    RecoveredWithSequelae,
    Fatal,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ActionTaken {
    NoneRequired,
    StudyDrugInterrupted,
    StudyDrugReduced,
    StudyDrugDiscontinued,
    SubjectWithdrawn,
    MedicationGiven,
    ProcedurePerformed,
    Hospitalized,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    ClinicalTrial(ClinicalTrial),
    TrialParticipant(TrialParticipant),
    TrialVisit(TrialVisit),
    AdverseEvent(AdverseEvent),
}

#[hdk_link_types]
pub enum LinkTypes {
    TrialToParticipants,
    TrialToVisits,
    TrialToAdverseEvents,
    PatientToTrials,
    ProviderToTrials,
    ActiveTrials,
    CompletedTrials,
    RecruitingTrials,
    TrialsBySponsor,
    TrialsByPhase,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::ClinicalTrial(t) => validate_trial(&t),
                EntryTypes::TrialParticipant(p) => validate_participant(&p),
                EntryTypes::TrialVisit(v) => validate_visit(&v),
                EntryTypes::AdverseEvent(a) => validate_adverse_event(&a),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_trial(trial: &ClinicalTrial) -> ExternResult<ValidateCallbackResult> {
    if trial.trial_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Trial ID is required".to_string(),
        ));
    }
    if trial.title.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Trial title is required".to_string(),
        ));
    }
    if trial.matl_trust_score < 0.0 || trial.matl_trust_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "MATL trust score must be between 0.0 and 1.0".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_participant(participant: &TrialParticipant) -> ExternResult<ValidateCallbackResult> {
    if participant.participant_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Participant ID is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_visit(visit: &TrialVisit) -> ExternResult<ValidateCallbackResult> {
    if visit.visit_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Visit ID is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_adverse_event(event: &AdverseEvent) -> ExternResult<ValidateCallbackResult> {
    if event.event_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Event ID is required".to_string(),
        ));
    }
    if event.event_term.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Event term is required".to_string(),
        ));
    }
    if event.is_serious && event.seriousness.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Serious events must specify seriousness criteria".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
