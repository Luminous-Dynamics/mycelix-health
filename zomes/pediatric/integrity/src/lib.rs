//! Pediatric Integrity Zome
//!
//! Pediatric lifecycle management including growth tracking, immunizations,
//! developmental milestones, and adolescent health.

use hdi::prelude::*;

/// Age groups for pediatric care
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PediatricAgeGroup {
    Newborn,       // 0-28 days
    Infant,        // 1-12 months
    Toddler,       // 1-3 years
    Preschool,     // 3-5 years
    SchoolAge,     // 5-12 years
    Adolescent,    // 12-18 years
    YoungAdult,    // 18-21 years (transitional care)
}

/// Growth percentile categories
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PercentileCategory {
    BelowFifth,      // <5th percentile - underweight/short
    FifthToTenth,    // 5th-10th
    TenthToTwentyFifth,
    TwentyFifthToFiftieth,
    FiftiethToSeventyFifth,
    SeventyFifthToNinetieth,
    NinetiethToNinetyFifth,
    AboveNinetyFifth, // >95th percentile - overweight/tall
}

/// Developmental domains
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DevelopmentalDomain {
    GrossMotor,
    FineMotor,
    Language,
    Cognitive,
    SocialEmotional,
    AdaptiveBehavior,
}

/// Milestone status
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MilestoneStatus {
    NotYetExpected,
    Expected,
    Achieved,
    Delayed,
    NeedsEvaluation,
}

/// Immunization status
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ImmunizationStatus {
    UpToDate,
    Due,
    Overdue,
    Incomplete,
    Exempt(String), // reason for exemption
    Contraindicated(String), // medical reason
}

/// Vaccine types (CDC recommended schedule)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum VaccineType {
    HepB,           // Hepatitis B
    RV,             // Rotavirus
    DTaP,           // Diphtheria, Tetanus, Pertussis
    Hib,            // Haemophilus influenzae type b
    PCV,            // Pneumococcal conjugate
    IPV,            // Inactivated Poliovirus
    Influenza,      // Flu
    MMR,            // Measles, Mumps, Rubella
    Varicella,      // Chickenpox
    HepA,           // Hepatitis A
    MenACWY,        // Meningococcal
    HPV,            // Human Papillomavirus
    Tdap,           // Tetanus, Diphtheria, Pertussis (adolescent)
    MenB,           // Meningococcal B
    COVID19,        // COVID-19
    Other(String),
}

/// Screening types for pediatric visits
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PediatricScreening {
    NewbornMetabolic,
    NewbornHearing,
    VisionScreening,
    HearingScreening,
    LeadScreening,
    AnemiaScreening,
    TuberculosisRisk,
    DevelopmentalSurveillance,
    AutismSpecific(String), // M-CHAT, etc.
    BehavioralHealth,
    DepressionScreening, // PHQ-A for adolescents
    SubstanceUse,        // CRAFFT
    SexualHealth,
    EatingDisorders,
    SportsPhysical,
    Other(String),
}

/// Well-child visit schedule
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WellChildVisit {
    Newborn,           // 3-5 days
    OneMonth,
    TwoMonths,
    FourMonths,
    SixMonths,
    NineMonths,
    TwelveMonths,
    FifteenMonths,
    EighteenMonths,
    TwentyFourMonths,
    ThirtyMonths,
    ThreeYears,
    Annual,            // 4+ years
}

/// Growth measurement record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GrowthMeasurement {
    pub patient_hash: ActionHash,
    pub measurement_date: Timestamp,
    pub age_months: u32,
    pub weight_kg: f64,
    pub weight_percentile: Option<f64>,
    pub height_cm: f64,
    pub height_percentile: Option<f64>,
    pub head_circumference_cm: Option<f64>, // for children <3 years
    pub head_percentile: Option<f64>,
    pub bmi: Option<f64>,
    pub bmi_percentile: Option<f64>,
    pub measured_by: ActionHash,
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

/// Immunization record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ImmunizationRecord {
    pub patient_hash: ActionHash,
    pub vaccine_type: VaccineType,
    pub vaccine_name: String,
    pub cvx_code: Option<String>,           // CDC vaccine code
    pub manufacturer: Option<String>,
    pub lot_number: Option<String>,
    pub dose_number: u8,                     // 1st, 2nd, 3rd, etc.
    pub doses_in_series: Option<u8>,
    pub administration_date: Timestamp,
    pub expiration_date: Option<Timestamp>,
    pub site: Option<String>,                // injection site
    pub route: Option<String>,               // IM, SC, oral, etc.
    pub administered_by: ActionHash,
    pub location: Option<String>,
    pub adverse_reaction: Option<String>,
    pub vis_given: bool,                     // Vaccine Information Statement provided
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

