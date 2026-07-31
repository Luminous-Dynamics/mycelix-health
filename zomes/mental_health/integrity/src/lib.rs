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
    pub mood_score: u8,    // 1-10
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

// ── Rehabilitation & Recovery Types ──

/// Treatment phase in the recovery continuum.
///
/// Science: SAMHSA recovery model — 4 stages of recovery.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TreatmentPhase {
    /// Medical stabilization and withdrawal management
    Detoxification,
    /// First 90 days: building coping skills, establishing routines
    EarlyAbstinence,
    /// 3-12 months: maintaining gains, deepening community ties
    Maintenance,
    /// 1+ years: integration, giving back, sustained wellness
    AdvancedRecovery,
}

/// Milestone type for tracking recovery progress.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MilestoneType {
    /// Sobriety duration (days since last use)
    SobrietyDate { days: u32 },
    /// 12-step program step completion
    StepCompletion { step: u8 },
    /// Transition between treatment phases
    TreatmentPhaseTransition {
        from: TreatmentPhase,
        to: TreatmentPhase,
    },
    /// Consecutive days of medication adherence
    MedicationAdherenceStreak { days: u32 },
    /// Peer support sessions attended
    PeerSupportSession { count: u32 },
    /// Stable employment achieved
    EmploymentMilestone,
    /// Stable housing secured
    HousingStability,
    /// Custom milestone
    Custom(String),
}

/// Recovery milestone — attestable progress marker.
///
/// Milestones can be self-reported or verified by a sponsor/counselor.
/// Verified milestones carry more weight in the 4D Trust Profile.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RecoveryMilestone {
    pub patient_hash: ActionHash,
    pub milestone_type: MilestoneType,
    pub achieved_at: Timestamp,
    /// Sponsor or counselor who attested this milestone (optional).
    pub verified_by: Option<AgentPubKey>,
    pub notes: String,
    pub created_at: Timestamp,
}

/// Coping action with self-rated effectiveness.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CopingAction {
    pub trigger_category: String,
    pub strategy: String,
    /// Self-rated effectiveness after use (1-10), if rated.
    pub effectiveness_rating: Option<u8>,
}

/// Relapse prevention plan — personal trigger inventory and coping strategies.
///
/// Science: Marlatt & Gordon (1985) — relapse prevention model.
/// Witkiewitz & Marlatt (2004) — dynamic model of relapse.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RelapsePrevention {
    pub patient_hash: ActionHash,
    /// Personal trigger inventory
    pub triggers: Vec<String>,
    /// Identified high-risk situations
    pub high_risk_situations: Vec<String>,
    /// Planned coping responses for each trigger/situation
    pub coping_plan: Vec<CopingAction>,
    /// Emergency support contacts (maps to Care Circle members)
    pub support_contacts: Vec<ContactInfo>,
    pub last_reviewed: Timestamp,
    pub created_at: Timestamp,
}

/// Recovery check-in for longitudinal tracking.
///
/// Science: McLellan et al. (2005) — continuing care monitoring.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RecoveryCheckIn {
    pub patient_hash: ActionHash,
    pub checked_in_by: AgentPubKey,
    /// Self-rated mood (1-10)
    pub mood_score: u8,
    /// Craving intensity (0-10, 0 = none)
    pub craving_intensity: u8,
    /// Triggers encountered since last check-in
    pub triggers_encountered: Vec<String>,
    /// Coping strategies used
    pub coping_used: Vec<String>,
    /// Sleep quality (1-10)
    pub sleep_quality: u8,
    /// Social support quality (1-10)
    pub social_support_quality: u8,
    pub notes: Option<String>,
    pub timestamp: Timestamp,
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
    RecoveryMilestone(RecoveryMilestone),
    RelapsePrevention(RelapsePrevention),
    RecoveryCheckIn(RecoveryCheckIn),
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
    PatientToMilestones,
    PatientToRelapsePrevention,
    PatientToCheckIns,
}

