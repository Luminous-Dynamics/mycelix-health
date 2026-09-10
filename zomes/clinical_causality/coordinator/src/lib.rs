#![deny(unsafe_code)]
//! Thin coordinator for qualified clinical causal-assessment attestations.
//!
//! Causal assessment/qualification belongs to the pure crates and trust enforcement
//! belongs to the integrity zome. This coordinator only commits exact entries/links.

use clinical_causality_integrity::{
    CausalAssessmentVerifierAuthorization, CausalAttestationCorrection, EntryTypes, LinkTypes,
    QualifiedCausalAssessmentAttestation,
};
use hdk::prelude::*;

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
