#![deny(unsafe_code)]
//! Canonical admission and bounded discovery for trusted population-accountant state.
//!
//! The high-assurance read path admits only integrity-validated index members and
//! returns a nested network-backed snapshot whose observed state membership and
//! every observed correction set remain stable through final closure checks.
//! This is not a proof of global DHT completeness.

use hdk::prelude::*;
use population_accountant_index_integrity::{
    accountant_lineage_anchor_hash, LinkTypes, PopulationAccountantLineageAnchorV1,
};
use population_accountant_integrity::{
    LinkTypes as AccountantLinkTypes, PopulationAccountantStateCorrection,
    TrustedPopulationAccountantState,
};

const MAX_STATES_PER_LINEAGE: usize = 4096;
const MAX_CORRECTIONS_PER_STATE: usize = 256;
const MAX_TOTAL_CORRECTIONS: usize = 16_384;

#[derive(Clone, Serialize, Deserialize)]
pub struct CanonicalAccountantObservedCorrectionV1 {
    pub action_hash: ActionHash,
    pub correction: PopulationAccountantStateCorrection,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CanonicalAccountantObservedStateV1 {
    pub action_hash: ActionHash,
    pub state: TrustedPopulationAccountantState,
    pub corrections: Vec<CanonicalAccountantObservedCorrectionV1>,
    pub observed_correction_target_count: usize,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CanonicalAccountantReadBoundaryV1 {
    NestedNetworkBackedStableReadsWithFinalClosureChecks,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CanonicalPopulationAccountantLineageSnapshotV1 {
    pub lineage: PopulationAccountantLineageAnchorV1,
    pub admitted_states: Vec<CanonicalAccountantObservedStateV1>,
    pub observed_state_count: usize,
    pub observed_total_correction_count: usize,
    pub read_boundary: CanonicalAccountantReadBoundaryV1,
    pub observation_started_at: Timestamp,
    pub observation_completed_at: Timestamp,
}

#[hdk_extern]
pub fn admit_population_accountant_state(state_hash: ActionHash) -> ExternResult<ActionHash> {
    let record = get(state_hash.clone(), GetOptions::network())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Trusted population accountant state was not found".into())
    ))?;
    let state: TrustedPopulationAccountantState =
        decode_entry(&record, "trusted population accountant state")?;
    let lineage = PopulationAccountantLineageAnchorV1::from_state(&state)?;
    let anchor = accountant_lineage_anchor_hash(&lineage)?;
    create_link(anchor, state_hash, LinkTypes::LineageToStates, ())
}

/// Materialize one exact accountant lineage from its deterministic canonical-admission index.
///
/// Closure sequence:
/// 1. read admitted state set A;
/// 2. for each state, stable-double-read and materialize corrections A/B;
/// 3. read admitted state set B and require A == B;
/// 4. re-read every state's correction set C and require it still equals A/B.
///
/// This proves only a bounded stable observed view at this conductor. It cannot prove
/// absence of unpropagated DHT activity or future transitions.
#[hdk_extern]
pub fn materialize_canonical_population_accountant_lineage(
    lineage: PopulationAccountantLineageAnchorV1,
) -> ExternResult<CanonicalPopulationAccountantLineageSnapshotV1> {
    let observation_started_at = sys_time()?;
    let anchor = accountant_lineage_anchor_hash(&lineage)?;
    let first_state_hashes = observed_state_hashes(&anchor)?;
    let mut admitted_states = Vec::with_capacity(first_state_hashes.len());
    let mut total_corrections = 0usize;

    for state_hash in &first_state_hashes {
        let record = get(state_hash.clone(), GetOptions::network())?.ok_or(wasm_error!(
            WasmErrorInner::Guest(
                "Observed accountant admission target could not be materialized".into()
            )
        ))?;
        let state: TrustedPopulationAccountantState =
            decode_entry(&record, "trusted population accountant state")?;
        let actual_lineage = PopulationAccountantLineageAnchorV1::from_state(&state)?;
        if actual_lineage != lineage {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Observed accountant admission crosses canonical lineage".into()
            )));
        }

        let first_corrections = observed_correction_hashes(state_hash)?;
        total_corrections = total_corrections
            .checked_add(first_corrections.len())
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Population accountant correction count overflow".into()
            )))?;
        if total_corrections > MAX_TOTAL_CORRECTIONS {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "Population accountant lineage has more than {MAX_TOTAL_CORRECTIONS} observed corrections"
            ))));
        }

        let mut corrections = Vec::with_capacity(first_corrections.len());
        for correction_hash in &first_corrections {
            let correction_record =
                get(correction_hash.clone(), GetOptions::network())?.ok_or(wasm_error!(
                    WasmErrorInner::Guest(
                        "Observed accountant correction could not be materialized".into()
                    )
                ))?;
            let correction: PopulationAccountantStateCorrection =
                decode_entry(&correction_record, "population accountant correction")?;
            if correction.state_hash != *state_hash
                || correction.private_receipt_commitment
                    != state.projection.private_receipt_commitment
            {
                return Err(wasm_error!(WasmErrorInner::Guest(
                    "Observed accountant correction crosses state lineage".into()
                )));
            }
            corrections.push(CanonicalAccountantObservedCorrectionV1 {
                action_hash: correction_hash.clone(),
                correction,
            });
        }

        let second_corrections = observed_correction_hashes(state_hash)?;
        if first_corrections != second_corrections {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Accountant correction set changed during inner materialization; retry".into()
            )));
        }

        admitted_states.push(CanonicalAccountantObservedStateV1 {
            action_hash: state_hash.clone(),
            state,
            corrections,
            observed_correction_target_count: first_corrections.len(),
        });
    }

    let second_state_hashes = observed_state_hashes(&anchor)?;
    if first_state_hashes != second_state_hashes {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Canonical accountant admission set changed during materialization; retry".into()
        )));
    }

    for state in &admitted_states {
        let final_corrections = observed_correction_hashes(&state.action_hash)?;
        let expected: Vec<ActionHash> = state
            .corrections
            .iter()
            .map(|value| value.action_hash.clone())
            .collect();
        if final_corrections != expected {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Accountant correction set changed before final snapshot closure; retry".into()
            )));
        }
    }

    let observation_completed_at = sys_time()?;
    Ok(CanonicalPopulationAccountantLineageSnapshotV1 {
        lineage,
        admitted_states,
        observed_state_count: first_state_hashes.len(),
        observed_total_correction_count: total_corrections,
        read_boundary: CanonicalAccountantReadBoundaryV1::NestedNetworkBackedStableReadsWithFinalClosureChecks,
        observation_started_at,
        observation_completed_at,
    })
}

