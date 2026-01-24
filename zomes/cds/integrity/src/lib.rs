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
    }
}
