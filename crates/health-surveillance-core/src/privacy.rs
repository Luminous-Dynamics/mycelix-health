//! Release-boundary policies for aggregate surveillance evidence.
//!
//! These checks are deliberately structural. Meeting them does **not** prove
//! formal k-anonymity, differential privacy, or that all re-identification risk
//! has been eliminated. Producers remain responsible for domain-specific privacy
//! review and may add stronger mechanisms before publication.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{GeographicPrecision, ObservationId, SurveillanceError, SurveillanceObservation};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AggregateReleasePolicy {
    pub min_cohort_size: u64,
    pub min_window_s: u64,
    pub max_geographic_precision: GeographicPrecision,
}

impl AggregateReleasePolicy {
    pub fn new(
        min_cohort_size: u64,
        min_window_s: u64,
        max_geographic_precision: GeographicPrecision,
    ) -> Result<Self, ReleasePolicyError> {
        if min_cohort_size == 0 {
            return Err(ReleasePolicyError::ZeroMinimumCohort);
        }
        if min_window_s == 0 {
            return Err(ReleasePolicyError::ZeroMinimumWindow);
        }
        Ok(Self {
            min_cohort_size,
            min_window_s,
            max_geographic_precision,
        })
    }

    pub fn assess(
        &self,
        observation: &SurveillanceObservation,
    ) -> Result<ReleaseAssessment, SurveillanceError> {
        observation.validate()?;
        let mut reasons = Vec::new();

        if observation.cohort_size < self.min_cohort_size {
            reasons.push(WithholdReason::CohortTooSmall {
                observed: observation.cohort_size,
                minimum: self.min_cohort_size,
            });
        }
        if observation.window.duration_s() < self.min_window_s {
            reasons.push(WithholdReason::WindowTooShort {
                observed_s: observation.window.duration_s(),
                minimum_s: self.min_window_s,
            });
        }
        if observation.geography.precision > self.max_geographic_precision {
            reasons.push(WithholdReason::GeographyTooPrecise {
                observed: observation.geography.precision,
                maximum: self.max_geographic_precision,
            });
        }

        Ok(ReleaseAssessment {
            observation_id: observation.id()?,
            policy: *self,
            reasons,
        })
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ReleasePolicyError {
    #[error("minimum cohort size must be greater than zero")]
    ZeroMinimumCohort,
    #[error("minimum release window must be greater than zero")]
    ZeroMinimumWindow,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WithholdReason {
    CohortTooSmall { observed: u64, minimum: u64 },
    WindowTooShort { observed_s: u64, minimum_s: u64 },
    GeographyTooPrecise {
        observed: GeographicPrecision,
        maximum: GeographicPrecision,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseAssessment {
    pub observation_id: ObservationId,
    pub policy: AggregateReleasePolicy,
    reasons: Vec<WithholdReason>,
}

impl ReleaseAssessment {
    pub fn eligible_for_release(&self) -> bool {
        self.reasons.is_empty()
    }

    pub fn reasons(&self) -> &[WithholdReason] {
        &self.reasons
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FreshnessPolicy {
    pub max_age_s: u64,
}

impl FreshnessPolicy {
    pub fn assess(
        &self,
        observation: &SurveillanceObservation,
        now_unix_s: i64,
    ) -> Result<FreshnessStatus, FreshnessError> {
        observation
            .validate()
            .map_err(FreshnessError::InvalidObservation)?;
        if now_unix_s < observation.reported_at_unix_s {
            return Err(FreshnessError::ReportFromFuture);
        }
        let age_s = now_unix_s.abs_diff(observation.reported_at_unix_s);
        if age_s <= self.max_age_s {
            Ok(FreshnessStatus::Fresh { age_s })
        } else {
            Ok(FreshnessStatus::Stale {
                age_s,
                max_age_s: self.max_age_s,
            })
        }
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum FreshnessError {
    #[error("invalid surveillance observation: {0}")]
    InvalidObservation(SurveillanceError),
    #[error("observation report timestamp is in the future relative to the evaluator clock")]
    ReportFromFuture,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessStatus {
    Fresh { age_s: u64 },
    Stale { age_s: u64, max_age_s: u64 },
}

#[cfg(test)]
mod tests {
    use crate::{
        BoundedUncertainty, EvidenceProvenance, GeographicScope, IndependenceGroup, MetricKind,
        ObservationWindow, ObservedMetric, SignalFamily, SourceKind,
    };

    use super::*;

    fn observation(cohort_size: u64, precision: GeographicPrecision) -> SurveillanceObservation {
        SurveillanceObservation::new(
            SignalFamily::Respiratory,
            SourceKind::ClinicalSyndromicAggregate,
            "feed-a",
            IndependenceGroup::new("lineage-a").unwrap(),
            GeographicScope::new("district", "d17", precision).unwrap(),
            ObservationWindow::new(1_000, 4_600).unwrap(),
            4_700,
            cohort_size,
            ObservedMetric::new(
                MetricKind::RatePer100k,
                20.0,
                BoundedUncertainty::new(18.0, 23.0).unwrap(),
                "per_100k",
            )
            .unwrap(),
            EvidenceProvenance::new(
                "producer-a",
                "protocol-v1",
                "rev-1",
                Some("upstream-a"),
                [1; 32],
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn release_policy_withholds_small_or_overprecise_aggregates() {
        let policy = AggregateReleasePolicy::new(50, 3_600, GeographicPrecision::District).unwrap();
        let assessment = policy
            .assess(&observation(10, GeographicPrecision::Facility))
            .unwrap();
        assert!(!assessment.eligible_for_release());
        assert_eq!(assessment.reasons().len(), 2);
    }

    #[test]
    fn release_policy_accepts_structurally_eligible_aggregate() {
        let policy = AggregateReleasePolicy::new(50, 3_600, GeographicPrecision::District).unwrap();
        let assessment = policy
            .assess(&observation(100, GeographicPrecision::District))
            .unwrap();
        assert!(assessment.eligible_for_release());
    }

    #[test]
    fn freshness_is_explicit_and_does_not_upgrade_stale_evidence() {
        let observation = observation(100, GeographicPrecision::District);
        let policy = FreshnessPolicy { max_age_s: 300 };
        assert_eq!(
            policy.assess(&observation, 5_100).unwrap(),
            FreshnessStatus::Stale {
                age_s: 400,
                max_age_s: 300
            }
        );
    }
}
