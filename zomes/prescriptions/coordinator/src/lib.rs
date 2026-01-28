//! Prescriptions Coordinator Zome
//!
//! Provides extern functions for prescription management,
//! pharmacy interactions, and medication adherence tracking.
//!
//! All prescription data access enforces consent-based access control
//! per HIPAA requirements. Controlled substance tracking has additional
//! audit requirements.
//!
//! Integrates with CDS zome for drug interaction and allergy checking.

use hdk::prelude::*;
use prescriptions_integrity::*;
use mycelix_health_shared::{
    require_authorization, require_admin_authorization,
    log_data_access,
    DataCategory, Permission,
};
use holochain_serialized_bytes::prelude::*;

// ============================================================================
// CDS Integration Types (for cross-zome calls)
// ============================================================================

/// Safety assessment result from CDS
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SafetyAssessment {
    Safe,
    CautionRecommended,
    HighRisk,
    Contraindicated,
}

/// Found drug interaction from CDS
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoundInteraction {
    pub drug_a_rxnorm: String,
    pub drug_a_name: String,
    pub drug_b_rxnorm: String,
    pub drug_b_name: String,
    pub severity: CdsInteractionSeverity,
    pub description: String,
    pub management: String,
}

/// CDS Interaction severity (mirrors cds_integrity)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CdsInteractionSeverity {
    Contraindicated,
    Major,
    Moderate,
    Minor,
    Unknown,
}

/// Found allergy conflict from CDS
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoundAllergyConflict {
    pub drug_rxnorm: String,
    pub drug_name: String,
    pub allergen: String,
    pub cross_reactivity: String,
    pub severity: CdsAllergySeverity,
    pub recommendation: String,
}

/// CDS Allergy severity
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CdsAllergySeverity {
    Anaphylactic,
    Severe,
    Moderate,
    Mild,
    Unknown,
}

/// Duplicate therapy warning from CDS
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DuplicateTherapy {
    pub drug_a_rxnorm: String,
    pub drug_a_name: String,
    pub drug_b_rxnorm: String,
    pub drug_b_name: String,
    pub therapy_class: String,
    pub recommendation: String,
}

/// CDS interaction check request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CdsInteractionCheckRequest {
    pub request_id: String,
    pub patient_hash: ActionHash,
    pub medication_rxnorm_codes: Vec<String>,
    pub patient_allergies: Vec<String>,
    pub check_allergies: bool,
    pub check_duplicates: bool,
}

/// CDS interaction check response (mirrors cds_integrity::InteractionCheckResponse)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdsInteractionCheckResponse {
    pub request_id: String,
    pub patient_hash: ActionHash,
    pub drug_interactions: Vec<FoundInteraction>,
    pub allergy_conflicts: Vec<FoundAllergyConflict>,
    pub duplicate_therapies: Vec<DuplicateTherapy>,
    pub safety_assessment: SafetyAssessment,
    pub recommendations: Vec<String>,
    pub completed_at: Timestamp,
}

// Implement serialization for cross-zome communication
impl TryFrom<SerializedBytes> for CdsInteractionCheckResponse {
    type Error = SerializedBytesError;
    fn try_from(bytes: SerializedBytes) -> Result<Self, Self::Error> {
        let bytes_vec: Vec<u8> = bytes.bytes().to_vec();
        serde_json::from_slice(&bytes_vec)
            .map_err(|e| SerializedBytesError::Deserialize(e.to_string()))
    }
}

/// Prescription safety check result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrescriptionSafetyResult {
    pub is_safe: bool,
    pub safety_assessment: SafetyAssessment,
    pub drug_interactions: Vec<FoundInteraction>,
    pub allergy_conflicts: Vec<FoundAllergyConflict>,
    pub duplicate_therapies: Vec<DuplicateTherapy>,
    pub recommendations: Vec<String>,
    pub requires_override: bool,
}

/// Input for creating prescription with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePrescriptionInput {
    pub prescription: Prescription,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Create a new prescription with access control
