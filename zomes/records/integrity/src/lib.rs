// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Medical Records and Health Data Integrity Zome
//! 
//! Defines entry types for medical records, encounters, diagnoses,
//! procedures, lab results, and imaging with HL7 FHIR alignment.

use hdi::prelude::*;

/// Medical encounter/visit record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Encounter {
    pub encounter_id: String,
    pub patient_hash: ActionHash,
    pub provider_hash: ActionHash,
    pub encounter_type: EncounterType,
    pub status: EncounterStatus,
    pub start_time: Timestamp,
    pub end_time: Option<Timestamp>,
    pub location: Option<String>,
    pub chief_complaint: String,
    pub diagnoses: Vec<Diagnosis>,
    pub procedures: Vec<ProcedurePerformed>,
    pub notes: String,
    /// Consent hash authorizing this record
    pub consent_hash: ActionHash,
    /// Epistemic classification of the encounter documentation
    pub epistemic_level: EpistemicLevel,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EncounterType {
    Office,
    Emergency,
    Inpatient,
    Outpatient,
    Telehealth,
    HomeVisit,
    Procedure,
    Surgery,
    LabOnly,
    ImagingOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EncounterStatus {
    Planned,
    InProgress,
    Completed,
    Cancelled,
    NoShow,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EpistemicLevel {
    /// E0: Unverified patient-reported
    PatientReported,
    /// E1: Provider observation
    ProviderObserved,
    /// E2: Lab/test confirmed
    TestConfirmed,
    /// E3: Multi-provider consensus
    Consensus,
}

/// Diagnosis entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Diagnosis {
    pub diagnosis_id: String,
    pub patient_hash: ActionHash,
    pub encounter_hash: Option<ActionHash>,
    /// ICD-10 code
    pub icd10_code: String,
    /// SNOMED CT code if available
    pub snomed_code: Option<String>,
    pub description: String,
    pub diagnosis_type: DiagnosisType,
    pub status: DiagnosisStatus,
    pub onset_date: Option<String>,
    pub resolution_date: Option<String>,
    pub diagnosing_provider: AgentPubKey,
    pub severity: Option<DiagnosisSeverity>,
    pub notes: Option<String>,
    pub epistemic_level: EpistemicLevel,
    pub created_at: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DiagnosisType {
    Primary,
    Secondary,
    Differential,
    RuledOut,
    WorkingDiagnosis,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DiagnosisStatus {
    Active,
    Resolved,
    Inactive,
    Recurrence,
    Remission,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DiagnosisSeverity {
    Mild,
    Moderate,
    Severe,
    Critical,
}

/// Procedure record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ProcedurePerformed {
    pub procedure_id: String,
    pub patient_hash: ActionHash,
    pub encounter_hash: ActionHash,
    /// CPT code
    pub cpt_code: String,
    /// HCPCS code if applicable
    pub hcpcs_code: Option<String>,
    pub description: String,
    pub performed_by: AgentPubKey,
    pub performed_at: Timestamp,
    pub location: String,
    pub outcome: ProcedureOutcome,
    pub complications: Vec<String>,
    pub notes: Option<String>,
    pub consent_hash: ActionHash,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ProcedureOutcome {
    Successful,
    PartialSuccess,
    Unsuccessful,
    Complicated,
    Aborted,
}

/// Lab result entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct LabResult {
    pub result_id: String,
    pub patient_hash: ActionHash,
    pub encounter_hash: Option<ActionHash>,
    pub ordering_provider: AgentPubKey,
    /// LOINC code for the test
    pub loinc_code: String,
    pub test_name: String,
    pub value: String,
    pub unit: String,
    pub reference_range: String,
    pub interpretation: LabInterpretation,
    pub specimen_type: String,
    pub collection_time: Timestamp,
    pub result_time: Timestamp,
    pub performing_lab: String,
    pub notes: Option<String>,
    /// Critical results must be flagged
    pub is_critical: bool,
    pub acknowledged_by: Option<AgentPubKey>,
    pub acknowledged_at: Option<Timestamp>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum LabInterpretation {
    Normal,
    Abnormal,
    High,
    Low,
    Critical,
    Inconclusive,
}

/// Imaging study result
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ImagingStudy {
    pub study_id: String,
    pub patient_hash: ActionHash,
    pub encounter_hash: Option<ActionHash>,
    pub ordering_provider: AgentPubKey,
    pub modality: ImagingModality,
    pub body_site: String,
    pub laterality: Option<Laterality>,
    pub study_date: Timestamp,
    pub indication: String,
    pub findings: String,
    pub impression: String,
    pub interpreting_radiologist: AgentPubKey,
    pub report_date: Timestamp,
    /// DICOM study instance UID
    pub dicom_uid: Option<String>,
    /// Link to image storage (encrypted reference)
    pub image_reference: Option<String>,
    pub is_critical: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ImagingModality {
    XRay,
    CT,
    MRI,
    Ultrasound,
    PET,
    Mammography,
    Fluoroscopy,
    NuclearMedicine,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Laterality {
    Left,
    Right,
    Bilateral,
}

/// Vital signs record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct VitalSigns {
    pub patient_hash: ActionHash,
    pub encounter_hash: Option<ActionHash>,
    pub recorded_at: Timestamp,
    pub recorded_by: AgentPubKey,
    pub temperature_celsius: Option<f64>,
    pub heart_rate_bpm: Option<u32>,
    pub blood_pressure_systolic: Option<u32>,
    pub blood_pressure_diastolic: Option<u32>,
    pub respiratory_rate: Option<u32>,
    pub oxygen_saturation: Option<f64>,
    pub height_cm: Option<f64>,
    pub weight_kg: Option<f64>,
    pub bmi: Option<f64>,
    pub pain_level: Option<u8>,
    pub notes: Option<String>,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Encounter(Encounter),
    Diagnosis(Diagnosis),
    ProcedurePerformed(ProcedurePerformed),
    LabResult(LabResult),
    ImagingStudy(ImagingStudy),
    VitalSigns(VitalSigns),
    /// Encrypted health record — stores any health entry encrypted with patient's key.
    /// The ciphertext can only be decrypted by the patient or authorized consent holders.
    /// The `data_category` is stored in cleartext for consent-checking without decryption.
    EncryptedRecord(EncryptedRecord),
}

/// An encrypted health record. The actual clinical data (lab result, encounter, etc.)
/// is serialized to MessagePack, then encrypted with the patient's public key.
/// Only the patient or a consent-granted agent can decrypt it.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct EncryptedRecord {
    /// Patient this record belongs to.
    pub patient_hash: ActionHash,
    /// Key fingerprint used for encryption (first 8 bytes of BLAKE3 hash of public key).
    pub key_fingerprint: [u8; 8],
    /// Encrypted payload (serialized health entry + XChaCha20-Poly1305).
    pub ciphertext: Vec<u8>,
    /// Nonce for decryption (24 bytes).
    pub nonce: [u8; 24],
    /// Data category in cleartext — allows consent checking without decryption.
    pub data_category: String,
    /// Original entry type name (e.g., "LabResult", "Encounter") — for deserialization after decryption.
    pub entry_type: String,
    /// Encrypted at (microseconds since UNIX epoch).
    pub encrypted_at: i64,
}

#[hdk_link_types]
pub enum LinkTypes {
    PatientToEncounters,
    EncounterToDiagnoses,
    EncounterToProcedures,
    PatientToLabResults,
    PatientToImaging,
    PatientToVitals,
    ProviderToEncounters,
    DiagnosisUpdates,
    EncounterUpdates,
    LabResultUpdates,
    CriticalResults,
    /// Patient → encrypted records
    PatientToEncryptedRecords,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::Encounter(e) => validate_encounter(&e),
                EntryTypes::Diagnosis(d) => validate_diagnosis(&d),
                EntryTypes::ProcedurePerformed(p) => validate_procedure(&p),
                EntryTypes::LabResult(l) => validate_lab_result(&l),
                EntryTypes::ImagingStudy(i) => validate_imaging(&i),
                EntryTypes::VitalSigns(v) => validate_vitals(&v),
                EntryTypes::EncryptedRecord(e) => validate_encrypted_record(&e),
            },
            OpEntry::UpdateEntry { app_entry, .. } => match app_entry {
                EntryTypes::Encounter(e) => validate_encounter(&e),
                EntryTypes::Diagnosis(d) => validate_diagnosis(&d),
                EntryTypes::ProcedurePerformed(p) => validate_procedure(&p),
                EntryTypes::LabResult(l) => validate_lab_result(&l),
                EntryTypes::ImagingStudy(i) => validate_imaging(&i),
                EntryTypes::VitalSigns(v) => validate_vitals(&v),
                EntryTypes::EncryptedRecord(e) => validate_encrypted_record(&e),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_encounter(encounter: &Encounter) -> ExternResult<ValidateCallbackResult> {
    if encounter.encounter_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Encounter ID is required".to_string(),
        ));
    }
    if encounter.chief_complaint.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Chief complaint is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_diagnosis(diagnosis: &Diagnosis) -> ExternResult<ValidateCallbackResult> {
    if diagnosis.icd10_code.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ICD-10 code is required".to_string(),
        ));
    }
    if diagnosis.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Diagnosis description is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_procedure(_procedure: &ProcedurePerformed) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

fn validate_lab_result(lab: &LabResult) -> ExternResult<ValidateCallbackResult> {
    if lab.loinc_code.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "LOINC code is required for lab results".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_imaging(_imaging: &ImagingStudy) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

fn validate_vitals(vitals: &VitalSigns) -> ExternResult<ValidateCallbackResult> {
    // Validate reasonable ranges
    if let Some(hr) = vitals.heart_rate_bpm {
        if hr < 20 || hr > 300 {
            return Ok(ValidateCallbackResult::Invalid(
                "Heart rate out of valid range".to_string(),
            ));
        }
    }
    if let Some(o2) = vitals.oxygen_saturation {
        if o2 < 0.0 || o2 > 100.0 {
            return Ok(ValidateCallbackResult::Invalid(
                "Oxygen saturation must be 0-100%".to_string(),
            ));
        }
    }
    if let Some(pain) = vitals.pain_level {
        if pain > 10 {
            return Ok(ValidateCallbackResult::Invalid(
                "Pain level must be 0-10".to_string(),
            ));
        }
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_encrypted_record(record: &EncryptedRecord) -> ExternResult<ValidateCallbackResult> {
    if record.ciphertext.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Ciphertext cannot be empty".to_string(),
        ));
    }
    if record.entry_type.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Entry type must be specified for decryption routing".to_string(),
        ));
    }
    if record.data_category.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Data category required for consent checking".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
