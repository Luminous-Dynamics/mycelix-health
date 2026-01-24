//! Mental Health Integrity Zome
//!
//! Behavioral health management with enhanced privacy protections.
//! Supports 42 CFR Part 2 compliance, segmented consent, and crisis protocols.

use hdi::prelude::*;

/// Mental health screening instruments
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MentalHealthInstrument {
    /// Patient Health Questionnaire (depression)
    PHQ9,
    /// PHQ-2 brief screen
    PHQ2,
    /// Generalized Anxiety Disorder scale
    GAD7,
    /// Columbia Suicide Severity Rating Scale
    CSSRS,
    /// CAGE questionnaire (alcohol)
    CAGE,
    /// AUDIT (alcohol use)
    AUDIT,
    /// DAST (drug abuse)
    DAST10,
    /// PCL-5 (PTSD)
    PCL5,
    /// MDQ (bipolar)
    MDQ,
    /// Edinburgh Postnatal Depression Scale
    EPDS,
    /// Pediatric Symptom Checklist
    PSC17,
    /// Custom instrument
    Custom(String),
}

/// Severity levels for mental health conditions
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    None,
    Minimal,
    Mild,
    Moderate,
    ModeratelySevere,
    Severe,
}

/// Substance categories for 42 CFR Part 2
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SubstanceCategory {
    Alcohol,
    Cannabis,
    Opioids,
    Stimulants,
    Sedatives,
    Hallucinogens,
    Tobacco,
    Other(String),
}

/// Crisis level
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CrisisLevel {
    None,
    LowRisk,
    ModerateRisk,
    HighRisk,
    Imminent,
}

/// Treatment modality
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TreatmentModality {
    IndividualTherapy,
    GroupTherapy,
    FamilyTherapy,
    Medication,
    IntensiveOutpatient,
    PartialHospitalization,
    Inpatient,
    CrisisIntervention,
    PeerSupport,
    Telehealth,
    Other(String),
}

/// Safety plan status
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SafetyPlanStatus {
    Active,
    NeedsUpdate,
    Expired,
    NotApplicable,
}

/// 42 CFR Part 2 consent type
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Part2ConsentType {
    /// General disclosure
    GeneralDisclosure,
    /// Re-disclosure prohibited notice
    RedisclosureProhibited,
    /// Medical emergency exception
    MedicalEmergency,
    /// Research exception
    Research,
    /// Court order
    CourtOrder,
    /// Audit and evaluation
    AuditEvaluation,
}

/// Mental health screening result
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MentalHealthScreening {
    pub patient_hash: ActionHash,
    pub provider_hash: AgentPubKey,
    pub instrument: MentalHealthInstrument,
    pub screening_date: Timestamp,
    pub raw_score: u32,
    pub severity: Severity,
    pub responses: Vec<(String, u8)>, // question_id -> score
    pub interpretation: String,
    pub follow_up_recommended: bool,
    pub crisis_indicators_present: bool,
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

/// Mood/symptom tracking entry (patient self-report)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MoodEntry {
    pub patient_hash: ActionHash,
    pub entry_date: Timestamp,
    pub mood_score: u8, // 1-10
    pub anxiety_score: u8, // 1-10
    pub sleep_quality: u8, // 1-10
    pub sleep_hours: Option<f32>,
    pub energy_level: u8, // 1-10
    pub appetite: Option<String>,
    pub medications_taken: bool,
    pub activities: Vec<String>,
    pub triggers: Vec<String>,
    pub coping_strategies_used: Vec<String>,
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

/// Treatment plan
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MentalHealthTreatmentPlan {
    pub patient_hash: ActionHash,
    pub provider_hash: AgentPubKey,
    pub primary_diagnosis_icd10: String,
    pub secondary_diagnoses: Vec<String>,
    pub treatment_goals: Vec<TreatmentGoal>,
    pub modalities: Vec<TreatmentModality>,
    pub medications: Vec<PsychMedication>,
    pub session_frequency: String,
    pub estimated_duration: Option<String>,
    pub crisis_plan_hash: Option<ActionHash>,
    pub effective_date: Timestamp,
    pub review_date: Timestamp,
    pub status: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Treatment goal
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TreatmentGoal {
    pub goal_id: String,
    pub description: String,
    pub target_date: Option<Timestamp>,
    pub progress: String, // Not Started, In Progress, Achieved
    pub interventions: Vec<String>,
}

/// Psychiatric medication
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PsychMedication {
    pub name: String,
    pub rxnorm_code: Option<String>,
    pub dosage: String,
    pub frequency: String,
    pub prescriber_hash: ActionHash,
    pub start_date: Timestamp,
    pub target_symptoms: Vec<String>,
    pub side_effects_reported: Vec<String>,
}

/// Safety/crisis plan
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SafetyPlan {
    pub patient_hash: ActionHash,
    pub provider_hash: AgentPubKey,
    pub warning_signs: Vec<String>,
    pub internal_coping_strategies: Vec<String>,
    pub people_for_distraction: Vec<ContactInfo>,
    pub people_for_help: Vec<ContactInfo>,
    pub professionals_to_contact: Vec<ContactInfo>,
    pub crisis_line_988: bool,
    pub additional_crisis_resources: Vec<String>,
    pub environment_safety_steps: Vec<String>,
    pub reasons_for_living: Vec<String>,
    pub status: SafetyPlanStatus,
    pub created_at: Timestamp,
    pub last_reviewed: Timestamp,
    pub next_review_date: Timestamp,
}

/// Contact info for safety plan
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactInfo {
    pub name: String,
    pub relationship: Option<String>,
    pub phone: String,
    pub available_hours: Option<String>,
}

/// Crisis event record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CrisisEvent {
    pub patient_hash: ActionHash,
    pub reporter_hash: AgentPubKey,
    pub event_date: Timestamp,
    pub crisis_level: CrisisLevel,
    pub suicidal_ideation: bool,
    pub homicidal_ideation: bool,
    pub self_harm: bool,
    pub substance_intoxication: bool,
    pub psychotic_symptoms: bool,
    pub description: String,
    pub intervention_taken: String,
    pub disposition: String, // e.g., "Discharged home", "Inpatient admission"
    pub follow_up_plan: String,
    pub safety_plan_reviewed: bool,
    pub created_at: Timestamp,
}

/// 42 CFR Part 2 specific consent for substance abuse records
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Part2Consent {
    pub patient_hash: ActionHash,
    pub consent_type: Part2ConsentType,
    pub disclosing_program: String,
    pub recipient_name: String,
    pub recipient_hash: Option<ActionHash>,
    pub purpose: String,
    pub information_to_disclose: Vec<String>,
    pub substances_covered: Vec<SubstanceCategory>,
    pub effective_date: Timestamp,
    pub expiration_date: Option<Timestamp>,
    pub right_to_revoke_explained: bool,
    pub patient_signature_date: Timestamp,
    pub witness_name: Option<String>,
    pub is_revoked: bool,
    pub revocation_date: Option<Timestamp>,
    pub created_at: Timestamp,
}

/// Therapy session note (protected)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TherapyNote {
    pub patient_hash: ActionHash,
    pub provider_hash: AgentPubKey,
    pub session_date: Timestamp,
    pub session_type: TreatmentModality,
    pub duration_minutes: u32,
    pub presenting_concerns: String,
    pub mental_status: Option<String>,
    pub interventions_used: Vec<String>,
    pub patient_response: String,
    pub risk_assessment: Option<CrisisLevel>,
    pub plan_for_next_session: String,
    /// These are psychotherapy notes - extra protected under HIPAA
    pub is_psychotherapy_note: bool,
    pub created_at: Timestamp,
}

