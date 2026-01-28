//! Patient Identity and Demographics Coordinator Zome
//!
//! Provides extern functions for patient CRUD operations,
//! identity linking, and health summary management.
//!
//! All data access functions enforce consent-based access control.

use hdk::prelude::*;
use patient_integrity::*;
use mycelix_health_shared::{
    require_authorization, require_admin_authorization,
    log_data_access,
    DataCategory, Permission, GetPatientInput,
};

/// Create a new patient profile
#[hdk_extern]
pub fn create_patient(patient: Patient) -> ExternResult<Record> {
    let patient_hash = create_entry(&EntryTypes::Patient(patient.clone()))?;
    let record = get(patient_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find newly created patient".to_string())))?;
    
    // Link to global patients anchor
    let patients_anchor = anchor_hash("all_patients")?;
    create_link(
        patients_anchor,
        patient_hash,
        LinkTypes::AllPatients,
        (),
    )?;
    
    Ok(record)
}

/// Get a patient by their action hash (without access control - internal use only)
fn get_patient_internal(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(patient_hash, GetOptions::default())
}

/// Get a patient by their action hash with consent-based access control
#[hdk_extern]
pub fn get_patient(input: GetPatientInput) -> ExternResult<Option<Record>> {
    // Require authorization before accessing PHI
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Demographics,
        Permission::Read,
        input.is_emergency,
    )?;

    // Get the patient data
    let record = get_patient_internal(input.patient_hash.clone())?;

    // Log the access for audit trail
    if record.is_some() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::Demographics],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            input.emergency_reason,
        )?;
    }

    Ok(record)
}

/// Input for updating a patient with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdatePatientInput {
    pub original_hash: ActionHash,
    pub updated_patient: Patient,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Update an existing patient with consent-based access control
#[hdk_extern]
pub fn update_patient(input: UpdatePatientInput) -> ExternResult<Record> {
    // Require Write authorization before modifying PHI
    let auth = require_authorization(
        input.original_hash.clone(),
        DataCategory::Demographics,
        Permission::Write,
        input.is_emergency,
    )?;

    let updated_hash = update_entry(input.original_hash.clone(), &input.updated_patient)?;
    let record = get(updated_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated patient".to_string())))?;

    // Create update link for history tracking
    create_link(
        input.original_hash.clone(),
        updated_hash,
        LinkTypes::PatientUpdates,
        (),
    )?;

    // Log the access for audit trail
    log_data_access(
        input.original_hash,
        vec![DataCategory::Demographics],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(record)
}

/// Input for deleting a patient with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct DeletePatientInput {
    pub patient_hash: ActionHash,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Delete a patient (soft delete - mark as inactive) with access control
#[hdk_extern]
pub fn delete_patient(input: DeletePatientInput) -> ExternResult<ActionHash> {
    // Require Delete authorization
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::All,
        Permission::Delete,
        input.is_emergency,
    )?;

    let result = delete_entry(input.patient_hash.clone())?;

    // Log the deletion for audit trail
    log_data_access(
        input.patient_hash,
        vec![DataCategory::All],
        Permission::Delete,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(result)
}

/// Get all patients (admin function - requires admin authorization)
#[hdk_extern]
pub fn get_all_patients(_: ()) -> ExternResult<Vec<Record>> {
    // Require admin authorization for bulk patient access
    require_admin_authorization()?;

    let patients_anchor = anchor_hash("all_patients")?;
    let links = get_links(LinkQuery::try_new(patients_anchor, LinkTypes::AllPatients)?, GetStrategy::default())?;

    let mut patients = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                patients.push(record);
            }
        }
    }

    Ok(patients)
}

/// Internal version without access control for internal queries
fn get_all_patients_internal() -> ExternResult<Vec<Record>> {
    let patients_anchor = anchor_hash("all_patients")?;
    let links = get_links(LinkQuery::try_new(patients_anchor, LinkTypes::AllPatients)?, GetStrategy::default())?;

    let mut patients = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                patients.push(record);
            }
        }
    }

    Ok(patients)
}

/// Input for searching patients with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct SearchPatientsInput {
    pub name: String,
}

/// Search patients by name (requires admin authorization for bulk search)
#[hdk_extern]
pub fn search_patients_by_name(input: SearchPatientsInput) -> ExternResult<Vec<Record>> {
    // Require admin authorization for patient search (accessing multiple PHI records)
    require_admin_authorization()?;

    let all_patients = get_all_patients_internal()?;
    let name_lower = input.name.to_lowercase();

    let filtered: Vec<Record> = all_patients
        .into_iter()
        .filter(|record| {
            if let Some(patient) = record.entry().to_app_option::<Patient>().ok().flatten() {
                patient.first_name.to_lowercase().contains(&name_lower)
                    || patient.last_name.to_lowercase().contains(&name_lower)
            } else {
                false
            }
        })
        .collect();

    Ok(filtered)
}

