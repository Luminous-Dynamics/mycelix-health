#![deny(unsafe_code)]
//! Assertion-native runtime trust for Symthaea clinical distribution/OOD evidence.
//!
//! This v3 path is intentionally separate from legacy capsule-bound distribution
//! trust. It consumes the opaque structural proof produced by
//! `mycelix-symthaea-clinical-distribution-assessment-v2` and the opaque,
//! DNA-rooted evaluator-currentness proof produced by
//! `mycelix-clinical-distribution-currentness`.
//!
//! The resulting receipt proves only that one exact Symthaea assertion's
//! distribution assessment was produced while the exact admitted evaluator had a
//! positive, bounded currentness lease, and that the same currentness proof was
//! still live when trust was evaluated. It does not establish model validity,
//! clinical utility, global revocation completeness, presentation authority, or
//! treatment authority.

use mycelix_clinical_distribution_currentness::{
    DistributionEvaluatorCurrentnessDigestV1, DistributionEvaluatorCurrentnessError,
    DistributionEvaluatorCurrentnessPolicyDigestV1, DistributionEvaluatorCurrentnessPolicyV1,
    VerifiedDistributionEvaluatorCurrentnessV1,
};
use mycelix_clinical_distribution_trust::{
    ClinicalDistributionTrustError, DistributionEvaluatorTrustPolicyV1,
    CLINICAL_DISTRIBUTION_TRUST_POLICY_VERSION,
};
use mycelix_clinical_evidence::{ArtifactIdentity, ContentDigest};
use mycelix_symthaea_clinical_distribution_assessment_v2::{
    DistributionArtifactIdentityV2, SymthaeaClinicalDistributionPolicyV2,
    SymthaeaDistributionAssessmentDigestV2, SymthaeaDistributionAssessmentStatusV2,
    SymthaeaDistributionAssessmentV2Error, SymthaeaDistributionPolicyDigestV2,
    SymthaeaModelIdentityDigestV1, ValidatedSymthaeaClinicalDistributionAssessmentV2,
};
use thiserror::Error;

const DETECTOR_BRIDGE_DERIVE_KEY: &str =
    "mycelix.health.symthaea-distribution-detector-runtime-bridge.v1";
const POLICY_BRIDGE_DERIVE_KEY: &str =
    "mycelix.health.symthaea-distribution-policy-runtime-bridge.v1";
const RECEIPT_DERIVE_KEY: &str =
    "mycelix.health.symthaea-distribution-runtime-trust-receipt-v3.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymthaeaDistributionTrustReceiptDigestV3([u8; 32]);

impl SymthaeaDistributionTrustReceiptDigestV3 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Canonical runtime identity for a Symthaea-v2 distribution detector.
///
/// The bridge hashes the complete versioned detector identity into its own domain
/// rather than re-labeling the detector's opaque 32-byte digest. This prevents a
/// raw digest from another identity namespace from being silently interpreted as
/// an equivalent runtime artifact.
pub fn runtime_detector_identity_for_symthaea_v3(
    detector: &DistributionArtifactIdentityV2,
) -> Result<ArtifactIdentity, ClinicalDistributionTrustV3Error> {
    detector.validate()?;
    let mut framed = Vec::new();
    framed.extend_from_slice(&detector.identity_version.to_be_bytes());
    append_bytes(&mut framed, detector.name.as_bytes())?;
    append_bytes(&mut framed, detector.version.as_bytes())?;
    append_bytes(&mut framed, &detector.digest)?;

    Ok(ArtifactIdentity {
        name: detector.name.clone(),
        version: detector.version.clone(),
        digest: bridge_content_digest(DETECTOR_BRIDGE_DERIVE_KEY, &framed),
    })
}

/// Canonical runtime digest for the exact Symthaea-v2 structural OOD policy.
///
/// The structural policy already has a domain-separated BLAKE3 digest. The
/// runtime bridge hashes that digest again in a distinct namespace so the generic
/// runtime trust API cannot confuse it with a legacy distribution-policy digest.
pub fn runtime_distribution_policy_digest_for_symthaea_v3(
    policy: &SymthaeaClinicalDistributionPolicyV2,
) -> Result<ContentDigest, ClinicalDistributionTrustV3Error> {
    let structural_digest = policy.digest()?;
    Ok(bridge_content_digest(
        POLICY_BRIDGE_DERIVE_KEY,
        structural_digest.as_bytes(),
    ))
}