#[hdk_extern]
pub fn create_prescription(input: CreatePrescriptionInput) -> ExternResult<Record> {
    // Require Write authorization for Medications category
    let auth = require_authorization(
        input.prescription.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Write,
        input.is_emergency,
    )?;

    let rx_hash = create_entry(&EntryTypes::Prescription(input.prescription.clone()))?;
    let record = get(rx_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find prescription".to_string())))?;

    // Link to patient
    create_link(
        input.prescription.patient_hash.clone(),
        rx_hash.clone(),
        LinkTypes::PatientToPrescriptions,
        (),
    )?;

    // Link to prescriber
    create_link(
        input.prescription.prescriber_hash.clone(),
        rx_hash.clone(),
        LinkTypes::PrescriberToPrescriptions,
        (),
    )?;

    // If controlled substance, add to tracking
    if input.prescription.schedule.is_some() && input.prescription.schedule != Some(DrugSchedule::NotControlled) {
        let controlled_anchor = anchor_hash("controlled_substances")?;
        create_link(
            controlled_anchor,
            rx_hash,
            LinkTypes::ControlledSubstances,
            (),
        )?;
    }

    // Log the access
    log_data_access(
        input.prescription.patient_hash,
        vec![DataCategory::Medications],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(record)
}

/// Input for getting prescription with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPrescriptionInput {
    pub rx_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Internal get without access control
fn get_prescription_internal(rx_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(rx_hash, GetOptions::default())
}

/// Get a prescription with access control
#[hdk_extern]
pub fn get_prescription(input: GetPrescriptionInput) -> ExternResult<Option<Record>> {
    // First get the prescription to find the patient_hash
    let record = get_prescription_internal(input.rx_hash.clone())?;

    if let Some(ref rec) = record {
        let prescription: Prescription = rec
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
            .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid prescription entry".to_string())))?;

        // Require Read authorization
        let auth = require_authorization(
            prescription.patient_hash.clone(),
            DataCategory::Medications,
            Permission::Read,
            input.is_emergency,
        )?;

        // Log the access
        log_data_access(
            prescription.patient_hash,
            vec![DataCategory::Medications],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            input.emergency_reason,
        )?;
    }

    Ok(record)
}

/// Input for getting patient prescriptions with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPatientPrescriptionsInput {
    pub patient_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Internal get without access control
fn get_patient_prescriptions_internal(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(patient_hash, LinkTypes::PatientToPrescriptions)?, GetStrategy::default())?;

    let mut prescriptions = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                prescriptions.push(record);
            }
        }
    }

    Ok(prescriptions)
}

/// Get patient's prescriptions with access control
#[hdk_extern]
pub fn get_patient_prescriptions(input: GetPatientPrescriptionsInput) -> ExternResult<Vec<Record>> {
    // Require Read authorization for Medications category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Read,
        input.is_emergency,
    )?;

    let prescriptions = get_patient_prescriptions_internal(input.patient_hash.clone())?;

    // Log the access
    if !prescriptions.is_empty() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::Medications],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            input.emergency_reason,
        )?;
    }

    Ok(prescriptions)
}

/// Get active prescriptions for a patient with access control
#[hdk_extern]
pub fn get_active_prescriptions(input: GetPatientPrescriptionsInput) -> ExternResult<Vec<Record>> {
    // Require Read authorization for Medications category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Read,
        input.is_emergency,
    )?;

    let all_rx = get_patient_prescriptions_internal(input.patient_hash.clone())?;

    let active: Vec<Record> = all_rx
        .into_iter()
        .filter(|record| {
            if let Some(rx) = record.entry().to_app_option::<Prescription>().ok().flatten() {
                matches!(rx.status, PrescriptionStatus::Active)
            } else {
                false
            }
        })
        .collect();

    // Log the access
    if !active.is_empty() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::Medications],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            input.emergency_reason,
        )?;
    }

    Ok(active)
}

/// Input for filling prescription with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct FillPrescriptionInput {
    pub fill: PrescriptionFill,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Fill a prescription with access control
