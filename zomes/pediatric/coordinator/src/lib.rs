//! Pediatric Coordinator Zome
//!
//! Pediatric care coordination including growth tracking, immunization schedules,
//! developmental milestones, and well-child visits.

use hdk::prelude::*;
use pediatric_integrity::*;

/// Record a growth measurement
#[hdk_extern]
pub fn record_growth(measurement: GrowthMeasurement) -> ExternResult<ActionHash> {
    let action_hash = create_entry(&EntryTypes::GrowthMeasurement(measurement.clone()))?;

    create_link(
        measurement.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToGrowth,
        (),
    )?;

    Ok(action_hash)
}

/// Get growth history for a patient
#[hdk_extern]
pub fn get_growth_history(patient_hash: ActionHash) -> ExternResult<Vec<GrowthMeasurement>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToGrowth)?, GetStrategy::default(),
    )?;

    let mut measurements = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(measurement) = record.entry().to_app_option::<GrowthMeasurement>().ok().flatten() {
                    measurements.push(measurement);
                }
            }
        }
    }

    // Sort by age
    measurements.sort_by(|a, b| a.age_months.cmp(&b.age_months));

    Ok(measurements)
}

/// Calculate BMI and percentile for a growth measurement
#[hdk_extern]
pub fn calculate_growth_percentiles(input: CalculatePercentilesInput) -> ExternResult<GrowthPercentiles> {
    // BMI = weight (kg) / height (m)^2
    let height_m = input.height_cm / 100.0;
    let bmi = input.weight_kg / (height_m * height_m);

    // In a real implementation, these would look up CDC/WHO growth charts
    // based on age and sex. This is a simplified placeholder.
    Ok(GrowthPercentiles {
        weight_percentile: estimate_percentile(input.weight_kg, input.age_months, &input.sex, "weight"),
        height_percentile: estimate_percentile(input.height_cm, input.age_months, &input.sex, "height"),
        bmi,
        bmi_percentile: estimate_percentile(bmi, input.age_months, &input.sex, "bmi"),
        head_percentile: input.head_circumference_cm.map(|hc|
            estimate_percentile(hc, input.age_months, &input.sex, "head")
        ),
    })
}

fn estimate_percentile(_value: f64, _age_months: u32, _sex: &str, _measurement_type: &str) -> f64 {
    // Placeholder - real implementation would use CDC/WHO growth charts
    // This returns a dummy value
    50.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalculatePercentilesInput {
    pub weight_kg: f64,
    pub height_cm: f64,
    pub head_circumference_cm: Option<f64>,
    pub age_months: u32,
    pub sex: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GrowthPercentiles {
    pub weight_percentile: f64,
    pub height_percentile: f64,
    pub bmi: f64,
    pub bmi_percentile: f64,
    pub head_percentile: Option<f64>,
}

/// Record an immunization
#[hdk_extern]
pub fn record_immunization(record: ImmunizationRecord) -> ExternResult<ActionHash> {
    let action_hash = create_entry(&EntryTypes::ImmunizationRecord(record.clone()))?;

    create_link(
        record.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToImmunizations,
        (),
    )?;

    // Link by vaccine type for easy lookup
    let vaccine_tag = format!("vaccine:{:?}", record.vaccine_type);
    let vaccine_path = Path::from(vaccine_tag.as_str());
    create_link(
        vaccine_path.path_entry_hash()?,
        action_hash.clone(),
        LinkTypes::VaccineTypeToRecords,
        (),
    )?;

    Ok(action_hash)
}

/// Get immunization history for a patient
#[hdk_extern]
pub fn get_immunization_history(patient_hash: ActionHash) -> ExternResult<Vec<ImmunizationRecord>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToImmunizations)?, GetStrategy::default(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(imm_record) = record.entry().to_app_option::<ImmunizationRecord>().ok().flatten() {
                    records.push(imm_record);
                }
            }
        }
    }

    // Sort by date
    records.sort_by(|a, b| a.administration_date.cmp(&b.administration_date));

    Ok(records)
}

