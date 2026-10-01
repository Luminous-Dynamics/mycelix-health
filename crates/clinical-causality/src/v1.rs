use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use mycelix_clinical_semantics::ClinicalFact;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

const OBSERVED_EVENT_TAG: &[u8] = b"mycelix-health/clinical-observed-event-v1";
const ASSOCIATION_TAG: &[u8] = b"mycelix-health/clinical-exposure-association-v1";
const POLICY_TAG: &[u8] = b"mycelix-health/clinical-causal-assessment-policy-v1";
const ASSESSMENT_TAG: &[u8] = b"mycelix-health/clinical-causal-assessment-v1";
const MAX_FACTS: usize = 256;
const MAX_FACTORS: usize = 128;
const MAX_MISSING: usize = 64;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservedClinicalEventV1 {
    pub schema_version: u16,
    pub event_id: String,
    pub patient_subject_binding_evidence_digest: StoredDigest,
    pub facts: Vec<ClinicalFact>,
    pub onset_micros: i64,
    pub end_micros: Option<i64>,
    pub recorded_at_micros: i64,
}

impl ObservedClinicalEventV1 {
    pub fn validate(&self) -> Result<(), CausalityError> {
        if self.schema_version != 1 {
            return Err(CausalityError::UnsupportedObservedEventVersion(self.schema_version));
        }
        if self.event_id.trim().is_empty() {
            return Err(CausalityError::MissingEventId);
        }
        require_domain(
            self.patient_subject_binding_evidence_digest,
            DigestDomain::PatientSubjectBindingEvidence,
        )?;
        if self.facts.is_empty() {
            return Err(CausalityError::MissingObservedFacts);
        }
        if self.facts.len() > MAX_FACTS {
            return Err(CausalityError::TooManyObservedFacts);
        }
        for fact in &self.facts {
            fact.validate_machine_actionable()
                .map_err(|error| CausalityError::InvalidClinicalFact(error.to_string()))?;
        }
        let first_subject = &self.facts[0].subject;
        if self.facts.iter().any(|fact| &fact.subject != first_subject) {
            return Err(CausalityError::MixedObservedSubjects);
        }
        if self.end_micros.is_some_and(|end| end < self.onset_micros) {
            return Err(CausalityError::ObservedEventEndsBeforeOnset);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, CausalityError> {
        self.validate()?;
        hash_json(DigestDomain::ClinicalObservedEvent, OBSERVED_EVENT_TAG, self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MedicationAdministrationExposureV1 {
    pub medication_artifact_digest: StoredDigest,
    pub administration_receipt_digest: StoredDigest,
    pub administration_occurrence_digest: StoredDigest,
    pub patient_subject_binding_evidence_digest: StoredDigest,
    pub exposure_start_micros: i64,
    pub exposure_end_micros: Option<i64>,
}

impl MedicationAdministrationExposureV1 {
    pub fn validate(&self) -> Result<(), CausalityError> {
        require_domain(
            self.medication_artifact_digest,
            DigestDomain::MedicationRequestArtifact,
        )?;
        require_domain(
            self.administration_receipt_digest,
            DigestDomain::MedicationAdministrationReceipt,
        )?;
        require_domain(
            self.administration_occurrence_digest,
            DigestDomain::MedicationAdministrationOccurrence,
        )?;
        require_domain(
            self.patient_subject_binding_evidence_digest,
            DigestDomain::PatientSubjectBindingEvidence,
        )?;
        if self
            .exposure_end_micros
            .is_some_and(|end| end < self.exposure_start_micros)
        {
            return Err(CausalityError::ExposureEndsBeforeStart);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TemporalRelationshipV1 {
    EventBeforeExposure,
    OverlapsExposure,
    EventAfterExposure { latency_micros: i64 },
    /// Timing data cannot prove before/overlap/after. Most commonly this means an
    /// event began before exposure but has no known resolution time.
    Indeterminate,
}

impl TemporalRelationshipV1 {
    pub fn permits_positive_relationship(self) -> bool {
        matches!(self, Self::OverlapsExposure | Self::EventAfterExposure { .. })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExposureAssociationV1 {
    pub schema_version: u16,
    pub observed_event_digest: StoredDigest,
    pub exposure: MedicationAdministrationExposureV1,
    pub temporal_relationship: TemporalRelationshipV1,
}

impl ExposureAssociationV1 {
    pub fn derive(
        observed: &ObservedClinicalEventV1,
        exposure: MedicationAdministrationExposureV1,
    ) -> Result<Self, CausalityError> {
        observed.validate()?;
        exposure.validate()?;
        if observed.patient_subject_binding_evidence_digest
            != exposure.patient_subject_binding_evidence_digest
        {
            return Err(CausalityError::SubjectBindingMismatch);
        }

        let exposure_end = exposure
            .exposure_end_micros
            .unwrap_or(exposure.exposure_start_micros);
        let temporal_relationship = match observed.end_micros {
            Some(observed_end) if observed_end < exposure.exposure_start_micros => {
                TemporalRelationshipV1::EventBeforeExposure
            }
            _ if observed.onset_micros > exposure_end => {
                TemporalRelationshipV1::EventAfterExposure {
                    latency_micros: observed.onset_micros.saturating_sub(exposure_end),
                }
            }
            Some(_) => TemporalRelationshipV1::OverlapsExposure,
            None if observed.onset_micros >= exposure.exposure_start_micros => {
                TemporalRelationshipV1::OverlapsExposure
            }
            None => TemporalRelationshipV1::Indeterminate,
        };

        let association = Self {
            schema_version: 1,
            observed_event_digest: observed.verified_digest()?.stored(),
            exposure,
            temporal_relationship,
        };
        association.validate()?;
        Ok(association)
    }

    pub fn validate(&self) -> Result<(), CausalityError> {
        if self.schema_version != 1 {
            return Err(CausalityError::UnsupportedAssociationVersion(self.schema_version));
        }
        require_domain(
            self.observed_event_digest,
            DigestDomain::ClinicalObservedEvent,
        )?;
        self.exposure.validate()?;
        if let TemporalRelationshipV1::EventAfterExposure { latency_micros } =
            self.temporal_relationship
        {
            if latency_micros <= 0 {
                return Err(CausalityError::InvalidPositiveLatency);
            }
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, CausalityError> {
        self.validate()?;
        hash_json(
            DigestDomain::ClinicalExposureAssociation,
            ASSOCIATION_TAG,
            self,
        )
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum CausalEvidenceKindV1 {
    TemporalPlausibility,
    ObjectiveConfirmation,
    Dechallenge,
    Rechallenge,
    DoseResponse,
    KnownMechanism,
    PriorCaseOrLiterature,
    AlternativeEtiologyReview,
    ConcomitantExposureReview,
    BaselineComparison,
    AggregateSignal,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvidenceDirectionV1 {
    SupportsRelationship,
    ChallengesRelationship,
    Mixed,
    NoFinding,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CausalEvidenceItemV1 {
    pub kind: CausalEvidenceKindV1,
    pub direction: EvidenceDirectionV1,
    /// Exact evidence artifacts supporting this factor. `Unknown` alone may omit
    /// artifact evidence; even a `NoFinding` means a review occurred and must name
    /// the evidence proving that review.
    pub evidence_digests: Vec<StoredDigest>,
}

impl CausalEvidenceItemV1 {
    fn validate(&self) -> Result<(), CausalityError> {
        if self.evidence_digests.len() > MAX_FACTS {
            return Err(CausalityError::TooManyEvidenceDigests);
        }
        for digest in &self.evidence_digests {
            digest.validate_shape()?;
        }
        if self.direction != EvidenceDirectionV1::Unknown && self.evidence_digests.is_empty() {
            return Err(CausalityError::EvidenceClaimWithoutArtifact);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CausalConclusionV1 {
    Indeterminate,
    TemporalAssociationOnly,
    EvidenceSuggestsRelationship,
    EvidenceSupportsRelationship,
    EvidenceAgainstRelationship,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExternalCausalityScaleLabelV1 {
    pub system: String,
    pub version: String,
    pub label: String,
}

impl ExternalCausalityScaleLabelV1 {
    fn validate(&self) -> Result<(), CausalityError> {
        if self.system.trim().is_empty()
            || self.version.trim().is_empty()
            || self.label.trim().is_empty()
        {
            return Err(CausalityError::IncompleteExternalScaleLabel);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CausalAssessmentPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub required_review_kinds_for_positive: Vec<CausalEvidenceKindV1>,
    pub min_distinct_non_temporal_support_for_suggests: u8,
    pub min_distinct_non_temporal_support_for_supports: u8,
}

impl CausalAssessmentPolicyV1 {
    pub fn strict_default(policy_id: impl Into<String>) -> Self {
        Self {
            schema_version: 1,
            policy_id: policy_id.into(),
            required_review_kinds_for_positive: vec![
                CausalEvidenceKindV1::AlternativeEtiologyReview,
                CausalEvidenceKindV1::ConcomitantExposureReview,
            ],
            min_distinct_non_temporal_support_for_suggests: 1,
            min_distinct_non_temporal_support_for_supports: 2,
        }
    }

    pub fn validate(&self) -> Result<(), CausalityError> {
        if self.schema_version != 1 {
            return Err(CausalityError::UnsupportedPolicyVersion(self.schema_version));
        }
        if self.policy_id.trim().is_empty() {
            return Err(CausalityError::MissingPolicyId);
        }
        if self.min_distinct_non_temporal_support_for_suggests == 0 {
            return Err(CausalityError::PositiveThresholdCannotBeZero);
        }
        if self.min_distinct_non_temporal_support_for_supports
            < self.min_distinct_non_temporal_support_for_suggests
        {
            return Err(CausalityError::SupportThresholdBelowSuggestThreshold);
        }
        let unique: BTreeSet<_> = self
            .required_review_kinds_for_positive
            .iter()
            .copied()
            .collect();
        if unique.len() != self.required_review_kinds_for_positive.len() {
            return Err(CausalityError::DuplicateRequiredReviewKind);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, CausalityError> {
        self.validate()?;
        hash_json(
            DigestDomain::ClinicalCausalAssessmentPolicy,
            POLICY_TAG,
            self,
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MissingCausalEvidenceV1 {
    pub kind: CausalEvidenceKindV1,
    pub reason_code: String,
}

impl MissingCausalEvidenceV1 {
    fn validate(&self) -> Result<(), CausalityError> {
        if self.reason_code.trim().is_empty() {
            return Err(CausalityError::MissingEvidenceReasonCode);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CausalAssessmentV1 {
    pub schema_version: u16,
    pub association_digest: StoredDigest,
    pub policy_digest: StoredDigest,
    pub conclusion: CausalConclusionV1,
    pub evidence: Vec<CausalEvidenceItemV1>,
    pub missing_evidence: Vec<MissingCausalEvidenceV1>,
    pub assessment_method: String,
    pub assessment_method_version: String,
    pub external_scale_label: Option<ExternalCausalityScaleLabelV1>,
    /// Optional calibrated confidence. Absence means no calibrated probability is claimed.
    pub calibrated_confidence: Option<f64>,
    pub assessed_at_micros: i64,
}

impl CausalAssessmentV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        association: &ExposureAssociationV1,
        policy: &CausalAssessmentPolicyV1,
        conclusion: CausalConclusionV1,
        evidence: Vec<CausalEvidenceItemV1>,
        missing_evidence: Vec<MissingCausalEvidenceV1>,
        assessment_method: impl Into<String>,
        assessment_method_version: impl Into<String>,
        external_scale_label: Option<ExternalCausalityScaleLabelV1>,
        calibrated_confidence: Option<f64>,
        assessed_at_micros: i64,
    ) -> Result<Self, CausalityError> {
        association.validate()?;
        policy.validate()?;
        let assessment = Self {
            schema_version: 1,
            association_digest: association.verified_digest()?.stored(),
            policy_digest: policy.verified_digest()?.stored(),
            conclusion,
            evidence,
            missing_evidence,
            assessment_method: assessment_method.into(),
            assessment_method_version: assessment_method_version.into(),
            external_scale_label,
            calibrated_confidence,
            assessed_at_micros,
        };
        assessment.validate_against(association, policy)?;
        Ok(assessment)
    }

    pub fn validate_against(
        &self,
        association: &ExposureAssociationV1,
        policy: &CausalAssessmentPolicyV1,
    ) -> Result<(), CausalityError> {
        if self.schema_version != 1 {
            return Err(CausalityError::UnsupportedAssessmentVersion(self.schema_version));
        }
        if self.association_digest != association.verified_digest()?.stored() {
            return Err(CausalityError::AssociationDigestMismatch);
        }
        if self.policy_digest != policy.verified_digest()?.stored() {
            return Err(CausalityError::PolicyDigestMismatch);
        }
        if self.assessment_method.trim().is_empty()
            || self.assessment_method_version.trim().is_empty()
        {
            return Err(CausalityError::MissingAssessmentMethod);
        }
        if let Some(label) = &self.external_scale_label {
            label.validate()?;
        }
        if let Some(confidence) = self.calibrated_confidence {
            if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
                return Err(CausalityError::InvalidCalibratedConfidence);
            }
        }
        if self.evidence.len() > MAX_FACTORS {
            return Err(CausalityError::TooManyCausalFactors);
        }
        if self.missing_evidence.len() > MAX_MISSING {
            return Err(CausalityError::TooManyMissingEvidenceItems);
        }
        for item in &self.evidence {
            item.validate()?;
        }
        for missing in &self.missing_evidence {
            missing.validate()?;
        }

        let missing_kinds: BTreeSet<_> = self.missing_evidence.iter().map(|item| item.kind).collect();
        if missing_kinds.len() != self.missing_evidence.len() {
            return Err(CausalityError::DuplicateMissingEvidenceKind);
        }
        let reviewed_kinds: BTreeSet<_> = self
            .evidence
            .iter()
            .filter(|item| item.direction != EvidenceDirectionV1::Unknown)
            .map(|item| item.kind)
            .collect();
        let required_review_missing = policy
            .required_review_kinds_for_positive
            .iter()
            .any(|kind| !reviewed_kinds.contains(kind) || missing_kinds.contains(kind));

        let supporting_non_temporal: BTreeSet<_> = self
            .evidence
            .iter()
            .filter(|item| {
                item.kind != CausalEvidenceKindV1::TemporalPlausibility
                    && item.direction == EvidenceDirectionV1::SupportsRelationship
            })
            .map(|item| item.kind)
            .collect();
        let challenging_non_temporal: BTreeSet<_> = self
            .evidence
            .iter()
            .filter(|item| {
                item.kind != CausalEvidenceKindV1::TemporalPlausibility
                    && item.direction == EvidenceDirectionV1::ChallengesRelationship
            })
            .map(|item| item.kind)
            .collect();
        let has_non_temporal_directional = self.evidence.iter().any(|item| {
            item.kind != CausalEvidenceKindV1::TemporalPlausibility
                && matches!(
                    item.direction,
                    EvidenceDirectionV1::SupportsRelationship
                        | EvidenceDirectionV1::ChallengesRelationship
                        | EvidenceDirectionV1::Mixed
                )
        });

        match self.conclusion {
            CausalConclusionV1::Indeterminate => {}
            CausalConclusionV1::TemporalAssociationOnly => {
                ensure_temporal_association(association)?;
                if has_non_temporal_directional {
                    return Err(CausalityError::TemporalOnlyHasNonTemporalDirectionalEvidence);
                }
            }
            CausalConclusionV1::EvidenceSuggestsRelationship => {
                ensure_positive_temporality(association)?;
                if required_review_missing || !self.missing_evidence.is_empty() {
                    return Err(CausalityError::PositiveConclusionWithMissingRequiredEvidence);
                }
                if supporting_non_temporal.len()
                    < usize::from(policy.min_distinct_non_temporal_support_for_suggests)
                {
                    return Err(CausalityError::InsufficientDistinctSupportingEvidence);
                }
            }
            CausalConclusionV1::EvidenceSupportsRelationship => {
                ensure_positive_temporality(association)?;
                if required_review_missing || !self.missing_evidence.is_empty() {
                    return Err(CausalityError::PositiveConclusionWithMissingRequiredEvidence);
                }
                if supporting_non_temporal.len()
                    < usize::from(policy.min_distinct_non_temporal_support_for_supports)
                {
                    return Err(CausalityError::InsufficientDistinctSupportingEvidence);
                }
            }
            CausalConclusionV1::EvidenceAgainstRelationship => {
                if challenging_non_temporal.is_empty() {
                    return Err(CausalityError::NegativeConclusionWithoutChallengingEvidence);
                }
                if required_review_missing || !self.missing_evidence.is_empty() {
                    return Err(CausalityError::NegativeConclusionWithMissingRequiredEvidence);
                }
            }
        }
        Ok(())
    }

    pub fn verified_digest(
        &self,
        association: &ExposureAssociationV1,
        policy: &CausalAssessmentPolicyV1,
    ) -> Result<VerifiedDigest, CausalityError> {
        self.validate_against(association, policy)?;
        hash_json(
            DigestDomain::ClinicalCausalAssessment,
            ASSESSMENT_TAG,
            self,
        )
    }

    pub fn has_mixed_directional_evidence(&self) -> bool {
        let has_support = self
            .evidence
            .iter()
            .any(|item| item.direction == EvidenceDirectionV1::SupportsRelationship);
        let has_challenge = self
            .evidence
            .iter()
            .any(|item| item.direction == EvidenceDirectionV1::ChallengesRelationship);
        has_support && has_challenge
    }
}

fn ensure_temporal_association(association: &ExposureAssociationV1) -> Result<(), CausalityError> {
    match association.temporal_relationship {
        TemporalRelationshipV1::EventBeforeExposure => {
            Err(CausalityError::TemporalAssociationConclusionPredatesExposure)
        }
        TemporalRelationshipV1::Indeterminate => {
            Err(CausalityError::TemporalAssociationConclusionIndeterminate)
        }
        TemporalRelationshipV1::OverlapsExposure
        | TemporalRelationshipV1::EventAfterExposure { .. } => Ok(()),
    }
}

fn ensure_positive_temporality(association: &ExposureAssociationV1) -> Result<(), CausalityError> {
    match association.temporal_relationship {
        TemporalRelationshipV1::EventBeforeExposure => {
            Err(CausalityError::PositiveConclusionPredatesExposure)
        }
        TemporalRelationshipV1::Indeterminate => {
            Err(CausalityError::PositiveConclusionWithIndeterminateTemporality)
        }
        TemporalRelationshipV1::OverlapsExposure
        | TemporalRelationshipV1::EventAfterExposure { .. } => Ok(()),
    }
}

fn require_domain(digest: StoredDigest, expected: DigestDomain) -> Result<(), CausalityError> {
    digest.validate_shape()?;
    if digest.domain != expected {
        return Err(CausalityError::WrongDigestDomain {
            expected,
            actual: digest.domain,
        });
    }
    Ok(())
}

fn hash_json<T: Serialize>(
    domain: DigestDomain,
    tag: &[u8],
    value: &T,
) -> Result<VerifiedDigest, CausalityError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| CausalityError::Serialization(error.to_string()))?;
    let mut framed = Vec::with_capacity(tag.len() + 1 + encoded.len());
    framed.extend_from_slice(tag);
    framed.push(0);
    framed.extend_from_slice(&encoded);
    Ok(hash_canonical_bytes(domain, &framed)?)
}

#[derive(Debug, Error, PartialEq)]
pub enum CausalityError {
    #[error("unsupported observed-event version {0}")]
    UnsupportedObservedEventVersion(u16),
    #[error("unsupported exposure-association version {0}")]
    UnsupportedAssociationVersion(u16),
    #[error("unsupported causal-assessment policy version {0}")]
    UnsupportedPolicyVersion(u16),
    #[error("unsupported causal-assessment version {0}")]
    UnsupportedAssessmentVersion(u16),
    #[error("observed event ID is required")]
    MissingEventId,
    #[error("observed event requires at least one machine-actionable clinical fact")]
    MissingObservedFacts,
    #[error("observed event contains too many clinical facts")]
    TooManyObservedFacts,
    #[error("observed event mixes multiple clinical subjects")]
    MixedObservedSubjects,
    #[error("observed event ends before onset")]
    ObservedEventEndsBeforeOnset,
    #[error("exposure ends before it starts")]
    ExposureEndsBeforeStart,
    #[error("observed event and exposure patient-subject bindings differ")]
    SubjectBindingMismatch,
    #[error("event-after-exposure latency must be positive")]
    InvalidPositiveLatency,
    #[error("causal evidence item contains too many evidence digests")]
    TooManyEvidenceDigests,
    #[error("causal evidence claim requires at least one evidence artifact unless state is Unknown")]
    EvidenceClaimWithoutArtifact,
    #[error("external causality scale label requires system, version, and label")]
    IncompleteExternalScaleLabel,
    #[error("causal assessment policy ID is required")]
    MissingPolicyId,
    #[error("positive causal support threshold cannot be zero")]
    PositiveThresholdCannotBeZero,
    #[error("strong-support threshold cannot be below suggests-relationship threshold")]
    SupportThresholdBelowSuggestThreshold,
    #[error("required causal review kind is duplicated")]
    DuplicateRequiredReviewKind,
    #[error("missing causal evidence kind is duplicated")]
    DuplicateMissingEvidenceKind,
    #[error("missing-evidence reason code is required")]
    MissingEvidenceReasonCode,
    #[error("causal assessment method and version are required")]
    MissingAssessmentMethod,
    #[error("calibrated confidence must be finite and between 0 and 1")]
    InvalidCalibratedConfidence,
    #[error("causal assessment contains too many factors")]
    TooManyCausalFactors,
    #[error("causal assessment contains too many missing-evidence items")]
    TooManyMissingEvidenceItems,
    #[error("causal assessment association digest does not match supplied association")]
    AssociationDigestMismatch,
    #[error("causal assessment policy digest does not match supplied policy")]
    PolicyDigestMismatch,
    #[error("positive relationship conclusion is incompatible with an event that predates exposure")]
    PositiveConclusionPredatesExposure,
    #[error("positive relationship conclusion is incompatible with indeterminate temporal ordering")]
    PositiveConclusionWithIndeterminateTemporality,
    #[error("temporal-association-only conclusion is incompatible with an event that predates exposure")]
    TemporalAssociationConclusionPredatesExposure,
    #[error("temporal-association-only conclusion requires established overlap/after-exposure timing")]
    TemporalAssociationConclusionIndeterminate,
    #[error("temporal-association-only conclusion contains non-temporal directional evidence")]
    TemporalOnlyHasNonTemporalDirectionalEvidence,
    #[error("positive relationship conclusion has missing policy-required evidence")]
    PositiveConclusionWithMissingRequiredEvidence,
    #[error("positive relationship conclusion lacks enough distinct non-temporal support kinds")]
    InsufficientDistinctSupportingEvidence,
    #[error("negative relationship conclusion lacks non-temporal challenging evidence")]
    NegativeConclusionWithoutChallengingEvidence,
    #[error("negative relationship conclusion has missing policy-required evidence")]
    NegativeConclusionWithMissingRequiredEvidence,
    #[error("clinical fact is invalid: {0}")]
    InvalidClinicalFact(String),
    #[error("digest domain mismatch: expected {expected:?}, got {actual:?}")]
    WrongDigestDomain {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("causal artifact serialization failed: {0}")]
    Serialization(String),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
}