/// Validate mental health entries
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => validate_create_entry(action, app_entry),
            OpEntry::UpdateEntry {
                app_entry, action, ..
            } => validate_update_entry(action, app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // Deliberately left permissive: the coordinator never calls
        // delete_link or delete_entry anywhere in this zome (confirmed
        // via grep), so RegisterDeleteLink/RegisterDelete hardening
        // would be pure defense-in-depth with zero functional impact --
        // still worth doing given the pattern established elsewhere this
        // pass, but this zome's flagged item was specifically about
        // author-binding on create/update, so scoped narrowly here.
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
            // Previously left fully permissive (`Ok(Valid)` unconditionally
            // via the catch-all `_` arm) -- the 16th confirmed instance of
            // this exact bug pattern this pass. Found + fixed 2026-07-09
            // during the P0 author-binding pass.
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

fn validate_create_entry(
    action: Create,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::MentalHealthScreening(screening) => {
            // Author-binding: create_screening already derives
            // provider_hash from agent_info(), so this is
            // belt-and-suspenders. Found + fixed 2026-07-09 during the
            // P0 author-binding pass -- this entry type had NO
            // validation at all before this fix.
            if screening.provider_hash != action.author {
                return Ok(ValidateCallbackResult::Invalid(
                    "provider_hash must correspond to the committing agent".into(),
                ));
            }
            Ok(ValidateCallbackResult::Valid)
        }
        // MoodEntry is patient self-report with no identity field to bind
        // (patient_hash is an ActionHash reference, not an AgentPubKey).
        EntryTypes::MoodEntry(_) => Ok(ValidateCallbackResult::Valid),
        EntryTypes::MentalHealthTreatmentPlan(plan) => {
            // Same belt-and-suspenders pattern as screening --
            // create_treatment_plan already derives provider_hash.
            if plan.provider_hash != action.author {
                return Ok(ValidateCallbackResult::Invalid(
                    "provider_hash must correspond to the committing agent".into(),
                ));
            }
            Ok(ValidateCallbackResult::Valid)
        }
        EntryTypes::SafetyPlan(plan) => {
            if plan.provider_hash != action.author {
                return Ok(ValidateCallbackResult::Invalid(
                    "provider_hash must correspond to the committing agent".into(),
                ));
            }
            validate_safety_plan(&plan)
        }
        EntryTypes::CrisisEvent(event) => {
            if event.reporter_hash != action.author {
                return Ok(ValidateCallbackResult::Invalid(
                    "reporter_hash must correspond to the committing agent".into(),
                ));
            }
            validate_crisis_event(&event)
        }
        // Part2Consent has no agent-identity field (recipient_name/
        // disclosing_program describe external parties, not committers).
        EntryTypes::Part2Consent(consent) => validate_part2_consent(&consent),
        EntryTypes::TherapyNote(note) => {
            if note.provider_hash != action.author {
                return Ok(ValidateCallbackResult::Invalid(
                    "provider_hash must correspond to the committing agent".into(),
                ));
            }
            Ok(ValidateCallbackResult::Valid)
        }
        // peer_specialist_hash is an ActionHash reference to a provider
        // RECORD, not a directly comparable AgentPubKey -- no binding
        // possible without an extra must_get lookup; not attempted here.
        EntryTypes::PeerSupportConnection(_) => Ok(ValidateCallbackResult::Valid),
        EntryTypes::RecoveryMilestone(milestone) => {
            // verified_by is always None on create (log_milestone
            // hardcodes it) -- this check is defense-in-depth should that
            // ever change, matching the same "if Some, must equal
            // author" idiom used for cds's ClinicalAlert.acknowledged_by.
            if let Some(verified_by) = &milestone.verified_by {
                if *verified_by != action.author {
                    return Ok(ValidateCallbackResult::Invalid(
                        "verified_by must correspond to the committing agent".into(),
                    ));
                }
            }
            Ok(ValidateCallbackResult::Valid)
        }
        // No agent-identity field.
        EntryTypes::RelapsePrevention(plan) => validate_relapse_prevention(&plan),
        EntryTypes::RecoveryCheckIn(check_in) => {
            if check_in.checked_in_by != action.author {
                return Ok(ValidateCallbackResult::Invalid(
                    "checked_in_by must correspond to the committing agent".into(),
                ));
            }
            validate_recovery_check_in(&check_in)
        }
    }
}

