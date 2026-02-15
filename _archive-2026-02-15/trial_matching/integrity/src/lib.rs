//! Trial Matching Integrity Zome
//!
//! Defines entry types for matching patients to clinical trials
//! based on eligibility criteria, preferences, and availability.

use hdi::prelude::*;

/// Type of eligibility criterion
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CriterionType {
    /// Must match for inclusion
    Inclusion,
    /// Must not match (exclusion)
    Exclusion,
}

/// Category of eligibility criterion
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CriterionCategory {
    Age,
    Gender,
    Diagnosis,
    Medication,
    LabValue,
    VitalSign,
    Procedure,
    Allergy,
    Comorbidity,
    Geographic,
    Language,
    InsuranceStatus,
    PriorTreatment,
    PerformanceStatus,
    Biomarker,
    Genetic,
    Pregnancy,
    Other,
}

/// Comparison operator for criteria
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Between,
    In,
    NotIn,
    Contains,
    NotContains,
    Exists,
    NotExists,
}

/// Status of a match
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MatchStatus {
    /// Patient matches all criteria
    Eligible,
    /// Patient may match, needs review
    PotentialMatch,
    /// Missing information to determine
    Indeterminate,
    /// Patient does not match
    Ineligible,
    /// Patient explicitly excluded
    Excluded,
}

/// Patient interest level
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InterestLevel {
    VeryInterested,
    SomewhatInterested,
    Neutral,
    NotInterested,
    DoNotContact,
}

/// Notification preference for trial opportunities
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum NotificationPreference {
    Immediate,
    Daily,
    Weekly,
    Monthly,
    Never,
}

/// A structured eligibility criterion
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct EligibilityCriterion {
    /// Unique ID within the trial
    pub criterion_id: String,
    /// Trial this criterion belongs to
    pub trial_hash: ActionHash,
    /// Inclusion or exclusion
    pub criterion_type: CriterionType,
    /// Category of criterion
    pub category: CriterionCategory,
    /// Human-readable description
    pub description: String,
    /// Field/attribute to check
    pub field: String,
    /// Comparison operator
    pub operator: ComparisonOperator,
    /// Value(s) to compare against
    pub value: String,
    /// Secondary value for range checks
    pub value_secondary: Option<String>,
    /// Unit of measurement if applicable
    pub unit: Option<String>,
    /// Time window for temporal criteria (days)
    pub time_window_days: Option<u32>,
    /// Whether criterion can be waived
    pub waivable: bool,
    /// Priority for matching (higher = more important)
    pub priority: u32,
}

/// Patient profile for matching
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MatchingProfile {
    /// Patient hash
    pub patient_hash: ActionHash,
    /// Age in years
    pub age: u32,
    /// Gender
    pub gender: String,
    /// Active diagnoses (ICD-10 codes)
    pub diagnoses: Vec<String>,
    /// Current medications (RxNorm codes)
    pub medications: Vec<String>,
    /// Known allergies
    pub allergies: Vec<String>,
    /// Recent lab values (LOINC code -> value)
    pub lab_values: Vec<(String, String)>,
    /// Recent vital signs
    pub vital_signs: Vec<(String, String)>,
    /// Prior procedures (CPT codes)
    pub procedures: Vec<String>,
    /// Geographic location (state/region)
    pub location: Option<String>,
    /// Languages spoken
    pub languages: Vec<String>,
    /// Insurance status
    pub has_insurance: bool,
    /// Performance status (ECOG 0-5)
    pub performance_status: Option<u32>,
    /// Biomarker results
    pub biomarkers: Vec<(String, String)>,
    /// Last updated
    pub updated_at: Timestamp,
}

