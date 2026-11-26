#![deny(unsafe_code)]
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
    /// Encrypted health record.
    EncryptedRecord(EncryptedRecord),
    /// Amendment request (HIPAA right to amend).
    AmendmentRequest(AmendmentRequestEntry),
    /// SDOH screening result.
    SdohScreening(SdohScreeningEntry),
}

/// An encrypted health record. The actual clinical data (lab result, encounter, etc.)
/// is serialized to MessagePack, then encrypted with the patient's public key.
/// Only the patient or a consent-granted agent can decrypt it.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct EncryptedRecord {
    /// Envelope format version. Version 1 authenticates the legacy routing metadata;
    /// version 2 also authenticates clinical event time and encounter identity.
    pub envelope_version: u8,
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
    /// Clinical event time, distinct from local ingestion/encryption time.
    /// Absent on legacy v1 records; never inferred from encrypted_at.
    #[serde(default)]
    pub clinical_event_time: Option<Timestamp>,
    /// Exact encounter containing the resource, where one exists.
    /// Absent on legacy v1 records and records without a resolvable encounter.
    #[serde(default)]
    pub encounter_hash: Option<ActionHash>,
}

/// Canonical v1 AAD. Keep this wire shape unchanged so previously stored v1
/// ciphertext continues to authenticate with the exact bytes used originally.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EncryptedRecordAad {
    pub envelope_version: u8,
    pub patient_hash: ActionHash,
    pub key_fingerprint: [u8; 8],
    pub data_category: String,
    pub entry_type: String,
    pub encrypted_at: i64,
}

/// Canonical v2 AAD. Clinical event time and encounter are authenticated by
/// XChaCha20-Poly1305 and cannot be changed without invalidating the tag.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EncryptedRecordAadV2 {
    pub envelope_version: u8,
    pub patient_hash: ActionHash,
    pub key_fingerprint: [u8; 8],
    pub data_category: String,
    pub entry_type: String,
    pub encrypted_at: i64,
    pub clinical_event_time: Timestamp,
    pub encounter_hash: Option<ActionHash>,
}

impl EncryptedRecord {
    /// Legacy v1 AAD. Deliberately excludes v2 fields to preserve old ciphertext.
    pub fn aad(&self) -> EncryptedRecordAad {
        EncryptedRecordAad {
            envelope_version: self.envelope_version,
            patient_hash: self.patient_hash.clone(),
            key_fingerprint: self.key_fingerprint,
            data_category: self.data_category.clone(),
            entry_type: self.entry_type.clone(),
            encrypted_at: self.encrypted_at,
        }
    }

    pub fn aad_v2(&self) -> Option<EncryptedRecordAadV2> {
        Some(EncryptedRecordAadV2 {
            envelope_version: self.envelope_version,
            patient_hash: self.patient_hash.clone(),
            key_fingerprint: self.key_fingerprint,
            data_category: self.data_category.clone(),
            entry_type: self.entry_type.clone(),
            encrypted_at: self.encrypted_at,
            clinical_event_time: self.clinical_event_time.clone()?,
            encounter_hash: self.encounter_hash.clone(),
        })
    }
}

/// Amendment request (HIPAA 45 CFR 164.526).
/// Patients can request changes to their health records.
/// Providers must respond within 60 days.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AmendmentRequestEntry {
    /// Record to amend.
    pub record_hash: ActionHash,
    /// Patient requesting.
    pub patient_hash: ActionHash,
    /// What the patient wants changed.
    pub requested_change: String,
    /// Reason for the amendment.
    pub reason: String,
    /// Status: Pending, Approved, Denied.
    pub status: AmendmentStatus,
    /// Provider who reviewed (set on decision).
    pub reviewer: Option<AgentPubKey>,
    /// Denial reason (required by HIPAA if denied).
    pub denial_reason: Option<String>,
    /// Hash of the amended record (if approved).
    pub amended_record_hash: Option<ActionHash>,
    /// When requested.
    pub requested_at: Timestamp,
    /// When decided.
    pub decided_at: Option<Timestamp>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AmendmentStatus {
    Pending,
    Approved,
    Denied,
}