/// Link patient to Mycelix identity with bidirectional DID ↔ Patient links
///
/// This creates:
/// 1. A PatientIdentityLink entry with verification details
/// 2. A PatientToDID link from patient to DID anchor for forward lookup
/// 3. A DIDToPatient link from DID anchor to patient for reverse lookup
/// 4. A PatientToIdentityLink link from patient to the identity link record
#[hdk_extern]
pub fn link_patient_to_identity(input: LinkIdentityInput) -> ExternResult<Record> {
    let link = PatientIdentityLink {
        patient_hash: input.patient_hash.clone(),
        did: input.did.clone(),
        identity_provider: input.identity_provider,
        verified_at: sys_time()?,
        verification_method: input.verification_method,
        confidence_score: input.confidence_score,
    };

    let link_hash = create_entry(&EntryTypes::PatientIdentityLink(link))?;
    let record = get(link_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find identity link".to_string())))?;

    // Create DID anchor for bidirectional lookups
    let did_anchor = anchor_hash(&format!("did:{}", input.did))?;

    // Link from Patient → DID anchor (forward lookup)
    create_link(
        input.patient_hash.clone(),
        did_anchor.clone(),
        LinkTypes::PatientToDID,
        input.did.as_bytes().to_vec(),
    )?;

    // Link from DID anchor → Patient (reverse lookup for cross-domain resolution)
    create_link(
        did_anchor,
        input.patient_hash.clone(),
        LinkTypes::DIDToPatient,
        (),
    )?;

    // Link from Patient → IdentityLink record
    create_link(
        input.patient_hash,
        link_hash,
        LinkTypes::PatientToIdentityLink,
        (),
    )?;

    Ok(record)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LinkIdentityInput {
    pub patient_hash: ActionHash,
    /// Decentralized Identifier from Mycelix Identity hApp
    /// Format: did:mycelix:<agent_pub_key_b64> or did:web:<domain>
    pub did: String,
    pub identity_provider: String,
    pub verification_method: String,
    pub confidence_score: f64,
}

/// Input for looking up patient by DID
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPatientByDIDInput {
    /// The DID to look up (e.g., did:mycelix:abc123 or did:web:example.com)
    pub did: String,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get patient by their Decentralized Identifier (DID)
///
/// This enables cross-domain identity resolution:
/// - Mycelix Identity hApp can look up a patient's health records by DID
/// - Other hApps can verify a patient's identity before accessing health data
/// - Supports did:mycelix: and did:web: methods
#[hdk_extern]
pub fn get_patient_by_did(input: GetPatientByDIDInput) -> ExternResult<Option<Record>> {
    // Create the DID anchor hash for lookup
    let did_anchor = anchor_hash(&format!("did:{}", input.did))?;

    // Get links from DID anchor to patient(s)
    let links = get_links(
        LinkQuery::try_new(did_anchor, LinkTypes::DIDToPatient)?,
        GetStrategy::default(),
    )?;

    // Return the first linked patient (DIDs should map to exactly one patient)
    for link in links {
        if let Some(patient_hash) = link.target.into_action_hash() {
            // Check authorization before returning patient data
            let auth = require_authorization(
                patient_hash.clone(),
                DataCategory::Demographics,
                Permission::Read,
                input.is_emergency,
            )?;

            if let Some(record) = get_patient_internal(patient_hash.clone())? {
                // Log the access for audit trail
                log_data_access(
                    patient_hash,
                    vec![DataCategory::Demographics],
                    Permission::Read,
                    auth.consent_hash,
                    auth.emergency_override,
                    input.emergency_reason.clone(),
                )?;

                return Ok(Some(record));
            }
        }
    }

    Ok(None)
}

/// Get DID for a patient
#[derive(Serialize, Deserialize, Debug)]
pub struct GetDIDForPatientInput {
    pub patient_hash: ActionHash,
}

/// Output containing patient's DID information
#[derive(Serialize, Deserialize, Debug)]
pub struct PatientDIDInfo {
    pub patient_hash: ActionHash,
    pub did: String,
    pub identity_provider: String,
    pub verified_at: Timestamp,
    pub confidence_score: f64,
}

/// Get the DID associated with a patient
///
/// Returns the patient's verified DID for cross-domain identity sharing
#[hdk_extern]
pub fn get_did_for_patient(input: GetDIDForPatientInput) -> ExternResult<Option<PatientDIDInfo>> {
    // Get identity links for this patient
    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToIdentityLink)?,
        GetStrategy::default(),
    )?;

    // Return the most recent identity link
    for link in links {
        if let Some(link_hash) = link.target.into_action_hash() {
            if let Some(record) = get(link_hash, GetOptions::default())? {
                if let Some(identity_link) = record.entry().to_app_option::<PatientIdentityLink>().ok().flatten() {
                    return Ok(Some(PatientDIDInfo {
                        patient_hash: input.patient_hash,
                        did: identity_link.did,
                        identity_provider: identity_link.identity_provider,
                        verified_at: identity_link.verified_at,
                        confidence_score: identity_link.confidence_score,
                    }));
                }
            }
        }
    }

    Ok(None)
}

/// Verify that a DID is linked to a specific patient
#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyDIDPatientLinkInput {
    pub did: String,
    pub patient_hash: ActionHash,
}