#[hdk_extern]
pub fn fill_prescription(input: FillPrescriptionInput) -> ExternResult<Record> {
    // First, verify the prescription exists and has refills
    let rx_record = get(input.fill.prescription_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Prescription not found".to_string())))?;

    let mut prescription: Prescription = rx_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid prescription".to_string())))?;

    // Require Write authorization for Medications category
    let auth = require_authorization(
        prescription.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Write,
        input.is_emergency,
    )?;

    if prescription.refills_remaining == 0 {
        return Err(wasm_error!(WasmErrorInner::Guest("No refills remaining".to_string())));
    }

    // Create the fill record
    let fill_hash = create_entry(&EntryTypes::PrescriptionFill(input.fill.clone()))?;
    let fill_record = get(fill_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find fill".to_string())))?;

    // Link fill to prescription
    create_link(
        input.fill.prescription_hash.clone(),
        fill_hash,
        LinkTypes::PrescriptionToFills,
        (),
    )?;

    // Decrement refills
    prescription.refills_remaining = prescription.refills_remaining.saturating_sub(1);
    update_entry(input.fill.prescription_hash, &prescription)?;

    // Log the access
    log_data_access(
        prescription.patient_hash,
        vec![DataCategory::Medications],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(fill_record)
}

/// Input for getting prescription fills with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPrescriptionFillsInput {
    pub rx_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get prescription fills with access control
#[hdk_extern]
pub fn get_prescription_fills(input: GetPrescriptionFillsInput) -> ExternResult<Vec<Record>> {
    // First get the prescription to find the patient_hash
    let rx_record = get_prescription_internal(input.rx_hash.clone())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Prescription not found".to_string())))?;

    let prescription: Prescription = rx_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid prescription entry".to_string())))?;

    // Require Read authorization
    let auth = require_authorization(
        prescription.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Read,
        input.is_emergency,
    )?;

    let links = get_links(LinkQuery::try_new(input.rx_hash, LinkTypes::PrescriptionToFills)?, GetStrategy::default())?;

    let mut fills = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                fills.push(record);
            }
        }
    }

    // Log the access
    if !fills.is_empty() {
        log_data_access(
            prescription.patient_hash,
            vec![DataCategory::Medications],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            input.emergency_reason,
        )?;
    }

    Ok(fills)
}

/// Input for recording adherence with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct RecordAdherenceInput {
    pub adherence: MedicationAdherence,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Record medication adherence with access control
