//! Prescriptions Coordinator Zome
//!
//! Provides extern functions for prescription management,
//! pharmacy interactions, and medication adherence tracking.
//!
//! All prescription data access enforces consent-based access control
//! per HIPAA requirements. Controlled substance tracking has additional
//! audit requirements.

use hdk::prelude::*;
use prescriptions_integrity::*;
use mycelix_health_shared::{
    require_authorization, require_admin_authorization,
    log_data_access,
    DataCategory, Permission,
};

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
