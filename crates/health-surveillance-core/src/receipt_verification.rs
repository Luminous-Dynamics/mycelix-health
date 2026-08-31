use crate::{
    BundleError, EvidenceBundle, LineageDiversityAssessment, ReleaseAssessment, SurveillanceError,
    SurveillanceObservation,
};

impl ReleaseAssessment {
    /// Recompute this exact policy against the supplied observation and require
    /// the resulting deterministic release receipt to equal this one.
    ///
    /// Successful deserialization alone does not verify a stored or received
    /// assessment. This check binds the policy, observation identity, and exact
    /// withhold reasons again from source content.
    pub fn verifies_for_observation(
        &self,
        observation: &SurveillanceObservation,
    ) -> Result<bool, SurveillanceError> {
        let expected = self.policy.assess(observation)?;
        Ok(self == &expected)
    }
}

impl LineageDiversityAssessment {
    /// Recompute this exact structural lineage-diversity policy against the
    /// supplied bundle and require the deterministic assessment to match.
    ///
    /// This verifies receipt/content binding only. A matching result still does
    /// not establish true causal/statistical independence of the claimed groups.
    pub fn verifies_for_bundle(&self, bundle: &EvidenceBundle) -> Result<bool, BundleError> {
        let expected = bundle.assess_lineage_diversity(self.policy)?;
        Ok(self == &expected)
    }
}
