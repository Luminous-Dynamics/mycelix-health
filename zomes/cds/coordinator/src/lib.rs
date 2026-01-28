//! Clinical Decision Support (CDS) Coordinator Zome
//!
//! Provides extern functions for clinical decision support including:
//! - Drug interaction checking
//! - Clinical alert management
//! - Guideline compliance tracking
//!
//! All data access enforces consent-based access control.

use hdk::prelude::*;
use cds_integrity::*;
use mycelix_health_shared::{
    require_authorization, log_data_access,
    DataCategory, Permission, anchor_hash,
};

// ============================================================================
// Drug Interaction Functions
// ============================================================================

/// Create a new drug interaction record
#[hdk_extern]
pub fn create_drug_interaction(interaction: DrugInteraction) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::DrugInteraction(interaction.clone()))?;
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find drug interaction".to_string())))?;

    // Link from both drugs to this interaction for efficient lookup
    let drug_a_anchor = anchor_hash(&format!("drug_{}", interaction.drug_a_rxnorm))?;
    let drug_b_anchor = anchor_hash(&format!("drug_{}", interaction.drug_b_rxnorm))?;

    create_link(drug_a_anchor, hash.clone(), LinkTypes::DrugToInteractions, ())?;
    create_link(drug_b_anchor, hash.clone(), LinkTypes::DrugToInteractions, ())?;

    // Link to all interactions anchor
    let all_anchor = anchor_hash("all_drug_interactions")?;
    create_link(all_anchor, hash, LinkTypes::AllDrugInteractions, ())?;

    Ok(record)
}

/// Input for checking drug interactions
#[derive(Serialize, Deserialize, Debug)]
pub struct CheckDrugInteractionsInput {
    pub medication_rxnorm_codes: Vec<String>,
}

