#![deny(unsafe_code)]
//! Symthaea-native structural distribution/OOD assessment.
//!
//! The legacy Mycelix distribution assessment binds an OOD result to a
//! `ClinicalEvidenceCapsule`. That is intentionally not reused for Symthaea v2
//! probabilistic model assertions because forcing a prediction into the legacy
//! capsule would reintroduce an implicit `EvaluationState` conversion.
//!
//! This crate instead binds one distribution assessment directly to the exact
//! verified Symthaea model assertion and its upstream wire/context/admission
//! identities. The resulting validated value is structural evidence only; it is
//! not evaluator trust, runtime currentness, clinical utility, or presentation
//! authority.

use mycelix_symthaea_clinical_model_assertion::VerifiedSymthaeaModelAssertionV1;
use mycelix_symthaea_clinical_wire_v2::{
    SymthaeaDigestAlgorithmV2, SymthaeaEvidenceIdentityV2, SymthaeaModelIdentityV2,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const SYMTHAEA_DISTRIBUTION_ASSESSMENT_V2_VERSION: u16 = 1;
pub const SYMTHAEA_DISTRIBUTION_POLICY_V2_VERSION: u16 = 1;
pub const SYMTHAEA_DISTRIBUTION_IDENTITY_VERSION: u16 = 1;

const MAX_TOKEN_BYTES: usize = 4096;
const MAX_REFERENCE_BYTES: usize = 16 * 1024;
const MAX_ASSESSMENT_AGE_MICROS: i64 = 31_536_000_000_000; // 365 days
const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000; // five minutes
const MODEL_DERIVE_KEY: &str = "mycelix.health.symthaea-model-identity.v1";
const POLICY_DERIVE_KEY: &str = "mycelix.health.symthaea-distribution-policy-v2.v1";
const ASSESSMENT_DERIVE_KEY: &str = "mycelix.health.symthaea-distribution-assessment-v2.v1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SymthaeaModelIdentityDigestV1([u8; 32]);

impl SymthaeaModelIdentityDigestV1 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DistributionArtifactIdentityV2 {
    pub identity_version: u16,
    pub name: String,
    pub version: String,
    pub digest: [u8; 32],
}

impl DistributionArtifactIdentityV2 {
    pub fn validate(&self) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
        if self.identity_version != SYMTHAEA_DISTRIBUTION_IDENTITY_VERSION {
            return Err(SymthaeaDistributionAssessmentV2Error::UnsupportedIdentityVersion(
                self.identity_version,
            ));
        }
        validate_token(&self.name)?;
        validate_token(&self.version)?;
        validate_nonzero_digest(&self.digest)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DistributionEvidenceIdentityV2 {
    pub identity_version: u16,
    pub namespace: String,
    pub artifact_id: String,
    pub digest: [u8; 32],
}

impl DistributionEvidenceIdentityV2 {
    pub fn validate(&self) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
        if self.identity_version != SYMTHAEA_DISTRIBUTION_IDENTITY_VERSION {
            return Err(SymthaeaDistributionAssessmentV2Error::UnsupportedIdentityVersion(
                self.identity_version,
            ));
        }
        validate_token(&self.namespace)?;
        validate_token(&self.artifact_id)?;
        validate_nonzero_digest(&self.digest)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SymthaeaDistributionAssessmentStatusV2 {
    NotRun,
    Unavailable,
    Indeterminate,
    InDistribution,
    OutOfDistribution,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SymthaeaDistributionDirectionV2 {
    HigherMeansMoreInDistribution,
    LowerMeansMoreInDistribution,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct SymthaeaDistributionNumericBoundaryV2 {
    pub score: f64,
    pub threshold: f64,
    pub direction: SymthaeaDistributionDirectionV2,
}

impl SymthaeaDistributionNumericBoundaryV2 {
    fn validate(&self) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
        if !self.score.is_finite() || !self.threshold.is_finite() {
            return Err(SymthaeaDistributionAssessmentV2Error::InvalidNumericBoundary);
        }
        Ok(())
    }

    fn implies_in_distribution(&self) -> bool {
        match self.direction {
            SymthaeaDistributionDirectionV2::HigherMeansMoreInDistribution => {
                self.score >= self.threshold
            }
            SymthaeaDistributionDirectionV2::LowerMeansMoreInDistribution => {
                self.score <= self.threshold
            }
        }
    }
}

/// Serializable detector result bound to the exact Symthaea model assertion.
///
/// All four upstream identities are carried explicitly even though the assertion
/// digest already commits to them. The redundancy is intentional: downstream
/// trust adapters can compare each boundary directly and reject substitution
/// without reconstructing hidden producer state.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SymthaeaClinicalDistributionAssessmentV2 {
    pub schema_version: u16,
    pub assessment_id: String,
    pub assertion_digest: [u8; 32],
    pub wire_digest: [u8; 32],
    pub evidence_context_digest: [u8; 32],
    pub admission_policy_digest: [u8; 32],
    pub subject_id: String,
    pub model_identity_digest: [u8; 32],
    pub detector: DistributionArtifactIdentityV2,
    pub reference_population: String,
    pub reference_domain: DistributionEvidenceIdentityV2,
    pub status: SymthaeaDistributionAssessmentStatusV2,
    pub numeric_boundary: Option<SymthaeaDistributionNumericBoundaryV2>,
    pub assessment_evidence: Option<DistributionEvidenceIdentityV2>,
    pub assessed_at_micros: i64,
}

impl SymthaeaClinicalDistributionAssessmentV2 {
    pub fn validate(&self) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
        if self.schema_version != SYMTHAEA_DISTRIBUTION_ASSESSMENT_V2_VERSION {
            return Err(SymthaeaDistributionAssessmentV2Error::UnsupportedAssessmentVersion(
                self.schema_version,
            ));
        }
        validate_token(&self.assessment_id)?;
        validate_nonzero_digest(&self.assertion_digest)?;
        validate_nonzero_digest(&self.wire_digest)?;
        validate_nonzero_digest(&self.evidence_context_digest)?;
        validate_nonzero_digest(&self.admission_policy_digest)?;
        validate_token(&self.subject_id)?;
        validate_nonzero_digest(&self.model_identity_digest)?;
        self.detector.validate()?;
        validate_reference(&self.reference_population)?;
        self.reference_domain.validate()?;
        if self.assessed_at_micros <= 0 {
            return Err(SymthaeaDistributionAssessmentV2Error::InvalidAssessmentTime);
        }

        if let Some(boundary) = &self.numeric_boundary {
            boundary.validate()?;
            if matches!(
                self.status,
                SymthaeaDistributionAssessmentStatusV2::InDistribution
                    | SymthaeaDistributionAssessmentStatusV2::OutOfDistribution
            ) {
                let implied_in = boundary.implies_in_distribution();
                if (self.status == SymthaeaDistributionAssessmentStatusV2::InDistribution
                    && !implied_in)
                    || (self.status
                        == SymthaeaDistributionAssessmentStatusV2::OutOfDistribution
                        && implied_in)
                {
                    return Err(SymthaeaDistributionAssessmentV2Error::NumericStatusMismatch);
                }
            }
        }

        let detector_produced = matches!(
            self.status,
            SymthaeaDistributionAssessmentStatusV2::Indeterminate
                | SymthaeaDistributionAssessmentStatusV2::InDistribution
                | SymthaeaDistributionAssessmentStatusV2::OutOfDistribution
        );
        if detector_produced && self.assessment_evidence.is_none() {
            return Err(SymthaeaDistributionAssessmentV2Error::MissingAssessmentEvidence);
        }
        if let Some(evidence) = &self.assessment_evidence {
            evidence.validate()?;
        }
        Ok(())
    }

    pub fn digest(
        &self,
    ) -> Result<SymthaeaDistributionAssessmentDigestV2, SymthaeaDistributionAssessmentV2Error>
    {
        self.validate()?;
        let mut framed = Vec::new();
        framed.extend_from_slice(&self.schema_version.to_be_bytes());
        append_string(&mut framed, &self.assessment_id)?;
        framed.extend_from_slice(&self.assertion_digest);
        framed.extend_from_slice(&self.wire_digest);
        framed.extend_from_slice(&self.evidence_context_digest);
        framed.extend_from_slice(&self.admission_policy_digest);
        append_string(&mut framed, &self.subject_id)?;
        framed.extend_from_slice(&self.model_identity_digest);
        append_distribution_artifact(&mut framed, &self.detector)?;
        append_string(&mut framed, &self.reference_population)?;
        append_distribution_evidence(&mut framed, &self.reference_domain)?;
        framed.push(status_tag(self.status));
        append_numeric_boundary(&mut framed, self.numeric_boundary.as_ref());
        append_optional_distribution_evidence(&mut framed, self.assessment_evidence.as_ref())?;
        framed.extend_from_slice(&self.assessed_at_micros.to_be_bytes());
        Ok(SymthaeaDistributionAssessmentDigestV2(domain_hash(
            ASSESSMENT_DERIVE_KEY,
            &framed,
        )))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SymthaeaClinicalDistributionPolicyV2 {
    pub schema_version: u16,
    pub policy_id: String,
    pub required_detector: DistributionArtifactIdentityV2,
    pub required_reference_population: String,
    pub required_reference_domain: DistributionEvidenceIdentityV2,
    pub max_age_micros: i64,
    pub max_future_skew_micros: i64,
}

impl SymthaeaClinicalDistributionPolicyV2 {
    pub fn validate(&self) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
        if self.schema_version != SYMTHAEA_DISTRIBUTION_POLICY_V2_VERSION {
            return Err(SymthaeaDistributionAssessmentV2Error::UnsupportedPolicyVersion(
                self.schema_version,
            ));
        }
        validate_token(&self.policy_id)?;
        self.required_detector.validate()?;
        validate_reference(&self.required_reference_population)?;
        self.required_reference_domain.validate()?;
        if self.max_age_micros <= 0 || self.max_age_micros > MAX_ASSESSMENT_AGE_MICROS {
            return Err(SymthaeaDistributionAssessmentV2Error::InvalidMaximumAge);
        }
        if self.max_future_skew_micros < 0
            || self.max_future_skew_micros > MAX_FUTURE_SKEW_MICROS
        {
            return Err(SymthaeaDistributionAssessmentV2Error::InvalidMaximumFutureSkew);
        }
        Ok(())
    }

    pub fn digest(
        &self,
    ) -> Result<SymthaeaDistributionPolicyDigestV2, SymthaeaDistributionAssessmentV2Error> {
        self.validate()?;
        let mut framed = Vec::new();
        framed.extend_from_slice(&self.schema_version.to_be_bytes());
        append_string(&mut framed, &self.policy_id)?;
        append_distribution_artifact(&mut framed, &self.required_detector)?;
        append_string(&mut framed, &self.required_reference_population)?;
        append_distribution_evidence(&mut framed, &self.required_reference_domain)?;
        framed.extend_from_slice(&self.max_age_micros.to_be_bytes());
        framed.extend_from_slice(&self.max_future_skew_micros.to_be_bytes());
        Ok(SymthaeaDistributionPolicyDigestV2(domain_hash(
            POLICY_DERIVE_KEY,
            &framed,
        )))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymthaeaDistributionAssessmentDigestV2([u8; 32]);

impl SymthaeaDistributionAssessmentDigestV2 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymthaeaDistributionPolicyDigestV2([u8; 32]);

impl SymthaeaDistributionPolicyDigestV2 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Non-serializable structural proof for one exact Symthaea assertion.
pub struct ValidatedSymthaeaClinicalDistributionAssessmentV2 {
    assertion_digest: [u8; 32],
    wire_digest: [u8; 32],
    evidence_context_digest: [u8; 32],
    admission_policy_digest: [u8; 32],
    subject_id: String,
    model_identity_digest: SymthaeaModelIdentityDigestV1,
    detector: DistributionArtifactIdentityV2,
    reference_population: String,
    reference_domain: DistributionEvidenceIdentityV2,
    status: SymthaeaDistributionAssessmentStatusV2,
    assessment_digest: SymthaeaDistributionAssessmentDigestV2,
    policy_digest: SymthaeaDistributionPolicyDigestV2,
    assessed_at_micros: i64,
}

impl ValidatedSymthaeaClinicalDistributionAssessmentV2 {
    #[must_use]
    pub const fn assertion_digest(&self) -> &[u8; 32] {
        &self.assertion_digest
    }

    #[must_use]
    pub const fn wire_digest(&self) -> &[u8; 32] {
        &self.wire_digest
    }

    #[must_use]
    pub const fn evidence_context_digest(&self) -> &[u8; 32] {
        &self.evidence_context_digest
    }

    #[must_use]
    pub const fn admission_policy_digest(&self) -> &[u8; 32] {
        &self.admission_policy_digest
    }

    pub fn subject_id(&self) -> &str {
        &self.subject_id
    }

    #[must_use]
    pub const fn model_identity_digest(&self) -> SymthaeaModelIdentityDigestV1 {
        self.model_identity_digest
    }

    pub fn detector(&self) -> &DistributionArtifactIdentityV2 {
        &self.detector
    }

    pub fn reference_population(&self) -> &str {
        &self.reference_population
    }

    pub fn reference_domain(&self) -> &DistributionEvidenceIdentityV2 {
        &self.reference_domain
    }

    #[must_use]
    pub const fn status(&self) -> SymthaeaDistributionAssessmentStatusV2 {
        self.status
    }

    #[must_use]
    pub const fn assessment_digest(&self) -> SymthaeaDistributionAssessmentDigestV2 {
        self.assessment_digest
    }

    #[must_use]
    pub const fn policy_digest(&self) -> SymthaeaDistributionPolicyDigestV2 {
        self.policy_digest
    }

    #[must_use]
    pub const fn assessed_at_micros(&self) -> i64 {
        self.assessed_at_micros
    }
}

pub fn validate_symthaea_distribution_assessment_v2(
    assertion: &VerifiedSymthaeaModelAssertionV1,
    assessment: &SymthaeaClinicalDistributionAssessmentV2,
    policy: &SymthaeaClinicalDistributionPolicyV2,
    now_micros: i64,
) -> Result<ValidatedSymthaeaClinicalDistributionAssessmentV2, SymthaeaDistributionAssessmentV2Error>
{
    assessment.validate()?;
    policy.validate()?;
    if now_micros <= 0 {
        return Err(SymthaeaDistributionAssessmentV2Error::InvalidCurrentTime);
    }

    if assessment.assertion_digest != *assertion.assertion_digest().as_bytes() {
        return Err(SymthaeaDistributionAssessmentV2Error::AssertionDigestMismatch);
    }
    if assessment.wire_digest != *assertion.wire_digest().as_bytes() {
        return Err(SymthaeaDistributionAssessmentV2Error::WireDigestMismatch);
    }
    if assessment.evidence_context_digest != *assertion.context_digest().as_bytes() {
        return Err(SymthaeaDistributionAssessmentV2Error::EvidenceContextDigestMismatch);
    }
    if assessment.admission_policy_digest != *assertion.admission_policy_digest().as_bytes() {
        return Err(SymthaeaDistributionAssessmentV2Error::AdmissionPolicyDigestMismatch);
    }
    if assessment.subject_id != assertion.subject_id() {
        return Err(SymthaeaDistributionAssessmentV2Error::SubjectMismatch);
    }

    let expected_model = symthaea_model_identity_digest_v1(assertion.model())?;
    if assessment.model_identity_digest != *expected_model.as_bytes() {
        return Err(SymthaeaDistributionAssessmentV2Error::ModelIdentityMismatch);
    }
    if assessment.detector != policy.required_detector {
        return Err(SymthaeaDistributionAssessmentV2Error::DetectorMismatch);
    }
    if assessment.reference_population != policy.required_reference_population {
        return Err(SymthaeaDistributionAssessmentV2Error::ReferencePopulationMismatch);
    }
    if assessment.reference_domain != policy.required_reference_domain {
        return Err(SymthaeaDistributionAssessmentV2Error::ReferenceDomainMismatch);
    }

    if assessment.assessed_at_micros > now_micros {
        let skew = assessment.assessed_at_micros.saturating_sub(now_micros);
        if skew > policy.max_future_skew_micros {
            return Err(SymthaeaDistributionAssessmentV2Error::AssessmentFromFuture);
        }
    } else {
        let age = now_micros.saturating_sub(assessment.assessed_at_micros);
        if age > policy.max_age_micros {
            return Err(SymthaeaDistributionAssessmentV2Error::StaleAssessment);
        }
    }

    Ok(ValidatedSymthaeaClinicalDistributionAssessmentV2 {
        assertion_digest: assessment.assertion_digest,
        wire_digest: assessment.wire_digest,
        evidence_context_digest: assessment.evidence_context_digest,
        admission_policy_digest: assessment.admission_policy_digest,
        subject_id: assessment.subject_id.clone(),
        model_identity_digest: expected_model,
        detector: assessment.detector.clone(),
        reference_population: assessment.reference_population.clone(),
        reference_domain: assessment.reference_domain.clone(),
        status: assessment.status,
        assessment_digest: assessment.digest()?,
        policy_digest: policy.digest()?,
        assessed_at_micros: assessment.assessed_at_micros,
    })
}

pub fn symthaea_model_identity_digest_v1(
    model: &SymthaeaModelIdentityV2,
) -> Result<SymthaeaModelIdentityDigestV1, SymthaeaDistributionAssessmentV2Error> {
    let mut framed = Vec::new();
    append_symthaea_string(&mut framed, &model.model.name)?;
    append_symthaea_string(&mut framed, &model.model.version)?;
    append_symthaea_digest(&mut framed, &model.model.digest);
    append_symthaea_digest(&mut framed, &model.input_schema_digest);
    append_symthaea_digest(&mut framed, &model.output_schema_digest);
    append_optional_symthaea_evidence(&mut framed, model.training_lineage.as_ref())?;
    append_optional_symthaea_evidence(&mut framed, model.evaluation_lineage.as_ref())?;
    append_optional_symthaea_evidence(&mut framed, model.calibration_evidence.as_ref())?;
    Ok(SymthaeaModelIdentityDigestV1(domain_hash(
        MODEL_DERIVE_KEY,
        &framed,
    )))
}

fn append_distribution_artifact(
    target: &mut Vec<u8>,
    identity: &DistributionArtifactIdentityV2,
) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
    identity.validate()?;
    target.extend_from_slice(&identity.identity_version.to_be_bytes());
    append_string(target, &identity.name)?;
    append_string(target, &identity.version)?;
    target.extend_from_slice(&identity.digest);
    Ok(())
}

fn append_distribution_evidence(
    target: &mut Vec<u8>,
    identity: &DistributionEvidenceIdentityV2,
) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
    identity.validate()?;
    target.extend_from_slice(&identity.identity_version.to_be_bytes());
    append_string(target, &identity.namespace)?;
    append_string(target, &identity.artifact_id)?;
    target.extend_from_slice(&identity.digest);
    Ok(())
}

fn append_optional_distribution_evidence(
    target: &mut Vec<u8>,
    identity: Option<&DistributionEvidenceIdentityV2>,
) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
    match identity {
        Some(identity) => {
            target.push(1);
            append_distribution_evidence(target, identity)
        }
        None => {
            target.push(0);
            Ok(())
        }
    }
}

fn append_numeric_boundary(
    target: &mut Vec<u8>,
    boundary: Option<&SymthaeaDistributionNumericBoundaryV2>,
) {
    match boundary {
        Some(boundary) => {
            target.push(1);
            target.extend_from_slice(&boundary.score.to_bits().to_be_bytes());
            target.extend_from_slice(&boundary.threshold.to_bits().to_be_bytes());
            target.push(match boundary.direction {
                SymthaeaDistributionDirectionV2::HigherMeansMoreInDistribution => 0,
                SymthaeaDistributionDirectionV2::LowerMeansMoreInDistribution => 1,
            });
        }
        None => target.push(0),
    }
}

fn append_optional_symthaea_evidence(
    target: &mut Vec<u8>,
    identity: Option<&SymthaeaEvidenceIdentityV2>,
) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
    match identity {
        Some(identity) => {
            target.push(1);
            append_symthaea_evidence(target, identity)
        }
        None => {
            target.push(0);
            Ok(())
        }
    }
}

fn append_symthaea_evidence(
    target: &mut Vec<u8>,
    identity: &SymthaeaEvidenceIdentityV2,
) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
    target.extend_from_slice(&identity.identity_version.to_be_bytes());
    append_symthaea_string(target, &identity.namespace)?;
    append_symthaea_string(target, &identity.artifact_id)?;
    append_symthaea_digest(target, &identity.digest);
    Ok(())
}