/// Only three entry types have a live coordinator update path (confirmed
/// via grep for `update_entry`): Part2Consent (revoke_part2_consent),
/// MentalHealthTreatmentPlan (update_treatment_goal/close_treatment_plan),
/// and RecoveryMilestone (verify_milestone). The other 8 entry types have
/// no update call at all and are made explicitly immutable. Reviewed
/// 2026-07-09 during the P0 author-binding pass -- previously ALL 11
/// entry types routed updates through the same create-shaped validator
/// (most of which did zero validation at all), so a modified coordinator
/// could silently rewrite any field of any entry, including
/// provider_hash on a therapy note or treatment plan.
fn validate_update_entry(
    action: Update,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::Part2Consent(consent) => validate_update_part2_consent(action, consent),
        EntryTypes::MentalHealthTreatmentPlan(plan) => validate_update_treatment_plan(action, plan),
        EntryTypes::RecoveryMilestone(milestone) => validate_update_milestone(action, milestone),
        EntryTypes::MentalHealthScreening(_) => Ok(ValidateCallbackResult::Invalid(
            "Mental health screenings are immutable".into(),
        )),
        EntryTypes::MoodEntry(_) => Ok(ValidateCallbackResult::Invalid(
            "Mood entries are immutable".into(),
        )),
        EntryTypes::SafetyPlan(_) => Ok(ValidateCallbackResult::Invalid(
            "Safety plans are immutable; create a new one".into(),
        )),
        EntryTypes::CrisisEvent(_) => Ok(ValidateCallbackResult::Invalid(
            "Crisis events are immutable".into(),
        )),
        EntryTypes::TherapyNote(_) => Ok(ValidateCallbackResult::Invalid(
            "Therapy notes are immutable".into(),
        )),
        EntryTypes::PeerSupportConnection(_) => Ok(ValidateCallbackResult::Invalid(
            "Peer support connections are immutable".into(),
        )),
        EntryTypes::RelapsePrevention(_) => Ok(ValidateCallbackResult::Invalid(
            "Relapse prevention plans are immutable; create a new one".into(),
        )),
        EntryTypes::RecoveryCheckIn(_) => Ok(ValidateCallbackResult::Invalid(
            "Recovery check-ins are immutable".into(),
        )),
    }
}

