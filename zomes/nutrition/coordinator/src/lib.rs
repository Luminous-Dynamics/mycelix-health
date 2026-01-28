//! Nutrition Coordinator Zome
//!
//! Coordinator functions for nutrition tracking, dietary restrictions,
//! drug-food interactions, and nutrition recommendations.
//!
//! Integrates with the health-food SDK bridge module.

use hdk::prelude::*;
use nutrition_integrity::*;

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

    Ok(records)
}

/// Add a new dietary restriction
#[hdk_extern]
pub fn add_dietary_restriction(restriction: DietaryRestriction) -> ExternResult<Record> {
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

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get created restriction".into())))
}

/// Update a dietary restriction
#[hdk_extern]
pub fn update_dietary_restriction(input: UpdateRestrictionInput) -> ExternResult<Record> {
    update_entry(input.original_hash.clone(), &input.updated_restriction)?;

    get(input.original_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get updated restriction".into())))
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

    update_entry(restriction_hash.clone(), &restriction)?;

    get(restriction_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get updated restriction".into())))
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

    Ok(FoodSafetyResult {
        safe: contraindicated.is_empty(),
        warnings,
        contraindicated,
    })
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
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToGoals)?,
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

    Ok(records)
}

/// Create a nutrition goal
#[hdk_extern]
pub fn create_nutrition_goal(goal: NutritionGoal) -> ExternResult<Record> {
    let action_hash = create_entry(&EntryTypes::NutritionGoal(goal.clone()))?;

    create_link(
        goal.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToGoals,
        (),
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get created goal".into())))
}

/// Update a nutrition goal
#[hdk_extern]
pub fn update_nutrition_goal(input: UpdateGoalInput) -> ExternResult<Record> {
    update_entry(input.original_hash.clone(), &input.updated_goal)?;

    get(input.original_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get updated goal".into())))
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
    let action_hash = create_entry(&EntryTypes::MealLog(meal.clone()))?;

    // Link from patient
    create_link(
        meal.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToMeals,
        (),
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get created meal log".into())))
}

/// Get patient's meal logs for a date range
#[hdk_extern]
pub fn get_patient_meals(input: GetMealsInput) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(input.patient_hash, LinkTypes::PatientToMeals)?,
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
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToRecommendations)?,
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

    Ok(records)
}

/// Create a nutrition recommendation
#[hdk_extern]
pub fn create_recommendation(rec: NutritionRecommendation) -> ExternResult<Record> {
    let action_hash = create_entry(&EntryTypes::NutritionRecommendation(rec.clone()))?;

    create_link(
        rec.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToRecommendations,
        (),
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get created recommendation".into())))
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

    update_entry(rec_hash.clone(), &rec)?;

    get(rec_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get updated recommendation".into())))
}
