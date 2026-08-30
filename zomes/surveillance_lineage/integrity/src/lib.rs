#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Signed, append-only lineage evidence for released aggregate observations.
//!
//! Lineage trust is intentionally separate from producer authority. This zome
//! accepts an attestation only when its exact security-domain / attestor-DID /
//! profile tuple is frozen into this DNA's lineage-attestor policy and the
//! detached Ed25519 signature verifies over the canonical lineage transcript.
//!
//! Lineage evidence is additive. It does not gate base observation publication,
//! and conflicting valid attestations are not overwritten or collapsed here.

use hdi::prelude::*;
use health_surveillance_authority::ProducerAuthorityGrant;
use health_surveillance_core::{CanonicalId, ReleaseAssessment, SurveillanceObservation};
use health_surveillance_endorsement::AuthorizedObservationEndorsement;
pub use health_surveillance_lineage::*;

const MAX_TRUSTED_LINEAGE_ATTESTORS: usize = 64;
const ED25519_SIGNATURE_BYTES: usize = 64;

/// Read-only serialization mirror of `surveillance_integrity::ReleasedSurveillanceObservation`.
///
/// Keeping this mirror in shared semantic types avoids linking one integrity-zome
/// implementation into another WASM. `policy_id` is represented by its transparent
/// 32-byte wire value; lineage validation does not interpret release-policy authority.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReleasedSurveillanceObservationMirror {
    pub observation: SurveillanceObservation,
    pub release_assessment: ReleaseAssessment,
    pub policy_revision: CanonicalId,
    pub policy_id: [u8; 32],
    pub authority_grant: ProducerAuthorityGrant,
    pub authority_signature: Vec<u8>,
    pub status_endorsement: AuthorizedObservationEndorsement,
    pub status_endorsement_signature: Vec<u8>,
    pub publisher: AgentPubKey,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TrustedLineageAttestorProperties {
    pub security_domain: String,
    pub attestor_did: String,
    pub attestation_profile_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LineageAttestorPolicyProperties {
    pub trusted_attestors: Vec<TrustedLineageAttestorProperties>,
}

/// This view intentionally contains only the lineage property. Extra DNA
/// properties owned by the base surveillance zome are ignored by serde.
#[dna_properties]
#[derive(Clone, PartialEq)]
pub struct LineageDnaProperties {
    pub lineage_attestor_policy: Option<LineageAttestorPolicyProperties>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfiguredTrustedLineageAttestor {
    pub security_domain: CanonicalId,
    pub attestor_did: CanonicalId,
    pub attestation_profile_id: CanonicalId,
    pub attestor_pubkey: AgentPubKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfiguredLineageAttestorPolicy {
    pub trusted_attestors: Vec<ConfiguredTrustedLineageAttestor>,
}

#[derive(Clone, Debug)]
pub struct LineageVerificationPlan {
    pub attestor_pubkey: AgentPubKey,
    pub signing_transcript: Vec<u8>,
    pub attestation_id: LineageAttestationId,
}

pub fn configured_lineage_attestor_policy() -> ExternResult<ConfiguredLineageAttestorPolicy> {
    let properties = LineageDnaProperties::try_from_dna_properties()?;
    let configured = properties.lineage_attestor_policy.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "Surveillance lineage publication is disabled: no DNA lineage_attestor_policy is configured"
                .to_string()
        ))
    })?;
    configured_lineage_attestor_policy_from_properties(&configured).map_err(|message| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Invalid surveillance lineage-attestor policy: {message}"
        )))
    })
}

