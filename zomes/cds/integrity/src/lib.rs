//! Clinical Decision Support (CDS) Integrity Zome
//!
//! Defines entry types for clinical decision support including:
//! - Drug-drug interaction checking
//! - Clinical alerts and reminders
//! - Evidence-based care guidelines
//! - Allergy cross-checking
//!
//! HIPAA and clinical safety compliant.

use hdi::prelude::*;

// ============================================================================
// Drug Interaction Types
// ============================================================================

/// Drug-drug interaction record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DrugInteraction {
    /// Unique interaction identifier
    pub interaction_id: String,
    /// First drug RxNorm code
    pub drug_a_rxnorm: String,
    /// First drug name
    pub drug_a_name: String,
    /// Second drug RxNorm code
    pub drug_b_rxnorm: String,
    /// Second drug name
    pub drug_b_name: String,
    /// Interaction severity
    pub severity: InteractionSeverity,
    /// Clinical description of the interaction
    pub description: String,
    /// Clinical effects of the interaction
    pub clinical_effects: Vec<String>,
    /// Management recommendations
    pub management: String,
    /// Supporting evidence/references
    pub evidence_references: Vec<String>,
    /// Source database (e.g., DrugBank, Medscape)
    pub source: String,
    /// Last reviewed date
    pub last_reviewed: Timestamp,
}

/// Interaction severity levels
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum InteractionSeverity {
    /// May cause minor issues, monitoring recommended
    Minor,
    /// May require dosage adjustment or monitoring
    Moderate,
    /// Combination generally should be avoided
    Major,
    /// Combination is contraindicated - do not use together
    Contraindicated,
}

/// Drug-allergy cross-reference
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DrugAllergyInteraction {
    /// Drug RxNorm code
    pub drug_rxnorm: String,
    /// Drug name
    pub drug_name: String,
    /// Related allergen class
    pub allergen_class: String,
    /// Specific allergens that may cross-react
    pub cross_reactive_allergens: Vec<String>,
    /// Severity of potential reaction
    pub severity: AllergySeverity,
    /// Clinical notes
    pub notes: String,
    /// Source of information
    pub source: String,
}

/// Allergy severity for cross-reaction checking
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AllergySeverity {
    Mild,
    Moderate,
    Severe,
    Anaphylactic,
}

// ============================================================================
// Clinical Alert Types
// ============================================================================

/// Clinical alert for patient care
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ClinicalAlert {
    /// Unique alert identifier
    pub alert_id: String,
    /// Patient this alert is for
    pub patient_hash: ActionHash,
    /// Type of alert
    pub alert_type: AlertType,
    /// Alert priority level
    pub priority: AlertPriority,
    /// Alert category for filtering
    pub category: AlertCategory,
    /// Short summary message
    pub message: String,
    /// Detailed description
    pub details: Option<String>,
    /// Triggering condition or event
    pub trigger: String,
    /// Recommended actions
    pub recommended_actions: Vec<String>,
    /// Whether the alert has been acknowledged
    pub acknowledged: bool,
    /// Who acknowledged the alert
    pub acknowledged_by: Option<AgentPubKey>,
    /// When the alert was acknowledged
    pub acknowledged_at: Option<Timestamp>,
    /// Acknowledgment notes
    pub acknowledgment_notes: Option<String>,
    /// Whether the alert has been resolved
    pub resolved: bool,
    /// Resolution notes
    pub resolution_notes: Option<String>,
    /// When the alert was created
    pub created_at: Timestamp,
    /// When the alert expires (if applicable)
    pub expires_at: Option<Timestamp>,
    /// Related data hashes (medications, conditions, etc.)
    pub related_data: Vec<ActionHash>,
}

/// Types of clinical alerts
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AlertType {
    /// Drug-drug interaction detected
    DrugInteraction,
    /// Drug-allergy conflict
    DrugAllergyConflict,
    /// Drug-disease contraindication
    DrugDiseaseContraindication,
    /// Duplicate therapy detected
    DuplicateTherapy,
    /// Dosage outside recommended range
    DosageAlert,
    /// Lab value requires attention
    LabResultAlert,
    /// Preventive care reminder
    PreventiveCareReminder,
    /// Medication refill needed
    RefillReminder,
    /// Vital sign concern
    VitalSignAlert,
    /// Care gap identified
    CareGap,
    /// Custom alert
    Custom(String),
}

/// Alert priority levels
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AlertPriority {
    /// Informational - no immediate action required
    Low,
    /// Should be reviewed at next encounter
    Medium,
    /// Requires attention soon
    High,
    /// Immediate action required - safety concern
    Critical,
}