/// Get immunization status for a patient
#[hdk_extern]
pub fn get_immunization_status(input: ImmunizationStatusInput) -> ExternResult<ImmunizationStatusOutput> {
    let history = get_immunization_history(input.patient_hash.clone())?;

    // Get required vaccines for age
    let required = get_required_vaccines(input.age_months);

    let mut missing = Vec::new();
    let mut due_soon = Vec::new();
    let mut up_to_date = Vec::new();

    for (vaccine, required_doses) in required {
        let received: Vec<&ImmunizationRecord> = history.iter()
            .filter(|r| std::mem::discriminant(&r.vaccine_type) == std::mem::discriminant(&vaccine))
            .collect();

        let received_count = received.len() as u8;

        if received_count >= required_doses {
            up_to_date.push(format!("{:?}", vaccine));
        } else if received_count > 0 {
            due_soon.push(VaccineDue {
                vaccine: format!("{:?}", vaccine),
                dose_needed: received_count + 1,
                doses_required: required_doses,
            });
        } else {
            missing.push(format!("{:?}", vaccine));
        }
    }

    let status = if missing.is_empty() && due_soon.is_empty() {
        ImmunizationStatus::UpToDate
    } else if !missing.is_empty() {
        ImmunizationStatus::Overdue
    } else {
        ImmunizationStatus::Due
    };

    Ok(ImmunizationStatusOutput {
        overall_status: status,
        up_to_date,
        due_soon,
        missing,
    })
}

fn get_required_vaccines(age_months: u32) -> Vec<(VaccineType, u8)> {
    // CDC recommended schedule - simplified
    let mut required = Vec::new();

    // Birth
    required.push((VaccineType::HepB, 1));

    if age_months >= 2 {
        required.push((VaccineType::HepB, 2));
        required.push((VaccineType::RV, 1));
        required.push((VaccineType::DTaP, 1));
        required.push((VaccineType::Hib, 1));
        required.push((VaccineType::PCV, 1));
        required.push((VaccineType::IPV, 1));
    }

    if age_months >= 4 {
        required.push((VaccineType::RV, 2));
        required.push((VaccineType::DTaP, 2));
        required.push((VaccineType::Hib, 2));
        required.push((VaccineType::PCV, 2));
        required.push((VaccineType::IPV, 2));
    }

    if age_months >= 6 {
        required.push((VaccineType::HepB, 3));
        required.push((VaccineType::DTaP, 3));
        required.push((VaccineType::PCV, 3));
        required.push((VaccineType::Influenza, 1));
    }

    if age_months >= 12 {
        required.push((VaccineType::Hib, 3));
        required.push((VaccineType::PCV, 4));
        required.push((VaccineType::MMR, 1));
        required.push((VaccineType::Varicella, 1));
        required.push((VaccineType::HepA, 1));
    }

    if age_months >= 15 {
        required.push((VaccineType::DTaP, 4));
    }

    if age_months >= 18 {
        required.push((VaccineType::HepA, 2));
    }

    if age_months >= 48 { // 4 years
        required.push((VaccineType::DTaP, 5));
        required.push((VaccineType::IPV, 4));
        required.push((VaccineType::MMR, 2));
        required.push((VaccineType::Varicella, 2));
    }

    if age_months >= 132 { // 11 years
        required.push((VaccineType::Tdap, 1));
        required.push((VaccineType::HPV, 1));
        required.push((VaccineType::MenACWY, 1));
    }

    if age_months >= 192 { // 16 years
        required.push((VaccineType::MenACWY, 2));
    }

    required
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImmunizationStatusInput {
    pub patient_hash: ActionHash,
    pub age_months: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImmunizationStatusOutput {
    pub overall_status: ImmunizationStatus,
    pub up_to_date: Vec<String>,
    pub due_soon: Vec<VaccineDue>,
    pub missing: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaccineDue {
    pub vaccine: String,
    pub dose_needed: u8,
    pub doses_required: u8,
}

/// Record a developmental milestone assessment
#[hdk_extern]
pub fn record_milestone(milestone: DevelopmentalMilestone) -> ExternResult<ActionHash> {
    let action_hash = create_entry(&EntryTypes::DevelopmentalMilestone(milestone.clone()))?;

    create_link(
        milestone.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToMilestones,
        (),
    )?;

    Ok(action_hash)
}

/// Get developmental milestone history
#[hdk_extern]
pub fn get_milestones(patient_hash: ActionHash) -> ExternResult<Vec<DevelopmentalMilestone>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToMilestones)?, GetStrategy::default(),
    )?;

    let mut milestones = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(milestone) = record.entry().to_app_option::<DevelopmentalMilestone>().ok().flatten() {
                    milestones.push(milestone);
                }
            }
        }
    }

    milestones.sort_by(|a, b| a.assessment_date.cmp(&b.assessment_date));

    Ok(milestones)
}

