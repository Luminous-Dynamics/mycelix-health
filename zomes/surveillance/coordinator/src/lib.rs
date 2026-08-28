#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Coordinator API for policy- and producer-authority-bound surveillance.
//!
//! Coordinator preflight exists for fast caller feedback only. Every DHT peer
//! independently repeats release-policy, issuer-trust, grant-scope, subject, and
//! detached-signature verification in the integrity callback.

use hdk::prelude::*;
use surveillance_integrity::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitSurveillanceObservationInput {
    pub observation: SurveillanceObservation,
    pub authority_grant: ProducerAuthorityGrant,
    /// Detached Ed25519 signature by the grant issuer over the exact shared
    /// `ProducerAuthorityGrant::signing_transcript()` bytes.
    pub authority_signature: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitSurveillanceObservationOutput {
    pub action_hash: ActionHash,
    pub observation_id: ObservationId,
    pub policy_revision: CanonicalId,
    pub policy_id: ReleasePolicyId,
    pub authority_grant_id: ProducerAuthorityGrantId,
    pub publisher: AgentPubKey,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReleasePolicyView {
    pub policy_revision: CanonicalId,
    pub policy_id: ReleasePolicyId,
    pub policy: AggregateReleasePolicy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustedAuthorityIssuerView {
    pub security_domain: CanonicalId,
    pub issuer_did: CanonicalId,
    pub credential_schema_id: CanonicalId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProducerAuthorityPolicyView {
    pub max_grant_lifetime_s: u64,
    pub trusted_issuers: Vec<TrustedAuthorityIssuerView>,
}

/// Return the release policy frozen into this DNA's integrity properties.
#[hdk_extern]
pub fn get_release_policy(_: ()) -> ExternResult<ReleasePolicyView> {
    let configured = configured_release_policy()?;
    Ok(ReleasePolicyView {
        policy_revision: configured.policy_revision,
        policy_id: configured.policy_id,
        policy: configured.policy,
    })
}

/// Return the producer-authority trust contract frozen into DNA properties.
#[hdk_extern]
pub fn get_producer_authority_policy(_: ()) -> ExternResult<ProducerAuthorityPolicyView> {
    let configured = configured_producer_authority_policy()?;
    Ok(ProducerAuthorityPolicyView {
        max_grant_lifetime_s: configured.max_grant_lifetime_s,
        trusted_issuers: configured
            .trusted_issuers
            .into_iter()
            .map(|issuer| TrustedAuthorityIssuerView {
                security_domain: issuer.security_domain,
                issuer_did: issuer.issuer_did,
                credential_schema_id: issuer.credential_schema_id,
            })
            .collect(),
    })
}

/// Submit one aggregate surveillance observation with a signed producer grant.
///
/// The caller supplies the external issuer proof; the local coordinator does not
/// mint institutional authority. It only verifies the proof against DNA-bound
/// trusted issuers before attempting the DHT write.
#[hdk_extern]
pub fn submit_surveillance_observation(
    input: SubmitSurveillanceObservationInput,
) -> ExternResult<SubmitSurveillanceObservationOutput> {
    let release_policy = configured_release_policy()?;
    let authority_policy = configured_producer_authority_policy()?;
    let publisher = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;
    let now_unix_s = now.as_micros().div_euclid(1_000_000);

    let release_assessment = release_policy
        .policy
        .assess(&input.observation)
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Invalid surveillance observation: {e}"
            )))
        })?;

    let observation_id = input.observation.id().map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Could not derive observation identity: {e}"
        )))
    })?;
    let authority_grant_id = input.authority_grant.id().map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Could not derive producer-authority identity: {e}"
        )))
    })?;

    let entry = ReleasedSurveillanceObservation {
        observation: input.observation,
        release_assessment,
        policy_revision: release_policy.policy_revision.clone(),
        policy_id: release_policy.policy_id,
        authority_grant: input.authority_grant,
        authority_signature: input.authority_signature,
        publisher: publisher.clone(),
    };

    let plan = validate_released_entry_semantics(
        &publisher,
        now_unix_s,
        &entry,
        &release_policy,
        &authority_policy,
    )
    .map_err(|message| wasm_error!(WasmErrorInner::Guest(message)))?;

    if !verify_authority_signature(&plan, &entry.authority_signature)? {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "producer-authority signature verification failed".to_string()
        )));
    }

    let action_hash = create_entry(&EntryTypes::ReleasedSurveillanceObservation(entry))?;

    Ok(SubmitSurveillanceObservationOutput {
        action_hash,
        observation_id,
        policy_revision: release_policy.policy_revision,
        policy_id: release_policy.policy_id,
        authority_grant_id,
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
