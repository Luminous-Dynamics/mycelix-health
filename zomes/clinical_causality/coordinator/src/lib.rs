#![deny(unsafe_code)]
//! Thin coordinator for qualified clinical causal-assessment attestations.
//!
//! Causal assessment/qualification belongs to the pure crates and trust enforcement
//! belongs to the integrity zome. This coordinator commits exact entries/links and
//! exposes a bounded stable-double-read materializer for one known attestation.

use clinical_causality_integrity::{
    CausalAssessmentVerifierAuthorization, CausalAttestationCorrection, EntryTypes, LinkTypes,
    QualifiedCausalAssessmentAttestation,
};
use hdk::prelude::*;

const MAX_CORRECTIONS_PER_ATTESTATION: usize = 256;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObservedCausalCorrectionV1 {
    pub action_hash: ActionHash,
    pub correction: CausalAttestationCorrection,
}

/// One locally observed, stable-double-read publication snapshot.
///
/// This is deliberately not named a globally complete read set. Holochain propagation is
/// eventually consistent: a correction that has not reached this conductor cannot be proven
/// absent. The guarantee here is narrower: the correction-link target set was unchanged across
/// two local queries surrounding materialization, and every observed target was fetched and
/// decoded successfully.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CausalAttestationPublicationSnapshotV1 {
    pub attestation_action_hash: ActionHash,
    pub attestation: QualifiedCausalAssessmentAttestation,
    pub corrections: Vec<ObservedCausalCorrectionV1>,
    pub observation_started_at: Timestamp,
    pub observation_completed_at: Timestamp,
}

#[hdk_extern]
pub fn create_causal_verifier_authorization(
    authorization: CausalAssessmentVerifierAuthorization,
) -> ExternResult<Record> {
    let hash = create_entry(EntryTypes::CausalAssessmentVerifierAuthorization(authorization))?;
    get(hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Committed causal verifier authorization could not be read back".to_string()
    )))
}

#[hdk_extern]
pub fn publish_qualified_causal_attestation(
    attestation: QualifiedCausalAssessmentAttestation,
) -> ExternResult<Record> {
    let hash = create_entry(EntryTypes::QualifiedCausalAssessmentAttestation(attestation))?;
    get(hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Committed qualified causal attestation could not be read back".to_string()
    )))
}

#[hdk_extern]
pub fn append_causal_attestation_correction(
    correction: CausalAttestationCorrection,
) -> ExternResult<Record> {
    let attestation_hash = correction.attestation_hash.clone();
    let correction_hash = create_entry(EntryTypes::CausalAttestationCorrection(correction))?;
    create_link(
        attestation_hash,
        correction_hash.clone(),
        LinkTypes::AttestationToCorrections,
        (),
    )?;
    get(correction_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Committed causal attestation correction could not be read back".to_string()
    )))
}

/// Materialize one known causal-attestation publication and every correction link observed by
/// this conductor, requiring the correction target set to be stable across two local reads.
///
/// This does **not** prove global absence of unseen/unpropagated corrections or discover every
/// other publication that may share the same opaque receipt commitment. Consumers must retain
/// that bounded-read distinction in any safety decision.
#[hdk_extern]
pub fn materialize_causal_attestation_publication_snapshot(
    attestation_hash: ActionHash,
) -> ExternResult<CausalAttestationPublicationSnapshotV1> {
    let observation_started_at = sys_time()?;

    let attestation_record = get(attestation_hash.clone(), GetOptions::default())?.ok_or(
        wasm_error!(WasmErrorInner::Guest(
            "Qualified causal attestation was not found".to_string()
        )),
    )?;
    let attestation: QualifiedCausalAssessmentAttestation =
        decode_entry(&attestation_record, "qualified causal attestation")?;

    let first_targets = observed_correction_targets(&attestation_hash)?;
    let mut corrections = Vec::with_capacity(first_targets.len());

    for correction_hash in &first_targets {
        let record = get(correction_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
            WasmErrorInner::Guest(
                "Observed causal correction link target could not be materialized".to_string()
            )
        ))?;
        let correction: CausalAttestationCorrection =
            decode_entry(&record, "causal attestation correction")?;

        if correction.attestation_hash != attestation_hash {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Observed causal correction does not target the requested attestation".to_string()
            )));
        }
        if correction.qualified_receipt_commitment
            != attestation.attestation.qualified_receipt_commitment
        {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Observed causal correction crosses opaque receipt-commitment lineage".to_string()
            )));
        }

        corrections.push(ObservedCausalCorrectionV1 {
            action_hash: correction_hash.clone(),
            correction,
        });
    }

    let second_targets = observed_correction_targets(&attestation_hash)?;
    let observation_completed_at = sys_time()?;
    if first_targets != second_targets {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Causal correction link set changed during materialization; retry from a fresh snapshot"
                .to_string()
        )));
    }

    Ok(CausalAttestationPublicationSnapshotV1 {
        attestation_action_hash: attestation_hash,
        attestation,
        corrections,
        observation_started_at,
        observation_completed_at,
    })
}

fn observed_correction_targets(attestation_hash: &ActionHash) -> ExternResult<Vec<ActionHash>> {
    let links = get_links(
        LinkQuery::try_new(
            attestation_hash.clone(),
            LinkTypes::AttestationToCorrections,
        )?,
        GetStrategy::default(),
    )?;

    if links.len() > MAX_CORRECTIONS_PER_ATTESTATION {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Causal attestation has more than {MAX_CORRECTIONS_PER_ATTESTATION} observed correction links; refusing unbounded materialization"
        ))));
    }

    let mut targets = Vec::with_capacity(links.len());
    for link in links {
        let target = link.target.into_action_hash().ok_or(wasm_error!(
            WasmErrorInner::Guest(
                "Causal correction link target must be an ActionHash".to_string()
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
