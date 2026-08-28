#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Coordinator API for additive signed surveillance lineage evidence.
//!
//! The caller supplies an externally signed lineage attestation. The coordinator
//! verifies it for fast feedback, but the lineage integrity zome independently
//! repeats target-observation binding, attestor-policy, and signature checks.

use hdk::prelude::*;
use health_surveillance_core::{CanonicalId, ObservationId};
use surveillance_lineage_integrity::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitLineageAttestationInput {
    pub observation_action: ActionHash,
    pub attestation: EvidenceLineageAttestation,
    pub attestation_signature: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitLineageAttestationOutput {
    pub action_hash: ActionHash,
    pub observation_action: ActionHash,
    pub observation_id: ObservationId,
    pub attestation_id: LineageAttestationId,
    pub submitted_by: AgentPubKey,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustedLineageAttestorView {
    pub security_domain: CanonicalId,
    pub attestor_did: CanonicalId,
    pub attestation_profile_id: CanonicalId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LineageAttestorPolicyView {
    pub trusted_attestors: Vec<TrustedLineageAttestorView>,
}

#[hdk_extern]
pub fn get_lineage_attestor_policy(_: ()) -> ExternResult<LineageAttestorPolicyView> {
    let configured = configured_lineage_attestor_policy()?;
    Ok(LineageAttestorPolicyView {
        trusted_attestors: configured
            .trusted_attestors
            .into_iter()
            .map(|attestor| TrustedLineageAttestorView {
                security_domain: attestor.security_domain,
                attestor_did: attestor.attestor_did,
                attestation_profile_id: attestor.attestation_profile_id,
            })
            .collect(),
    })
}

#[hdk_extern]
pub fn submit_lineage_attestation(
    input: SubmitLineageAttestationInput,
) -> ExternResult<SubmitLineageAttestationOutput> {
    let record = get(input.observation_action.clone(), GetOptions::default())?.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "referenced released surveillance observation was not found".to_string()
        ))
    })?;
    let released: ReleasedSurveillanceObservationMirror = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "observation_action does not decode as a released surveillance observation"
                    .to_string()
            ))
        })?;

    let submitted_by = agent_info()?.agent_initial_pubkey;
    let entry = ReleasedLineageAttestation {
        observation_action: input.observation_action.clone(),
        attestation: input.attestation,
        attestation_signature: input.attestation_signature,
        submitted_by: submitted_by.clone(),
    };

    let policy = configured_lineage_attestor_policy()?;
    let plan = validate_lineage_attestation_semantics(&entry, &released.observation, &policy)
        .map_err(|message| wasm_error!(WasmErrorInner::Guest(message)))?;
    if !verify_lineage_signature(&plan, &entry.attestation_signature)? {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "lineage attestation signature verification failed".to_string()
        )));
    }

    let observation_id = released.observation.id().map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "could not derive referenced observation identity: {e}"
        )))
    })?;
    let attestation_id = plan.attestation_id;
    let action_hash = create_entry(&EntryTypes::ReleasedLineageAttestation(entry))?;

    Ok(SubmitLineageAttestationOutput {
        action_hash,
        observation_action: input.observation_action,
        observation_id,
        attestation_id,
        submitted_by,
    })
}

#[hdk_extern]
pub fn get_lineage_attestation(
    action_hash: ActionHash,
) -> ExternResult<Option<ReleasedLineageAttestation>> {
    let record = match get(action_hash, GetOptions::default())? {
        Some(record) => record,
        None => return Ok(None),
    };
    record
        .entry()
        .to_app_option::<ReleasedLineageAttestation>()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "stored record is not a released lineage attestation: {e}"
            )))
        })
}