/// Build the generic deployable evaluator trust policy for one exact Symthaea-v2
/// distribution policy.
///
/// The returned policy can be rooted in the existing clinical-distribution-trust
/// DNA machinery. Evaluation still re-derives both bridge identities and rejects
/// a caller-supplied policy that differs from them.
pub fn build_symthaea_distribution_trust_policy_v3(
    policy_id: impl Into<String>,
    structural_policy: &SymthaeaClinicalDistributionPolicyV2,
    max_future_skew_micros: i64,
) -> Result<DistributionEvaluatorTrustPolicyV1, ClinicalDistributionTrustV3Error> {
    structural_policy.validate()?;
    let trust_policy = DistributionEvaluatorTrustPolicyV1 {
        schema_version: CLINICAL_DISTRIBUTION_TRUST_POLICY_VERSION,
        policy_id: policy_id.into(),
        distribution_policy_digest: runtime_distribution_policy_digest_for_symthaea_v3(
            structural_policy,
        )?,
        required_detector: runtime_detector_identity_for_symthaea_v3(
            &structural_policy.required_detector,
        )?,
        max_future_skew_micros,
    };
    trust_policy.validate()?;
    Ok(trust_policy)
}

/// Non-cloneable, non-serializable proof that one exact assertion-native OOD
/// assessment was produced and consumed inside one already-established runtime
/// evaluator-currentness interval.
pub struct SymthaeaDistributionTrustReceiptV3 {
    assertion_digest: [u8; 32],
    wire_digest: [u8; 32],
    evidence_context_digest: [u8; 32],
    admission_policy_digest: [u8; 32],
    subject_id: String,
    model_identity_digest: SymthaeaModelIdentityDigestV1,
    assessment_digest: SymthaeaDistributionAssessmentDigestV2,
    structural_policy_digest: SymthaeaDistributionPolicyDigestV2,
    runtime_detector: ArtifactIdentity,
    runtime_distribution_policy_digest: ContentDigest,
    trust_policy_digest: ContentDigest,
    evaluator_admission_evidence_digest: ContentDigest,
    currentness_policy_digest: DistributionEvaluatorCurrentnessPolicyDigestV1,
    currentness_digest: DistributionEvaluatorCurrentnessDigestV1,
    status: SymthaeaDistributionAssessmentStatusV2,
    assessed_at_micros: i64,
    currentness_verified_at_micros: i64,
    currentness_valid_until_micros: i64,
    trusted_at_micros: i64,
    receipt_digest: SymthaeaDistributionTrustReceiptDigestV3,
}

impl SymthaeaDistributionTrustReceiptV3 {
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

    #[must_use]
    pub const fn assessment_digest(&self) -> SymthaeaDistributionAssessmentDigestV2 {
        self.assessment_digest
    }

    #[must_use]
    pub const fn structural_policy_digest(&self) -> SymthaeaDistributionPolicyDigestV2 {
        self.structural_policy_digest
    }

    pub fn runtime_detector(&self) -> &ArtifactIdentity {
        &self.runtime_detector
    }

    pub fn runtime_distribution_policy_digest(&self) -> &ContentDigest {
        &self.runtime_distribution_policy_digest
    }

    pub fn trust_policy_digest(&self) -> &ContentDigest {
        &self.trust_policy_digest
    }

    pub fn evaluator_admission_evidence_digest(&self) -> &ContentDigest {
        &self.evaluator_admission_evidence_digest
    }

    #[must_use]
    pub const fn currentness_policy_digest(
        &self,
    ) -> DistributionEvaluatorCurrentnessPolicyDigestV1 {
        self.currentness_policy_digest
    }

    #[must_use]
    pub const fn currentness_digest(&self) -> DistributionEvaluatorCurrentnessDigestV1 {
        self.currentness_digest
    }

    #[must_use]
    pub const fn status(&self) -> SymthaeaDistributionAssessmentStatusV2 {
        self.status
    }

    #[must_use]
    pub const fn assessed_at_micros(&self) -> i64 {
        self.assessed_at_micros
    }

    #[must_use]
    pub const fn currentness_verified_at_micros(&self) -> i64 {
        self.currentness_verified_at_micros
    }

