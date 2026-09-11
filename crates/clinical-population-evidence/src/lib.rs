#![deny(unsafe_code)]
//! Population-level clinical evidence primitives.
//!
//! V1 deliberately separates four artifact classes:
//!
//! 1. hypothesis-generating safety signals;
//! 2. denominator-based population associations;
//! 3. causal effect estimates under an explicit causal design;
//! 4. replication evidence across distinct findings.
//!
//! There is intentionally no conversion API that upgrades one class into the next
//! without supplying the additional evidence required by the stronger class.

use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

const SIGNAL_POLICY_TAG: &[u8] = b"mycelix-health/population-signal-policy-v1";
const SIGNAL_TAG: &[u8] = b"mycelix-health/safety-signal-candidate-v1";
const COHORT_TAG: &[u8] = b"mycelix-health/population-cohort-v1";
const ASSOCIATION_POLICY_TAG: &[u8] = b"mycelix-health/population-association-policy-v1";
const ASSOCIATION_TAG: &[u8] = b"mycelix-health/population-association-v1";
const EFFECT_POLICY_TAG: &[u8] = b"mycelix-health/causal-effect-policy-v1";
const EFFECT_TAG: &[u8] = b"mycelix-health/causal-effect-estimate-v1";
const REPLICATION_TAG: &[u8] = b"mycelix-health/population-replication-v1";

