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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CausalEvidenceUseV1 {
    ObservedEvent,
    ExposureAssociation,
    Factor(CausalEvidenceKindV1),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CausalEvidenceTrustPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
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

/// Verifier-owned admission for one exact artifact and one or more exact causal uses.
/// A trusted adapter must construct this from a verified deployment/registry record.
pub struct VerifiedCausalEvidenceAdmission {
    evidence_artifact_digest: StoredDigest,
    allowed_uses: BTreeSet<CausalEvidenceUseV1>,
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
        allowed_uses: Vec<CausalEvidenceUseV1>,
        valid_from_micros: i64,
        valid_until_micros: Option<i64>,
        revoked_at_micros: Option<i64>,
        trust_policy_digest: VerifiedDigest,
        admission_evidence_digest: VerifiedDigest,
    ) -> Result<Self, CausalEvidenceTrustError> {
        trust_policy_digest.require_domain(DigestDomain::ClinicalCausalEvidenceTrustPolicy)?;
        admission_evidence_digest.require_domain(DigestDomain::ClinicalArtifact)?;
        if allowed_uses.is_empty() {
            return Err(CausalEvidenceTrustError::AdmissionHasNoUses);
        }
        let original_len = allowed_uses.len();
        let allowed_uses: BTreeSet<_> = allowed_uses.into_iter().collect();
        if allowed_uses.len() != original_len {
            return Err(CausalEvidenceTrustError::DuplicateAdmissionUse);
        }
        validate_window(valid_from_micros, valid_until_micros, revoked_at_micros)?;
        Ok(Self {
            evidence_artifact_digest: evidence_artifact_digest.stored(),
            allowed_uses,
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
        usage: CausalEvidenceUseV1,
        trust_policy_digest: StoredDigest,
    ) -> bool {
        self.trust_policy_digest == trust_policy_digest
            && self.evidence_artifact_digest == artifact_digest
            && self.allowed_uses.contains(&usage)
    }
}

#[derive(Serialize)]
struct TrustReceiptMaterial {
    assessment_digest: StoredDigest,
    observed_event_digest: StoredDigest,
    association_digest: StoredDigest,
    causal_assessment_policy_digest: StoredDigest,
    trust_policy_digest: StoredDigest,
    admitted_uses: Vec<(CausalEvidenceUseV1, StoredDigest)>,
    admission_evidence_digests: Vec<StoredDigest>,
    evaluated_at_micros: i64,
}

/// Single-owner proof that the base event, exposure association, and every factor
/// artifact actually used by one assessment were admitted for their exact causal use.
pub struct CausalEvidenceTrustReceipt {
    assessment_digest: StoredDigest,
    observed_event_digest: StoredDigest,
    association_digest: StoredDigest,
    causal_assessment_policy_digest: StoredDigest,
    trust_policy_digest: StoredDigest,
    admitted_uses: Vec<(CausalEvidenceUseV1, StoredDigest)>,
    admission_evidence_digests: Vec<StoredDigest>,
    evaluated_at_micros: i64,
    receipt_digest: StoredDigest,
}

impl CausalEvidenceTrustReceipt {
    pub fn assessment_digest(&self) -> StoredDigest {
        self.assessment_digest
    }
    pub fn observed_event_digest(&self) -> StoredDigest {
        self.observed_event_digest
    }
    pub fn association_digest(&self) -> StoredDigest {
        self.association_digest
    }
    pub fn causal_assessment_policy_digest(&self) -> StoredDigest {
        self.causal_assessment_policy_digest
    }
    pub fn trust_policy_digest(&self) -> StoredDigest {
        self.trust_policy_digest
    }
    pub fn admitted_uses(&self) -> &[(CausalEvidenceUseV1, StoredDigest)] {
        &self.admitted_uses
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
    let association_digest = association
        .verified_digest()
        .map_err(|error| CausalEvidenceTrustError::InvalidAssessment(error.to_string()))?;
    let trust_policy_digest = trust_policy.verified_digest()?;

    if at_micros.saturating_add(trust_policy.max_future_skew_micros)
        < assessment.assessed_at_micros
    {
        return Err(CausalEvidenceTrustError::AssessmentFromFuture);
    }

    let mut required: Vec<(CausalEvidenceUseV1, StoredDigest)> = vec![
        (
            CausalEvidenceUseV1::ObservedEvent,
            association.observed_event_digest,
        ),
        (
            CausalEvidenceUseV1::ExposureAssociation,
            association_digest.stored(),
        ),
    ];
    for item in &assessment.evidence {
        for artifact_digest in &item.evidence_digests {
            required.push((CausalEvidenceUseV1::Factor(item.kind), *artifact_digest));
        }
    }
    required.sort();
    if required.windows(2).any(|window| window[0] == window[1]) {
        return Err(CausalEvidenceTrustError::DuplicateEvidenceUse);
    }

    let mut admission_evidence = BTreeSet::new();
    for (usage, artifact_digest) in &required {
        artifact_digest.validate_shape()?;
        let admission = admissions
            .iter()
            .filter(|admission| admission.active_at(at_micros))
            .filter(|admission| {
                admission.admits(*artifact_digest, *usage, trust_policy_digest.stored())
            })
            .min_by_key(|admission| admission.admission_evidence_digest)
            .ok_or(CausalEvidenceTrustError::EvidenceArtifactNotAdmitted {
                usage: *usage,
                digest: *artifact_digest,
            })?;
        admission_evidence.insert(admission.admission_evidence_digest);
    }
    let admission_evidence_digests: Vec<_> = admission_evidence.into_iter().collect();

    let material = TrustReceiptMaterial {
        assessment_digest: assessment_digest.stored(),
        observed_event_digest: association.observed_event_digest,
        association_digest: association_digest.stored(),
        causal_assessment_policy_digest: assessment_policy_digest.stored(),
        trust_policy_digest: trust_policy_digest.stored(),
        admitted_uses: required.clone(),
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
        observed_event_digest: material.observed_event_digest,
        association_digest: material.association_digest,
        causal_assessment_policy_digest: material.causal_assessment_policy_digest,
        trust_policy_digest: material.trust_policy_digest,
        admitted_uses: required,
        admission_evidence_digests,
        evaluated_at_micros: at_micros,
        receipt_digest: receipt_digest.stored(),
    })
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
    #[error("causal evidence admission requires at least one allowed use")]
    AdmissionHasNoUses,
    #[error("causal evidence admission repeats an allowed use")]
    DuplicateAdmissionUse,
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
    #[error("same exact causal evidence use appears more than once")]
    DuplicateEvidenceUse,
    #[error("causal evidence artifact is not admitted for use {usage:?}: {digest:?}")]
    EvidenceArtifactNotAdmitted {
        usage: CausalEvidenceUseV1,
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
