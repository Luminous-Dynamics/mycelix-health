#![deny(unsafe_code)]
//! Deployment-scoped trust admission for causal-evidence artifacts.
//!
//! A causal assessment says how evidence is interpreted. This crate answers the
//! independent question: did deployment policy admit the exact artifacts used for
//! each evidence kind at the time of assessment?

use mycelix_clinical_causality::{
    CausalAssessmentPolicyV1, CausalAssessmentV1, CausalEvidenceKindV1, ExposureAssociationV1,
};
use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use thiserror::Error;

const POLICY_TAG: &[u8] = b"mycelix-health/clinical-causal-evidence-trust-policy-v1";
const RECEIPT_TAG: &[u8] = b"mycelix-health/clinical-causal-evidence-trust-receipt-v1";
const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000;
const MAX_ADMISSIONS: usize = 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CausalEvidenceTrustPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    /// Exact causal-assessment policy whose evidence this trust policy may admit.
    pub causal_assessment_policy_digest: StoredDigest,
    pub max_future_skew_micros: i64,
}

impl CausalEvidenceTrustPolicyV1 {
    pub fn validate(&self) -> Result<(), CausalEvidenceTrustError> {
        if self.schema_version != 1 {
            return Err(CausalEvidenceTrustError::UnsupportedPolicyVersion(
                self.schema_version,
            ));
        }
        if self.policy_id.trim().is_empty() {
            return Err(CausalEvidenceTrustError::MissingPolicyId);
        }
        require_domain(
            self.causal_assessment_policy_digest,
            DigestDomain::ClinicalCausalAssessmentPolicy,
        )?;
        if self.max_future_skew_micros < 0
            || self.max_future_skew_micros > MAX_FUTURE_SKEW_MICROS
        {
            return Err(CausalEvidenceTrustError::InvalidFutureSkewPolicy);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, CausalEvidenceTrustError> {
        self.validate()?;
        hash_json(
            DigestDomain::ClinicalCausalEvidenceTrustPolicy,
            POLICY_TAG,
            self,
        )
    }
}

/// Verifier-owned proof that one exact causal evidence artifact has been admitted
/// for one or more evidence kinds under one exact trust policy.
///
/// No serde implementation is provided. Constructing this type does not happen from
/// arbitrary wire JSON; a trusted registry/configuration adapter must first verify
/// the admission record represented by `admission_evidence_digest`.
pub struct VerifiedCausalEvidenceAdmission {
    evidence_artifact_digest: StoredDigest,
    allowed_kinds: BTreeSet<CausalEvidenceKindV1>,
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    revoked_at_micros: Option<i64>,
    trust_policy_digest: StoredDigest,
    admission_evidence_digest: StoredDigest,
}

impl VerifiedCausalEvidenceAdmission {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_record(
        evidence_artifact_digest: VerifiedDigest,
        allowed_kinds: Vec<CausalEvidenceKindV1>,
        valid_from_micros: i64,
        valid_until_micros: Option<i64>,
        revoked_at_micros: Option<i64>,
        trust_policy_digest: VerifiedDigest,
        admission_evidence_digest: VerifiedDigest,
    ) -> Result<Self, CausalEvidenceTrustError> {
        trust_policy_digest.require_domain(DigestDomain::ClinicalCausalEvidenceTrustPolicy)?;
        admission_evidence_digest.require_domain(DigestDomain::ClinicalArtifact)?;
        if allowed_kinds.is_empty() {
            return Err(CausalEvidenceTrustError::AdmissionHasNoKinds);
        }
        let allowed_kinds: BTreeSet<_> = allowed_kinds.into_iter().collect();
        if allowed_kinds.is_empty() {
            return Err(CausalEvidenceTrustError::AdmissionHasNoKinds);
        }
        validate_window(valid_from_micros, valid_until_micros, revoked_at_micros)?;
        Ok(Self {
            evidence_artifact_digest: evidence_artifact_digest.stored(),
            allowed_kinds,
            valid_from_micros,
            valid_until_micros,
            revoked_at_micros,
            trust_policy_digest: trust_policy_digest.stored(),
            admission_evidence_digest: admission_evidence_digest.stored(),
        })
    }

    fn active_at(&self, at_micros: i64) -> bool {
        at_micros >= self.valid_from_micros
            && self
                .valid_until_micros
                .map(|until| at_micros < until)
                .unwrap_or(true)
            && self
                .revoked_at_micros
                .map(|revoked| at_micros < revoked)
                .unwrap_or(true)
    }

    fn admits(
        &self,
        artifact_digest: StoredDigest,
        kind: CausalEvidenceKindV1,
        trust_policy_digest: StoredDigest,
    ) -> bool {
        self.trust_policy_digest == trust_policy_digest
            && self.evidence_artifact_digest == artifact_digest
            && self.allowed_kinds.contains(&kind)
    }
}

#[derive(Serialize)]
struct TrustReceiptMaterial {
    assessment_digest: StoredDigest,
    causal_assessment_policy_digest: StoredDigest,
    trust_policy_digest: StoredDigest,
    evidence_artifact_digests: Vec<StoredDigest>,
    admission_evidence_digests: Vec<StoredDigest>,
    evaluated_at_micros: i64,
}

/// Single-owner proof that every non-unknown evidence artifact actually used by one
/// exact assessment was admitted for the evidence kind under one active deployment
/// trust policy. Intentionally not Clone/Copy/Serialize/Deserialize/Debug.
pub struct CausalEvidenceTrustReceipt {
    assessment_digest: StoredDigest,
    causal_assessment_policy_digest: StoredDigest,
    trust_policy_digest: StoredDigest,
    evidence_artifact_digests: Vec<StoredDigest>,
    admission_evidence_digests: Vec<StoredDigest>,
    evaluated_at_micros: i64,
    receipt_digest: StoredDigest,
}

impl CausalEvidenceTrustReceipt {
    pub fn assessment_digest(&self) -> StoredDigest {
        self.assessment_digest
    }

    pub fn causal_assessment_policy_digest(&self) -> StoredDigest {
        self.causal_assessment_policy_digest
    }

    pub fn trust_policy_digest(&self) -> StoredDigest {
        self.trust_policy_digest
    }

    pub fn evidence_artifact_digests(&self) -> &[StoredDigest] {
        &self.evidence_artifact_digests
    }

    pub fn admission_evidence_digests(&self) -> &[StoredDigest] {
        &self.admission_evidence_digests
    }

    pub fn evaluated_at_micros(&self) -> i64 {
        self.evaluated_at_micros
    }

    pub fn receipt_digest(&self) -> StoredDigest {
        self.receipt_digest
    }
}

pub fn evaluate_causal_evidence_trust(
    assessment: &CausalAssessmentV1,
    association: &ExposureAssociationV1,
    assessment_policy: &CausalAssessmentPolicyV1,
    trust_policy: &CausalEvidenceTrustPolicyV1,
    admissions: &[VerifiedCausalEvidenceAdmission],
    at_micros: i64,
) -> Result<CausalEvidenceTrustReceipt, CausalEvidenceTrustError> {
    assessment
        .validate_against(association, assessment_policy)
        .map_err(|error| CausalEvidenceTrustError::InvalidAssessment(error.to_string()))?;
    trust_policy.validate()?;
    if admissions.len() > MAX_ADMISSIONS {
        return Err(CausalEvidenceTrustError::TooManyAdmissions);
    }

    let assessment_policy_digest = assessment_policy
        .verified_digest()
        .map_err(|error| CausalEvidenceTrustError::InvalidAssessment(error.to_string()))?;
    if trust_policy.causal_assessment_policy_digest != assessment_policy_digest.stored() {
        return Err(CausalEvidenceTrustError::AssessmentPolicyMismatch);
    }
    let assessment_digest = assessment
        .verified_digest(association, assessment_policy)
        .map_err(|error| CausalEvidenceTrustError::InvalidAssessment(error.to_string()))?;
    let trust_policy_digest = trust_policy.verified_digest()?;

    if at_micros
        .saturating_add(trust_policy.max_future_skew_micros)
        < assessment.assessed_at_micros
    {
        return Err(CausalEvidenceTrustError::AssessmentFromFuture);
    }

    let mut seen_use = HashSet::new();
    let mut evidence_artifacts = BTreeSet::new();
    let mut admission_evidence = BTreeSet::new();

    for item in &assessment.evidence {
        for artifact_digest in &item.evidence_digests {
            artifact_digest.validate_shape()?;
            if !seen_use.insert((item.kind, *artifact_digest)) {
                return Err(CausalEvidenceTrustError::DuplicateEvidenceUse {
                    kind: item.kind,
                });
            }
            let admission = admissions
                .iter()
                .filter(|admission| admission.active_at(at_micros))
                .filter(|admission| {
                    admission.admits(*artifact_digest, item.kind, trust_policy_digest.stored())
                })
                .min_by_key(|admission| admission.admission_evidence_digest.value)
                .ok_or(CausalEvidenceTrustError::EvidenceArtifactNotAdmitted {
                    kind: item.kind,
                    digest: *artifact_digest,
                })?;
            evidence_artifacts.insert(digest_sort_key(*artifact_digest));
            admission_evidence.insert(digest_sort_key(admission.admission_evidence_digest));
        }
    }

    let evidence_artifact_digests: Vec<_> = evidence_artifacts
        .into_iter()
        .map(|(_, _, value, digest)| StoredDigest {
            algorithm: digest.algorithm,
            domain: digest.domain,
            value,
        })
        .collect();
    let admission_evidence_digests: Vec<_> = admission_evidence
        .into_iter()
        .map(|(_, _, value, digest)| StoredDigest {
            algorithm: digest.algorithm,
            domain: digest.domain,
            value,
        })
        .collect();

    let material = TrustReceiptMaterial {
        assessment_digest: assessment_digest.stored(),
        causal_assessment_policy_digest: assessment_policy_digest.stored(),
        trust_policy_digest: trust_policy_digest.stored(),
        evidence_artifact_digests: evidence_artifact_digests.clone(),
        admission_evidence_digests: admission_evidence_digests.clone(),
        evaluated_at_micros: at_micros,
    };
    let receipt_digest = hash_json(
        DigestDomain::ClinicalCausalEvidenceTrustReceipt,
        RECEIPT_TAG,
        &material,
    )?;

    Ok(CausalEvidenceTrustReceipt {
        assessment_digest: material.assessment_digest,
        causal_assessment_policy_digest: material.causal_assessment_policy_digest,
        trust_policy_digest: material.trust_policy_digest,
        evidence_artifact_digests,
        admission_evidence_digests,
        evaluated_at_micros: at_micros,
        receipt_digest: receipt_digest.stored(),
    })
}

fn digest_sort_key(
    digest: StoredDigest,
) -> (u8, u16, [u8; 32], StoredDigest) {
    let algorithm_rank = match digest.algorithm {
        mycelix_clinical_integrity::DigestAlgorithm::Blake3_256 => 0,
    };
    let domain_rank = digest.domain as u16;
    (algorithm_rank, domain_rank, digest.value, digest)
}

fn validate_window(
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    revoked_at_micros: Option<i64>,
) -> Result<(), CausalEvidenceTrustError> {
    if valid_until_micros.is_some_and(|until| until <= valid_from_micros) {
        return Err(CausalEvidenceTrustError::InvalidValidityWindow);
    }
    if revoked_at_micros.is_some_and(|revoked| revoked < valid_from_micros) {
        return Err(CausalEvidenceTrustError::RevocationPredatesValidity);
    }
    Ok(())
}

fn require_domain(
    digest: StoredDigest,
    expected: DigestDomain,
) -> Result<(), CausalEvidenceTrustError> {
    digest.validate_shape()?;
    if digest.domain != expected {
        return Err(CausalEvidenceTrustError::WrongDigestDomain {
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
) -> Result<VerifiedDigest, CausalEvidenceTrustError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| CausalEvidenceTrustError::Serialization(error.to_string()))?;
    let mut framed = Vec::with_capacity(tag.len() + 1 + encoded.len());
    framed.extend_from_slice(tag);
    framed.push(0);
    framed.extend_from_slice(&encoded);
    Ok(hash_canonical_bytes(domain, &framed)?)
}

#[derive(Debug, Error, PartialEq)]
pub enum CausalEvidenceTrustError {
    #[error("unsupported causal evidence trust policy version {0}")]
    UnsupportedPolicyVersion(u16),
    #[error("causal evidence trust policy ID is required")]
    MissingPolicyId,
    #[error("causal evidence trust policy future-skew bound is invalid")]
    InvalidFutureSkewPolicy,
    #[error("causal evidence admission requires at least one allowed evidence kind")]
    AdmissionHasNoKinds,
    #[error("causal evidence admission validity window is invalid")]
    InvalidValidityWindow,
    #[error("causal evidence admission revocation predates validity")]
    RevocationPredatesValidity,
    #[error("causal evidence trust input contains too many admissions")]
    TooManyAdmissions,
    #[error("causal evidence trust policy does not bind the supplied causal assessment policy")]
    AssessmentPolicyMismatch,
    #[error("causal assessment time is implausibly in the future")]
    AssessmentFromFuture,
    #[error("same evidence artifact is used more than once for evidence kind {kind:?}")]
    DuplicateEvidenceUse { kind: CausalEvidenceKindV1 },
    #[error("causal evidence artifact is not admitted for kind {kind:?}: {digest:?}")]
    EvidenceArtifactNotAdmitted {
        kind: CausalEvidenceKindV1,
        digest: StoredDigest,
    },
    #[error("causal assessment is invalid: {0}")]
    InvalidAssessment(String),
    #[error("digest domain mismatch: expected {expected:?}, got {actual:?}")]
    WrongDigestDomain {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("causal evidence trust artifact serialization failed: {0}")]
    Serialization(String),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
}