/// Check for interactions between a list of medications
#[hdk_extern]
pub fn check_drug_interactions(input: CheckDrugInteractionsInput) -> ExternResult<Vec<FoundInteraction>> {
    let mut found_interactions = Vec::new();

    // Check each pair of medications
    for i in 0..input.medication_rxnorm_codes.len() {
        for j in (i + 1)..input.medication_rxnorm_codes.len() {
            let drug_a = &input.medication_rxnorm_codes[i];
            let drug_b = &input.medication_rxnorm_codes[j];

            // Look up interactions for drug_a
            let drug_anchor = anchor_hash(&format!("drug_{}", drug_a))?;
            let links = get_links(
                LinkQuery::try_new(drug_anchor, LinkTypes::DrugToInteractions)?, GetStrategy::default())?;

            for link in links {
                if let Some(hash) = link.target.into_action_hash() {
                    if let Some(record) = get(hash, GetOptions::default())? {
                        if let Some(interaction) = record.entry().to_app_option::<DrugInteraction>().ok().flatten() {
                            // Check if this interaction involves both drugs
                            if (interaction.drug_a_rxnorm == *drug_a && interaction.drug_b_rxnorm == *drug_b)
                                || (interaction.drug_a_rxnorm == *drug_b && interaction.drug_b_rxnorm == *drug_a)
                            {
                                found_interactions.push(FoundInteraction {
                                    drug_a_rxnorm: interaction.drug_a_rxnorm,
                                    drug_a_name: interaction.drug_a_name,
                                    drug_b_rxnorm: interaction.drug_b_rxnorm,
                                    drug_b_name: interaction.drug_b_name,
                                    severity: interaction.severity,
                                    description: interaction.description,
                                    management: interaction.management,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(found_interactions)
}

/// Create a drug-allergy interaction record
#[hdk_extern]
pub fn create_drug_allergy_interaction(interaction: DrugAllergyInteraction) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::DrugAllergyInteraction(interaction.clone()))?;
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find drug allergy interaction".to_string())))?;

    // Link from drug to allergy interaction
    let drug_anchor = anchor_hash(&format!("drug_{}", interaction.drug_rxnorm))?;
    create_link(drug_anchor, hash, LinkTypes::DrugToAllergyInteractions, ())?;

    Ok(record)
}

/// Input for checking drug-allergy conflicts
#[derive(Serialize, Deserialize, Debug)]
pub struct CheckAllergyConflictsInput {
    pub medication_rxnorm_codes: Vec<String>,
    pub patient_allergies: Vec<String>,
}

/// Check for conflicts between medications and patient allergies
#[hdk_extern]
pub fn check_allergy_conflicts(input: CheckAllergyConflictsInput) -> ExternResult<Vec<FoundAllergyConflict>> {
    let mut conflicts = Vec::new();

    for drug_code in &input.medication_rxnorm_codes {
        let drug_anchor = anchor_hash(&format!("drug_{}", drug_code))?;
        let links = get_links(
            LinkQuery::try_new(drug_anchor, LinkTypes::DrugToAllergyInteractions)?, GetStrategy::default())?;

        for link in links {
            if let Some(hash) = link.target.into_action_hash() {
                if let Some(record) = get(hash, GetOptions::default())? {
                    if let Some(interaction) = record.entry().to_app_option::<DrugAllergyInteraction>().ok().flatten() {
                        // Check if patient has this allergen or cross-reactive allergen
                        for allergy in &input.patient_allergies {
                            if interaction.allergen_class.to_lowercase().contains(&allergy.to_lowercase())
                                || interaction.cross_reactive_allergens.iter().any(|a| a.to_lowercase().contains(&allergy.to_lowercase()))
                            {
                                conflicts.push(FoundAllergyConflict {
                                    drug_rxnorm: interaction.drug_rxnorm.clone(),
                                    drug_name: interaction.drug_name.clone(),
                                    allergen: allergy.clone(),
                                    cross_reactivity: interaction.allergen_class.clone(),
                                    severity: interaction.severity.clone(),
                                    recommendation: format!("Consider alternative medication. {}", interaction.notes),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(conflicts)
}

// ============================================================================
// Clinical Alert Functions
// ============================================================================

/// Input for creating a clinical alert
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateAlertInput {
    pub patient_hash: ActionHash,
    pub alert_type: AlertType,
    pub priority: AlertPriority,
    pub category: AlertCategory,
    pub message: String,
    pub details: Option<String>,
    pub trigger: String,
    pub recommended_actions: Vec<String>,
    pub expires_at: Option<Timestamp>,
    pub related_data: Vec<ActionHash>,
}

/// Create a clinical alert for a patient
#[hdk_extern]
pub fn create_clinical_alert(input: CreateAlertInput) -> ExternResult<Record> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;
    let alert = ClinicalAlert {
        alert_id: format!("ALERT-{}", sys_time()?.as_micros()),
        patient_hash: input.patient_hash.clone(),
        alert_type: input.alert_type,
        priority: input.priority,
        category: input.category,
        message: input.message,
        details: input.details,
        trigger: input.trigger,
        recommended_actions: input.recommended_actions,
        acknowledged: false,
        acknowledged_by: None,
        acknowledged_at: None,
        acknowledgment_notes: None,
        resolved: false,
        resolution_notes: None,
        created_at: sys_time()?,
        expires_at: input.expires_at,
        related_data: input.related_data,
    };

    let hash = create_entry(&EntryTypes::ClinicalAlert(alert))?;
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find clinical alert".to_string())))?;

    // Link from patient to alert
    create_link(
        input.patient_hash.clone(),
        hash,
        LinkTypes::PatientToAlerts,
        (),
    )?;

    log_data_access(
        input.patient_hash,
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Input for getting patient alerts with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPatientAlertsInput {
    pub patient_hash: ActionHash,
    pub include_acknowledged: bool,
    pub include_resolved: bool,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get alerts for a patient with access control
#[hdk_extern]
pub fn get_patient_alerts(input: GetPatientAlertsInput) -> ExternResult<Vec<Record>> {
    // Require authorization
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToAlerts)?, GetStrategy::default())?;

    let mut alerts = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(alert) = record.entry().to_app_option::<ClinicalAlert>().ok().flatten() {
                    // Filter based on input criteria
                    let include = (input.include_acknowledged || !alert.acknowledged)
                        && (input.include_resolved || !alert.resolved);

                    if include {
                        alerts.push(record);
                    }
                }
            }
        }
    }

    // Log access
    log_data_access(
        input.patient_hash,
        vec![DataCategory::All],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(alerts)
}

/// Input for acknowledging an alert
#[derive(Serialize, Deserialize, Debug)]
pub struct AcknowledgeAlertInput {
    pub alert_hash: ActionHash,
    pub notes: Option<String>,
}

/// Acknowledge a clinical alert
#[hdk_extern]
pub fn acknowledge_alert(input: AcknowledgeAlertInput) -> ExternResult<Record> {
    let record = get(input.alert_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Alert not found".to_string())))?;

    let mut alert: ClinicalAlert = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid alert entry".to_string())))?;

    let patient_hash = alert.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;

    alert.acknowledged = true;
    alert.acknowledged_by = Some(agent_info()?.agent_initial_pubkey);
    alert.acknowledged_at = Some(sys_time()?);
    alert.acknowledgment_notes = input.notes;

    let updated_hash = update_entry(input.alert_hash.clone(), &alert)?;
    let updated_record = get(updated_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated alert".to_string())))?;

    create_link(input.alert_hash, updated_hash, LinkTypes::AlertUpdates, ())?;

    log_data_access(
        patient_hash,
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(updated_record)
}

/// Input for resolving an alert
#[derive(Serialize, Deserialize, Debug)]
pub struct ResolveAlertInput {
    pub alert_hash: ActionHash,
    pub resolution_notes: String,
}

/// Resolve a clinical alert
#[hdk_extern]
pub fn resolve_alert(input: ResolveAlertInput) -> ExternResult<Record> {
    let record = get(input.alert_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Alert not found".to_string())))?;

    let mut alert: ClinicalAlert = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid alert entry".to_string())))?;

    let patient_hash = alert.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;

    alert.resolved = true;
    alert.resolution_notes = Some(input.resolution_notes);

    let updated_hash = update_entry(input.alert_hash.clone(), &alert)?;
    let updated_record = get(updated_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated alert".to_string())))?;

    create_link(input.alert_hash, updated_hash, LinkTypes::AlertUpdates, ())?;

    log_data_access(
        patient_hash,
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(updated_record)
}

// ============================================================================
// Clinical Guideline Functions
// ============================================================================

/// Create a clinical guideline
#[hdk_extern]
pub fn create_clinical_guideline(guideline: ClinicalGuideline) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::ClinicalGuideline(guideline.clone()))?;
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find guideline".to_string())))?;

    // Link to all active guidelines anchor
    if guideline.is_active {
        let all_anchor = anchor_hash("all_active_guidelines")?;
        create_link(all_anchor, hash.clone(), LinkTypes::AllActiveGuidelines, ())?;
    }

    // Link to applicable conditions
    for condition_code in &guideline.applicable_conditions {
        let condition_anchor = anchor_hash(&format!("condition_{}", condition_code))?;
        create_link(condition_anchor, hash.clone(), LinkTypes::GuidelineToConditions, ())?;
    }

    Ok(record)
}

/// Get all active clinical guidelines
#[hdk_extern]
pub fn get_all_active_guidelines(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash("all_active_guidelines")?;
    let links = get_links(
        LinkQuery::try_new(anchor, LinkTypes::AllActiveGuidelines)?, GetStrategy::default())?;

    let mut guidelines = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                guidelines.push(record);
            }
        }
    }

    Ok(guidelines)
}

/// Input for getting guidelines for a condition
#[derive(Serialize, Deserialize, Debug)]
pub struct GetGuidelinesForConditionInput {
    pub condition_icd10: String,
}

/// Get guidelines applicable to a specific condition
#[hdk_extern]
pub fn get_guidelines_for_condition(input: GetGuidelinesForConditionInput) -> ExternResult<Vec<Record>> {
    let condition_anchor = anchor_hash(&format!("condition_{}", input.condition_icd10))?;
    let links = get_links(
        LinkQuery::try_new(condition_anchor, LinkTypes::GuidelineToConditions)?, GetStrategy::default())?;

    let mut guidelines = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                guidelines.push(record);
            }
        }
    }

    Ok(guidelines)
}

/// Create or update patient guideline status
#[hdk_extern]
pub fn update_patient_guideline_status(status: PatientGuidelineStatus) -> ExternResult<Record> {
    let auth = require_authorization(
        status.patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;
    let hash = create_entry(&EntryTypes::PatientGuidelineStatus(status.clone()))?;
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find guideline status".to_string())))?;

    // Link from patient to guideline status
    create_link(
        status.patient_hash.clone(),
        hash,
        LinkTypes::PatientToGuidelineStatuses,
        (),
    )?;

    log_data_access(
        status.patient_hash,
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Input for getting patient guideline statuses
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPatientGuidelineStatusInput {
    pub patient_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get guideline compliance status for a patient
#[hdk_extern]
pub fn get_patient_guideline_statuses(input: GetPatientGuidelineStatusInput) -> ExternResult<Vec<Record>> {
    // Require authorization
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToGuidelineStatuses)?, GetStrategy::default())?;

    let mut statuses = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                statuses.push(record);
            }
        }
    }

    // Log access
    log_data_access(
        input.patient_hash,
        vec![DataCategory::All],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(statuses)
}

// ============================================================================
// Comprehensive Interaction Check
// ============================================================================

/// Perform a comprehensive interaction check for a patient
#[hdk_extern]
pub fn perform_interaction_check(request: InteractionCheckRequest) -> ExternResult<Record> {
    let auth = require_authorization(
        request.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Write,
        false,
    )?;
    // Save the request (hash available for future use, e.g., linking request to response)
    let _request_hash = create_entry(&EntryTypes::InteractionCheckRequest(request.clone()))?;

    // Check drug-drug interactions
    let drug_interactions = check_drug_interactions(CheckDrugInteractionsInput {
        medication_rxnorm_codes: request.medication_rxnorm_codes.clone(),
    })?;

    // Check drug-allergy conflicts if requested and allergies provided
    let allergy_conflicts = if request.check_allergies && !request.patient_allergies.is_empty() {
        check_allergy_conflicts(CheckAllergyConflictsInput {
            medication_rxnorm_codes: request.medication_rxnorm_codes.clone(),
            patient_allergies: request.patient_allergies.clone(),
        })?
    } else {
        Vec::new()
    };

    // Check for duplicate therapies if requested
    let duplicate_therapies = if request.check_duplicates {
        check_duplicate_therapies(&request.medication_rxnorm_codes)?
    } else {
        Vec::new()
    };

    // Determine safety assessment based on all findings
    let safety_assessment = determine_safety_assessment(
        &drug_interactions,
        &allergy_conflicts,
        &duplicate_therapies,
    );

    // Generate recommendations from all findings
    let mut recommendations = Vec::new();

    // Add drug interaction recommendations
    for interaction in &drug_interactions {
        recommendations.push(interaction.management.clone());
    }

    // Add allergy conflict recommendations
    for conflict in &allergy_conflicts {
        recommendations.push(conflict.recommendation.clone());
    }

    // Add duplicate therapy recommendations
    for duplicate in &duplicate_therapies {
        recommendations.push(duplicate.recommendation.clone());
    }

    let response = InteractionCheckResponse {
        request_id: request.request_id,
        patient_hash: request.patient_hash.clone(),
        drug_interactions,
        allergy_conflicts,
        duplicate_therapies,
        safety_assessment,
        recommendations,
        completed_at: sys_time()?,
    };

    let response_hash = create_entry(&EntryTypes::InteractionCheckResponse(response))?;
    let response_record = get(response_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find response".to_string())))?;

    // Link from patient to interaction check
    create_link(
        request.patient_hash.clone(),
        response_hash,
        LinkTypes::PatientToInteractionChecks,
        (),
    )?;

    log_data_access(
        request.patient_hash,
        vec![DataCategory::Medications],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(response_record)
}

/// Determine overall safety assessment based on all findings
fn determine_safety_assessment(
    drug_interactions: &[FoundInteraction],
    allergy_conflicts: &[FoundAllergyConflict],
    duplicate_therapies: &[DuplicateTherapy],
) -> SafetyAssessment {
    // Check for contraindicated drug interactions
    if drug_interactions.iter().any(|i| matches!(i.severity, InteractionSeverity::Contraindicated)) {
        return SafetyAssessment::Contraindicated;
    }

    // Check for anaphylactic allergy risk (highest severity)
    if allergy_conflicts.iter().any(|c| matches!(c.severity, AllergySeverity::Anaphylactic)) {
        return SafetyAssessment::Contraindicated;
    }

    // Check for severe allergy risk
    if allergy_conflicts.iter().any(|c| matches!(c.severity, AllergySeverity::Severe)) {
        return SafetyAssessment::HighRisk;
    }

    // Check for major drug interactions
    if drug_interactions.iter().any(|i| matches!(i.severity, InteractionSeverity::Major)) {
        return SafetyAssessment::HighRisk;
    }

    // Check for moderate allergy risk
    if allergy_conflicts.iter().any(|c| matches!(c.severity, AllergySeverity::Moderate)) {
        return SafetyAssessment::CautionRecommended;
    }

    // Check for moderate drug interactions or any duplicate therapies
    if drug_interactions.iter().any(|i| matches!(i.severity, InteractionSeverity::Moderate))
        || !duplicate_therapies.is_empty()
    {
        return SafetyAssessment::CautionRecommended;
    }

    // Check for mild allergies or minor interactions
    if allergy_conflicts.iter().any(|c| matches!(c.severity, AllergySeverity::Mild))
        || drug_interactions.iter().any(|i| matches!(i.severity, InteractionSeverity::Minor))
    {
        return SafetyAssessment::CautionRecommended;
    }

    SafetyAssessment::Safe
}

// ============================================================================
// Pharmacogenomics Functions
// ============================================================================

/// Input for creating a pharmacogenomic profile
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePgxProfileInput {
    pub patient_hash: ActionHash,
    pub gene_variants: Vec<GeneVariant>,
    pub testing_source: String,
    pub lab_identifier: Option<String>,
    pub hdc_encoded_profile: Option<String>,
}

/// Create a pharmacogenomic profile for a patient
#[hdk_extern]
pub fn create_pgx_profile(input: CreatePgxProfileInput) -> ExternResult<Record> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::GeneticData,
        Permission::Write,
        false,
    )?;

    let now = sys_time()?;
    let profile = PharmacogenomicProfile {
        profile_id: format!("PGX-{}", now.as_micros()),
        patient_hash: input.patient_hash.clone(),
        gene_variants: input.gene_variants,
        hdc_encoded_profile: input.hdc_encoded_profile,
        hdc_threshold: Some(0.70), // Default similarity threshold
        testing_source: input.testing_source,
        lab_identifier: input.lab_identifier,
        test_date: Timestamp::from_micros(now.as_micros() as i64),
        last_updated: Timestamp::from_micros(now.as_micros() as i64),
        version: 1,
    };

    let hash = create_entry(&EntryTypes::PharmacogenomicProfile(profile))?;
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find PGx profile".to_string())))?;

    // Link from patient to profile
    create_link(
        input.patient_hash.clone(),
        hash,
        LinkTypes::PatientToPgxProfile,
        (),
    )?;

    log_data_access(
        input.patient_hash,
        vec![DataCategory::GeneticData],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Input for getting patient's PGx profile
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPgxProfileInput {
    pub patient_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get patient's pharmacogenomic profile
#[hdk_extern]
pub fn get_pgx_profile(input: GetPgxProfileInput) -> ExternResult<Option<Record>> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::GeneticData,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToPgxProfile)?, GetStrategy::default())?;

    let result = if let Some(link) = links.last() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            get(hash, GetOptions::default())?
        } else {
            None
        }
    } else {
        None
    };

    log_data_access(
        input.patient_hash,
        vec![DataCategory::GeneticData],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(result)
}

/// Create a drug-gene interaction record
#[hdk_extern]
pub fn create_drug_gene_interaction(interaction: DrugGeneInteraction) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::DrugGeneInteraction(interaction.clone()))?;
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find drug-gene interaction".to_string())))?;

    // Link from drug to interaction
    let drug_anchor = anchor_hash(&format!("drug_pgx_{}", interaction.drug_rxnorm))?;
    create_link(drug_anchor, hash.clone(), LinkTypes::DrugToGeneInteractions, ())?;

    // Link from gene to interaction
    let gene_anchor = anchor_hash(&format!("gene_{}", interaction.gene))?;
    create_link(gene_anchor, hash, LinkTypes::GeneToDrugInteractions, ())?;

    Ok(record)
}

/// Input for checking pharmacogenomic interactions
#[derive(Serialize, Deserialize, Debug)]
pub struct CheckPgxInteractionInput {
    pub patient_hash: ActionHash,
    pub drug_rxnorm: String,
    pub drug_name: String,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Check pharmacogenomic implications for a drug and patient
#[hdk_extern]
pub fn check_pgx_interaction(input: CheckPgxInteractionInput) -> ExternResult<PharmacogenomicCheckResult> {
    // Get patient's PGx profile
    let profile_record = get_pgx_profile(GetPgxProfileInput {
        patient_hash: input.patient_hash.clone(),
        is_emergency: input.is_emergency,
        emergency_reason: input.emergency_reason.clone(),
    })?;

    let profile = match profile_record {
        Some(rec) => rec.entry().to_app_option::<PharmacogenomicProfile>().ok().flatten(),
        None => None,
    };

    // If no profile, return "no data" result
    if profile.is_none() {
        return Ok(PharmacogenomicCheckResult {
            patient_hash: input.patient_hash,
            drug_rxnorm: input.drug_rxnorm,
            drug_name: input.drug_name,
            gene_findings: Vec::new(),
            overall_recommendation: DosingRecommendation::InsufficientEvidence,
            summary: "No pharmacogenomic profile available for this patient".to_string(),
            detailed_recommendations: vec![
                "Consider ordering pharmacogenomic testing".to_string(),
                "Proceed with standard dosing while monitoring for adverse effects".to_string(),
            ],
            confidence: 0.0,
        });
    }

    let profile = profile.unwrap();
    let mut gene_findings = Vec::new();
    let mut recommendations = Vec::new();
    let mut worst_recommendation = DosingRecommendation::StandardDose;

    // Look up drug-gene interactions for this drug
    let drug_anchor = anchor_hash(&format!("drug_pgx_{}", input.drug_rxnorm))?;
    let interaction_links = get_links(
        LinkQuery::try_new(drug_anchor, LinkTypes::DrugToGeneInteractions)?, GetStrategy::default())?;

    for link in interaction_links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(interaction) = record.entry().to_app_option::<DrugGeneInteraction>().ok().flatten() {
                    // Check if patient has this gene in their profile
                    if let Some(patient_variant) = profile.gene_variants.iter().find(|v| v.gene == interaction.gene) {
                        // Find the implication for patient's phenotype
                        if let Some(implication) = interaction.phenotype_implications.iter()
                            .find(|i| i.phenotype == patient_variant.phenotype)
                        {
                            let impact = match &implication.recommendation {
                                DosingRecommendation::Avoid => DrugImpact::IncreasedToxicity,
                                DosingRecommendation::ReducedDose => DrugImpact::AlteredMetabolism,
                                DosingRecommendation::IncreasedDose => DrugImpact::ReducedEfficacy,
                                DosingRecommendation::UseAlternative => DrugImpact::IncreasedToxicity,
                                DosingRecommendation::MonitorClosely => DrugImpact::AlteredMetabolism,
                                _ => DrugImpact::NoImpact,
                            };

                            gene_findings.push(GeneDrugFinding {
                                gene: interaction.gene.clone(),
                                patient_phenotype: patient_variant.phenotype.clone(),
                                impact,
                                recommendation: implication.clinical_notes.clone(),
                            });

                            recommendations.push(format!(
                                "{}: {} - {}",
                                interaction.gene,
                                format!("{:?}", patient_variant.phenotype),
                                implication.clinical_notes
                            ));

                            // Track worst recommendation
                            worst_recommendation = worse_recommendation(
                                worst_recommendation.clone(),
                                implication.recommendation.clone(),
                            );
                        }
                    }
                }
            }
        }
    }

    // Build summary
    let summary = if gene_findings.is_empty() {
        "No actionable pharmacogenomic interactions found for this drug".to_string()
    } else {
        format!(
            "Found {} gene(s) affecting {} metabolism: {}",
            gene_findings.len(),
            input.drug_name,
            gene_findings.iter().map(|f| f.gene.as_str()).collect::<Vec<_>>().join(", ")
        )
    };

    // Calculate confidence based on findings and evidence level
    let confidence = if gene_findings.is_empty() {
        0.5 // Moderate confidence if no interactions found
    } else {
        0.85 // High confidence with PGx data
    };

    Ok(PharmacogenomicCheckResult {
        patient_hash: input.patient_hash,
        drug_rxnorm: input.drug_rxnorm,
        drug_name: input.drug_name,
        gene_findings,
        overall_recommendation: worst_recommendation,
        summary,
        detailed_recommendations: recommendations,
        confidence,
    })
}

/// Helper to determine worse of two recommendations
fn worse_recommendation(a: DosingRecommendation, b: DosingRecommendation) -> DosingRecommendation {
    let severity = |r: &DosingRecommendation| match r {
        DosingRecommendation::Avoid => 6,
        DosingRecommendation::UseAlternative => 5,
        DosingRecommendation::MonitorClosely => 4,
        DosingRecommendation::ReducedDose => 3,
        DosingRecommendation::IncreasedDose => 3,
        DosingRecommendation::InsufficientEvidence => 2,
        DosingRecommendation::StandardDose => 1,
    };

    if severity(&a) >= severity(&b) { a } else { b }
}

/// Check for duplicate therapies (medications in the same therapeutic class)
fn check_duplicate_therapies(medication_rxnorm_codes: &[String]) -> ExternResult<Vec<DuplicateTherapy>> {
    let mut duplicates = Vec::new();

    // Define common therapeutic classes by RxNorm prefixes/patterns
    // In production, this would use a comprehensive drug classification database
    let therapy_classes: Vec<(&str, Vec<&str>, &str)> = vec![
        // (class_name, [rxnorm_prefixes], recommendation)
        ("ACE Inhibitors", vec!["198188", "261962", "308962"],
         "Multiple ACE inhibitors detected. Consider reviewing for therapeutic duplication."),
        ("Statins", vec!["617310", "617311", "617312"],
         "Multiple statins detected. Combination statin therapy is generally not recommended."),
        ("Beta Blockers", vec!["866511", "866924", "866427"],
         "Multiple beta blockers detected. Review for therapeutic necessity."),
        ("Proton Pump Inhibitors", vec!["283742", "311355", "261961"],
         "Multiple PPIs detected. Generally only one PPI needed."),
        ("NSAIDs", vec!["197803", "197804", "197805"],
         "Multiple NSAIDs detected. Increases risk of GI bleeding and renal impairment."),
        ("Benzodiazepines", vec!["197589", "197590", "197591"],
         "Multiple benzodiazepines detected. Increases sedation and fall risk."),
        ("Opioids", vec!["197696", "197697", "197698"],
         "Multiple opioids detected. Review for appropriateness and overdose risk."),
    ];

    for (class_name, prefixes, recommendation) in therapy_classes {
        let matching_drugs: Vec<&String> = medication_rxnorm_codes
            .iter()
            .filter(|code| prefixes.iter().any(|prefix| code.starts_with(prefix)))
            .collect();

        if matching_drugs.len() > 1 {
            // Found duplicate therapy - create pairs
            for i in 0..matching_drugs.len() {
                for j in (i + 1)..matching_drugs.len() {
                    duplicates.push(DuplicateTherapy {
                        drug_a_rxnorm: matching_drugs[i].clone(),
                        drug_a_name: format!("{} medication", class_name),
                        drug_b_rxnorm: matching_drugs[j].clone(),
                        drug_b_name: format!("{} medication", class_name),
                        therapy_class: class_name.to_string(),
                        recommendation: recommendation.to_string(),
                    });
                }
            }
        }
    }

    Ok(duplicates)
}