/// Peer support connection
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PeerSupportConnection {
    pub patient_hash: ActionHash,
    pub peer_specialist_hash: ActionHash,
    pub connection_type: String,
    pub meeting_frequency: String,
    pub goals: Vec<String>,
    pub start_date: Timestamp,
    pub status: String,
    pub created_at: Timestamp,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    MentalHealthScreening(MentalHealthScreening),
    MoodEntry(MoodEntry),
    MentalHealthTreatmentPlan(MentalHealthTreatmentPlan),
    SafetyPlan(SafetyPlan),
    CrisisEvent(CrisisEvent),
    Part2Consent(Part2Consent),
    TherapyNote(TherapyNote),
    PeerSupportConnection(PeerSupportConnection),
}

#[hdk_link_types]
pub enum LinkTypes {
    PatientToScreenings,
    PatientToMoodEntries,
    PatientToTreatmentPlans,
    PatientToSafetyPlan,
    PatientToCrisisEvents,
    PatientToPart2Consents,
    PatientToTherapyNotes,
    ProviderToPatients,
    PatientToPeerSupport,
}

/// Validate mental health entries
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => validate_create_entry(app_entry),
            OpEntry::UpdateEntry { app_entry, .. } => validate_create_entry(app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(entry: EntryTypes) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::CrisisEvent(event) => validate_crisis_event(&event),
        EntryTypes::Part2Consent(consent) => validate_part2_consent(&consent),
        EntryTypes::SafetyPlan(plan) => validate_safety_plan(&plan),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_crisis_event(event: &CrisisEvent) -> ExternResult<ValidateCallbackResult> {
    // Must have description
    if event.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Crisis event must have description".to_string(),
        ));
    }

    // Must have intervention
    if event.intervention_taken.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Crisis event must document intervention taken".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_part2_consent(consent: &Part2Consent) -> ExternResult<ValidateCallbackResult> {
    // Must have purpose
    if consent.purpose.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Part 2 consent must specify purpose".to_string(),
        ));
    }

    // Must have recipient
    if consent.recipient_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Part 2 consent must specify recipient".to_string(),
        ));
    }

    // Right to revoke must be explained
    if !consent.right_to_revoke_explained {
        return Ok(ValidateCallbackResult::Invalid(
            "Patient must be informed of right to revoke consent".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_safety_plan(plan: &SafetyPlan) -> ExternResult<ValidateCallbackResult> {
    // Must have at least one warning sign
    if plan.warning_signs.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Safety plan must include warning signs".to_string(),
        ));
    }

    // Must have at least one coping strategy
    if plan.internal_coping_strategies.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Safety plan must include coping strategies".to_string(),
        ));
    }

    // 988 should be included
    if !plan.crisis_line_988 {
        return Ok(ValidateCallbackResult::Invalid(
            "Safety plan should include 988 crisis line".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}
