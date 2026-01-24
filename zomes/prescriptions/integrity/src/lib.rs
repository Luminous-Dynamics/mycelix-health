//! Prescription Management Integrity Zome
//! 
//! Defines entry types for prescriptions, medication orders,
//! refills, and pharmacy interactions with RxNorm alignment.

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
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::Prescription(rx) => validate_prescription(&rx),
                EntryTypes::PrescriptionFill(fill) => validate_fill(&fill),
                EntryTypes::MedicationAdherence(_) => Ok(ValidateCallbackResult::Valid),
                EntryTypes::DrugInteractionAlert(_) => Ok(ValidateCallbackResult::Valid),
                EntryTypes::Pharmacy(p) => validate_pharmacy(&p),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_prescription(rx: &Prescription) -> ExternResult<ValidateCallbackResult> {
    if rx.prescription_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Prescription ID is required".to_string(),
        ));
    }
    if rx.rxnorm_code.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RxNorm code is required".to_string(),
        ));
    }
    if rx.medication_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Medication name is required".to_string(),
        ));
    }
    if rx.dosage_instructions.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Dosage instructions are required".to_string(),
        ));
    }
    // Controlled substances must have DEA number
    if rx.schedule.is_some() && rx.schedule != Some(DrugSchedule::NotControlled) {
        if rx.dea_number.is_none() {
            return Ok(ValidateCallbackResult::Invalid(
                "DEA number required for controlled substances".to_string(),
            ));
        }
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_fill(fill: &PrescriptionFill) -> ExternResult<ValidateCallbackResult> {
    if fill.fill_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Fill ID is required".to_string(),
        ));
    }
    if fill.ndc_dispensed.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "NDC of dispensed medication is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_pharmacy(pharmacy: &Pharmacy) -> ExternResult<ValidateCallbackResult> {
    if pharmacy.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Pharmacy name is required".to_string(),
        ));
    }
    if pharmacy.matl_trust_score < 0.0 || pharmacy.matl_trust_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "MATL trust score must be between 0.0 and 1.0".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
