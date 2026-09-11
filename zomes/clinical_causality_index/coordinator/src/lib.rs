#![deny(unsafe_code)]
//! Canonical admission/discovery for privacy-minimized causal attestations.
//!
//! Admission is represented by an append-only validated link from a deterministic
//! synthetic anchor derived from the already-public opaque receipt commitment to
//! one exact causal attestation action. High-assurance readers consume only
//! index-admitted publications; unindexed attestations remain noncanonical evidence.

use clinical_causality_index_integrity::{
    commitment_index_anchor_hash, LinkTypes as IndexLinkTypes, OpaqueCausalCommitmentV1,
};
use clinical_causality_integrity::{
    CausalAttestationCorrection, LinkTypes as CausalLinkTypes,
    QualifiedCausalAssessmentAttestation,
};
use hdk::prelude::*;

const MAX_INDEXED_ATTESTATIONS_PER_COMMITMENT: usize = 256;
const MAX_CORRECTIONS_PER_ATTESTATION: usize = 256;
const MAX_TOTAL_CORRECTIONS_PER_CANONICAL_SNAPSHOT: usize = 4096;

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

#[derive(Clone, Serialize, Deserialize)]
pub struct CanonicalObservedCausalCorrectionV1 {
    pub action_hash: ActionHash,
    pub correction: CausalAttestationCorrection,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CanonicalCorrectionReadBoundaryV1 {
    NetworkBackedStableDoubleReadWithFinalClosureCheck,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CanonicalCausalPublicationSnapshotV1 {
    pub attestation_action_hash: ActionHash,
    pub attestation: QualifiedCausalAssessmentAttestation,
    pub corrections: Vec<CanonicalObservedCausalCorrectionV1>,
    pub correction_read_boundary: CanonicalCorrectionReadBoundaryV1,
    pub observed_correction_target_count: usize,
    pub correction_observation_started_at: Timestamp,
    pub correction_observation_completed_at: Timestamp,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CanonicalCausalCommitmentReadBoundaryV1 {
    NestedNetworkBackedStableReadsWithFinalClosureChecks,
}

/// One bounded canonical snapshot of every index-admitted publication observed for
/// one opaque receipt commitment plus every correction observed for those publications.
#[derive(Clone, Serialize, Deserialize)]
pub struct CanonicalCausalCommitmentSnapshotV1 {
    pub commitment: OpaqueCausalCommitmentV1,
    pub anchor_hash: EntryHash,
    pub publications: Vec<CanonicalCausalPublicationSnapshotV1>,
    pub read_boundary: CanonicalCausalCommitmentReadBoundaryV1,
    pub observed_attestation_target_count: usize,
    pub observed_total_correction_target_count: usize,
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
        IndexLinkTypes::CommitmentToAttestations,
        (),
    )
}

/// Discover index-admitted causal attestations for one exact opaque receipt
/// commitment, requiring the target set to remain stable across two network-backed
/// reads around materialization.
#[hdk_extern]
pub fn materialize_causal_commitment_index_snapshot(
    commitment: OpaqueCausalCommitmentV1,
) -> ExternResult<CausalCommitmentIndexSnapshotV1> {
    let observation_started_at = sys_time()?;
    let anchor_hash = commitment_index_anchor_hash(commitment)?;
    let first_targets = observed_attestation_targets(&anchor_hash)?;

    let mut attestations = Vec::with_capacity(first_targets.len());
    for action_hash in &first_targets {
        let attestation = materialize_exact_attestation(action_hash, commitment)?;
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

/// Materialize one bounded canonical commitment snapshot.
///
/// The operation stable-double-reads the canonical index, stable-double-reads every
/// admitted publication's correction set, rechecks the outer index, and finally
/// rechecks every correction target set once more. It therefore captures a tightly
/// bounded observed state while making no claim that an unpropagated future/remote
/// admission or correction does not exist.
#[hdk_extern]
pub fn materialize_canonical_causal_commitment_snapshot(
    commitment: OpaqueCausalCommitmentV1,
) -> ExternResult<CanonicalCausalCommitmentSnapshotV1> {
    let observation_started_at = sys_time()?;
    let anchor_hash = commitment_index_anchor_hash(commitment)?;
    let first_attestation_targets = observed_attestation_targets(&anchor_hash)?;

    let mut total_corrections = 0usize;
    let mut publications = Vec::with_capacity(first_attestation_targets.len());

    for attestation_hash in &first_attestation_targets {
        let publication = materialize_canonical_publication(attestation_hash, commitment)?;
        total_corrections = total_corrections
            .checked_add(publication.observed_correction_target_count)
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Canonical causal snapshot correction count overflow".to_string()
            )))?;
        if total_corrections > MAX_TOTAL_CORRECTIONS_PER_CANONICAL_SNAPSHOT {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "Canonical causal snapshot exceeds total correction limit of {MAX_TOTAL_CORRECTIONS_PER_CANONICAL_SNAPSHOT}"
            ))));
        }
        publications.push(publication);
    }

    let second_attestation_targets = observed_attestation_targets(&anchor_hash)?;
    if first_attestation_targets != second_attestation_targets {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Canonical causal index changed while publication corrections were being materialized"
                .to_string()
        )));
    }

    // Final closure pass: ensure no observed correction target set changed after its
    // inner double-read while later publications were being materialized.
    for publication in &publications {
        let final_targets = observed_correction_targets(&publication.attestation_action_hash)?;
        let expected_targets: Vec<ActionHash> = publication
            .corrections
            .iter()
            .map(|record| record.action_hash.clone())
            .collect();
        if final_targets != expected_targets {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Canonical causal correction set changed before final snapshot closure"
                    .to_string()
            )));
        }
    }

    let observation_completed_at = sys_time()?;
    Ok(CanonicalCausalCommitmentSnapshotV1 {
        commitment,
        anchor_hash,
        publications,
        read_boundary:
            CanonicalCausalCommitmentReadBoundaryV1::NestedNetworkBackedStableReadsWithFinalClosureChecks,
        observed_attestation_target_count: first_attestation_targets.len(),
        observed_total_correction_target_count: total_corrections,
        observation_started_at,
        observation_completed_at,
    })
}