pub fn configured_lineage_attestor_policy_from_properties(
    properties: &LineageAttestorPolicyProperties,
) -> Result<ConfiguredLineageAttestorPolicy, String> {
    if properties.trusted_attestors.is_empty() {
        return Err("trusted_attestors must contain at least one attestor".to_string());
    }
    if properties.trusted_attestors.len() > MAX_TRUSTED_LINEAGE_ATTESTORS {
        return Err(format!(
            "trusted_attestors exceeds maximum of {MAX_TRUSTED_LINEAGE_ATTESTORS}"
        ));
    }

    let mut trusted_attestors = Vec::with_capacity(properties.trusted_attestors.len());
    for attestor in &properties.trusted_attestors {
        let security_domain = CanonicalId::new(attestor.security_domain.clone())
            .map_err(|e| format!("invalid lineage security_domain: {e}"))?;
        let attestor_did = CanonicalId::new(attestor.attestor_did.clone())
            .map_err(|e| format!("invalid lineage attestor_did: {e}"))?;
        let attestation_profile_id = CanonicalId::new(attestor.attestation_profile_id.clone())
            .map_err(|e| format!("invalid lineage attestation_profile_id: {e}"))?;
        let attestor_pubkey = parse_mycelix_did_agent(&attestor_did)?;
        let configured = ConfiguredTrustedLineageAttestor {
            security_domain,
            attestor_did,
            attestation_profile_id,
            attestor_pubkey,
        };
        if trusted_attestors.iter().any(|existing| existing == &configured) {
            return Err("trusted_attestors contains a duplicate trust tuple".to_string());
        }
        trusted_attestors.push(configured);
    }

    trusted_attestors.sort_by(|a, b| {
        (
            a.security_domain.as_str(),
            a.attestor_did.as_str(),
            a.attestation_profile_id.as_str(),
        )
            .cmp(&(
                b.security_domain.as_str(),
                b.attestor_did.as_str(),
                b.attestation_profile_id.as_str(),
            ))
    });

    Ok(ConfiguredLineageAttestorPolicy { trusted_attestors })
}

fn parse_mycelix_did_agent(did: &CanonicalId) -> Result<AgentPubKey, String> {
    let encoded = did
        .as_str()
        .strip_prefix("did:mycelix:")
        .ok_or_else(|| "trusted lineage attestor must use did:mycelix".to_string())?;
    AgentPubKey::try_from(encoded.to_string())
        .map_err(|_| "could not parse lineage attestor did:mycelix AgentPubKey".to_string())
}

/// One immutable signed lineage statement about one already released observation.
///
/// `submitted_by` authenticates the DHT relay/author. It is intentionally not
/// required to equal the attestor: a valid signed attestation may be relayed by
/// another agent without changing who made the lineage claim.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ReleasedLineageAttestation {
    pub observation_action: ActionHash,
    pub attestation: EvidenceLineageAttestation,
    pub attestation_signature: Vec<u8>,
    pub submitted_by: AgentPubKey,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    ReleasedLineageAttestation(ReleasedLineageAttestation),
}

#[hdk_link_types]
pub enum LinkTypes {
    Reserved,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::ReleasedLineageAttestation(entry) => {
                    validate_released_lineage_attestation(&action.author, &entry)
                }
            },
            OpEntry::UpdateEntry { .. } => Ok(ValidateCallbackResult::Invalid(
                "Released lineage attestations are append-only and cannot be updated".into(),
            )),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(_) => Ok(ValidateCallbackResult::Invalid(
            "Released lineage attestations are append-only and cannot be updated".into(),
        )),
        FlatOp::RegisterDelete(_) => Ok(ValidateCallbackResult::Invalid(
            "Released lineage attestations are evidence records and cannot be deleted".into(),
        )),
        FlatOp::RegisterCreateLink { .. } => Ok(ValidateCallbackResult::Invalid(
            "Surveillance lineage v1 defines no publishable link operations".into(),
        )),
        FlatOp::RegisterDeleteLink { .. } => Ok(ValidateCallbackResult::Invalid(
            "Surveillance lineage v1 defines no deletable link operations".into(),
        )),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_released_lineage_attestation(
    action_author: &AgentPubKey,
    entry: &ReleasedLineageAttestation,
) -> ExternResult<ValidateCallbackResult> {
    if &entry.submitted_by != action_author {
        return Ok(ValidateCallbackResult::Invalid(
            "lineage submitted_by must equal the Holochain action author".to_string(),
        ));
    }

    let record = must_get_valid_record(entry.observation_action.clone())?;
    let released: ReleasedSurveillanceObservationMirror = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "lineage observation_action does not decode as a released surveillance observation"
                    .to_string()
            ))
        })?;

    let policy = configured_lineage_attestor_policy()?;
    let plan = match validate_lineage_attestation_semantics(entry, &released.observation, &policy) {
        Ok(plan) => plan,
        Err(message) => return Ok(ValidateCallbackResult::Invalid(message)),
    };

    match verify_lineage_signature(&plan, &entry.attestation_signature)? {
        true => Ok(ValidateCallbackResult::Valid),
        false => Ok(ValidateCallbackResult::Invalid(
            "lineage attestation signature verification failed".to_string(),
        )),
    }
}

