#![deny(unsafe_code)]
//! Canonical producer-authority contracts for aggregate public-health surveillance.
//!
//! This crate deliberately contains **no cryptographic implementation** and no
//! network I/O. It defines the exact semantics and domain-separated transcript
//! that an Identity/Xenia adapter may sign and a surveillance verifier may
//! verify. Keeping that contract in one place prevents issuer/verifier drift.
//!
//! A [`ProducerAuthorityGrant`] is only a claimed authorization payload until an
//! external verifier establishes its cryptographic proof and issuer trust. The
//! [`ClaimedScopeAssessment`] likewise answers only whether an observation fits
//! the payload's declared scope; it never upgrades an unsigned/unverified grant
//! into trusted authority.

use std::fmt;

use health_surveillance_core::{
    CanonicalId, GeographicPrecision, GeographicScope, ObservationId, SignalFamily, SourceKind,
    SurveillanceObservation,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const PRODUCER_AUTHORITY_SCHEMA_V1: u16 = 1;
pub const MAX_SCOPE_VALUES: usize = 64;
pub const PRODUCER_AUTHORITY_ID_DOMAIN_V1: &[u8] =
    b"mycelix-health-surveillance-producer-authority-id-v1\0";
pub const PRODUCER_AUTHORITY_SIGNING_DOMAIN_V1: &[u8] =
    b"mycelix-health-surveillance-producer-authority-signing-v1\0";

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum AuthorityError {
    #[error("authority schema version {0} is unsupported")]
    UnsupportedSchema(u16),
    #[error("issuer and subject identities must be DID identifiers")]
    InvalidDid,
    #[error("authority validity requires valid_until_unix_s > valid_from_unix_s")]
    InvalidValidityWindow,
    #[error("authority issuance nonce must not be all zeroes")]
    ZeroNonce,
    #[error("scope collection '{0}' must contain at least one value")]
    EmptyScope(&'static str),
    #[error("scope collection '{0}' exceeds the direct-value bound")]
    ScopeTooLarge(&'static str),
    #[error("scope collection '{0}' contains a duplicate value")]
    DuplicateScopeValue(&'static str),
    #[error("invalid surveillance observation: {0}")]
    InvalidObservation(String),
    #[error("invalid canonical identifier: {0}")]
    InvalidIdentifier(String),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ProducerAuthorityGrantId([u8; 32]);

impl ProducerAuthorityGrantId {
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

impl fmt::Display for ProducerAuthorityGrantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProducerAuthorityScope {
    allowed_source_kinds: Vec<SourceKind>,
    allowed_signal_families: Vec<SignalFamily>,
    allowed_source_instances: Vec<CanonicalId>,
    allowed_acquisition_protocols: Vec<CanonicalId>,
    allowed_geographies: Vec<GeographicScope>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProducerAuthorityScopeWire {
    allowed_source_kinds: Vec<SourceKind>,
    allowed_signal_families: Vec<SignalFamily>,
    allowed_source_instances: Vec<CanonicalId>,
    allowed_acquisition_protocols: Vec<CanonicalId>,
    allowed_geographies: Vec<GeographicScope>,
}

impl<'de> Deserialize<'de> for ProducerAuthorityScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ProducerAuthorityScopeWire::deserialize(deserializer)?;
        Self::new(
            wire.allowed_source_kinds,
            wire.allowed_signal_families,
            wire.allowed_source_instances,
            wire.allowed_acquisition_protocols,
            wire.allowed_geographies,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl ProducerAuthorityScope {
    pub fn new(
        mut allowed_source_kinds: Vec<SourceKind>,
        mut allowed_signal_families: Vec<SignalFamily>,
        mut allowed_source_instances: Vec<CanonicalId>,
        mut allowed_acquisition_protocols: Vec<CanonicalId>,
        mut allowed_geographies: Vec<GeographicScope>,
    ) -> Result<Self, AuthorityError> {
        validate_inbound_len("source_kinds", allowed_source_kinds.len())?;
        validate_inbound_len("signal_families", allowed_signal_families.len())?;
        validate_inbound_len("source_instances", allowed_source_instances.len())?;
        validate_inbound_len(
            "acquisition_protocols",
            allowed_acquisition_protocols.len(),
        )?;
        validate_inbound_len("geographies", allowed_geographies.len())?;

        allowed_source_kinds.sort_by_key(source_kind_key);
        reject_adjacent_duplicates("source_kinds", &allowed_source_kinds)?;

        allowed_signal_families.sort_by_key(signal_family_key);
        reject_adjacent_duplicates("signal_families", &allowed_signal_families)?;

        allowed_source_instances.sort();
        reject_adjacent_duplicates("source_instances", &allowed_source_instances)?;

        allowed_acquisition_protocols.sort();
        reject_adjacent_duplicates(
            "acquisition_protocols",
            &allowed_acquisition_protocols,
        )?;

        allowed_geographies.sort_by_key(geography_key);
        reject_adjacent_duplicates("geographies", &allowed_geographies)?;

        Ok(Self {
            allowed_source_kinds,
            allowed_signal_families,
            allowed_source_instances,
            allowed_acquisition_protocols,
            allowed_geographies,
        })
    }

    pub fn allowed_source_kinds(&self) -> &[SourceKind] {
        &self.allowed_source_kinds
    }

    pub fn allowed_signal_families(&self) -> &[SignalFamily] {
        &self.allowed_signal_families
    }

    pub fn allowed_source_instances(&self) -> &[CanonicalId] {
        &self.allowed_source_instances
    }

    pub fn allowed_acquisition_protocols(&self) -> &[CanonicalId] {
        &self.allowed_acquisition_protocols
    }

    pub fn allowed_geographies(&self) -> &[GeographicScope] {
        &self.allowed_geographies
    }

    pub fn validate(&self) -> Result<(), AuthorityError> {
        Self::new(
            self.allowed_source_kinds.clone(),
            self.allowed_signal_families.clone(),
            self.allowed_source_instances.clone(),
            self.allowed_acquisition_protocols.clone(),
            self.allowed_geographies.clone(),
        )
        .map(|_| ())
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProducerAuthorityGrant {
    schema_version: u16,
    /// Security/trust domain in which the issuer identity is meaningful.
    security_domain: CanonicalId,
    /// Credential schema whose semantics an adapter claims this grant implements.
    credential_schema_id: CanonicalId,
    /// DID of the authority issuer.
    issuer_did: CanonicalId,
    /// DID of the subject allowed to present/use this authority.
    subject_did: CanonicalId,
    /// Producer identity that must exactly match observation provenance.
    producer: CanonicalId,
    scope: ProducerAuthorityScope,
    valid_from_unix_s: i64,
    valid_until_unix_s: i64,
    issuance_nonce: [u8; 32],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProducerAuthorityGrantWire {
    schema_version: u16,
    security_domain: CanonicalId,
    credential_schema_id: CanonicalId,
    issuer_did: CanonicalId,
    subject_did: CanonicalId,
    producer: CanonicalId,
    scope: ProducerAuthorityScope,
    valid_from_unix_s: i64,
    valid_until_unix_s: i64,
    issuance_nonce: [u8; 32],
}

impl<'de> Deserialize<'de> for ProducerAuthorityGrant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ProducerAuthorityGrantWire::deserialize(deserializer)?;
        if wire.schema_version != PRODUCER_AUTHORITY_SCHEMA_V1 {
            return Err(serde::de::Error::custom(AuthorityError::UnsupportedSchema(
                wire.schema_version,
            )));
        }
        Self::new(
            wire.security_domain,
            wire.credential_schema_id,
            wire.issuer_did,
            wire.subject_did,
            wire.producer,
            wire.scope,
            wire.valid_from_unix_s,
            wire.valid_until_unix_s,
            wire.issuance_nonce,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl ProducerAuthorityGrant {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        security_domain: CanonicalId,
        credential_schema_id: CanonicalId,
        issuer_did: CanonicalId,
        subject_did: CanonicalId,
        producer: CanonicalId,
        scope: ProducerAuthorityScope,
        valid_from_unix_s: i64,
        valid_until_unix_s: i64,
        issuance_nonce: [u8; 32],
    ) -> Result<Self, AuthorityError> {
        if !issuer_did.as_str().starts_with("did:") || !subject_did.as_str().starts_with("did:") {
            return Err(AuthorityError::InvalidDid);
        }
        if valid_until_unix_s <= valid_from_unix_s {
            return Err(AuthorityError::InvalidValidityWindow);
        }
        if issuance_nonce == [0; 32] {
            return Err(AuthorityError::ZeroNonce);
        }
        scope.validate()?;

        Ok(Self {
            schema_version: PRODUCER_AUTHORITY_SCHEMA_V1,
            security_domain,
            credential_schema_id,
            issuer_did,
            subject_did,
            producer,
            scope,
            valid_from_unix_s,
            valid_until_unix_s,
            issuance_nonce,
        })
    }

    pub fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub fn security_domain(&self) -> &CanonicalId {
        &self.security_domain
    }

    pub fn credential_schema_id(&self) -> &CanonicalId {
        &self.credential_schema_id
    }

    pub fn issuer_did(&self) -> &CanonicalId {
        &self.issuer_did
    }

    pub fn subject_did(&self) -> &CanonicalId {
        &self.subject_did
    }

    pub fn producer(&self) -> &CanonicalId {
        &self.producer
    }

    pub fn scope(&self) -> &ProducerAuthorityScope {
        &self.scope
    }

    pub fn valid_from_unix_s(&self) -> i64 {
        self.valid_from_unix_s
    }

    pub fn valid_until_unix_s(&self) -> i64 {
        self.valid_until_unix_s
    }

    pub fn issuance_nonce(&self) -> &[u8; 32] {
        &self.issuance_nonce
    }

    pub fn validate(&self) -> Result<(), AuthorityError> {
        if self.schema_version != PRODUCER_AUTHORITY_SCHEMA_V1 {
            return Err(AuthorityError::UnsupportedSchema(self.schema_version));
        }
        if !self.issuer_did.as_str().starts_with("did:")
            || !self.subject_did.as_str().starts_with("did:")
        {
            return Err(AuthorityError::InvalidDid);
        }
        if self.valid_until_unix_s <= self.valid_from_unix_s {
            return Err(AuthorityError::InvalidValidityWindow);
        }
        if self.issuance_nonce == [0; 32] {
            return Err(AuthorityError::ZeroNonce);
        }
        self.scope.validate()
    }

    /// Immutable semantic identity of this exact claimed authority payload.
    pub fn id(&self) -> Result<ProducerAuthorityGrantId, AuthorityError> {
        self.validate()?;
        let payload = self.canonical_payload();
        let mut h = Sha256::new();
        h.update(PRODUCER_AUTHORITY_ID_DOMAIN_V1);
        h.update(payload);
        Ok(ProducerAuthorityGrantId(h.finalize().into()))
    }

    /// Exact bytes an external credential/signature adapter must authenticate.
    ///
    /// This transcript is stable, domain-separated, and independent of serde
    /// formatting. A signature over JSON bytes is not interchangeable with a
    /// signature over this contract.
    pub fn signing_transcript(&self) -> Result<Vec<u8>, AuthorityError> {
        self.validate()?;
        let payload = self.canonical_payload();
        let mut out = Vec::with_capacity(PRODUCER_AUTHORITY_SIGNING_DOMAIN_V1.len() + payload.len());
        out.extend_from_slice(PRODUCER_AUTHORITY_SIGNING_DOMAIN_V1);
        out.extend_from_slice(&payload);
        Ok(out)
    }

    /// Evaluate only whether the observation lies inside the **claimed** scope.
    ///
    /// This does not verify any signature, credential issuer, trust root,
    /// revocation status, or institutional identity.
    pub fn assess_claimed_scope(
        &self,
        observation: &SurveillanceObservation,
        evaluated_at_unix_s: i64,
    ) -> Result<ClaimedScopeAssessment, AuthorityError> {
        self.validate()?;
        observation
            .validate()
            .map_err(|e| AuthorityError::InvalidObservation(e.to_string()))?;

        let mut reasons = Vec::new();
        if evaluated_at_unix_s < self.valid_from_unix_s {
            reasons.push(ClaimedScopeDenyReason::GrantNotYetValid);
        }
        if evaluated_at_unix_s >= self.valid_until_unix_s {
            reasons.push(ClaimedScopeDenyReason::GrantExpired);
        }
        if observation.window.start_unix_s < self.valid_from_unix_s
            || observation.reported_at_unix_s >= self.valid_until_unix_s
        {
            reasons.push(ClaimedScopeDenyReason::ObservationTimeOutsideGrant);
        }
        if observation.provenance.producer != self.producer {
            reasons.push(ClaimedScopeDenyReason::ProducerMismatch);
        }
        if !self.scope.allowed_source_kinds.contains(&observation.source_kind) {
            reasons.push(ClaimedScopeDenyReason::SourceKindNotAllowed);
        }
        if !self
            .scope
            .allowed_signal_families
            .contains(&observation.signal)
        {
            reasons.push(ClaimedScopeDenyReason::SignalFamilyNotAllowed);
        }
        if !self
            .scope
            .allowed_source_instances
            .contains(&observation.source_instance)
        {
            reasons.push(ClaimedScopeDenyReason::SourceInstanceNotAllowed);
        }
        if !self
            .scope
            .allowed_acquisition_protocols
            .contains(&observation.provenance.acquisition_protocol)
        {
            reasons.push(ClaimedScopeDenyReason::AcquisitionProtocolNotAllowed);
        }
        if !self.scope.allowed_geographies.contains(&observation.geography) {
            reasons.push(ClaimedScopeDenyReason::GeographyNotAllowed);
        }

        Ok(ClaimedScopeAssessment {
            grant_id: self.id()?,
            observation_id: observation
                .id()
                .map_err(|e| AuthorityError::InvalidObservation(e.to_string()))?,
            evaluated_at_unix_s,
            permitted_by_claimed_scope: reasons.is_empty(),
            reasons,
        })
    }

    fn canonical_payload(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_u16(&mut out, self.schema_version);
        put_id(&mut out, &self.security_domain);
        put_id(&mut out, &self.credential_schema_id);
        put_id(&mut out, &self.issuer_did);
        put_id(&mut out, &self.subject_did);
        put_id(&mut out, &self.producer);
        put_i64(&mut out, self.valid_from_unix_s);
        put_i64(&mut out, self.valid_until_unix_s);
        out.extend_from_slice(&self.issuance_nonce);

        put_u32(&mut out, self.scope.allowed_source_kinds.len() as u32);
        for value in &self.scope.allowed_source_kinds {
            put_source_kind(&mut out, value);
        }
        put_u32(&mut out, self.scope.allowed_signal_families.len() as u32);
        for value in &self.scope.allowed_signal_families {
            put_signal_family(&mut out, value);
        }
        put_u32(&mut out, self.scope.allowed_source_instances.len() as u32);
        for value in &self.scope.allowed_source_instances {
            put_id(&mut out, value);
        }
        put_u32(
            &mut out,
            self.scope.allowed_acquisition_protocols.len() as u32,
        );
        for value in &self.scope.allowed_acquisition_protocols {
            put_id(&mut out, value);
        }
        put_u32(&mut out, self.scope.allowed_geographies.len() as u32);
        for value in &self.scope.allowed_geographies {
            put_geography(&mut out, value);
        }
        out
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimedScopeDenyReason {
    GrantNotYetValid,
    GrantExpired,
    ObservationTimeOutsideGrant,
    ProducerMismatch,
    SourceKindNotAllowed,
    SignalFamilyNotAllowed,
    SourceInstanceNotAllowed,
    AcquisitionProtocolNotAllowed,
    GeographyNotAllowed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClaimedScopeAssessment {
    pub grant_id: ProducerAuthorityGrantId,
    pub observation_id: ObservationId,
    pub evaluated_at_unix_s: i64,
    pub permitted_by_claimed_scope: bool,
    pub reasons: Vec<ClaimedScopeDenyReason>,
}

fn validate_inbound_len(name: &'static str, len: usize) -> Result<(), AuthorityError> {
    if len == 0 {
        return Err(AuthorityError::EmptyScope(name));
    }
    if len > MAX_SCOPE_VALUES {
        return Err(AuthorityError::ScopeTooLarge(name));
    }
    Ok(())
}

fn reject_adjacent_duplicates<T: PartialEq>(
    name: &'static str,
    values: &[T],
) -> Result<(), AuthorityError> {
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(AuthorityError::DuplicateScopeValue(name));
    }
    Ok(())
}

fn source_kind_key(value: &SourceKind) -> String {
    match value {
        SourceKind::ClinicalSyndromicAggregate => "00".to_string(),
        SourceKind::LaboratoryAggregate => "01".to_string(),
        SourceKind::WastewaterAggregate => "02".to_string(),
        SourceKind::EnvironmentalAggregate => "03".to_string(),
        SourceKind::AbsenteeismAggregate => "04".to_string(),
        SourceKind::HealthSystemCapacityAggregate => "05".to_string(),
        SourceKind::Other(id) => format!("ff:{}", id.as_str()),
    }
}

fn signal_family_key(value: &SignalFamily) -> String {
    match value {
        SignalFamily::Respiratory => "00".to_string(),
        SignalFamily::Gastrointestinal => "01".to_string(),
        SignalFamily::Febrile => "02".to_string(),
        SignalFamily::Neurological => "03".to_string(),
        SignalFamily::Dermatologic => "04".to_string(),
        SignalFamily::Other(id) => format!("ff:{}", id.as_str()),
    }
}

fn geography_key(value: &GeographicScope) -> String {
    format!(
        "{}:{}:{}",
        geography_precision_tag(value.precision),
        value.scheme.as_str(),
        value.code.as_str()
    )
}

fn geography_precision_tag(value: GeographicPrecision) -> u8 {
    match value {
        GeographicPrecision::Country => 0,
        GeographicPrecision::Region => 1,
        GeographicPrecision::District => 2,
        GeographicPrecision::Facility => 3,
    }
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn put_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn put_id(out: &mut Vec<u8>, value: &CanonicalId) {
    let bytes = value.as_str().as_bytes();
    put_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

fn put_source_kind(out: &mut Vec<u8>, value: &SourceKind) {
    match value {
        SourceKind::ClinicalSyndromicAggregate => out.push(0),
        SourceKind::LaboratoryAggregate => out.push(1),
        SourceKind::WastewaterAggregate => out.push(2),
        SourceKind::EnvironmentalAggregate => out.push(3),
        SourceKind::AbsenteeismAggregate => out.push(4),
        SourceKind::HealthSystemCapacityAggregate => out.push(5),
        SourceKind::Other(id) => {
            out.push(255);
            put_id(out, id);
        }
    }
}

fn put_signal_family(out: &mut Vec<u8>, value: &SignalFamily) {
    match value {
        SignalFamily::Respiratory => out.push(0),
        SignalFamily::Gastrointestinal => out.push(1),
        SignalFamily::Febrile => out.push(2),
        SignalFamily::Neurological => out.push(3),
        SignalFamily::Dermatologic => out.push(4),
        SignalFamily::Other(id) => {
            out.push(255);
            put_id(out, id);
        }
    }
}

fn put_geography(out: &mut Vec<u8>, value: &GeographicScope) {
    put_id(out, &value.scheme);
    put_id(out, &value.code);
    out.push(geography_precision_tag(value.precision));
}

#[cfg(test)]
mod tests {
    use super::*;
    use health_surveillance_core::{
        BoundedUncertainty, EvidenceProvenance, IndependenceGroup, MetricKind, ObservationWindow,
        ObservedMetric, SourceRecordDigest,
    };

    fn id(value: &str) -> CanonicalId {
        CanonicalId::new(value).unwrap()
    }

    fn geography(code: &str) -> GeographicScope {
        GeographicScope::new("health-district", code, GeographicPrecision::District).unwrap()
    }

    fn scope(source_kinds: Vec<SourceKind>) -> ProducerAuthorityScope {
        ProducerAuthorityScope::new(
            source_kinds,
            vec![SignalFamily::Respiratory],
            vec![id("lab-feed-a")],
            vec![id("aggregate-protocol-v1")],
            vec![geography("district-17")],
        )
        .unwrap()
    }

    fn grant(scope: ProducerAuthorityScope) -> ProducerAuthorityGrant {
        ProducerAuthorityGrant::new(
            id("identity-domain:public-health-v1"),
            id("mycelix:schema:health:surveillance-publisher:v1"),
            id("did:mycelix:issuer-a"),
            id("did:mycelix:publisher-a"),
            id("lab-a"),
            scope,
            1_000,
            10_000,
            [7; 32],
        )
        .unwrap()
    }

    fn observation(independence: &str) -> SurveillanceObservation {
        SurveillanceObservation::new(
            SignalFamily::Respiratory,
            SourceKind::LaboratoryAggregate,
            "lab-feed-a",
            IndependenceGroup::new(independence).unwrap(),
            geography("district-17"),
            ObservationWindow::new(2_000, 3_000).unwrap(),
            3_100,
            100,
            ObservedMetric::new(
                MetricKind::FractionPositive,
                0.2,
                BoundedUncertainty::new(0.1, 0.3).unwrap(),
                "fraction",
            )
            .unwrap(),
            EvidenceProvenance::new(
                "lab-a",
                "aggregate-protocol-v1",
                "rev-1",
                Some("upstream-a"),
                SourceRecordDigest::sha256([9; 32]).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn set_order_does_not_change_grant_identity_or_signing_transcript() {
        let a = grant(scope(vec![
            SourceKind::WastewaterAggregate,
            SourceKind::LaboratoryAggregate,
        ]));
        let b = grant(scope(vec![
            SourceKind::LaboratoryAggregate,
            SourceKind::WastewaterAggregate,
        ]));
        assert_eq!(a.id().unwrap(), b.id().unwrap());
        assert_eq!(a.signing_transcript().unwrap(), b.signing_transcript().unwrap());
    }

    #[test]
    fn duplicate_scope_values_are_rejected_instead_of_silently_inflating_scope() {
        let result = ProducerAuthorityScope::new(
            vec![
                SourceKind::LaboratoryAggregate,
                SourceKind::LaboratoryAggregate,
            ],
            vec![SignalFamily::Respiratory],
            vec![id("lab-feed-a")],
            vec![id("aggregate-protocol-v1")],
            vec![geography("district-17")],
        );
        assert_eq!(result, Err(AuthorityError::DuplicateScopeValue("source_kinds")));
    }

    #[test]
    fn security_domain_is_identity_significant() {
        let a = grant(scope(vec![SourceKind::LaboratoryAggregate]));
        let b = ProducerAuthorityGrant::new(
            id("identity-domain:other"),
            a.credential_schema_id().clone(),
            a.issuer_did().clone(),
            a.subject_did().clone(),
            a.producer().clone(),
            a.scope().clone(),
            a.valid_from_unix_s(),
            a.valid_until_unix_s(),
            *a.issuance_nonce(),
        )
        .unwrap();
        assert_ne!(a.id().unwrap(), b.id().unwrap());
    }

    #[test]
    fn matching_observation_is_inside_claimed_scope() {
        let grant = grant(scope(vec![SourceKind::LaboratoryAggregate]));
        let assessment = grant.assess_claimed_scope(&observation("lineage-a"), 4_000).unwrap();
        assert!(assessment.permitted_by_claimed_scope);
        assert!(assessment.reasons.is_empty());
    }

    #[test]
    fn mismatched_producer_protocol_and_geography_fail_closed() {
        let grant = grant(scope(vec![SourceKind::LaboratoryAggregate]));
        let mut obs = observation("lineage-a");
        obs.provenance.producer = id("other-lab");
        obs.provenance.acquisition_protocol = id("other-protocol");
        obs.geography = geography("district-18");

        let assessment = grant.assess_claimed_scope(&obs, 4_000).unwrap();
        assert!(!assessment.permitted_by_claimed_scope);
        assert!(assessment.reasons.contains(&ClaimedScopeDenyReason::ProducerMismatch));
        assert!(assessment
            .reasons
            .contains(&ClaimedScopeDenyReason::AcquisitionProtocolNotAllowed));
        assert!(assessment
            .reasons
            .contains(&ClaimedScopeDenyReason::GeographyNotAllowed));
    }

    #[test]
    fn expired_grant_or_observation_outside_grant_time_is_rejected() {
        let grant = grant(scope(vec![SourceKind::LaboratoryAggregate]));
        let at_expiry = grant.assess_claimed_scope(&observation("lineage-a"), 10_000).unwrap();
        assert!(at_expiry.reasons.contains(&ClaimedScopeDenyReason::GrantExpired));

        let mut late = observation("lineage-a");
        late.window = ObservationWindow::new(9_500, 9_900).unwrap();
        late.reported_at_unix_s = 10_000;
        let late_assessment = grant.assess_claimed_scope(&late, 9_999).unwrap();
        assert!(late_assessment
            .reasons
            .contains(&ClaimedScopeDenyReason::ObservationTimeOutsideGrant));
    }

    #[test]
    fn independence_group_is_deliberately_not_authenticated_by_producer_scope() {
        let grant = grant(scope(vec![SourceKind::LaboratoryAggregate]));
        let a = grant.assess_claimed_scope(&observation("claimed-lineage-a"), 4_000).unwrap();
        let b = grant.assess_claimed_scope(&observation("claimed-lineage-b"), 4_000).unwrap();
        assert!(a.permitted_by_claimed_scope);
        assert!(b.permitted_by_claimed_scope);
        assert_ne!(a.observation_id, b.observation_id);
    }

    #[test]
    fn zero_nonce_and_unknown_wire_fields_fail() {
        let zero_nonce = ProducerAuthorityGrant::new(
            id("identity-domain:public-health-v1"),
            id("mycelix:schema:health:surveillance-publisher:v1"),
            id("did:mycelix:issuer-a"),
            id("did:mycelix:publisher-a"),
            id("lab-a"),
            scope(vec![SourceKind::LaboratoryAggregate]),
            1_000,
            10_000,
            [0; 32],
        );
        assert_eq!(zero_nonce, Err(AuthorityError::ZeroNonce));

        let grant = grant(scope(vec![SourceKind::LaboratoryAggregate]));
        let mut value = serde_json::to_value(&grant).unwrap();
        value.as_object_mut().unwrap().insert("future_field".into(), serde_json::json!(1));
        assert!(serde_json::from_value::<ProducerAuthorityGrant>(value).is_err());
    }
}