/// Alert categories for organization
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AlertCategory {
    Safety,
    Quality,
    Compliance,
    Preventive,
    Administrative,
    Custom(String),
}

// ============================================================================
// Clinical Guideline Types
// ============================================================================

/// Evidence-based clinical guideline
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ClinicalGuideline {
    /// Unique guideline identifier
    pub guideline_id: String,
    /// Guideline title
    pub title: String,
    /// Short description
    pub summary: String,
    /// Full guideline content
    pub content: String,
    /// Applicable conditions (ICD-10 codes)
    pub applicable_conditions: Vec<String>,
    /// Target patient population
    pub target_population: String,
    /// Recommended interventions
    pub recommendations: Vec<GuidelineRecommendation>,
    /// Quality measures associated with this guideline
    pub quality_measures: Vec<String>,
    /// Evidence grade (A, B, C, D, E)
    pub evidence_grade: String,
    /// Source organization (e.g., AHA, USPSTF)
    pub source_organization: String,
    /// Publication/revision date
    pub publication_date: String,
    /// Guideline version
    pub version: String,
    /// Active status
    pub is_active: bool,
}

/// Recommendation within a guideline
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GuidelineRecommendation {
    /// Recommendation text
    pub text: String,
    /// Strength of recommendation (strong, moderate, weak)
    pub strength: String,
    /// Evidence quality
    pub evidence_quality: String,
    /// Specific actions
    pub actions: Vec<String>,
}

/// Patient-specific guideline recommendation status
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PatientGuidelineStatus {
    /// Patient hash
    pub patient_hash: ActionHash,
    /// Guideline reference
    pub guideline_id: String,
    /// Status of each recommendation
    pub recommendation_statuses: Vec<RecommendationStatus>,
    /// Overall compliance score (0-100)
    pub compliance_score: u8,
    /// Last assessed date
    pub last_assessed: Timestamp,
    /// Next assessment due
    pub next_assessment_due: Option<Timestamp>,
    /// Notes from assessor
    pub notes: Option<String>,
}

/// Status of a specific recommendation for a patient
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RecommendationStatus {
    /// Recommendation index
    pub recommendation_index: u32,
    /// Status
    pub status: ComplianceStatus,
    /// Date last checked
    pub checked_at: Timestamp,
    /// Evidence/documentation hash
    pub evidence_hash: Option<ActionHash>,
}

/// Compliance status for recommendations
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ComplianceStatus {
    Met,
    NotMet,
    PartiallyMet,
    NotApplicable,
    Pending,
}

// ============================================================================
// Interaction Check Request/Response
// ============================================================================

/// Request to check for drug interactions
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct InteractionCheckRequest {
    /// Request identifier
    pub request_id: String,
    /// Patient hash
    pub patient_hash: ActionHash,
    /// RxNorm codes of medications to check
    pub medication_rxnorm_codes: Vec<String>,
    /// Patient's known allergies (allergen names/classes)
    pub patient_allergies: Vec<String>,
    /// Include allergy cross-checks
    pub check_allergies: bool,
    /// Include duplicate therapy check
    pub check_duplicates: bool,
    /// Include dosage validation
    pub check_dosages: bool,
    /// Requested by
    pub requested_by: AgentPubKey,
    /// Requested at
    pub requested_at: Timestamp,
}

/// Response from interaction check
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct InteractionCheckResponse {
    /// Request this responds to
    pub request_id: String,
    /// Patient hash
    pub patient_hash: ActionHash,
    /// Found drug-drug interactions
    pub drug_interactions: Vec<FoundInteraction>,
    /// Found drug-allergy conflicts
    pub allergy_conflicts: Vec<FoundAllergyConflict>,
    /// Found duplicate therapies
    pub duplicate_therapies: Vec<DuplicateTherapy>,
    /// Overall safety assessment
    pub safety_assessment: SafetyAssessment,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Check completed at
    pub completed_at: Timestamp,
}

/// A found drug-drug interaction
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FoundInteraction {
    pub drug_a_rxnorm: String,
    pub drug_a_name: String,
    pub drug_b_rxnorm: String,
    pub drug_b_name: String,
    pub severity: InteractionSeverity,
    pub description: String,
    pub management: String,
}

/// A found drug-allergy conflict
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FoundAllergyConflict {
    pub drug_rxnorm: String,
    pub drug_name: String,
    pub allergen: String,
    pub cross_reactivity: String,
    pub severity: AllergySeverity,
    pub recommendation: String,
}

