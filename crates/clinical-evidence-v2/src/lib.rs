#![deny(unsafe_code)]
//! Typed artifact identities for strict Mycelix clinical evidence capsules.
//!
//! The v1 clinical-evidence crate intentionally carries a generic `ContentDigest`.
//! That remains useful for legacy and heterogeneous evidence sources, but is too
//! weak for model-backed clinical inference where a digest must preserve the exact
//! canonicalization/domain that produced it.
//!
//! This crate therefore wraps a validated v1 capsule with a strict typed evidence
//! layer. It does not replace v1 in place and it does not confer clinical authority.

use mycelix_clinical_evidence::{ClinicalEvidenceCapsule, EvidenceRole};
use mycelix_clinical_fact_snapshot::ClinicalFactSnapshotDigestV1;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

pub const CLINICAL_EVIDENCE_CAPSULE_V2_VERSION: u16 = 2;
pub const TYPED_EVIDENCE_IDENTITY_VERSION: u16 = 1;
pub const MYCELIX_CLINICAL_FACT_SNAPSHOT_NAMESPACE_V1: &str =
    "mycelix/clinical-fact-snapshot/v1";

/// Digest algorithms supported by the typed-evidence identity v1 contract.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TypedEvidenceDigestAlgorithmV1 {
    Blake3_256,
}

/// Exact semantic artifact identity.
///
/// `namespace` names the canonicalization/digest domain, not merely the broad
/// content type. Two equal digest byte strings under different namespaces are
/// intentionally different evidence identities.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct TypedEvidenceIdentityV1 {
    pub schema_version: u16,
    pub namespace: String,
    pub artifact_id: String,
    pub algorithm: TypedEvidenceDigestAlgorithmV1,
    pub digest: [u8; 32],
}

impl TypedEvidenceIdentityV1 {
    pub fn validate(&self) -> Result<(), ClinicalEvidenceV2Error> {
        if self.schema_version != TYPED_EVIDENCE_IDENTITY_VERSION {
            return Err(ClinicalEvidenceV2Error::UnsupportedTypedIdentityVersion(
                self.schema_version,
            ));
        }
        validate_token(&self.namespace)?;
        validate_token(&self.artifact_id)?;
        if self.digest == [0u8; 32] {
            return Err(ClinicalEvidenceV2Error::ZeroTypedDigest);
        }
        Ok(())
    }

    /// Construct the exact Mycelix ClinicalFact snapshot identity without
    /// flattening its dedicated digest domain into a generic string digest.
    #[must_use]
    pub fn clinical_fact_snapshot_v1(
        fact_id: impl Into<String>,
        digest: ClinicalFactSnapshotDigestV1,
    ) -> Self {
        Self {
            schema_version: TYPED_EVIDENCE_IDENTITY_VERSION,
            namespace: MYCELIX_CLINICAL_FACT_SNAPSHOT_NAMESPACE_V1.to_string(),
            artifact_id: fact_id.into(),
            algorithm: TypedEvidenceDigestAlgorithmV1::Blake3_256,
            digest: digest.into_bytes(),
        }
    }
}

/// Typed identity for one fact used by a clinical evidence capsule.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TypedFactEvidenceV1 {
    pub fact_id: String,
    pub role: EvidenceRole,
    pub identity: TypedEvidenceIdentityV1,
}

impl TypedFactEvidenceV1 {
    pub fn validate(&self) -> Result<(), ClinicalEvidenceV2Error> {
        validate_token(&self.fact_id)?;
        self.identity.validate()?;
        if self.identity.namespace != MYCELIX_CLINICAL_FACT_SNAPSHOT_NAMESPACE_V1 {
            return Err(ClinicalEvidenceV2Error::UnexpectedFactEvidenceNamespace);
        }
        if self.identity.artifact_id != self.fact_id {
            return Err(ClinicalEvidenceV2Error::FactArtifactIdMismatch);
        }
        Ok(())
    }

    #[must_use]
    pub fn from_clinical_fact_snapshot(
        fact_id: impl Into<String>,
        role: EvidenceRole,
        digest: ClinicalFactSnapshotDigestV1,
    ) -> Self {
        let fact_id = fact_id.into();
        Self {
            identity: TypedEvidenceIdentityV1::clinical_fact_snapshot_v1(
                fact_id.clone(),
                digest,
            ),
            fact_id,
            role,
        }
    }
}

/// Strict typed-evidence wrapper around an otherwise valid v1 capsule.
///
/// v2 intentionally does not expose or duplicate `promotion_state()`. It proves
/// representation integrity only. Separate qualification/promotion code must
/// decide whether a validated v2 capsule may advance in a clinical workflow.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClinicalEvidenceCapsuleV2 {
    pub schema_version: u16,
    pub core: ClinicalEvidenceCapsule,
    pub typed_fact_evidence: Vec<TypedFactEvidenceV1>,
}

