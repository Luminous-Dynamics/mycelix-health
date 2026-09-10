#![deny(unsafe_code)]
//! Thin coordinator for emergency medication override activation attestations.
//!
//! Clinical/emergency authority is intentionally not implemented here. This zome
//! only commits and queries entries; the integrity zome independently enforces
//! DNA-rooted exact-target verifier authorization and append-only lineage.

use hdk::prelude::*;
use medication_emergency_integrity::*;

#[hdk_extern]
pub fn issue_emergency_verifier_authorization(
    authorization: EmergencyMedicationVerifierAuthorization,
) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::EmergencyMedicationVerifierAuthorization(
        authorization,
    ))?;
    get_required_record(hash)
}

#[hdk_extern]
pub fn record_emergency_medication_activation(
    activation: EmergencyMedicationActivation,
) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::EmergencyMedicationActivation(activation))?;
    get_required_record(hash)
}

#[hdk_extern]
pub fn terminate_emergency_medication_activation(
    termination: EmergencyMedicationTermination,
) -> ExternResult<Record> {
    let activation_hash = termination.activation_hash.clone();
    let hash = create_entry(&EntryTypes::EmergencyMedicationTermination(termination))?;
    create_link(
        activation_hash,
        hash.clone(),
        LinkTypes::ActivationToTerminations,
        (),
    )?;
    get_required_record(hash)
}

#[hdk_extern]
pub fn get_emergency_medication_activation(
    hash: ActionHash,
) -> ExternResult<Option<Record>> {
    get(hash, GetOptions::default())
}

#[hdk_extern]
pub fn get_emergency_activation_terminations(
    activation_hash: ActionHash,
) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(activation_hash, LinkTypes::ActivationToTerminations)?,
        GetStrategy::default(),
    )?;
    let mut records = Vec::with_capacity(links.len());
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }
    Ok(records)
}

fn get_required_record(hash: ActionHash) -> ExternResult<Record> {
    get(hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Committed emergency medication record could not be read back".to_string()
    )))
}
