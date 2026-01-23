//! AI Health Advocate Integrity Zome
//!
//! Entry types and validation for the AI Health Advocate system that:
//! - Prepares patients for appointments
//! - Analyzes health records and generates insights
//! - Monitors for potential issues and alerts
//! - Provides 24/7 health guidance
//! - Tracks provider performance and outcomes

use hdi::prelude::*;

/// Define the entry types for the advocate zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    /// AI-generated appointment preparation
    AppointmentPrep(AppointmentPrep),
    /// Health insight or recommendation
    HealthInsight(HealthInsight),
    /// Provider review and rating
    ProviderReview(ProviderReview),
    /// Question the AI recommends asking
    RecommendedQuestion(RecommendedQuestion),
    /// Alert for potential health issue
    HealthAlert(HealthAlert),
    /// Advocate conversation session
    AdvocateSession(AdvocateSession),
    /// Patient preferences for the advocate
    AdvocatePreferences(AdvocatePreferences),
    /// Second opinion request
    SecondOpinionRequest(SecondOpinionRequest),
    /// Medication interaction check
    MedicationCheck(MedicationCheck),
}

/// Link types for the advocate zome
#[hdk_link_types]
pub enum LinkTypes {
    PatientToPreps,
    PatientToInsights,
    PatientToReviews,
    PatientToQuestions,
    PatientToAlerts,
    PatientToSessions,
    PatientToPreferences,
    ProviderToReviews,
    AppointmentToPrep,
    ActiveAlerts,
    SecondOpinionRequests,
    MedicationChecks,
}

// ==================== APPOINTMENT PREPARATION ====================

/// AI-prepared appointment briefing
#[hdk_entry_helper]
#[derive(Clone)]
pub struct AppointmentPrep {
    /// Unique preparation ID
    pub prep_id: String,
    /// Patient this prep is for
    pub patient_hash: ActionHash,
    /// Provider appointment is with
    pub provider_hash: Option<ActionHash>,
    /// Provider's name (for display)
    pub provider_name: String,
    /// Appointment date/time (microseconds since epoch)
    pub appointment_time: i64,
    /// Type of appointment
    pub appointment_type: AppointmentType,
    /// Key points from medical history relevant to this visit
    pub relevant_history_summary: String,
    /// Recent changes since last visit
    pub recent_changes: Vec<RecentChange>,
    /// Questions AI recommends asking
    pub recommended_questions: Vec<String>,
    /// Points to bring up with provider
    pub discussion_points: Vec<String>,
    /// Current medications to review
    pub medications_to_review: Vec<MedicationReview>,
    /// Provider's track record (if available)
    pub provider_track_record: Option<ProviderTrackRecord>,
    /// Preparation generated at
    pub generated_at: i64,
    /// AI model/version used
    pub ai_model_version: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence_score: f32,
    /// Whether patient has reviewed this prep
    pub reviewed: bool,
    /// Patient notes/additions
    pub patient_notes: Option<String>,
}

/// Type of appointment
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AppointmentType {
    /// Routine checkup
    RoutineCheckup,
    /// Follow-up visit
    FollowUp,
    /// Specialist consultation
    SpecialistConsult,
    /// Urgent care
    UrgentCare,
    /// Emergency department
    Emergency,
    /// Telehealth
    Telehealth,
    /// Mental health
    MentalHealth,
    /// Chronic disease management
    ChronicDiseaseManagement,
    /// Preventive care
    PreventiveCare,
    /// Procedure/surgery
    Procedure,
    /// Other
    Other(String),
}

/// Recent change since last visit
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecentChange {
    /// Category of change
    pub category: ChangeCategory,
    /// Plain-language description
    pub description: String,
    /// When the change occurred
    pub occurred_at: i64,
    /// Severity/importance
    pub importance: Importance,
}

/// Categories of health changes
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ChangeCategory {
    NewSymptom,
    SymptomChange,
    MedicationChange,
    LabResult,
    NewDiagnosis,
    LifestyleChange,
    MentalHealth,
    Weight,
    VitalSigns,
    Other(String),
}

/// Importance level
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Importance {
    Critical,
    High,
    Medium,
    Low,
    Informational,
}

/// Medication to review at appointment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MedicationReview {
    /// Medication name
    pub medication_name: String,
    /// Current dosage
    pub dosage: String,
    /// Reason for review
    pub review_reason: MedicationReviewReason,
    /// Questions to ask
    pub questions: Vec<String>,
}