fn append_symthaea_digest(target: &mut Vec<u8>, digest: &mycelix_symthaea_clinical_wire_v2::SymthaeaDigestV2) {
    target.push(match digest.algorithm {
        SymthaeaDigestAlgorithmV2::Blake3_256 => 0,
    });
    target.extend_from_slice(&digest.value);
}

fn append_symthaea_string(
    target: &mut Vec<u8>,
    value: &str,
) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
    validate_reference(value)?;
    append_len_prefixed(target, value.as_bytes())
}

fn append_string(
    target: &mut Vec<u8>,
    value: &str,
) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
    append_len_prefixed(target, value.as_bytes())
}

fn append_len_prefixed(
    target: &mut Vec<u8>,
    value: &[u8],
) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
    let len = u32::try_from(value.len())
        .map_err(|_| SymthaeaDistributionAssessmentV2Error::DigestMaterialTooLarge)?;
    target.extend_from_slice(&len.to_be_bytes());
    target.extend_from_slice(value);
    Ok(())
}

fn status_tag(status: SymthaeaDistributionAssessmentStatusV2) -> u8 {
    match status {
        SymthaeaDistributionAssessmentStatusV2::NotRun => 0,
        SymthaeaDistributionAssessmentStatusV2::Unavailable => 1,
        SymthaeaDistributionAssessmentStatusV2::Indeterminate => 2,
        SymthaeaDistributionAssessmentStatusV2::InDistribution => 3,
        SymthaeaDistributionAssessmentStatusV2::OutOfDistribution => 4,
    }
}