/// Verify that a DID is linked to the specified patient
///
/// Returns true if the DID is linked to the patient with high confidence
#[hdk_extern]
pub fn verify_did_patient_link(input: VerifyDIDPatientLinkInput) -> ExternResult<bool> {
    // Look up patient by DID
    let did_anchor = anchor_hash(&format!("did:{}", input.did))?;

    let links = get_links(
        LinkQuery::try_new(did_anchor, LinkTypes::DIDToPatient)?,
        GetStrategy::default(),
    )?;

    // Check if any link points to the specified patient
    for link in links {
        if let Some(linked_patient_hash) = link.target.into_action_hash() {
            if linked_patient_hash == input.patient_hash {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Create patient health summary
#[hdk_extern]
pub fn create_health_summary(summary: PatientHealthSummary) -> ExternResult<Record> {
    let summary_hash = create_entry(&EntryTypes::PatientHealthSummary(summary.clone()))?;
    let record = get(summary_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find health summary".to_string())))?;
    
    Ok(record)
}

/// Get patient's health summary with access control
#[hdk_extern]
pub fn get_patient_health_summary(input: GetPatientInput) -> ExternResult<Option<Record>> {
    // Require authorization - health summary includes sensitive data
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::All, // Health summary includes all categories
        Permission::Read,
        input.is_emergency,
    )?;

    // Get the summary
    let record = get(input.patient_hash.clone(), GetOptions::default())?;

    // Log the access
    if record.is_some() {
        log_data_access(
            input.patient_hash,
            vec![DataCategory::All],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            input.emergency_reason,
        )?;
    }

    Ok(record)
}

/// Input for adding allergy with access control
#[derive(Serialize, Deserialize, Debug)]
pub struct AddAllergyInput {
    pub patient_hash: ActionHash,
    pub allergy: Allergy,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Add allergy to patient record with access control
#[hdk_extern]
pub fn add_patient_allergy(input: AddAllergyInput) -> ExternResult<Record> {
    // Require Write authorization for Allergies category
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Allergies,
        Permission::Write,
        input.is_emergency,
    )?;

    let record = get_patient_internal(input.patient_hash.clone())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Patient not found".to_string())))?;

    let mut patient: Patient = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid patient entry".to_string())))?;

    patient.allergies.push(input.allergy);
    patient.updated_at = sys_time()?;

    let updated_hash = update_entry(input.patient_hash.clone(), &patient)?;
    let updated_record = get(updated_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated patient".to_string())))?;

    create_link(
        input.patient_hash.clone(),
        updated_hash,
        LinkTypes::PatientUpdates,
        (),
    )?;

    // Log the access
    log_data_access(
        input.patient_hash,
        vec![DataCategory::Allergies],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        input.emergency_reason,
    )?;

    Ok(updated_record)
}

/// Input for looking up patient by MRN
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPatientByMrnInput {
    pub mrn: String,
    pub is_emergency: bool,
    pub emergency_reason: Option<String>,
}

/// Get patient by MRN with access control
#[hdk_extern]
pub fn get_patient_by_mrn(input: GetPatientByMrnInput) -> ExternResult<Option<Record>> {
    // Note: This function searches all patients to find by MRN
    // which requires admin authorization for the search itself
    require_admin_authorization()?;

    let all_patients = get_all_patients_internal()?;

    for record in all_patients {
        if let Some(patient) = record.entry().to_app_option::<Patient>().ok().flatten() {
            if patient.mrn == Some(input.mrn.clone()) {
                // Found the patient - now check if caller has access to this specific patient
                let patient_hash = record.action_address().clone();

                let auth = require_authorization(
                    patient_hash.clone(),
                    DataCategory::Demographics,
                    Permission::Read,
                    input.is_emergency,
                )?;

                // Log the access
                log_data_access(
                    patient_hash,
                    vec![DataCategory::Demographics],
                    Permission::Read,
                    auth.consent_hash,
                    auth.emergency_override,
                    input.emergency_reason.clone(),
                )?;

                return Ok(Some(record));
            }
        }
    }

    Ok(None)
}

// Helper function to create anchor hash
/// Anchor entry for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}
