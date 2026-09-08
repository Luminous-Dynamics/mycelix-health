#![deny(unsafe_code)]
//! Canonical dimension-specific evidence-lineage attestations.
//!
//! This crate deliberately does **not** define a global `is_independent` boolean.
//! Independence is contextual and can be overclaimed easily. Instead, a trusted
//! lineage adapter may attest known lineage identifiers on separate dimensions
//! such as upstream data, sampling frame, measurement system, processing
//! pipeline, and operator/control domain. Missing knowledge is represented
//! explicitly as `Unknown` rather than as an empty set.
//!
//! The crate is crypto-free and I/O-free. It defines stable content identities,
//! signing transcripts, exact observation binding, and conservative pairwise
//! comparison semantics. Cryptographic trust policy belongs in a later adapter/
//! integrity tranche.

use std::collections::BTreeSet;
use std::fmt;

use health_surveillance_core::{CanonicalId, ObservationId, SurveillanceObservation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const LINEAGE_ATTESTATION_SCHEMA_V1: u16 = 1;
pub const MAX_LINEAGE_IDS_PER_DIMENSION: usize = 64;
pub const LINEAGE_ATTESTATION_ID_DOMAIN_V1: &[u8] =
    b"mycelix-health-surveillance-lineage-attestation-id-v1\0";
pub const LINEAGE_ATTESTATION_SIGNING_DOMAIN_V1: &[u8] =
    b"mycelix-health-surveillance-lineage-attestation-signing-v1\0";

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum LineageError {
    #[error("lineage attestation schema version {0} is unsupported")]
    UnsupportedSchema(u16),
    #[error("lineage attestor must use a DID identifier")]
    InvalidDid,
    #[error("known lineage dimension must contain at least one identifier")]
    EmptyKnownDimension,
    #[error("lineage dimension exceeds the maximum direct identifier count")]
    DimensionTooLarge,
    #[error("lineage descriptor must contain at least one known dimension")]
    NoKnownLineageFacts,
    #[error("lineage evidence commitment must not be all zeroes")]
    ZeroEvidenceCommitment,
    #[error("lineage attestation nonce must not be all zeroes")]
    ZeroNonce,
    #[error("invalid surveillance observation: {0}")]
    InvalidObservation(String),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct LineageAttestationId([u8; 32]);

impl LineageAttestationId {
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

impl fmt::Display for LineageAttestationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// Explicit knowledge state for one lineage dimension.
///
/// `Unknown` is not equivalent to `Known(empty)`. Known sets are non-empty and
/// bounded. BTreeSet makes semantic ordering canonical and prevents duplicate
/// aliases.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LineageKnowledge {
    Unknown,
    Known(BTreeSet<CanonicalId>),
}

impl LineageKnowledge {
    pub fn unknown() -> Self {
        Self::Unknown
    }

    pub fn known(values: impl IntoIterator<Item = CanonicalId>) -> Result<Self, LineageError> {
        let values: BTreeSet<CanonicalId> = values.into_iter().collect();
        let knowledge = Self::Known(values);
        knowledge.validate()?;
        Ok(knowledge)
    }

    pub fn validate(&self) -> Result<(), LineageError> {
        match self {
            Self::Unknown => Ok(()),
            Self::Known(values) => {
                if values.is_empty() {
                    return Err(LineageError::EmptyKnownDimension);
                }
                if values.len() > MAX_LINEAGE_IDS_PER_DIMENSION {
                    return Err(LineageError::DimensionTooLarge);
                }
                Ok(())
            }
        }
    }

    pub fn is_known(&self) -> bool {
        matches!(self, Self::Known(_))
    }

    pub fn values(&self) -> Option<&BTreeSet<CanonicalId>> {
        match self {
            Self::Unknown => None,
            Self::Known(values) => Some(values),
        }
    }
}

/// Dimension-specific lineage facts for one exact observation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LineageDescriptor {
    /// Root datasets/feeds from which this evidence ultimately derives.
    pub upstream_roots: LineageKnowledge,
    /// Sampling populations/frames; shared frames can create correlated evidence.
    pub sampling_frames: LineageKnowledge,
    /// Collection/sensor/site systems that produced primary observations.
    pub collection_systems: LineageKnowledge,
    /// Measurement/instrument/laboratory systems.
    pub measurement_systems: LineageKnowledge,
    /// Transformation/software/analysis pipelines applied before publication.
    pub processing_pipelines: LineageKnowledge,
    /// Organization/control domains capable of coordinating or biasing sources.
    pub operator_control_domains: LineageKnowledge,
}

impl LineageDescriptor {
    pub fn new(
        upstream_roots: LineageKnowledge,
        sampling_frames: LineageKnowledge,
        collection_systems: LineageKnowledge,
        measurement_systems: LineageKnowledge,
        processing_pipelines: LineageKnowledge,
        operator_control_domains: LineageKnowledge,
    ) -> Result<Self, LineageError> {
        let descriptor = Self {
            upstream_roots,
            sampling_frames,
            collection_systems,
            measurement_systems,
            processing_pipelines,
            operator_control_domains,
        };
        descriptor.validate()?;
        Ok(descriptor)
    }

    pub fn validate(&self) -> Result<(), LineageError> {
        for knowledge in [
            &self.upstream_roots,
            &self.sampling_frames,
            &self.collection_systems,
            &self.measurement_systems,
            &self.processing_pipelines,
            &self.operator_control_domains,
        ] {
            knowledge.validate()?;
        }
        if ![
            &self.upstream_roots,
            &self.sampling_frames,
            &self.collection_systems,
            &self.measurement_systems,
            &self.processing_pipelines,
            &self.operator_control_domains,
        ]
        .iter()
        .any(|knowledge| knowledge.is_known())
        {
            return Err(LineageError::NoKnownLineageFacts);
        }
        Ok(())
    }
}

/// Signed semantic payload for an exact observation's known lineage facts.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct EvidenceLineageAttestation {
    schema_version: u16,
    security_domain: CanonicalId,
    attestation_profile_id: CanonicalId,
    attestor_did: CanonicalId,
    observation_id: ObservationId,
    descriptor: LineageDescriptor,
    assessed_at_unix_s: i64,
    evidence_commitment: [u8; 32],
    issuance_nonce: [u8; 32],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceLineageAttestationWire {
    schema_version: u16,
    security_domain: CanonicalId,
    attestation_profile_id: CanonicalId,
    attestor_did: CanonicalId,
    observation_id: ObservationId,
    descriptor: LineageDescriptor,
    assessed_at_unix_s: i64,
    evidence_commitment: [u8; 32],
    issuance_nonce: [u8; 32],
}

impl<'de> Deserialize<'de> for EvidenceLineageAttestation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = EvidenceLineageAttestationWire::deserialize(deserializer)?;
        if wire.schema_version != LINEAGE_ATTESTATION_SCHEMA_V1 {
            return Err(serde::de::Error::custom(LineageError::UnsupportedSchema(
                wire.schema_version,
            )));
        }
        Self::new(
            wire.security_domain,
            wire.attestation_profile_id,
            wire.attestor_did,
            wire.observation_id,
            wire.descriptor,
            wire.assessed_at_unix_s,
            wire.evidence_commitment,
            wire.issuance_nonce,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl EvidenceLineageAttestation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        security_domain: CanonicalId,
        attestation_profile_id: CanonicalId,
        attestor_did: CanonicalId,
        observation_id: ObservationId,
        descriptor: LineageDescriptor,
        assessed_at_unix_s: i64,
        evidence_commitment: [u8; 32],
        issuance_nonce: [u8; 32],
    ) -> Result<Self, LineageError> {
        if !attestor_did.as_str().starts_with("did:") {
            return Err(LineageError::InvalidDid);
        }
        if evidence_commitment == [0; 32] {
            return Err(LineageError::ZeroEvidenceCommitment);
        }
        if issuance_nonce == [0; 32] {
            return Err(LineageError::ZeroNonce);
        }
        descriptor.validate()?;
        Ok(Self {
            schema_version: LINEAGE_ATTESTATION_SCHEMA_V1,
            security_domain,
            attestation_profile_id,
            attestor_did,
            observation_id,
            descriptor,
            assessed_at_unix_s,
            evidence_commitment,
            issuance_nonce,
        })
    }

    pub fn security_domain(&self) -> &CanonicalId {
        &self.security_domain
    }

    pub fn attestation_profile_id(&self) -> &CanonicalId {
        &self.attestation_profile_id
    }

    pub fn attestor_did(&self) -> &CanonicalId {
        &self.attestor_did
    }

    pub fn observation_id(&self) -> ObservationId {
        self.observation_id
    }

    pub fn descriptor(&self) -> &LineageDescriptor {
        &self.descriptor
    }

    pub fn assessed_at_unix_s(&self) -> i64 {
        self.assessed_at_unix_s
    }

    pub fn evidence_commitment(&self) -> &[u8; 32] {
        &self.evidence_commitment
    }

    pub fn validate(&self) -> Result<(), LineageError> {
        if self.schema_version != LINEAGE_ATTESTATION_SCHEMA_V1 {
            return Err(LineageError::UnsupportedSchema(self.schema_version));
        }
        if !self.attestor_did.as_str().starts_with("did:") {
            return Err(LineageError::InvalidDid);
        }
        if self.evidence_commitment == [0; 32] {
            return Err(LineageError::ZeroEvidenceCommitment);
        }
        if self.issuance_nonce == [0; 32] {
            return Err(LineageError::ZeroNonce);
        }
        self.descriptor.validate()
    }

    pub fn binds_observation(
        &self,
        observation: &SurveillanceObservation,
    ) -> Result<bool, LineageError> {
        self.validate()?;
        let observation_id = observation
            .id()
            .map_err(|e| LineageError::InvalidObservation(e.to_string()))?;
        Ok(self.observation_id == observation_id)
    }

    pub fn id(&self) -> Result<LineageAttestationId, LineageError> {
        self.validate()?;
        let payload = self.canonical_payload();
        let mut h = Sha256::new();
        h.update(LINEAGE_ATTESTATION_ID_DOMAIN_V1);
        h.update(payload);
        Ok(LineageAttestationId(h.finalize().into()))
    }

    pub fn signing_transcript(&self) -> Result<Vec<u8>, LineageError> {
        self.validate()?;
        let payload = self.canonical_payload();
        let mut out = Vec::with_capacity(LINEAGE_ATTESTATION_SIGNING_DOMAIN_V1.len() + payload.len());
        out.extend_from_slice(LINEAGE_ATTESTATION_SIGNING_DOMAIN_V1);
        out.extend_from_slice(&payload);
        Ok(out)
    }

    /// Conservative comparison of dimension-specific lineage facts.
    ///
    /// `NoKnownOverlap` means only that the two attested known identifier sets
    /// are disjoint on that dimension. It is deliberately **not** named or
    /// exposed as proof of statistical/causal independence.
    pub fn compare(&self, other: &Self) -> Result<LineageComparison, LineageError> {
        self.validate()?;
        other.validate()?;
        Ok(LineageComparison {
            left_attestation_id: self.id()?,
            right_attestation_id: other.id()?,
            same_security_domain: self.security_domain == other.security_domain,
            same_attestation_profile: self.attestation_profile_id == other.attestation_profile_id,
            dimensions: vec![
                dimension_comparison(
                    LineageDimension::UpstreamRoots,
                    &self.descriptor.upstream_roots,
                    &other.descriptor.upstream_roots,
                ),
                dimension_comparison(
                    LineageDimension::SamplingFrames,
                    &self.descriptor.sampling_frames,
                    &other.descriptor.sampling_frames,
                ),
                dimension_comparison(
                    LineageDimension::CollectionSystems,
                    &self.descriptor.collection_systems,
                    &other.descriptor.collection_systems,
                ),
                dimension_comparison(
                    LineageDimension::MeasurementSystems,
                    &self.descriptor.measurement_systems,
                    &other.descriptor.measurement_systems,
                ),
                dimension_comparison(
                    LineageDimension::ProcessingPipelines,
                    &self.descriptor.processing_pipelines,
                    &other.descriptor.processing_pipelines,
                ),
                dimension_comparison(
                    LineageDimension::OperatorControlDomains,
                    &self.descriptor.operator_control_domains,
                    &other.descriptor.operator_control_domains,
                ),
            ],
        })
    }

    fn canonical_payload(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_u16(&mut out, self.schema_version);
        put_id(&mut out, &self.security_domain);
        put_id(&mut out, &self.attestation_profile_id);
        put_id(&mut out, &self.attestor_did);
        out.extend_from_slice(self.observation_id.as_bytes());
        put_descriptor(&mut out, &self.descriptor);
        put_i64(&mut out, self.assessed_at_unix_s);
        out.extend_from_slice(&self.evidence_commitment);
        out.extend_from_slice(&self.issuance_nonce);
        out
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LineageDimension {
    UpstreamRoots,
    SamplingFrames,
    CollectionSystems,
    MeasurementSystems,
    ProcessingPipelines,
    OperatorControlDomains,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DimensionRelation {
    /// Both attestations know identifiers on this dimension and share at least one.
    SharedKnown,
    /// Both know non-empty identifier sets and those sets are disjoint.
    NoKnownOverlap,
    /// At least one attestation explicitly lacks knowledge on this dimension.
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DimensionComparison {
    pub dimension: LineageDimension,
    pub relation: DimensionRelation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LineageComparison {
    pub left_attestation_id: LineageAttestationId,
    pub right_attestation_id: LineageAttestationId,
    pub same_security_domain: bool,
    pub same_attestation_profile: bool,
    pub dimensions: Vec<DimensionComparison>,
}

impl LineageComparison {
    pub fn relation(&self, dimension: LineageDimension) -> Option<DimensionRelation> {
        self.dimensions
            .iter()
            .find(|comparison| comparison.dimension == dimension)
            .map(|comparison| comparison.relation)
    }

    pub fn has_known_shared_dimension(&self) -> bool {
        self.dimensions
            .iter()
            .any(|comparison| comparison.relation == DimensionRelation::SharedKnown)
    }

    pub fn has_unknown_dimension(&self) -> bool {
        self.dimensions
            .iter()
            .any(|comparison| comparison.relation == DimensionRelation::Unknown)
    }
}

fn dimension_comparison(
    dimension: LineageDimension,
    left: &LineageKnowledge,
    right: &LineageKnowledge,
) -> DimensionComparison {
    let relation = match (left.values(), right.values()) {
        (Some(left), Some(right)) => {
            if left.is_disjoint(right) {
                DimensionRelation::NoKnownOverlap
            } else {
                DimensionRelation::SharedKnown
            }
        }
        _ => DimensionRelation::Unknown,
    };
    DimensionComparison { dimension, relation }
}

fn put_descriptor(out: &mut Vec<u8>, descriptor: &LineageDescriptor) {
    for knowledge in [
        &descriptor.upstream_roots,
        &descriptor.sampling_frames,
        &descriptor.collection_systems,
        &descriptor.measurement_systems,
        &descriptor.processing_pipelines,
        &descriptor.operator_control_domains,
    ] {
        put_knowledge(out, knowledge);
    }
}

fn put_knowledge(out: &mut Vec<u8>, knowledge: &LineageKnowledge) {
    match knowledge {
        LineageKnowledge::Unknown => out.push(0),
        LineageKnowledge::Known(values) => {
            out.push(1);
            out.extend_from_slice(&(values.len() as u32).to_be_bytes());
            for value in values {
                put_id(out, value);
            }
        }
    }
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
    use health_surveillance_core::{
        BoundedUncertainty, EvidenceProvenance, GeographicPrecision, GeographicScope,
        IndependenceGroup, MetricKind, ObservationWindow, ObservedMetric, SignalFamily, SourceKind,
        SourceRecordDigest,
    };

    fn id(value: &str) -> CanonicalId {
        CanonicalId::new(value).unwrap()
    }

    fn known(values: &[&str]) -> LineageKnowledge {
        LineageKnowledge::known(values.iter().map(|value| id(value))).unwrap()
    }

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

    fn descriptor(root: &str, sampling: LineageKnowledge) -> LineageDescriptor {
        LineageDescriptor::new(
            known(&[root]),
            sampling,
            known(&["collection:district-17"]),
            known(&["instrument:lab-analyzer-a"]),
            known(&["pipeline:aggregate-v1"]),
            known(&["control:org-a"]),
        )
        .unwrap()
    }

    fn attestation(
        obs: &SurveillanceObservation,
        root: &str,
        sampling: LineageKnowledge,
    ) -> EvidenceLineageAttestation {
        EvidenceLineageAttestation::new(
            id("lineage-domain:public-health-v1"),
            id("lineage-profile:multi-dimension-v1"),
            id("did:mycelix:lineage-auditor-a"),
            obs.id().unwrap(),
            descriptor(root, sampling),
            13_800,
            [4; 32],
            [5; 32],
        )
        .unwrap()
    }

    #[test]
    fn all_unknown_descriptor_is_rejected() {
        assert_eq!(
            LineageDescriptor::new(
                LineageKnowledge::Unknown,
                LineageKnowledge::Unknown,
                LineageKnowledge::Unknown,
                LineageKnowledge::Unknown,
                LineageKnowledge::Unknown,
                LineageKnowledge::Unknown,
            ),
            Err(LineageError::NoKnownLineageFacts)
        );
    }

    #[test]
    fn known_set_order_does_not_change_attestation_identity() {
        let obs = observation("lineage-a");
        let mut a = descriptor("root-a", known(&["sample-b", "sample-a"]));
        let b = descriptor("root-a", known(&["sample-a", "sample-b"]));
        assert_eq!(a, b);
        a.sampling_frames = known(&["sample-a", "sample-b"]);
        let left = EvidenceLineageAttestation::new(
            id("lineage-domain:public-health-v1"),
            id("lineage-profile:multi-dimension-v1"),
            id("did:mycelix:lineage-auditor-a"),
            obs.id().unwrap(),
            a,
            13_800,
            [4; 32],
            [5; 32],
        )
        .unwrap();
        let right = EvidenceLineageAttestation::new(
            id("lineage-domain:public-health-v1"),
            id("lineage-profile:multi-dimension-v1"),
            id("did:mycelix:lineage-auditor-a"),
            obs.id().unwrap(),
            b,
            13_800,
            [4; 32],
            [5; 32],
        )
        .unwrap();
        assert_eq!(left.id().unwrap(), right.id().unwrap());
    }

    #[test]
    fn shared_upstream_root_is_detected_even_when_sampling_is_distinct() {
        let obs_a = observation("lineage-a");
        let mut obs_b = observation("lineage-b");
        obs_b.provenance.source_revision = id("rev-2");
        let a = attestation(&obs_a, "root-shared", known(&["sample-a"]));
        let b = attestation(&obs_b, "root-shared", known(&["sample-b"]));
        let comparison = a.compare(&b).unwrap();
        assert_eq!(
            comparison.relation(LineageDimension::UpstreamRoots),
            Some(DimensionRelation::SharedKnown)
        );
        assert_eq!(
            comparison.relation(LineageDimension::SamplingFrames),
            Some(DimensionRelation::NoKnownOverlap)
        );
        assert!(comparison.has_known_shared_dimension());
    }

    #[test]
    fn unknown_is_not_promoted_to_no_known_overlap() {
        let obs_a = observation("lineage-a");
        let mut obs_b = observation("lineage-b");
        obs_b.provenance.source_revision = id("rev-2");
        let a = attestation(&obs_a, "root-a", LineageKnowledge::Unknown);
        let b = attestation(&obs_b, "root-b", known(&["sample-b"]));
        let comparison = a.compare(&b).unwrap();
        assert_eq!(
            comparison.relation(LineageDimension::SamplingFrames),
            Some(DimensionRelation::Unknown)
        );
        assert!(comparison.has_unknown_dimension());
    }

    #[test]
    fn attestation_binds_exact_observation_identity() {
        let obs = observation("lineage-a");
        let changed = observation("lineage-b");
        let attestation = attestation(&obs, "root-a", known(&["sample-a"]));
        assert!(attestation.binds_observation(&obs).unwrap());
        assert!(!attestation.binds_observation(&changed).unwrap());
    }

    #[test]
    fn independence_group_change_cannot_reuse_lineage_attestation() {
        let original = observation("claimed-independent-a");
        let changed = observation("claimed-independent-b");
        let attestation = attestation(&original, "root-a", known(&["sample-a"]));
        assert_ne!(original.id().unwrap(), changed.id().unwrap());
        assert!(!attestation.binds_observation(&changed).unwrap());
    }
}