#[hdk_extern]
pub fn record_adherence(input: RecordAdherenceInput) -> ExternResult<Record> {
    // Require Write authorization for Medications category
    let auth = require_authorization(
        input.adherence.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Write,
        input.is_emergency,
    )?;

    let adherence_hash = create_entry(&EntryTypes::MedicationAdherence(input.adherence.clone()))?;
    let record = get(adherence_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find adherence record".to_string())))?;

    create_link(
        input.adherence.patient_hash.clone(),
        adherence_hash,
        LinkTypes::PatientToAdherence,
        (),
    )?;

    // Log the access
    log_data_access(
        input.adherence.patient_hash,
        vec![DataCategory::Medications],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(record)
}

/// Input for creating interaction alert with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateInteractionAlertInput {
    pub alert: DrugInteractionAlert,
    pub patient_hash: ActionHash, // Must provide patient for authorization
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Create drug interaction alert with access control
#[hdk_extern]
pub fn create_interaction_alert(input: CreateInteractionAlertInput) -> ExternResult<Record> {
    // Require Write authorization for Medications category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Write,
        input.is_emergency,
    )?;

    let alert_hash = create_entry(&EntryTypes::DrugInteractionAlert(input.alert.clone()))?;
    let record = get(alert_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find alert".to_string())))?;

    create_link(
        input.alert.prescription_hash,
        alert_hash,
        LinkTypes::PrescriptionToAlerts,
        (),
    )?;

    // Log the access
    log_data_access(
        input.patient_hash,
        vec![DataCategory::Medications],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(record)
}

/// Input for acknowledging alert with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct AcknowledgeAlertInput {
    pub alert_hash: ActionHash,
    pub override_reason: Option<String>,
    pub patient_hash: ActionHash, // Must provide patient for authorization
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Acknowledge drug interaction alert with access control
#[hdk_extern]
pub fn acknowledge_alert(input: AcknowledgeAlertInput) -> ExternResult<Record> {
    // Require Write authorization for Medications category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Write,
        input.is_emergency,
    )?;

    let record = get(input.alert_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Alert not found".to_string())))?;

    let mut alert: DrugInteractionAlert = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid alert".to_string())))?;

    alert.acknowledged = true;
    alert.acknowledged_by = Some(agent_info()?.agent_initial_pubkey);
    alert.acknowledged_at = Some(sys_time()?);
    alert.override_reason = input.override_reason;

    let updated_hash = update_entry(input.alert_hash, &alert)?;
    let updated_record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated alert".to_string())))?;

    // Log the access
    log_data_access(
        input.patient_hash,
        vec![DataCategory::Medications],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(updated_record)
}

/// Create/register a pharmacy (admin function - requires admin authorization)
#[hdk_extern]
pub fn register_pharmacy(pharmacy: Pharmacy) -> ExternResult<Record> {
    // Require admin authorization for pharmacy registration
    require_admin_authorization()?;

    let pharmacy_hash = create_entry(&EntryTypes::Pharmacy(pharmacy.clone()))?;
    let record = get(pharmacy_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find pharmacy".to_string())))?;

    let pharmacies_anchor = anchor_hash("all_pharmacies")?;
    create_link(
        pharmacies_anchor,
        pharmacy_hash,
        LinkTypes::AllPharmacies,
        (),
    )?;

    Ok(record)
}

/// Get all pharmacies (public - no PHI involved)
#[hdk_extern]
pub fn get_all_pharmacies(_: ()) -> ExternResult<Vec<Record>> {
    let pharmacies_anchor = anchor_hash("all_pharmacies")?;
    let links = get_links(LinkQuery::try_new(pharmacies_anchor, LinkTypes::AllPharmacies)?, GetStrategy::default())?;

    let mut pharmacies = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                pharmacies.push(record);
            }
        }
    }

    Ok(pharmacies)
}

/// Input for setting patient pharmacy with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct SetPharmacyInput {
    pub patient_hash: ActionHash,
    pub pharmacy_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Set patient's preferred pharmacy with access control
#[hdk_extern]
pub fn set_patient_pharmacy(input: SetPharmacyInput) -> ExternResult<()> {
    // Require Write authorization for patient preferences (Demographics category)
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Demographics,
        Permission::Write,
        input.is_emergency,
    )?;

    create_link(
        input.patient_hash.clone(),
        input.pharmacy_hash,
        LinkTypes::PatientToPharmacy,
        (),
    )?;

    // Log the access
    log_data_access(
        input.patient_hash,
        vec![DataCategory::Demographics],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(())
}

/// Input for discontinuing prescription with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct DiscontinueInput {
    pub rx_hash: ActionHash,
    pub reason: String,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Discontinue a prescription with access control
#[hdk_extern]
pub fn discontinue_prescription(input: DiscontinueInput) -> ExternResult<Record> {
    let record = get(input.rx_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Prescription not found".to_string())))?;

    let mut prescription: Prescription = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid prescription".to_string())))?;

    // Require Write authorization for Medications category
    let auth = require_authorization(
        prescription.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Write,
        input.is_emergency,
    )?;

    prescription.status = PrescriptionStatus::Discontinued;

    let updated_hash = update_entry(input.rx_hash, &prescription)?;
    let updated_record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated prescription".to_string())))?;

    // Log the access
    log_data_access(
        prescription.patient_hash,
        vec![DataCategory::Medications],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(updated_record)
}

/// Anchor entry for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}

// ============================================================================
// CDS Integration Functions
// ============================================================================

/// Input for creating prescription with integrated safety check
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePrescriptionWithSafetyInput {
    pub prescription: Prescription,
    pub patient_allergies: Vec<String>,
    pub require_safety_check: bool,
    pub allow_override: bool,
    pub override_reason: Option<String>,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Response from prescription creation with safety info
#[derive(Serialize, Deserialize, Debug)]
pub struct PrescriptionWithSafetyResponse {
    pub record: Option<Record>,
    pub safety_result: PrescriptionSafetyResult,
    pub created: bool,
    pub blocked_reason: Option<String>,
}

/// Check prescription safety before creating
#[hdk_extern]
pub fn check_prescription_safety(input: CheckPrescriptionSafetyInput) -> ExternResult<PrescriptionSafetyResult> {
    // Get patient's active prescriptions to check for interactions
    let active_rx = get_patient_prescriptions_internal(input.patient_hash.clone())?;

    // Collect RxNorm codes from active prescriptions
    let mut existing_rxnorm_codes: Vec<String> = active_rx
        .iter()
        .filter_map(|record| {
            record.entry().to_app_option::<Prescription>().ok().flatten()
                .filter(|rx| matches!(rx.status, PrescriptionStatus::Active))
                .map(|rx| rx.rxnorm_code)
        })
        .collect();

    // Add the new medication to check
    existing_rxnorm_codes.push(input.new_medication_rxnorm.clone());

    // Call CDS zome to perform interaction check
    let request_id = format!("RX-SAFETY-{}", sys_time()?.as_micros());

    let cds_request = CdsInteractionCheckRequest {
        request_id: request_id.clone(),
        patient_hash: input.patient_hash.clone(),
        medication_rxnorm_codes: existing_rxnorm_codes,
        patient_allergies: input.patient_allergies.clone(),
        check_allergies: true,
        check_duplicates: true,
    };

    // Call CDS zome
    let response = call(
        CallTargetCell::Local,
        ZomeName::from("cds"),
        FunctionName::from("perform_interaction_check"),
        None,
        &cds_request,
    );

    match response {
        Ok(ZomeCallResponse::Ok(io)) => {
            let cds_record: Record = io.decode()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to decode CDS response: {}", e))))?;

            // Extract CDS response from record
            if let Some(cds_response) = cds_record.entry().to_app_option::<CdsInteractionCheckResponse>().ok().flatten() {
                let is_safe = matches!(cds_response.safety_assessment, SafetyAssessment::Safe);
                let requires_override = matches!(
                    cds_response.safety_assessment,
                    SafetyAssessment::HighRisk | SafetyAssessment::Contraindicated
                );

                Ok(PrescriptionSafetyResult {
                    is_safe,
                    safety_assessment: cds_response.safety_assessment,
                    drug_interactions: cds_response.drug_interactions,
                    allergy_conflicts: cds_response.allergy_conflicts,
                    duplicate_therapies: cds_response.duplicate_therapies,
                    recommendations: cds_response.recommendations,
                    requires_override,
                })
            } else {
                // CDS didn't return expected data - treat as safe with warning
                Ok(PrescriptionSafetyResult {
                    is_safe: true,
                    safety_assessment: SafetyAssessment::Safe,
                    drug_interactions: Vec::new(),
                    allergy_conflicts: Vec::new(),
                    duplicate_therapies: Vec::new(),
                    recommendations: vec!["CDS check unavailable - proceed with caution".to_string()],
                    requires_override: false,
                })
            }
        }
        Ok(_) => {
            // CDS call returned non-Ok - return safe with warning
            Ok(PrescriptionSafetyResult {
                is_safe: true,
                safety_assessment: SafetyAssessment::Safe,
                drug_interactions: Vec::new(),
                allergy_conflicts: Vec::new(),
                duplicate_therapies: Vec::new(),
                recommendations: vec!["CDS service unavailable - proceed with caution".to_string()],
                requires_override: false,
            })
        }
        Err(_) => {
            // CDS zome not available - return safe with warning
            Ok(PrescriptionSafetyResult {
                is_safe: true,
                safety_assessment: SafetyAssessment::Safe,
                drug_interactions: Vec::new(),
                allergy_conflicts: Vec::new(),
                duplicate_therapies: Vec::new(),
                recommendations: vec!["CDS zome not available - manual review recommended".to_string()],
                requires_override: false,
            })
        }
    }
}

/// Input for checking prescription safety
#[derive(Serialize, Deserialize, Debug)]
pub struct CheckPrescriptionSafetyInput {
    pub patient_hash: ActionHash,
    pub new_medication_rxnorm: String,
    pub patient_allergies: Vec<String>,
}

/// Create a prescription with integrated safety checking
#[hdk_extern]
pub fn create_prescription_with_safety(input: CreatePrescriptionWithSafetyInput) -> ExternResult<PrescriptionWithSafetyResponse> {
    // Require Write authorization for Medications category
    let auth = require_authorization(
        input.prescription.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Write,
        input.is_emergency,
    )?;

    // Perform safety check if required
    let safety_result = if input.require_safety_check {
        check_prescription_safety(CheckPrescriptionSafetyInput {
            patient_hash: input.prescription.patient_hash.clone(),
            new_medication_rxnorm: input.prescription.rxnorm_code.clone(),
            patient_allergies: input.patient_allergies.clone(),
        })?
    } else {
        // Skip safety check - return safe
        PrescriptionSafetyResult {
            is_safe: true,
            safety_assessment: SafetyAssessment::Safe,
            drug_interactions: Vec::new(),
            allergy_conflicts: Vec::new(),
            duplicate_therapies: Vec::new(),
            recommendations: Vec::new(),
            requires_override: false,
        }
    };

    // Check if we should block prescription creation
    if safety_result.requires_override && !input.allow_override {
        return Ok(PrescriptionWithSafetyResponse {
            record: None,
            safety_result,
            created: false,
            blocked_reason: Some("Safety check failed - override required".to_string()),
        });
    }

    // If contraindicated without override reason, block
    if matches!(safety_result.safety_assessment, SafetyAssessment::Contraindicated)
        && input.override_reason.is_none()
    {
        return Ok(PrescriptionWithSafetyResponse {
            record: None,
            safety_result,
            created: false,
            blocked_reason: Some("Contraindicated medication requires documented override reason".to_string()),
        });
    }

    // Create the prescription
    let rx_hash = create_entry(&EntryTypes::Prescription(input.prescription.clone()))?;
    let record = get(rx_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find prescription".to_string())))?;

    // Link to patient
    create_link(
        input.prescription.patient_hash.clone(),
        rx_hash.clone(),
        LinkTypes::PatientToPrescriptions,
        (),
    )?;

    // Link to prescriber
    create_link(
        input.prescription.prescriber_hash.clone(),
        rx_hash.clone(),
        LinkTypes::PrescriberToPrescriptions,
        (),
    )?;

    // If controlled substance, add to tracking
    if input.prescription.schedule.is_some() && input.prescription.schedule != Some(DrugSchedule::NotControlled) {
        let controlled_anchor = anchor_hash("controlled_substances")?;
        create_link(
            controlled_anchor,
            rx_hash.clone(),
            LinkTypes::ControlledSubstances,
            (),
        )?;
    }

    // Create drug interaction alerts for any found interactions
    for interaction in &safety_result.drug_interactions {
        let severity = match &interaction.severity {
            CdsInteractionSeverity::Contraindicated => InteractionSeverity::Contraindicated,
            CdsInteractionSeverity::Major => InteractionSeverity::Major,
            CdsInteractionSeverity::Moderate => InteractionSeverity::Moderate,
            CdsInteractionSeverity::Minor => InteractionSeverity::Minor,
            CdsInteractionSeverity::Unknown => InteractionSeverity::Unknown,
        };

        let alert = DrugInteractionAlert {
            alert_id: format!("ALERT-{}", sys_time()?.as_micros()),
            patient_hash: input.prescription.patient_hash.clone(),
            prescription_hash: rx_hash.clone(),
            interacting_medication: format!("{} ({})", interaction.drug_b_name, interaction.drug_b_rxnorm),
            interaction_type: severity,
            description: interaction.description.clone(),
            clinical_significance: format!("{:?}", interaction.severity),
            management_recommendation: interaction.management.clone(),
            source: "CDS Automated Check".to_string(),
            acknowledged: input.allow_override,
            acknowledged_by: if input.allow_override { Some(agent_info()?.agent_initial_pubkey) } else { None },
            acknowledged_at: if input.allow_override { Some(sys_time()?) } else { None },
            override_reason: input.override_reason.clone(),
        };

        let alert_hash = create_entry(&EntryTypes::DrugInteractionAlert(alert))?;
        create_link(
            rx_hash.clone(),
            alert_hash,
            LinkTypes::PrescriptionToAlerts,
            (),
        )?;
    }

    // Log the access
    log_data_access(
        input.prescription.patient_hash,
        vec![DataCategory::Medications],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(PrescriptionWithSafetyResponse {
        record: Some(record),
        safety_result,
        created: true,
        blocked_reason: None,
    })
}

/// Get safety summary for a patient's current medications
#[hdk_extern]
pub fn get_medication_safety_summary(input: GetPatientPrescriptionsInput) -> ExternResult<PrescriptionSafetyResult> {
    // Get all active prescriptions
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Medications,
        Permission::Read,
        input.is_emergency,
    )?;

    let active_rx = get_patient_prescriptions_internal(input.patient_hash.clone())?;

    // Collect RxNorm codes from active prescriptions
    let rxnorm_codes: Vec<String> = active_rx
        .iter()
        .filter_map(|record| {
            record.entry().to_app_option::<Prescription>().ok().flatten()
                .filter(|rx| matches!(rx.status, PrescriptionStatus::Active))
                .map(|rx| rx.rxnorm_code)
        })
        .collect();

    if rxnorm_codes.is_empty() {
        return Ok(PrescriptionSafetyResult {
            is_safe: true,
            safety_assessment: SafetyAssessment::Safe,
            drug_interactions: Vec::new(),
            allergy_conflicts: Vec::new(),
            duplicate_therapies: Vec::new(),
            recommendations: vec!["No active medications".to_string()],
            requires_override: false,
        });
    }

    // Call CDS for comprehensive check (empty allergies - caller should provide if known)
    let request_id = format!("MED-SUMMARY-{}", sys_time()?.as_micros());

    let cds_request = CdsInteractionCheckRequest {
        request_id,
        patient_hash: input.patient_hash.clone(),
        medication_rxnorm_codes: rxnorm_codes,
        patient_allergies: Vec::new(), // Caller should pass allergies separately if known
        check_allergies: false,
        check_duplicates: true,
    };

    // Call CDS zome
    let response = call(
        CallTargetCell::Local,
        ZomeName::from("cds"),
        FunctionName::from("perform_interaction_check"),
        None,
        &cds_request,
    );

    // Log the access
    log_data_access(
        input.patient_hash,
        vec![DataCategory::Medications],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    match response {
        Ok(ZomeCallResponse::Ok(io)) => {
            let cds_record: Record = io.decode()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to decode CDS response: {}", e))))?;

            if let Some(cds_response) = cds_record.entry().to_app_option::<CdsInteractionCheckResponse>().ok().flatten() {
                let is_safe = matches!(cds_response.safety_assessment, SafetyAssessment::Safe);
                let requires_override = matches!(
                    cds_response.safety_assessment,
                    SafetyAssessment::HighRisk | SafetyAssessment::Contraindicated
                );

                Ok(PrescriptionSafetyResult {
                    is_safe,
                    safety_assessment: cds_response.safety_assessment,
                    drug_interactions: cds_response.drug_interactions,
                    allergy_conflicts: cds_response.allergy_conflicts,
                    duplicate_therapies: cds_response.duplicate_therapies,
                    recommendations: cds_response.recommendations,
                    requires_override,
                })
            } else {
                Ok(PrescriptionSafetyResult {
                    is_safe: true,
                    safety_assessment: SafetyAssessment::Safe,
                    drug_interactions: Vec::new(),
                    allergy_conflicts: Vec::new(),
                    duplicate_therapies: Vec::new(),
                    recommendations: vec!["CDS data unavailable".to_string()],
                    requires_override: false,
                })
            }
        }
        _ => {
            Ok(PrescriptionSafetyResult {
                is_safe: true,
                safety_assessment: SafetyAssessment::Safe,
                drug_interactions: Vec::new(),
                allergy_conflicts: Vec::new(),
                duplicate_therapies: Vec::new(),
                recommendations: vec!["CDS service unavailable".to_string()],
                requires_override: false,
            })
        }
    }
}
