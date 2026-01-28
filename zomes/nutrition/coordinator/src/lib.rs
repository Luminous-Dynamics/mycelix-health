//! Nutrition Coordinator Zome
//!
//! Coordinator functions for nutrition tracking, dietary restrictions,
//! drug-food interactions, and nutrition recommendations.
//!
//! Integrates with the health-food SDK bridge module.

use hdk::prelude::*;
use nutrition_integrity::*;
use mycelix_health_shared::{require_authorization, log_data_access, DataCategory, Permission};

// ============================================================================
// Anchor Entry for Indexing
// ============================================================================

/// Anchor entry for creating deterministic entry hashes for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

/// Create an anchor hash for indexing
fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}

// ============================================================================
// Dietary Restriction Functions
// ============================================================================

/// Get all dietary restrictions for a patient
#[hdk_extern]
pub fn get_patient_restrictions(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::Allergies,
        Permission::Read,
        false,
    )?;
    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToRestrictions)?,
        GetStrategy::default(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    if !records.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::Allergies],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(records)
}

/// Add a new dietary restriction
#[hdk_extern]
pub fn add_dietary_restriction(restriction: DietaryRestriction) -> ExternResult<Record> {
    let auth = require_authorization(
        restriction.patient_hash.clone(),
        DataCategory::Allergies,
        Permission::Write,
        false,
    )?;
    let action_hash = create_entry(&EntryTypes::DietaryRestriction(restriction.clone()))?;

    // Link from patient to restriction
    create_link(
        restriction.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToRestrictions,
        (),
    )?;

    // If linked to allergy, create that link too
    if let Some(allergy_hash) = &restriction.linked_allergy_hash {
        create_link(
            action_hash.clone(),
            allergy_hash.clone(),
            LinkTypes::RestrictionToAllergy,
            (),
        )?;
    }

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get created restriction".into())))?;

    log_data_access(
        restriction.patient_hash,
        vec![DataCategory::Allergies],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Update a dietary restriction
#[hdk_extern]
pub fn update_dietary_restriction(input: UpdateRestrictionInput) -> ExternResult<Record> {
    let record = get(input.original_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Restriction not found".into())))?;

    let existing: DietaryRestriction = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid restriction entry".into())))?;

    if existing.patient_hash != input.updated_restriction.patient_hash {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Cannot change patient_hash on a restriction".into()
        )));
    }

    let auth = require_authorization(
        input.updated_restriction.patient_hash.clone(),
        DataCategory::Allergies,
        Permission::Write,
        false,
    )?;

    let updated_hash = update_entry(input.original_hash.clone(), &input.updated_restriction)?;

    let updated_record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get updated restriction".into())))?;

    log_data_access(
        input.updated_restriction.patient_hash,
        vec![DataCategory::Allergies],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(updated_record)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateRestrictionInput {
    pub original_hash: ActionHash,
    pub updated_restriction: DietaryRestriction,
}

/// Deactivate a dietary restriction
#[hdk_extern]
pub fn deactivate_restriction(restriction_hash: ActionHash) -> ExternResult<Record> {
    let record = get(restriction_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Restriction not found".into())))?;

    let mut restriction: DietaryRestriction = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid restriction entry".into())))?;

    restriction.active = false;
    restriction.updated_at = sys_time()?;

    let auth = require_authorization(
        restriction.patient_hash.clone(),
        DataCategory::Allergies,
        Permission::Write,
        false,
    )?;

    let updated_hash = update_entry(restriction_hash, &restriction)?;
    let updated_record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get updated restriction".into())))?;

    log_data_access(
        restriction.patient_hash,
        vec![DataCategory::Allergies],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(updated_record)
}

// ============================================================================
// Drug-Food Interaction Functions
// ============================================================================

/// Get drug-food interactions for a medication
#[hdk_extern]
pub fn get_drug_food_interactions(medication_name: String) -> ExternResult<Vec<Record>> {
    // Create anchor for medication lookup
    let medication_anchor = anchor_hash(&medication_name.to_lowercase())?;

    let links = get_links(
        LinkQuery::try_new(medication_anchor, LinkTypes::MedicationToInteractions)?,
        GetStrategy::default(),
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

/// Add a drug-food interaction entry
#[hdk_extern]
pub fn add_drug_food_interaction(interaction: DrugFoodInteraction) -> ExternResult<Record> {
    let action_hash = create_entry(&EntryTypes::DrugFoodInteraction(interaction.clone()))?;

    // Link from medication anchor
    let medication_anchor = anchor_hash(&interaction.medication_name.to_lowercase())?;
    create_link(
        medication_anchor,
        action_hash.clone(),
        LinkTypes::MedicationToInteractions,
        (),
    )?;

    // Link from food category
    let food_anchor = anchor_hash(&format!("food:{:?}", interaction.food_category))?;
    create_link(
        food_anchor,
        action_hash.clone(),
        LinkTypes::FoodCategoryToInteractions,
        (),
    )?;

    // Link to all interactions index
    let all_anchor = anchor_hash("all_interactions")?;
    create_link(
        all_anchor,
        action_hash.clone(),
        LinkTypes::AllInteractions,
        (),
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get created interaction".into())))
}

/// Get all drug-food interactions
#[hdk_extern]
pub fn get_all_interactions(_: ()) -> ExternResult<Vec<Record>> {
    let all_anchor = anchor_hash("all_interactions")?;

    let links = get_links(
        LinkQuery::try_new(all_anchor, LinkTypes::AllInteractions)?,
        GetStrategy::default(),
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

/// Check food safety for a patient based on their medications
#[hdk_extern]
pub fn check_food_safety(input: CheckFoodSafetyInput) -> ExternResult<FoodSafetyResult> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Read,
        false,
    )?;
    let mut warnings = Vec::new();
    let mut contraindicated = Vec::new();

    // Get patient's active medications (would integrate with prescriptions zome)
    // For now, check against provided food categories

    for category in &input.food_categories {
        let food_anchor = anchor_hash(&format!("food:{:?}", category))?;

        let links = get_links(
            LinkQuery::try_new(food_anchor, LinkTypes::FoodCategoryToInteractions)?,
            GetStrategy::default(),
        )?;

        for link in links {
            if let Some(hash) = link.target.into_action_hash() {
                if let Some(record) = get(hash, GetOptions::default())? {
                    if let Some(interaction) = record
                        .entry()
                        .to_app_option::<DrugFoodInteraction>()
                        .ok()
                        .flatten()
                    {
                        match interaction.severity {
                            InteractionSeverity::Contraindicated => {
                                contraindicated.push(FoodWarning {
                                    food_category: category.clone(),
                                    medication: interaction.medication_name.clone(),
                                    severity: "Contraindicated".to_string(),
                                    recommendation: interaction.recommendation.clone(),
                                });
                            }
                            InteractionSeverity::Major => {
                                warnings.push(FoodWarning {
                                    food_category: category.clone(),
                                    medication: interaction.medication_name.clone(),
                                    severity: "Major".to_string(),
                                    recommendation: interaction.recommendation.clone(),
                                });
                            }
                            _ => {
                                warnings.push(FoodWarning {
                                    food_category: category.clone(),
                                    medication: interaction.medication_name.clone(),
                                    severity: format!("{:?}", interaction.severity),
                                    recommendation: interaction.recommendation.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    let result = FoodSafetyResult {
        safe: contraindicated.is_empty(),
        warnings,
        contraindicated,
    };

    log_data_access(
        input.patient_hash,
        vec![DataCategory::Medications],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(result)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CheckFoodSafetyInput {
    pub patient_hash: ActionHash,
    pub food_categories: Vec<FoodCategory>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FoodSafetyResult {
    pub safe: bool,
    pub warnings: Vec<FoodWarning>,
    pub contraindicated: Vec<FoodWarning>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FoodWarning {
    pub food_category: FoodCategory,
    pub medication: String,
    pub severity: String,
    pub recommendation: String,
}

// ============================================================================
// Nutrition Goal Functions
// ============================================================================

/// Get active nutrition goals for a patient
#[hdk_extern]
pub fn get_patient_goals(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        false,
    )?;
    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToGoals)?,
        GetStrategy::default(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                // Filter for active goals
                if let Some(goal) = record
                    .entry()
                    .to_app_option::<NutritionGoal>()
                    .ok()
                    .flatten()
                {
                    if goal.active {
                        records.push(record);
                    }
                }
            }
        }
    }

    if !records.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::All],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(records)
}

/// Create a nutrition goal
#[hdk_extern]
pub fn create_nutrition_goal(goal: NutritionGoal) -> ExternResult<Record> {
    let auth = require_authorization(
        goal.patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;
    let action_hash = create_entry(&EntryTypes::NutritionGoal(goal.clone()))?;

    create_link(
        goal.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToGoals,
        (),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get created goal".into())))?;

    log_data_access(
        goal.patient_hash,
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Update a nutrition goal
#[hdk_extern]
pub fn update_nutrition_goal(input: UpdateGoalInput) -> ExternResult<Record> {
    let record = get(input.original_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Goal not found".into())))?;

    let existing: NutritionGoal = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid goal entry".into())))?;

    if existing.patient_hash != input.updated_goal.patient_hash {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Cannot change patient_hash on a goal".into()
        )));
    }

    let auth = require_authorization(
        input.updated_goal.patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;

    let updated_hash = update_entry(input.original_hash, &input.updated_goal)?;

    let updated_record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get updated goal".into())))?;

    log_data_access(
        input.updated_goal.patient_hash,
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(updated_record)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateGoalInput {
    pub original_hash: ActionHash,
    pub updated_goal: NutritionGoal,
}

// ============================================================================
// Meal Logging Functions
// ============================================================================

/// Log a meal
#[hdk_extern]
pub fn log_meal(meal: MealLog) -> ExternResult<Record> {
    let auth = require_authorization(
        meal.patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;
    let action_hash = create_entry(&EntryTypes::MealLog(meal.clone()))?;

    // Link from patient
    create_link(
        meal.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToMeals,
        (),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get created meal log".into())))?;

    log_data_access(
        meal.patient_hash,
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Get patient's meal logs for a date range
#[hdk_extern]
pub fn get_patient_meals(input: GetMealsInput) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        false,
    )?;
    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToMeals)?,
        GetStrategy::default(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(meal) = record
                    .entry()
                    .to_app_option::<MealLog>()
                    .ok()
                    .flatten()
                {
                    // Filter by date range
                    let meal_time = meal.timestamp.as_micros();
                    if meal_time >= input.start_date && meal_time <= input.end_date {
                        records.push(record);
                    }
                }
            }
        }
    }

    if !records.is_empty() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::All],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(records)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetMealsInput {
    pub patient_hash: ActionHash,
    pub start_date: i64,
    pub end_date: i64,
}

/// Get daily nutrition summary
#[hdk_extern]
pub fn get_daily_nutrition_summary(input: DailySummaryInput) -> ExternResult<DailyNutritionSummary> {
    // Get meals for the day
    let day_start = input.date;
    let day_end = input.date + 86_400_000_000; // 24 hours in microseconds

    let meals = get_patient_meals(GetMealsInput {
        patient_hash: input.patient_hash.clone(),
        start_date: day_start,
        end_date: day_end,
    })?;

    let mut total_calories = 0u32;
    let mut total_protein = 0.0f64;
    let mut total_carbs = 0.0f64;
    let mut total_fat = 0.0f64;
    let mut total_fiber = 0.0f64;
    let mut total_sodium = 0u32;
    let mut meal_count = 0u32;
    let mut restriction_violations = Vec::new();

    for record in &meals {
        if let Some(meal) = record
            .entry()
            .to_app_option::<MealLog>()
            .ok()
            .flatten()
        {
            meal_count += 1;
            total_calories += meal.total_calories.unwrap_or(0);
            total_protein += meal.total_protein_g.unwrap_or(0.0);
            total_carbs += meal.total_carbs_g.unwrap_or(0.0);
            total_fat += meal.total_fat_g.unwrap_or(0.0);
            total_fiber += meal.total_fiber_g.unwrap_or(0.0);
            total_sodium += meal.total_sodium_mg.unwrap_or(0);

            // Collect restriction violations
            restriction_violations.extend(meal.flagged_restrictions.clone());
        }
    }

    // Get goal progress
    let goals = get_patient_goals(input.patient_hash)?;
    let goal_progress = if let Some(goal_record) = goals.first() {
        if let Some(goal) = goal_record
            .entry()
            .to_app_option::<NutritionGoal>()
            .ok()
            .flatten()
        {
            Some(GoalProgress {
                calories_progress: goal
                    .target_calories
                    .map(|t| (total_calories as f64 / t as f64 * 100.0) as u32),
                protein_progress: goal
                    .target_protein_g
                    .map(|t| (total_protein / t as f64 * 100.0) as u32),
                fiber_progress: goal
                    .target_fiber_g
                    .map(|t| (total_fiber / t as f64 * 100.0) as u32),
                sodium_progress: goal
                    .target_sodium_mg
                    .map(|t| (total_sodium as f64 / t as f64 * 100.0) as u32),
            })
        } else {
            None
        }
    } else {
        None
    };

    Ok(DailyNutritionSummary {
        total_calories,
        total_protein_g: total_protein,
        total_carbs_g: total_carbs,
        total_fat_g: total_fat,
        total_fiber_g: total_fiber,
        total_sodium_mg: total_sodium,
        meal_count,
        restriction_violations,
        goal_progress,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DailySummaryInput {
    pub patient_hash: ActionHash,
    pub date: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DailyNutritionSummary {
    pub total_calories: u32,
    pub total_protein_g: f64,
    pub total_carbs_g: f64,
    pub total_fat_g: f64,
    pub total_fiber_g: f64,
    pub total_sodium_mg: u32,
    pub meal_count: u32,
    pub restriction_violations: Vec<String>,
    pub goal_progress: Option<GoalProgress>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GoalProgress {
    pub calories_progress: Option<u32>,
    pub protein_progress: Option<u32>,
    pub fiber_progress: Option<u32>,
    pub sodium_progress: Option<u32>,
}

// ============================================================================
// Nutrition Recommendation Functions
// ============================================================================

/// Get patient's active recommendations
#[hdk_extern]
pub fn get_patient_recommendations(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        false,
    )?;
    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToRecommendations)?,
        GetStrategy::default(),
    )?;

    let mut records = Vec::new();
    let now = sys_time()?.as_micros();

    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(rec) = record
                    .entry()
                    .to_app_option::<NutritionRecommendation>()
                    .ok()
                    .flatten()
                {
                    // Filter for non-expired, non-acknowledged
                    let not_expired = rec
                        .expires_at
                        .map(|e| e.as_micros() > now)
                        .unwrap_or(true);
                    if not_expired && !rec.acknowledged {
                        records.push(record);
                    }
                }
            }
        }
    }

    if !records.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::All],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(records)
}

/// Create a nutrition recommendation
#[hdk_extern]
pub fn create_recommendation(rec: NutritionRecommendation) -> ExternResult<Record> {
    let auth = require_authorization(
        rec.patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;
    let action_hash = create_entry(&EntryTypes::NutritionRecommendation(rec.clone()))?;

    create_link(
        rec.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToRecommendations,
        (),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get created recommendation".into())))?;

    log_data_access(
        rec.patient_hash,
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Acknowledge a recommendation
#[hdk_extern]
pub fn acknowledge_recommendation(rec_hash: ActionHash) -> ExternResult<Record> {
    let record = get(rec_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Recommendation not found".into())))?;

    let mut rec: NutritionRecommendation = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid recommendation entry".into())))?;

    rec.acknowledged = true;
    rec.acknowledged_at = Some(sys_time()?);

    let auth = require_authorization(
        rec.patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;

    let updated_hash = update_entry(rec_hash, &rec)?;

    let updated_record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get updated recommendation".into())))?;

    log_data_access(
        rec.patient_hash,
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(updated_record)
}

// ============================================================================
// SDOH Food Security Integration
// ============================================================================

/// Input for generating recommendations from SDOH food security screening
#[derive(Serialize, Deserialize, Debug)]
pub struct SdohFoodSecurityInput {
    pub patient_hash: ActionHash,
    /// Food security level from SDOH screening (High, Marginal, Low, VeryLow)
    pub security_level: String,
    /// Whether patient has access to healthy food
    pub access_to_healthy_food: bool,
    /// Whether patient has transportation to grocery
    pub has_transportation: bool,
    /// Affordability score (0-100)
    pub affordability_score: u8,
    /// Barriers to healthy eating
    pub barriers: Vec<String>,
    /// Optional SDOH screening hash for linking
    pub sdoh_screening_hash: Option<ActionHash>,
}

/// Output containing generated recommendations
#[derive(Serialize, Deserialize, Debug)]
pub struct SdohNutritionRecommendationsOutput {
    pub recommendations_created: u32,
    pub recommendation_hashes: Vec<ActionHash>,
}

/// Generate nutrition recommendations based on SDOH food security screening
///
/// This function bridges the SDOH module with nutrition recommendations,
/// automatically creating relevant recommendations when food insecurity
/// or barriers to healthy eating are identified.
#[hdk_extern]
pub fn generate_recommendations_from_sdoh(
    input: SdohFoodSecurityInput,
) -> ExternResult<SdohNutritionRecommendationsOutput> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;
    let mut recommendations_created = 0;
    let mut recommendation_hashes = Vec::new();
    let now = sys_time()?;

    // Generate recommendations based on food security level
    match input.security_level.as_str() {
        "VeryLow" | "Low" => {
            // High priority: Food assistance programs
            let rec = NutritionRecommendation {
                recommendation_id: format!("SDOH-FA-{}", now.as_micros()),
                patient_hash: input.patient_hash.clone(),
                source: RecommendationSource::System,
                source_hash: input.sdoh_screening_hash.clone(),
                recommendation_type: RecommendationType::General,
                title: "Food Assistance Program Referral".to_string(),
                description: "Based on your screening, you may be eligible for food assistance programs like SNAP (food stamps), WIC, or local food banks. These programs can help ensure you have access to nutritious food.".to_string(),
                rationale: Some(format!("Food security screening indicated {} level", input.security_level)),
                linked_conditions: vec!["Food Insecurity".to_string()],
                linked_medications: vec![],
                priority: if input.security_level == "VeryLow" {
                    RecommendationPriority::Critical
                } else {
                    RecommendationPriority::High
                },
                created_at: now.clone(),
                expires_at: None,
                acknowledged: false,
                acknowledged_at: None,
            };

            let hash = create_entry(&EntryTypes::NutritionRecommendation(rec.clone()))?;
            create_link(
                input.patient_hash.clone(),
                hash.clone(),
                LinkTypes::PatientToRecommendations,
                (),
            )?;
            recommendation_hashes.push(hash);
            recommendations_created += 1;

            // Budget-friendly nutrition tips
            let budget_rec = NutritionRecommendation {
                recommendation_id: format!("SDOH-BN-{}", now.as_micros()),
                patient_hash: input.patient_hash.clone(),
                source: RecommendationSource::System,
                source_hash: input.sdoh_screening_hash.clone(),
                recommendation_type: RecommendationType::MealPlan,
                title: "Budget-Friendly Nutrition Guide".to_string(),
                description: "Nutritious eating on a budget: Focus on beans, lentils, eggs, frozen vegetables, oatmeal, and in-season produce. These foods provide excellent nutrition at lower cost.".to_string(),
                rationale: Some("Affordability is a barrier to healthy eating".to_string()),
                linked_conditions: vec!["Food Insecurity".to_string()],
                linked_medications: vec![],
                priority: RecommendationPriority::High,
                created_at: now.clone(),
                expires_at: None,
                acknowledged: false,
                acknowledged_at: None,
            };

            let hash = create_entry(&EntryTypes::NutritionRecommendation(budget_rec.clone()))?;
            create_link(
                input.patient_hash.clone(),
                hash.clone(),
                LinkTypes::PatientToRecommendations,
                (),
            )?;
            recommendation_hashes.push(hash);
            recommendations_created += 1;
        }
        "Marginal" => {
            // Medium priority: Preventive guidance
            let rec = NutritionRecommendation {
                recommendation_id: format!("SDOH-PG-{}", now.as_micros()),
                patient_hash: input.patient_hash.clone(),
                source: RecommendationSource::System,
                source_hash: input.sdoh_screening_hash.clone(),
                recommendation_type: RecommendationType::General,
                title: "Nutritional Planning Resources".to_string(),
                description: "Consider meal planning and batch cooking to maximize food value. Local community resources may also be available to help with food access.".to_string(),
                rationale: Some("Marginal food security identified - preventive intervention".to_string()),
                linked_conditions: vec![],
                linked_medications: vec![],
                priority: RecommendationPriority::Medium,
                created_at: now.clone(),
                expires_at: None,
                acknowledged: false,
                acknowledged_at: None,
            };

            let hash = create_entry(&EntryTypes::NutritionRecommendation(rec.clone()))?;
            create_link(
                input.patient_hash.clone(),
                hash.clone(),
                LinkTypes::PatientToRecommendations,
                (),
            )?;
            recommendation_hashes.push(hash);
            recommendations_created += 1;
        }
        _ => {} // High security level - no specific recommendations needed
    }

    // Transportation barrier recommendation
    if !input.has_transportation && !input.access_to_healthy_food {
        let rec = NutritionRecommendation {
            recommendation_id: format!("SDOH-TR-{}", now.as_micros()),
            patient_hash: input.patient_hash.clone(),
            source: RecommendationSource::System,
            source_hash: input.sdoh_screening_hash.clone(),
            recommendation_type: RecommendationType::General,
            title: "Food Delivery and Transportation Options".to_string(),
            description: "Many food assistance programs offer delivery services. Grocery delivery services, community shuttles, or volunteer driver programs may also help with food access.".to_string(),
            rationale: Some("Transportation barrier to healthy food access identified".to_string()),
            linked_conditions: vec!["Transportation Barrier".to_string()],
            linked_medications: vec![],
            priority: RecommendationPriority::High,
            created_at: now.clone(),
            expires_at: None,
            acknowledged: false,
            acknowledged_at: None,
        };

        let hash = create_entry(&EntryTypes::NutritionRecommendation(rec.clone()))?;
        create_link(
            input.patient_hash.clone(),
            hash.clone(),
            LinkTypes::PatientToRecommendations,
            (),
        )?;
        recommendation_hashes.push(hash);
        recommendations_created += 1;
    }

    // Generate barrier-specific recommendations
    for barrier in &input.barriers {
        let barrier_lower = barrier.to_lowercase();

        if barrier_lower.contains("cooking") || barrier_lower.contains("kitchen") {
            let rec = NutritionRecommendation {
                recommendation_id: format!("SDOH-CK-{}", now.as_micros()),
                patient_hash: input.patient_hash.clone(),
                source: RecommendationSource::System,
                source_hash: input.sdoh_screening_hash.clone(),
                recommendation_type: RecommendationType::MealPlan,
                title: "No-Cook Nutrition Options".to_string(),
                description: "Nutritious foods that require no cooking: fresh fruits, vegetables with hummus, yogurt, cheese, nuts, whole grain bread with nut butter, pre-cooked rotisserie chicken.".to_string(),
                rationale: Some(format!("Barrier identified: {}", barrier)),
                linked_conditions: vec!["Cooking Limitation".to_string()],
                linked_medications: vec![],
                priority: RecommendationPriority::Medium,
                created_at: now.clone(),
                expires_at: None,
                acknowledged: false,
                acknowledged_at: None,
            };

            let hash = create_entry(&EntryTypes::NutritionRecommendation(rec.clone()))?;
            create_link(
                input.patient_hash.clone(),
                hash.clone(),
                LinkTypes::PatientToRecommendations,
                (),
            )?;
            recommendation_hashes.push(hash);
            recommendations_created += 1;
        }

        if barrier_lower.contains("time") || barrier_lower.contains("busy") {
            let rec = NutritionRecommendation {
                recommendation_id: format!("SDOH-QM-{}", now.as_micros()),
                patient_hash: input.patient_hash.clone(),
                source: RecommendationSource::System,
                source_hash: input.sdoh_screening_hash.clone(),
                recommendation_type: RecommendationType::MealPlan,
                title: "Quick Healthy Meals Guide".to_string(),
                description: "15-minute healthy meal ideas: Stir-fry with frozen vegetables, egg scrambles, grain bowls with pre-cooked ingredients, wraps with lean protein and vegetables.".to_string(),
                rationale: Some(format!("Barrier identified: {}", barrier)),
                linked_conditions: vec!["Time Constraint".to_string()],
                linked_medications: vec![],
                priority: RecommendationPriority::Medium,
                created_at: now.clone(),
                expires_at: None,
                acknowledged: false,
                acknowledged_at: None,
            };

            let hash = create_entry(&EntryTypes::NutritionRecommendation(rec.clone()))?;
            create_link(
                input.patient_hash.clone(),
                hash.clone(),
                LinkTypes::PatientToRecommendations,
                (),
            )?;
            recommendation_hashes.push(hash);
            recommendations_created += 1;
        }
    }

    if recommendations_created > 0 {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::All],
            Permission::Write,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(SdohNutritionRecommendationsOutput {
        recommendations_created,
        recommendation_hashes,
    })
}
