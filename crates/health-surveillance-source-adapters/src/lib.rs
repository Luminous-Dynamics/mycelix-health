#![deny(unsafe_code)]
//! Pure, aggregate-only adapters into `health-surveillance-core`.
//!
//! These adapters deliberately start *after* source-specific aggregation. They do
//! not parse patient records, FHIR resources, laboratory line lists, exact
//! locations, genomes, or other individual/raw clinical material. Their only
//! output is a validated [`SurveillanceObservation`] carrying aggregate evidence.
//!
//! The adapters also do not manufacture epistemic confidence. Callers must supply
//! explicit uncertainty bounds produced by the source pipeline; the core contract
//! verifies that the computed/declared estimate lies inside those bounds.

use health_surveillance_core::{
    BoundedUncertainty, CanonicalId, EvidenceProvenance, GeographicScope, IndependenceGroup,
    MetricKind, ObservationWindow, ObservedMetric, SignalFamily, SourceKind,
    SurveillanceError, SurveillanceObservation,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum AdapterError {
    #[error(transparent)]
    Surveillance(#[from] SurveillanceError),
    #[error("aggregate denominator must be greater than zero")]
    ZeroDenominator,
    #[error("fraction numerator cannot exceed denominator")]
    NumeratorExceedsDenominator,
    #[error("aggregate sample count must be greater than zero")]
    EmptySampleSet,
    #[error("capacity measured_units cannot exceed total_units")]
    CapacityExceedsTotal,
}

/// Source/provenance fields shared by all adapter inputs.
///
/// There are intentionally no patient identifiers, addresses, coordinates, raw
/// records, or free-form clinical payload fields in this type.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservationContext {
    pub source_instance: CanonicalId,
    pub independence_group: IndependenceGroup,
    pub geography: GeographicScope,
    pub window: ObservationWindow,
    pub reported_at_unix_s: i64,
    pub provenance: EvidenceProvenance,
}

impl ObservationContext {
    pub fn new(
        source_instance: CanonicalId,
        independence_group: IndependenceGroup,
        geography: GeographicScope,
        window: ObservationWindow,
        reported_at_unix_s: i64,
        provenance: EvidenceProvenance,
    ) -> Result<Self, AdapterError> {
        window.validate()?;
        provenance.validate()?;
        if reported_at_unix_s < window.end_unix_s {
            return Err(AdapterError::Surveillance(
                SurveillanceError::ReportedBeforeWindowEnd,
            ));
        }
        Ok(Self {
            source_instance,
            independence_group,
            geography,
            window,
            reported_at_unix_s,
            provenance,
        })
    }

    fn build(
        self,
        signal: SignalFamily,
        source_kind: SourceKind,
        cohort_size: u64,
        metric: ObservedMetric,
    ) -> Result<SurveillanceObservation, AdapterError> {
        SurveillanceObservation::new(
            signal,
            source_kind,
            self.source_instance.as_str().to_string(),
            self.independence_group,
            self.geography,
            self.window,
            self.reported_at_unix_s,
            cohort_size,
            metric,
            self.provenance,
        )
        .map_err(AdapterError::from)
    }
}

/// Already-aggregated laboratory positives over an aggregate denominator.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LaboratoryFractionInput {
    pub context: ObservationContext,
    pub signal: SignalFamily,
    pub positive_count: u64,
    pub tested_count: u64,
    /// Source-supplied uncertainty for the resulting fraction in `[0, 1]`.
    pub uncertainty: BoundedUncertainty,
}

pub fn adapt_laboratory_fraction(
    input: LaboratoryFractionInput,
) -> Result<SurveillanceObservation, AdapterError> {
    if input.tested_count == 0 {
        return Err(AdapterError::ZeroDenominator);
    }
    if input.positive_count > input.tested_count {
        return Err(AdapterError::NumeratorExceedsDenominator);
    }

    let estimate = input.positive_count as f64 / input.tested_count as f64;
    let metric = ObservedMetric::new(
        MetricKind::FractionPositive,
        estimate,
        input.uncertainty,
        "fraction",
    )?;

    input.context.build(
        input.signal,
        SourceKind::LaboratoryAggregate,
        input.tested_count,
        metric,
    )
}

/// Already-aggregated syndromic event count over an aggregate population/visit
/// denominator. The adapter computes a rate per 100,000 but does not infer the
/// syndrome from patient-level records.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SyndromicRateInput {
    pub context: ObservationContext,
    pub signal: SignalFamily,
    pub event_count: u64,
    pub denominator: u64,
    /// Source-supplied uncertainty in rate-per-100k units.
    pub uncertainty: BoundedUncertainty,
}