/// SDOH screening record (CMS quality reporting).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SdohScreeningEntry {
    pub patient_hash: ActionHash,
    /// Instrument used (e.g., "PRAPARE", "AHC-HRSN").
    pub instrument: String,
    /// Individual responses serialized as JSON.
    pub responses_json: String,
    /// Risk domains identified.
    pub risk_domains: Vec<String>,
    /// Overall risk level.
    pub overall_risk: String,
    /// Referrals made.
    pub referrals_json: String,
    /// Screener agent.
    pub screener: AgentPubKey,
    /// When screened.
    pub screened_at: Timestamp,
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
    PatientToEncryptedRecords,
    PatientToAmendments,
    PatientToSdohScreenings,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => validate_entry(app_entry),
            OpEntry::UpdateEntry { app_entry, .. } => validate_entry(app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_entry(entry: EntryTypes) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::EncryptedRecord(record) => validate_encrypted_record(&record),
        EntryTypes::Encounter(entry) => {
            validate_plaintext_phi("Encounter", || validate_encounter(&entry))
        }
        EntryTypes::Diagnosis(entry) => {
            validate_plaintext_phi("Diagnosis", || validate_diagnosis(&entry))
        }
        EntryTypes::ProcedurePerformed(entry) => {
            validate_plaintext_phi("ProcedurePerformed", || validate_procedure(&entry))
        }
        EntryTypes::LabResult(entry) => {
            validate_plaintext_phi("LabResult", || validate_lab_result(&entry))
        }
        EntryTypes::ImagingStudy(entry) => {
            validate_plaintext_phi("ImagingStudy", || validate_imaging(&entry))
        }
        EntryTypes::VitalSigns(entry) => {
            validate_plaintext_phi("VitalSigns", || validate_vitals(&entry))
        }
        EntryTypes::AmendmentRequest(_) => {
            validate_plaintext_phi("AmendmentRequest", || Ok(ValidateCallbackResult::Valid))
        }
        EntryTypes::SdohScreening(_) => {
            validate_plaintext_phi("SdohScreening", || Ok(ValidateCallbackResult::Valid))
        }
    }
}

fn validate_plaintext_phi<F>(
    entry_type: &str,
    validation: F,
) -> ExternResult<ValidateCallbackResult>
where
    F: FnOnce() -> ExternResult<ValidateCallbackResult>,
{
    #[cfg(feature = "dangerous-plaintext-phi")]
    {
        let _ = entry_type;
        validation()
    }

    #[cfg(not(feature = "dangerous-plaintext-phi"))]
    {
        let _ = validation;
        Ok(ValidateCallbackResult::Invalid(format!(
            "Plaintext PHI entry {} is disabled; commit an EncryptedRecord envelope",
            entry_type
        )))
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
        if !(20..=300).contains(&hr) {
            return Ok(ValidateCallbackResult::Invalid(
                "Heart rate out of valid range".to_string(),
            ));
        }
    }
    if let Some(o2) = vitals.oxygen_saturation {
        if !(0.0..=100.0).contains(&o2) {
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
    match record.envelope_version {
        1 if record.clinical_event_time.is_none() && record.encounter_hash.is_none() => {}
        1 => {
            return Ok(ValidateCallbackResult::Invalid(
                "Legacy v1 envelopes cannot carry unauthenticated clinical context".to_string(),
            ));
        }
        2 if record.clinical_event_time.is_some()
            && encrypted_v2_route_is_supported(&record.entry_type, &record.data_category)
            && (record.entry_type != "ProcedurePerformed" || record.encounter_hash.is_some()) => {}
        2 => {
            return Ok(ValidateCallbackResult::Invalid(
                "V2 encrypted envelopes require a clinical event time".to_string(),
            ));
        }
        _ => {
            return Ok(ValidateCallbackResult::Invalid(
                "Unsupported encrypted record envelope version".to_string(),
            ));
        }
    }
    if record.ciphertext.len() < 16 {
        return Ok(ValidateCallbackResult::Invalid(
            "Ciphertext must include an AEAD authentication tag".to_string(),
        ));
    }
    if record.key_fingerprint == [0u8; 8] {
        return Ok(ValidateCallbackResult::Invalid(
            "Key fingerprint cannot be all zeroes".to_string(),
        ));
    }
    if record.encrypted_at <= 0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Encrypted timestamp must be positive".to_string(),
        ));
    }
    if !encrypted_route_is_valid(&record.entry_type, &record.data_category) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Unsupported or mismatched encrypted route {} / {}",
            record.entry_type, record.data_category
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn encrypted_v2_route_is_supported(entry_type: &str, data_category: &str) -> bool {
    matches!(
        (entry_type, data_category),
        ("LabResult", "LabResults")
            | ("ProcedurePerformed", "Procedures")
            | ("ImagingStudy", "ImagingStudies")
            | ("VitalSigns", "VitalSigns")
    )
}

