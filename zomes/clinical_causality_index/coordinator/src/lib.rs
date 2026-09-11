#![deny(unsafe_code)]
//! Canonical admission/discovery for privacy-minimized causal attestations.
//!
//! Admission is represented by an append-only validated link from a deterministic
//! synthetic anchor derived from the already-public opaque receipt commitment to
//! one exact causal attestation action. High-assurance readers consume only
//! index-admitted publications; unindexed attestations remain noncanonical evidence.

use clinical_causality_index_integrity::{
    commitment_index_anchor_hash, LinkTypes, OpaqueCausalCommitmentV1,
};
use clinical_causality_integrity::QualifiedCausalAssessmentAttestation;
use hdk::prelude::*;

const MAX_INDEXED_ATTESTATIONS_PER_COMMITMENT: usize = 256;

#[derive(Clone, Serialize, Deserialize)]
pub struct AdmitCausalAttestationInputV1 {
    pub attestation_action_hash: ActionHash,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct IndexedCausalAttestationV1 {
    pub action_hash: ActionHash,
    pub attestation: QualifiedCausalAssessmentAttestation,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CausalCommitmentIndexReadBoundaryV1 {
    NetworkBackedStableDoubleRead,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CausalCommitmentIndexSnapshotV1 {
    pub commitment: OpaqueCausalCommitmentV1,
    pub anchor_hash: EntryHash,
    pub attestations: Vec<IndexedCausalAttestationV1>,
    pub read_boundary: CausalCommitmentIndexReadBoundaryV1,
    pub observed_attestation_target_count: usize,
    pub observation_started_at: Timestamp,
    pub observation_completed_at: Timestamp,
}

/// Admit one exact already-published causal attestation into the canonical
/// commitment-scoped index.
///
/// Integrity validation requires the link author to be the target attestation
/// author and the deterministic base to match the target's opaque commitment.
#[hdk_extern]
pub fn admit_qualified_causal_attestation(
    input: AdmitCausalAttestationInputV1,
) -> ExternResult<ActionHash> {
    let record = get(
        input.attestation_action_hash.clone(),
        GetOptions::network(),
    )?
    .ok_or(wasm_error!(WasmErrorInner::Guest(
        "Qualified causal attestation was not found for canonical admission".to_string()
    )))?;
    let attestation: QualifiedCausalAssessmentAttestation =
        decode_entry(&record, "qualified causal attestation")?;
    let base = commitment_index_anchor_hash(
        attestation.attestation.qualified_receipt_commitment,
    )?;

    create_link(
        base,
        input.attestation_action_hash,
        LinkTypes::CommitmentToAttestations,
        (),
    )
}

/// Discover index-admitted causal attestations for one exact opaque receipt
/// commitment, requiring the target set to remain stable across two network-backed
/// reads around materialization.
///
/// This proves only stability of the observed canonical-index view during this
/// interval. It does not prove that an unpropagated valid admission does not exist.
#[hdk_extern]
pub fn materialize_causal_commitment_index_snapshot(
    commitment: OpaqueCausalCommitmentV1,
) -> ExternResult<CausalCommitmentIndexSnapshotV1> {
    let observation_started_at = sys_time()?;
    let anchor_hash = commitment_index_anchor_hash(commitment)?;
    let first_targets = observed_attestation_targets(&anchor_hash)?;

    let mut attestations = Vec::with_capacity(first_targets.len());
    for action_hash in &first_targets {
        let record = get(action_hash.clone(), GetOptions::network())?.ok_or(wasm_error!(
            WasmErrorInner::Guest(
                "Observed canonical causal attestation could not be materialized".to_string()
            )
        ))?;
        let attestation: QualifiedCausalAssessmentAttestation =
            decode_entry(&record, "qualified causal attestation")?;
        if attestation.attestation.qualified_receipt_commitment != commitment {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Canonical causal index target crosses opaque receipt-commitment lineage"
                    .to_string()
            )));
        }
        attestations.push(IndexedCausalAttestationV1 {
            action_hash: action_hash.clone(),
            attestation,
        });
    }

    let second_targets = observed_attestation_targets(&anchor_hash)?;
    let observation_completed_at = sys_time()?;
    if first_targets != second_targets {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Canonical causal index target set changed during materialization; retry from a fresh snapshot"
                .to_string()
        )));
    }

    Ok(CausalCommitmentIndexSnapshotV1 {
        commitment,
        anchor_hash,
        attestations,
        read_boundary: CausalCommitmentIndexReadBoundaryV1::NetworkBackedStableDoubleRead,
        observed_attestation_target_count: first_targets.len(),
        observation_started_at,
        observation_completed_at,
    })
}

fn observed_attestation_targets(anchor_hash: &EntryHash) -> ExternResult<Vec<ActionHash>> {
    let links = get_links(
        LinkQuery::try_new(
            anchor_hash.clone(),
            LinkTypes::CommitmentToAttestations,
        )?,
        GetStrategy::Network,
    )?;

    if links.len() > MAX_INDEXED_ATTESTATIONS_PER_COMMITMENT {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Causal receipt commitment has more than {MAX_INDEXED_ATTESTATIONS_PER_COMMITMENT} observed canonical attestation links; refusing unbounded materialization"
        ))));
    }

    let mut targets = Vec::with_capacity(links.len());
    for link in links {
        let target = link.target.into_action_hash().ok_or(wasm_error!(
            WasmErrorInner::Guest(
                "Canonical causal commitment index target must be an ActionHash".to_string()
            )
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
