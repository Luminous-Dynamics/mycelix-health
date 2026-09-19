#![deny(unsafe_code)]
//! Cryptographically policy-bound authority evaluation.
//!
//! The legacy `mycelix-clinical-authority` API accepts a policy digest and its
//! credential/scope requirements independently. This crate closes that binding gap
//! for high-assurance workflows by deriving the digest from one typed policy and
//! constructing the underlying authority request internally.

use mycelix_clinical_authority::{
    evaluate_authority, AuthorityDecision, AuthorityError, AuthorityPermit, AuthorityPurpose,
    AuthorityRequest, CredentialKind, EvidenceDigest, JurisdictionCode, PrincipalBinding,
    ResolvedCredentialEvidence, ScopeCode,
};
use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const POLICY_TAG: &[u8] = b"mycelix-health/authority-policy-v1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuthorityPurposeV1 {
    ClinicalReview,
    Prescribe,
    Dispense,
    Administer,
    TrialEligibilityReview,
    ResearchEthicsReview,
}

impl AuthorityPurposeV1 {
    fn legacy(self) -> AuthorityPurpose {
        match self {
            Self::ClinicalReview => AuthorityPurpose::ClinicalReview,
            Self::Prescribe => AuthorityPurpose::Prescribe,
            Self::Dispense => AuthorityPurpose::Dispense,
            Self::Administer => AuthorityPurpose::Administer,
            Self::TrialEligibilityReview => AuthorityPurpose::TrialEligibilityReview,
            Self::ResearchEthicsReview => AuthorityPurpose::ResearchEthicsReview,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CredentialRequirementV1 {
    PractitionerLicense,
    ControlledSubstanceRegistration,
    OrganizationDelegation,
    SpecialtyCertification,
    ResearchEthicsAppointment,
}

impl CredentialRequirementV1 {
    fn legacy(self) -> CredentialKind {
        match self {
            Self::PractitionerLicense => CredentialKind::PractitionerLicense,
            Self::ControlledSubstanceRegistration => CredentialKind::ControlledSubstanceRegistration,
            Self::OrganizationDelegation => CredentialKind::OrganizationDelegation,
            Self::SpecialtyCertification => CredentialKind::SpecialtyCertification,
            Self::ResearchEthicsAppointment => CredentialKind::ResearchEthicsAppointment,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthorityPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub purpose: AuthorityPurposeV1,
    pub jurisdiction: JurisdictionCode,
    pub required_credentials: Vec<CredentialRequirementV1>,
    pub required_scopes: Vec<ScopeCode>,
}

#[derive(Serialize)]
struct CanonicalAuthorityPolicyV1<'a> {
    schema_version: u16,
    policy_id: &'a str,
    purpose: AuthorityPurposeV1,
    jurisdiction_system: &'a str,
    jurisdiction_code: &'a str,
    required_credentials: Vec<CredentialRequirementV1>,
    required_scopes: Vec<CanonicalScope<'a>>,
}

#[derive(Serialize)]
struct CanonicalScope<'a> {
    system: &'a str,
    code: &'a str,
}

impl AuthorityPolicyV1 {
    pub fn validate(&self) -> Result<(), PolicyBoundAuthorityError> {
        if self.schema_version != 1 {
            return Err(PolicyBoundAuthorityError::UnsupportedPolicyVersion(
                self.schema_version,
            ));
        }
        if self.policy_id.trim().is_empty() {
            return Err(PolicyBoundAuthorityError::MissingPolicyId);
        }
        self.jurisdiction
            .validate()
            .map_err(PolicyBoundAuthorityError::Authority)?;
        if self.required_credentials.is_empty() {
            return Err(PolicyBoundAuthorityError::MissingCredentialRequirements);
        }
        if self.required_scopes.is_empty() {
            return Err(PolicyBoundAuthorityError::MissingScopeRequirements);
        }

        let mut credentials = self.required_credentials.clone();
        credentials.sort_unstable();
        if credentials.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(PolicyBoundAuthorityError::DuplicateCredentialRequirement);
        }

        let mut scopes: Vec<_> = self
            .required_scopes
            .iter()
            .map(|scope| {
                scope
                    .validate()
                    .map_err(PolicyBoundAuthorityError::Authority)?;
                Ok((scope.system.as_str(), scope.code.as_str()))
            })
            .collect::<Result<_, PolicyBoundAuthorityError>>()?;
        scopes.sort_unstable();
        if scopes.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(PolicyBoundAuthorityError::DuplicateScopeRequirement);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, PolicyBoundAuthorityError> {
        self.validate()?;
        let mut credentials = self.required_credentials.clone();
        credentials.sort_unstable();
        let mut scopes: Vec<_> = self
            .required_scopes
            .iter()
            .map(|scope| CanonicalScope {
                system: scope.system.as_str(),
                code: scope.code.as_str(),
            })
            .collect();
        scopes.sort_unstable_by(|left, right| {
            (left.system, left.code).cmp(&(right.system, right.code))
        });
        let material = CanonicalAuthorityPolicyV1 {
            schema_version: self.schema_version,
            policy_id: self.policy_id.as_str(),
            purpose: self.purpose,
            jurisdiction_system: self.jurisdiction.system.as_str(),
            jurisdiction_code: self.jurisdiction.code.as_str(),
            required_credentials: credentials,
            required_scopes: scopes,
        };
        let encoded = serde_json::to_vec(&material)
            .map_err(|error| PolicyBoundAuthorityError::Serialization(error.to_string()))?;
        let mut framed = Vec::with_capacity(POLICY_TAG.len() + 1 + encoded.len());
        framed.extend_from_slice(POLICY_TAG);
        framed.push(0);
        framed.extend_from_slice(&encoded);
        Ok(hash_canonical_bytes(DigestDomain::AuthorityPolicy, &framed)?)
    }

    fn authority_request(
        &self,
        principal: PrincipalBinding,
        target_artifact_digest: EvidenceDigest,
        evaluated_at_micros: i64,
    ) -> Result<AuthorityRequest, PolicyBoundAuthorityError> {
        let policy_digest = self.verified_digest()?;
        Ok(AuthorityRequest {
            principal,
            target_artifact_digest,
            policy_digest: EvidenceDigest(policy_digest.value()),
            purpose: self.purpose.legacy(),
            jurisdiction: self.jurisdiction.clone(),
            required_credential_kinds: self
                .required_credentials
                .iter()
                .copied()
                .map(CredentialRequirementV1::legacy)
                .collect(),
            required_scopes: self.required_scopes.clone(),
            evaluated_at_micros,
        })
    }
}

/// Non-cloneable, non-serializable authority proof whose requirements are bound to
/// the exact policy digest exposed by `policy_digest()`.
pub struct PolicyBoundAuthorityPermit {
    inner: AuthorityPermit,
    policy_digest: StoredDigest,
}

impl PolicyBoundAuthorityPermit {
    pub fn principal(&self) -> PrincipalBinding {
        self.inner.principal()
    }

    pub fn purpose(&self) -> AuthorityPurpose {
        self.inner.purpose()
    }

    pub fn jurisdiction(&self) -> &JurisdictionCode {
        self.inner.jurisdiction()
    }

    pub fn target_artifact_digest(&self) -> EvidenceDigest {
        self.inner.target_artifact_digest()
    }

    pub fn policy_digest(&self) -> StoredDigest {
        self.policy_digest
    }

    pub fn evaluated_at_micros(&self) -> i64 {
        self.inner.evaluated_at_micros()
    }

    pub fn supporting_evidence_digests(&self) -> &[EvidenceDigest] {
        self.inner.supporting_evidence_digests()
    }
}

pub struct PolicyBoundAuthorityEvaluation {
    pub decision: AuthorityDecision,
    pub denials: Vec<mycelix_clinical_authority::AuthorityDenial>,
    permit: Option<PolicyBoundAuthorityPermit>,
}

impl PolicyBoundAuthorityEvaluation {
    pub fn into_permit(self) -> Option<PolicyBoundAuthorityPermit> {
        self.permit
    }
}

pub fn evaluate_policy_bound_authority(
    policy: &AuthorityPolicyV1,
    principal: PrincipalBinding,
    target_artifact_digest: EvidenceDigest,
    credentials: &[ResolvedCredentialEvidence],
    evaluated_at_micros: i64,
) -> Result<PolicyBoundAuthorityEvaluation, PolicyBoundAuthorityError> {
    let request = policy.authority_request(principal, target_artifact_digest, evaluated_at_micros)?;
    let policy_digest = policy.verified_digest()?.stored();
    let evaluation = evaluate_authority(&request, credentials)?;
    let decision = evaluation.decision;
    let denials = evaluation.denials.clone();
    let permit = evaluation.into_permit().map(|inner| PolicyBoundAuthorityPermit {
        inner,
        policy_digest,
    });
    Ok(PolicyBoundAuthorityEvaluation {
        decision,
        denials,
        permit,
    })
}

#[derive(Debug, Error)]
pub enum PolicyBoundAuthorityError {
    #[error("unsupported authority policy version {0}")]
    UnsupportedPolicyVersion(u16),
    #[error("authority policy ID is required")]
    MissingPolicyId,
    #[error("authority policy requires at least one credential kind")]
    MissingCredentialRequirements,
    #[error("authority policy contains duplicate credential requirements")]
    DuplicateCredentialRequirement,
    #[error("authority policy requires at least one scope")]
    MissingScopeRequirements,
    #[error("authority policy contains duplicate scope requirements")]
    DuplicateScopeRequirement,
    #[error("authority policy serialization failed: {0}")]
    Serialization(String),
    #[error(transparent)]
    Authority(#[from] AuthorityError),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(scopes: Vec<ScopeCode>) -> AuthorityPolicyV1 {
        AuthorityPolicyV1 {
            schema_version: 1,
            policy_id: "causal-review-v1".into(),
            purpose: AuthorityPurposeV1::ClinicalReview,
            jurisdiction: JurisdictionCode {
                system: "urn:iso:std:iso:3166-2".into(),
                code: "US-TX".into(),
            },
            required_credentials: vec![CredentialRequirementV1::PractitionerLicense],
            required_scopes: scopes,
        }
    }

    fn scope(code: &str) -> ScopeCode {
        ScopeCode {
            system: "https://mycelix.health/authority-scope/v1".into(),
            code: code.into(),
        }
    }

    #[test]
    fn requirement_order_does_not_change_policy_identity() {
        let a = policy(vec![scope("causality.review"), scope("clinical.review")]);
        let b = policy(vec![scope("clinical.review"), scope("causality.review")]);
        assert_eq!(
            a.verified_digest().unwrap().stored(),
            b.verified_digest().unwrap().stored()
        );
    }

    #[test]
    fn changing_scope_changes_policy_identity() {
        let a = policy(vec![scope("causality.review")]);
        let b = policy(vec![scope("medication.review")]);
        assert_ne!(
            a.verified_digest().unwrap().stored(),
            b.verified_digest().unwrap().stored()
        );
    }

    #[test]
    fn duplicate_requirement_is_rejected() {
        let result = policy(vec![scope("causality.review"), scope("causality.review")]).validate();
        assert!(matches!(
            result,
            Err(PolicyBoundAuthorityError::DuplicateScopeRequirement)
        ));
    }
}
