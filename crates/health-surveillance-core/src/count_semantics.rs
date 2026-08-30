use crate::SurveillanceObservation;

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