/// Record a developmental screening
#[hdk_extern]
pub fn record_developmental_screening(screening: DevelopmentalScreening) -> ExternResult<ActionHash> {
    let action_hash = create_entry(&EntryTypes::DevelopmentalScreening(screening.clone()))?;

    create_link(
        screening.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToScreenings,
        (),
    )?;

    Ok(action_hash)
}

/// Record a well-child visit
#[hdk_extern]
pub fn record_well_child_visit(visit: WellChildVisitRecord) -> ExternResult<ActionHash> {
    let action_hash = create_entry(&EntryTypes::WellChildVisitRecord(visit.clone()))?;

    create_link(
        visit.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToWellChildVisits,
        (),
    )?;

    Ok(action_hash)
}

/// Get well-child visit history
#[hdk_extern]
pub fn get_well_child_visits(patient_hash: ActionHash) -> ExternResult<Vec<WellChildVisitRecord>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToWellChildVisits)?, GetStrategy::default(),
    )?;

    let mut visits = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(visit) = record.entry().to_app_option::<WellChildVisitRecord>().ok().flatten() {
                    visits.push(visit);
                }
            }
        }
    }

    visits.sort_by(|a, b| a.visit_date.cmp(&b.visit_date));

    Ok(visits)
}

/// Get next recommended well-child visit
#[hdk_extern]
pub fn get_next_well_child_visit(input: NextVisitInput) -> ExternResult<NextVisitRecommendation> {
    let visits = get_well_child_visits(input.patient_hash.clone())?;

    let completed_visits: Vec<WellChildVisit> = visits.iter()
        .map(|v| v.visit_type.clone())
        .collect();

    let schedule = get_well_child_schedule();

    for (visit_type, age_months, name) in schedule {
        if input.age_months >= age_months && !completed_visits.contains(&visit_type) {
            return Ok(NextVisitRecommendation {
                visit_type,
                visit_name: name.to_string(),
                recommended_age_months: age_months,
                is_overdue: input.age_months > age_months + 1,
            });
        }
    }

    // All caught up, recommend next annual visit
    Ok(NextVisitRecommendation {
        visit_type: WellChildVisit::Annual,
        visit_name: "Annual Well-Child Visit".to_string(),
        recommended_age_months: ((input.age_months / 12) + 1) * 12,
        is_overdue: false,
    })
}