const MAX_ID_LEN: usize = 160;
const MAX_ALLOWED_DESIGNS: usize = 32;
const MAX_REPLICATION_FINDINGS: usize = 64;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SignalDetectionMethodV1 {
    SpontaneousCaseReview,
    Disproportionality,
    DesignatedMedicalEvent,
    TemporalCluster,
    LiteratureSignal,
    ExternalRegulatorySignal,
    OtherPreSpecifiedMethod,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SignalReviewStatusV1 {
    Candidate,
    ValidatedForReview,
    ConfirmedForFurtherAssessment,
    UnderAssessment,
    Refuted,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SafetySignalPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub minimum_case_count: u64,
    pub require_distinct_subject_count: bool,
}

impl SafetySignalPolicyV1 {
    pub fn validate(&self) -> Result<(), PopulationEvidenceError> {
        require_version(self.schema_version, "signal policy")?;
        validate_id("signal policy ID", &self.policy_id)?;
        if self.minimum_case_count == 0 {
            return Err(PopulationEvidenceError::ZeroMinimumCaseCount);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, PopulationEvidenceError> {
        self.validate()?;
        hash_json(DigestDomain::ClinicalPopulationSignalPolicy, SIGNAL_POLICY_TAG, self)
    }
}

/// Hypothesis-generating safety signal.
///
/// This type has no denominator, incidence, risk, or causal-effect field by design.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SafetySignalCandidateV1 {
    pub schema_version: u16,
    pub signal_id: String,
    pub exposure_definition_digest: StoredDigest,
    pub outcome_definition_digest: StoredDigest,
    pub exact_case_set_digest: StoredDigest,
    pub source_evidence_digest: StoredDigest,
    pub detection_method: SignalDetectionMethodV1,
    pub case_count: u64,
    pub distinct_subject_count: Option<u64>,
    pub method_result_digest: StoredDigest,
    pub review_status: SignalReviewStatusV1,
    pub policy: SafetySignalPolicyV1,
    pub detected_at_micros: i64,
}

impl SafetySignalCandidateV1 {
    pub fn validate(&self) -> Result<(), PopulationEvidenceError> {
        require_version(self.schema_version, "signal candidate")?;
        validate_id("signal ID", &self.signal_id)?;
        require_shape(self.exposure_definition_digest)?;
        require_shape(self.outcome_definition_digest)?;
        require_shape(self.exact_case_set_digest)?;
        require_shape(self.source_evidence_digest)?;
        require_shape(self.method_result_digest)?;
        self.policy.validate()?;

        if self.case_count < self.policy.minimum_case_count {
            return Err(PopulationEvidenceError::InsufficientCasesForSignalPolicy);
        }
        if self.case_count == 0 {
            return Err(PopulationEvidenceError::ZeroCaseCount);
        }
        if let Some(distinct) = self.distinct_subject_count {
            if distinct == 0 || distinct > self.case_count {
                return Err(PopulationEvidenceError::InvalidDistinctSubjectCount);
            }
        } else if self.policy.require_distinct_subject_count {
            return Err(PopulationEvidenceError::MissingDistinctSubjectCount);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, PopulationEvidenceError> {
        self.validate()?;
        hash_json(DigestDomain::ClinicalSafetySignalCandidate, SIGNAL_TAG, self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum PopulationDenominatorV1 {
    /// Binary outcome in a person-based cohort. V1 requires at most one outcome per
    /// observed subject for this denominator class.
    PersonsAtRisk {
        observed_subjects: u64,
        subjects_with_outcome: u64,
    },
    /// Event count over exact aggregate person-time. Person-time is represented in
    /// microseconds to avoid an ambiguous free-form unit.
    PersonTimeMicros {
        person_time_micros: u128,
        outcome_events: u64,
    },
}

impl PopulationDenominatorV1 {
    fn validate(&self) -> Result<(), PopulationEvidenceError> {
        match self {
            Self::PersonsAtRisk {
                observed_subjects,
                subjects_with_outcome,
            } => {
                if *observed_subjects == 0 {
                    return Err(PopulationEvidenceError::ZeroPopulationDenominator);
                }
                if subjects_with_outcome > observed_subjects {
                    return Err(PopulationEvidenceError::OutcomeCountExceedsPopulation);
                }
            }
            Self::PersonTimeMicros {
                person_time_micros, ..
            } => {
                if *person_time_micros == 0 {
                    return Err(PopulationEvidenceError::ZeroPopulationDenominator);
                }
            }
        }
        Ok(())
    }

    fn kind(&self) -> PopulationDenominatorKindV1 {
        match self {
            Self::PersonsAtRisk { .. } => PopulationDenominatorKindV1::PersonsAtRisk,
            Self::PersonTimeMicros { .. } => PopulationDenominatorKindV1::PersonTime,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PopulationDenominatorKindV1 {
    PersonsAtRisk,
    PersonTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PopulationCohortSummaryV1 {
    pub schema_version: u16,
    pub cohort_id: String,
    pub population_definition_digest: StoredDigest,
    pub exposure_definition_digest: StoredDigest,
    pub outcome_definition_digest: StoredDigest,
    pub source_dataset_digest: StoredDigest,
    pub index_time_definition_digest: StoredDigest,
    pub followup_definition_digest: StoredDigest,
    pub deduplication_policy_digest: StoredDigest,
    pub strata_definition_digest: Option<StoredDigest>,
    pub denominator: PopulationDenominatorV1,
}

impl PopulationCohortSummaryV1 {
    pub fn validate(&self) -> Result<(), PopulationEvidenceError> {
        require_version(self.schema_version, "population cohort")?;
        validate_id("cohort ID", &self.cohort_id)?;
        for digest in [
            self.population_definition_digest,
            self.exposure_definition_digest,
            self.outcome_definition_digest,
            self.source_dataset_digest,
            self.index_time_definition_digest,
            self.followup_definition_digest,
            self.deduplication_policy_digest,
        ] {
            require_shape(digest)?;
        }
        if let Some(digest) = self.strata_definition_digest {
            require_shape(digest)?;
        }
        self.denominator.validate()
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, PopulationEvidenceError> {
        self.validate()?;
        hash_json(DigestDomain::ClinicalPopulationCohort, COHORT_TAG, self)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PopulationAssociationMeasureV1 {
    RiskRatio,
    RiskDifference,
    OddsRatio,
    IncidenceRateRatio,
    OtherPreSpecifiedMeasure,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PopulationAssociationPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub require_same_outcome_definition: bool,
    pub require_same_followup_definition: bool,
    pub require_same_denominator_kind: bool,
}

impl PopulationAssociationPolicyV1 {
    pub fn strict_default(policy_id: impl Into<String>) -> Self {
        Self {
            schema_version: 1,
            policy_id: policy_id.into(),
            require_same_outcome_definition: true,
            require_same_followup_definition: true,
            require_same_denominator_kind: true,
        }
    }

    pub fn validate(&self) -> Result<(), PopulationEvidenceError> {
        require_version(self.schema_version, "association policy")?;
        validate_id("association policy ID", &self.policy_id)
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, PopulationEvidenceError> {
        self.validate()?;
        hash_json(
            DigestDomain::ClinicalPopulationAssociationPolicy,
            ASSOCIATION_POLICY_TAG,
            self,
        )
    }
}

/// Denominator-based association. This is not a causal-effect estimate.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DenominatorBasedAssociationV1 {
    pub schema_version: u16,
    pub association_id: String,
    pub exposed: PopulationCohortSummaryV1,
    pub comparator: PopulationCohortSummaryV1,
    pub measure: PopulationAssociationMeasureV1,
    pub statistical_method_digest: StoredDigest,
    pub exact_analysis_result_digest: StoredDigest,
    pub policy: PopulationAssociationPolicyV1,
    pub analyzed_at_micros: i64,
}

impl DenominatorBasedAssociationV1 {
    pub fn validate(&self) -> Result<(), PopulationEvidenceError> {
        require_version(self.schema_version, "population association")?;
        validate_id("association ID", &self.association_id)?;
        self.exposed.validate()?;
        self.comparator.validate()?;
        self.policy.validate()?;
        require_shape(self.statistical_method_digest)?;
        require_shape(self.exact_analysis_result_digest)?;

        let exposed_digest = self.exposed.verified_digest()?.stored();
        let comparator_digest = self.comparator.verified_digest()?.stored();
        if exposed_digest == comparator_digest {
            return Err(PopulationEvidenceError::IdenticalAssociationCohorts);
        }
        if self.policy.require_same_outcome_definition
            && self.exposed.outcome_definition_digest != self.comparator.outcome_definition_digest
        {
            return Err(PopulationEvidenceError::AssociationOutcomeDefinitionMismatch);
        }
        if self.policy.require_same_followup_definition
            && self.exposed.followup_definition_digest != self.comparator.followup_definition_digest
        {
            return Err(PopulationEvidenceError::AssociationFollowupDefinitionMismatch);
        }
        if self.policy.require_same_denominator_kind
            && self.exposed.denominator.kind() != self.comparator.denominator.kind()
        {
            return Err(PopulationEvidenceError::AssociationDenominatorKindMismatch);
        }
        match self.measure {
            PopulationAssociationMeasureV1::RiskRatio
            | PopulationAssociationMeasureV1::RiskDifference
            | PopulationAssociationMeasureV1::OddsRatio => {
                if self.exposed.denominator.kind() != PopulationDenominatorKindV1::PersonsAtRisk
                    || self.comparator.denominator.kind()
                        != PopulationDenominatorKindV1::PersonsAtRisk
                {
                    return Err(PopulationEvidenceError::MeasureIncompatibleWithDenominator);
                }
            }
            PopulationAssociationMeasureV1::IncidenceRateRatio => {
                if self.exposed.denominator.kind() != PopulationDenominatorKindV1::PersonTime
                    || self.comparator.denominator.kind() != PopulationDenominatorKindV1::PersonTime
                {
                    return Err(PopulationEvidenceError::MeasureIncompatibleWithDenominator);
                }
            }
            PopulationAssociationMeasureV1::OtherPreSpecifiedMeasure => {}
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, PopulationEvidenceError> {
        self.validate()?;
        hash_json(DigestDomain::ClinicalPopulationAssociation, ASSOCIATION_TAG, self)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum CausalStudyDesignV1 {
    RandomizedTrial,
    TargetTrialEmulation,
    PropensityScoreWeightedCohort,
    InstrumentalVariable,
    DifferenceInDifferences,
    InterruptedTimeSeries,
    RegressionDiscontinuity,
    OtherPreSpecifiedDesign,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CausalEffectPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub allowed_designs: Vec<CausalStudyDesignV1>,
    pub require_sensitivity_analysis: bool,
    pub require_negative_control_review: bool,
}

impl CausalEffectPolicyV1 {
    pub fn validate(&self) -> Result<(), PopulationEvidenceError> {
        require_version(self.schema_version, "causal effect policy")?;
        validate_id("causal effect policy ID", &self.policy_id)?;
        if self.allowed_designs.is_empty() || self.allowed_designs.len() > MAX_ALLOWED_DESIGNS {
            return Err(PopulationEvidenceError::InvalidAllowedDesignSet);
        }
        let unique: BTreeSet<_> = self.allowed_designs.iter().copied().collect();
        if unique.len() != self.allowed_designs.len() {
            return Err(PopulationEvidenceError::DuplicateAllowedDesign);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, PopulationEvidenceError> {
        self.validate()?;
        hash_json(DigestDomain::ClinicalCausalEffectPolicy, EFFECT_POLICY_TAG, self)
    }
}

/// Causal effect estimate under one explicit design/evidence package.
///
/// This is still an estimate, not causal truth. V1 intentionally requires the
/// association lineage plus explicit causal-design, confounding, diagnostics, and
/// estimator-result evidence. No API promotes an association automatically.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CausalEffectEstimateV1 {
    pub schema_version: u16,
    pub estimate_id: String,
    pub association_digest: StoredDigest,
    pub design: CausalStudyDesignV1,
    pub design_specification_digest: StoredDigest,
    pub confounder_strategy_digest: StoredDigest,
    pub diagnostics_digest: StoredDigest,
    pub exact_estimator_result_digest: StoredDigest,
    pub sensitivity_analysis_digest: Option<StoredDigest>,
    pub negative_control_review_digest: Option<StoredDigest>,
    pub policy: CausalEffectPolicyV1,
    pub estimated_at_micros: i64,
}

impl CausalEffectEstimateV1 {
    pub fn validate(&self) -> Result<(), PopulationEvidenceError> {
        require_version(self.schema_version, "causal effect estimate")?;
        validate_id("causal effect estimate ID", &self.estimate_id)?;
        require_domain(
            self.association_digest,
            DigestDomain::ClinicalPopulationAssociation,
        )?;
        for digest in [
            self.design_specification_digest,
            self.confounder_strategy_digest,
            self.diagnostics_digest,
            self.exact_estimator_result_digest,
        ] {
            require_shape(digest)?;
        }
        if let Some(digest) = self.sensitivity_analysis_digest {
            require_shape(digest)?;
        }
        if let Some(digest) = self.negative_control_review_digest {
            require_shape(digest)?;
        }
        self.policy.validate()?;
        if !self.policy.allowed_designs.contains(&self.design) {
            return Err(PopulationEvidenceError::CausalDesignNotAllowedByPolicy);
        }
        if self.policy.require_sensitivity_analysis && self.sensitivity_analysis_digest.is_none() {
            return Err(PopulationEvidenceError::MissingSensitivityAnalysis);
        }
        if self.policy.require_negative_control_review
            && self.negative_control_review_digest.is_none()
        {
            return Err(PopulationEvidenceError::MissingNegativeControlReview);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, PopulationEvidenceError> {
        self.validate()?;
        hash_json(DigestDomain::ClinicalCausalEffectEstimate, EFFECT_TAG, self)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReplicatedFindingKindV1 {
    SafetySignalCandidate,
    DenominatorBasedAssociation,
    CausalEffectEstimate,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReplicationConclusionV1 {
    Concordant,
    Mixed,
    Discordant,
    Indeterminate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PopulationReplicationV1 {
    pub schema_version: u16,
    pub replication_id: String,
    pub finding_kind: ReplicatedFindingKindV1,
    pub finding_digests: Vec<StoredDigest>,
    pub independence_evidence_digest: StoredDigest,
    pub replication_method_digest: StoredDigest,
    pub conclusion: ReplicationConclusionV1,
    pub assessed_at_micros: i64,
}

impl PopulationReplicationV1 {
    pub fn validate(&self) -> Result<(), PopulationEvidenceError> {
        require_version(self.schema_version, "population replication")?;
        validate_id("replication ID", &self.replication_id)?;
        if self.finding_digests.len() < 2
            || self.finding_digests.len() > MAX_REPLICATION_FINDINGS
        {
            return Err(PopulationEvidenceError::InvalidReplicationFindingCount);
        }
        let expected = match self.finding_kind {
            ReplicatedFindingKindV1::SafetySignalCandidate => {
                DigestDomain::ClinicalSafetySignalCandidate
            }
            ReplicatedFindingKindV1::DenominatorBasedAssociation => {
                DigestDomain::ClinicalPopulationAssociation
            }
            ReplicatedFindingKindV1::CausalEffectEstimate => {
                DigestDomain::ClinicalCausalEffectEstimate
            }
        };
        let mut unique = BTreeSet::new();
        for digest in &self.finding_digests {
            require_domain(*digest, expected)?;
            unique.insert(*digest);
        }
        if unique.len() != self.finding_digests.len() {
            return Err(PopulationEvidenceError::DuplicateReplicationFinding);
        }
        require_shape(self.independence_evidence_digest)?;
        require_shape(self.replication_method_digest)?;
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, PopulationEvidenceError> {
        self.validate()?;
        hash_json(DigestDomain::ClinicalPopulationReplication, REPLICATION_TAG, self)
    }
}

fn require_version(version: u16, label: &'static str) -> Result<(), PopulationEvidenceError> {
    if version != 1 {
        return Err(PopulationEvidenceError::UnsupportedVersion { label, version });
    }
    Ok(())
}

fn validate_id(label: &'static str, value: &str) -> Result<(), PopulationEvidenceError> {
    if value.trim().is_empty() {
        return Err(PopulationEvidenceError::MissingId(label));
    }
    if value.len() > MAX_ID_LEN {
        return Err(PopulationEvidenceError::OversizedId(label));
    }
    Ok(())
}

fn require_shape(digest: StoredDigest) -> Result<(), PopulationEvidenceError> {
    digest.validate_shape()?;
    Ok(())
}

fn require_domain(
    digest: StoredDigest,
    expected: DigestDomain,
) -> Result<(), PopulationEvidenceError> {
    digest.validate_shape()?;
    if digest.domain != expected {
        return Err(PopulationEvidenceError::WrongDigestDomain {
            expected,
            actual: digest.domain,
        });
    }
    Ok(())
}

fn hash_json<T: Serialize>(
    domain: DigestDomain,
    tag: &'static [u8],
    value: &T,
) -> Result<VerifiedDigest, PopulationEvidenceError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| PopulationEvidenceError::Serialization(error.to_string()))?;
    let mut framed = Vec::with_capacity(tag.len() + 1 + encoded.len());
    framed.extend_from_slice(tag);
    framed.push(0);
    framed.extend_from_slice(&encoded);
    Ok(hash_canonical_bytes(domain, &framed)?)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PopulationEvidenceError {
    #[error("unsupported {label} version {version}")]
    UnsupportedVersion { label: &'static str, version: u16 },
    #[error("{0} is required")]
    MissingId(&'static str),
    #[error("{0} exceeds the v1 identifier bound")]
    OversizedId(&'static str),
    #[error("signal policy minimum case count cannot be zero")]
    ZeroMinimumCaseCount,
    #[error("signal candidate has zero cases")]
    ZeroCaseCount,
    #[error("signal candidate does not satisfy its minimum case-count policy")]
    InsufficientCasesForSignalPolicy,
    #[error("signal candidate distinct-subject count is invalid")]
    InvalidDistinctSubjectCount,
    #[error("signal policy requires an explicit distinct-subject count")]
    MissingDistinctSubjectCount,
    #[error("population denominator cannot be zero")]
    ZeroPopulationDenominator,
    #[error("binary-outcome count exceeds observed population")]
    OutcomeCountExceedsPopulation,
    #[error("association exposed and comparator cohorts are identical")]
    IdenticalAssociationCohorts,
    #[error("association cohorts use different outcome definitions")]
    AssociationOutcomeDefinitionMismatch,
    #[error("association cohorts use different follow-up definitions")]
    AssociationFollowupDefinitionMismatch,
    #[error("association cohorts use incompatible denominator kinds")]
    AssociationDenominatorKindMismatch,
    #[error("association measure is incompatible with supplied denominator kind")]
    MeasureIncompatibleWithDenominator,
    #[error("causal effect policy allowed-design set is empty or too large")]
    InvalidAllowedDesignSet,
    #[error("causal effect policy contains duplicate allowed designs")]
    DuplicateAllowedDesign,
    #[error("causal design is not allowed by the bound policy")]
    CausalDesignNotAllowedByPolicy,
    #[error("causal effect policy requires sensitivity-analysis evidence")]
    MissingSensitivityAnalysis,
    #[error("causal effect policy requires negative-control review evidence")]
    MissingNegativeControlReview,
    #[error("replication must contain 2..={MAX_REPLICATION_FINDINGS} findings")]
    InvalidReplicationFindingCount,
    #[error("replication contains a duplicate finding digest")]
    DuplicateReplicationFinding,
    #[error("population evidence digest domain mismatch: expected {expected:?}, got {actual:?}")]
    WrongDigestDomain {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("population evidence serialization failed: {0}")]
    Serialization(String),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
}