/// Why a medication should be reviewed
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MedicationReviewReason {
    /// Time for refill
    RefillNeeded,
    /// Potential interaction detected
    PotentialInteraction,
    /// Side effects reported
    SideEffects,
    /// Adherence issues
    AdherenceIssues,
    /// New alternative available
    NewAlternativeAvailable,
    /// Cost concerns
    CostConcerns,
    /// Routine review
    RoutineReview,
}

/// Provider's historical performance
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProviderTrackRecord {
    /// Number of previous visits
    pub previous_visits: u32,
    /// Average wait time (minutes)
    pub average_wait_minutes: Option<u32>,
    /// Patient satisfaction score (1-5)
    pub satisfaction_score: Option<f32>,
    /// Strengths noted
    pub strengths: Vec<String>,
    /// Areas for improvement
    pub improvement_areas: Vec<String>,
}

// ==================== HEALTH INSIGHTS ====================

/// AI-generated health insight
#[hdk_entry_helper]
#[derive(Clone)]
pub struct HealthInsight {
    /// Unique insight ID
    pub insight_id: String,
    /// Patient this insight is for
    pub patient_hash: ActionHash,
    /// Type of insight
    pub insight_type: InsightType,
    /// Plain-language summary
    pub summary: String,
    /// Detailed explanation
    pub explanation: String,
    /// Evidence supporting this insight
    pub evidence: Vec<EvidenceItem>,
    /// Recommended actions
    pub recommended_actions: Vec<RecommendedAction>,
    /// Urgency level
    pub urgency: Urgency,
    /// Confidence score
    pub confidence: f32,
    /// Generated at
    pub generated_at: i64,
    /// Patient has acknowledged
    pub acknowledged: bool,
    /// Patient action taken
    pub action_taken: Option<String>,
}

/// Type of health insight
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InsightType {
    /// Trend detected in data
    TrendDetected,
    /// Preventive care recommendation
    PreventiveCare,
    /// Risk factor identified
    RiskFactor,
    /// Lifestyle optimization
    LifestyleOptimization,
    /// Medication optimization
    MedicationOptimization,
    /// Mental health observation
    MentalHealthObservation,
    /// Nutrition recommendation
    Nutrition,
    /// Exercise recommendation
    Exercise,
    /// Sleep pattern observation
    Sleep,
    /// Follow-up reminder
    FollowUpReminder,
    /// Cost-saving opportunity
    CostSaving,
    /// Research/trial opportunity
    ResearchOpportunity,
}

/// Evidence supporting an insight
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvidenceItem {
    /// Type of evidence
    pub evidence_type: EvidenceType,
    /// Description
    pub description: String,
    /// Source (e.g., "Lab result from 2024-01-15")
    pub source: String,
    /// Link to original data if available
    pub data_reference: Option<ActionHash>,
}

/// Types of evidence
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum EvidenceType {
    LabResult,
    VitalSign,
    PatientReported,
    ProviderObservation,
    ResearchStudy,
    ClinicalGuideline,
    PopulationData,
    PatternDetection,
}

/// Recommended action
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecommendedAction {
    /// Action description
    pub action: String,
    /// Priority
    pub priority: Importance,
    /// Timeline
    pub timeline: String,
    /// Who should do this (patient, provider, etc.)
    pub responsible_party: String,
}

/// Urgency level for insights
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Urgency {
    /// Needs immediate attention
    Immediate,
    /// Should be addressed soon
    Soon,
    /// Can be addressed at next visit
    NextVisit,
    /// General awareness
    Informational,
}

// ==================== PROVIDER REVIEWS ====================

/// Provider review/rating
#[hdk_entry_helper]
#[derive(Clone)]
pub struct ProviderReview {
    /// Unique review ID
    pub review_id: String,
    /// Patient who had the experience
    pub patient_hash: ActionHash,
    /// Provider being reviewed
    pub provider_hash: ActionHash,
    /// Provider name
    pub provider_name: String,
    /// Visit date
    pub visit_date: i64,
    /// Overall rating (1-5)
    pub overall_rating: u8,
    /// Rating categories
    pub category_ratings: CategoryRatings,
    /// What went well
    pub positives: Vec<String>,
    /// Areas for improvement
    pub improvements: Vec<String>,
    /// Would recommend to others
    pub would_recommend: bool,
    /// Wait time in minutes
    pub wait_time_minutes: Option<u32>,
    /// Time spent with provider in minutes
    pub time_with_provider_minutes: Option<u32>,
    /// Created at
    pub created_at: i64,
    /// Whether this is public/anonymized
    pub anonymized: bool,
}

