#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Policy-, producer-authority-, and exact-status-bound aggregate surveillance.
//!
//! Publication fails closed unless the DNA defines a structural release policy,
//! trusted producer-authority issuers, and at least one accepted positive status
//! endorsement profile. Every peer revalidates the aggregate, release policy,
//! publisher/subject binding, producer grant, exact observation endorsement, and
//! both detached Ed25519 signatures locally.
//!
//! This authenticates publication authority and one issuer assertion of current
//! standing for the exact observation. It does not authenticate scientific
//! lineage independence, diagnose disease, declare outbreaks, recommend
//! treatment, or authorize emergency response.

use hdi::prelude::*;
pub use health_surveillance_authority::*;
pub use health_surveillance_core::*;
pub use health_surveillance_endorsement::*;
use sha2::{Digest, Sha256};

pub const RELEASE_POLICY_ID_DOMAIN_V1: &[u8] =
    b"mycelix-health-surveillance-release-policy-v1\0";
const MAX_TRUSTED_AUTHORITY_ISSUERS: usize = 64;
const MAX_STATUS_PROFILES: usize = 64;
const ED25519_SIGNATURE_BYTES: usize = 64;

/// Immutable semantic identity of the exact release-policy authority configured
/// into one surveillance DNA. A human revision label alone is not an identity.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ReleasePolicyId([u8; 32]);

impl ReleasePolicyId {
    fn from_validated_properties(
        policy_revision: &CanonicalId,
        properties: &ReleasePolicyProperties,
    ) -> Self {
        let mut h = Sha256::new();
        h.update(RELEASE_POLICY_ID_DOMAIN_V1);
        let revision = policy_revision.as_str().as_bytes();
        h.update((revision.len() as u32).to_be_bytes());
        h.update(revision);
        h.update(properties.min_cohort_size.to_be_bytes());
        h.update(properties.min_window_s.to_be_bytes());
        h.update([match properties.max_geographic_precision {
            GeographicPrecision::Country => 0,
            GeographicPrecision::Region => 1,
            GeographicPrecision::District => 2,
            GeographicPrecision::Facility => 3,
        }]);
        Self(h.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReleasePolicyProperties {
    pub policy_revision: String,
    pub min_cohort_size: u64,
    pub min_window_s: u64,
    pub max_geographic_precision: GeographicPrecision,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TrustedAuthorityIssuerProperties {
    pub security_domain: String,
    pub issuer_did: String,
    pub credential_schema_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProducerAuthorityPolicyProperties {
    /// Upper bound on one broad producer grant. Finite grants remain useful for
    /// degraded/offline resilience but do not replace exact online endorsement.
    pub max_grant_lifetime_s: u64,
    pub trusted_issuers: Vec<TrustedAuthorityIssuerProperties>,
    /// Exact positive status-check profiles that may endorse observations.
    /// Missing/empty fails closed.
    #[serde(default)]
    pub accepted_status_profiles: Vec<String>,
}

#[dna_properties]
#[derive(Clone, Debug, PartialEq)]
pub struct SurveillanceDnaProperties {
    pub release_policy: Option<ReleasePolicyProperties>,
    pub producer_authority_policy: Option<ProducerAuthorityPolicyProperties>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConfiguredReleasePolicy {
    pub policy_revision: CanonicalId,
    pub policy_id: ReleasePolicyId,
    pub policy: AggregateReleasePolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfiguredTrustedAuthorityIssuer {
    pub security_domain: CanonicalId,
    pub issuer_did: CanonicalId,
    pub credential_schema_id: CanonicalId,
    pub issuer_pubkey: AgentPubKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfiguredProducerAuthorityPolicy {
    pub max_grant_lifetime_s: u64,
    pub trusted_issuers: Vec<ConfiguredTrustedAuthorityIssuer>,
    pub accepted_status_profiles: Vec<CanonicalId>,
}

#[derive(Clone, Debug)]
pub struct AuthorityVerificationPlan {
    pub issuer_pubkey: AgentPubKey,
    pub signing_transcript: Vec<u8>,
    pub grant_id: ProducerAuthorityGrantId,
    pub scope_assessment: ClaimedScopeAssessment,
    pub endorsement_signing_transcript: Vec<u8>,
    pub endorsement_id: ObservationEndorsementId,
    pub endorsement_binding: EndorsementBindingAssessment,
}

pub fn configured_release_policy() -> ExternResult<ConfiguredReleasePolicy> {
    let properties = SurveillanceDnaProperties::try_from_dna_properties()?;
    let configured = properties.release_policy.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "Public-health surveillance publication is disabled: no DNA release_policy is configured"
                .to_string()
        ))
    })?;
    configured_release_policy_from_properties(&configured).map_err(|message| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Invalid surveillance DNA release policy: {message}"
        )))
    })
}

