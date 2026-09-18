#![deny(unsafe_code)]
//! Thin coordinator for DNA-rooted clinical distribution evaluator admission.
//!
//! Deliberately contains no evaluator-trust decision logic. The coordinator only
//! commits/reads entries and links; integrity validation independently enforces
//! DNA-rooted exact-target authorization and append-only revocation semantics.

use clinical_distribution_trust_integrity::*;
use hdk::prelude::*;

#[hdk_extern]
pub fn issue_distribution_evaluator_authorization(
    authorization: DistributionEvaluatorVerifierAuthorization,
) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::DistributionEvaluatorVerifierAuthorization(
        authorization,
    ))?;
    get_required_record(hash)
}

#[hdk_extern]
pub fn admit_distribution_evaluator(
    admission: QualifiedDistributionEvaluatorAdmission,
) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::QualifiedDistributionEvaluatorAdmission(admission))?;
    get_required_record(hash)
}

#[hdk_extern]
pub fn revoke_distribution_evaluator(
    revocation: DistributionEvaluatorAdmissionRevocation,
) -> ExternResult<Record> {
    let admission_hash = revocation.admission_hash.clone();
    let hash = create_entry(&EntryTypes::DistributionEvaluatorAdmissionRevocation(
        revocation,
    ))?;
    create_link(
        admission_hash,
        hash.clone(),
        LinkTypes::AdmissionToRevocations,
        (),
    )?;
    get_required_record(hash)
}

#[hdk_extern]
pub fn get_distribution_evaluator_admission(
    hash: ActionHash,
) -> ExternResult<Option<Record>> {
    get(hash, GetOptions::default())
}

#[hdk_extern]
pub fn get_distribution_evaluator_revocations(
    admission_hash: ActionHash,
) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(admission_hash, LinkTypes::AdmissionToRevocations)?,
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
        "Committed clinical distribution trust record could not be read back".to_string()
    )))
}