/// Duplicate therapy finding
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DuplicateTherapy {
    pub drug_a_rxnorm: String,
    pub drug_a_name: String,
    pub drug_b_rxnorm: String,
    pub drug_b_name: String,
    pub therapy_class: String,
    pub recommendation: String,
}

/// Overall safety assessment
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SafetyAssessment {
    Safe,
    CautionRecommended,
    HighRisk,
    Contraindicated,
}

// ============================================================================
// Pharmacogenomics Types (HDC-Enhanced)
// ============================================================================

/// Patient's pharmacogenomic profile with HDC-encoded variant data
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PharmacogenomicProfile {
    /// Profile identifier
    pub profile_id: String,
    /// Patient this profile belongs to
    pub patient_hash: ActionHash,
    /// Gene variants with phenotype predictions
    pub gene_variants: Vec<GeneVariant>,
    /// HDC-encoded representation of genetic profile (for privacy-preserving matching)
    /// This is the base64-encoded hypervector from hdc-core
    pub hdc_encoded_profile: Option<String>,
    /// HDC similarity threshold used for encoding
    pub hdc_threshold: Option<f64>,
    /// Source of genetic testing
    pub testing_source: String,
    /// Testing lab identifier
    pub lab_identifier: Option<String>,
    /// Date of genetic testing
    pub test_date: Timestamp,
    /// Last updated
    pub last_updated: Timestamp,
    /// Profile version for updates
    pub version: u32,
}

/// Individual gene variant with phenotype
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GeneVariant {
    /// Gene symbol (e.g., "CYP2D6", "CYP2C19")
    pub gene: String,
    /// Star allele diplotype (e.g., "*1/*4", "*2/*3")
    pub diplotype: String,
    /// HDC-encoded variant signature (base64)
    pub hdc_signature: Option<String>,
    /// Predicted phenotype
    pub phenotype: MetabolizerPhenotype,
    /// Activity score if applicable
    pub activity_score: Option<f64>,
    /// Clinical implications
    pub clinical_implications: Vec<String>,
}

/// Drug metabolizer phenotype categories per CPIC guidelines
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MetabolizerPhenotype {
    /// Extremely rapid drug metabolism
    UltrarapidMetabolizer,
    /// Faster than normal metabolism
    RapidMetabolizer,
    /// Normal drug metabolism
    NormalMetabolizer,
    /// Intermediate metabolizer status
    IntermediateMetabolizer,
    /// Significantly reduced metabolism
    PoorMetabolizer,
    /// Cannot be determined
    Indeterminate,
}

/// Drug-gene interaction for pharmacogenomics
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DrugGeneInteraction {
    /// Interaction identifier
    pub interaction_id: String,
    /// Drug RxNorm code
    pub drug_rxnorm: String,
    /// Drug name
    pub drug_name: String,
    /// Gene symbol
    pub gene: String,
    /// Affected phenotypes and their implications
    pub phenotype_implications: Vec<PhenotypeImplication>,
    /// CPIC evidence level
    pub cpic_level: CpicLevel,
    /// DPWG evidence level
    pub dpwg_level: Option<String>,
    /// Source guidelines
    pub guideline_sources: Vec<String>,
    /// Last reviewed
    pub last_reviewed: Timestamp,
    /// HDC vector for fast similarity lookup
    pub hdc_drug_vector: Option<String>,
}

/// Implication for a specific phenotype
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PhenotypeImplication {
    /// Which phenotype this applies to
    pub phenotype: MetabolizerPhenotype,
    /// Dosing recommendation category
    pub recommendation: DosingRecommendation,
    /// Dosing adjustment percentage (e.g., 50 for "reduce by 50%")
    pub dose_adjustment_percent: Option<i32>,
    /// Alternative drugs to consider
    pub alternatives: Vec<String>,
    /// Clinical notes
    pub clinical_notes: String,
}

/// Dosing recommendation categories
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DosingRecommendation {
    /// Use standard dosing
    StandardDose,
    /// Start with lower dose
    ReducedDose,
    /// Start with higher dose
    IncreasedDose,
    /// Use alternative drug
    UseAlternative,
    /// Avoid this drug
    Avoid,
    /// Requires therapeutic drug monitoring
    MonitorClosely,
    /// Insufficient evidence for recommendation
    InsufficientEvidence,
}

/// CPIC evidence levels
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CpicLevel {
    /// Strong evidence, actionable PGx
    A,
    /// Moderate evidence, actionable PGx
    B,
    /// Weak evidence, likely actionable
    C,
    /// Informative only
    D,
}