pub fn configured_producer_authority_policy() -> ExternResult<ConfiguredProducerAuthorityPolicy> {
    let properties = SurveillanceDnaProperties::try_from_dna_properties()?;
    let configured = properties.producer_authority_policy.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "Public-health surveillance publication is disabled: no DNA producer_authority_policy is configured"
                .to_string()
        ))
    })?;
    configured_producer_authority_policy_from_properties(&configured).map_err(|message| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Invalid surveillance DNA producer authority policy: {message}"
        )))
    })
}

pub fn configured_release_policy_from_properties(
    properties: &ReleasePolicyProperties,
) -> Result<ConfiguredReleasePolicy, String> {
    let policy_revision = CanonicalId::new(properties.policy_revision.clone())
        .map_err(|e| format!("invalid policy_revision: {e}"))?;
    let policy = AggregateReleasePolicy::new(
        properties.min_cohort_size,
        properties.min_window_s,
        properties.max_geographic_precision,
    )
    .map_err(|e| e.to_string())?;
    let policy_id = ReleasePolicyId::from_validated_properties(&policy_revision, properties);
    Ok(ConfiguredReleasePolicy {
        policy_revision,
        policy_id,
        policy,
    })
}

pub fn configured_producer_authority_policy_from_properties(
    properties: &ProducerAuthorityPolicyProperties,
) -> Result<ConfiguredProducerAuthorityPolicy, String> {
    if properties.max_grant_lifetime_s == 0 {
        return Err("max_grant_lifetime_s must be greater than zero".to_string());
    }
    if properties.trusted_issuers.is_empty() {
        return Err("trusted_issuers must contain at least one issuer".to_string());
    }
    if properties.trusted_issuers.len() > MAX_TRUSTED_AUTHORITY_ISSUERS {
        return Err(format!(
            "trusted_issuers exceeds maximum of {MAX_TRUSTED_AUTHORITY_ISSUERS}"
        ));
    }
    if properties.accepted_status_profiles.is_empty() {
        return Err("accepted_status_profiles must contain at least one profile".to_string());
    }
    if properties.accepted_status_profiles.len() > MAX_STATUS_PROFILES {
        return Err(format!(
            "accepted_status_profiles exceeds maximum of {MAX_STATUS_PROFILES}"
        ));
    }

    let mut trusted_issuers = Vec::with_capacity(properties.trusted_issuers.len());
    for issuer in &properties.trusted_issuers {
        let security_domain = CanonicalId::new(issuer.security_domain.clone())
            .map_err(|e| format!("invalid authority security_domain: {e}"))?;
        let issuer_did = CanonicalId::new(issuer.issuer_did.clone())
            .map_err(|e| format!("invalid authority issuer_did: {e}"))?;
        let credential_schema_id = CanonicalId::new(issuer.credential_schema_id.clone())
            .map_err(|e| format!("invalid authority credential_schema_id: {e}"))?;
        let issuer_pubkey = parse_mycelix_did_agent(&issuer_did)?;
        let configured = ConfiguredTrustedAuthorityIssuer {
            security_domain,
            issuer_did,
            credential_schema_id,
            issuer_pubkey,
        };
        if trusted_issuers.iter().any(|existing| existing == &configured) {
            return Err("trusted_issuers contains a duplicate issuer tuple".to_string());
        }
        trusted_issuers.push(configured);
    }
    trusted_issuers.sort_by(|a, b| {
        (
            a.security_domain.as_str(),
            a.issuer_did.as_str(),
            a.credential_schema_id.as_str(),
        )
            .cmp(&(
                b.security_domain.as_str(),
                b.issuer_did.as_str(),
                b.credential_schema_id.as_str(),
            ))
    });

    let mut accepted_status_profiles = Vec::with_capacity(properties.accepted_status_profiles.len());
    for profile in &properties.accepted_status_profiles {
        let profile = CanonicalId::new(profile.clone())
            .map_err(|e| format!("invalid accepted status profile: {e}"))?;
        if accepted_status_profiles.contains(&profile) {
            return Err("accepted_status_profiles contains a duplicate profile".to_string());
        }
        accepted_status_profiles.push(profile);
    }
    accepted_status_profiles.sort();

    Ok(ConfiguredProducerAuthorityPolicy {
        max_grant_lifetime_s: properties.max_grant_lifetime_s,
        trusted_issuers,
        accepted_status_profiles,
    })
}

