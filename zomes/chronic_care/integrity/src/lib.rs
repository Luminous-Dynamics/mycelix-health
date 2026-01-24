//! Chronic Care Integrity Zome
//!
//! Chronic disease management for diabetes, heart failure, COPD, CKD, and cancer survivorship.

use hdi::prelude::*;

/// Chronic condition types
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChronicCondition {
    Diabetes(DiabetesType),
    HeartFailure(HeartFailureClass),
    COPD(COPDStage),
    ChronicKidneyDisease(CKDStage),
    Hypertension,
    Asthma,
    CancerSurvivorship(String), // cancer type
    MultipleSclerosis,
    RheumatoidArthritis,
    Obesity,
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DiabetesType {
    Type1,
    Type2,
    Gestational,
    Prediabetes,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum HeartFailureClass {
    ClassI,   // No symptoms
    ClassII,  // Mild symptoms
    ClassIII, // Marked limitation
    ClassIV,  // Severe symptoms
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum COPDStage {
    Mild,
    Moderate,
    Severe,
    VerySevere,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CKDStage {
    Stage1, // eGFR >= 90
    Stage2, // eGFR 60-89
    Stage3a, // eGFR 45-59
    Stage3b, // eGFR 30-44
    Stage4, // eGFR 15-29
    Stage5, // eGFR < 15
}

/// Care plan status
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CarePlanStatus {
    Draft,
    Active,
    OnHold,
    Completed,
    Cancelled,
}

/// Alert severity
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Urgent,
    Critical,
}

/// Chronic disease enrollment
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ChronicDiseaseEnrollment {
    pub patient_hash: ActionHash,
    pub condition: ChronicCondition,
    pub icd10_code: String,
    pub diagnosis_date: Timestamp,
    pub enrolled_date: Timestamp,
    pub primary_provider_hash: ActionHash,
    pub care_team: Vec<ActionHash>,
    pub is_active: bool,
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

/// Care plan for chronic condition
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ChronicCarePlan {
    pub enrollment_hash: ActionHash,
    pub patient_hash: ActionHash,
    pub condition: ChronicCondition,
    pub goals: Vec<CareGoal>,
    pub medications: Vec<String>,
    pub monitoring_schedule: MonitoringSchedule,
    pub self_management_tasks: Vec<SelfManagementTask>,
    pub education_topics: Vec<String>,
    pub status: CarePlanStatus,
    pub effective_date: Timestamp,
    pub review_date: Timestamp,
    pub created_by: ActionHash,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Care goal
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CareGoal {
    pub goal_id: String,
    pub description: String,
    pub target_value: Option<String>,
    pub target_date: Option<Timestamp>,
    pub status: String,
    pub progress_notes: Vec<String>,
}

/// Monitoring schedule
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MonitoringSchedule {
    pub vitals_frequency: String,
    pub lab_tests: Vec<(String, String)>, // (test, frequency)
    pub office_visits: String,
    pub specialist_follow_ups: Vec<(String, String)>,
}

/// Self-management task
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelfManagementTask {
    pub task_id: String,
    pub description: String,
    pub frequency: String,
    pub instructions: Option<String>,
}

/// Patient-reported outcome (e.g., daily glucose, weight, symptoms)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PatientReportedOutcome {
    pub patient_hash: ActionHash,
    pub enrollment_hash: ActionHash,
    pub measurement_type: String,
    pub value: f64,
    pub unit: String,
    pub measurement_date: Timestamp,
    pub context: Option<String>, // e.g., "fasting", "post-meal"
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

/// Condition-specific metrics
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DiabetesMetrics {
    pub patient_hash: ActionHash,
    pub measurement_date: Timestamp,
    pub fasting_glucose: Option<f64>,
    pub post_meal_glucose: Option<f64>,
    pub hba1c: Option<f64>,
    pub insulin_units: Option<f64>,
    pub carbs_consumed: Option<u32>,
    pub hypoglycemic_events: u32,
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HeartFailureMetrics {
    pub patient_hash: ActionHash,
    pub measurement_date: Timestamp,
    pub weight_kg: f64,
    pub weight_change_kg: Option<f64>,
    pub blood_pressure_systolic: Option<u32>,
    pub blood_pressure_diastolic: Option<u32>,
    pub heart_rate: Option<u32>,
    pub edema_level: Option<u8>, // 0-4
    pub dyspnea_level: Option<u8>, // 0-4
    pub fatigue_level: Option<u8>, // 0-10
    pub fluid_intake_ml: Option<u32>,
    pub sodium_intake_mg: Option<u32>,
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct COPDMetrics {
    pub patient_hash: ActionHash,
    pub measurement_date: Timestamp,
    pub peak_flow: Option<u32>,
    pub fev1: Option<f64>,
    pub oxygen_saturation: Option<u8>,
    pub rescue_inhaler_uses: u32,
    pub symptom_score: Option<u8>, // CAT score
    pub exacerbation: bool,
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

/// Medication adherence record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MedicationAdherence {
    pub patient_hash: ActionHash,
    pub medication_name: String,
    pub rxnorm_code: Option<String>,
    pub scheduled_date: Timestamp,
    pub taken: bool,
    pub taken_time: Option<Timestamp>,
    pub dosage: String,
    pub reason_missed: Option<String>,
    pub side_effects_reported: Vec<String>,
    pub created_at: Timestamp,
}

/// Chronic care alert
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ChronicCareAlert {
    pub patient_hash: ActionHash,
    pub enrollment_hash: ActionHash,
    pub alert_type: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub trigger_value: Option<String>,
    pub threshold: Option<String>,
    pub recommended_action: Option<String>,
    pub acknowledged: bool,
    pub acknowledged_by: Option<ActionHash>,
    pub acknowledged_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

/// Exacerbation/flare record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ExacerbationEvent {
    pub patient_hash: ActionHash,
    pub enrollment_hash: ActionHash,
    pub condition: ChronicCondition,
    pub onset_date: Timestamp,
    pub symptoms: Vec<String>,
    pub severity: AlertSeverity,
    pub treatment_given: Vec<String>,
    pub hospitalization_required: bool,
    pub resolution_date: Option<Timestamp>,
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    ChronicDiseaseEnrollment(ChronicDiseaseEnrollment),
    ChronicCarePlan(ChronicCarePlan),
    PatientReportedOutcome(PatientReportedOutcome),
    DiabetesMetrics(DiabetesMetrics),
    HeartFailureMetrics(HeartFailureMetrics),
    COPDMetrics(COPDMetrics),
    MedicationAdherence(MedicationAdherence),
    ChronicCareAlert(ChronicCareAlert),
    ExacerbationEvent(ExacerbationEvent),
}

#[hdk_link_types]
pub enum LinkTypes {
    PatientToEnrollments,
    EnrollmentToCarePlans,
    EnrollmentToOutcomes,
    EnrollmentToAlerts,
    PatientToAdherence,
    EnrollmentToExacerbations,
    ConditionTypeToEnrollments,
}

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
        EntryTypes::ChronicCarePlan(plan) => validate_care_plan(&plan),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_care_plan(plan: &ChronicCarePlan) -> ExternResult<ValidateCallbackResult> {
    if plan.goals.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Care plan must have at least one goal".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