/// Category-specific ratings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CategoryRatings {
    /// Communication (1-5)
    pub communication: Option<u8>,
    /// Explanation clarity (1-5)
    pub explanation_clarity: Option<u8>,
    /// Listening skills (1-5)
    pub listening: Option<u8>,
    /// Respect/empathy (1-5)
    pub respect: Option<u8>,
    /// Thoroughness (1-5)
    pub thoroughness: Option<u8>,
    /// Follow-up quality (1-5)
    pub follow_up: Option<u8>,
    /// Office staff (1-5)
    pub office_staff: Option<u8>,
    /// Availability (1-5)
    pub availability: Option<u8>,
}

// ==================== RECOMMENDED QUESTIONS ====================

/// Question recommended for patient to ask
#[hdk_entry_helper]
#[derive(Clone)]
pub struct RecommendedQuestion {
    /// Unique question ID
    pub question_id: String,
    /// Patient this is for
    pub patient_hash: ActionHash,
    /// The question to ask
    pub question: String,
    /// Why this question is important
    pub rationale: String,
    /// Context (appointment, condition, etc.)
    pub context: QuestionContext,
    /// Priority
    pub priority: Importance,
    /// Related condition/topic
    pub related_topic: String,
    /// Generated at
    pub generated_at: i64,
    /// Whether patient asked this
    pub asked: bool,
    /// Answer received (if any)
    pub answer_received: Option<String>,
}

/// Context for a recommended question
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum QuestionContext {
    /// Before an appointment
    PreAppointment(ActionHash),
    /// Related to a diagnosis
    Diagnosis(String),
    /// Related to a medication
    Medication(String),
    /// Related to a procedure
    Procedure(String),
    /// General health
    GeneralHealth,
    /// Second opinion
    SecondOpinion,
}

// ==================== HEALTH ALERTS ====================

/// Alert for potential health issue
#[hdk_entry_helper]
#[derive(Clone)]
pub struct HealthAlert {
    /// Unique alert ID
    pub alert_id: String,
    /// Patient
    pub patient_hash: ActionHash,
    /// Type of alert
    pub alert_type: AlertType,
    /// Plain-language title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Severity
    pub severity: AlertSeverity,
    /// Recommended action
    pub recommended_action: String,
    /// Time limit for action (if applicable)
    pub action_deadline: Option<i64>,
    /// Evidence/data triggering alert
    pub triggering_data: Vec<String>,
    /// Generated at
    pub generated_at: i64,
    /// Status
    pub status: AlertStatus,
    /// Acknowledged at
    pub acknowledged_at: Option<i64>,
    /// Resolved at
    pub resolved_at: Option<i64>,
    /// Resolution notes
    pub resolution_notes: Option<String>,
}

/// Type of health alert
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AlertType {
    /// Critical lab result
    CriticalLabResult,
    /// Drug interaction detected
    DrugInteraction,
    /// Vital sign concern
    VitalSignConcern,
    /// Symptom pattern detected
    SymptomPattern,
    /// Missed appointment/follow-up
    MissedFollowUp,
    /// Medication adherence concern
    MedicationAdherence,
    /// Preventive care overdue
    PreventiveCareOverdue,
    /// Mental health concern
    MentalHealthConcern,
    /// Recall/safety alert
    SafetyRecall,
    /// Insurance/authorization expiring
    InsuranceAlert,
    /// Research opportunity match
    ResearchMatch,
}

/// Alert severity
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AlertSeverity {
    /// Life-threatening - seek immediate care
    Critical,
    /// Serious - contact provider today
    High,
    /// Important - address within a week
    Medium,
    /// Awareness - discuss at next visit
    Low,
    /// Informational
    Info,
}

/// Alert status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AlertStatus {
    /// New, unacknowledged
    New,
    /// Patient has seen it
    Acknowledged,
    /// Being addressed
    InProgress,
    /// Resolved
    Resolved,
    /// Dismissed (false positive, etc.)
    Dismissed,
    /// Escalated to provider
    Escalated,
}

// ==================== ADVOCATE SESSIONS ====================

/// Conversation session with the AI advocate
#[hdk_entry_helper]
#[derive(Clone)]
pub struct AdvocateSession {
    /// Unique session ID
    pub session_id: String,
    /// Patient
    pub patient_hash: ActionHash,
    /// Session topic/purpose
    pub topic: String,
    /// Conversation messages
    pub messages: Vec<ConversationMessage>,
    /// Started at
    pub started_at: i64,
    /// Last message at
    pub last_message_at: i64,
    /// Session ended
    pub ended: bool,
    /// Session summary (generated at end)
    pub summary: Option<String>,
    /// Action items from session
    pub action_items: Vec<String>,
    /// Satisfaction rating (1-5)
    pub satisfaction_rating: Option<u8>,
}

