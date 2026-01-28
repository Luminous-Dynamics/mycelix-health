//! Nutrition Integrity Zome
//!
//! Entry types and validation for nutrition tracking, dietary restrictions,
//! drug-food interactions, and nutrition recommendations.
//!
//! This zome complements the health-food SDK integration module.

use hdi::prelude::*;

// ============================================================================
// Dietary Restriction Types
// ============================================================================

/// Types of dietary restrictions
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DietaryRestrictionType {
    /// Immune-mediated allergic reaction (IgE)
    Allergy,
    /// Non-immune intolerance (lactose, etc.)
    Intolerance,
    /// Disease-related restriction (celiac, PKU)
    MedicalCondition,
    /// Medication-related restriction
    DrugInteraction,
    /// Faith-based restriction
    Religious,
    /// Personal choice (vegan, etc.)
    Ethical,
    /// Other restriction type
    Other,
}

/// Severity levels for dietary restrictions
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RestrictionSeverity {
    /// Anaphylaxis risk - life threatening
    LifeThreatening,
    /// Serious reaction
    Severe,
    /// Significant discomfort
    Moderate,
    /// Minor symptoms
    Mild,
    /// No physical reaction - preference only
    Preference,
}

/// Food categories for restriction matching
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FoodCategory {
    Dairy,
    Eggs,
    Fish,
    Shellfish,
    TreeNuts,
    Peanuts,
    Wheat,
    Soy,
    Sesame,
    Gluten,
    Lactose,
    Fructose,
    Sulfites,
    Nightshades,
    Citrus,
    Meat,
    Pork,
    Alcohol,
    Caffeine,
    Other,
}

/// A dietary restriction for a patient
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DietaryRestriction {
    pub restriction_id: String,
    pub patient_hash: ActionHash,
    pub restriction_type: DietaryRestrictionType,
    pub severity: RestrictionSeverity,
    pub food_category: FoodCategory,
    pub specific_foods: Vec<String>,
    pub clinical_notes: Option<String>,
    pub diagnosed_by: Option<ActionHash>,
    pub diagnosed_at: Option<Timestamp>,
    pub verified_by: Option<ActionHash>,
    pub verified_at: Option<Timestamp>,
    pub linked_allergy_hash: Option<ActionHash>,
    pub linked_condition_hash: Option<ActionHash>,
    pub linked_medication_hash: Option<ActionHash>,
    pub active: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

// ============================================================================
// Drug-Food Interaction Types
// ============================================================================

/// Interaction type for drug-food combinations
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InteractionType {
    /// Must avoid completely
    Avoid,
    /// Limit consumption
    Limit,
    /// Separate timing (take medication X hours from food)
    TimeSeparate,
    /// Monitor closely when combined
    MonitorClosely,
}

/// Severity of drug-food interaction
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InteractionSeverity {
    /// Must not be combined
    Contraindicated,
    /// Significant clinical effect
    Major,
    /// Moderate clinical effect
    Moderate,
    /// Minor clinical effect
    Minor,
}

/// Evidence level for interaction
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EvidenceLevel {
    /// Well-documented in literature
    Established,
    /// Highly probable based on evidence
    Probable,
    /// Suspected but limited data
    Suspected,
    /// Theoretical based on mechanism
    Theoretical,
}

/// A drug-food interaction entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DrugFoodInteraction {
    pub interaction_id: String,
    pub medication_name: String,
    pub medication_rxcui: Option<String>,
    pub food_category: FoodCategory,
    pub specific_foods: Vec<String>,
    pub interaction_type: InteractionType,
    pub severity: InteractionSeverity,
    pub description: String,
    pub mechanism: Option<String>,
    pub clinical_effect: Option<String>,
    pub recommendation: String,
    pub evidence_level: EvidenceLevel,
    pub sources: Vec<String>,
    pub created_by: AgentPubKey,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

// ============================================================================
// Nutrition Goal Types
// ============================================================================

/// Type of nutrition goal
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NutritionGoalType {
    WeightManagement,
    GlucoseControl,
    HeartHealth,
    RenalDiet,
    GIHealth,
    General,
}

/// A nutrition goal for a patient
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct NutritionGoal {
    pub goal_id: String,
    pub patient_hash: ActionHash,
    pub goal_type: NutritionGoalType,
    pub target_calories: Option<u32>,
    pub target_protein_g: Option<u32>,
    pub target_carbs_g: Option<u32>,
    pub target_fat_g: Option<u32>,
    pub target_fiber_g: Option<u32>,
    pub target_sodium_mg: Option<u32>,
    pub target_potassium_mg: Option<u32>,
    pub restrictions: Vec<String>,
    pub prescribed_by: Option<ActionHash>,
    pub start_date: Timestamp,
    pub end_date: Option<Timestamp>,
    pub notes: Option<String>,
    pub active: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

// ============================================================================
// Meal Logging Types
// ============================================================================

/// Type of meal
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MealType {
    Breakfast,
    Lunch,
    Dinner,
    Snack,
    Supplement,
}

/// A food item in a meal
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MealItem {
    pub name: String,
    pub quantity: f64,
    pub unit: String,
    pub calories: Option<u32>,
    pub protein_g: Option<f64>,
    pub carbs_g: Option<f64>,
    pub fat_g: Option<f64>,
    pub fiber_g: Option<f64>,
    pub sodium_mg: Option<u32>,
    pub categories: Vec<FoodCategory>,
    pub barcode: Option<String>,
    pub brand_name: Option<String>,
}

/// A meal log entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MealLog {
    pub log_id: String,
    pub patient_hash: ActionHash,
    pub meal_type: MealType,
    pub timestamp: Timestamp,
    pub foods: Vec<MealItem>,
    pub total_calories: Option<u32>,
    pub total_protein_g: Option<f64>,
    pub total_carbs_g: Option<f64>,
    pub total_fat_g: Option<f64>,
    pub total_fiber_g: Option<f64>,
    pub total_sodium_mg: Option<u32>,
    pub notes: Option<String>,
    pub photo_hash: Option<String>,
    pub location: Option<String>,
    pub flagged_restrictions: Vec<String>,
    pub created_at: Timestamp,
}

// ============================================================================
// Nutrition Recommendation Types
// ============================================================================

/// Source of nutrition recommendation
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RecommendationSource {
    Provider,
    AI,
    HealthTwin,
    System,
}

/// Type of nutrition recommendation
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RecommendationType {
    MealPlan,
    FoodSwap,
    Supplement,
    Avoidance,
    Timing,
    Portion,
    General,
}

/// Priority level for recommendation
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Critical,
    High,
    Medium,
    Low,
}