    #[must_use]
    pub const fn currentness_valid_until_micros(&self) -> i64 {
        self.currentness_valid_until_micros
    }

    #[must_use]
    pub const fn trusted_at_micros(&self) -> i64 {
        self.trusted_at_micros
    }

    #[must_use]
    pub const fn receipt_digest(&self) -> SymthaeaDistributionTrustReceiptDigestV3 {
        self.receipt_digest
    }
}

/// Evaluate assertion-native OOD trust under an already-established runtime
/// evaluator-currentness proof.
///
/// The critical ordering theorem is lease-before-use:
///
/// `currentness.verified_at <= assessment.assessed_at < currentness.valid_until`
///
/// and the same currentness proof must remain active at `trusted_at_micros`.
/// Therefore an evaluator cannot produce an assessment while untrusted and have
/// that old assessment become trusted merely because the evaluator is admitted or
/// leased later.
pub fn evaluate_symthaea_distribution_trust_v3(
    structural: &ValidatedSymthaeaClinicalDistributionAssessmentV2,
    structural_policy: &SymthaeaClinicalDistributionPolicyV2,
    trust_policy: &DistributionEvaluatorTrustPolicyV1,
    currentness_policy: &DistributionEvaluatorCurrentnessPolicyV1,
    currentness: &VerifiedDistributionEvaluatorCurrentnessV1,
    trusted_at_micros: i64,
) -> Result<SymthaeaDistributionTrustReceiptV3, ClinicalDistributionTrustV3Error> {
    structural_policy.validate()?;
    trust_policy.validate()?;
    if trusted_at_micros <= 0 {
        return Err(ClinicalDistributionTrustV3Error::InvalidTrustEvaluationTime);
    }

    let structural_policy_digest = structural_policy.digest()?;
    if structural.policy_digest() != structural_policy_digest {
        return Err(ClinicalDistributionTrustV3Error::StructuralPolicyMismatch);
    }

    let runtime_detector =
        runtime_detector_identity_for_symthaea_v3(&structural_policy.required_detector)?;
    let runtime_distribution_policy_digest =
        runtime_distribution_policy_digest_for_symthaea_v3(structural_policy)?;

    if trust_policy.distribution_policy_digest != runtime_distribution_policy_digest {
        return Err(ClinicalDistributionTrustV3Error::TrustPolicyStructuralPolicyMismatch);
    }
    if trust_policy.required_detector != runtime_detector {
        return Err(ClinicalDistributionTrustV3Error::TrustPolicyDetectorMismatch);
    }

    let trust_policy_digest = trust_policy.digest()?;
    let currentness_policy_digest = currentness_policy.digest()?;
    if currentness.policy_digest() != currentness_policy_digest {
        return Err(ClinicalDistributionTrustV3Error::CurrentnessPolicyMismatch);
    }
    if currentness.detector() != &runtime_detector {
        return Err(ClinicalDistributionTrustV3Error::CurrentnessDetectorMismatch);
    }
    if currentness.distribution_policy_digest() != &runtime_distribution_policy_digest {
        return Err(ClinicalDistributionTrustV3Error::CurrentnessDistributionPolicyMismatch);
    }
    if currentness.trust_policy_digest() != &trust_policy_digest {
        return Err(ClinicalDistributionTrustV3Error::CurrentnessTrustPolicyMismatch);
    }

    let assessed_at_micros = structural.assessed_at_micros();
    if currentness.verified_at_micros() > assessed_at_micros {
        return Err(ClinicalDistributionTrustV3Error::CurrentnessEstablishedAfterAssessment);
    }
    if assessed_at_micros >= currentness.valid_until_micros() {
        return Err(ClinicalDistributionTrustV3Error::CurrentnessExpiredAtAssessment);
    }
    if !currentness.active_at(assessed_at_micros) {
        return Err(ClinicalDistributionTrustV3Error::CurrentnessInactiveAtAssessment);
    }
    if assessed_at_micros > trusted_at_micros {
        return Err(ClinicalDistributionTrustV3Error::AssessmentAfterTrustEvaluation);
    }
    if !currentness.active_at(trusted_at_micros) {
        return Err(ClinicalDistributionTrustV3Error::CurrentnessInactiveAtTrustEvaluation);
    }

    let assessment_digest = structural.assessment_digest();
    let model_identity_digest = structural.model_identity_digest();
    let currentness_digest = currentness.currentness_digest();
    let evaluator_admission_evidence_digest = currentness.admission_evidence_digest().clone();

    let receipt_digest = compute_receipt_digest(ReceiptDigestMaterial {
        assertion_digest: structural.assertion_digest(),
        wire_digest: structural.wire_digest(),
        evidence_context_digest: structural.evidence_context_digest(),
        admission_policy_digest: structural.admission_policy_digest(),
        subject_id: structural.subject_id(),
        model_identity_digest: &model_identity_digest,
        assessment_digest: &assessment_digest,
        structural_policy_digest: &structural_policy_digest,
        runtime_detector: &runtime_detector,
        runtime_distribution_policy_digest: &runtime_distribution_policy_digest,
        trust_policy_digest: &trust_policy_digest,
        evaluator_admission_evidence_digest: &evaluator_admission_evidence_digest,
        currentness_policy_digest: &currentness_policy_digest,
        currentness_digest: &currentness_digest,
        status: structural.status(),
        assessed_at_micros,
        currentness_verified_at_micros: currentness.verified_at_micros(),
        currentness_valid_until_micros: currentness.valid_until_micros(),
        trusted_at_micros,
    })?;

    Ok(SymthaeaDistributionTrustReceiptV3 {
        assertion_digest: *structural.assertion_digest(),
        wire_digest: *structural.wire_digest(),
        evidence_context_digest: *structural.evidence_context_digest(),
        admission_policy_digest: *structural.admission_policy_digest(),
        subject_id: structural.subject_id().to_owned(),
        model_identity_digest,
        assessment_digest,
        structural_policy_digest,
        runtime_detector,
        runtime_distribution_policy_digest,
        trust_policy_digest,
        evaluator_admission_evidence_digest,
        currentness_policy_digest,
        currentness_digest,
        status: structural.status(),
        assessed_at_micros,
        currentness_verified_at_micros: currentness.verified_at_micros(),
        currentness_valid_until_micros: currentness.valid_until_micros(),
        trusted_at_micros,
        receipt_digest,
    })
}