/// A message in a conversation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConversationMessage {
    /// Message ID
    pub message_id: String,
    /// Who sent it
    pub sender: MessageSender,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: i64,
    /// Message type
    pub message_type: MessageType,
}

/// Who sent the message
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MessageSender {
    Patient,
    Advocate,
    System,
}

/// Type of message
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MessageType {
    Text,
    Question,
    Answer,
    Recommendation,
    Alert,
    Summary,
}

// ==================== ADVOCATE PREFERENCES ====================

/// Patient's preferences for the AI advocate
#[hdk_entry_helper]
#[derive(Clone)]
pub struct AdvocatePreferences {
    /// Patient
    pub patient_hash: ActionHash,
    /// Preferred communication style
    pub communication_style: CommunicationStyle,
    /// Health literacy level (for explanation detail)
    pub health_literacy: HealthLiteracyLevel,
    /// Topics patient wants proactive alerts for
    pub alert_topics: Vec<AlertType>,
    /// Topics patient does NOT want alerts for
    pub muted_topics: Vec<AlertType>,
    /// Preferred language
    pub language: String,
    /// Enable appointment prep
    pub enable_appointment_prep: bool,
    /// Enable proactive insights
    pub enable_proactive_insights: bool,
    /// Enable medication monitoring
    pub enable_medication_monitoring: bool,
    /// Daily summary enabled
    pub enable_daily_summary: bool,
    /// Preferred summary time (hour 0-23)
    pub daily_summary_hour: Option<u8>,
    /// Updated at
    pub updated_at: i64,
}

/// Communication style preference
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CommunicationStyle {
    /// Brief, just the facts
    Concise,
    /// Balanced detail
    Balanced,
    /// Thorough explanations
    Detailed,
    /// Supportive, empathetic
    Supportive,
    /// Clinical, professional
    Clinical,
}

/// Health literacy level (for tailoring explanations)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum HealthLiteracyLevel {
    /// Simple language, avoid jargon
    Basic,
    /// Some medical terms okay
    Intermediate,
    /// Comfortable with medical terminology
    Advanced,
    /// Healthcare professional
    Professional,
}

// ==================== SECOND OPINION REQUEST ====================

/// Request for second opinion on diagnosis/treatment
#[hdk_entry_helper]
#[derive(Clone)]
pub struct SecondOpinionRequest {
    /// Unique request ID
    pub request_id: String,
    /// Patient requesting
    pub patient_hash: ActionHash,
    /// Original diagnosis/treatment being questioned
    pub original_diagnosis: String,
    /// Original provider
    pub original_provider: String,
    /// Patient's concerns
    pub concerns: String,
    /// Relevant records to share
    pub relevant_record_hashes: Vec<ActionHash>,
    /// Request type
    pub request_type: SecondOpinionType,
    /// Status
    pub status: SecondOpinionStatus,
    /// AI analysis (if requested)
    pub ai_analysis: Option<AISecondOpinionAnalysis>,
    /// Created at
    pub created_at: i64,
    /// Updated at
    pub updated_at: i64,
}

/// Type of second opinion
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SecondOpinionType {
    /// AI analysis of medical literature
    AIAnalysis,
    /// Request referral to another provider
    ProviderReferral,
    /// Both AI and provider
    Both,
}

/// Status of second opinion request
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SecondOpinionStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

/// AI's analysis for second opinion
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AISecondOpinionAnalysis {
    /// Summary of analysis
    pub summary: String,
    /// Supporting evidence from medical literature
    pub literature_evidence: Vec<String>,
    /// Alternative diagnoses to consider
    pub alternative_considerations: Vec<String>,
    /// Questions to ask original provider
    pub questions_for_provider: Vec<String>,
    /// Confidence in original diagnosis
    pub confidence_in_original: f32,
    /// Generated at
    pub generated_at: i64,
    /// Disclaimer
    pub disclaimer: String,
}

// ==================== MEDICATION CHECK ====================