/// A nutrition recommendation
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct NutritionRecommendation {
    pub recommendation_id: String,
    pub patient_hash: ActionHash,
    pub source: RecommendationSource,
    pub source_hash: Option<ActionHash>,
    pub recommendation_type: RecommendationType,
    pub title: String,
    pub description: String,
    pub rationale: Option<String>,
    pub linked_conditions: Vec<String>,
    pub linked_medications: Vec<String>,
    pub priority: RecommendationPriority,
    pub created_at: Timestamp,
    pub expires_at: Option<Timestamp>,
    pub acknowledged: bool,
    pub acknowledged_at: Option<Timestamp>,
}

// ============================================================================
// Entry Types and Link Types
// ============================================================================

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    DietaryRestriction(DietaryRestriction),
    DrugFoodInteraction(DrugFoodInteraction),
    NutritionGoal(NutritionGoal),
    MealLog(MealLog),
    NutritionRecommendation(NutritionRecommendation),
}

#[hdk_link_types]
pub enum LinkTypes {
    /// Patient to their dietary restrictions
    PatientToRestrictions,
    /// Patient to their nutrition goals
    PatientToGoals,
    /// Patient to their meal logs
    PatientToMeals,
    /// Patient to their recommendations
    PatientToRecommendations,
    /// Food category to interactions
    FoodCategoryToInteractions,
    /// Medication to interactions
    MedicationToInteractions,
    /// Restriction to linked allergy
    RestrictionToAllergy,
    /// Goal to meal logs (tracking)
    GoalToMeals,
    /// All interactions index
    AllInteractions,
}

// ============================================================================
// Validation
// ============================================================================

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
        EntryTypes::DietaryRestriction(r) => validate_restriction(&r),
        EntryTypes::DrugFoodInteraction(i) => validate_interaction(&i),
        EntryTypes::NutritionGoal(g) => validate_goal(&g),
        EntryTypes::MealLog(m) => validate_meal_log(&m),
        EntryTypes::NutritionRecommendation(r) => validate_recommendation(&r),
    }
}

fn validate_restriction(r: &DietaryRestriction) -> ExternResult<ValidateCallbackResult> {
    if r.restriction_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Restriction ID is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_interaction(i: &DrugFoodInteraction) -> ExternResult<ValidateCallbackResult> {
    if i.interaction_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Interaction ID is required".to_string(),
        ));
    }
    if i.medication_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Medication name is required".to_string(),
        ));
    }
    if i.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Description is required".to_string(),
        ));
    }
    if i.recommendation.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Recommendation is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_goal(g: &NutritionGoal) -> ExternResult<ValidateCallbackResult> {
    if g.goal_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Goal ID is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_meal_log(m: &MealLog) -> ExternResult<ValidateCallbackResult> {
    if m.log_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Log ID is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_recommendation(r: &NutritionRecommendation) -> ExternResult<ValidateCallbackResult> {
    if r.recommendation_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Recommendation ID is required".to_string(),
        ));
    }
    if r.title.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Title is required".to_string(),
        ));
    }
    if r.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Description is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