/// Patient preferences for trial participation
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PatientPreferences {
    /// Patient hash
    pub patient_hash: ActionHash,
    /// Overall interest in trials
    pub trial_interest: InterestLevel,
    /// Preferred notification frequency
    pub notification_preference: NotificationPreference,
    /// Willing to travel (miles)
    pub max_travel_distance: Option<u32>,
    /// Willing to take time off work
    pub flexible_schedule: bool,
    /// Conditions of interest (ICD-10 codes)
    pub conditions_of_interest: Vec<String>,
    /// Conditions to exclude
    pub excluded_conditions: Vec<String>,
    /// Preferred trial phases
    pub preferred_phases: Vec<String>,
    /// Acceptable visit frequency (per month)
    pub max_visits_per_month: Option<u32>,
    /// Willing to receive placebo
    pub accepts_placebo: bool,
    /// Preferred languages for materials
    pub preferred_languages: Vec<String>,
    /// Accessibility needs
    pub accessibility_needs: Vec<String>,
    /// Caregiver availability
    pub has_caregiver_support: bool,
    /// Updated timestamp
    pub updated_at: Timestamp,
}

/// Result of matching a patient to a trial
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MatchResult {
    /// Unique match ID
    pub match_id: String,
    /// Patient hash
    pub patient_hash: ActionHash,
    /// Trial hash
    pub trial_hash: ActionHash,
    /// Overall match status
    pub status: MatchStatus,
    /// Match score (0-100)
    pub score: u32,
    /// Criteria met
    pub criteria_met: Vec<String>,
    /// Criteria not met
    pub criteria_not_met: Vec<String>,
    /// Criteria with missing data
    pub criteria_indeterminate: Vec<String>,
    /// Whether patient has been notified
    pub patient_notified: bool,
    /// Patient's response if any
    pub patient_interest: Option<InterestLevel>,
    /// Provider review status
    pub provider_reviewed: bool,
    /// Provider's recommendation
    pub provider_recommendation: Option<String>,
    /// Match timestamp
    pub matched_at: Timestamp,
    /// Last updated
    pub updated_at: Timestamp,
}

/// Notification sent to patient about trial opportunity
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TrialNotification {
    /// Notification ID
    pub notification_id: String,
    /// Match this notification is for
    pub match_hash: ActionHash,
    /// Patient hash
    pub patient_hash: ActionHash,
    /// Trial hash
    pub trial_hash: ActionHash,
    /// Notification method used
    pub method: String,
    /// Notification sent timestamp
    pub sent_at: Timestamp,
    /// Whether notification was read
    pub read_at: Option<Timestamp>,
    /// Patient's response
    pub response: Option<InterestLevel>,
    /// Response timestamp
    pub responded_at: Option<Timestamp>,
}

/// Entry types for the trial matching zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    EligibilityCriterion(EligibilityCriterion),
    MatchingProfile(MatchingProfile),
    PatientPreferences(PatientPreferences),
    MatchResult(MatchResult),
    TrialNotification(TrialNotification),
}

/// Link types for the trial matching zome
#[hdk_link_types]
pub enum LinkTypes {
    /// Trial to its criteria
    TrialToCriteria,
    /// Patient to their matching profile
    PatientToProfile,
    /// Patient to their preferences
    PatientToPreferences,
    /// Patient to their match results
    PatientToMatches,
    /// Trial to its match results
    TrialToMatches,
    /// Match to notifications
    MatchToNotifications,
    /// Patient to notifications
    PatientToNotifications,
    /// Index of eligible matches by trial
    EligibleMatchesByTrial,
}

/// Validation callbacks
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::EligibilityCriterion(criterion) => validate_criterion(&criterion),
                EntryTypes::MatchingProfile(profile) => validate_profile(&profile),
                EntryTypes::MatchResult(result) => validate_match_result(&result),
                _ => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_criterion(criterion: &EligibilityCriterion) -> ExternResult<ValidateCallbackResult> {
    if criterion.criterion_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Criterion ID is required".to_string()));
    }
    if criterion.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Criterion description is required".to_string()));
    }
    if criterion.field.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Criterion field is required".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_profile(profile: &MatchingProfile) -> ExternResult<ValidateCallbackResult> {
    if profile.age > 150 {
        return Ok(ValidateCallbackResult::Invalid("Invalid age".to_string()));
    }
    if profile.gender.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Gender is required".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_match_result(result: &MatchResult) -> ExternResult<ValidateCallbackResult> {
    if result.match_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Match ID is required".to_string()));
    }
    if result.score > 100 {
        return Ok(ValidateCallbackResult::Invalid("Score must be 0-100".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}
