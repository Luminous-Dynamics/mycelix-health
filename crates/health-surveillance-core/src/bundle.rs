//! Evidence bundling and lineage-diversity accounting.
//!
//! A bundle can establish that multiple observations are distinct, temporally
//! overlapping, and drawn from some number of independent source lineages. It
//! does **not** establish that those observations agree, that a causal outbreak
//! exists, or that any operational response is warranted.

use std::{collections::HashSet, fmt};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    GeographicScope, ObservationId, ObservationWindow, SignalFamily, SourceKind, SurveillanceError,
    SurveillanceObservation,
};

pub const BUNDLE_ID_DOMAIN_V1: &[u8] = b"mycelix-health-surveillance-bundle-v1\0";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct EvidenceBundleId([u8; 32]);

impl EvidenceBundleId {
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

impl fmt::Display for EvidenceBundleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct EvidenceBundle {
    observations: Vec<SurveillanceObservation>,
}

impl EvidenceBundle {
    pub fn new(mut observations: Vec<SurveillanceObservation>) -> Result<Self, BundleError> {
        if observations.is_empty() {
            return Err(BundleError::EmptyBundle);
        }

        for observation in &observations {
            observation
                .validate()
                .map_err(BundleError::InvalidObservation)?;
        }

        let expected_signal = observations[0].signal.clone();
        let expected_geography = observations[0].geography.clone();
        if observations
            .iter()
            .skip(1)
            .any(|observation| observation.signal != expected_signal)
        {
            return Err(BundleError::MixedSignalFamily);
        }
        if observations
            .iter()
            .skip(1)
            .any(|observation| observation.geography != expected_geography)
        {
            return Err(BundleError::MixedGeography);
        }

        let latest_start = observations
            .iter()
            .map(|observation| observation.window.start_unix_s)
            .max()
            .expect("non-empty bundle");
        let earliest_end = observations
            .iter()
            .map(|observation| observation.window.end_unix_s)
            .min()
            .expect("non-empty bundle");
        if earliest_end <= latest_start {
            return Err(BundleError::NonOverlappingWindows);
        }

        let mut keyed = observations
            .drain(..)
            .map(|observation| {
                let id = observation.id().map_err(BundleError::InvalidObservation)?;
                Ok((id, observation))
            })
            .collect::<Result<Vec<_>, BundleError>>()?;
        keyed.sort_by_key(|(id, _)| *id);

        if keyed.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err(BundleError::DuplicateObservation);
        }

        Ok(Self {
            observations: keyed
                .into_iter()
                .map(|(_, observation)| observation)
                .collect(),
        })
    }

    pub fn observations(&self) -> &[SurveillanceObservation] {
        &self.observations
    }

    pub fn signal(&self) -> &SignalFamily {
        &self.observations[0].signal
    }

    pub fn geography(&self) -> &GeographicScope {
        &self.observations[0].geography
    }

    pub fn overlap_window(&self) -> ObservationWindow {
        let latest_start = self
            .observations
            .iter()
            .map(|observation| observation.window.start_unix_s)
            .max()
            .expect("validated bundle is non-empty");
        let earliest_end = self
            .observations
            .iter()
            .map(|observation| observation.window.end_unix_s)
            .min()
            .expect("validated bundle is non-empty");
        ObservationWindow {
            start_unix_s: latest_start,
            end_unix_s: earliest_end,
        }
    }

    pub fn id(&self) -> Result<EvidenceBundleId, BundleError> {
        self.verify()?;
        let mut h = Sha256::new();
        h.update(BUNDLE_ID_DOMAIN_V1);
        h.update((self.observations.len() as u64).to_be_bytes());
        for observation in &self.observations {
            h.update(
                observation
                    .id()
                    .map_err(BundleError::InvalidObservation)?
                    .as_bytes(),
            );
        }
        Ok(EvidenceBundleId(h.finalize().into()))
    }

    pub fn verify(&self) -> Result<(), BundleError> {
        let reconstructed = Self::new(self.observations.clone())?;
        let lhs: Vec<ObservationId> = self
            .observations
            .iter()
            .map(|observation| observation.id().map_err(BundleError::InvalidObservation))
            .collect::<Result<_, _>>()?;
        let rhs: Vec<ObservationId> = reconstructed
            .observations
            .iter()
            .map(|observation| observation.id().map_err(BundleError::InvalidObservation))
            .collect::<Result<_, _>>()?;
        if lhs != rhs {
            return Err(BundleError::NonCanonicalOrder);
        }
        Ok(())
    }