fn get_well_child_schedule() -> Vec<(WellChildVisit, u32, &'static str)> {
    vec![
        (WellChildVisit::Newborn, 0, "Newborn Visit (3-5 days)"),
        (WellChildVisit::OneMonth, 1, "1 Month Visit"),
        (WellChildVisit::TwoMonths, 2, "2 Month Visit"),
        (WellChildVisit::FourMonths, 4, "4 Month Visit"),
        (WellChildVisit::SixMonths, 6, "6 Month Visit"),
        (WellChildVisit::NineMonths, 9, "9 Month Visit"),
        (WellChildVisit::TwelveMonths, 12, "12 Month Visit"),
        (WellChildVisit::FifteenMonths, 15, "15 Month Visit"),
        (WellChildVisit::EighteenMonths, 18, "18 Month Visit"),
        (WellChildVisit::TwentyFourMonths, 24, "2 Year Visit"),
        (WellChildVisit::ThirtyMonths, 30, "2.5 Year Visit"),
        (WellChildVisit::ThreeYears, 36, "3 Year Visit"),
    ]
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NextVisitInput {
    pub patient_hash: ActionHash,
    pub age_months: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NextVisitRecommendation {
    pub visit_type: WellChildVisit,
    pub visit_name: String,
    pub recommended_age_months: u32,
    pub is_overdue: bool,
}

/// Record a pediatric condition
#[hdk_extern]
pub fn record_condition(condition: PediatricCondition) -> ExternResult<ActionHash> {
    let action_hash = create_entry(&EntryTypes::PediatricCondition(condition.clone()))?;

    create_link(
        condition.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToConditions,
        (),
    )?;

    Ok(action_hash)
}

/// Record school health record
#[hdk_extern]
pub fn record_school_health(record: SchoolHealthRecord) -> ExternResult<ActionHash> {
    let action_hash = create_entry(&EntryTypes::SchoolHealthRecord(record.clone()))?;

    create_link(
        record.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToSchoolRecords,
        (),
    )?;

    Ok(action_hash)
}

/// Record adolescent health assessment
#[hdk_extern]
pub fn record_adolescent_health(record: AdolescentHealthRecord) -> ExternResult<ActionHash> {
    let action_hash = create_entry(&EntryTypes::AdolescentHealthRecord(record.clone()))?;

    create_link(
        record.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToAdolescentRecords,
        (),
    )?;

    Ok(action_hash)
}

/// Record newborn information
#[hdk_extern]
pub fn record_newborn(record: NewbornRecord) -> ExternResult<ActionHash> {
    let action_hash = create_entry(&EntryTypes::NewbornRecord(record.clone()))?;

    create_link(
        record.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToNewbornRecord,
        (),
    )?;

    Ok(action_hash)
}

/// Get pediatric summary for a patient
#[hdk_extern]
pub fn get_pediatric_summary(input: PediatricSummaryInput) -> ExternResult<PediatricSummary> {
    let growth = get_growth_history(input.patient_hash.clone())?;
    let immunization_status = get_immunization_status(ImmunizationStatusInput {
        patient_hash: input.patient_hash.clone(),
        age_months: input.age_months,
    })?;
    let visits = get_well_child_visits(input.patient_hash.clone())?;
    let milestones = get_milestones(input.patient_hash.clone())?;

    let next_visit = get_next_well_child_visit(NextVisitInput {
        patient_hash: input.patient_hash.clone(),
        age_months: input.age_months,
    })?;

    let latest_growth = growth.last().cloned();

    let delayed_milestones: Vec<String> = milestones.iter()
        .filter(|m| m.status == MilestoneStatus::Delayed || m.status == MilestoneStatus::NeedsEvaluation)
        .map(|m| m.milestone_description.clone())
        .collect();

    let age_group = get_age_group(input.age_months);

    Ok(PediatricSummary {
        patient_hash: input.patient_hash,
        age_months: input.age_months,
        age_group,
        latest_growth,
        immunization_status: immunization_status.overall_status,
        vaccines_due: immunization_status.due_soon.len() as u32 + immunization_status.missing.len() as u32,
        well_child_visits_completed: visits.len() as u32,
        next_visit_recommendation: next_visit.visit_name,
        next_visit_overdue: next_visit.is_overdue,
        developmental_concerns: !delayed_milestones.is_empty(),
        delayed_milestones,
    })
}

fn get_age_group(age_months: u32) -> PediatricAgeGroup {
    if age_months == 0 {
        PediatricAgeGroup::Newborn
    } else if age_months < 12 {
        PediatricAgeGroup::Infant
    } else if age_months < 36 {
        PediatricAgeGroup::Toddler
    } else if age_months < 60 {
        PediatricAgeGroup::Preschool
    } else if age_months < 144 {
        PediatricAgeGroup::SchoolAge
    } else if age_months < 216 {
        PediatricAgeGroup::Adolescent
    } else {
        PediatricAgeGroup::YoungAdult
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PediatricSummaryInput {
    pub patient_hash: ActionHash,
    pub age_months: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PediatricSummary {
    pub patient_hash: ActionHash,
    pub age_months: u32,
    pub age_group: PediatricAgeGroup,
    pub latest_growth: Option<GrowthMeasurement>,
    pub immunization_status: ImmunizationStatus,
    pub vaccines_due: u32,
    pub well_child_visits_completed: u32,
    pub next_visit_recommendation: String,
    pub next_visit_overdue: bool,
    pub developmental_concerns: bool,
    pub delayed_milestones: Vec<String>,
}