/// Pharmacogenomic check result
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PharmacogenomicCheckResult {
    /// Patient hash
    pub patient_hash: ActionHash,
    /// Drug checked
    pub drug_rxnorm: String,
    /// Drug name
    pub drug_name: String,
    /// Relevant gene findings
    pub gene_findings: Vec<GeneDrugFinding>,
    /// Overall recommendation
    pub overall_recommendation: DosingRecommendation,
    /// Summary message
    pub summary: String,
    /// Detailed recommendations
    pub detailed_recommendations: Vec<String>,
    /// Confidence in prediction (0.0-1.0)
    pub confidence: f64,
}

/// Finding for a specific gene-drug pair
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GeneDrugFinding {
    /// Gene symbol
    pub gene: String,
    /// Patient's phenotype for this gene
    pub patient_phenotype: MetabolizerPhenotype,
    /// Impact on this drug
    pub impact: DrugImpact,
    /// Specific recommendation
    pub recommendation: String,
}

/// Impact of genetic variant on drug
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DrugImpact {
    /// No significant impact
    NoImpact,
    /// Reduced efficacy
    ReducedEfficacy,
    /// Increased efficacy
    IncreasedEfficacy,
    /// Increased toxicity risk
    IncreasedToxicity,
    /// Both efficacy and toxicity affected
    AlteredMetabolism,
}

// ============================================================================
// Entry and Link Type Enums
// ============================================================================

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    DrugInteraction(DrugInteraction),
    DrugAllergyInteraction(DrugAllergyInteraction),
    ClinicalAlert(ClinicalAlert),
    ClinicalGuideline(ClinicalGuideline),
    PatientGuidelineStatus(PatientGuidelineStatus),
    InteractionCheckRequest(InteractionCheckRequest),
    InteractionCheckResponse(InteractionCheckResponse),
    PharmacogenomicProfile(PharmacogenomicProfile),
    DrugGeneInteraction(DrugGeneInteraction),
}

#[hdk_link_types]
pub enum LinkTypes {
    /// Patient to their clinical alerts
    PatientToAlerts,
    /// Patient to their guideline statuses
    PatientToGuidelineStatuses,
    /// Patient to interaction check history
    PatientToInteractionChecks,
    /// Drug to its known interactions
    DrugToInteractions,
    /// Drug to allergy cross-references
    DrugToAllergyInteractions,
    /// Guideline to applicable conditions
    GuidelineToConditions,
    /// All active guidelines
    AllActiveGuidelines,
    /// All drug interactions (for lookup)
    AllDrugInteractions,
    /// Alert updates
    AlertUpdates,
    /// Patient to pharmacogenomic profile
    PatientToPgxProfile,
    /// Drug to gene interactions
    DrugToGeneInteractions,
    /// Gene to drug interactions
    GeneToDrugInteractions,
}

// ============================================================================
// Validation Functions
// ============================================================================

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => validate_create_entry(app_entry),
            OpEntry::UpdateEntry { app_entry, .. } => validate_create_entry(app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterCreateLink { link_type, .. } => validate_link(link_type),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(entry: EntryTypes) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::DrugInteraction(interaction) => validate_drug_interaction(&interaction),
        EntryTypes::DrugAllergyInteraction(interaction) => validate_allergy_interaction(&interaction),
        EntryTypes::ClinicalAlert(alert) => validate_clinical_alert(&alert),
        EntryTypes::ClinicalGuideline(guideline) => validate_clinical_guideline(&guideline),
        EntryTypes::PatientGuidelineStatus(status) => validate_guideline_status(&status),
        EntryTypes::InteractionCheckRequest(request) => validate_interaction_request(&request),
        EntryTypes::InteractionCheckResponse(response) => validate_interaction_response(&response),
        EntryTypes::PharmacogenomicProfile(profile) => validate_pgx_profile(&profile),
        EntryTypes::DrugGeneInteraction(interaction) => validate_drug_gene_interaction(&interaction),
    }
}