/// Content restricted to is_revoked/revocation_date -- the exact fields
/// revoke_part2_consent changes. No author requirement: revocation is
/// gated on require_authorization (patient-data write access), not on
/// being the original consent's committer -- no established authority
/// model here to bind against. Case (c).
fn validate_update_part2_consent(
    action: Update,
    consent: Part2Consent,
) -> ExternResult<ValidateCallbackResult> {
    let original_record = must_get_valid_record(action.original_action_address.clone())?;
    let original: Part2Consent = original_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Original consent not found".into()
        )))?;

    if consent.patient_hash != original.patient_hash
        || consent.consent_type != original.consent_type
        || consent.disclosing_program != original.disclosing_program
        || consent.recipient_name != original.recipient_name
        || consent.recipient_hash != original.recipient_hash
        || consent.purpose != original.purpose
        || consent.information_to_disclose != original.information_to_disclose
        || consent.substances_covered != original.substances_covered
        || consent.effective_date != original.effective_date
        || consent.expiration_date != original.expiration_date
        || consent.right_to_revoke_explained != original.right_to_revoke_explained
        || consent.patient_signature_date != original.patient_signature_date
        || consent.witness_name != original.witness_name
        || consent.created_at != original.created_at
    {
        return Ok(ValidateCallbackResult::Invalid(
            "Only is_revoked/revocation_date can change on a Part 2 consent update".into(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Content restricted to treatment_goals (progress field only, per
/// goal)/status/updated_at -- the exact fields update_treatment_goal/
/// close_treatment_plan change. No author requirement: both are gated on
/// require_authorization (patient-data write access), not on being the
/// plan's original provider -- a different provider may legitimately
/// take over a patient's care. Case (c).
fn validate_update_treatment_plan(
    action: Update,
    plan: MentalHealthTreatmentPlan,
) -> ExternResult<ValidateCallbackResult> {
    let original_record = must_get_valid_record(action.original_action_address.clone())?;
    let original: MentalHealthTreatmentPlan = original_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Original treatment plan not found".into()
        )))?;

    let goals_match = plan.treatment_goals.len() == original.treatment_goals.len()
        && plan
            .treatment_goals
            .iter()
            .zip(original.treatment_goals.iter())
            .all(|(new, old)| {
                new.goal_id == old.goal_id
                    && new.description == old.description
                    && new.target_date == old.target_date
                    && new.interventions == old.interventions
            });

    if !goals_match
        || plan.patient_hash != original.patient_hash
        || plan.provider_hash != original.provider_hash
        || plan.primary_diagnosis_icd10 != original.primary_diagnosis_icd10
        || plan.secondary_diagnoses != original.secondary_diagnoses
        || plan.modalities != original.modalities
        || plan.medications != original.medications
        || plan.session_frequency != original.session_frequency
        || plan.estimated_duration != original.estimated_duration
        || plan.crisis_plan_hash != original.crisis_plan_hash
        || plan.effective_date != original.effective_date
        || plan.review_date != original.review_date
        || plan.created_at != original.created_at
    {
        return Ok(ValidateCallbackResult::Invalid(
            "Only goal progress/status/updated_at can change on a treatment plan update".into(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Content restricted to verified_by -- the exact field verify_milestone
/// changes. Author-binding DOES apply here (unlike the two update
/// validators above): verify_milestone always derives verified_by from
/// agent_info(), so whoever names themselves as verifier must be the
/// actual committing agent -- belt-and-suspenders.
fn validate_update_milestone(
    action: Update,
    milestone: RecoveryMilestone,
) -> ExternResult<ValidateCallbackResult> {
    if let Some(verified_by) = &milestone.verified_by {
        if *verified_by != action.author {
            return Ok(ValidateCallbackResult::Invalid(
                "verified_by must correspond to the committing agent".into(),
            ));
        }
    }

    let original_record = must_get_valid_record(action.original_action_address.clone())?;
    let original: RecoveryMilestone = original_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Original milestone not found".into()
        )))?;

    if milestone.patient_hash != original.patient_hash
        || milestone.milestone_type != original.milestone_type
        || milestone.achieved_at != original.achieved_at
        || milestone.notes != original.notes
        || milestone.created_at != original.created_at
    {
        return Ok(ValidateCallbackResult::Invalid(
            "Only verified_by can change on a milestone update".into(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
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

fn validate_recovery_check_in(check_in: &RecoveryCheckIn) -> ExternResult<ValidateCallbackResult> {
    if check_in.mood_score < 1 || check_in.mood_score > 10 {
        return Ok(ValidateCallbackResult::Invalid(
            "Mood score must be 1-10".to_string(),
        ));
    }
    if check_in.craving_intensity > 10 {
        return Ok(ValidateCallbackResult::Invalid(
            "Craving intensity must be 0-10".to_string(),
        ));
    }
    if check_in.sleep_quality < 1 || check_in.sleep_quality > 10 {
        return Ok(ValidateCallbackResult::Invalid(
            "Sleep quality must be 1-10".to_string(),
        ));
    }
    if check_in.social_support_quality < 1 || check_in.social_support_quality > 10 {
        return Ok(ValidateCallbackResult::Invalid(
            "Social support quality must be 1-10".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_relapse_prevention(plan: &RelapsePrevention) -> ExternResult<ValidateCallbackResult> {
    if plan.triggers.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Relapse prevention plan must identify at least one trigger".to_string(),
        ));
    }
    if plan.coping_plan.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Relapse prevention plan must include at least one coping action".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

#[cfg(test)]
mod author_binding_tests {
    use super::*;

    fn create_action(author: AgentPubKey) -> Create {
        Create {
            author,
            timestamp: Timestamp::from_micros(0),
            action_seq: 0,
            prev_action: ActionHash::from_raw_36(vec![0u8; 36]),
            entry_type: EntryType::App(AppEntryDef::new(
                EntryDefIndex::from(0),
                0.into(),
                EntryVisibility::Public,
            )),
            entry_hash: EntryHash::from_raw_36(vec![0u8; 36]),
            weight: Default::default(),
        }
    }

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

    fn other_agent() -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![1u8; 36])
    }

    fn valid_screening(provider_hash: AgentPubKey) -> MentalHealthScreening {
        MentalHealthScreening {
            patient_hash: ActionHash::from_raw_36(vec![2u8; 36]),
            provider_hash,
            instrument: MentalHealthInstrument::PHQ9,
            screening_date: Timestamp::from_micros(0),
            raw_score: 10,
            severity: Severity::Moderate,
            responses: vec![],
            interpretation: "moderate".into(),
            follow_up_recommended: true,
            crisis_indicators_present: false,
            notes: None,
            created_at: Timestamp::from_micros(0),
        }
    }

    #[test]
    fn create_screening_valid_when_provider_matches_committer() {
        let author = me();
        let s = valid_screening(author.clone());
        let result =
            validate_create_entry(create_action(author), EntryTypes::MentalHealthScreening(s))
                .unwrap();
        assert_eq!(result, ValidateCallbackResult::Valid);
    }

    #[test]
    fn create_screening_forgery_rejected() {
        let s = valid_screening(me());
        let result = validate_create_entry(
            create_action(other_agent()),
            EntryTypes::MentalHealthScreening(s),
        )
        .unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    fn valid_crisis_event(reporter_hash: AgentPubKey) -> CrisisEvent {
        CrisisEvent {
            patient_hash: ActionHash::from_raw_36(vec![2u8; 36]),
            reporter_hash,
            event_date: Timestamp::from_micros(0),
            crisis_level: CrisisLevel::ModerateRisk,
            suicidal_ideation: false,
            homicidal_ideation: false,
            self_harm: false,
            substance_intoxication: false,
            psychotic_symptoms: false,
            description: "desc".into(),
            intervention_taken: "assessed".into(),
            disposition: "Discharged home".into(),
            follow_up_plan: "follow up".into(),
            safety_plan_reviewed: true,
            created_at: Timestamp::from_micros(0),
        }
    }

    #[test]
    fn create_crisis_event_forgery_rejected() {
        let e = valid_crisis_event(me());
        let result =
            validate_create_entry(create_action(other_agent()), EntryTypes::CrisisEvent(e))
                .unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    fn valid_check_in(checked_in_by: AgentPubKey) -> RecoveryCheckIn {
        RecoveryCheckIn {
            patient_hash: ActionHash::from_raw_36(vec![2u8; 36]),
            checked_in_by,
            mood_score: 5,
            craving_intensity: 2,
            triggers_encountered: vec![],
            coping_used: vec![],
            sleep_quality: 7,
            social_support_quality: 6,
            notes: None,
            timestamp: Timestamp::from_micros(0),
        }
    }

    #[test]
    fn create_check_in_forgery_rejected() {
        let c = valid_check_in(me());
        let result =
            validate_create_entry(create_action(other_agent()), EntryTypes::RecoveryCheckIn(c))
                .unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    fn valid_milestone(verified_by: Option<AgentPubKey>) -> RecoveryMilestone {
        RecoveryMilestone {
            patient_hash: ActionHash::from_raw_36(vec![2u8; 36]),
            milestone_type: MilestoneType::EmploymentMilestone,
            achieved_at: Timestamp::from_micros(0),
            verified_by,
            notes: "".into(),
            created_at: Timestamp::from_micros(0),
        }
    }

    #[test]
    fn create_milestone_valid_with_no_verifier() {
        let m = valid_milestone(None);
        let result =
            validate_create_entry(create_action(me()), EntryTypes::RecoveryMilestone(m)).unwrap();
        assert_eq!(result, ValidateCallbackResult::Valid);
    }

    #[test]
    fn milestone_verify_forgery_rejected_before_must_get() {
        // verified_by must match the committing agent; checked before
        // must_get_valid_record, so testable without a live HDI host.
        let m = valid_milestone(Some(me()));
        let result = validate_update_milestone(update_action(other_agent()), m).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn update_entry_rejects_mood_entry_update() {
        let m = MoodEntry {
            patient_hash: ActionHash::from_raw_36(vec![2u8; 36]),
            entry_date: Timestamp::from_micros(0),
            mood_score: 5,
            anxiety_score: 3,
            sleep_quality: 7,
            sleep_hours: None,
            energy_level: 6,
            appetite: None,
            medications_taken: true,
            activities: vec![],
            triggers: vec![],
            coping_strategies_used: vec![],
            notes: None,
            created_at: Timestamp::from_micros(0),
        };
        let result = validate_update_entry(update_action(me()), EntryTypes::MoodEntry(m)).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }
}
