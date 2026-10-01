#![deny(unsafe_code)]
//! Thin coordinator for qualified medication administration attestations.
//!
//! Clinical qualification belongs to the pure administration stack and DHT trust
//! enforcement belongs to the integrity zome. This coordinator only commits exact
//! entries/links; a modified coordinator must not be able to bypass validation.

use hdk::prelude::*;
use medication_administration_integrity::{
    EntryTypes, LinkTypes, MedicationAdministrationCorrection,
    MedicationAdministrationVerifierAuthorization, QualifiedMedicationAdministration,
};

#[hdk_extern]
pub fn create_verifier_authorization(
    authorization: MedicationAdministrationVerifierAuthorization,
) -> ExternResult<Record> {
    let hash = create_entry(EntryTypes::MedicationAdministrationVerifierAuthorization(
        authorization,
    ))?;
    get(hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Committed administration verifier authorization could not be read back".to_string()
    )))
}

#[hdk_extern]
pub fn publish_qualified_administration(
    administration: QualifiedMedicationAdministration,
) -> ExternResult<Record> {
    let hash = create_entry(EntryTypes::QualifiedMedicationAdministration(administration))?;
    get(hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Committed qualified administration could not be read back".to_string()
    )))
}

#[hdk_extern]
pub fn append_administration_correction(
    correction: MedicationAdministrationCorrection,
) -> ExternResult<Record> {
    let administration_hash = correction.administration_hash.clone();
    let correction_hash = create_entry(EntryTypes::MedicationAdministrationCorrection(
        correction,
    ))?;
    create_link(
        administration_hash,
        correction_hash.clone(),
        LinkTypes::AdministrationToCorrections,
        (),
    )?;
    get(correction_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Committed administration correction could not be read back".to_string()
    )))
}
