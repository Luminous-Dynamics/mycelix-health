#![deny(unsafe_code)]
//! Thin coordinator for DNA-rooted administration occurrence bindings.
//!
//! Trust and exact-target enforcement live in the integrity zome. This coordinator
//! only commits exact entries/links so a modified coordinator cannot bypass admission.

use hdk::prelude::*;
use medication_administration_occurrence_integrity::{
    EntryTypes, LinkTypes, MedicationAdministrationOccurrenceBindingCorrection,
    MedicationAdministrationOccurrenceVerifierAuthorization,
    QualifiedMedicationAdministrationOccurrenceBinding,
};

#[hdk_extern]
pub fn create_occurrence_verifier_authorization(
    authorization: MedicationAdministrationOccurrenceVerifierAuthorization,
) -> ExternResult<Record> {
    let hash = create_entry(
        EntryTypes::MedicationAdministrationOccurrenceVerifierAuthorization(authorization),
    )?;
    get(hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Committed administration occurrence verifier authorization could not be read back"
            .to_string()
    )))
}

#[hdk_extern]
pub fn publish_qualified_occurrence_binding(
    binding: QualifiedMedicationAdministrationOccurrenceBinding,
) -> ExternResult<Record> {
    let hash = create_entry(EntryTypes::QualifiedMedicationAdministrationOccurrenceBinding(
        binding,
    ))?;
    get(hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Committed qualified administration occurrence binding could not be read back".to_string()
    )))
}

#[hdk_extern]
pub fn append_occurrence_binding_correction(
    correction: MedicationAdministrationOccurrenceBindingCorrection,
) -> ExternResult<Record> {
    let binding_hash = correction.binding_hash.clone();
    let correction_hash = create_entry(
        EntryTypes::MedicationAdministrationOccurrenceBindingCorrection(correction),
    )?;
    create_link(
        binding_hash,
        correction_hash.clone(),
        LinkTypes::BindingToCorrections,
        (),
    )?;
    get(correction_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Committed administration occurrence binding correction could not be read back"
            .to_string()
    )))
}
