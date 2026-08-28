use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::identity::ObservationId;

pub const SURVEILLANCE_SCHEMA_V1: u16 = 1;
const MAX_ID_LEN: usize = 128;

#[derive(Clone, Debug, Error, PartialEq)]
pub enum SurveillanceError {
    #[error("identifier is empty")]
    EmptyIdentifier,
    #[error("identifier exceeds 128 bytes")]
    IdentifierTooLong,
    #[error("identifier contains unsupported characters")]
    InvalidIdentifierCharacters,
    #[error("observation window must have end > start")]
    InvalidWindow,
    #[error("reported_at_unix_s must be at or after the observation window end")]
    ReportedBeforeWindowEnd,
    #[error("metric estimate must be finite")]
    NonFiniteEstimate,
    #[error("uncertainty bounds must be finite")]
    NonFiniteUncertainty,
    #[error("uncertainty lower bound exceeds upper bound")]
    ReversedUncertainty,
    #[error("metric estimate is outside the admissible range for its metric kind")]
    MetricOutOfRange,
    #[error("metric estimate is outside its declared uncertainty interval")]
    EstimateOutsideUncertainty,
    #[error("count metrics must be non-negative whole numbers")]
    InvalidCountEstimate,
    #[error("cohort_size must be greater than zero")]
    EmptyCohort,
    #[error("source_record_digest must not be all zeroes")]
    ZeroSourceRecordDigest,
    #[error("schema version {0} is unsupported")]
    UnsupportedSchema(u16),
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct CanonicalId(String);

impl CanonicalId {
    pub fn new(value: impl Into<String>) -> Result<Self, SurveillanceError> {
        let value = value.into();
        if value.is_empty() {
            return Err(SurveillanceError::EmptyIdentifier);
        }
        if value.len() > MAX_ID_LEN {
            return Err(SurveillanceError::IdentifierTooLong);
        }
        if !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b':' | b'/'))
        {
            return Err(SurveillanceError::InvalidIdentifierCharacters);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CanonicalId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum GeographicPrecision {
    Country,
    Region,
    District,
    Facility,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GeographicScope {
    /// Namespace/scheme for the geographic code, e.g. ISO-3166 or a health-district scheme.
    pub scheme: CanonicalId,
    /// Code within the declared scheme. Exact addresses and coordinates are intentionally absent.
    pub code: CanonicalId,
    pub precision: GeographicPrecision,
}

impl GeographicScope {
    pub fn new(
        scheme: impl Into<String>,
        code: impl Into<String>,
        precision: GeographicPrecision,
    ) -> Result<Self, SurveillanceError> {
        Ok(Self {
            scheme: CanonicalId::new(scheme)?,
            code: CanonicalId::new(code)?,
            precision,
        })
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ObservationWindow {
    pub start_unix_s: i64,
    pub end_unix_s: i64,
}

impl ObservationWindow {
    pub fn new(start_unix_s: i64, end_unix_s: i64) -> Result<Self, SurveillanceError> {
        if end_unix_s <= start_unix_s {
            return Err(SurveillanceError::InvalidWindow);
        }
        Ok(Self {
            start_unix_s,
            end_unix_s,
        })
    }

    pub fn duration_s(&self) -> u64 {
        self.end_unix_s.abs_diff(self.start_unix_s)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SignalFamily {
    Respiratory,
    Gastrointestinal,
    Febrile,
    Neurological,
    Dermatologic,
    Other(CanonicalId),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    ClinicalSyndromicAggregate,
    LaboratoryAggregate,
    WastewaterAggregate,
    EnvironmentalAggregate,
    AbsenteeismAggregate,
    HealthSystemCapacityAggregate,
    Other(CanonicalId),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct IndependenceGroup(pub CanonicalId);

impl IndependenceGroup {
    pub fn new(value: impl Into<String>) -> Result<Self, SurveillanceError> {
        Ok(Self(CanonicalId::new(value)?))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct BoundedUncertainty {
    pub lower: f64,
    pub upper: f64,
}

impl BoundedUncertainty {
    pub fn new(lower: f64, upper: f64) -> Result<Self, SurveillanceError> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(SurveillanceError::NonFiniteUncertainty);
        }
        if lower > upper {
            return Err(SurveillanceError::ReversedUncertainty);
        }
        Ok(Self { lower, upper })
    }

    pub fn contains(&self, value: f64) -> bool {
        value >= self.lower && value <= self.upper
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    Count,
    RatePer100k,
    FractionPositive,
    ConcentrationIndex,
    CapacityFraction,
    Other(CanonicalId),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ObservedMetric {
    pub kind: MetricKind,
    pub estimate: f64,
    pub uncertainty: BoundedUncertainty,
    /// Canonical unit id such as `count`, `per_100k`, `fraction`, or a source-defined unit.
    pub unit: CanonicalId,
}

impl ObservedMetric {
    pub fn new(
        kind: MetricKind,
        estimate: f64,
        uncertainty: BoundedUncertainty,
        unit: impl Into<String>,
    ) -> Result<Self, SurveillanceError> {
        if !estimate.is_finite() {
            return Err(SurveillanceError::NonFiniteEstimate);
        }
        if !uncertainty.lower.is_finite() || !uncertainty.upper.is_finite() {
            return Err(SurveillanceError::NonFiniteUncertainty);
        }
        if uncertainty.lower > uncertainty.upper {
            return Err(SurveillanceError::ReversedUncertainty);
        }
        if !uncertainty.contains(estimate) {
            return Err(SurveillanceError::EstimateOutsideUncertainty);
        }
        match &kind {
            MetricKind::Count => {
                if estimate < 0.0 || estimate.fract() != 0.0 || uncertainty.lower < 0.0 {
                    return Err(SurveillanceError::InvalidCountEstimate);
                }
            }
            MetricKind::RatePer100k | MetricKind::ConcentrationIndex => {
                if estimate < 0.0 || uncertainty.lower < 0.0 {
                    return Err(SurveillanceError::MetricOutOfRange);
                }
            }
            MetricKind::FractionPositive | MetricKind::CapacityFraction => {
                if !(0.0..=1.0).contains(&estimate)
                    || uncertainty.lower < 0.0
                    || uncertainty.upper > 1.0
                {
                    return Err(SurveillanceError::MetricOutOfRange);
                }
            }
            MetricKind::Other(_) => {}
        }
        Ok(Self {
            kind,
            estimate,
            uncertainty,
            unit: CanonicalId::new(unit)?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EvidenceProvenance {
    /// Pseudonymous or institutional producer identity; not an individual patient id.
    pub producer: CanonicalId,
    /// Acquisition or aggregation protocol identity.
    pub acquisition_protocol: CanonicalId,
    /// Revision of the producing transformation/software/data contract.
    pub source_revision: CanonicalId,
    /// Optional stable upstream dataset/feed identity for derivative-source tracing.
    pub upstream_set: Option<CanonicalId>,
    /// Digest captured by the producer for the aggregate source record.
    pub source_record_digest: [u8; 32],
}

impl EvidenceProvenance {
    pub fn new(
        producer: impl Into<String>,
        acquisition_protocol: impl Into<String>,
        source_revision: impl Into<String>,
        upstream_set: Option<impl Into<String>>,
        source_record_digest: [u8; 32],
    ) -> Result<Self, SurveillanceError> {
        if source_record_digest == [0; 32] {
            return Err(SurveillanceError::ZeroSourceRecordDigest);
        }
        Ok(Self {
            producer: CanonicalId::new(producer)?,
            acquisition_protocol: CanonicalId::new(acquisition_protocol)?,
            source_revision: CanonicalId::new(source_revision)?,
            upstream_set: match upstream_set {
                Some(v) => Some(CanonicalId::new(v)?),
                None => None,
            },
            source_record_digest,
        })
    }

    pub fn validate(&self) -> Result<(), SurveillanceError> {
        if self.source_record_digest == [0; 32] {
            return Err(SurveillanceError::ZeroSourceRecordDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SurveillanceObservation {
    pub schema_version: u16,
    pub signal: SignalFamily,
    pub source_kind: SourceKind,
    /// Stable source instance id. This identifies an aggregate feed/sensor, not a person.
    pub source_instance: CanonicalId,
    /// Lineage group used to avoid treating derivative/correlated feeds as independent evidence.
    pub independence_group: IndependenceGroup,
    pub geography: GeographicScope,
    pub window: ObservationWindow,
    pub reported_at_unix_s: i64,
    /// Number of contributing records/samples/units in the released aggregate.
    pub cohort_size: u64,
    pub metric: ObservedMetric,
    pub provenance: EvidenceProvenance,
}

impl SurveillanceObservation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        signal: SignalFamily,
        source_kind: SourceKind,
        source_instance: impl Into<String>,
        independence_group: IndependenceGroup,
        geography: GeographicScope,
        window: ObservationWindow,
        reported_at_unix_s: i64,
        cohort_size: u64,
        metric: ObservedMetric,
        provenance: EvidenceProvenance,
    ) -> Result<Self, SurveillanceError> {
        if cohort_size == 0 {
            return Err(SurveillanceError::EmptyCohort);
        }
        if reported_at_unix_s < window.end_unix_s {
            return Err(SurveillanceError::ReportedBeforeWindowEnd);
        }
        Ok(Self {
            schema_version: SURVEILLANCE_SCHEMA_V1,
            signal,
            source_kind,
            source_instance: CanonicalId::new(source_instance)?,
            independence_group,
            geography,
            window,
            reported_at_unix_s,
            cohort_size,
            metric,
            provenance,
        })
    }

    pub fn validate(&self) -> Result<(), SurveillanceError> {
        if self.schema_version != SURVEILLANCE_SCHEMA_V1 {
            return Err(SurveillanceError::UnsupportedSchema(self.schema_version));
        }
        if self.cohort_size == 0 {
            return Err(SurveillanceError::EmptyCohort);
        }
        if self.window.end_unix_s <= self.window.start_unix_s {
            return Err(SurveillanceError::InvalidWindow);
        }
        if self.reported_at_unix_s < self.window.end_unix_s {
            return Err(SurveillanceError::ReportedBeforeWindowEnd);
        }
        self.provenance.validate()?;
        ObservedMetric::new(
            self.metric.kind.clone(),
            self.metric.estimate,
            self.metric.uncertainty,
            self.metric.unit.as_str().to_string(),
        )?;
        Ok(())
    }

    pub fn id(&self) -> Result<ObservationId, SurveillanceError> {
        self.validate()?;
        Ok(ObservationId::from_observation(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provenance() -> EvidenceProvenance {
        EvidenceProvenance::new(
            "hospital-network-a",
            "weekly-respiratory-v1",
            "rev-7",
            Some("ehr-feed-a"),
            [7; 32],
        )
        .unwrap()
    }

    #[test]
    fn canonical_ids_reject_whitespace_and_empty_values() {
        assert_eq!(CanonicalId::new(""), Err(SurveillanceError::EmptyIdentifier));
        assert_eq!(
            CanonicalId::new("contains spaces"),
            Err(SurveillanceError::InvalidIdentifierCharacters)
        );
        assert!(CanonicalId::new("iso:ZA-GP/district_1").is_ok());
    }

    #[test]
    fn uncertainty_must_be_bounded_and_ordered() {
        assert_eq!(
            BoundedUncertainty::new(f64::NAN, 1.0),
            Err(SurveillanceError::NonFiniteUncertainty)
        );
        assert_eq!(
            BoundedUncertainty::new(2.0, 1.0),
            Err(SurveillanceError::ReversedUncertainty)
        );
    }

    #[test]
    fn fraction_metric_requires_interval_inside_unit_range() {
        let u = BoundedUncertainty::new(-0.1, 0.5).unwrap();
        assert_eq!(
            ObservedMetric::new(MetricKind::FractionPositive, 0.2, u, "fraction"),
            Err(SurveillanceError::MetricOutOfRange)
        );
    }

    #[test]
    fn observation_has_no_individual_level_fields_and_validates() {
        let obs = SurveillanceObservation::new(
            SignalFamily::Respiratory,
            SourceKind::ClinicalSyndromicAggregate,
            "syndromic-feed-a",
            IndependenceGroup::new("ehr-lineage-a").unwrap(),
            GeographicScope::new("health-district", "district-17", GeographicPrecision::District)
                .unwrap(),
            ObservationWindow::new(1_000, 2_000).unwrap(),
            2_100,
            250,
            ObservedMetric::new(
                MetricKind::RatePer100k,
                37.0,
                BoundedUncertainty::new(32.0, 43.0).unwrap(),
                "per_100k",
            )
            .unwrap(),
            provenance(),
        )
        .unwrap();

        assert!(obs.validate().is_ok());
        assert!(obs.id().is_ok());
    }

    #[test]
    fn reporting_timestamp_cannot_precede_window_end() {
        let result = SurveillanceObservation::new(
            SignalFamily::Respiratory,
            SourceKind::LaboratoryAggregate,
            "lab-feed-a",
            IndependenceGroup::new("lab-lineage-a").unwrap(),
            GeographicScope::new("health-district", "district-17", GeographicPrecision::District)
                .unwrap(),
            ObservationWindow::new(1_000, 2_000).unwrap(),
            1_999,
            100,
            ObservedMetric::new(
                MetricKind::FractionPositive,
                0.2,
                BoundedUncertainty::new(0.1, 0.3).unwrap(),
                "fraction",
            )
            .unwrap(),
            provenance(),
        );
        assert_eq!(result, Err(SurveillanceError::ReportedBeforeWindowEnd));
    }

    #[test]
    fn zero_source_digest_cannot_be_used_as_provenance_placeholder() {
        assert_eq!(
            EvidenceProvenance::new(
                "producer-a",
                "protocol-v1",
                "rev-1",
                Some("upstream-a"),
                [0; 32],
            ),
            Err(SurveillanceError::ZeroSourceRecordDigest)
        );
    }
}