pub fn adapt_syndromic_rate_per_100k(
    input: SyndromicRateInput,
) -> Result<SurveillanceObservation, AdapterError> {
    if input.denominator == 0 {
        return Err(AdapterError::ZeroDenominator);
    }

    let estimate = (input.event_count as f64 / input.denominator as f64) * 100_000.0;
    let metric = ObservedMetric::new(
        MetricKind::RatePer100k,
        estimate,
        input.uncertainty,
        "per_100k",
    )?;

    input.context.build(
        input.signal,
        SourceKind::ClinicalSyndromicAggregate,
        input.denominator,
        metric,
    )
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConcentrationSource {
    Wastewater,
    Environmental,
}

/// Already-aggregated concentration/index result from wastewater or another
/// environmental monitoring pipeline.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConcentrationAggregateInput {
    pub context: ObservationContext,
    pub signal: SignalFamily,
    pub source: ConcentrationSource,
    /// Number of aggregate samples/units contributing to this released value.
    pub sample_count: u64,
    pub estimate: f64,
    pub uncertainty: BoundedUncertainty,
    /// Source-defined canonical aggregate unit. Raw assay payloads do not belong here.
    pub unit: CanonicalId,
}

pub fn adapt_concentration(
    input: ConcentrationAggregateInput,
) -> Result<SurveillanceObservation, AdapterError> {
    if input.sample_count == 0 {
        return Err(AdapterError::EmptySampleSet);
    }

    let source_kind = match input.source {
        ConcentrationSource::Wastewater => SourceKind::WastewaterAggregate,
        ConcentrationSource::Environmental => SourceKind::EnvironmentalAggregate,
    };
    let metric = ObservedMetric::new(
        MetricKind::ConcentrationIndex,
        input.estimate,
        input.uncertainty,
        input.unit.as_str().to_string(),
    )?;

    input
        .context
        .build(input.signal, source_kind, input.sample_count, metric)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapacitySemantics {
    Available,
    Occupied,
}

impl CapacitySemantics {
    fn unit(self) -> &'static str {
        match self {
            Self::Available => "fraction_available",
            Self::Occupied => "fraction_occupied",
        }
    }
}

/// Already-aggregated health-system capacity count. `signal` identifies the
/// service/syndrome family to which the capacity snapshot applies; deployments
/// may use `SignalFamily::Other(...)` for all-cause or service-specific capacity.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CapacityFractionInput {
    pub context: ObservationContext,
    pub signal: SignalFamily,
    pub measured_units: u64,
    pub total_units: u64,
    pub semantics: CapacitySemantics,
    /// Source-supplied uncertainty for the resulting capacity fraction.
    pub uncertainty: BoundedUncertainty,
}