fn encrypted_route_is_valid(entry_type: &str, data_category: &str) -> bool {
    matches!(
        (entry_type, data_category),
        ("Encounter", "Procedures")
            | ("Diagnosis", "Diagnoses")
            | ("ProcedurePerformed", "Procedures")
            | ("LabResult", "LabResults")
            | ("ImagingStudy", "ImagingStudies")
            | ("VitalSigns", "VitalSigns")
            | ("SdohScreening", "Demographics")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encrypted_record_fixture(
        envelope_version: u8,
        clinical_event_time: Option<Timestamp>,
        encounter_hash: Option<ActionHash>,
        entry_type: &str,
        data_category: &str,
    ) -> EncryptedRecord {
        EncryptedRecord {
            envelope_version,
            patient_hash: ActionHash::from_raw_36(vec![1u8; 36]),
            key_fingerprint: [1u8; 8],
            ciphertext: vec![2u8; 16],
            nonce: [3u8; 24],
            data_category: data_category.to_string(),
            entry_type: entry_type.to_string(),
            encrypted_at: 1_000_000,
            clinical_event_time,
            encounter_hash,
        }
    }

    #[test]
    fn encrypted_routes_are_exact_and_fail_closed() {
        assert!(encrypted_route_is_valid("LabResult", "LabResults"));
        assert!(encrypted_route_is_valid("SdohScreening", "Demographics"));
        assert!(!encrypted_route_is_valid("LabResult", "MentalHealth"));
        assert!(!encrypted_route_is_valid("Unknown", "All"));
    }

    #[test]
    fn encrypted_record_v1_is_legacy_context_free_only() {
        let legacy = encrypted_record_fixture(1, None, None, "LabResult", "LabResults");
        assert!(matches!(
            validate_encrypted_record(&legacy).expect("v1 validation"),
            ValidateCallbackResult::Valid
        ));

        let injected = encrypted_record_fixture(
            1,
            Some(Timestamp::from_micros(900_000)),
            None,
            "LabResult",
            "LabResults",
        );
        assert!(matches!(
            validate_encrypted_record(&injected).expect("v1 metadata validation"),
            ValidateCallbackResult::Invalid(_)
        ));
    }

    #[test]
    fn encrypted_record_v2_requires_supported_typed_context() {
        let missing_time = encrypted_record_fixture(2, None, None, "LabResult", "LabResults");
        assert!(matches!(
            validate_encrypted_record(&missing_time).expect("missing-time validation"),
            ValidateCallbackResult::Invalid(_)
        ));

        let unsupported_type = encrypted_record_fixture(
            2,
            Some(Timestamp::from_micros(900_000)),
            None,
            "Diagnosis",
            "Diagnoses",
        );
        assert!(matches!(
            validate_encrypted_record(&unsupported_type).expect("unsupported type validation"),
            ValidateCallbackResult::Invalid(_)
        ));

        let procedure_without_encounter = encrypted_record_fixture(
            2,
            Some(Timestamp::from_micros(900_000)),
            None,
            "ProcedurePerformed",
            "Procedures",
        );
        assert!(matches!(
            validate_encrypted_record(&procedure_without_encounter)
                .expect("procedure encounter validation"),
            ValidateCallbackResult::Invalid(_)
        ));

        let valid_v2 = encrypted_record_fixture(
            2,
            Some(Timestamp::from_micros(900_000)),
            Some(ActionHash::from_raw_36(vec![4u8; 36])),
            "ProcedurePerformed",
            "Procedures",
        );
        assert!(matches!(
            validate_encrypted_record(&valid_v2).expect("valid v2 validation"),
            ValidateCallbackResult::Valid
        ));
    }

    #[cfg(not(feature = "dangerous-plaintext-phi"))]
    #[test]
    fn plaintext_phi_is_rejected_by_default() {
        let result = validate_plaintext_phi("LabResult", || Ok(ValidateCallbackResult::Valid))
            .expect("validation result");
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }
}