/// Medication interaction/safety check
#[hdk_entry_helper]
#[derive(Clone)]
pub struct MedicationCheck {
    /// Unique check ID
    pub check_id: String,
    /// Patient
    pub patient_hash: ActionHash,
    /// Medications checked
    pub medications: Vec<MedicationInfo>,
    /// Interactions found
    pub interactions: Vec<DrugInteraction>,
    /// Contraindications based on conditions
    pub contraindications: Vec<Contraindication>,
    /// Optimization opportunities
    pub optimizations: Vec<MedicationOptimization>,
    /// Overall safety score (0-100)
    pub safety_score: u8,
    /// Summary
    pub summary: String,
    /// Generated at
    pub generated_at: i64,
}

/// Medication information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MedicationInfo {
    /// Drug name
    pub name: String,
    /// Dosage
    pub dosage: String,
    /// Frequency
    pub frequency: String,
    /// Prescribing provider
    pub prescriber: Option<String>,
}

/// Drug interaction
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DrugInteraction {
    /// First drug
    pub drug_a: String,
    /// Second drug
    pub drug_b: String,
    /// Severity
    pub severity: InteractionSeverity,
    /// Description
    pub description: String,
    /// Clinical significance
    pub clinical_significance: String,
    /// Recommended action
    pub recommendation: String,
}

/// Interaction severity
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InteractionSeverity {
    /// Life-threatening
    Contraindicated,
    /// Major - avoid if possible
    Major,
    /// Moderate - monitor closely
    Moderate,
    /// Minor - usually not clinically significant
    Minor,
}

/// Contraindication based on patient condition
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Contraindication {
    /// Drug with issue
    pub drug: String,
    /// Condition it conflicts with
    pub condition: String,
    /// Severity
    pub severity: InteractionSeverity,
    /// Description
    pub description: String,
    /// Recommendation
    pub recommendation: String,
}

/// Medication optimization opportunity
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MedicationOptimization {
    /// Type of optimization
    pub optimization_type: OptimizationType,
    /// Description
    pub description: String,
    /// Potential benefit
    pub potential_benefit: String,
    /// Action required
    pub action: String,
}

/// Types of medication optimization
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum OptimizationType {
    /// Generic alternative available
    GenericAvailable,
    /// Therapeutic alternative (different drug, same effect)
    TherapeuticAlternative,
    /// Dosage optimization
    DosageOptimization,
    /// Deprescribing opportunity
    Deprescribing,
    /// Timing optimization
    TimingOptimization,
    /// Cost savings
    CostSavings,
}

// ==================== VALIDATION ====================

/// Validate an appointment prep
pub fn validate_appointment_prep(prep: &AppointmentPrep) -> ExternResult<ValidateCallbackResult> {
    if prep.prep_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Prep ID required".to_string(),
        ));
    }

    if prep.provider_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Provider name required".to_string(),
        ));
    }

    if prep.confidence_score < 0.0 || prep.confidence_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Confidence must be between 0 and 1".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate a health insight
pub fn validate_health_insight(insight: &HealthInsight) -> ExternResult<ValidateCallbackResult> {
    if insight.insight_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Insight ID required".to_string(),
        ));
    }

    if insight.summary.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Summary required".to_string(),
        ));
    }

    if insight.confidence < 0.0 || insight.confidence > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Confidence must be between 0 and 1".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate a provider review
pub fn validate_provider_review(review: &ProviderReview) -> ExternResult<ValidateCallbackResult> {
    if review.review_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Review ID required".to_string(),
        ));
    }

    if review.overall_rating < 1 || review.overall_rating > 5 {
        return Ok(ValidateCallbackResult::Invalid(
            "Rating must be between 1 and 5".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate a health alert
pub fn validate_health_alert(alert: &HealthAlert) -> ExternResult<ValidateCallbackResult> {
    if alert.alert_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Alert ID required".to_string(),
        ));
    }

    if alert.title.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Alert title required".to_string(),
        ));
    }

    if alert.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Alert description required".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate advocate preferences
pub fn validate_advocate_preferences(
    prefs: &AdvocatePreferences,
) -> ExternResult<ValidateCallbackResult> {
    if let Some(hour) = prefs.daily_summary_hour {
        if hour > 23 {
            return Ok(ValidateCallbackResult::Invalid(
                "Daily summary hour must be 0-23".to_string(),
            ));
        }
    }

    if prefs.language.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Language required".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate medication check
pub fn validate_medication_check(check: &MedicationCheck) -> ExternResult<ValidateCallbackResult> {
    if check.check_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Check ID required".to_string(),
        ));
    }

    if check.medications.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one medication required".to_string(),
        ));
    }

    if check.safety_score > 100 {
        return Ok(ValidateCallbackResult::Invalid(
            "Safety score must be 0-100".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}