pub fn adapt_capacity_fraction(
    input: CapacityFractionInput,
) -> Result<SurveillanceObservation, AdapterError> {
    if input.total_units == 0 {
        return Err(AdapterError::ZeroDenominator);
    }
    if input.measured_units > input.total_units {
        return Err(AdapterError::CapacityExceedsTotal);
    }

    let estimate = input.measured_units as f64 / input.total_units as f64;
    let metric = ObservedMetric::new(
        MetricKind::CapacityFraction,
        estimate,
        input.uncertainty,
        input.semantics.unit(),
    )?;

    input.context.build(
        input.signal,
        SourceKind::HealthSystemCapacityAggregate,
        input.total_units,
        metric,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use health_surveillance_core::{
        Digest32Algorithm, GeographicPrecision, SourceRecordDigest,
    };

    fn context(source: &str) -> ObservationContext {
        ObservationContext::new(
            CanonicalId::new(source).unwrap(),
            IndependenceGroup::new(format!("{source}-lineage")).unwrap(),
            GeographicScope::new(
                "health-district",
                "district-17",
                GeographicPrecision::District,
            )
            .unwrap(),
            ObservationWindow::new(10_000, 13_600).unwrap(),
            13_700,
            EvidenceProvenance::new(
                "producer-a",
                "aggregate-protocol-v1",
                "rev-1",
                Some("upstream-a"),
                SourceRecordDigest::new(Digest32Algorithm::Sha256, [7; 32]).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn laboratory_fraction_is_computed_from_aggregate_counts() {
        let observation = adapt_laboratory_fraction(LaboratoryFractionInput {
            context: context("lab-feed-a"),
            signal: SignalFamily::Respiratory,
            positive_count: 20,
            tested_count: 100,
            uncertainty: BoundedUncertainty::new(0.15, 0.25).unwrap(),
        })
        .unwrap();

        assert_eq!(observation.source_kind, SourceKind::LaboratoryAggregate);
        assert_eq!(observation.cohort_size, 100);
        assert_eq!(observation.metric.kind, MetricKind::FractionPositive);
        assert!((observation.metric.estimate - 0.20).abs() < f64::EPSILON);
        assert_eq!(observation.metric.unit.as_str(), "fraction");
    }

    #[test]
    fn laboratory_fraction_rejects_impossible_aggregate_counts() {
        let result = adapt_laboratory_fraction(LaboratoryFractionInput {
            context: context("lab-feed-a"),
            signal: SignalFamily::Respiratory,
            positive_count: 101,
            tested_count: 100,
            uncertainty: BoundedUncertainty::new(0.0, 1.0).unwrap(),
        });
        assert_eq!(result, Err(AdapterError::NumeratorExceedsDenominator));
    }

    #[test]
    fn source_uncertainty_must_cover_the_computed_fraction() {
        let result = adapt_laboratory_fraction(LaboratoryFractionInput {
            context: context("lab-feed-a"),
            signal: SignalFamily::Respiratory,
            positive_count: 20,
            tested_count: 100,
            uncertainty: BoundedUncertainty::new(0.30, 0.40).unwrap(),
        });
        assert_eq!(
            result,
            Err(AdapterError::Surveillance(
                SurveillanceError::EstimateOutsideUncertainty
            ))
        );
    }

    #[test]
    fn syndromic_rate_uses_only_preaggregated_event_and_denominator_counts() {
        let observation = adapt_syndromic_rate_per_100k(SyndromicRateInput {
            context: context("syndromic-feed-a"),
            signal: SignalFamily::Respiratory,
            event_count: 50,
            denominator: 1_000,
            uncertainty: BoundedUncertainty::new(4_500.0, 5_500.0).unwrap(),
        })
        .unwrap();

        assert_eq!(
            observation.source_kind,
            SourceKind::ClinicalSyndromicAggregate
        );
        assert_eq!(observation.metric.kind, MetricKind::RatePer100k);
        assert!((observation.metric.estimate - 5_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn wastewater_and_environmental_sources_remain_distinct() {
        let wastewater = adapt_concentration(ConcentrationAggregateInput {
            context: context("wastewater-feed-a"),
            signal: SignalFamily::Respiratory,
            source: ConcentrationSource::Wastewater,
            sample_count: 12,
            estimate: 3.4,
            uncertainty: BoundedUncertainty::new(3.0, 3.8).unwrap(),
            unit: CanonicalId::new("normalized_concentration_index").unwrap(),
        })
        .unwrap();
        let environmental = adapt_concentration(ConcentrationAggregateInput {
            context: context("environment-feed-a"),
            signal: SignalFamily::Respiratory,
            source: ConcentrationSource::Environmental,
            sample_count: 12,
            estimate: 3.4,
            uncertainty: BoundedUncertainty::new(3.0, 3.8).unwrap(),
            unit: CanonicalId::new("normalized_concentration_index").unwrap(),
        })
        .unwrap();

        assert_eq!(wastewater.source_kind, SourceKind::WastewaterAggregate);
        assert_eq!(environmental.source_kind, SourceKind::EnvironmentalAggregate);
    }

    #[test]
    fn capacity_semantics_are_explicit_in_the_metric_unit() {
        let available = adapt_capacity_fraction(CapacityFractionInput {
            context: context("capacity-feed-a"),
            signal: SignalFamily::Other(CanonicalId::new("all-cause").unwrap()),
            measured_units: 25,
            total_units: 100,
            semantics: CapacitySemantics::Available,
            uncertainty: BoundedUncertainty::new(0.20, 0.30).unwrap(),
        })
        .unwrap();
        let occupied = adapt_capacity_fraction(CapacityFractionInput {
            context: context("capacity-feed-b"),
            signal: SignalFamily::Other(CanonicalId::new("all-cause").unwrap()),
            measured_units: 75,
            total_units: 100,
            semantics: CapacitySemantics::Occupied,
            uncertainty: BoundedUncertainty::new(0.70, 0.80).unwrap(),
        })
        .unwrap();

        assert_eq!(available.metric.unit.as_str(), "fraction_available");
        assert_eq!(occupied.metric.unit.as_str(), "fraction_occupied");
        assert_eq!(
            available.source_kind,
            SourceKind::HealthSystemCapacityAggregate
        );
    }

    #[test]
    fn zero_denominators_and_empty_sample_sets_fail_closed() {
        let lab = adapt_laboratory_fraction(LaboratoryFractionInput {
            context: context("lab-feed-a"),
            signal: SignalFamily::Respiratory,
            positive_count: 0,
            tested_count: 0,
            uncertainty: BoundedUncertainty::new(0.0, 0.0).unwrap(),
        });
        assert_eq!(lab, Err(AdapterError::ZeroDenominator));

        let wastewater = adapt_concentration(ConcentrationAggregateInput {
            context: context("wastewater-feed-a"),
            signal: SignalFamily::Respiratory,
            source: ConcentrationSource::Wastewater,
            sample_count: 0,
            estimate: 1.0,
            uncertainty: BoundedUncertainty::new(0.5, 1.5).unwrap(),
            unit: CanonicalId::new("index").unwrap(),
        });
        assert_eq!(wastewater, Err(AdapterError::EmptySampleSet));
    }

    #[test]
    fn context_rejects_reporting_before_window_end() {
        let result = ObservationContext::new(
            CanonicalId::new("feed-a").unwrap(),
            IndependenceGroup::new("lineage-a").unwrap(),
            GeographicScope::new(
                "health-district",
                "district-17",
                GeographicPrecision::District,
            )
            .unwrap(),
            ObservationWindow::new(10_000, 13_600).unwrap(),
            13_599,
            EvidenceProvenance::new(
                "producer-a",
                "aggregate-protocol-v1",
                "rev-1",
                None::<String>,
                SourceRecordDigest::sha256([8; 32]).unwrap(),
            )
            .unwrap(),
        );

        assert_eq!(
            result,
            Err(AdapterError::Surveillance(
                SurveillanceError::ReportedBeforeWindowEnd
            ))
        );
    }
}
