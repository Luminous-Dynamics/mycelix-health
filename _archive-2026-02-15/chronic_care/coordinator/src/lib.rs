//! Chronic Care Coordinator Zome
//!
//! Chronic disease management functions for diabetes, heart failure, COPD, CKD, and more.

use hdk::prelude::*;
use chronic_care_integrity::*;
use mycelix_health_shared::{require_authorization, log_data_access, DataCategory, Permission};

/// Create an anchor hash for linking using Path
fn anchor_hash(anchor: &str) -> ExternResult<EntryHash> {
    let path = Path::from(anchor);
    path.path_entry_hash()
}

/// Enroll a patient in a chronic disease management program
#[hdk_extern]
pub fn enroll_patient(enrollment: ChronicDiseaseEnrollment) -> ExternResult<ActionHash> {
    let auth = require_authorization(
        enrollment.patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Write,
        false,
    )?;
    let action_hash = create_entry(&EntryTypes::ChronicDiseaseEnrollment(enrollment.clone()))?;

    // Link from patient to enrollment
    create_link(
        enrollment.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToEnrollments,
        (),
    )?;

    // Link from condition type to enrollment for easy filtering
    let condition_tag = get_condition_tag(&enrollment.condition);
    let condition_anchor = anchor_hash(&condition_tag)?;
    create_link(
        condition_anchor,
        action_hash.clone(),
        LinkTypes::ConditionTypeToEnrollments,
        (),
    )?;

    log_data_access(
        enrollment.patient_hash,
        vec![DataCategory::Diagnoses],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(action_hash)
}

fn get_condition_tag(condition: &ChronicCondition) -> String {
    match condition {
        ChronicCondition::Diabetes(_) => "diabetes".to_string(),
        ChronicCondition::HeartFailure(_) => "heart_failure".to_string(),
        ChronicCondition::COPD(_) => "copd".to_string(),
        ChronicCondition::ChronicKidneyDisease(_) => "ckd".to_string(),
        ChronicCondition::Hypertension => "hypertension".to_string(),
        ChronicCondition::Asthma => "asthma".to_string(),
        ChronicCondition::CancerSurvivorship(_) => "cancer_survivorship".to_string(),
        ChronicCondition::MultipleSclerosis => "multiple_sclerosis".to_string(),
        ChronicCondition::RheumatoidArthritis => "rheumatoid_arthritis".to_string(),
        ChronicCondition::Obesity => "obesity".to_string(),
        ChronicCondition::Other(_) => "other".to_string(),
    }
}

/// Get all enrollments for a patient
#[hdk_extern]
pub fn get_patient_enrollments(patient_hash: ActionHash) -> ExternResult<Vec<ChronicDiseaseEnrollment>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Read,
        false,
    )?;
    let patient_hash_for_links = patient_hash.clone();
    let links = get_links(
        LinkQuery::try_new(patient_hash_for_links, LinkTypes::PatientToEnrollments)?, GetStrategy::default(),
    )?;

    let mut enrollments = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(enrollment) = record.entry().to_app_option::<ChronicDiseaseEnrollment>().ok().flatten() {
                    enrollments.push(enrollment);
                }
            }
        }
    }

    if !enrollments.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::Diagnoses],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(enrollments)
}