fn parse_mycelix_did_agent(did: &CanonicalId) -> Result<AgentPubKey, String> {
    let encoded = did
        .as_str()
        .strip_prefix("did:mycelix:")
        .ok_or_else(|| "trusted producer-authority issuer must use did:mycelix".to_string())?;
    AgentPubKey::try_from(encoded.to_string())
        .map_err(|_| "could not parse did:mycelix issuer AgentPubKey".to_string())
}

fn expected_subject_did(author: &AgentPubKey) -> Result<CanonicalId, String> {
    CanonicalId::new(format!("did:mycelix:{author}"))
        .map_err(|e| format!("could not derive publisher DID: {e}"))
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ReleasedSurveillanceObservation {
    pub observation: SurveillanceObservation,
    pub release_assessment: ReleaseAssessment,
    pub policy_revision: CanonicalId,
    pub policy_id: ReleasePolicyId,
    pub authority_grant: ProducerAuthorityGrant,
    pub authority_signature: Vec<u8>,
    /// Positive issuer status assertion bound to this exact observation.
    pub status_endorsement: AuthorizedObservationEndorsement,
    /// Detached Ed25519 signature over `status_endorsement.signing_transcript()`.
    pub status_endorsement_signature: Vec<u8>,
    pub publisher: AgentPubKey,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    ReleasedSurveillanceObservation(ReleasedSurveillanceObservation),
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
                EntryTypes::ReleasedSurveillanceObservation(entry) => {
                    validate_released_entry(&action.author, action.timestamp, &entry)
                }
            },
            OpEntry::UpdateEntry { .. } => Ok(ValidateCallbackResult::Invalid(
                "Released surveillance observations are append-only and cannot be updated".into(),
            )),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(_) => Ok(ValidateCallbackResult::Invalid(
            "Released surveillance observations are append-only and cannot be updated".into(),
        )),
        FlatOp::RegisterDelete(_) => Ok(ValidateCallbackResult::Invalid(
            "Released surveillance observations are evidence records and cannot be deleted".into(),
        )),
        FlatOp::RegisterCreateLink { .. } => Ok(ValidateCallbackResult::Invalid(
            "Surveillance v1 defines no publishable link operations".into(),
        )),
        FlatOp::RegisterDeleteLink { .. } => Ok(ValidateCallbackResult::Invalid(
            "Surveillance v1 defines no deletable link operations".into(),
        )),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_released_entry(
    action_author: &AgentPubKey,
    action_timestamp: Timestamp,
    entry: &ReleasedSurveillanceObservation,
) -> ExternResult<ValidateCallbackResult> {
    let release_policy = configured_release_policy()?;
    let authority_policy = configured_producer_authority_policy()?;
    let authored_at_unix_s = action_timestamp.as_micros().div_euclid(1_000_000);
    let plan = match validate_released_entry_semantics(
        action_author,
        authored_at_unix_s,
        entry,
        &release_policy,
        &authority_policy,
    ) {
        Ok(plan) => plan,
        Err(message) => return Ok(ValidateCallbackResult::Invalid(message)),
    };

    if !verify_authority_signature(&plan, &entry.authority_signature)? {
        return Ok(ValidateCallbackResult::Invalid(
            "producer-authority signature verification failed".to_string(),
        ));
    }
    if !verify_endorsement_signature(&plan, &entry.status_endorsement_signature)? {
        return Ok(ValidateCallbackResult::Invalid(
            "exact-observation status endorsement signature verification failed".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

/// Perform every deterministic semantic check except host cryptography.
pub fn validate_released_entry_semantics(
    action_author: &AgentPubKey,
    authored_at_unix_s: i64,
    entry: &ReleasedSurveillanceObservation,
    release_policy: &ConfiguredReleasePolicy,
    authority_policy: &ConfiguredProducerAuthorityPolicy,
) -> Result<AuthorityVerificationPlan, String> {
    if &entry.publisher != action_author {
        return Err("publisher must equal the Holochain action author".to_string());
    }
    if entry.policy_revision != release_policy.policy_revision {
        return Err("entry policy_revision does not match the DNA release policy".to_string());
    }
    if entry.policy_id != release_policy.policy_id {
        return Err("entry policy_id does not match the exact DNA release policy".to_string());
    }
    if entry.authority_signature.len() != ED25519_SIGNATURE_BYTES {
        return Err(format!(
            "producer-authority Ed25519 signature must be {ED25519_SIGNATURE_BYTES} bytes"
        ));
    }
    if entry.status_endorsement_signature.len() != ED25519_SIGNATURE_BYTES {
        return Err(format!(
            "status-endorsement Ed25519 signature must be {ED25519_SIGNATURE_BYTES} bytes"
        ));
    }

    entry
        .observation
        .validate()
        .map_err(|e| format!("invalid surveillance observation: {e}"))?;
    entry
        .authority_grant
        .validate()
        .map_err(|e| format!("invalid producer-authority grant: {e}"))?;
    entry
        .status_endorsement
        .validate()
        .map_err(|e| format!("invalid exact-observation endorsement: {e}"))?;

    let expected_release = release_policy
        .policy
        .assess(&entry.observation)
        .map_err(|e| format!("release assessment failed: {e}"))?;
    if !expected_release.eligible_for_release() {
        return Err(format!(
            "observation does not satisfy the DNA release policy: {:?}",
            expected_release.reasons()
        ));
    }
    if entry.release_assessment != expected_release {
        return Err(
            "stored release assessment does not match deterministic policy evaluation".to_string(),
        );
    }

    let expected_subject = expected_subject_did(action_author)?;
    if entry.authority_grant.subject_did() != &expected_subject {
        return Err("producer-authority subject DID does not match DHT publisher".to_string());
    }

    let grant_lifetime_i64 = entry
        .authority_grant
        .valid_until_unix_s()
        .checked_sub(entry.authority_grant.valid_from_unix_s())
        .ok_or_else(|| "producer-authority grant lifetime overflow".to_string())?;
    let grant_lifetime_s = u64::try_from(grant_lifetime_i64)
        .map_err(|_| "producer-authority grant lifetime is invalid".to_string())?;
    if grant_lifetime_s > authority_policy.max_grant_lifetime_s {
        return Err(format!(
            "producer-authority grant lifetime exceeds DNA maximum of {} seconds",
            authority_policy.max_grant_lifetime_s
        ));
    }

    let trusted_issuer = authority_policy
        .trusted_issuers
        .iter()
        .find(|issuer| {
            &issuer.security_domain == entry.authority_grant.security_domain()
                && &issuer.issuer_did == entry.authority_grant.issuer_did()
                && &issuer.credential_schema_id == entry.authority_grant.credential_schema_id()
        })
        .ok_or_else(|| {
            "producer-authority issuer/security-domain/schema tuple is not trusted by this DNA"
                .to_string()
        })?;

    let scope_assessment = entry
        .authority_grant
        .assess_claimed_scope(&entry.observation, authored_at_unix_s)
        .map_err(|e| format!("producer-authority scope assessment failed: {e}"))?;
    if !scope_assessment.permitted_by_claimed_scope {
        return Err(format!(
            "observation is outside producer-authority scope: {:?}",
            scope_assessment.reasons
        ));
    }

    if !authority_policy
        .accepted_status_profiles
        .contains(entry.status_endorsement.status_profile_id())
    {
        return Err("status endorsement profile is not accepted by this DNA".to_string());
    }

    let endorsement_binding = entry
        .status_endorsement
        .assess_binding(
            &entry.authority_grant,
            &entry.observation,
            release_policy.policy_id.as_bytes(),
            &expected_subject,
        )
        .map_err(|e| format!("status endorsement binding failed: {e}"))?;
    if !endorsement_binding.binds_exactly {
        return Err(format!(
            "status endorsement does not bind this exact observation: {:?}",
            endorsement_binding.reasons
        ));
    }

    let signing_transcript = entry
        .authority_grant
        .signing_transcript()
        .map_err(|e| format!("could not construct producer-authority transcript: {e}"))?;
    let endorsement_signing_transcript = entry
        .status_endorsement
        .signing_transcript()
        .map_err(|e| format!("could not construct status endorsement transcript: {e}"))?;

    Ok(AuthorityVerificationPlan {
        issuer_pubkey: trusted_issuer.issuer_pubkey.clone(),
        signing_transcript,
        grant_id: entry
            .authority_grant
            .id()
            .map_err(|e| format!("could not derive producer-authority ID: {e}"))?,
        scope_assessment,
        endorsement_signing_transcript,
        endorsement_id: entry
            .status_endorsement
            .id()
            .map_err(|e| format!("could not derive status endorsement ID: {e}"))?,
        endorsement_binding,
    })
}

pub fn verify_authority_signature(
    plan: &AuthorityVerificationPlan,
    signature_bytes: &[u8],
) -> ExternResult<bool> {
    verify_ed25519_raw(
        plan.issuer_pubkey.clone(),
        &plan.signing_transcript,
        signature_bytes,
    )
}

pub fn verify_endorsement_signature(
    plan: &AuthorityVerificationPlan,
    signature_bytes: &[u8],
) -> ExternResult<bool> {
    verify_ed25519_raw(
        plan.issuer_pubkey.clone(),
        &plan.endorsement_signing_transcript,
        signature_bytes,
    )
}

fn verify_ed25519_raw(
    issuer_pubkey: AgentPubKey,
    transcript: &[u8],
    signature_bytes: &[u8],
) -> ExternResult<bool> {
    let raw: [u8; ED25519_SIGNATURE_BYTES] = match signature_bytes.try_into() {
        Ok(raw) => raw,
        Err(_) => return Ok(false),
    };
    verify_signature_raw(
        issuer_pubkey,
        Signature::from(raw),
        transcript.to_vec(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> CanonicalId {
        CanonicalId::new(value).unwrap()
    }

    fn agent(byte: u8) -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![byte; 36])
    }

    fn did(agent: &AgentPubKey) -> String {
        format!("did:mycelix:{agent}")
    }

    fn release_policy() -> ConfiguredReleasePolicy {
        configured_release_policy_from_properties(&ReleasePolicyProperties {
            policy_revision: "district-release-v1".to_string(),
            min_cohort_size: 50,
            min_window_s: 3_600,
            max_geographic_precision: GeographicPrecision::District,
        })
        .unwrap()
    }

    fn authority_policy(issuer: &AgentPubKey) -> ConfiguredProducerAuthorityPolicy {
        configured_producer_authority_policy_from_properties(
            &ProducerAuthorityPolicyProperties {
                max_grant_lifetime_s: 86_400,
                trusted_issuers: vec![TrustedAuthorityIssuerProperties {
                    security_domain: "identity-domain:public-health-v1".to_string(),
                    issuer_did: did(issuer),
                    credential_schema_id:
                        "mycelix:schema:health:surveillance-publisher:v1".to_string(),
                }],
                accepted_status_profiles: vec!["mycelix-vc-active-status-v1".to_string()],
            },
        )
        .unwrap()
    }

    fn observation(cohort_size: u64) -> SurveillanceObservation {
        SurveillanceObservation::new(
            SignalFamily::Respiratory,
            SourceKind::LaboratoryAggregate,
            "lab-feed-a",
            IndependenceGroup::new("lab-lineage-a").unwrap(),
            GeographicScope::new("health-district", "district-17", GeographicPrecision::District)
                .unwrap(),
            ObservationWindow::new(10_000, 13_600).unwrap(),
            13_700,
            cohort_size,
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
                "rev-1",
                Some("upstream-lab-a"),
                SourceRecordDigest::sha256([7; 32]).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn grant(publisher: &AgentPubKey, issuer: &AgentPubKey) -> ProducerAuthorityGrant {
        ProducerAuthorityGrant::new(
            id("identity-domain:public-health-v1"),
            id("mycelix:schema:health:surveillance-publisher:v1"),
            id(&did(issuer)),
            id(&did(publisher)),
            id("lab-a"),
            ProducerAuthorityScope::new(
                vec![SourceKind::LaboratoryAggregate],
                vec![SignalFamily::Respiratory],
                vec![id("lab-feed-a")],
                vec![id("aggregate-protocol-v1")],
                vec![GeographicScope::new(
                    "health-district",
                    "district-17",
                    GeographicPrecision::District,
                )
                .unwrap()],
            )
            .unwrap(),
            9_000,
            20_000,
            [5; 32],
        )
        .unwrap()
    }

    fn released_entry(
        publisher: &AgentPubKey,
        issuer: &AgentPubKey,
        observation: SurveillanceObservation,
    ) -> ReleasedSurveillanceObservation {
        let release = release_policy();
        let assessment = release.policy.assess(&observation).unwrap();
        let authority_grant = grant(publisher, issuer);
        let status_endorsement = AuthorizedObservationEndorsement::new(
            id("mycelix-vc-active-status-v1"),
            authority_grant.issuer_did().clone(),
            authority_grant.id().unwrap(),
            observation.id().unwrap(),
            *release.policy_id.as_bytes(),
            id(&did(publisher)),
            observation.provenance.producer.clone(),
            13_701,
            [9; 32],
            [8; 32],
        )
        .unwrap();
        ReleasedSurveillanceObservation {
            observation,
            release_assessment: assessment,
            policy_revision: release.policy_revision,
            policy_id: release.policy_id,
            authority_grant,
            authority_signature: vec![1; ED25519_SIGNATURE_BYTES],
            status_endorsement,
            status_endorsement_signature: vec![2; ED25519_SIGNATURE_BYTES],
            publisher: publisher.clone(),
        }
    }

    #[test]
    fn missing_status_profile_fails_closed() {
        let issuer = agent(3);
        assert!(configured_producer_authority_policy_from_properties(
            &ProducerAuthorityPolicyProperties {
                max_grant_lifetime_s: 3600,
                trusted_issuers: vec![TrustedAuthorityIssuerProperties {
                    security_domain: "identity-domain:public-health-v1".into(),
                    issuer_did: did(&issuer),
                    credential_schema_id:
                        "mycelix:schema:health:surveillance-publisher:v1".into(),
                }],
                accepted_status_profiles: vec![],
            }
        )
        .is_err());
    }

    #[test]
    fn exact_release_authority_and_status_semantics_produce_plan() {
        let publisher = agent(1);
        let issuer = agent(3);
        let entry = released_entry(&publisher, &issuer, observation(100));
        let plan = validate_released_entry_semantics(
            &publisher,
            14_000,
            &entry,
            &release_policy(),
            &authority_policy(&issuer),
        )
        .unwrap();
        assert_eq!(plan.issuer_pubkey, issuer);
        assert!(plan.scope_assessment.permitted_by_claimed_scope);
        assert!(plan.endorsement_binding.binds_exactly);
        assert_eq!(plan.endorsement_id, entry.status_endorsement.id().unwrap());
    }

    #[test]
    fn publisher_subject_substitution_fails() {
        let publisher = agent(1);
        let issuer = agent(3);
        let entry = released_entry(&publisher, &issuer, observation(100));
        assert!(validate_released_entry_semantics(
            &agent(2),
            14_000,
            &entry,
            &release_policy(),
            &authority_policy(&issuer),
        )
        .is_err());
    }

    #[test]
    fn observation_substitution_fails_exact_status_binding() {
        let publisher = agent(1);
        let issuer = agent(3);
        let mut entry = released_entry(&publisher, &issuer, observation(100));
        entry.observation.provenance.source_revision = id("rev-2");
        entry.release_assessment = release_policy().policy.assess(&entry.observation).unwrap();
        assert!(validate_released_entry_semantics(
            &publisher,
            14_000,
            &entry,
            &release_policy(),
            &authority_policy(&issuer),
        )
        .is_err());
    }

    #[test]
    fn unaccepted_status_profile_fails_before_crypto() {
        let publisher = agent(1);
        let issuer = agent(3);
        let entry = released_entry(&publisher, &issuer, observation(100));
        let mut policy = authority_policy(&issuer);
        policy.accepted_status_profiles = vec![id("different-status-profile")];
        assert!(validate_released_entry_semantics(
            &publisher,
            14_000,
            &entry,
            &release_policy(),
            &policy,
        )
        .is_err());
    }

    #[test]
    fn malformed_status_signature_fails_before_host_crypto() {
        let publisher = agent(1);
        let issuer = agent(3);
        let mut entry = released_entry(&publisher, &issuer, observation(100));
        entry.status_endorsement_signature = vec![2; 63];
        assert!(validate_released_entry_semantics(
            &publisher,
            14_000,
            &entry,
            &release_policy(),
            &authority_policy(&issuer),
        )
        .is_err());
    }

    #[test]
    fn release_policy_substitution_fails_status_binding() {
        let publisher = agent(1);
        let issuer = agent(3);
        let entry = released_entry(&publisher, &issuer, observation(100));
        let other_release = configured_release_policy_from_properties(&ReleasePolicyProperties {
            policy_revision: "other-release".to_string(),
            min_cohort_size: 50,
            min_window_s: 3_600,
            max_geographic_precision: GeographicPrecision::District,
        })
        .unwrap();
        assert!(validate_released_entry_semantics(
            &publisher,
            14_000,
            &entry,
            &other_release,
            &authority_policy(&issuer),
        )
        .is_err());
    }
}
