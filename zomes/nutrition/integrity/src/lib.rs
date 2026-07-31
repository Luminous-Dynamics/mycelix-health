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
            OpEntry::CreateEntry { app_entry, action } => validate_create_entry(action, app_entry),
            OpEntry::UpdateEntry {
                app_entry, action, ..
            } => validate_update_entry(action, app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // Previously left fully permissive via the catch-all `_` arm --
        // the 17th confirmed instance of the wide-open RegisterUpdate/
        // RegisterDelete bug this pass. Found + fixed 2026-07-09 during
        // the P0 author-binding pass. This coordinator never calls
        // delete_link/delete_entry (confirmed via grep), so the
        // RegisterDeleteLink/RegisterDelete hardening below is pure
        // defense-in-depth.
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
        // No agent-identity field: diagnosed_by/verified_by are
        // Option<ActionHash> references to other records, not a
        // directly comparable AgentPubKey. Case (a).
        EntryTypes::DietaryRestriction(r) => validate_restriction(&r),
        EntryTypes::DrugFoodInteraction(i) => {
            // Author-binding: the coordinator's add_drug_food_interaction
            // previously took the FULL struct straight from caller input
            // with ZERO derivation from agent_info() -- any agent could
            // forge a victim as created_by. Found + fixed 2026-07-09
            // during the P0 author-binding pass (coordinator-side fix
            // applied alongside this).
            if i.created_by != action.author {
                return Ok(ValidateCallbackResult::Invalid(
                    "created_by must correspond to the committing agent".into(),
                ));
            }
            validate_interaction(&i)
        }
        // No agent-identity field: prescribed_by is Option<ActionHash>.
        EntryTypes::NutritionGoal(g) => validate_goal(&g),
        // No agent-identity field.
        EntryTypes::MealLog(m) => validate_meal_log(&m),
        // No agent-identity field: source_hash is Option<ActionHash>.
        EntryTypes::NutritionRecommendation(r) => validate_recommendation(&r),
    }
}

/// DietaryRestriction and NutritionGoal both have a live, intentionally
/// broad "edit almost anything" update flow -- the coordinator itself
/// only guards patient_hash from changing (client-side only, previously
/// bypassable). NutritionRecommendation's only update path
/// (acknowledge_recommendation) is narrow (acknowledged/acknowledged_at
/// only). MealLog and DrugFoodInteraction have no live update call at
/// all (confirmed via grep for `update_entry`) and are made immutable.
/// Reviewed 2026-07-09 during the P0 author-binding pass: this isn't an
/// author-forgery finding for these types (no identity field exists),
/// but the previous unconditional `Ok(Valid)` meant a modified
/// coordinator could silently re-point ANY of these at a different
/// patient via patient_hash -- a genuine patient-record-mismatch vector.
fn validate_update_entry(
    action: Update,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::DietaryRestriction(r) => {
            let original_record = must_get_valid_record(action.original_action_address.clone())?;
            let original: DietaryRestriction = original_record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "Original restriction not found".into()
                )))?;
            if r.patient_hash != original.patient_hash {
                return Ok(ValidateCallbackResult::Invalid(
                    "patient_hash cannot change on a restriction update".into(),
                ));
            }
            validate_restriction(&r)
        }
        EntryTypes::NutritionGoal(g) => {
            let original_record = must_get_valid_record(action.original_action_address.clone())?;
            let original: NutritionGoal = original_record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "Original goal not found".into()
                )))?;
            if g.patient_hash != original.patient_hash {
                return Ok(ValidateCallbackResult::Invalid(
                    "patient_hash cannot change on a goal update".into(),
                ));
            }
            validate_goal(&g)
        }
        EntryTypes::NutritionRecommendation(r) => {
            let original_record = must_get_valid_record(action.original_action_address.clone())?;
            let original: NutritionRecommendation = original_record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "Original recommendation not found".into()
                )))?;
            if r.recommendation_id != original.recommendation_id
                || r.patient_hash != original.patient_hash
                || r.source != original.source
                || r.source_hash != original.source_hash
                || r.recommendation_type != original.recommendation_type
                || r.title != original.title
                || r.description != original.description
                || r.rationale != original.rationale
                || r.linked_conditions != original.linked_conditions
                || r.linked_medications != original.linked_medications
                || r.priority != original.priority
                || r.created_at != original.created_at
                || r.expires_at != original.expires_at
            {
                return Ok(ValidateCallbackResult::Invalid(
                    "Only acknowledged/acknowledged_at can change on a recommendation update"
                        .into(),
                ));
            }
            Ok(ValidateCallbackResult::Valid)
        }
        EntryTypes::MealLog(_) => Ok(ValidateCallbackResult::Invalid(
            "Meal logs are immutable".into(),
        )),
        EntryTypes::DrugFoodInteraction(_) => Ok(ValidateCallbackResult::Invalid(
            "Drug-food interactions are immutable".into(),
        )),
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

    fn valid_interaction(created_by: AgentPubKey) -> DrugFoodInteraction {
        DrugFoodInteraction {
            interaction_id: "i-1".into(),
            medication_name: "warfarin".into(),
            medication_rxcui: None,
            food_category: FoodCategory::Other,
            specific_foods: vec![],
            interaction_type: InteractionType::MonitorClosely,
            severity: InteractionSeverity::Moderate,
            description: "vitamin K interaction".into(),
            mechanism: None,
            clinical_effect: None,
            recommendation: "monitor INR".into(),
            evidence_level: EvidenceLevel::Established,
            sources: vec![],
            created_by,
            created_at: Timestamp::from_micros(0),
            updated_at: Timestamp::from_micros(0),
        }
    }

    #[test]
    fn create_interaction_valid_when_creator_matches_committer() {
        let author = me();
        let i = valid_interaction(author.clone());
        let result =
            validate_create_entry(create_action(author), EntryTypes::DrugFoodInteraction(i))
                .unwrap();
        assert_eq!(result, ValidateCallbackResult::Valid);
    }

    #[test]
    fn create_interaction_forgery_rejected() {
        let i = valid_interaction(me());
        let result = validate_create_entry(
            create_action(other_agent()),
            EntryTypes::DrugFoodInteraction(i),
        )
        .unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn update_entry_rejects_drug_food_interaction_update() {
        let i = valid_interaction(me());
        let result =
            validate_update_entry(update_action(me()), EntryTypes::DrugFoodInteraction(i)).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn update_entry_rejects_meal_log_update() {
        let m = MealLog {
            log_id: "l-1".into(),
            patient_hash: ActionHash::from_raw_36(vec![2u8; 36]),
            meal_type: MealType::Lunch,
            timestamp: Timestamp::from_micros(0),
            foods: vec![],
            total_calories: None,
            total_protein_g: None,
            total_carbs_g: None,
            total_fat_g: None,
            total_fiber_g: None,
            total_sodium_mg: None,
            notes: None,
            photo_hash: None,
            location: None,
            flagged_restrictions: vec![],
            created_at: Timestamp::from_micros(0),
        };
        let result = validate_update_entry(update_action(me()), EntryTypes::MealLog(m)).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }
}