/// Pure semantic validation once integrity has resolved the referenced released
/// observation. This is the substitution-test boundary before host crypto.
pub fn validate_lineage_attestation_semantics(
    entry: &ReleasedLineageAttestation,
    observation: &SurveillanceObservation,
    policy: &ConfiguredLineageAttestorPolicy,
) -> Result<LineageVerificationPlan, String> {
    if entry.attestation_signature.len() != ED25519_SIGNATURE_BYTES {
        return Err(format!(
            "lineage Ed25519 signature must be {ED25519_SIGNATURE_BYTES} bytes"
        ));
    }
    entry
        .attestation
        .validate()
        .map_err(|e| format!("invalid lineage attestation: {e}"))?;

    if !entry
        .attestation
        .binds_observation(observation)
        .map_err(|e| format!("lineage observation binding failed: {e}"))?
    {
        return Err("lineage attestation does not bind the referenced observation".to_string());
    }
    if entry.attestation.assessed_at_unix_s() < observation.reported_at_unix_s {
        return Err(
            "lineage attestation assessment time cannot claim to precede observation report time"
                .to_string(),
        );
    }

    let trusted = policy
        .trusted_attestors
        .iter()
        .find(|trusted| {
            &trusted.security_domain == entry.attestation.security_domain()
                && &trusted.attestor_did == entry.attestation.attestor_did()
                && &trusted.attestation_profile_id == entry.attestation.attestation_profile_id()
        })
        .ok_or_else(|| {
            "lineage security-domain/attestor/profile tuple is not trusted by this DNA".to_string()
        })?;

    let signing_transcript = entry
        .attestation
        .signing_transcript()
        .map_err(|e| format!("could not construct lineage signing transcript: {e}"))?;
    let attestation_id = entry
        .attestation
        .id()
        .map_err(|e| format!("could not derive lineage attestation ID: {e}"))?;

    Ok(LineageVerificationPlan {
        attestor_pubkey: trusted.attestor_pubkey.clone(),
        signing_transcript,
        attestation_id,
    })
}