fn validate_drug_interaction(interaction: &DrugInteraction) -> ExternResult<ValidateCallbackResult> {
    if interaction.interaction_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Interaction ID cannot be empty".to_string(),
        ));
    }

    if interaction.drug_a_rxnorm.is_empty() || interaction.drug_b_rxnorm.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Both drug RxNorm codes are required".to_string(),
        ));
    }

    if interaction.drug_a_name.is_empty() || interaction.drug_b_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Both drug names are required".to_string(),
        ));
    }

    if interaction.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Interaction description is required".to_string(),
        ));
    }

    if interaction.management.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Management recommendation is required".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_allergy_interaction(interaction: &DrugAllergyInteraction) -> ExternResult<ValidateCallbackResult> {
    if interaction.drug_rxnorm.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Drug RxNorm code is required".to_string(),
        ));
    }

    if interaction.allergen_class.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Allergen class is required".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_clinical_alert(alert: &ClinicalAlert) -> ExternResult<ValidateCallbackResult> {
    if alert.alert_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Alert ID cannot be empty".to_string(),
        ));
    }

    if alert.message.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Alert message is required".to_string(),
        ));
    }

    if alert.trigger.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Alert trigger is required".to_string(),
        ));
    }

    // Validate acknowledgment consistency
    if alert.acknowledged && alert.acknowledged_by.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "Acknowledged alert must have acknowledger".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_clinical_guideline(guideline: &ClinicalGuideline) -> ExternResult<ValidateCallbackResult> {
    if guideline.guideline_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Guideline ID cannot be empty".to_string(),
        ));
    }

    if guideline.title.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Guideline title is required".to_string(),
        ));
    }

    if guideline.source_organization.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Source organization is required for clinical guidelines".to_string(),
        ));
    }

    // Validate evidence grade
    let valid_grades = ["A", "B", "C", "D", "E", "I"];
    if !valid_grades.contains(&guideline.evidence_grade.to_uppercase().as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid evidence grade: {}. Must be one of: {:?}", guideline.evidence_grade, valid_grades),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_guideline_status(status: &PatientGuidelineStatus) -> ExternResult<ValidateCallbackResult> {
    if status.guideline_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Guideline ID is required".to_string(),
        ));
    }

    if status.compliance_score > 100 {
        return Ok(ValidateCallbackResult::Invalid(
            "Compliance score must be between 0 and 100".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_interaction_request(request: &InteractionCheckRequest) -> ExternResult<ValidateCallbackResult> {
    if request.request_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Request ID cannot be empty".to_string(),
        ));
    }

    if request.medication_rxnorm_codes.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one medication must be provided for interaction check".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_interaction_response(response: &InteractionCheckResponse) -> ExternResult<ValidateCallbackResult> {
    if response.request_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Response must reference a request ID".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_pgx_profile(profile: &PharmacogenomicProfile) -> ExternResult<ValidateCallbackResult> {
    if profile.profile_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Profile ID cannot be empty".to_string(),
        ));
    }

    if profile.testing_source.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Testing source is required".to_string(),
        ));
    }

    if profile.gene_variants.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one gene variant is required".to_string(),
        ));
    }

    // Validate each gene variant
    for variant in &profile.gene_variants {
        if variant.gene.is_empty() {
            return Ok(ValidateCallbackResult::Invalid(
                "Gene symbol cannot be empty".to_string(),
            ));
        }
        if variant.diplotype.is_empty() {
            return Ok(ValidateCallbackResult::Invalid(
                format!("Diplotype required for gene {}", variant.gene),
            ));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_drug_gene_interaction(interaction: &DrugGeneInteraction) -> ExternResult<ValidateCallbackResult> {
    if interaction.interaction_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Interaction ID cannot be empty".to_string(),
        ));
    }

    if interaction.drug_rxnorm.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Drug RxNorm code is required".to_string(),
        ));
    }

    if interaction.gene.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Gene symbol is required".to_string(),
        ));
    }

    if interaction.phenotype_implications.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one phenotype implication is required".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_link(link_type: LinkTypes) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        LinkTypes::PatientToAlerts => Ok(ValidateCallbackResult::Valid),
        LinkTypes::PatientToGuidelineStatuses => Ok(ValidateCallbackResult::Valid),
        LinkTypes::PatientToInteractionChecks => Ok(ValidateCallbackResult::Valid),
        LinkTypes::DrugToInteractions => Ok(ValidateCallbackResult::Valid),
        LinkTypes::DrugToAllergyInteractions => Ok(ValidateCallbackResult::Valid),
        LinkTypes::GuidelineToConditions => Ok(ValidateCallbackResult::Valid),
        LinkTypes::AllActiveGuidelines => Ok(ValidateCallbackResult::Valid),
        LinkTypes::AllDrugInteractions => Ok(ValidateCallbackResult::Valid),
        LinkTypes::AlertUpdates => Ok(ValidateCallbackResult::Valid),
        LinkTypes::PatientToPgxProfile => Ok(ValidateCallbackResult::Valid),
        LinkTypes::DrugToGeneInteractions => Ok(ValidateCallbackResult::Valid),
        LinkTypes::GeneToDrugInteractions => Ok(ValidateCallbackResult::Valid),
    }
}
