#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Coordinator API for policy-bound aggregate public-health surveillance.
//!
//! The coordinator performs the same release-policy evaluation as the integrity
//! callback for fast caller feedback, but integrity validation remains the source
//! of truth. No function in this zome diagnoses disease, declares an outbreak, or
//! creates emergency authority.

use hdk::prelude::*;
use surveillance_integrity::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitSurveillanceObservationInput {
    pub observation: SurveillanceObservation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitSurveillanceObservationOutput {
    pub action_hash: ActionHash,
    pub observation_id: ObservationId,
    pub policy_revision: CanonicalId,
    pub policy_id: ReleasePolicyId,
    pub publisher: AgentPubKey,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReleasePolicyView {
    pub policy_revision: CanonicalId,
    pub policy_id: ReleasePolicyId,
    pub policy: AggregateReleasePolicy,
}

/// Return the release policy frozen into this DNA's integrity properties.
/// Fails closed when publication is disabled or the property contract is invalid.
#[hdk_extern]
pub fn get_release_policy(_: ()) -> ExternResult<ReleasePolicyView> {
    let configured = configured_release_policy()?;
    Ok(ReleasePolicyView {
        policy_revision: configured.policy_revision,
        policy_id: configured.policy_id,
        policy: configured.policy,
    })
}

/// Submit one aggregate surveillance observation.
///
/// This performs a coordinator-side preflight. Every peer independently repeats
/// the same policy evaluation in the integrity callback before accepting the DHT
/// operation.
#[hdk_extern]
pub fn submit_surveillance_observation(
    input: SubmitSurveillanceObservationInput,
) -> ExternResult<SubmitSurveillanceObservationOutput> {
    let configured = configured_release_policy()?;
    let assessment = configured.policy.assess(&input.observation).map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Invalid surveillance observation: {e}"
        )))
    })?;

    if !assessment.eligible_for_release() {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Observation withheld by DNA release policy: {:?}",
            assessment.reasons()
        ))));
    }

    let publisher = agent_info()?.agent_initial_pubkey;
    let observation_id = input.observation.id().map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Could not derive observation identity: {e}"
        )))
    })?;

    let entry = ReleasedSurveillanceObservation {
        observation: input.observation,
        release_assessment: assessment,
        policy_revision: configured.policy_revision.clone(),
        policy_id: configured.policy_id,
        publisher: publisher.clone(),
    };

    let action_hash = create_entry(&EntryTypes::ReleasedSurveillanceObservation(entry))?;

    Ok(SubmitSurveillanceObservationOutput {
        action_hash,
        observation_id,
        policy_revision: configured.policy_revision,
        policy_id: configured.policy_id,
        publisher,
    })
}

/// Fetch one released aggregate observation by its action hash.
#[hdk_extern]
pub fn get_surveillance_observation(
    action_hash: ActionHash,
) -> ExternResult<Option<ReleasedSurveillanceObservation>> {
    let record = match get(action_hash, GetOptions::default())? {
        Some(record) => record,
        None => return Ok(None),
    };

    record
        .entry()
        .to_app_option::<ReleasedSurveillanceObservation>()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Stored record is not a released surveillance observation: {e}"
            )))
        })
}
