use crate::{
    BundleError, EvidenceBundle, EvidenceBundleId, ObservationId, SurveillanceError,
    SurveillanceObservation,
};

impl ObservationId {
    /// Recompute the canonical v1 observation identity and compare it with this
    /// received/stored identifier.
    ///
    /// A deserialized 32-byte ID is only a claimed content identity until this
    /// check (or the equivalent `observation.id()` comparison) succeeds.
    pub fn matches_observation(
        &self,
        observation: &SurveillanceObservation,
    ) -> Result<bool, SurveillanceError> {
        Ok(*self == observation.id()?)
    }
}

impl EvidenceBundleId {
    /// Recompute the canonical v1 bundle identity and compare it with this
    /// received/stored identifier.
    ///
    /// This verifies content binding only. It does not authenticate the producer,
    /// establish source independence, or confer operational authority.
    pub fn matches_bundle(&self, bundle: &EvidenceBundle) -> Result<bool, BundleError> {
        Ok(*self == bundle.id()?)
    }
}