impl ClinicalEvidenceCapsuleV2 {
    pub fn validate(&self) -> Result<(), ClinicalEvidenceV2Error> {
        if self.schema_version != CLINICAL_EVIDENCE_CAPSULE_V2_VERSION {
            return Err(ClinicalEvidenceV2Error::UnsupportedCapsuleVersion(
                self.schema_version,
            ));
        }
        self.core
            .validate()
            .map_err(|_| ClinicalEvidenceV2Error::InvalidCoreCapsule)?;

        // v2 has exactly one fact-identity path. Carrying a legacy generic digest
        // alongside a typed identity would permit disagreement or downgrade.
        if self
            .core
            .fact_evidence
            .iter()
            .any(|evidence| evidence.fact_digest.is_some())
        {
            return Err(ClinicalEvidenceV2Error::LegacyFactDigestPresent);
        }

        if self.typed_fact_evidence.len() != self.core.fact_evidence.len() {
            return Err(ClinicalEvidenceV2Error::TypedFactCoverageMismatch);
        }

        let mut typed_by_fact: HashMap<&str, &TypedFactEvidenceV1> = HashMap::new();
        let mut typed_identities = HashSet::new();
        for evidence in &self.typed_fact_evidence {
            evidence.validate()?;
            if typed_by_fact
                .insert(evidence.fact_id.as_str(), evidence)
                .is_some()
            {
                return Err(ClinicalEvidenceV2Error::DuplicateTypedFactEvidence);
            }
            let key = (
                evidence.identity.namespace.as_str(),
                evidence.identity.artifact_id.as_str(),
                evidence.identity.digest,
            );
            if !typed_identities.insert(key) {
                return Err(ClinicalEvidenceV2Error::DuplicateTypedEvidenceIdentity);
            }
        }

        for legacy in &self.core.fact_evidence {
            let typed = typed_by_fact
                .get(legacy.fact_id.as_str())
                .copied()
                .ok_or(ClinicalEvidenceV2Error::TypedFactCoverageMismatch)?;
            if typed.role != legacy.role {
                return Err(ClinicalEvidenceV2Error::FactEvidenceRoleMismatch);
            }
        }

        Ok(())
    }
}