struct ReceiptDigestMaterial<'a> {
    assertion_digest: &'a [u8; 32],
    wire_digest: &'a [u8; 32],
    evidence_context_digest: &'a [u8; 32],
    admission_policy_digest: &'a [u8; 32],
    subject_id: &'a str,
    model_identity_digest: &'a SymthaeaModelIdentityDigestV1,
    assessment_digest: &'a SymthaeaDistributionAssessmentDigestV2,
    structural_policy_digest: &'a SymthaeaDistributionPolicyDigestV2,
    runtime_detector: &'a ArtifactIdentity,
    runtime_distribution_policy_digest: &'a ContentDigest,
    trust_policy_digest: &'a ContentDigest,
    evaluator_admission_evidence_digest: &'a ContentDigest,
    currentness_policy_digest: &'a DistributionEvaluatorCurrentnessPolicyDigestV1,
    currentness_digest: &'a DistributionEvaluatorCurrentnessDigestV1,
    status: SymthaeaDistributionAssessmentStatusV2,
    assessed_at_micros: i64,
    currentness_verified_at_micros: i64,
    currentness_valid_until_micros: i64,
    trusted_at_micros: i64,
}

fn compute_receipt_digest(
    material: ReceiptDigestMaterial<'_>,
) -> Result<SymthaeaDistributionTrustReceiptDigestV3, ClinicalDistributionTrustV3Error> {
    let mut framed = Vec::new();
    append_bytes(&mut framed, material.assertion_digest)?;
    append_bytes(&mut framed, material.wire_digest)?;
    append_bytes(&mut framed, material.evidence_context_digest)?;
    append_bytes(&mut framed, material.admission_policy_digest)?;
    append_bytes(&mut framed, material.subject_id.as_bytes())?;
    append_bytes(&mut framed, material.model_identity_digest.as_bytes())?;
    append_bytes(&mut framed, material.assessment_digest.as_bytes())?;
    append_bytes(&mut framed, material.structural_policy_digest.as_bytes())?;
    append_artifact_identity(&mut framed, material.runtime_detector)?;
    append_content_digest(&mut framed, material.runtime_distribution_policy_digest)?;
    append_content_digest(&mut framed, material.trust_policy_digest)?;
    append_content_digest(&mut framed, material.evaluator_admission_evidence_digest)?;
    append_bytes(&mut framed, material.currentness_policy_digest.as_bytes())?;
    append_bytes(&mut framed, material.currentness_digest.as_bytes())?;
    framed.push(status_tag(material.status));
    framed.extend_from_slice(&material.assessed_at_micros.to_be_bytes());
    framed.extend_from_slice(&material.currentness_verified_at_micros.to_be_bytes());
    framed.extend_from_slice(&material.currentness_valid_until_micros.to_be_bytes());
    framed.extend_from_slice(&material.trusted_at_micros.to_be_bytes());

    let mut hasher = blake3::Hasher::new_derive_key(RECEIPT_DERIVE_KEY);
    hasher.update(&(framed.len() as u64).to_be_bytes());
    hasher.update(&framed);
    Ok(SymthaeaDistributionTrustReceiptDigestV3(
        *hasher.finalize().as_bytes(),
    ))
}