fn materialize_canonical_publication(
    attestation_hash: &ActionHash,
    commitment: OpaqueCausalCommitmentV1,
) -> ExternResult<CanonicalCausalPublicationSnapshotV1> {
    let correction_observation_started_at = sys_time()?;
    let attestation = materialize_exact_attestation(attestation_hash, commitment)?;
    let first_correction_targets = observed_correction_targets(attestation_hash)?;

    let mut corrections = Vec::with_capacity(first_correction_targets.len());
    for correction_hash in &first_correction_targets {
        let record = get(correction_hash.clone(), GetOptions::network())?.ok_or(wasm_error!(
            WasmErrorInner::Guest(
                "Observed canonical causal correction could not be materialized".to_string()
            )
        ))?;
        let correction: CausalAttestationCorrection =
            decode_entry(&record, "causal attestation correction")?;
        if correction.attestation_hash != *attestation_hash {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Canonical causal correction targets another attestation".to_string()
            )));
        }
        if correction.qualified_receipt_commitment != commitment {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Canonical causal correction crosses opaque receipt-commitment lineage"
                    .to_string()
            )));
        }
        corrections.push(CanonicalObservedCausalCorrectionV1 {
            action_hash: correction_hash.clone(),
            correction,
        });
    }

    let second_correction_targets = observed_correction_targets(attestation_hash)?;
    let correction_observation_completed_at = sys_time()?;
    if first_correction_targets != second_correction_targets {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Canonical causal correction set changed during publication materialization"
                .to_string()
        )));
    }

    Ok(CanonicalCausalPublicationSnapshotV1 {
        attestation_action_hash: attestation_hash.clone(),
        attestation,
        corrections,
        correction_read_boundary:
            CanonicalCorrectionReadBoundaryV1::NetworkBackedStableDoubleReadWithFinalClosureCheck,
        observed_correction_target_count: first_correction_targets.len(),
        correction_observation_started_at,
        correction_observation_completed_at,
    })
}

fn materialize_exact_attestation(
    action_hash: &ActionHash,
    commitment: OpaqueCausalCommitmentV1,
) -> ExternResult<QualifiedCausalAssessmentAttestation> {
    let record = get(action_hash.clone(), GetOptions::network())?.ok_or(wasm_error!(
        WasmErrorInner::Guest(
            "Observed canonical causal attestation could not be materialized".to_string()
        )
    ))?;
    let attestation: QualifiedCausalAssessmentAttestation =
        decode_entry(&record, "qualified causal attestation")?;
    if attestation.attestation.qualified_receipt_commitment != commitment {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Canonical causal index target crosses opaque receipt-commitment lineage".to_string()
        )));
    }
    Ok(attestation)
}

fn observed_attestation_targets(anchor_hash: &EntryHash) -> ExternResult<Vec<ActionHash>> {
    let links = get_links(
        LinkQuery::try_new(
            anchor_hash.clone(),
            IndexLinkTypes::CommitmentToAttestations,
        )?,
        GetStrategy::Network,
    )?;

    if links.len() > MAX_INDEXED_ATTESTATIONS_PER_COMMITMENT {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Causal receipt commitment has more than {MAX_INDEXED_ATTESTATIONS_PER_COMMITMENT} observed canonical attestation links; refusing unbounded materialization"
        ))));
    }

    deduplicated_action_targets(links, "Canonical causal commitment index target")
}

fn observed_correction_targets(attestation_hash: &ActionHash) -> ExternResult<Vec<ActionHash>> {
    let links = get_links(
        LinkQuery::try_new(
            attestation_hash.clone(),
            CausalLinkTypes::AttestationToCorrections,
        )?,
        GetStrategy::Network,
    )?;

    if links.len() > MAX_CORRECTIONS_PER_ATTESTATION {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Canonical causal attestation has more than {MAX_CORRECTIONS_PER_ATTESTATION} observed correction links; refusing unbounded materialization"
        ))));
    }

    deduplicated_action_targets(links, "Canonical causal correction target")
}

fn deduplicated_action_targets(
    links: Vec<Link>,
    label: &'static str,
) -> ExternResult<Vec<ActionHash>> {
    let mut targets = Vec::with_capacity(links.len());
    for link in links {
        let target = link.target.into_action_hash().ok_or(wasm_error!(
            WasmErrorInner::Guest(format!("{label} must be an ActionHash"))
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
