//! Trial Matching Coordinator Zome
//!
//! Provides extern functions for matching patients to clinical trials
//! based on eligibility criteria and patient preferences.

use hdk::prelude::*;
use trial_matching_integrity::*;

// ==================== ELIGIBILITY CRITERIA ====================

/// Create an eligibility criterion for a trial
#[hdk_extern]
pub fn create_criterion(criterion: EligibilityCriterion) -> ExternResult<Record> {
    let criterion_hash = create_entry(&EntryTypes::EligibilityCriterion(criterion.clone()))?;
    let record = get(criterion_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find criterion".to_string())))?;

    // Link to trial
    create_link(
        criterion.trial_hash,
        criterion_hash,
        LinkTypes::TrialToCriteria,
        (),
    )?;

    Ok(record)
}

/// Get all criteria for a trial
#[hdk_extern]
pub fn get_trial_criteria(trial_hash: ActionHash) -> ExternResult<Vec<Record>> {
    get_linked_records(trial_hash, LinkTypes::TrialToCriteria)
}

/// Bulk create criteria for a trial
#[hdk_extern]
pub fn create_trial_criteria(input: BulkCriteriaInput) -> ExternResult<Vec<ActionHash>> {
    let mut hashes = Vec::new();
    for criterion in input.criteria {
        let hash = create_entry(&EntryTypes::EligibilityCriterion(criterion.clone()))?;
        create_link(
            input.trial_hash.clone(),
            hash.clone(),
            LinkTypes::TrialToCriteria,
            (),
        )?;
        hashes.push(hash);
    }
    Ok(hashes)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BulkCriteriaInput {
    pub trial_hash: ActionHash,
    pub criteria: Vec<EligibilityCriterion>,
}

// ==================== PATIENT PROFILES ====================

/// Create or update patient matching profile
#[hdk_extern]
pub fn upsert_matching_profile(profile: MatchingProfile) -> ExternResult<Record> {
    let profile_hash = create_entry(&EntryTypes::MatchingProfile(profile.clone()))?;
    let record = get(profile_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find profile".to_string())))?;

    // Link to patient
    create_link(
        profile.patient_hash,
        profile_hash,
        LinkTypes::PatientToProfile,
        (),
    )?;

    Ok(record)
}

/// Get patient's matching profile
#[hdk_extern]
pub fn get_patient_profile(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    let records = get_linked_records(patient_hash, LinkTypes::PatientToProfile)?;
    // Return most recent
    Ok(records.into_iter().last())
}

/// Update patient preferences
#[hdk_extern]
pub fn update_preferences(preferences: PatientPreferences) -> ExternResult<Record> {
    let pref_hash = create_entry(&EntryTypes::PatientPreferences(preferences.clone()))?;
    let record = get(pref_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find preferences".to_string())))?;

    // Link to patient
    create_link(
        preferences.patient_hash,
        pref_hash,
        LinkTypes::PatientToPreferences,
        (),
    )?;

    Ok(record)
}

/// Get patient's preferences
#[hdk_extern]
pub fn get_patient_preferences(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    let records = get_linked_records(patient_hash, LinkTypes::PatientToPreferences)?;
    Ok(records.into_iter().last())
}

// ==================== MATCHING ====================

/// Match a patient against a specific trial
#[hdk_extern]
pub fn match_patient_to_trial(input: MatchInput) -> ExternResult<Record> {
    // Get patient profile
    let profile_record = get_patient_profile(input.patient_hash.clone())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Patient profile not found".to_string())))?;

    let profile: MatchingProfile = profile_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid profile".to_string())))?;

    // Get trial criteria
    let criteria_records = get_trial_criteria(input.trial_hash.clone())?;
    let criteria: Vec<EligibilityCriterion> = criteria_records
        .iter()
        .filter_map(|r| r.entry().to_app_option::<EligibilityCriterion>().ok().flatten())
        .collect();

    // Evaluate each criterion
    let mut criteria_met = Vec::new();
    let mut criteria_not_met = Vec::new();
    let mut criteria_indeterminate = Vec::new();

    for criterion in &criteria {
        match evaluate_criterion(&profile, criterion) {
            CriterionResult::Met => criteria_met.push(criterion.criterion_id.clone()),
            CriterionResult::NotMet => criteria_not_met.push(criterion.criterion_id.clone()),
            CriterionResult::Indeterminate => criteria_indeterminate.push(criterion.criterion_id.clone()),
        }
    }

    // Calculate status and score
    let (status, score) = calculate_match_status(&criteria_met, &criteria_not_met, &criteria_indeterminate, &criteria);

    let result = MatchResult {
        match_id: format!("MATCH-{}-{}",
            input.patient_hash.to_string().chars().take(8).collect::<String>(),
            sys_time()?.as_micros()
        ),
        patient_hash: input.patient_hash.clone(),
        trial_hash: input.trial_hash.clone(),
        status,
        score,
        criteria_met,
        criteria_not_met,
        criteria_indeterminate,
        patient_notified: false,
        patient_interest: None,
        provider_reviewed: false,
        provider_recommendation: None,
        matched_at: sys_time()?,
        updated_at: sys_time()?,
    };

    let result_hash = create_entry(&EntryTypes::MatchResult(result.clone()))?;
    let record = get(result_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find match result".to_string())))?;

    // Link to patient
    create_link(
        input.patient_hash,
        result_hash.clone(),
        LinkTypes::PatientToMatches,
        (),
    )?;

    // Link to trial
    create_link(
        input.trial_hash.clone(),
        result_hash.clone(),
        LinkTypes::TrialToMatches,
        (),
    )?;

    // If eligible, add to eligible matches index
    if result.status == MatchStatus::Eligible {
        let eligible_anchor = anchor_hash(&format!("eligible_{}", input.trial_hash))?;
        create_link(eligible_anchor, result_hash, LinkTypes::EligibleMatchesByTrial, ())?;
    }

    Ok(record)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MatchInput {
    pub patient_hash: ActionHash,
    pub trial_hash: ActionHash,
}

enum CriterionResult {
    Met,
    NotMet,
    Indeterminate,
}

fn evaluate_criterion(profile: &MatchingProfile, criterion: &EligibilityCriterion) -> CriterionResult {
    let result = match criterion.category {
        CriterionCategory::Age => {
            evaluate_numeric_criterion(profile.age as f64, criterion)
        }
        CriterionCategory::Gender => {
            evaluate_string_criterion(&profile.gender, criterion)
        }
        CriterionCategory::Diagnosis => {
            evaluate_list_criterion(&profile.diagnoses, criterion)
        }
        CriterionCategory::Medication => {
            evaluate_list_criterion(&profile.medications, criterion)
        }
        CriterionCategory::Allergy => {
            evaluate_list_criterion(&profile.allergies, criterion)
        }
        CriterionCategory::LabValue => {
            if let Some((_, value)) = profile.lab_values.iter().find(|(code, _)| code == &criterion.field) {
                if let Ok(v) = value.parse::<f64>() {
                    evaluate_numeric_criterion(v, criterion)
                } else {
                    CriterionResult::Indeterminate
                }
            } else {
                CriterionResult::Indeterminate
            }
        }
        CriterionCategory::PerformanceStatus => {
            if let Some(ps) = profile.performance_status {
                evaluate_numeric_criterion(ps as f64, criterion)
            } else {
                CriterionResult::Indeterminate
            }
        }
        CriterionCategory::Geographic => {
            if let Some(ref loc) = profile.location {
                evaluate_string_criterion(loc, criterion)
            } else {
                CriterionResult::Indeterminate
            }
        }
        _ => CriterionResult::Indeterminate,
    };

    // Flip result for exclusion criteria
    if criterion.criterion_type == CriterionType::Exclusion {
        match result {
            CriterionResult::Met => CriterionResult::NotMet,
            CriterionResult::NotMet => CriterionResult::Met,
            CriterionResult::Indeterminate => CriterionResult::Indeterminate,
        }
    } else {
        result
    }
}

fn evaluate_numeric_criterion(value: f64, criterion: &EligibilityCriterion) -> CriterionResult {
    let target: f64 = match criterion.value.parse() {
        Ok(v) => v,
        Err(_) => return CriterionResult::Indeterminate,
    };

    let result = match criterion.operator {
        ComparisonOperator::Equals => (value - target).abs() < 0.001,
        ComparisonOperator::NotEquals => (value - target).abs() >= 0.001,
        ComparisonOperator::GreaterThan => value > target,
        ComparisonOperator::GreaterThanOrEqual => value >= target,
        ComparisonOperator::LessThan => value < target,
        ComparisonOperator::LessThanOrEqual => value <= target,
        ComparisonOperator::Between => {
            if let Some(ref upper) = criterion.value_secondary {
                if let Ok(upper_val) = upper.parse::<f64>() {
                    value >= target && value <= upper_val
                } else {
                    return CriterionResult::Indeterminate;
                }
            } else {
                return CriterionResult::Indeterminate;
            }
        }
        _ => return CriterionResult::Indeterminate,
    };

    if result { CriterionResult::Met } else { CriterionResult::NotMet }
}

fn evaluate_string_criterion(value: &str, criterion: &EligibilityCriterion) -> CriterionResult {
    let result = match criterion.operator {
        ComparisonOperator::Equals => value.eq_ignore_ascii_case(&criterion.value),
        ComparisonOperator::NotEquals => !value.eq_ignore_ascii_case(&criterion.value),
        ComparisonOperator::Contains => value.to_lowercase().contains(&criterion.value.to_lowercase()),
        ComparisonOperator::NotContains => !value.to_lowercase().contains(&criterion.value.to_lowercase()),
        ComparisonOperator::In => {
            criterion.value.split(',').any(|v| v.trim().eq_ignore_ascii_case(value))
        }
        ComparisonOperator::NotIn => {
            !criterion.value.split(',').any(|v| v.trim().eq_ignore_ascii_case(value))
        }
        _ => return CriterionResult::Indeterminate,
    };

    if result { CriterionResult::Met } else { CriterionResult::NotMet }
}

fn evaluate_list_criterion(values: &[String], criterion: &EligibilityCriterion) -> CriterionResult {
    let target_values: Vec<&str> = criterion.value.split(',').map(|s| s.trim()).collect();

    let result = match criterion.operator {
        ComparisonOperator::Contains | ComparisonOperator::In => {
            target_values.iter().any(|t| values.iter().any(|v| v.eq_ignore_ascii_case(t)))
        }
        ComparisonOperator::NotContains | ComparisonOperator::NotIn => {
            !target_values.iter().any(|t| values.iter().any(|v| v.eq_ignore_ascii_case(t)))
        }
        ComparisonOperator::Exists => !values.is_empty(),
        ComparisonOperator::NotExists => values.is_empty(),
        _ => return CriterionResult::Indeterminate,
    };

    if result { CriterionResult::Met } else { CriterionResult::NotMet }
}

fn calculate_match_status(
    met: &[String],
    not_met: &[String],
    indeterminate: &[String],
    all_criteria: &[EligibilityCriterion],
) -> (MatchStatus, u32) {
    // Check for any exclusion criteria that were not met (meaning they matched and patient is excluded)
    let excluded = all_criteria.iter().any(|c| {
        c.criterion_type == CriterionType::Exclusion && not_met.contains(&c.criterion_id)
    });

    if excluded {
        return (MatchStatus::Excluded, 0);
    }

    // Check for unmet inclusion criteria
    let unmet_required = all_criteria.iter().any(|c| {
        c.criterion_type == CriterionType::Inclusion && !c.waivable && not_met.contains(&c.criterion_id)
    });

    if unmet_required {
        return (MatchStatus::Ineligible, calculate_score(met.len(), not_met.len(), indeterminate.len()));
    }

    // Check for indeterminate criteria
    if !indeterminate.is_empty() {
        return (MatchStatus::Indeterminate, calculate_score(met.len(), not_met.len(), indeterminate.len()));
    }

    // Check for waivable criteria not met
    if !not_met.is_empty() {
        return (MatchStatus::PotentialMatch, calculate_score(met.len(), not_met.len(), indeterminate.len()));
    }

    (MatchStatus::Eligible, 100)
}

fn calculate_score(met: usize, not_met: usize, indeterminate: usize) -> u32 {
    let total = met + not_met + indeterminate;
    if total == 0 {
        return 0;
    }
    ((met as f64 / total as f64) * 100.0) as u32
}

/// Find matching trials for a patient
#[hdk_extern]
pub fn find_trials_for_patient(input: FindTrialsInput) -> ExternResult<Vec<Record>> {
    // This would typically query available trials and match against each
    // For now, return matches from patient's existing matches
    get_patient_matches(input.patient_hash)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FindTrialsInput {
    pub patient_hash: ActionHash,
    pub min_score: Option<u32>,
}

/// Get patient's match results
#[hdk_extern]
pub fn get_patient_matches(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    get_linked_records(patient_hash, LinkTypes::PatientToMatches)
}

/// Get eligible matches for a trial
#[hdk_extern]
pub fn get_eligible_matches(trial_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash(&format!("eligible_{}", trial_hash))?;
    get_linked_records(anchor, LinkTypes::EligibleMatchesByTrial)
}

// ==================== NOTIFICATIONS ====================

/// Send notification to patient about trial opportunity
#[hdk_extern]
pub fn send_trial_notification(notification: TrialNotification) -> ExternResult<Record> {
    let notif_hash = create_entry(&EntryTypes::TrialNotification(notification.clone()))?;
    let record = get(notif_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find notification".to_string())))?;

    // Link to match
    create_link(
        notification.match_hash,
        notif_hash.clone(),
        LinkTypes::MatchToNotifications,
        (),
    )?;

    // Link to patient
    create_link(
        notification.patient_hash,
        notif_hash,
        LinkTypes::PatientToNotifications,
        (),
    )?;

    Ok(record)
}

/// Record patient response to notification
#[hdk_extern]
pub fn record_notification_response(input: NotificationResponseInput) -> ExternResult<Record> {
    let record = get(input.notification_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Notification not found".to_string())))?;

    let mut notification: TrialNotification = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid notification".to_string())))?;

    notification.response = Some(input.interest);
    notification.responded_at = Some(sys_time()?);

    let updated_hash = update_entry(input.notification_hash, &notification)?;
    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated notification".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NotificationResponseInput {
    pub notification_hash: ActionHash,
    pub interest: InterestLevel,
}

/// Get patient's notifications
#[hdk_extern]
pub fn get_patient_notifications(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    get_linked_records(patient_hash, LinkTypes::PatientToNotifications)
}

// ==================== HELPER FUNCTIONS ====================

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}

fn get_linked_records(base: impl Into<AnyLinkableHash>, link_type: LinkTypes) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(base.into(), link_type)?,
        GetStrategy::default()
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