fn append_artifact_identity(
    target: &mut Vec<u8>,
    artifact: &ArtifactIdentity,
) -> Result<(), ClinicalDistributionTrustV3Error> {
    append_bytes(target, artifact.name.as_bytes())?;
    append_bytes(target, artifact.version.as_bytes())?;
    append_content_digest(target, &artifact.digest)
}

fn append_content_digest(
    target: &mut Vec<u8>,
    digest: &ContentDigest,
) -> Result<(), ClinicalDistributionTrustV3Error> {
    append_bytes(target, digest.algorithm.as_bytes())?;
    append_bytes(target, digest.value.as_bytes())
}

fn append_bytes(
    target: &mut Vec<u8>,
    value: &[u8],
) -> Result<(), ClinicalDistributionTrustV3Error> {
    let len = u32::try_from(value.len())
        .map_err(|_| ClinicalDistributionTrustV3Error::DigestMaterialTooLarge)?;
    target.extend_from_slice(&len.to_be_bytes());
    target.extend_from_slice(value);
    Ok(())
}

fn bridge_content_digest(context: &str, payload: &[u8]) -> ContentDigest {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&(payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    ContentDigest {
        algorithm: "blake3-256".into(),
        value: hasher.finalize().to_hex().to_string(),
    }
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

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClinicalDistributionTrustV3Error {
    #[error(transparent)]
    Structural(#[from] SymthaeaDistributionAssessmentV2Error),
    #[error(transparent)]
    TrustPolicy(#[from] ClinicalDistributionTrustError),
    #[error(transparent)]
    Currentness(#[from] DistributionEvaluatorCurrentnessError),
    #[error("trust evaluation time must be positive")]
    InvalidTrustEvaluationTime,
    #[error("validated Symthaea assessment belongs to another structural distribution policy")]
    StructuralPolicyMismatch,
    #[error("runtime evaluator trust policy does not bind the canonical Symthaea structural-policy bridge")]
    TrustPolicyStructuralPolicyMismatch,
    #[error("runtime evaluator trust policy does not bind the canonical Symthaea detector bridge")]
    TrustPolicyDetectorMismatch,
    #[error("runtime currentness proof belongs to another currentness policy")]
    CurrentnessPolicyMismatch,
    #[error("runtime currentness proof belongs to another detector identity")]
    CurrentnessDetectorMismatch,
    #[error("runtime currentness proof belongs to another structural distribution policy")]
    CurrentnessDistributionPolicyMismatch,
    #[error("runtime currentness proof belongs to another evaluator trust policy")]
    CurrentnessTrustPolicyMismatch,
    #[error("runtime evaluator currentness was established only after the assessment was produced")]
    CurrentnessEstablishedAfterAssessment,
    #[error("runtime evaluator currentness had already expired when the assessment was produced")]
    CurrentnessExpiredAtAssessment,
    #[error("runtime evaluator currentness was not active when the assessment was produced")]
    CurrentnessInactiveAtAssessment,
    #[error("assessment timestamp follows trust-evaluation time")]
    AssessmentAfterTrustEvaluation,
    #[error("runtime evaluator currentness is not active at trust-evaluation time")]
    CurrentnessInactiveAtTrustEvaluation,
    #[error("receipt digest material exceeds framing limits")]
    DigestMaterialTooLarge,
}