pub fn verify_lineage_signature(
    plan: &LineageVerificationPlan,
    signature_bytes: &[u8],
) -> ExternResult<bool> {
    let raw: [u8; ED25519_SIGNATURE_BYTES] = match signature_bytes.try_into() {
        Ok(raw) => raw,
        Err(_) => return Ok(false),
    };
    verify_signature_raw(
        plan.attestor_pubkey.clone(),
        Signature::from(raw),
        plan.signing_transcript.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use health_surveillance_core::{
        BoundedUncertainty, EvidenceProvenance, GeographicPrecision, GeographicScope,
        IndependenceGroup, MetricKind, ObservationWindow, ObservedMetric, SignalFamily, SourceKind,
        SourceRecordDigest,
    };

    fn id(value: &str) -> CanonicalId {
        CanonicalId::new(value).unwrap()
    }

    fn agent(byte: u8) -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![byte; 36])
    }

    fn did(agent: &AgentPubKey) -> String {
        format!("did:mycelix:{agent}")
    }

    fn observation(revision: &str) -> SurveillanceObservation {
        SurveillanceObservation::new(
            SignalFamily::Respiratory,
            SourceKind::LaboratoryAggregate,
            "lab-feed-a",
            IndependenceGroup::new("producer-claim-lineage-a").unwrap(),
            GeographicScope::new("health-district", "district-17", GeographicPrecision::District)
                .unwrap(),
            ObservationWindow::new(10_000, 13_600).unwrap(),
            13_700,
            100,
            ObservedMetric::new(
                MetricKind::FractionPositive,
                0.20,
                BoundedUncertainty::new(0.15, 0.25).unwrap(),
                "fraction",
            )
            .unwrap(),
            EvidenceProvenance::new(
                "lab-a",
                "aggregate-protocol-v1",
                revision,
                Some("upstream-lab-a"),
                SourceRecordDigest::sha256([7; 32]).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn policy(attestor: &AgentPubKey) -> ConfiguredLineageAttestorPolicy {
        configured_lineage_attestor_policy_from_properties(&LineageAttestorPolicyProperties {
            trusted_attestors: vec![TrustedLineageAttestorProperties {
                security_domain: "lineage-domain:public-health-v1".into(),
                attestor_did: did(attestor),
                attestation_profile_id: "lineage-profile:multi-dimension-v1".into(),
            }],
        })
        .unwrap()
    }

    fn attestation(
        observation: &SurveillanceObservation,
        attestor: &AgentPubKey,
    ) -> EvidenceLineageAttestation {
        EvidenceLineageAttestation::new(
            id("lineage-domain:public-health-v1"),
            id("lineage-profile:multi-dimension-v1"),
            id(&did(attestor)),
            observation.id().unwrap(),
            LineageDescriptor::new(
                LineageKnowledge::known([id("root:dataset-a")]).unwrap(),
                LineageKnowledge::known([id("sample:district-17")]).unwrap(),
                LineageKnowledge::unknown(),
                LineageKnowledge::known([id("instrument:lab-a")]).unwrap(),
                LineageKnowledge::known([id("pipeline:aggregate-v1")]).unwrap(),
                LineageKnowledge::known([id("control:audited-lab-a")]).unwrap(),
            )
            .unwrap(),
            13_800,
            [4; 32],
            [5; 32],
        )
        .unwrap()
    }

    fn entry(observation: &SurveillanceObservation, attestor: &AgentPubKey) -> ReleasedLineageAttestation {
        ReleasedLineageAttestation {
            observation_action: ActionHash::from_raw_36(vec![7; 36]),
            attestation: attestation(observation, attestor),
            attestation_signature: vec![1; 64],
            submitted_by: agent(9),
        }
    }

    #[test]
    fn empty_trust_policy_fails_closed() {
        assert!(configured_lineage_attestor_policy_from_properties(
            &LineageAttestorPolicyProperties {
                trusted_attestors: vec![],
            }
        )
        .is_err());
    }

    #[test]
    fn exact_lineage_semantics_produce_verification_plan() {
        let attestor = agent(3);
        let observation = observation("rev-1");
        let entry = entry(&observation, &attestor);
        let plan = validate_lineage_attestation_semantics(&entry, &observation, &policy(&attestor))
            .unwrap();
        assert_eq!(plan.attestor_pubkey, attestor);
        assert_eq!(plan.attestation_id, entry.attestation.id().unwrap());
    }

    #[test]
    fn observation_substitution_fails_before_crypto() {
        let attestor = agent(3);
        let original = observation("rev-1");
        let changed = observation("rev-2");
        let entry = entry(&original, &attestor);
        assert!(validate_lineage_attestation_semantics(&entry, &changed, &policy(&attestor)).is_err());
    }

    #[test]
    fn unrelated_attestor_does_not_inherit_trust() {
        let trusted_attestor = agent(3);
        let other_attestor = agent(2);
        let observation = observation("rev-1");
        let entry = entry(&observation, &other_attestor);
        assert!(validate_lineage_attestation_semantics(
            &entry,
            &observation,
            &policy(&trusted_attestor),
        )
        .is_err());
    }

    #[test]
    fn relay_identity_is_not_lineage_authority() {
        let attestor = agent(3);
        let observation = observation("rev-1");
        let mut entry = entry(&observation, &attestor);
        entry.submitted_by = agent(42);
        assert!(validate_lineage_attestation_semantics(
            &entry,
            &observation,
            &policy(&attestor),
        )
        .is_ok());
    }

    #[test]
    fn malformed_signature_length_fails_before_host_crypto() {
        let attestor = agent(3);
        let observation = observation("rev-1");
        let mut entry = entry(&observation, &attestor);
        entry.attestation_signature = vec![1; 63];
        assert!(validate_lineage_attestation_semantics(
            &entry,
            &observation,
            &policy(&attestor),
        )
        .is_err());
    }
}
