use mycelix_clinical_authority::{AuthorityPurpose, EvidenceDigest, JurisdictionCode, PrincipalBinding};
use mycelix_clinical_authority_policy::{AuthorityPolicyV1, PolicyBoundAuthorityPermit};
use mycelix_clinical_causality::{
    CausalAssessmentPolicyV1, CausalAssessmentV1, CausalConclusionV1, CausalEvidenceKindV1,
    EvidenceDirectionV1, ExposureAssociationV1,
};
use mycelix_clinical_causality_trust::CausalEvidenceTrustReceipt;
use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const POLICY_TAG: &[u8] = b"mycelix-health/clinical-causal-qualification-policy-v1";
const RECEIPT_TAG: &[u8] = b"mycelix-health/clinical-causal-qualified-receipt-v1";
const MAX_AGE_MICROS: i64 = 86_400_000_000;
const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CausalQualificationPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub causal_assessment_policy_digest: StoredDigest,
    pub causal_evidence_trust_policy_digest: StoredDigest,
    pub assessor_authority_policy_digest: StoredDigest,
    pub max_assessment_age_micros: i64,
    pub max_authority_age_micros: i64,
    pub max_evidence_trust_age_micros: i64,
    pub max_future_skew_micros: i64,
}

impl CausalQualificationPolicyV1 {
    pub fn validate(&self) -> Result<(), CausalQualificationError> {
        if self.schema_version != 1 {
            return Err(CausalQualificationError::UnsupportedPolicyVersion(self.schema_version));
        }
        if self.policy_id.trim().is_empty() {
            return Err(CausalQualificationError::MissingPolicyId);
        }
        require_domain(
            self.causal_assessment_policy_digest,
            DigestDomain::ClinicalCausalAssessmentPolicy,
        )?;
        require_domain(
            self.causal_evidence_trust_policy_digest,
            DigestDomain::ClinicalCausalEvidenceTrustPolicy,
        )?;
        require_domain(
            self.assessor_authority_policy_digest,
            DigestDomain::AuthorityPolicy,
        )?;
        validate_age(self.max_assessment_age_micros)?;
        validate_age(self.max_authority_age_micros)?;
        validate_age(self.max_evidence_trust_age_micros)?;
        if self.max_future_skew_micros < 0
            || self.max_future_skew_micros > MAX_FUTURE_SKEW_MICROS
        {
            return Err(CausalQualificationError::InvalidFutureSkewPolicy);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, CausalQualificationError> {
        self.validate()?;
        hash_json(DigestDomain::ClinicalCausalQualificationPolicy, POLICY_TAG, self)
    }
}

/// Single-owner capability proving one exact causal assessment crossed the
/// evidence-trust and policy-bound assessor-authority boundaries.
pub struct QualifiedCausalAssessmentCapability {
    assessment_digest: StoredDigest,
    observed_event_digest: StoredDigest,
    association_digest: StoredDigest,
    causal_assessment_policy_digest: StoredDigest,
    causal_evidence_trust_policy_digest: StoredDigest,
    causal_evidence_trust_receipt_digest: StoredDigest,
    qualification_policy_digest: StoredDigest,
    assessor_principal: PrincipalBinding,
    assessor_jurisdiction: JurisdictionCode,
    authority_policy_digest: StoredDigest,
    authority_evidence_digests: Vec<[u8; 32]>,
    conclusion: CausalConclusionV1,
    qualified_at_micros: i64,
}

impl QualifiedCausalAssessmentCapability {
    pub fn assessment_digest(&self) -> StoredDigest {
        self.assessment_digest
    }

    pub fn conclusion(&self) -> CausalConclusionV1 {
        self.conclusion
    }

    pub fn assessor_principal(&self) -> PrincipalBinding {
        self.assessor_principal
    }

    pub fn into_audit_receipt(
        self,
    ) -> Result<QualifiedCausalAssessmentReceiptV1, CausalQualificationError> {
        let material = QualifiedReceiptMaterial {
            schema_version: 1,
            assessment_digest: self.assessment_digest,
            observed_event_digest: self.observed_event_digest,
            association_digest: self.association_digest,
            causal_assessment_policy_digest: self.causal_assessment_policy_digest,
            causal_evidence_trust_policy_digest: self.causal_evidence_trust_policy_digest,
            causal_evidence_trust_receipt_digest: self.causal_evidence_trust_receipt_digest,
            qualification_policy_digest: self.qualification_policy_digest,
            assessor_principal_binding: self.assessor_principal.0,
            assessor_jurisdiction: self.assessor_jurisdiction,
            authority_policy_digest: self.authority_policy_digest,
            authority_evidence_digests: self.authority_evidence_digests,
            conclusion: self.conclusion,
            qualified_at_micros: self.qualified_at_micros,
        };
        let receipt_digest = material.verified_digest()?.stored();
        Ok(QualifiedCausalAssessmentReceiptV1 {
            schema_version: material.schema_version,
            assessment_digest: material.assessment_digest,
            observed_event_digest: material.observed_event_digest,
            association_digest: material.association_digest,
            causal_assessment_policy_digest: material.causal_assessment_policy_digest,
            causal_evidence_trust_policy_digest: material.causal_evidence_trust_policy_digest,
            causal_evidence_trust_receipt_digest: material.causal_evidence_trust_receipt_digest,
            qualification_policy_digest: material.qualification_policy_digest,
            assessor_principal_binding: material.assessor_principal_binding,
            assessor_jurisdiction: material.assessor_jurisdiction,
            authority_policy_digest: material.authority_policy_digest,
            authority_evidence_digests: material.authority_evidence_digests,
            conclusion: material.conclusion,
            qualified_at_micros: material.qualified_at_micros,
            receipt_digest,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QualifiedCausalAssessmentReceiptV1 {
    pub schema_version: u16,
    pub assessment_digest: StoredDigest,
    pub observed_event_digest: StoredDigest,
    pub association_digest: StoredDigest,
    pub causal_assessment_policy_digest: StoredDigest,
    pub causal_evidence_trust_policy_digest: StoredDigest,
    pub causal_evidence_trust_receipt_digest: StoredDigest,
    pub qualification_policy_digest: StoredDigest,
    pub assessor_principal_binding: [u8; 32],
    pub assessor_jurisdiction: JurisdictionCode,
    pub authority_policy_digest: StoredDigest,
    pub authority_evidence_digests: Vec<[u8; 32]>,
    pub conclusion: CausalConclusionV1,
    pub qualified_at_micros: i64,
    pub receipt_digest: StoredDigest,
}

#[derive(Serialize)]
struct QualifiedReceiptMaterial {
    schema_version: u16,
    assessment_digest: StoredDigest,
    observed_event_digest: StoredDigest,
    association_digest: StoredDigest,
    causal_assessment_policy_digest: StoredDigest,
    causal_evidence_trust_policy_digest: StoredDigest,
    causal_evidence_trust_receipt_digest: StoredDigest,
    qualification_policy_digest: StoredDigest,
    assessor_principal_binding: [u8; 32],
    assessor_jurisdiction: JurisdictionCode,
    authority_policy_digest: StoredDigest,
    authority_evidence_digests: Vec<[u8; 32]>,
    conclusion: CausalConclusionV1,
    qualified_at_micros: i64,
}

impl QualifiedReceiptMaterial {
    fn verified_digest(&self) -> Result<VerifiedDigest, CausalQualificationError> {
        hash_json(DigestDomain::ClinicalCausalQualifiedReceipt, RECEIPT_TAG, self)
    }
}

impl QualifiedCausalAssessmentReceiptV1 {
    pub fn validate_shape(&self) -> Result<(), CausalQualificationError> {
        if self.schema_version != 1 {
            return Err(CausalQualificationError::UnsupportedReceiptVersion(self.schema_version));
        }
        require_domain(self.assessment_digest, DigestDomain::ClinicalCausalAssessment)?;
        require_domain(self.observed_event_digest, DigestDomain::ClinicalObservedEvent)?;
        require_domain(self.association_digest, DigestDomain::ClinicalExposureAssociation)?;
        require_domain(
            self.causal_assessment_policy_digest,
            DigestDomain::ClinicalCausalAssessmentPolicy,
        )?;
        require_domain(
            self.causal_evidence_trust_policy_digest,
            DigestDomain::ClinicalCausalEvidenceTrustPolicy,
        )?;
        require_domain(
            self.causal_evidence_trust_receipt_digest,
            DigestDomain::ClinicalCausalEvidenceTrustReceipt,
        )?;
        require_domain(
            self.qualification_policy_digest,
            DigestDomain::ClinicalCausalQualificationPolicy,
        )?;
        require_domain(self.authority_policy_digest, DigestDomain::AuthorityPolicy)?;
        require_domain(
            self.receipt_digest,
            DigestDomain::ClinicalCausalQualifiedReceipt,
        )?;
        if self.assessor_principal_binding == [0u8; 32] {
            return Err(CausalQualificationError::ZeroAssessorPrincipal);
        }
        if self.authority_evidence_digests.is_empty()
            || self
                .authority_evidence_digests
                .iter()
                .any(|digest| *digest == [0u8; 32])
        {
            return Err(CausalQualificationError::InvalidAuthorityEvidence);
        }
        self.assessor_jurisdiction
            .validate()
            .map_err(|error| CausalQualificationError::Authority(error.to_string()))?;
        let material = QualifiedReceiptMaterial {
            schema_version: self.schema_version,
            assessment_digest: self.assessment_digest,
            observed_event_digest: self.observed_event_digest,
            association_digest: self.association_digest,
            causal_assessment_policy_digest: self.causal_assessment_policy_digest,
            causal_evidence_trust_policy_digest: self.causal_evidence_trust_policy_digest,
            causal_evidence_trust_receipt_digest: self.causal_evidence_trust_receipt_digest,
            qualification_policy_digest: self.qualification_policy_digest,
            assessor_principal_binding: self.assessor_principal_binding,
            assessor_jurisdiction: self.assessor_jurisdiction.clone(),
            authority_policy_digest: self.authority_policy_digest,
            authority_evidence_digests: self.authority_evidence_digests.clone(),
            conclusion: self.conclusion,
            qualified_at_micros: self.qualified_at_micros,
        };
        if material.verified_digest()?.stored() != self.receipt_digest {
            return Err(CausalQualificationError::ReceiptDigestMismatch);
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn qualify_causal_assessment(
    assessment: &CausalAssessmentV1,
    association: &ExposureAssociationV1,
    assessment_policy: &CausalAssessmentPolicyV1,
    evidence_trust: CausalEvidenceTrustReceipt,
    authority_policy: &AuthorityPolicyV1,
    authority: PolicyBoundAuthorityPermit,
    qualification_policy: &CausalQualificationPolicyV1,
    at_micros: i64,
) -> Result<QualifiedCausalAssessmentCapability, CausalQualificationError> {
    assessment
        .validate_against(association, assessment_policy)
        .map_err(|error| CausalQualificationError::Assessment(error.to_string()))?;
    qualification_policy.validate()?;

    let assessment_digest = assessment
        .verified_digest(association, assessment_policy)
        .map_err(|error| CausalQualificationError::Assessment(error.to_string()))?;
    let association_digest = association
        .verified_digest()
        .map_err(|error| CausalQualificationError::Assessment(error.to_string()))?;
    let assessment_policy_digest = assessment_policy
        .verified_digest()
        .map_err(|error| CausalQualificationError::Assessment(error.to_string()))?;
    let authority_policy_digest = authority_policy
        .verified_digest()
        .map_err(|error| CausalQualificationError::Authority(error.to_string()))?;
    let qualification_policy_digest = qualification_policy.verified_digest()?;

    if qualification_policy.causal_assessment_policy_digest != assessment_policy_digest.stored() {
        return Err(CausalQualificationError::AssessmentPolicyMismatch);
    }
    if qualification_policy.causal_evidence_trust_policy_digest != evidence_trust.trust_policy_digest() {
        return Err(CausalQualificationError::EvidenceTrustPolicyMismatch);
    }
    if qualification_policy.assessor_authority_policy_digest != authority_policy_digest.stored() {
        return Err(CausalQualificationError::AuthorityPolicyMismatch);
    }

    if evidence_trust.assessment_digest() != assessment_digest.stored()
        || evidence_trust.association_digest() != association_digest.stored()
        || evidence_trust.observed_event_digest() != association.observed_event_digest
        || evidence_trust.causal_assessment_policy_digest() != assessment_policy_digest.stored()
    {
        return Err(CausalQualificationError::EvidenceTrustLineageMismatch);
    }

    if authority_policy.purpose != mycelix_clinical_authority_policy::AuthorityPurposeV1::ClinicalReview
        || authority.purpose() != AuthorityPurpose::ClinicalReview
    {
        return Err(CausalQualificationError::WrongAuthorityPurpose);
    }
    if authority.policy_digest() != authority_policy_digest.stored() {
        return Err(CausalQualificationError::AuthorityPolicyMismatch);
    }
    if authority.target_artifact_digest() != EvidenceDigest(assessment_digest.value()) {
        return Err(CausalQualificationError::AuthorityTargetMismatch);
    }

    validate_freshness(
        assessment.assessed_at_micros,
        qualification_policy.max_assessment_age_micros,
        qualification_policy.max_future_skew_micros,
        at_micros,
        CausalQualificationError::AssessmentTooOld,
        CausalQualificationError::AssessmentFromFuture,
    )?;
    validate_freshness(
        authority.evaluated_at_micros(),
        qualification_policy.max_authority_age_micros,
        qualification_policy.max_future_skew_micros,
        at_micros,
        CausalQualificationError::AuthorityTooOld,
        CausalQualificationError::AuthorityFromFuture,
    )?;
    validate_freshness(
        evidence_trust.evaluated_at_micros(),
        qualification_policy.max_evidence_trust_age_micros,
        qualification_policy.max_future_skew_micros,
        at_micros,
        CausalQualificationError::EvidenceTrustTooOld,
        CausalQualificationError::EvidenceTrustFromFuture,
    )?;

    if matches!(
        assessment.conclusion,
        CausalConclusionV1::EvidenceSuggestsRelationship
            | CausalConclusionV1::EvidenceSupportsRelationship
    ) {
        let has_temporal_plausibility = assessment.evidence.iter().any(|item| {
            item.kind == CausalEvidenceKindV1::TemporalPlausibility
                && item.direction == EvidenceDirectionV1::SupportsRelationship
                && !item.evidence_digests.is_empty()
        });
        if !has_temporal_plausibility {
            return Err(CausalQualificationError::PositiveConclusionWithoutTemporalPlausibility);
        }
    }

    let authority_evidence_digests: Vec<_> = authority
        .supporting_evidence_digests()
        .iter()
        .map(|digest| digest.0)
        .collect();
    if authority_evidence_digests.is_empty() {
        return Err(CausalQualificationError::InvalidAuthorityEvidence);
    }

    Ok(QualifiedCausalAssessmentCapability {
        assessment_digest: assessment_digest.stored(),
        observed_event_digest: association.observed_event_digest,
        association_digest: association_digest.stored(),
        causal_assessment_policy_digest: assessment_policy_digest.stored(),
        causal_evidence_trust_policy_digest: evidence_trust.trust_policy_digest(),
        causal_evidence_trust_receipt_digest: evidence_trust.receipt_digest(),
        qualification_policy_digest: qualification_policy_digest.stored(),
        assessor_principal: authority.principal(),
        assessor_jurisdiction: authority.jurisdiction().clone(),
        authority_policy_digest: authority_policy_digest.stored(),
        authority_evidence_digests,
        conclusion: assessment.conclusion,
        qualified_at_micros: at_micros,
    })
}

fn validate_age(value: i64) -> Result<(), CausalQualificationError> {
    if value <= 0 || value > MAX_AGE_MICROS {
        return Err(CausalQualificationError::InvalidAgePolicy);
    }
    Ok(())
}

fn validate_freshness(
    evidence_at: i64,
    max_age: i64,
    max_future_skew: i64,
    now: i64,
    old_error: CausalQualificationError,
    future_error: CausalQualificationError,
) -> Result<(), CausalQualificationError> {
    if evidence_at > now.saturating_add(max_future_skew) {
        return Err(future_error);
    }
    if now.saturating_sub(evidence_at) > max_age {
        return Err(old_error);
    }
    Ok(())
}

fn require_domain(
    digest: StoredDigest,
    expected: DigestDomain,
) -> Result<(), CausalQualificationError> {
    digest.validate_shape()?;
    if digest.domain != expected {
        return Err(CausalQualificationError::WrongDigestDomain {
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
) -> Result<VerifiedDigest, CausalQualificationError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| CausalQualificationError::Serialization(error.to_string()))?;
    let mut framed = Vec::with_capacity(tag.len() + 1 + encoded.len());
    framed.extend_from_slice(tag);
    framed.push(0);
    framed.extend_from_slice(&encoded);
    Ok(hash_canonical_bytes(domain, &framed)?)
}

#[derive(Debug, Error, PartialEq)]
pub enum CausalQualificationError {
    #[error("unsupported causal qualification policy version {0}")]
    UnsupportedPolicyVersion(u16),
    #[error("unsupported qualified causal receipt version {0}")]
    UnsupportedReceiptVersion(u16),
    #[error("causal qualification policy ID is required")]
    MissingPolicyId,
    #[error("causal qualification policy references the wrong causal-assessment policy")]
    AssessmentPolicyMismatch,
    #[error("causal qualification policy references the wrong evidence-trust policy")]
    EvidenceTrustPolicyMismatch,
    #[error("causal qualification policy references the wrong assessor-authority policy")]
    AuthorityPolicyMismatch,
    #[error("causal qualification age policy must be positive and at most 24 hours")]
    InvalidAgePolicy,
    #[error("causal qualification future-skew policy is invalid")]
    InvalidFutureSkewPolicy,
    #[error("causal assessment is invalid: {0}")]
    Assessment(String),
    #[error("causal evidence trust receipt does not match assessment/event/association/policy")]
    EvidenceTrustLineageMismatch,
    #[error("causal assessment requires policy-bound ClinicalReview authority")]
    WrongAuthorityPurpose,
    #[error("assessor authority targets a different causal assessment")]
    AuthorityTargetMismatch,
    #[error("causal assessment is too old for qualification")]
    AssessmentTooOld,
    #[error("causal assessment is implausibly in the future")]
    AssessmentFromFuture,
    #[error("assessor authority is too old for qualification")]
    AuthorityTooOld,
    #[error("assessor authority is implausibly in the future")]
    AuthorityFromFuture,
    #[error("causal evidence trust decision is too old for qualification")]
    EvidenceTrustTooOld,
    #[error("causal evidence trust decision is implausibly in the future")]
    EvidenceTrustFromFuture,
    #[error("positive causal conclusion lacks separately evidenced temporal plausibility")]
    PositiveConclusionWithoutTemporalPlausibility,
    #[error("assessor authority contains no supporting credential/issuer/status evidence")]
    InvalidAuthorityEvidence,
    #[error("assessor principal binding cannot be zero")]
    ZeroAssessorPrincipal,
    #[error("qualified causal receipt digest does not match its exact contents")]
    ReceiptDigestMismatch,
    #[error("authority data is invalid: {0}")]
    Authority(String),
    #[error("digest domain mismatch: expected {expected:?}, got {actual:?}")]
    WrongDigestDomain {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("causal qualification artifact serialization failed: {0}")]
    Serialization(String),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
}
