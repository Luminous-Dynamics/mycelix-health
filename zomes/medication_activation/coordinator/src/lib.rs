#![deny(unsafe_code)]
//! Thin coordinator for qualified medication activation attestations.
//!
//! Deliberately contains no clinical authorization logic. The coordinator only
//! commits/query entries; the integrity zome independently enforces DNA-rooted
//! verifier authority, exact policy binding, append-only evidence, and revocation
//! lineage. A modified coordinator therefore cannot bypass those rules.

use hdk::prelude::*;
use medication_activation_integrity::*;

#[hdk_extern]
pub fn issue_verifier_grant(
    grant: MedicationActivationVerifierGrant,
) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::MedicationActivationVerifierGrant(grant))?;
    get_required_record(hash)
}

#[hdk_extern]
pub fn record_qualified_activation(
    activation: QualifiedMedicationActivation,
) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::QualifiedMedicationActivation(activation))?;
    get_required_record(hash)
}

#[hdk_extern]
pub fn revoke_qualified_activation(
    revocation: MedicationActivationRevocation,
) -> ExternResult<Record> {
    let activation_hash = revocation.activation_hash.clone();
    let hash = create_entry(&EntryTypes::MedicationActivationRevocation(revocation))?;
    create_link(
        activation_hash,
        hash.clone(),
        LinkTypes::ActivationToRevocations,
        (),
    )?;
    get_required_record(hash)
}

#[hdk_extern]
pub fn get_qualified_activation(hash: ActionHash) -> ExternResult<Option<Record>> {
    get(hash, GetOptions::default())
}

#[hdk_extern]
pub fn get_activation_revocations(
    activation_hash: ActionHash,
) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(activation_hash, LinkTypes::ActivationToRevocations)?,
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
        "Committed medication activation record could not be read back".to_string()
    )))
}
