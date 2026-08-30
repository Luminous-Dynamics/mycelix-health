use crate::{AggregateReleasePolicy, SurveillanceObservation};

impl SurveillanceObservation {
    /// Return the v1 aggregate contributing-unit count under a semantically
    /// neutral name.
    ///
    /// This is exactly the existing `cohort_size` wire value; calling it through
    /// this accessor does not change serialization or `ObservationId` encoding.
    /// Its unit is defined by the source/acquisition protocol and may represent,
    /// for example, tests, visits/events, environmental samples, or capacity
    /// units. It does **not** imply a count of unique people unless a separately
    /// reviewed source protocol establishes that meaning.
    pub fn contributing_unit_count(&self) -> u64 {
        self.cohort_size
    }
}

impl AggregateReleasePolicy {
    /// Return the v1 minimum aggregate contributing-unit threshold under a
    /// semantically neutral name.
    ///
    /// This is exactly the existing `min_cohort_size` policy value. The accessor
    /// does not change serialization, release evaluation, or any policy identity
    /// derived by downstream DNA code. Its interpretation is source/profile
    /// specific and must not be advertised as a universal unique-human privacy
    /// threshold.
    pub fn min_contributing_unit_count(&self) -> u64 {
        self.min_cohort_size()
    }
}