fn validate_token(value: &str) -> Result<(), ClinicalEvidenceV2Error> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(ClinicalEvidenceV2Error::InvalidIdentityToken);
    }
    if value.len() > 4096 {
        return Err(ClinicalEvidenceV2Error::IdentityTokenTooLong);
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClinicalEvidenceV2Error {
    #[error("unsupported clinical evidence capsule v2 schema version {0}")]
    UnsupportedCapsuleVersion(u16),
    #[error("unsupported typed evidence identity schema version {0}")]
    UnsupportedTypedIdentityVersion(u16),
    #[error("typed evidence identity token is invalid")]
    InvalidIdentityToken,
    #[error("typed evidence identity token exceeds the v1 bound")]
    IdentityTokenTooLong,
    #[error("typed evidence digest cannot be all zero")]
    ZeroTypedDigest,
    #[error("typed fact evidence must use the Mycelix ClinicalFact snapshot namespace")]
    UnexpectedFactEvidenceNamespace,
    #[error("typed fact artifact id must equal fact_id")]
    FactArtifactIdMismatch,
    #[error("wrapped v1 clinical evidence capsule is invalid")]
    InvalidCoreCapsule,
    #[error("v2 forbids the legacy generic fact_digest path")]
    LegacyFactDigestPresent,
    #[error("typed fact evidence must cover the core fact evidence exactly")]
    TypedFactCoverageMismatch,
    #[error("duplicate typed fact evidence for one fact id")]
    DuplicateTypedFactEvidence,
    #[error("duplicate typed evidence identity")]
    DuplicateTypedEvidenceIdentity,
    #[error("typed fact evidence role differs from the core capsule role")]
    FactEvidenceRoleMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_evidence::{
        AssertionKind, AuthorityMode, ClinicalAuthority, ExecutionIdentity, FactEvidence,
        HumanReview, IntendedUse, QualificationLevel, ReviewStatus,
    };
    use mycelix_clinical_evidence::{ArtifactIdentity, ContentDigest};
    use mycelix_clinical_semantics::{EvaluationState, SubjectRef};

    fn digest(byte: u8) -> ClinicalFactSnapshotDigestV1 {
        // Test-only construction goes through the real snapshot digest type's
        // bytes by deriving one from a real digest-shaped value is unavailable;
        // instead use a tiny helper fact path in tests below when a valid typed
        // identity is needed.
        let _ = byte;
        panic!("unused helper")
    }

    fn core() -> ClinicalEvidenceCapsule {
        ClinicalEvidenceCapsule {
            capsule_id: "capsule-1".into(),
            subject: SubjectRef {
                resource_type: "Patient".into(),
                id: "patient-a".into(),
            },
            kind: AssertionKind::RiskPrediction,
            statement: "Candidate risk prediction".into(),
            code: None,
            state: EvaluationState::Satisfied,
            reasons: vec!["test".into()],
            fact_evidence: vec![FactEvidence {
                fact_id: "fact-1".into(),
                role: EvidenceRole::Supports,
                fact_digest: None,
            }],
            sources: vec![],
            missing_requirements: vec![],
            alternatives: vec![],
            uncertainty: None,
            execution: ExecutionIdentity {
                engine: ArtifactIdentity {
                    name: "symthaea".into(),
                    version: "0.1.0".into(),
                    digest: ContentDigest {
                        algorithm: "blake3".into(),
                        value: "11".repeat(32),
                    },
                },
                model: Some(ArtifactIdentity {
                    name: "clinical-model".into(),
                    version: "1.0.0".into(),
                    digest: ContentDigest {
                        algorithm: "blake3".into(),
                        value: "22".repeat(32),
                    },
                }),
                knowledge_artifact: None,
                environment_digest: None,
                operation: "evaluate".into(),
            },
            intended_use: IntendedUse {
                use_case: "shadow risk prediction".into(),
                intended_user: "research clinician".into(),
                population: "test population".into(),
                care_setting: "shadow".into(),
                qualification: QualificationLevel::ShadowClinical,
            },
            authority: ClinicalAuthority {
                mode: AuthorityMode::InformationalOnly,
                review: HumanReview {
                    status: ReviewStatus::NotRequired,
                    reviewer: None,
                    reviewed_at_micros: None,
                    notes: None,
                },
            },
            issued_at_micros: 1_000,
            supersedes_capsule_id: None,
        }
    }

    fn typed() -> TypedFactEvidenceV1 {
        TypedFactEvidenceV1 {
            fact_id: "fact-1".into(),
            role: EvidenceRole::Supports,
            identity: TypedEvidenceIdentityV1 {
                schema_version: TYPED_EVIDENCE_IDENTITY_VERSION,
                namespace: MYCELIX_CLINICAL_FACT_SNAPSHOT_NAMESPACE_V1.into(),
                artifact_id: "fact-1".into(),
                algorithm: TypedEvidenceDigestAlgorithmV1::Blake3_256,
                digest: [0xAB; 32],
            },
        }
    }

    #[test]
    fn strict_typed_capsule_validates() {
        let capsule = ClinicalEvidenceCapsuleV2 {
            schema_version: CLINICAL_EVIDENCE_CAPSULE_V2_VERSION,
            core: core(),
            typed_fact_evidence: vec![typed()],
        };
        assert_eq!(capsule.validate(), Ok(()));
    }

    #[test]
    fn missing_typed_fact_is_rejected() {
        let capsule = ClinicalEvidenceCapsuleV2 {
            schema_version: CLINICAL_EVIDENCE_CAPSULE_V2_VERSION,
            core: core(),
            typed_fact_evidence: vec![],
        };
        assert_eq!(
            capsule.validate(),
            Err(ClinicalEvidenceV2Error::TypedFactCoverageMismatch)
        );
    }

    #[test]
    fn legacy_generic_fact_digest_is_rejected() {
        let mut core = core();
        core.fact_evidence[0].fact_digest = Some(ContentDigest {
            algorithm: "blake3".into(),
            value: "ab".repeat(32),
        });
        let capsule = ClinicalEvidenceCapsuleV2 {
            schema_version: CLINICAL_EVIDENCE_CAPSULE_V2_VERSION,
            core,
            typed_fact_evidence: vec![typed()],
        };
        assert_eq!(
            capsule.validate(),
            Err(ClinicalEvidenceV2Error::LegacyFactDigestPresent)
        );
    }

    #[test]
    fn fact_role_mismatch_is_rejected() {
        let mut typed = typed();
        typed.role = EvidenceRole::Opposes;
        let capsule = ClinicalEvidenceCapsuleV2 {
            schema_version: CLINICAL_EVIDENCE_CAPSULE_V2_VERSION,
            core: core(),
            typed_fact_evidence: vec![typed],
        };
        assert_eq!(
            capsule.validate(),
            Err(ClinicalEvidenceV2Error::FactEvidenceRoleMismatch)
        );
    }

    #[test]
    fn wrong_namespace_is_rejected() {
        let mut typed = typed();
        typed.identity.namespace = "fhir/Observation/json/v1".into();
        assert_eq!(
            typed.validate(),
            Err(ClinicalEvidenceV2Error::UnexpectedFactEvidenceNamespace)
        );
    }

    #[test]
    fn artifact_id_substitution_is_rejected() {
        let mut typed = typed();
        typed.identity.artifact_id = "fact-2".into();
        assert_eq!(
            typed.validate(),
            Err(ClinicalEvidenceV2Error::FactArtifactIdMismatch)
        );
    }

    #[test]
    fn zero_typed_digest_is_rejected() {
        let mut typed = typed();
        typed.identity.digest = [0u8; 32];
        assert_eq!(
            typed.validate(),
            Err(ClinicalEvidenceV2Error::ZeroTypedDigest)
        );
    }
}