fn domain_hash(context: &str, framed: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&(framed.len() as u64).to_be_bytes());
    hasher.update(framed);
    *hasher.finalize().as_bytes()
}

fn validate_nonzero_digest(
    digest: &[u8; 32],
) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
    if *digest == [0u8; 32] {
        return Err(SymthaeaDistributionAssessmentV2Error::ZeroDigest);
    }
    Ok(())
}

fn validate_token(value: &str) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
    validate_text(value, MAX_TOKEN_BYTES)
}

fn validate_reference(value: &str) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
    validate_text(value, MAX_REFERENCE_BYTES)
}

fn validate_text(
    value: &str,
    max_bytes: usize,
) -> Result<(), SymthaeaDistributionAssessmentV2Error> {
    if value.is_empty()
        || value.len() > max_bytes
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(SymthaeaDistributionAssessmentV2Error::InvalidText);
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SymthaeaDistributionAssessmentV2Error {
    #[error("unsupported distribution identity version {0}")]
    UnsupportedIdentityVersion(u16),
    #[error("unsupported Symthaea distribution assessment version {0}")]
    UnsupportedAssessmentVersion(u16),
    #[error("unsupported Symthaea distribution policy version {0}")]
    UnsupportedPolicyVersion(u16),
    #[error("text field is empty, malformed, or exceeds its bound")]
    InvalidText,
    #[error("identity digest must not be zero")]
    ZeroDigest,
    #[error("distribution numeric boundary is invalid")]
    InvalidNumericBoundary,
    #[error("distribution status contradicts the numeric boundary")]
    NumericStatusMismatch,
    #[error("detector-produced distribution status requires assessment evidence")]
    MissingAssessmentEvidence,
    #[error("assessment time must be positive")]
    InvalidAssessmentTime,
    #[error("distribution policy maximum age is invalid")]
    InvalidMaximumAge,
    #[error("distribution policy maximum future skew is invalid")]
    InvalidMaximumFutureSkew,
    #[error("current time must be positive")]
    InvalidCurrentTime,
    #[error("assessment belongs to another model assertion")]
    AssertionDigestMismatch,
    #[error("assessment belongs to another Symthaea wire artifact")]
    WireDigestMismatch,
    #[error("assessment belongs to another verified evidence context")]
    EvidenceContextDigestMismatch,
    #[error("assessment belongs to another Symthaea admission policy")]
    AdmissionPolicyDigestMismatch,
    #[error("assessment subject differs from the verified model assertion")]
    SubjectMismatch,
    #[error("assessment model identity differs from the verified model assertion")]
    ModelIdentityMismatch,
    #[error("assessment detector differs from the distribution policy")]
    DetectorMismatch,
    #[error("assessment reference population differs from the distribution policy")]
    ReferencePopulationMismatch,
    #[error("assessment reference domain differs from the distribution policy")]
    ReferenceDomainMismatch,
    #[error("distribution assessment is too far in the future")]
    AssessmentFromFuture,
    #[error("distribution assessment is stale")]
    StaleAssessment,
    #[error("digest material exceeds framing limits")]
    DigestMaterialTooLarge,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_symthaea_clinical_wire_v2::{
        SymthaeaArtifactIdentityV2, SymthaeaDigestV2,
    };

    fn digest(seed: u8) -> SymthaeaDigestV2 {
        SymthaeaDigestV2 {
            algorithm: SymthaeaDigestAlgorithmV2::Blake3_256,
            value: [seed; 32],
        }
    }

    fn evidence(seed: u8) -> SymthaeaEvidenceIdentityV2 {
        SymthaeaEvidenceIdentityV2 {
            identity_version: 1,
            namespace: "test/evidence/v1".into(),
            artifact_id: format!("evidence-{seed}"),
            digest: digest(seed),
        }
    }

    fn model(seed: u8) -> SymthaeaModelIdentityV2 {
        SymthaeaModelIdentityV2 {
            model: SymthaeaArtifactIdentityV2 {
                name: "model".into(),
                version: "1".into(),
                digest: digest(seed),
            },
            input_schema_digest: digest(seed.wrapping_add(1)),
            output_schema_digest: digest(seed.wrapping_add(2)),
            training_lineage: Some(evidence(seed.wrapping_add(3))),
            evaluation_lineage: Some(evidence(seed.wrapping_add(4))),
            calibration_evidence: Some(evidence(seed.wrapping_add(5))),
        }
    }

    #[test]
    fn model_identity_digest_changes_with_lineage() {
        let first = symthaea_model_identity_digest_v1(&model(10)).unwrap();
        let mut changed = model(10);
        changed.evaluation_lineage = Some(evidence(99));
        let second = symthaea_model_identity_digest_v1(&changed).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn numeric_status_mismatch_fails_closed() {
        let assessment = SymthaeaClinicalDistributionAssessmentV2 {
            schema_version: 1,
            assessment_id: "assessment-1".into(),
            assertion_digest: [1; 32],
            wire_digest: [2; 32],
            evidence_context_digest: [3; 32],
            admission_policy_digest: [4; 32],
            subject_id: "patient-a".into(),
            model_identity_digest: [5; 32],
            detector: DistributionArtifactIdentityV2 {
                identity_version: 1,
                name: "ood".into(),
                version: "1".into(),
                digest: [6; 32],
            },
            reference_population: "adult-outpatient-v1".into(),
            reference_domain: DistributionEvidenceIdentityV2 {
                identity_version: 1,
                namespace: "mycelix/distribution-reference-domain/v1".into(),
                artifact_id: "adult-outpatient-v1".into(),
                digest: [7; 32],
            },
            status: SymthaeaDistributionAssessmentStatusV2::InDistribution,
            numeric_boundary: Some(SymthaeaDistributionNumericBoundaryV2 {
                score: 0.2,
                threshold: 0.5,
                direction: SymthaeaDistributionDirectionV2::HigherMeansMoreInDistribution,
            }),
            assessment_evidence: Some(DistributionEvidenceIdentityV2 {
                identity_version: 1,
                namespace: "mycelix/ood-assessment-evidence/v1".into(),
                artifact_id: "assessment-output-1".into(),
                digest: [8; 32],
            }),
            assessed_at_micros: 100,
        };
        assert_eq!(
            assessment.validate(),
            Err(SymthaeaDistributionAssessmentV2Error::NumericStatusMismatch)
        );
    }
}