/// Developmental milestone assessment
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DevelopmentalMilestone {
    pub patient_hash: ActionHash,
    pub assessment_date: Timestamp,
    pub age_months: u32,
    pub domain: DevelopmentalDomain,
    pub milestone_description: String,
    pub expected_age_months: u32,
    pub status: MilestoneStatus,
    pub assessment_tool: Option<String>,     // ASQ, PEDS, Denver, etc.
    pub assessed_by: ActionHash,
    pub concerns_noted: Option<String>,
    pub recommendations: Option<String>,
    pub referral_needed: bool,
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

/// Developmental screening result
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DevelopmentalScreening {
    pub patient_hash: ActionHash,
    pub screening_date: Timestamp,
    pub age_months: u32,
    pub tool_name: String,                   // ASQ-3, M-CHAT-R/F, PEDS, etc.
    pub tool_version: Option<String>,
    pub domain_scores: Vec<DomainScore>,
    pub overall_result: String,              // Pass, Fail, Borderline
    pub concerns_identified: Vec<String>,
    pub referrals_made: Vec<String>,
    pub follow_up_needed: bool,
    pub follow_up_date: Option<Timestamp>,
    pub administered_by: ActionHash,
    pub parent_concerns: Option<String>,
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DomainScore {
    pub domain: DevelopmentalDomain,
    pub raw_score: f64,
    pub percentile: Option<f64>,
    pub result: String,
}

/// Well-child visit record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct WellChildVisitRecord {
    pub patient_hash: ActionHash,
    pub visit_date: Timestamp,
    pub visit_type: WellChildVisit,
    pub age_at_visit: String,                // e.g., "15 months"
    pub growth_hash: Option<ActionHash>,     // Link to growth measurement
    pub immunizations_given: Vec<ActionHash>, // Links to immunization records
    pub screenings_performed: Vec<PediatricScreening>,
    pub developmental_concerns: Option<String>,
    pub nutrition_assessment: Option<String>,
    pub sleep_assessment: Option<String>,
    pub safety_counseling: Vec<String>,      // Topics covered
    pub anticipatory_guidance: Vec<String>,
    pub physical_exam_findings: Option<String>,
    pub provider_hash: ActionHash,
    pub next_visit_due: Option<Timestamp>,
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

/// Pediatric-specific condition tracking
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PediatricCondition {
    pub patient_hash: ActionHash,
    pub condition_name: String,
    pub icd10_code: Option<String>,
    pub onset_date: Option<Timestamp>,
    pub onset_age: Option<String>,
    pub condition_type: PediatricConditionType,
    pub severity: Option<String>,
    pub current_status: String,              // Active, Resolved, Chronic
    pub treatment_plan: Option<String>,
    pub specialist_involved: Option<ActionHash>,
    pub school_impact: Option<String>,       // IEP, 504 plan, accommodations
    pub follow_up_schedule: Option<String>,
    pub notes: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PediatricConditionType {
    Developmental,
    Behavioral,
    Chronic,
    Genetic,
    Congenital,
    Infectious,
    Allergic,
    Nutritional,
    Other(String),
}

/// School health record (for school collaboration)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SchoolHealthRecord {
    pub patient_hash: ActionHash,
    pub school_year: String,
    pub grade: String,
    pub school_name: Option<String>,
    pub immunization_status: ImmunizationStatus,
    pub vision_screening_date: Option<Timestamp>,
    pub vision_result: Option<String>,
    pub hearing_screening_date: Option<Timestamp>,
    pub hearing_result: Option<String>,
    pub physical_exam_date: Option<Timestamp>,
    pub allergies: Vec<String>,
    pub medications_at_school: Vec<SchoolMedication>,
    pub health_conditions: Vec<String>,
    pub care_plan: Option<String>,           // 504, IEP, health care plan
    pub emergency_action_plan: Option<String>,
    pub pe_restrictions: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SchoolMedication {
    pub medication_name: String,
    pub dosage: String,
    pub frequency: String,
    pub administration_time: String,
    pub prescriber: String,
    pub start_date: Timestamp,
    pub end_date: Option<Timestamp>,
}

/// Adolescent-specific health record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AdolescentHealthRecord {
    pub patient_hash: ActionHash,
    pub visit_date: Timestamp,
    pub headss_assessment: Option<HeadsAssessment>, // Psychosocial assessment
    pub menstrual_history: Option<MenstrualHistory>, // If applicable
    pub sexual_health: Option<SexualHealthAssessment>,
    pub substance_screening: Option<SubstanceScreening>,
    pub depression_screening: Option<DepressionScreening>,
    pub eating_concerns: Option<String>,
    pub sports_clearance: Option<bool>,
    pub transition_planning: Option<String>, // Transition to adult care
    pub confidential_time: bool,             // Time without parent present
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadsAssessment {
    pub home: Option<String>,
    pub education_employment: Option<String>,
    pub activities: Option<String>,
    pub drugs_substances: Option<String>,
    pub sexuality: Option<String>,
    pub suicide_depression: Option<String>,
    pub safety: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MenstrualHistory {
    pub menarche_age: Option<u8>,
    pub last_period: Option<Timestamp>,
    pub cycle_length_days: Option<u8>,
    pub duration_days: Option<u8>,
    pub flow: Option<String>,
    pub dysmenorrhea: bool,
    pub concerns: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SexualHealthAssessment {
    pub sexually_active: Option<bool>,
    pub contraception_use: Option<String>,
    pub sti_screening_date: Option<Timestamp>,
    pub education_provided: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubstanceScreening {
    pub tool_used: String,                   // CRAFFT
    pub score: u8,
    pub result: String,
    pub substances_reported: Vec<String>,
    pub intervention_needed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DepressionScreening {
    pub tool_used: String,                   // PHQ-A
    pub score: u8,
    pub severity: String,
    pub suicide_risk_assessed: bool,
    pub safety_plan_needed: bool,
}

/// Newborn record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct NewbornRecord {
    pub patient_hash: ActionHash,
    pub birth_date: Timestamp,
    pub gestational_age_weeks: u8,
    pub gestational_age_days: u8,
    pub birth_weight_grams: u32,
    pub birth_length_cm: f64,
    pub head_circumference_cm: f64,
    pub apgar_1min: Option<u8>,
    pub apgar_5min: Option<u8>,
    pub delivery_type: String,               // Vaginal, C-section
    pub complications: Vec<String>,
    pub nicu_stay: bool,
    pub nicu_days: Option<u32>,
    pub newborn_screening_completed: bool,
    pub newborn_screening_date: Option<Timestamp>,
    pub hearing_screen_result: Option<String>,
    pub cchd_screen_result: Option<String>,  // Critical congenital heart disease
    pub bilirubin_check: Option<f64>,
    pub feeding_type: String,                // Breast, formula, mixed
    pub discharge_date: Option<Timestamp>,
    pub notes: Option<String>,
    pub created_at: Timestamp,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    GrowthMeasurement(GrowthMeasurement),
    ImmunizationRecord(ImmunizationRecord),
    DevelopmentalMilestone(DevelopmentalMilestone),
    DevelopmentalScreening(DevelopmentalScreening),
    WellChildVisitRecord(WellChildVisitRecord),
    PediatricCondition(PediatricCondition),
    SchoolHealthRecord(SchoolHealthRecord),
    AdolescentHealthRecord(AdolescentHealthRecord),
    NewbornRecord(NewbornRecord),
}

#[hdk_link_types]
pub enum LinkTypes {
    PatientToGrowth,
    PatientToImmunizations,
    PatientToMilestones,
    PatientToScreenings,
    PatientToWellChildVisits,
    PatientToConditions,
    PatientToSchoolRecords,
    PatientToAdolescentRecords,
    PatientToNewbornRecord,
    VaccineTypeToRecords,
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
        EntryTypes::GrowthMeasurement(measurement) => validate_growth_measurement(&measurement),
        EntryTypes::ImmunizationRecord(record) => validate_immunization(&record),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_growth_measurement(measurement: &GrowthMeasurement) -> ExternResult<ValidateCallbackResult> {
    // Weight should be reasonable (0.3 - 150 kg for pediatrics)
    if measurement.weight_kg < 0.3 || measurement.weight_kg > 150.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Weight must be between 0.3 and 150 kg".to_string(),
        ));
    }

    // Height should be reasonable (30 - 220 cm)
    if measurement.height_cm < 30.0 || measurement.height_cm > 220.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Height must be between 30 and 220 cm".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_immunization(record: &ImmunizationRecord) -> ExternResult<ValidateCallbackResult> {
    // Dose number should be at least 1
    if record.dose_number < 1 {
        return Ok(ValidateCallbackResult::Invalid(
            "Dose number must be at least 1".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}
