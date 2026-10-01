#![deny(unsafe_code)]
//! Thin coordinator for trusted privacy-accountant state.
//!
//! Budget/accountant reasoning happens in the protected pure layer. Trust enforcement
//! happens in the integrity zome. This coordinator only commits exact entries/links
//! and materializes bounded network-backed correction snapshots.

use hdk::prelude::*;
use population_accountant_integrity::{
    EntryTypes, LinkTypes, PopulationAccountantStateCorrection,
    PopulationAccountantVerifierAuthorization, TrustedPopulationAccountantState,
};

const MAX_CORRECTIONS_PER_STATE: usize = 256;

#[derive(Clone, Serialize, Deserialize)]
pub struct ObservedPopulationAccountantCorrectionV1 {
    pub action_hash: ActionHash,
    pub correction: PopulationAccountantStateCorrection,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PopulationAccountantReadBoundaryV1 {
    NetworkBackedStableDoubleRead,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PopulationAccountantStateSnapshotV1 {
    pub state_action_hash: ActionHash,
    pub state: TrustedPopulationAccountantState,
    pub corrections: Vec<ObservedPopulationAccountantCorrectionV1>,
    pub read_boundary: PopulationAccountantReadBoundaryV1,
    pub observed_correction_target_count: usize,
    pub observation_started_at: Timestamp,
    pub observation_completed_at: Timestamp,
}

#[hdk_extern]
pub fn create_population_accountant_verifier_authorization(
    authorization: PopulationAccountantVerifierAuthorization,
) -> ExternResult<Record> {
    let hash = create_entry(EntryTypes::PopulationAccountantVerifierAuthorization(authorization))?;
    get(hash, GetOptions::network())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Committed population accountant verifier authorization could not be read back".into()
    )))
}

#[hdk_extern]
pub fn publish_trusted_population_accountant_state(
    state: TrustedPopulationAccountantState,
) -> ExternResult<Record> {
    let hash = create_entry(EntryTypes::TrustedPopulationAccountantState(state))?;
    get(hash, GetOptions::network())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Committed trusted population accountant state could not be read back".into()
    )))
}

#[hdk_extern]
pub fn append_population_accountant_state_correction(
    correction: PopulationAccountantStateCorrection,
) -> ExternResult<Record> {
    let state_hash = correction.state_hash.clone();
    let correction_hash = create_entry(EntryTypes::PopulationAccountantStateCorrection(correction))?;
    create_link(
        state_hash,
        correction_hash.clone(),
        LinkTypes::StateToCorrections,
        (),
    )?;
    get(correction_hash, GetOptions::network())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Committed population accountant correction could not be read back".into()
    )))
}

/// Materialize one known trusted accountant state plus every correction target observed
/// by this conductor, requiring the target set to be unchanged across two network reads.
/// This is bounded observed state, not proof of global DHT completeness.
#[hdk_extern]
pub fn materialize_population_accountant_state_snapshot(
    state_hash: ActionHash,
) -> ExternResult<PopulationAccountantStateSnapshotV1> {
    let observation_started_at = sys_time()?;
    let record = get(state_hash.clone(), GetOptions::network())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Trusted population accountant state was not found".into())
    ))?;
    let state: TrustedPopulationAccountantState =
        decode_entry(&record, "trusted population accountant state")?;

    let first_targets = observed_correction_targets(&state_hash)?;
    let mut corrections = Vec::with_capacity(first_targets.len());
    for correction_hash in &first_targets {
        let correction_record = get(correction_hash.clone(), GetOptions::network())?.ok_or(
            wasm_error!(WasmErrorInner::Guest(
                "Observed population accountant correction could not be materialized".into()
            )),
        )?;
        let correction: PopulationAccountantStateCorrection =
            decode_entry(&correction_record, "population accountant correction")?;
        if correction.state_hash != state_hash
            || correction.private_receipt_commitment
                != state.projection.private_receipt_commitment
        {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Observed population accountant correction crosses state lineage".into()
            )));
        }
        corrections.push(ObservedPopulationAccountantCorrectionV1 {
            action_hash: correction_hash.clone(),
            correction,
        });
    }

    let second_targets = observed_correction_targets(&state_hash)?;
    let observation_completed_at = sys_time()?;
    if first_targets != second_targets {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Population accountant correction set changed during materialization; retry".into()
        )));
    }

    Ok(PopulationAccountantStateSnapshotV1 {
        state_action_hash: state_hash,
        state,
        corrections,
        read_boundary: PopulationAccountantReadBoundaryV1::NetworkBackedStableDoubleRead,
        observed_correction_target_count: first_targets.len(),
        observation_started_at,
        observation_completed_at,
    })
}

fn observed_correction_targets(state_hash: &ActionHash) -> ExternResult<Vec<ActionHash>> {
    let links = get_links(
        LinkQuery::try_new(state_hash.clone(), LinkTypes::StateToCorrections)?,
        GetStrategy::Network,
    )?;
    if links.len() > MAX_CORRECTIONS_PER_STATE {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Population accountant state has more than {MAX_CORRECTIONS_PER_STATE} observed corrections"
        ))));
    }
    let mut targets = Vec::with_capacity(links.len());
    for link in links {
        let target = link.target.into_action_hash().ok_or(wasm_error!(
            WasmErrorInner::Guest("Population accountant correction target must be ActionHash".into())
        ))?;
        if !targets.contains(&target) {
            targets.push(target);
        }
    }
    targets.sort_by(|left, right| left.get_raw_36().cmp(right.get_raw_36()));
    Ok(targets)
}

fn decode_entry<T>(record: &Record, label: &'static str) -> ExternResult<T>
where
    T: TryFrom<SerializedBytes, Error = SerializedBytesError>,
{
    record.entry().to_app_option::<T>()
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(format!(
            "Referenced {label} entry is missing or wrong type"
        ))))
}