/// Create a care plan for a chronic condition enrollment
#[hdk_extern]
pub fn create_care_plan(plan: ChronicCarePlan) -> ExternResult<ActionHash> {
    let auth = require_authorization(
        plan.patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Write,
        false,
    )?;
    let action_hash = create_entry(&EntryTypes::ChronicCarePlan(plan.clone()))?;

    // Link from enrollment to care plan
    create_link(
        plan.enrollment_hash.clone(),
        action_hash.clone(),
        LinkTypes::EnrollmentToCarePlans,
        (),
    )?;

    log_data_access(
        plan.patient_hash,
        vec![DataCategory::Diagnoses],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(action_hash)
}

/// Update an existing care plan
#[hdk_extern]
pub fn update_care_plan(input: UpdateCarePlanInput) -> ExternResult<ActionHash> {
    let auth = require_authorization(
        input.updated_plan.patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Amend,
        false,
    )?;

    let updated_hash = update_entry(input.original_action_hash, &input.updated_plan)?;

    log_data_access(
        input.updated_plan.patient_hash,
        vec![DataCategory::Diagnoses],
        Permission::Amend,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(updated_hash)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateCarePlanInput {
    pub original_action_hash: ActionHash,
    pub updated_plan: ChronicCarePlan,
}

/// Get care plans for an enrollment
#[hdk_extern]
pub fn get_care_plans(enrollment_hash: ActionHash) -> ExternResult<Vec<ChronicCarePlan>> {
    let enrollment_record = get(enrollment_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Enrollment not found".to_string())))?;

    let enrollment: ChronicDiseaseEnrollment = enrollment_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid enrollment".to_string())))?;

    let auth = require_authorization(
        enrollment.patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(enrollment_hash, LinkTypes::EnrollmentToCarePlans)?, GetStrategy::default(),
    )?;

    let mut plans = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(plan) = record.entry().to_app_option::<ChronicCarePlan>().ok().flatten() {
                    plans.push(plan);
                }
            }
        }
    }

    if !plans.is_empty() {
        log_data_access(
            enrollment.patient_hash,
            vec![DataCategory::Diagnoses],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(plans)
}

/// Record a patient-reported outcome
#[hdk_extern]
pub fn record_outcome(outcome: PatientReportedOutcome) -> ExternResult<ActionHash> {
    let auth = require_authorization(
        outcome.patient_hash.clone(),
        DataCategory::VitalSigns,
        Permission::Write,
        false,
    )?;
    let action_hash = create_entry(&EntryTypes::PatientReportedOutcome(outcome.clone()))?;

    create_link(
        outcome.enrollment_hash.clone(),
        action_hash.clone(),
        LinkTypes::EnrollmentToOutcomes,
        (),
    )?;

    // Check if this outcome triggers an alert
    check_and_create_alert(&outcome)?;

    log_data_access(
        outcome.patient_hash,
        vec![DataCategory::VitalSigns],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(action_hash)
}

fn check_and_create_alert(outcome: &PatientReportedOutcome) -> ExternResult<()> {
    // Example thresholds - in production these would be configurable per patient
    let alert = match outcome.measurement_type.as_str() {
        "blood_glucose" => {
            if outcome.value > 300.0 {
                Some(("High Blood Glucose", AlertSeverity::Urgent, 300.0))
            } else if outcome.value < 70.0 {
                Some(("Low Blood Glucose", AlertSeverity::Critical, 70.0))
            } else {
                None
            }
        }
        "blood_pressure_systolic" => {
            if outcome.value > 180.0 {
                Some(("Hypertensive Crisis", AlertSeverity::Critical, 180.0))
            } else if outcome.value > 140.0 {
                Some(("Elevated Blood Pressure", AlertSeverity::Warning, 140.0))
            } else {
                None
            }
        }
        "weight_kg" => {
            // Weight gain > 2kg in a day can indicate heart failure exacerbation
            // This would need previous value comparison in production
            None
        }
        "oxygen_saturation" => {
            if outcome.value < 90.0 {
                Some(("Low Oxygen Saturation", AlertSeverity::Critical, 90.0))
            } else if outcome.value < 94.0 {
                Some(("Reduced Oxygen Saturation", AlertSeverity::Warning, 94.0))
            } else {
                None
            }
        }
        _ => None,
    };

    if let Some((alert_type, severity, threshold)) = alert {
        let chronic_alert = ChronicCareAlert {
            patient_hash: outcome.patient_hash.clone(),
            enrollment_hash: outcome.enrollment_hash.clone(),
            alert_type: alert_type.to_string(),
            severity,
            message: format!(
                "{}: {} {} (threshold: {})",
                alert_type, outcome.value, outcome.unit, threshold
            ),
            trigger_value: Some(outcome.value.to_string()),
            threshold: Some(threshold.to_string()),
            recommended_action: get_recommended_action(alert_type),
            acknowledged: false,
            acknowledged_by: None,
            acknowledged_at: None,
            created_at: get_sys_time()?,
        };

        create_alert(chronic_alert)?;
    }

    Ok(())
}

fn get_recommended_action(alert_type: &str) -> Option<String> {
    match alert_type {
        "Low Blood Glucose" => Some("Consume fast-acting carbohydrates immediately. If symptoms persist, seek emergency care.".to_string()),
        "High Blood Glucose" => Some("Check for ketones if diabetic. Contact healthcare provider for guidance.".to_string()),
        "Hypertensive Crisis" => Some("Seek immediate medical attention. Rest in a calm environment.".to_string()),
        "Low Oxygen Saturation" => Some("Use supplemental oxygen if prescribed. Seek immediate medical attention.".to_string()),
        _ => None,
    }
}

/// Record diabetes-specific metrics
#[hdk_extern]
pub fn record_diabetes_metrics(metrics: DiabetesMetrics) -> ExternResult<ActionHash> {
    let auth = require_authorization(
        metrics.patient_hash.clone(),
        DataCategory::LabResults,
        Permission::Write,
        false,
    )?;
    let action_hash = create_entry(&EntryTypes::DiabetesMetrics(metrics.clone()))?;

    // Check for hypoglycemic events
    if metrics.hypoglycemic_events > 0 {
        // Could trigger an alert or care plan review
    }

    // Check HbA1c targets
    if let Some(hba1c) = metrics.hba1c {
        if hba1c > 9.0 {
            // Significantly above target, may need care plan adjustment
        }
    }

    log_data_access(
        metrics.patient_hash,
        vec![DataCategory::LabResults],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(action_hash)
}

/// Record heart failure-specific metrics
#[hdk_extern]
pub fn record_heart_failure_metrics(metrics: HeartFailureMetrics) -> ExternResult<ActionHash> {
    let auth = require_authorization(
        metrics.patient_hash.clone(),
        DataCategory::VitalSigns,
        Permission::Write,
        false,
    )?;
    let action_hash = create_entry(&EntryTypes::HeartFailureMetrics(metrics.clone()))?;

    // Check for rapid weight gain (fluid retention)
    if let Some(weight_change) = metrics.weight_change_kg {
        if weight_change > 1.5 {
            // >1.5kg gain in a day may indicate fluid retention
            let alert = ChronicCareAlert {
                patient_hash: metrics.patient_hash.clone(),
                enrollment_hash: ActionHash::from_raw_36(vec![0; 36]), // Would need actual enrollment
                alert_type: "Rapid Weight Gain".to_string(),
                severity: AlertSeverity::Warning,
                message: format!("Weight increased by {:.1} kg in one day", weight_change),
                trigger_value: Some(weight_change.to_string()),
                threshold: Some("1.5".to_string()),
                recommended_action: Some("Check for swelling. May indicate fluid retention. Contact care team if symptoms worsen.".to_string()),
                acknowledged: false,
                acknowledged_by: None,
                acknowledged_at: None,
                created_at: get_sys_time()?,
            };
            create_entry(&EntryTypes::ChronicCareAlert(alert))?;
        }
    }

    log_data_access(
        metrics.patient_hash,
        vec![DataCategory::VitalSigns],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(action_hash)
}

/// Record COPD-specific metrics
#[hdk_extern]
pub fn record_copd_metrics(metrics: COPDMetrics) -> ExternResult<ActionHash> {
    let auth = require_authorization(
        metrics.patient_hash.clone(),
        DataCategory::VitalSigns,
        Permission::Write,
        false,
    )?;
    let action_hash = create_entry(&EntryTypes::COPDMetrics(metrics.clone()))?;

    // Check for exacerbation indicators
    if metrics.exacerbation {
        // Record exacerbation event and potentially trigger care protocol
    }

    // High rescue inhaler use may indicate poor control
    if metrics.rescue_inhaler_uses > 4 {
        // May need care plan review
    }

    log_data_access(
        metrics.patient_hash,
        vec![DataCategory::VitalSigns],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(action_hash)
}

/// Record medication adherence
#[hdk_extern]
pub fn record_medication_adherence(adherence: MedicationAdherence) -> ExternResult<ActionHash> {
    let auth = require_authorization(
        adherence.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Write,
        false,
    )?;
    let action_hash = create_entry(&EntryTypes::MedicationAdherence(adherence.clone()))?;

    create_link(
        adherence.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToAdherence,
        (),
    )?;

    log_data_access(
        adherence.patient_hash,
        vec![DataCategory::Medications],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(action_hash)
}

/// Get medication adherence rate for a patient
#[hdk_extern]
pub fn get_adherence_rate(input: AdherenceRateInput) -> ExternResult<AdherenceRateOutput> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Read,
        false,
    )?;
    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToAdherence)?, GetStrategy::default(),
    )?;

    let mut taken_count = 0;
    let mut total_count = 0;

    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(adherence) = record.entry().to_app_option::<MedicationAdherence>().ok().flatten() {
                    // Filter by medication if specified
                    if let Some(ref med_name) = input.medication_name {
                        if &adherence.medication_name != med_name {
                            continue;
                        }
                    }

                    // Filter by date range if specified
                    if let Some(start) = input.start_date {
                        if adherence.scheduled_date < start {
                            continue;
                        }
                    }
                    if let Some(end) = input.end_date {
                        if adherence.scheduled_date > end {
                            continue;
                        }
                    }

                    total_count += 1;
                    if adherence.taken {
                        taken_count += 1;
                    }
                }
            }
        }
    }

    let rate = if total_count > 0 {
        (taken_count as f64 / total_count as f64) * 100.0
    } else {
        0.0
    };

    let output = AdherenceRateOutput {
        taken_count,
        total_count,
        adherence_rate: rate,
    };

    log_data_access(
        input.patient_hash,
        vec![DataCategory::Medications],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(output)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdherenceRateInput {
    pub patient_hash: ActionHash,
    pub medication_name: Option<String>,
    pub start_date: Option<Timestamp>,
    pub end_date: Option<Timestamp>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdherenceRateOutput {
    pub taken_count: u32,
    pub total_count: u32,
    pub adherence_rate: f64,
}

/// Create a chronic care alert
#[hdk_extern]
pub fn create_alert(alert: ChronicCareAlert) -> ExternResult<ActionHash> {
    let auth = require_authorization(
        alert.patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Write,
        false,
    )?;
    let action_hash = create_entry(&EntryTypes::ChronicCareAlert(alert.clone()))?;

    create_link(
        alert.enrollment_hash.clone(),
        action_hash.clone(),
        LinkTypes::EnrollmentToAlerts,
        (),
    )?;

    log_data_access(
        alert.patient_hash,
        vec![DataCategory::Diagnoses],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(action_hash)
}

/// Acknowledge an alert
#[hdk_extern]
pub fn acknowledge_alert(input: AcknowledgeAlertInput) -> ExternResult<ActionHash> {
    let record = get(input.alert_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Alert not found".to_string())))?;

    let mut alert: ChronicCareAlert = record.entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid alert entry".to_string())))?;

    let auth = require_authorization(
        alert.patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Write,
        false,
    )?;

    alert.acknowledged = true;
    alert.acknowledged_by = Some(input.acknowledged_by);
    alert.acknowledged_at = Some(get_sys_time()?);

    let updated_hash = update_entry(input.alert_hash, &alert)?;

    log_data_access(
        alert.patient_hash,
        vec![DataCategory::Diagnoses],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(updated_hash)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AcknowledgeAlertInput {
    pub alert_hash: ActionHash,
    pub acknowledged_by: ActionHash,
}

/// Get unacknowledged alerts for an enrollment
#[hdk_extern]
pub fn get_pending_alerts(enrollment_hash: ActionHash) -> ExternResult<Vec<ChronicCareAlert>> {
    let enrollment_record = get(enrollment_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Enrollment not found".to_string())))?;

    let enrollment: ChronicDiseaseEnrollment = enrollment_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid enrollment".to_string())))?;

    let auth = require_authorization(
        enrollment.patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(enrollment_hash, LinkTypes::EnrollmentToAlerts)?, GetStrategy::default(),
    )?;

    let mut alerts = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(alert) = record.entry().to_app_option::<ChronicCareAlert>().ok().flatten() {
                    if !alert.acknowledged {
                        alerts.push(alert);
                    }
                }
            }
        }
    }

    // Sort by severity (Critical first)
    alerts.sort_by(|a, b| {
        let severity_order = |s: &AlertSeverity| match s {
            AlertSeverity::Critical => 0,
            AlertSeverity::Urgent => 1,
            AlertSeverity::Warning => 2,
            AlertSeverity::Info => 3,
        };
        severity_order(&a.severity).cmp(&severity_order(&b.severity))
    });

    if !alerts.is_empty() {
        log_data_access(
            enrollment.patient_hash,
            vec![DataCategory::Diagnoses],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(alerts)
}

/// Record an exacerbation event
#[hdk_extern]
pub fn record_exacerbation(event: ExacerbationEvent) -> ExternResult<ActionHash> {
    let auth = require_authorization(
        event.patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Write,
        false,
    )?;
    let action_hash = create_entry(&EntryTypes::ExacerbationEvent(event.clone()))?;

    create_link(
        event.enrollment_hash.clone(),
        action_hash.clone(),
        LinkTypes::EnrollmentToExacerbations,
        (),
    )?;

    // Create alert for exacerbation
    let alert = ChronicCareAlert {
        patient_hash: event.patient_hash.clone(),
        enrollment_hash: event.enrollment_hash.clone(),
        alert_type: "Exacerbation Event".to_string(),
        severity: event.severity.clone(),
        message: format!(
            "Patient experiencing exacerbation with symptoms: {}",
            event.symptoms.join(", ")
        ),
        trigger_value: None,
        threshold: None,
        recommended_action: if event.hospitalization_required {
            Some("Hospitalization may be required".to_string())
        } else {
            Some("Review care plan and consider treatment adjustment".to_string())
        },
        acknowledged: false,
        acknowledged_by: None,
        acknowledged_at: None,
        created_at: get_sys_time()?,
    };

    create_alert(alert)?;

    log_data_access(
        event.patient_hash,
        vec![DataCategory::Diagnoses],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(action_hash)
}

/// Get chronic care summary for a patient
#[hdk_extern]
pub fn get_chronic_care_summary(patient_hash: ActionHash) -> ExternResult<ChronicCareSummary> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::Diagnoses,
        Permission::Read,
        false,
    )?;
    let enrollments = get_patient_enrollments(patient_hash.clone())?;

    let total_pending_alerts = 0;
    let mut conditions = Vec::new();

    for enrollment in &enrollments {
        if enrollment.is_active {
            conditions.push(get_condition_tag(&enrollment.condition));

            // Count pending alerts for this enrollment
            // Would need enrollment hash tracking
        }
    }

    let summary = ChronicCareSummary {
        patient_hash,
        active_enrollments: enrollments.iter().filter(|e| e.is_active).count() as u32,
        conditions,
        pending_alerts: total_pending_alerts,
    };

    log_data_access(
        summary.patient_hash.clone(),
        vec![DataCategory::Diagnoses],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(summary)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChronicCareSummary {
    pub patient_hash: ActionHash,
    pub active_enrollments: u32,
    pub conditions: Vec<String>,
    pub pending_alerts: u32,
}

fn get_sys_time() -> ExternResult<Timestamp> {
    hdk::prelude::sys_time()
}
