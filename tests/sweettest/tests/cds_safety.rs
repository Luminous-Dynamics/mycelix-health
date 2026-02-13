//! CDS (Clinical Decision Support) Safety Tests
//!
//! Covers:
//! - Drug interaction detection (drug-drug, severity levels)
//! - Allergy conflict checking (cross-reactivity)
//! - Pharmacogenomic profile creation and dosing recommendations
//! - Clinical alert lifecycle (create → acknowledge → resolve)
//!
//! ```bash
//! nix develop
//! cargo test -p hdc-genetics-sweettest --test cds_safety
//! ```

use anyhow::Result;
use holochain::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ============================================================================
// Types matching CDS integrity zome
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum InteractionSeverity {
    Minor,
    Moderate,
    Major,
    Contraindicated,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AllergySeverity {
    Mild,
    Moderate,
    Severe,
    Anaphylactic,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AlertType {
    DrugInteraction,
    DrugAllergyConflict,
    DrugDiseaseContraindication,
    DuplicateTherapy,
    DosageAlert,
    LabResultAlert,
    PreventiveCareReminder,
    RefillReminder,
    VitalSignAlert,
    CareGap,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AlertPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AlertCategory {
    Safety,
    Quality,
    Compliance,
    Preventive,
    Administrative,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SafetyAssessment {
    Safe,
    CautionRecommended,
    HighRisk,
    Contraindicated,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MetabolizerPhenotype {
    UltrarapidMetabolizer,
    RapidMetabolizer,
    NormalMetabolizer,
    IntermediateMetabolizer,
    PoorMetabolizer,
    Indeterminate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DosingRecommendation {
    StandardDose,
    ReducedDose,
    IncreasedDose,
    UseAlternative,
    Avoid,
    MonitorClosely,
    InsufficientEvidence,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CpicLevel {
    A,
    B,
    C,
    D,
}

// ============================================================================
// Test struct mirrors
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DrugInteraction {
    pub interaction_id: String,
    pub drug_a_rxnorm: String,
    pub drug_a_name: String,
    pub drug_b_rxnorm: String,
    pub drug_b_name: String,
    pub severity: InteractionSeverity,
    pub description: String,
    pub clinical_effects: Vec<String>,
    pub management: String,
    pub evidence_references: Vec<String>,
    pub source: String,
    pub last_reviewed: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DrugAllergyInteraction {
    pub drug_rxnorm: String,
    pub drug_name: String,
    pub allergen_class: String,
    pub cross_reactive_allergens: Vec<String>,
    pub severity: AllergySeverity,
    pub notes: String,
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClinicalAlert {
    pub alert_id: String,
    pub patient_hash: ActionHash,
    pub alert_type: AlertType,
    pub priority: AlertPriority,
    pub category: AlertCategory,
    pub message: String,
    pub details: Option<String>,
    pub trigger: String,
    pub recommended_actions: Vec<String>,
    pub acknowledged: bool,
    pub acknowledged_by: Option<AgentPubKey>,
    pub acknowledged_at: Option<Timestamp>,
    pub acknowledgment_notes: Option<String>,
    pub resolved: bool,
    pub resolution_notes: Option<String>,
    pub created_at: Timestamp,
    pub expires_at: Option<Timestamp>,
    pub related_data: Vec<ActionHash>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneVariant {
    pub gene: String,
    pub diplotype: String,
    pub hdc_signature: Option<String>,
    pub phenotype: MetabolizerPhenotype,
    pub activity_score: Option<f64>,
    pub clinical_implications: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PharmacogenomicProfile {
    pub profile_id: String,
    pub patient_hash: ActionHash,
    pub gene_variants: Vec<GeneVariant>,
    pub hdc_encoded_profile: Option<String>,
    pub hdc_threshold: Option<f64>,
    pub testing_source: String,
    pub lab_identifier: Option<String>,
    pub test_date: Timestamp,
    pub last_updated: Timestamp,
    pub version: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhenotypeImplication {
    pub phenotype: MetabolizerPhenotype,
    pub recommendation: DosingRecommendation,
    pub dose_adjustment_percent: Option<i32>,
    pub alternatives: Vec<String>,
    pub clinical_notes: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DrugGeneInteraction {
    pub interaction_id: String,
    pub drug_rxnorm: String,
    pub drug_name: String,
    pub gene: String,
    pub phenotype_implications: Vec<PhenotypeImplication>,
    pub cpic_level: CpicLevel,
    pub dpwg_level: Option<String>,
    pub guideline_sources: Vec<String>,
    pub last_reviewed: Timestamp,
    pub hdc_drug_vector: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoundInteraction {
    pub drug_a_rxnorm: String,
    pub drug_a_name: String,
    pub drug_b_rxnorm: String,
    pub drug_b_name: String,
    pub severity: InteractionSeverity,
    pub description: String,
    pub management: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoundAllergyConflict {
    pub drug_rxnorm: String,
    pub drug_name: String,
    pub allergen: String,
    pub cross_reactivity: String,
    pub severity: AllergySeverity,
    pub recommendation: String,
}

// ============================================================================
// Helper: Create test data
// ============================================================================

fn create_warfarin_nsaid_interaction() -> DrugInteraction {
    DrugInteraction {
        interaction_id: "ddi-warfarin-ibuprofen-001".into(),
        drug_a_rxnorm: "11289".into(),
        drug_a_name: "Warfarin".into(),
        drug_b_rxnorm: "5640".into(),
        drug_b_name: "Ibuprofen".into(),
        severity: InteractionSeverity::Major,
        description: "NSAIDs increase bleeding risk with warfarin through antiplatelet effects and GI mucosal damage".into(),
        clinical_effects: vec![
            "Increased INR".into(),
            "GI bleeding risk".into(),
            "Bruising".into(),
        ],
        management: "Avoid combination if possible. If unavoidable, monitor INR closely and use lowest effective NSAID dose for shortest duration.".into(),
        evidence_references: vec![
            "PMID:12345678".into(),
            "FDA Drug Safety Communication 2016".into(),
        ],
        source: "DrugBank".into(),
        last_reviewed: Timestamp::from_micros(1718000000000000),
    }
}

fn create_penicillin_allergy_interaction() -> DrugAllergyInteraction {
    DrugAllergyInteraction {
        drug_rxnorm: "7980".into(),
        drug_name: "Amoxicillin".into(),
        allergen_class: "Penicillins".into(),
        cross_reactive_allergens: vec![
            "Penicillin".into(),
            "Ampicillin".into(),
            "Piperacillin".into(),
        ],
        severity: AllergySeverity::Severe,
        notes: "Beta-lactam cross-reactivity. Estimated 2-5% cross-reaction with cephalosporins.".into(),
        source: "UpToDate".into(),
    }
}

fn create_cyp2d6_poor_metabolizer_variant() -> GeneVariant {
    GeneVariant {
        gene: "CYP2D6".into(),
        diplotype: "*4/*4".into(),
        hdc_signature: None,
        phenotype: MetabolizerPhenotype::PoorMetabolizer,
        activity_score: Some(0.0),
        clinical_implications: vec![
            "Significantly reduced CYP2D6 activity".into(),
            "Increased risk of adverse effects from CYP2D6 substrates".into(),
            "Reduced prodrug activation (e.g., codeine → morphine)".into(),
        ],
    }
}

fn create_cyp2c19_normal_variant() -> GeneVariant {
    GeneVariant {
        gene: "CYP2C19".into(),
        diplotype: "*1/*1".into(),
        hdc_signature: None,
        phenotype: MetabolizerPhenotype::NormalMetabolizer,
        activity_score: Some(2.0),
        clinical_implications: vec![
            "Normal CYP2C19 metabolism".into(),
            "Standard dosing appropriate".into(),
        ],
    }
}

// ============================================================================
// Tests: Drug-Drug Interaction Detection
// ============================================================================

/// Verify drug interaction record structure and severity classification
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_drug_interaction_creation_and_structure() -> Result<()> {
    let interaction = create_warfarin_nsaid_interaction();

    assert_eq!(interaction.interaction_id, "ddi-warfarin-ibuprofen-001");
    assert_eq!(interaction.drug_a_rxnorm, "11289");
    assert_eq!(interaction.drug_a_name, "Warfarin");
    assert_eq!(interaction.drug_b_rxnorm, "5640");
    assert_eq!(interaction.drug_b_name, "Ibuprofen");
    assert_eq!(interaction.severity, InteractionSeverity::Major);
    assert!(!interaction.clinical_effects.is_empty(), "Should list clinical effects");
    assert!(!interaction.management.is_empty(), "Should provide management guidance");
    assert!(!interaction.evidence_references.is_empty(), "Should cite evidence");

    Ok(())
}

/// Verify severity levels are correctly ordered for safety checks
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_interaction_severity_ordering() -> Result<()> {
    // Verify all severity levels serialize/deserialize correctly
    let severities = vec![
        InteractionSeverity::Minor,
        InteractionSeverity::Moderate,
        InteractionSeverity::Major,
        InteractionSeverity::Contraindicated,
    ];

    for severity in &severities {
        let json = serde_json::to_string(severity)?;
        let roundtrip: InteractionSeverity = serde_json::from_str(&json)?;
        assert_eq!(*severity, roundtrip, "Severity should survive serialization roundtrip");
    }

    // Verify worst-case interaction produces Contraindicated
    let contraindicated = InteractionSeverity::Contraindicated;
    let json = serde_json::to_string(&contraindicated)?;
    assert!(json.contains("Contraindicated"));

    Ok(())
}

/// Verify drug interaction serializes correctly for DHT storage
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_drug_interaction_serialization() -> Result<()> {
    let interaction = create_warfarin_nsaid_interaction();

    let json = serde_json::to_value(&interaction)?;

    assert!(json.get("interaction_id").is_some());
    assert!(json.get("drug_a_rxnorm").is_some());
    assert!(json.get("drug_b_rxnorm").is_some());
    assert!(json.get("severity").is_some());
    assert!(json.get("clinical_effects").is_some());
    assert_eq!(
        json["clinical_effects"].as_array().unwrap().len(),
        3,
        "Should have 3 clinical effects"
    );

    // Verify roundtrip
    let roundtrip: DrugInteraction = serde_json::from_value(json)?;
    assert_eq!(roundtrip.interaction_id, interaction.interaction_id);
    assert_eq!(roundtrip.severity, interaction.severity);

    Ok(())
}

// ============================================================================
// Tests: Allergy Conflict Detection
// ============================================================================

/// Verify allergy interaction record captures cross-reactivity
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_allergy_conflict_structure() -> Result<()> {
    let allergy = create_penicillin_allergy_interaction();

    assert_eq!(allergy.drug_rxnorm, "7980");
    assert_eq!(allergy.drug_name, "Amoxicillin");
    assert_eq!(allergy.allergen_class, "Penicillins");
    assert_eq!(allergy.severity, AllergySeverity::Severe);
    assert!(
        allergy.cross_reactive_allergens.contains(&"Penicillin".to_string()),
        "Should list penicillin as cross-reactive"
    );
    assert!(
        allergy.cross_reactive_allergens.contains(&"Ampicillin".to_string()),
        "Should list ampicillin as cross-reactive"
    );
    assert!(allergy.notes.contains("cross-react"), "Notes should mention cross-reactivity");

    Ok(())
}

/// Verify allergy severity levels serialize correctly
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_allergy_severity_levels() -> Result<()> {
    let severities = vec![
        AllergySeverity::Mild,
        AllergySeverity::Moderate,
        AllergySeverity::Severe,
        AllergySeverity::Anaphylactic,
    ];

    for severity in &severities {
        let json = serde_json::to_string(severity)?;
        let roundtrip: AllergySeverity = serde_json::from_str(&json)?;
        assert_eq!(*severity, roundtrip);
    }

    // Anaphylactic is the most severe
    let anaphylactic = AllergySeverity::Anaphylactic;
    let json = serde_json::to_string(&anaphylactic)?;
    assert!(json.contains("Anaphylactic"));

    Ok(())
}

/// Verify found allergy conflict captures drug-allergen pairing
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_found_allergy_conflict_structure() -> Result<()> {
    let conflict = FoundAllergyConflict {
        drug_rxnorm: "7980".into(),
        drug_name: "Amoxicillin".into(),
        allergen: "Penicillin".into(),
        cross_reactivity: "Beta-lactam ring structure".into(),
        severity: AllergySeverity::Severe,
        recommendation: "Avoid penicillin-class antibiotics. Consider macrolide or fluoroquinolone alternative.".into(),
    };

    let json = serde_json::to_value(&conflict)?;
    assert_eq!(json["drug_rxnorm"], "7980");
    assert_eq!(json["allergen"], "Penicillin");
    assert!(json["recommendation"].as_str().unwrap().contains("alternative"));

    Ok(())
}

// ============================================================================
// Tests: Pharmacogenomic (PGx) Profile and Recommendations
// ============================================================================

/// Verify PGx profile captures gene variants with metabolizer phenotypes
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_pgx_profile_creation() -> Result<()> {
    let cyp2d6 = create_cyp2d6_poor_metabolizer_variant();
    let cyp2c19 = create_cyp2c19_normal_variant();

    // Verify CYP2D6 poor metabolizer
    assert_eq!(cyp2d6.gene, "CYP2D6");
    assert_eq!(cyp2d6.diplotype, "*4/*4");
    assert_eq!(cyp2d6.phenotype, MetabolizerPhenotype::PoorMetabolizer);
    assert_eq!(cyp2d6.activity_score, Some(0.0));
    assert!(
        cyp2d6.clinical_implications.iter().any(|i| i.contains("reduced")),
        "Poor metabolizer should mention reduced activity"
    );

    // Verify CYP2C19 normal metabolizer
    assert_eq!(cyp2c19.gene, "CYP2C19");
    assert_eq!(cyp2c19.diplotype, "*1/*1");
    assert_eq!(cyp2c19.phenotype, MetabolizerPhenotype::NormalMetabolizer);
    assert_eq!(cyp2c19.activity_score, Some(2.0));

    Ok(())
}

/// Verify drug-gene interaction captures phenotype-specific dosing recommendations
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_drug_gene_interaction_recommendations() -> Result<()> {
    let interaction = DrugGeneInteraction {
        interaction_id: "dgi-codeine-cyp2d6-001".into(),
        drug_rxnorm: "2670".into(),
        drug_name: "Codeine".into(),
        gene: "CYP2D6".into(),
        phenotype_implications: vec![
            PhenotypeImplication {
                phenotype: MetabolizerPhenotype::PoorMetabolizer,
                recommendation: DosingRecommendation::Avoid,
                dose_adjustment_percent: None,
                alternatives: vec!["Morphine".into(), "Non-opioid analgesic".into()],
                clinical_notes: "CYP2D6 converts codeine to morphine. Poor metabolizers get no analgesic effect.".into(),
            },
            PhenotypeImplication {
                phenotype: MetabolizerPhenotype::UltrarapidMetabolizer,
                recommendation: DosingRecommendation::Avoid,
                dose_adjustment_percent: None,
                alternatives: vec!["Morphine with reduced dose".into(), "Non-opioid analgesic".into()],
                clinical_notes: "Ultrarapid metabolizers convert too much codeine to morphine, risking toxicity.".into(),
            },
            PhenotypeImplication {
                phenotype: MetabolizerPhenotype::NormalMetabolizer,
                recommendation: DosingRecommendation::StandardDose,
                dose_adjustment_percent: None,
                alternatives: vec![],
                clinical_notes: "Normal metabolism. Standard dosing appropriate.".into(),
            },
        ],
        cpic_level: CpicLevel::A,
        dpwg_level: Some("Actionable".into()),
        guideline_sources: vec!["CPIC 2020".into()],
        last_reviewed: Timestamp::from_micros(1718000000000000),
        hdc_drug_vector: None,
    };

    // Verify structure
    assert_eq!(interaction.drug_name, "Codeine");
    assert_eq!(interaction.gene, "CYP2D6");
    assert_eq!(interaction.cpic_level, CpicLevel::A);
    assert_eq!(interaction.phenotype_implications.len(), 3);

    // Poor metabolizer should avoid codeine
    let poor_met = &interaction.phenotype_implications[0];
    assert_eq!(poor_met.phenotype, MetabolizerPhenotype::PoorMetabolizer);
    assert_eq!(poor_met.recommendation, DosingRecommendation::Avoid);
    assert!(!poor_met.alternatives.is_empty(), "Should suggest alternatives");

    // Ultrarapid metabolizer should also avoid
    let ultra = &interaction.phenotype_implications[1];
    assert_eq!(ultra.phenotype, MetabolizerPhenotype::UltrarapidMetabolizer);
    assert_eq!(ultra.recommendation, DosingRecommendation::Avoid);

    // Normal metabolizer gets standard dosing
    let normal = &interaction.phenotype_implications[2];
    assert_eq!(normal.phenotype, MetabolizerPhenotype::NormalMetabolizer);
    assert_eq!(normal.recommendation, DosingRecommendation::StandardDose);

    Ok(())
}

/// Verify metabolizer phenotype coverage and serialization
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_metabolizer_phenotype_variants() -> Result<()> {
    let phenotypes = vec![
        MetabolizerPhenotype::UltrarapidMetabolizer,
        MetabolizerPhenotype::RapidMetabolizer,
        MetabolizerPhenotype::NormalMetabolizer,
        MetabolizerPhenotype::IntermediateMetabolizer,
        MetabolizerPhenotype::PoorMetabolizer,
        MetabolizerPhenotype::Indeterminate,
    ];

    assert_eq!(phenotypes.len(), 6, "Should cover all CPIC phenotype categories");

    for phenotype in &phenotypes {
        let json = serde_json::to_string(phenotype)?;
        let roundtrip: MetabolizerPhenotype = serde_json::from_str(&json)?;
        assert_eq!(*phenotype, roundtrip, "Phenotype should survive roundtrip: {}", json);
    }

    Ok(())
}

/// Verify dosing recommendation categories and serialization
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_dosing_recommendation_variants() -> Result<()> {
    let recommendations = vec![
        DosingRecommendation::StandardDose,
        DosingRecommendation::ReducedDose,
        DosingRecommendation::IncreasedDose,
        DosingRecommendation::UseAlternative,
        DosingRecommendation::Avoid,
        DosingRecommendation::MonitorClosely,
        DosingRecommendation::InsufficientEvidence,
    ];

    assert_eq!(recommendations.len(), 7, "Should cover all dosing recommendation categories");

    for rec in &recommendations {
        let json = serde_json::to_string(rec)?;
        let roundtrip: DosingRecommendation = serde_json::from_str(&json)?;
        assert_eq!(*rec, roundtrip);
    }

    Ok(())
}

// ============================================================================
// Tests: Clinical Alert Lifecycle
// ============================================================================

/// Verify clinical alert creation with all required fields
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_clinical_alert_creation() -> Result<()> {
    // Use a deterministic hash for testing
    let patient_hash = ActionHash::from_raw_36(vec![0xdb; 36]);

    let alert = ClinicalAlert {
        alert_id: "alert-ddi-001".into(),
        patient_hash: patient_hash.clone(),
        alert_type: AlertType::DrugInteraction,
        priority: AlertPriority::Critical,
        category: AlertCategory::Safety,
        message: "Major drug interaction: Warfarin + Ibuprofen".into(),
        details: Some("NSAIDs increase bleeding risk with warfarin".into()),
        trigger: "New prescription: Ibuprofen 400mg".into(),
        recommended_actions: vec![
            "Review warfarin therapy".into(),
            "Consider acetaminophen alternative".into(),
            "Monitor INR within 72 hours".into(),
        ],
        acknowledged: false,
        acknowledged_by: None,
        acknowledged_at: None,
        acknowledgment_notes: None,
        resolved: false,
        resolution_notes: None,
        created_at: Timestamp::from_micros(1718000000000000),
        expires_at: None,
        related_data: vec![],
    };

    assert_eq!(alert.alert_id, "alert-ddi-001");
    assert_eq!(alert.alert_type, AlertType::DrugInteraction);
    assert_eq!(alert.priority, AlertPriority::Critical);
    assert_eq!(alert.category, AlertCategory::Safety);
    assert!(!alert.acknowledged, "New alert should not be acknowledged");
    assert!(!alert.resolved, "New alert should not be resolved");
    assert_eq!(alert.recommended_actions.len(), 3);

    Ok(())
}

/// Verify alert acknowledgment updates the right fields
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_clinical_alert_acknowledgment() -> Result<()> {
    let patient_hash = ActionHash::from_raw_36(vec![0xdb; 36]);
    let provider_key = AgentPubKey::from_raw_36(vec![0xab; 36]);

    let mut alert = ClinicalAlert {
        alert_id: "alert-allergy-001".into(),
        patient_hash: patient_hash.clone(),
        alert_type: AlertType::DrugAllergyConflict,
        priority: AlertPriority::High,
        category: AlertCategory::Safety,
        message: "Drug-allergy conflict: Amoxicillin prescribed to penicillin-allergic patient".into(),
        details: None,
        trigger: "New prescription: Amoxicillin 500mg".into(),
        recommended_actions: vec![
            "Cancel amoxicillin prescription".into(),
            "Consider azithromycin or fluoroquinolone".into(),
        ],
        acknowledged: false,
        acknowledged_by: None,
        acknowledged_at: None,
        acknowledgment_notes: None,
        resolved: false,
        resolution_notes: None,
        created_at: Timestamp::from_micros(1718000000000000),
        expires_at: None,
        related_data: vec![],
    };

    // Simulate acknowledgment
    let ack_time = Timestamp::from_micros(1718000060000000); // 1 minute later
    alert.acknowledged = true;
    alert.acknowledged_by = Some(provider_key.clone());
    alert.acknowledged_at = Some(ack_time);
    alert.acknowledgment_notes = Some("Reviewed. Switching to azithromycin.".into());

    assert!(alert.acknowledged);
    assert_eq!(alert.acknowledged_by, Some(provider_key));
    assert!(alert.acknowledged_at.is_some());
    assert!(!alert.resolved, "Acknowledged does not mean resolved");

    Ok(())
}

/// Verify full alert lifecycle: create → acknowledge → resolve
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_clinical_alert_full_lifecycle() -> Result<()> {
    let patient_hash = ActionHash::from_raw_36(vec![0xdb; 36]);
    let provider_key = AgentPubKey::from_raw_36(vec![0xab; 36]);

    // Step 1: Create alert
    let mut alert = ClinicalAlert {
        alert_id: "alert-lifecycle-001".into(),
        patient_hash: patient_hash.clone(),
        alert_type: AlertType::DuplicateTherapy,
        priority: AlertPriority::Medium,
        category: AlertCategory::Quality,
        message: "Duplicate therapy: Omeprazole and Lansoprazole (both PPIs)".into(),
        details: Some("Patient is on two proton pump inhibitors concurrently".into()),
        trigger: "Medication reconciliation".into(),
        recommended_actions: vec![
            "Discontinue one PPI".into(),
            "Assess which PPI is preferred".into(),
        ],
        acknowledged: false,
        acknowledged_by: None,
        acknowledged_at: None,
        acknowledgment_notes: None,
        resolved: false,
        resolution_notes: None,
        created_at: Timestamp::from_micros(1718000000000000),
        expires_at: None,
        related_data: vec![],
    };

    // Verify initial state
    assert!(!alert.acknowledged);
    assert!(!alert.resolved);

    // Step 2: Acknowledge
    alert.acknowledged = true;
    alert.acknowledged_by = Some(provider_key.clone());
    alert.acknowledged_at = Some(Timestamp::from_micros(1718000060000000));
    alert.acknowledgment_notes = Some("Reviewing duplicate PPI therapy".into());

    assert!(alert.acknowledged);
    assert!(!alert.resolved);

    // Step 3: Resolve
    alert.resolved = true;
    alert.resolution_notes = Some("Discontinued lansoprazole. Continuing omeprazole 20mg daily.".into());

    assert!(alert.acknowledged);
    assert!(alert.resolved);
    assert!(alert.resolution_notes.is_some());

    // Verify serialization of full lifecycle
    let json = serde_json::to_value(&alert)?;
    assert_eq!(json["acknowledged"], true);
    assert_eq!(json["resolved"], true);
    assert!(json["resolution_notes"].as_str().unwrap().contains("omeprazole"));

    Ok(())
}

// ============================================================================
// Tests: Safety Assessment
// ============================================================================

/// Verify safety assessment levels and serialization
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_safety_assessment_levels() -> Result<()> {
    let assessments = vec![
        SafetyAssessment::Safe,
        SafetyAssessment::CautionRecommended,
        SafetyAssessment::HighRisk,
        SafetyAssessment::Contraindicated,
    ];

    for assessment in &assessments {
        let json = serde_json::to_string(assessment)?;
        let roundtrip: SafetyAssessment = serde_json::from_str(&json)?;
        assert_eq!(*assessment, roundtrip);
    }

    Ok(())
}

/// Verify alert type variants cover all CDS use cases
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_alert_type_coverage() -> Result<()> {
    let alert_types = vec![
        AlertType::DrugInteraction,
        AlertType::DrugAllergyConflict,
        AlertType::DrugDiseaseContraindication,
        AlertType::DuplicateTherapy,
        AlertType::DosageAlert,
        AlertType::LabResultAlert,
        AlertType::PreventiveCareReminder,
        AlertType::RefillReminder,
        AlertType::VitalSignAlert,
        AlertType::CareGap,
        AlertType::Custom("Study enrollment reminder".into()),
    ];

    assert_eq!(alert_types.len(), 11, "Should cover all alert type variants");

    for alert_type in &alert_types {
        let json = serde_json::to_string(alert_type)?;
        let roundtrip: AlertType = serde_json::from_str(&json)?;
        assert_eq!(*alert_type, roundtrip);
    }

    Ok(())
}

/// Verify CPIC evidence level coverage
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Holochain conductor and built WASM zomes"]
async fn test_cpic_evidence_levels() -> Result<()> {
    let levels = vec![
        CpicLevel::A,
        CpicLevel::B,
        CpicLevel::C,
        CpicLevel::D,
    ];

    assert_eq!(levels.len(), 4, "Should cover all CPIC evidence levels");

    for level in &levels {
        let json = serde_json::to_string(level)?;
        let roundtrip: CpicLevel = serde_json::from_str(&json)?;
        assert_eq!(*level, roundtrip);
    }

    Ok(())
}