    pub fn assess_lineage_diversity(
        &self,
        policy: LineageDiversityPolicy,
    ) -> Result<LineageDiversityAssessment, BundleError> {
        self.verify()?;
        let independent_groups: HashSet<_> = self
            .observations
            .iter()
            .map(|observation| observation.independence_group.clone())
            .collect();
        let source_kinds: HashSet<_> = self
            .observations
            .iter()
            .map(|observation| observation.source_kind.clone())
            .collect();
        let independent_group_count = independent_groups.len();
        let source_kind_count = source_kinds.len();
        let status = if independent_group_count >= policy.min_independent_groups()
            && source_kind_count >= policy.min_source_kinds()
        {
            LineageDiversityStatus::MeetsPolicy
        } else {
            LineageDiversityStatus::Insufficient
        };
        Ok(LineageDiversityAssessment {
            bundle_id: self.id()?,
            policy,
            observation_count: self.observations.len(),
            independent_group_count,
            source_kind_count,
            derivative_or_correlated_count: self
                .observations
                .len()
                .saturating_sub(independent_group_count),
            status,
        })
    }
}

impl<'de> Deserialize<'de> for EvidenceBundle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            observations: Vec<SurveillanceObservation>,
        }
        let wire = Wire::deserialize(deserializer)?;
        EvidenceBundle::new(wire.observations).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum BundleError {
    #[error("evidence bundle must contain at least one observation")]
    EmptyBundle,
    #[error("invalid observation in bundle: {0}")]
    InvalidObservation(SurveillanceError),
    #[error("bundle contains duplicate observation content identities")]
    DuplicateObservation,
    #[error("v1 evidence bundle cannot mix signal families")]
    MixedSignalFamily,
    #[error("v1 evidence bundle cannot mix geographic scopes")]
    MixedGeography,
    #[error("bundle observations do not share a positive-width overlapping time window")]
    NonOverlappingWindows,
    #[error("bundle observation order is not canonical")]
    NonCanonicalOrder,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct LineageDiversityPolicy {
    min_independent_groups: usize,
    min_source_kinds: usize,
}

impl LineageDiversityPolicy {
    pub fn new(
        min_independent_groups: usize,
        min_source_kinds: usize,
    ) -> Result<Self, LineageDiversityPolicyError> {
        if min_independent_groups == 0 {
            return Err(LineageDiversityPolicyError::ZeroIndependentGroups);
        }
        if min_source_kinds == 0 {
            return Err(LineageDiversityPolicyError::ZeroSourceKinds);
        }
        Ok(Self {
            min_independent_groups,
            min_source_kinds,
        })
    }

    pub fn min_independent_groups(&self) -> usize {
        self.min_independent_groups
    }

    pub fn min_source_kinds(&self) -> usize {
        self.min_source_kinds
    }
}

impl<'de> Deserialize<'de> for LineageDiversityPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            min_independent_groups: usize,
            min_source_kinds: usize,
        }
        let wire = Wire::deserialize(deserializer)?;
        Self::new(wire.min_independent_groups, wire.min_source_kinds)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum LineageDiversityPolicyError {
    #[error("minimum independent-group count must be greater than zero")]
    ZeroIndependentGroups,
    #[error("minimum source-kind count must be greater than zero")]
    ZeroSourceKinds,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LineageDiversityStatus {
    MeetsPolicy,
    Insufficient,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LineageDiversityAssessment {
    pub bundle_id: EvidenceBundleId,
    pub policy: LineageDiversityPolicy,
    pub observation_count: usize,
    pub independent_group_count: usize,
    pub source_kind_count: usize,
    pub derivative_or_correlated_count: usize,
    pub status: LineageDiversityStatus,
}

#[cfg(test)]
mod tests {
    use crate::{
        BoundedUncertainty, EvidenceProvenance, GeographicPrecision, GeographicScope,
        IndependenceGroup, MetricKind, ObservedMetric, SourceRecordDigest,
    };

    use super::*;

    fn observation(
        source_kind: SourceKind,
        source: &str,
        independence_group: &str,
        window: ObservationWindow,
        digest_byte: u8,
    ) -> SurveillanceObservation {
        SurveillanceObservation::new(
            SignalFamily::Respiratory,
            source_kind,
            source,
            IndependenceGroup::new(independence_group).unwrap(),
            GeographicScope::new("district", "d17", GeographicPrecision::District).unwrap(),
            window,
            window.end_unix_s + 60,
            200,
            ObservedMetric::new(
                MetricKind::FractionPositive,
                0.2,
                BoundedUncertainty::new(0.1, 0.3).unwrap(),
                "fraction",
            )
            .unwrap(),
            EvidenceProvenance::new(
                source,
                "protocol-v1",
                "rev-1",
                Some(independence_group),
                SourceRecordDigest::sha256([digest_byte; 32]).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn bundle_identity_is_order_independent() {
        let a = observation(
            SourceKind::LaboratoryAggregate,
            "lab-a",
            "lineage-a",
            ObservationWindow::new(1_000, 2_000).unwrap(),
            1,
        );
        let b = observation(
            SourceKind::WastewaterAggregate,
            "ww-a",
            "lineage-b",
            ObservationWindow::new(1_500, 2_500).unwrap(),
            2,
        );
        let ab = EvidenceBundle::new(vec![a.clone(), b.clone()]).unwrap();
        let ba = EvidenceBundle::new(vec![b, a]).unwrap();
        assert_eq!(ab.id().unwrap(), ba.id().unwrap());
        assert_eq!(
            ab.overlap_window(),
            ObservationWindow::new(1_500, 2_000).unwrap()
        );
    }

    #[test]
    fn duplicate_content_identity_is_rejected() {
        let a = observation(
            SourceKind::LaboratoryAggregate,
            "lab-a",
            "lineage-a",
            ObservationWindow::new(1_000, 2_000).unwrap(),
            1,
        );
        assert_eq!(
            EvidenceBundle::new(vec![a.clone(), a]),
            Err(BundleError::DuplicateObservation)
        );
    }

    #[test]
    fn derivative_feeds_do_not_inflate_independent_lineage_count() {
        let a = observation(
            SourceKind::LaboratoryAggregate,
            "dashboard-a",
            "shared-upstream-lab",
            ObservationWindow::new(1_000, 2_000).unwrap(),
            1,
        );
        let b = observation(
            SourceKind::LaboratoryAggregate,
            "dashboard-b",
            "shared-upstream-lab",
            ObservationWindow::new(1_100, 2_100).unwrap(),
            2,
        );
        let c = observation(
            SourceKind::WastewaterAggregate,
            "ww-a",
            "independent-ww",
            ObservationWindow::new(1_200, 2_200).unwrap(),
            3,
        );
        let bundle = EvidenceBundle::new(vec![a, b, c]).unwrap();
        let assessment = bundle
            .assess_lineage_diversity(LineageDiversityPolicy::new(2, 2).unwrap())
            .unwrap();
        assert_eq!(assessment.observation_count, 3);
        assert_eq!(assessment.independent_group_count, 2);
        assert_eq!(assessment.derivative_or_correlated_count, 1);
        assert_eq!(assessment.status, LineageDiversityStatus::MeetsPolicy);
        assert_eq!(assessment.policy.min_independent_groups(), 2);
    }

    #[test]
    fn zero_lineage_thresholds_cannot_be_constructed() {
        assert_eq!(
            LineageDiversityPolicy::new(0, 1),
            Err(LineageDiversityPolicyError::ZeroIndependentGroups)
        );
    }

    #[test]
    fn non_overlapping_windows_are_rejected() {
        let a = observation(
            SourceKind::LaboratoryAggregate,
            "lab-a",
            "lineage-a",
            ObservationWindow::new(1_000, 2_000).unwrap(),
            1,
        );
        let b = observation(
            SourceKind::WastewaterAggregate,
            "ww-a",
            "lineage-b",
            ObservationWindow::new(2_000, 3_000).unwrap(),
            2,
        );
        assert_eq!(
            EvidenceBundle::new(vec![a, b]),
            Err(BundleError::NonOverlappingWindows)
        );
    }
}