fn observed_state_hashes(anchor: &EntryHash) -> ExternResult<Vec<ActionHash>> {
    let links = get_links(
        LinkQuery::try_new(anchor.clone(), LinkTypes::LineageToStates)?,
        GetStrategy::Network,
    )?;
    if links.len() > MAX_STATES_PER_LINEAGE {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Population accountant lineage has more than {MAX_STATES_PER_LINEAGE} admitted states"
        ))));
    }
    canonical_action_targets(links, "accountant state admission")
}

fn observed_correction_hashes(state_hash: &ActionHash) -> ExternResult<Vec<ActionHash>> {
    let links = get_links(
        LinkQuery::try_new(state_hash.clone(), AccountantLinkTypes::StateToCorrections)?,
        GetStrategy::Network,
    )?;
    if links.len() > MAX_CORRECTIONS_PER_STATE {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Population accountant state has more than {MAX_CORRECTIONS_PER_STATE} corrections"
        ))));
    }
    canonical_action_targets(links, "accountant correction")
}

fn canonical_action_targets(links: Vec<Link>, label: &'static str) -> ExternResult<Vec<ActionHash>> {
    let mut targets = Vec::with_capacity(links.len());
    for link in links {
        let target = link.target.into_action_hash().ok_or(wasm_error!(
            WasmErrorInner::Guest(format!("{label} link target must be ActionHash"))
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
    record
        .entry()
        .to_app_option::<T>()
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(format!(
            "Referenced {label} entry is missing or wrong type"
        ))))
}
