#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
#![allow(clippy::collapsible_match)]

//! Prescription Management Integrity Zome
//!
//! Legacy `Prescription.status` is no longer accepted as sufficient evidence of a
//! qualified medication activation. Existing historical entries remain readable,
//! but new legacy prescriptions cannot be created or promoted to `Active` without
//! the separate evidence-bearing activation lineage tracked by the high-assurance
//! medication stack.
//!
//! This integrity zome also moves entry-author continuity and cross-entry link
//! consistency out of coordinator-only code so a modified coordinator cannot bypass
//! the basic DHT authorization boundary.

use hdi::prelude::*;

/// Prescription entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Prescription {
    pub prescription_id: String,
    pub patient_hash: ActionHash,
    pub prescriber_hash: ActionHash,
    pub encounter_hash: Option<ActionHash>,
    /// RxNorm code for the medication
    pub rxnorm_code: String,
    /// NDC code if specific product
    pub ndc_code: Option<String>,
    pub medication_name: String,
    pub strength: String,
    pub form: MedicationForm,
    pub route: AdministrationRoute,
    pub dosage_instructions: String,
    pub quantity: u32,
    pub quantity_unit: String,
    pub refills_authorized: u32,
    pub refills_remaining: u32,
    pub days_supply: u32,
    pub dispense_as_written: bool,
    pub status: PrescriptionStatus,
    pub written_date: Timestamp,
    pub effective_date: Timestamp,
    pub expiration_date: Timestamp,
    /// For controlled substances
    pub schedule: Option<DrugSchedule>,
    pub dea_number: Option<String>,
    /// Pharmacy to fill
    pub pharmacy_hash: Option<ActionHash>,
    pub notes: Option<String>,
    /// Diagnosis justifying prescription
    pub indication: String,
    pub indication_icd10: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MedicationForm {
    Tablet,
    Capsule,
    Liquid,
    Injection,
    Topical,
    Patch,
    Inhaler,
    Drops,
    Suppository,
    Powder,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AdministrationRoute {
    Oral,
    Intravenous,
    Intramuscular,
    Subcutaneous,
    Topical,
    Transdermal,
    Inhalation,
    Ophthalmic,
    Otic,
    Nasal,
    Rectal,
    Sublingual,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PrescriptionStatus {
    Active,
    Completed,
    Discontinued,
    OnHold,
    Cancelled,
    Expired,
    EnteredInError,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DrugSchedule {
    /// Schedule I - No medical use, high abuse potential
    ScheduleI,
    /// Schedule II - High abuse potential (opioids, stimulants)
    ScheduleII,
    /// Schedule III - Moderate abuse potential
    ScheduleIII,
    /// Schedule IV - Low abuse potential (benzodiazepines)
    ScheduleIV,
    /// Schedule V - Lowest abuse potential
    ScheduleV,
    /// Not a controlled substance
    NotControlled,
}

/// Prescription fill/dispense record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PrescriptionFill {
    pub fill_id: String,
    pub prescription_hash: ActionHash,
    pub pharmacy_hash: ActionHash,
    pub pharmacist: AgentPubKey,
    pub fill_date: Timestamp,
    pub quantity_dispensed: u32,
    pub days_supply_dispensed: u32,
    pub ndc_dispensed: String,
    pub lot_number: Option<String>,
    pub expiration_date: Option<String>,
    pub patient_counseled: bool,
    pub drug_interactions_reviewed: bool,
    pub status: FillStatus,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum FillStatus {
    Pending,
    ReadyForPickup,
    Dispensed,
    PartialFill,
    Cancelled,
    Returned,
}

/// Medication adherence record (patient-reported or device-tracked)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MedicationAdherence {
    pub patient_hash: ActionHash,
    pub prescription_hash: ActionHash,
    pub recorded_at: Timestamp,
    pub dose_taken: bool,
    pub dose_time: Option<Timestamp>,
    pub source: AdherenceSource,
    pub notes: Option<String>,
    pub side_effects_reported: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AdherenceSource {
    PatientReported,
    SmartPillBottle,
    SmartWatch,
    CaregiverReported,
    PharmacyRecord,
}

/// Drug-drug interaction alert
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DrugInteractionAlert {
    pub alert_id: String,
    pub patient_hash: ActionHash,
    pub prescription_hash: ActionHash,
    pub interacting_medication: String,
    pub interaction_type: InteractionSeverity,
    pub description: String,
    pub clinical_significance: String,
    pub management_recommendation: String,
    pub source: String,
    pub acknowledged: bool,
    pub acknowledged_by: Option<AgentPubKey>,
    pub acknowledged_at: Option<Timestamp>,
    pub override_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum InteractionSeverity {
    Contraindicated,
    Major,
    Moderate,
    Minor,
    Unknown,
}

/// Pharmacy profile
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Pharmacy {
    pub pharmacy_id: String,
    pub name: String,
    pub pharmacy_type: PharmacyType,
    pub npi: Option<String>,
    pub dea_number: Option<String>,
    pub address_line1: String,
    pub address_line2: Option<String>,
    pub city: String,
    pub state_province: String,
    pub postal_code: String,
    pub country: String,
    pub phone: String,
    pub fax: Option<String>,
    pub email: Option<String>,
    pub hours: Option<String>,
    pub accepts_electronic_rx: bool,
    pub accepts_controlled_rx: bool,
    pub delivery_available: bool,
    pub matl_trust_score: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PharmacyType {
    Retail,
    Hospital,
    MailOrder,
    Compounding,
    Specialty,
    LongTermCare,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Prescription(Prescription),
    PrescriptionFill(PrescriptionFill),
    MedicationAdherence(MedicationAdherence),
    DrugInteractionAlert(DrugInteractionAlert),
    Pharmacy(Pharmacy),
}

#[hdk_link_types]
pub enum LinkTypes {
    PatientToPrescriptions,
    PrescriberToPrescriptions,
    PrescriptionToFills,
    PatientToAdherence,
    PrescriptionToAlerts,
    PatientToPharmacy,
    AllPharmacies,
    ControlledSubstances,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => {
                validate_create_entry(EntryCreationAction::Create(action), app_entry)
            }
            OpEntry::UpdateEntry {
                app_entry, action, ..
            } => validate_update_entry(action, app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            ..
        } => validate_create_link(link_type, base_address, target_address),
        FlatOp::RegisterDeleteLink {
            original_action,
            action,
            ..
        } => {
            if action.author != original_action.author {
                return invalid("Only the original link creator can delete a prescription link");
            }
            Ok(ValidateCallbackResult::Valid)
        }
        FlatOp::RegisterUpdate(update) => {
            let action = match &update {
                OpUpdate::Entry { action, .. }
                | OpUpdate::PrivateEntry { action, .. }
                | OpUpdate::Agent { action, .. }
                | OpUpdate::CapClaim { action, .. }
                | OpUpdate::CapGrant { action, .. } => action,
            };
            let original = must_get_action(action.original_action_address.clone())?;
            if *original.action().author() != action.author {
                return invalid("Only the original entry author can update a legacy prescription entry");
            }
            match update {
                OpUpdate::Entry { app_entry, action } => validate_update_entry(action, app_entry),
                _ => Ok(ValidateCallbackResult::Valid),
            }
        }
        FlatOp::RegisterDelete(OpDelete { action }) => {
            let original = must_get_action(action.deletes_address.clone())?;
            if *original.action().author() != action.author {
                return invalid("Only the original entry author can delete a legacy prescription entry");
            }
            Ok(ValidateCallbackResult::Valid)
        }
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(
    action: EntryCreationAction,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::Prescription(rx) => validate_new_legacy_prescription(&rx),
        EntryTypes::PrescriptionFill(fill) => {
            let shape = validate_fill(&fill)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            if action.author() != &fill.pharmacist {
                return invalid("Prescription fill pharmacist must be the entry author");
            }
            require_fillable_prescription(&fill.prescription_hash)
        }
        EntryTypes::MedicationAdherence(adherence) => validate_adherence(&adherence),
        EntryTypes::DrugInteractionAlert(alert) => validate_alert(&alert),
        EntryTypes::Pharmacy(pharmacy) => validate_pharmacy(&pharmacy),
    }
}

fn validate_update_entry(
    action: Update,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    let original_action = must_get_action(action.original_action_address.clone())?;
    if *original_action.action().author() != action.author {
        return invalid("Only the original entry author can update a legacy prescription entry");
    }

    let original_record = must_get_valid_record(action.original_action_address.clone())?;
    match entry {
        EntryTypes::Prescription(rx) => {
            let original: Prescription = decode_entry(&original_record, "prescription")?;
            validate_prescription_update(&original, &rx)
        }
        EntryTypes::PrescriptionFill(fill) => {
            let original: PrescriptionFill = decode_entry(&original_record, "prescription fill")?;
            if original.fill_id != fill.fill_id
                || original.prescription_hash != fill.prescription_hash
                || original.pharmacy_hash != fill.pharmacy_hash
                || original.pharmacist != fill.pharmacist
                || original.fill_date != fill.fill_date
                || original.quantity_dispensed != fill.quantity_dispensed
                || original.days_supply_dispensed != fill.days_supply_dispensed
                || original.ndc_dispensed != fill.ndc_dispensed
            {
                return invalid("Prescription fill identity/dispense facts are immutable");
            }
            validate_fill(&fill)
        }
        EntryTypes::MedicationAdherence(_) => invalid(
            "Medication adherence records are immutable observations; create a correcting event",
        ),
        EntryTypes::DrugInteractionAlert(alert) => {
            let original: DrugInteractionAlert = decode_entry(&original_record, "interaction alert")?;
            if original.alert_id != alert.alert_id
                || original.patient_hash != alert.patient_hash
                || original.prescription_hash != alert.prescription_hash
                || original.interacting_medication != alert.interacting_medication
                || original.interaction_type != alert.interaction_type
                || original.description != alert.description
                || original.clinical_significance != alert.clinical_significance
                || original.management_recommendation != alert.management_recommendation
                || original.source != alert.source
            {
                return invalid("Interaction alert clinical facts are immutable; only acknowledgement may change");
            }
            validate_alert(&alert)
        }
        EntryTypes::Pharmacy(pharmacy) => validate_pharmacy(&pharmacy),
    }
}

fn validate_new_legacy_prescription(rx: &Prescription) -> ExternResult<ValidateCallbackResult> {
    let shape = validate_prescription_shape(rx)?;
    if !matches!(shape, ValidateCallbackResult::Valid) {
        return Ok(shape);
    }
    if matches!(rx.status, PrescriptionStatus::Active) {
        return invalid(
            "Legacy Prescription entries cannot be created Active; use the evidence-bearing qualified activation path",
        );
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_prescription_update(
    original: &Prescription,
    updated: &Prescription,
) -> ExternResult<ValidateCallbackResult> {
    let shape = validate_prescription_shape(updated)?;
    if !matches!(shape, ValidateCallbackResult::Valid) {
        return Ok(shape);
    }

    if original.prescription_id != updated.prescription_id
        || original.patient_hash != updated.patient_hash
        || original.prescriber_hash != updated.prescriber_hash
        || original.encounter_hash != updated.encounter_hash
        || original.rxnorm_code != updated.rxnorm_code
        || original.ndc_code != updated.ndc_code
        || original.medication_name != updated.medication_name
        || original.strength != updated.strength
        || original.form != updated.form
        || original.route != updated.route
        || original.dosage_instructions != updated.dosage_instructions
        || original.quantity != updated.quantity
        || original.quantity_unit != updated.quantity_unit
        || original.refills_authorized != updated.refills_authorized
        || original.days_supply != updated.days_supply
        || original.dispense_as_written != updated.dispense_as_written
        || original.written_date != updated.written_date
        || original.effective_date != updated.effective_date
        || original.expiration_date != updated.expiration_date
        || original.schedule != updated.schedule
        || original.dea_number != updated.dea_number
        || original.indication != updated.indication
        || original.indication_icd10 != updated.indication_icd10
    {
        return invalid(
            "Legacy prescription clinical/identity facts are immutable; create a new versioned order",
        );
    }

    if updated.refills_remaining > original.refills_remaining {
        return invalid("Legacy prescription refills_remaining cannot increase through update");
    }
    if !valid_legacy_status_transition(&original.status, &updated.status) {
        return invalid(
            "Legacy prescription status transition is not allowed; promotion to Active requires qualified activation",
        );
    }
    Ok(ValidateCallbackResult::Valid)
}

fn valid_legacy_status_transition(from: &PrescriptionStatus, to: &PrescriptionStatus) -> bool {
    if from == to {
        return true;
    }
    match from {
        PrescriptionStatus::Active => matches!(
            to,
            PrescriptionStatus::Completed
                | PrescriptionStatus::Discontinued
                | PrescriptionStatus::OnHold
                | PrescriptionStatus::Cancelled
                | PrescriptionStatus::Expired
                | PrescriptionStatus::EnteredInError
        ),
        PrescriptionStatus::OnHold => matches!(
            to,
            PrescriptionStatus::Discontinued
                | PrescriptionStatus::Cancelled
                | PrescriptionStatus::Expired
                | PrescriptionStatus::EnteredInError
        ),
        PrescriptionStatus::Completed
        | PrescriptionStatus::Discontinued
        | PrescriptionStatus::Cancelled
        | PrescriptionStatus::Expired => matches!(to, PrescriptionStatus::EnteredInError),
        PrescriptionStatus::EnteredInError => false,
    }
}

fn validate_prescription_shape(rx: &Prescription) -> ExternResult<ValidateCallbackResult> {
    if rx.prescription_id.trim().is_empty() {
        return invalid("Prescription ID is required");
    }
    if rx.rxnorm_code.trim().is_empty() {
        return invalid("RxNorm code is required");
    }
    if rx.medication_name.trim().is_empty() {
        return invalid("Medication name is required");
    }
    if rx.strength.trim().is_empty() {
        return invalid("Medication strength is required for legacy prescription storage");
    }
    if rx.dosage_instructions.trim().is_empty() {
        return invalid("Dosage instructions are required");
    }
    if rx.quantity == 0 {
        return invalid("Prescription quantity must be greater than zero");
    }
    if rx.quantity_unit.trim().is_empty() {
        return invalid("Prescription quantity unit is required");
    }
    if rx.days_supply == 0 {
        return invalid("Prescription days supply must be greater than zero");
    }
    if rx.refills_remaining > rx.refills_authorized {
        return invalid("Prescription refills_remaining cannot exceed refills_authorized");
    }
    if rx.indication.trim().is_empty() {
        return invalid("Prescription indication is required");
    }
    if rx.effective_date < rx.written_date {
        return invalid("Prescription effective date cannot predate written date");
    }
    if rx.expiration_date <= rx.effective_date {
        return invalid("Prescription expiration date must follow effective date");
    }
    if rx.schedule.is_some()
        && rx.schedule != Some(DrugSchedule::NotControlled)
        && rx
            .dea_number
            .as_ref()
            .is_none_or(|number| number.trim().is_empty())
    {
        return invalid("DEA number required for controlled-substance legacy records");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_fill(fill: &PrescriptionFill) -> ExternResult<ValidateCallbackResult> {
    if fill.fill_id.trim().is_empty() {
        return invalid("Fill ID is required");
    }
    if fill.quantity_dispensed == 0 {
        return invalid("Dispensed quantity must be greater than zero");
    }
    if fill.days_supply_dispensed == 0 {
        return invalid("Dispensed days supply must be greater than zero");
    }
    if fill.ndc_dispensed.trim().is_empty() {
        return invalid("NDC of dispensed medication is required");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_adherence(adherence: &MedicationAdherence) -> ExternResult<ValidateCallbackResult> {
    if adherence.dose_taken && adherence.dose_time.is_none() {
        return invalid("Taken-dose adherence event requires a dose time");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_alert(alert: &DrugInteractionAlert) -> ExternResult<ValidateCallbackResult> {
    if alert.alert_id.trim().is_empty() {
        return invalid("Interaction alert ID is required");
    }
    if alert.source.trim().is_empty() {
        return invalid("Interaction alert source is required");
    }
    if alert.acknowledged {
        if alert.acknowledged_by.is_none() || alert.acknowledged_at.is_none() {
            return invalid("Acknowledged interaction alert requires actor and timestamp");
        }
        if matches!(alert.interaction_type, InteractionSeverity::Contraindicated | InteractionSeverity::Major)
            && alert
                .override_reason
                .as_ref()
                .is_none_or(|reason| reason.trim().is_empty())
        {
            return invalid("Acknowledging a major/contraindicated alert requires an override reason");
        }
    } else if alert.acknowledged_by.is_some() || alert.acknowledged_at.is_some() {
        return invalid("Unacknowledged interaction alert cannot carry acknowledgement actor/time");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_pharmacy(pharmacy: &Pharmacy) -> ExternResult<ValidateCallbackResult> {
    if pharmacy.pharmacy_id.trim().is_empty() || pharmacy.name.trim().is_empty() {
        return invalid("Pharmacy ID and name are required");
    }
    if !pharmacy.matl_trust_score.is_finite()
        || pharmacy.matl_trust_score < 0.0
        || pharmacy.matl_trust_score > 1.0
    {
        return invalid("MATL trust score must be finite and between 0.0 and 1.0");
    }
    if let Some(npi) = &pharmacy.npi {
        if npi.len() != 10 || !npi.chars().all(|c| c.is_ascii_digit()) {
            return invalid("Pharmacy NPI must be a 10-digit number");
        }
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_fillable_prescription(
    prescription_hash: &ActionHash,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(prescription_hash.clone())?;
    let prescription: Prescription = decode_entry(&record, "prescription")?;
    if !matches!(prescription.status, PrescriptionStatus::Active) {
        return invalid("Prescription fill requires an active prescription record");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_create_link(
    link_type: LinkTypes,
    base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        LinkTypes::PatientToPrescriptions => {
            let patient_hash = require_action_hash(base_address, "patient-prescription link base")?;
            let prescription_hash = require_action_hash(target_address, "patient-prescription link target")?;
            let record = must_get_valid_record(prescription_hash)?;
            let prescription: Prescription = decode_entry(&record, "prescription")?;
            if prescription.patient_hash != patient_hash {
                return invalid("Patient-to-prescription link base does not match prescription patient");
            }
            Ok(ValidateCallbackResult::Valid)
        }
        LinkTypes::PrescriberToPrescriptions => {
            let prescriber_hash = require_action_hash(base_address, "prescriber-prescription link base")?;
            let prescription_hash = require_action_hash(target_address, "prescriber-prescription link target")?;
            let record = must_get_valid_record(prescription_hash)?;
            let prescription: Prescription = decode_entry(&record, "prescription")?;
            if prescription.prescriber_hash != prescriber_hash {
                return invalid("Prescriber-to-prescription link base does not match prescription prescriber");
            }
            Ok(ValidateCallbackResult::Valid)
        }
        LinkTypes::PrescriptionToFills => {
            let prescription_hash = require_action_hash(base_address, "prescription-fill link base")?;
            let fill_hash = require_action_hash(target_address, "prescription-fill link target")?;
            let record = must_get_valid_record(fill_hash)?;
            let fill: PrescriptionFill = decode_entry(&record, "prescription fill")?;
            if fill.prescription_hash != prescription_hash {
                return invalid("Prescription-to-fill link target references another prescription");
            }
            Ok(ValidateCallbackResult::Valid)
        }
        LinkTypes::PatientToAdherence => {
            let patient_hash = require_action_hash(base_address, "patient-adherence link base")?;
            let adherence_hash = require_action_hash(target_address, "patient-adherence link target")?;
            let record = must_get_valid_record(adherence_hash)?;
            let adherence: MedicationAdherence = decode_entry(&record, "medication adherence")?;
            if adherence.patient_hash != patient_hash {
                return invalid("Patient-to-adherence link base does not match adherence patient");
            }
            Ok(ValidateCallbackResult::Valid)
        }
        LinkTypes::PrescriptionToAlerts => {
            let prescription_hash = require_action_hash(base_address, "prescription-alert link base")?;
            let alert_hash = require_action_hash(target_address, "prescription-alert link target")?;
            let record = must_get_valid_record(alert_hash)?;
            let alert: DrugInteractionAlert = decode_entry(&record, "interaction alert")?;
            if alert.prescription_hash != prescription_hash {
                return invalid("Prescription-to-alert link target references another prescription");
            }
            Ok(ValidateCallbackResult::Valid)
        }
        LinkTypes::PatientToPharmacy | LinkTypes::AllPharmacies => {
            let pharmacy_hash = require_action_hash(target_address, "pharmacy link target")?;
            let record = must_get_valid_record(pharmacy_hash)?;
            let _: Pharmacy = decode_entry(&record, "pharmacy")?;
            Ok(ValidateCallbackResult::Valid)
        }
        LinkTypes::ControlledSubstances => {
            let prescription_hash = require_action_hash(target_address, "controlled-substance link target")?;
            let record = must_get_valid_record(prescription_hash)?;
            let prescription: Prescription = decode_entry(&record, "prescription")?;
            if prescription.schedule.is_none()
                || prescription.schedule == Some(DrugSchedule::NotControlled)
            {
                return invalid("Controlled-substance index target is not a controlled-substance prescription");
            }
            Ok(ValidateCallbackResult::Valid)
        }
    }
}

fn require_action_hash(
    hash: AnyLinkableHash,
    field: &'static str,
) -> ExternResult<ActionHash> {
    hash.into_action_hash().ok_or(wasm_error!(WasmErrorInner::Guest(
        format!("{field} must be an ActionHash")
    )))
}

fn decode_entry<T>(record: &Record, label: &'static str) -> ExternResult<T>
where
    T: TryFrom<SerializedBytes, Error = SerializedBytesError>,
{
    record
        .entry()
        .to_app_option::<T>()
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(format!(
            "Referenced {label} entry is missing or wrong type"
        ))))
}

fn invalid(message: impl Into<String>) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action_hash(byte: u8) -> ActionHash {
        ActionHash::from_raw_36(vec![byte; 36])
    }

    fn prescription(status: PrescriptionStatus) -> Prescription {
        Prescription {
            prescription_id: "rx-1".into(),
            patient_hash: action_hash(1),
            prescriber_hash: action_hash(2),
            encounter_hash: None,
            rxnorm_code: "860975".into(),
            ndc_code: None,
            medication_name: "example".into(),
            strength: "10 mg".into(),
            form: MedicationForm::Tablet,
            route: AdministrationRoute::Oral,
            dosage_instructions: "one tablet daily".into(),
            quantity: 30,
            quantity_unit: "tablet".into(),
            refills_authorized: 2,
            refills_remaining: 2,
            days_supply: 30,
            dispense_as_written: false,
            status,
            written_date: Timestamp::from_micros(10),
            effective_date: Timestamp::from_micros(20),
            expiration_date: Timestamp::from_micros(1_000),
            schedule: Some(DrugSchedule::NotControlled),
            dea_number: None,
            pharmacy_hash: None,
            notes: None,
            indication: "test".into(),
            indication_icd10: None,
        }
    }

    #[test]
    fn legacy_active_prescription_create_is_rejected() {
        let result = validate_new_legacy_prescription(&prescription(PrescriptionStatus::Active))
            .unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn non_active_legacy_prescription_can_remain_as_storage_record() {
        assert_eq!(
            validate_new_legacy_prescription(&prescription(PrescriptionStatus::OnHold)).unwrap(),
            ValidateCallbackResult::Valid
        );
    }

    #[test]
    fn legacy_status_cannot_promote_to_active() {
        assert!(!valid_legacy_status_transition(
            &PrescriptionStatus::OnHold,
            &PrescriptionStatus::Active
        ));
    }

    #[test]
    fn active_legacy_status_can_only_move_toward_less_authority() {
        assert!(valid_legacy_status_transition(
            &PrescriptionStatus::Active,
            &PrescriptionStatus::Discontinued
        ));
        assert!(valid_legacy_status_transition(
            &PrescriptionStatus::Active,
            &PrescriptionStatus::OnHold
        ));
    }

    #[test]
    fn refills_cannot_exceed_authorized_amount() {
        let mut rx = prescription(PrescriptionStatus::OnHold);
        rx.refills_remaining = 3;
        let result = validate_prescription_shape(&rx).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn pharmacy_trust_score_must_be_finite() {
        let pharmacy = Pharmacy {
            pharmacy_id: "p-1".into(),
            name: "Example".into(),
            pharmacy_type: PharmacyType::Retail,
            npi: None,
            dea_number: None,
            address_line1: "1 Main".into(),
            address_line2: None,
            city: "Dallas".into(),
            state_province: "TX".into(),
            postal_code: "00000".into(),
            country: "US".into(),
            phone: "000".into(),
            fax: None,
            email: None,
            hours: None,
            accepts_electronic_rx: true,
            accepts_controlled_rx: false,
            delivery_available: false,
            matl_trust_score: f64::NAN,
        };
        let result = validate_pharmacy(&pharmacy).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }
}
