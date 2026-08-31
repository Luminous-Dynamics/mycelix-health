#![deny(unsafe_code)]
//! Canonical exact-observation status endorsements for aggregate surveillance.
//!
//! This crate is intentionally crypto-free and I/O-free. It defines the exact
//! semantic object a credential/status authority signs after checking that a
//! producer grant is still acceptable for one exact observation. A signature
//! adapter lives elsewhere.
//!
//! An endorsement is positive by construction: it means the issuer is asserting
//! that the referenced producer authority was acceptable under `status_profile_id`
//! for the exact observation, publisher, producer, and release-policy identity in
//! this payload. It is not a general-purpose trust score or emergency authority.

use std::fmt;

use health_surveillance_authority::{AuthorityError, ProducerAuthorityGrant, ProducerAuthorityGrantId};
use health_surveillance_core::{CanonicalId, ObservationId, SurveillanceObservation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const OBSERVATION_ENDORSEMENT_SCHEMA_V1: u16 = 1;
pub const OBSERVATION_ENDORSEMENT_ID_DOMAIN_V1: &[u8] =
    b"mycelix-health-surveillance-observation-endorsement-id-v1\0";
pub const OBSERVATION_ENDORSEMENT_SIGNING_DOMAIN_V1: &[u8] =
    b"mycelix-health-surveillance-observation-endorsement-signing-v1\0";

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum EndorsementError {
    #[error("endorsement schema version {0} is unsupported")]
    UnsupportedSchema(u16),
    #[error("endorsement issuer and publisher must be DID identifiers")]
    InvalidDid,
    #[error("release policy identity must not be all zeroes")]
    ZeroReleasePolicyId,
    #[error("status evidence commitment must not be all zeroes")]
    ZeroStatusEvidenceCommitment,
    #[error("endorsement issuance nonce must not be all zeroes")]
    ZeroNonce,
    #[error("invalid producer authority grant: {0}")]
    InvalidGrant(String),
    #[error("invalid surveillance observation: {0}")]
    InvalidObservation(String),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ObservationEndorsementId([u8; 32]);

impl ObservationEndorsementId {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(64);
        for byte in self.0 {
            use fmt::Write as _;
            write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
        }
        out
    }
}

impl fmt::Display for ObservationEndorsementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// Positive issuer assertion over one exact observation.
///
/// `status_evidence_commitment` is an opaque content commitment to the external
/// credential/status evidence the issuer evaluated. The DHT does not interpret
/// or fetch that external evidence during consensus validation.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct AuthorizedObservationEndorsement {
    schema_version: u16,
    status_profile_id: CanonicalId,
    issuer_did: CanonicalId,
    grant_id: ProducerAuthorityGrantId,
    observation_id: ObservationId,
    release_policy_id: [u8; 32],
    publisher_did: CanonicalId,
    producer: CanonicalId,
    checked_at_unix_s: i64,
    status_evidence_commitment: [u8; 32],
    issuance_nonce: [u8; 32],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthorizedObservationEndorsementWire {
    schema_version: u16,
    status_profile_id: CanonicalId,
    issuer_did: CanonicalId,
    grant_id: ProducerAuthorityGrantId,
    observation_id: ObservationId,
    release_policy_id: [u8; 32],
    publisher_did: CanonicalId,
    producer: CanonicalId,
    checked_at_unix_s: i64,
    status_evidence_commitment: [u8; 32],
    issuance_nonce: [u8; 32],
}

impl<'de> Deserialize<'de> for AuthorizedObservationEndorsement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = AuthorizedObservationEndorsementWire::deserialize(deserializer)?;
        if wire.schema_version != OBSERVATION_ENDORSEMENT_SCHEMA_V1 {
            return Err(serde::de::Error::custom(EndorsementError::UnsupportedSchema(
                wire.schema_version,
            )));
        }
        Self::new(
            wire.status_profile_id,
            wire.issuer_did,
            wire.grant_id,
            wire.observation_id,
            wire.release_policy_id,
            wire.publisher_did,
            wire.producer,
            wire.checked_at_unix_s,
            wire.status_evidence_commitment,
            wire.issuance_nonce,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl AuthorizedObservationEndorsement {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        status_profile_id: CanonicalId,
        issuer_did: CanonicalId,
        grant_id: ProducerAuthorityGrantId,
        observation_id: ObservationId,
        release_policy_id: [u8; 32],
        publisher_did: CanonicalId,
        producer: CanonicalId,
        checked_at_unix_s: i64,
        status_evidence_commitment: [u8; 32],
        issuance_nonce: [u8; 32],
    ) -> Result<Self, EndorsementError> {
        if !issuer_did.as_str().starts_with("did:") || !publisher_did.as_str().starts_with("did:") {
            return Err(EndorsementError::InvalidDid);
        }
        if release_policy_id == [0; 32] {
            return Err(EndorsementError::ZeroReleasePolicyId);
        }
        if status_evidence_commitment == [0; 32] {
            return Err(EndorsementError::ZeroStatusEvidenceCommitment);
        }
        if issuance_nonce == [0; 32] {
            return Err(EndorsementError::ZeroNonce);
        }

        Ok(Self {
            schema_version: OBSERVATION_ENDORSEMENT_SCHEMA_V1,
            status_profile_id,
            issuer_did,
            grant_id,
            observation_id,
            release_policy_id,
            publisher_did,
            producer,
            checked_at_unix_s,
            status_evidence_commitment,
            issuance_nonce,
        })
    }

    pub fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub fn status_profile_id(&self) -> &CanonicalId {
        &self.status_profile_id
    }

    pub fn issuer_did(&self) -> &CanonicalId {
        &self.issuer_did
    }

    pub fn grant_id(&self) -> ProducerAuthorityGrantId {
        self.grant_id
    }

    pub fn observation_id(&self) -> ObservationId {
        self.observation_id
    }

    pub fn release_policy_id(&self) -> &[u8; 32] {
        &self.release_policy_id
    }

    pub fn publisher_did(&self) -> &CanonicalId {
        &self.publisher_did
    }

    pub fn producer(&self) -> &CanonicalId {
        &self.producer
    }

    pub fn checked_at_unix_s(&self) -> i64 {
        self.checked_at_unix_s
    }

    pub fn status_evidence_commitment(&self) -> &[u8; 32] {
        &self.status_evidence_commitment
    }

    pub fn issuance_nonce(&self) -> &[u8; 32] {
        &self.issuance_nonce
    }

    pub fn validate(&self) -> Result<(), EndorsementError> {
        if self.schema_version != OBSERVATION_ENDORSEMENT_SCHEMA_V1 {
            return Err(EndorsementError::UnsupportedSchema(self.schema_version));
        }
        if !self.issuer_did.as_str().starts_with("did:")
            || !self.publisher_did.as_str().starts_with("did:")
        {
            return Err(EndorsementError::InvalidDid);
        }
        if self.release_policy_id == [0; 32] {
            return Err(EndorsementError::ZeroReleasePolicyId);
        }
        if self.status_evidence_commitment == [0; 32] {
            return Err(EndorsementError::ZeroStatusEvidenceCommitment);
        }
        if self.issuance_nonce == [0; 32] {
            return Err(EndorsementError::ZeroNonce);
        }
        Ok(())
    }

    /// Immutable semantic identity for this exact positive endorsement.
    pub fn id(&self) -> Result<ObservationEndorsementId, EndorsementError> {
        self.validate()?;
        let payload = self.canonical_payload();
        let mut h = Sha256::new();
        h.update(OBSERVATION_ENDORSEMENT_ID_DOMAIN_V1);
        h.update(payload);
        Ok(ObservationEndorsementId(h.finalize().into()))
    }

    /// Exact bytes an external status authority must sign.
    pub fn signing_transcript(&self) -> Result<Vec<u8>, EndorsementError> {
        self.validate()?;
        let payload = self.canonical_payload();
        let mut out =
            Vec::with_capacity(OBSERVATION_ENDORSEMENT_SIGNING_DOMAIN_V1.len() + payload.len());
        out.extend_from_slice(OBSERVATION_ENDORSEMENT_SIGNING_DOMAIN_V1);
        out.extend_from_slice(&payload);
        Ok(out)
    }

    /// Check whether this endorsement semantically binds one exact grant,
    /// observation, policy, and publisher.
    ///
    /// This does not verify the detached cryptographic signature. A verifier must
    /// establish the issuer key/trust profile separately.
    pub fn assess_binding(
        &self,
        grant: &ProducerAuthorityGrant,
        observation: &SurveillanceObservation,
        release_policy_id: &[u8; 32],
        publisher_did: &CanonicalId,
    ) -> Result<EndorsementBindingAssessment, EndorsementError> {
        self.validate()?;
        grant
            .validate()
            .map_err(|e: AuthorityError| EndorsementError::InvalidGrant(e.to_string()))?;
        observation
            .validate()
            .map_err(|e| EndorsementError::InvalidObservation(e.to_string()))?;

        let grant_id = grant
            .id()
            .map_err(|e| EndorsementError::InvalidGrant(e.to_string()))?;
        let observation_id = observation
            .id()
            .map_err(|e| EndorsementError::InvalidObservation(e.to_string()))?;

        let mut reasons = Vec::new();
        if self.grant_id != grant_id {
            reasons.push(EndorsementBindingDenyReason::GrantIdMismatch);
        }
        if self.observation_id != observation_id {
            reasons.push(EndorsementBindingDenyReason::ObservationIdMismatch);
        }
        if &self.release_policy_id != release_policy_id {
            reasons.push(EndorsementBindingDenyReason::ReleasePolicyMismatch);
        }
        if &self.publisher_did != publisher_did {
            reasons.push(EndorsementBindingDenyReason::PublisherMismatch);
        }
        if self.issuer_did != *grant.issuer_did() {
            reasons.push(EndorsementBindingDenyReason::IssuerMismatch);
        }
        if self.producer != *grant.producer() || self.producer != observation.provenance.producer {
            reasons.push(EndorsementBindingDenyReason::ProducerMismatch);
        }
        if self.checked_at_unix_s < observation.reported_at_unix_s {
            reasons.push(EndorsementBindingDenyReason::StatusCheckedBeforeObservationReported);
        }

        Ok(EndorsementBindingAssessment {
            endorsement_id: self.id()?,
            grant_id,
            observation_id,
            binds_exactly: reasons.is_empty(),
            reasons,
        })
    }

    fn canonical_payload(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_u16(&mut out, self.schema_version);
        put_id(&mut out, &self.status_profile_id);
        put_id(&mut out, &self.issuer_did);
        out.extend_from_slice(self.grant_id.as_bytes());
        out.extend_from_slice(self.observation_id.as_bytes());
        out.extend_from_slice(&self.release_policy_id);
        put_id(&mut out, &self.publisher_did);
        put_id(&mut out, &self.producer);
        put_i64(&mut out, self.checked_at_unix_s);
        out.extend_from_slice(&self.status_evidence_commitment);
        out.extend_from_slice(&self.issuance_nonce);
        out
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EndorsementBindingAssessment {
    pub endorsement_id: ObservationEndorsementId,
    pub grant_id: ProducerAuthorityGrantId,
    pub observation_id: ObservationId,
    pub binds_exactly: bool,
    pub reasons: Vec<EndorsementBindingDenyReason>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EndorsementBindingDenyReason {
    GrantIdMismatch,
    ObservationIdMismatch,
    ReleasePolicyMismatch,
    PublisherMismatch,
    IssuerMismatch,
    ProducerMismatch,
    StatusCheckedBeforeObservationReported,
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn put_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn put_id(out: &mut Vec<u8>, value: &CanonicalId) {
    let bytes = value.as_str().as_bytes();
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use health_surveillance_authority::ProducerAuthorityScope;
    use health_surveillance_core::{
        BoundedUncertainty, EvidenceProvenance, GeographicPrecision, GeographicScope,
        IndependenceGroup, MetricKind, ObservationWindow, ObservedMetric, SignalFamily, SourceKind,
        SourceRecordDigest,
    };

    fn observation(independence: &str) -> SurveillanceObservation {
        SurveillanceObservation::new(
            SignalFamily::Respiratory,
            SourceKind::LaboratoryAggregate,
            "lab-feed-a",
            IndependenceGroup::new(independence).unwrap(),
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
                "rev-1",
                Some("upstream-lab-a"),
                SourceRecordDigest::sha256([7; 32]).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn grant() -> ProducerAuthorityGrant {
        ProducerAuthorityGrant::new(
            CanonicalId::new("public-health.za.gp").unwrap(),
            CanonicalId::new("mycelix:schema:surveillance-producer:v1").unwrap(),
            CanonicalId::new("did:mycelix:issuer-a").unwrap(),
            CanonicalId::new("did:mycelix:publisher-a").unwrap(),
            CanonicalId::new("lab-a").unwrap(),
            ProducerAuthorityScope::new(
                vec![SourceKind::LaboratoryAggregate],
                vec![SignalFamily::Respiratory],
                vec![CanonicalId::new("lab-feed-a").unwrap()],
                vec![CanonicalId::new("aggregate-protocol-v1").unwrap()],
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
            [1; 32],
        )
        .unwrap()
    }

    fn endorsement(obs: &SurveillanceObservation, policy: [u8; 32]) -> AuthorizedObservationEndorsement {
        let grant = grant();
        AuthorizedObservationEndorsement::new(
            CanonicalId::new("mycelix-vc-active-status-v1").unwrap(),
            grant.issuer_did().clone(),
            grant.id().unwrap(),
            obs.id().unwrap(),
            policy,
            CanonicalId::new("did:mycelix:publisher-a").unwrap(),
            CanonicalId::new("lab-a").unwrap(),
            13_701,
            [9; 32],
            [8; 32],
        )
        .unwrap()
    }

    #[test]
    fn exact_observation_grant_policy_and_publisher_bind() {
        let obs = observation("lineage-a");
        let grant = grant();
        let policy = [3; 32];
        let endorsement = endorsement(&obs, policy);
        let publisher = CanonicalId::new("did:mycelix:publisher-a").unwrap();
        let assessment = endorsement
            .assess_binding(&grant, &obs, &policy, &publisher)
            .unwrap();
        assert!(assessment.binds_exactly);
        assert!(assessment.reasons.is_empty());
    }

    #[test]
    fn observation_substitution_is_rejected() {
        let original = observation("lineage-a");
        let substituted = observation("lineage-b");
        let grant = grant();
        let policy = [3; 32];
        let endorsement = endorsement(&original, policy);
        let publisher = CanonicalId::new("did:mycelix:publisher-a").unwrap();
        let assessment = endorsement
            .assess_binding(&grant, &substituted, &policy, &publisher)
            .unwrap();
        assert!(!assessment.binds_exactly);
        assert!(assessment
            .reasons
            .contains(&EndorsementBindingDenyReason::ObservationIdMismatch));
    }

    #[test]
    fn release_policy_substitution_is_rejected() {
        let obs = observation("lineage-a");
        let grant = grant();
        let endorsement = endorsement(&obs, [3; 32]);
        let publisher = CanonicalId::new("did:mycelix:publisher-a").unwrap();
        let assessment = endorsement
            .assess_binding(&grant, &obs, &[4; 32], &publisher)
            .unwrap();
        assert!(!assessment.binds_exactly);
        assert!(assessment
            .reasons
            .contains(&EndorsementBindingDenyReason::ReleasePolicyMismatch));
    }

    #[test]
    fn publisher_substitution_is_rejected() {
        let obs = observation("lineage-a");
        let grant = grant();
        let policy = [3; 32];
        let endorsement = endorsement(&obs, policy);
        let other = CanonicalId::new("did:mycelix:publisher-b").unwrap();
        let assessment = endorsement
            .assess_binding(&grant, &obs, &policy, &other)
            .unwrap();
        assert!(!assessment.binds_exactly);
        assert!(assessment
            .reasons
            .contains(&EndorsementBindingDenyReason::PublisherMismatch));
    }

    #[test]
    fn status_check_cannot_claim_to_precede_the_observation_report() {
        let obs = observation("lineage-a");
        let grant = grant();
        let policy = [3; 32];
        let endorsement = AuthorizedObservationEndorsement::new(
            CanonicalId::new("mycelix-vc-active-status-v1").unwrap(),
            grant.issuer_did().clone(),
            grant.id().unwrap(),
            obs.id().unwrap(),
            policy,
            CanonicalId::new("did:mycelix:publisher-a").unwrap(),
            CanonicalId::new("lab-a").unwrap(),
            13_699,
            [9; 32],
            [8; 32],
        )
        .unwrap();
        let publisher = CanonicalId::new("did:mycelix:publisher-a").unwrap();
        let assessment = endorsement
            .assess_binding(&grant, &obs, &policy, &publisher)
            .unwrap();
        assert!(!assessment.binds_exactly);
        assert!(assessment.reasons.contains(
            &EndorsementBindingDenyReason::StatusCheckedBeforeObservationReported
        ));
    }

    #[test]
    fn zero_commitments_and_nonces_fail_closed() {
        let obs = observation("lineage-a");
        let grant = grant();
        let common = || {
            (
                CanonicalId::new("mycelix-vc-active-status-v1").unwrap(),
                grant.issuer_did().clone(),
                grant.id().unwrap(),
                obs.id().unwrap(),
                [3; 32],
                CanonicalId::new("did:mycelix:publisher-a").unwrap(),
                CanonicalId::new("lab-a").unwrap(),
            )
        };
        let (profile, issuer, grant_id, observation_id, policy, publisher, producer) = common();
        assert_eq!(
            AuthorizedObservationEndorsement::new(
                profile.clone(), issuer.clone(), grant_id, observation_id, policy, publisher.clone(),
                producer.clone(), 13_701, [0; 32], [8; 32],
            ),
            Err(EndorsementError::ZeroStatusEvidenceCommitment)
        );
        assert_eq!(
            AuthorizedObservationEndorsement::new(
                profile, issuer, grant_id, observation_id, policy, publisher, producer, 13_701,
                [9; 32], [0; 32],
            ),
            Err(EndorsementError::ZeroNonce)
        );
    }
}
