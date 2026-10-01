#![deny(unsafe_code)]
//! Exact scientific-context binding for interactive population releases.
//!
//! This crate intentionally does not decide whether a release is authorized. It
//! computes one domain-separated identity for the exact scientific context that
//! an already-validated interactive DP accountant transition claims to release.

use mycelix_clinical_integrity::{hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest};
use mycelix_clinical_population_release::{
    PopulationFindingKindV1, PopulationReleaseDigestV1, PopulationReleaseError,
    PopulationReleaseModeV1, PopulationReleaseRequestV1,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InteractivePopulationReleaseContextV1 {
    pub schema_version: u16,
    pub release_policy_digest: PopulationReleaseDigestV1,
    pub source_finding_kind: PopulationFindingKindV1,
    pub source_finding_digest: StoredDigest,
    pub query_specification_digest: StoredDigest,
    pub output_schema_digest: StoredDigest,
    pub public_output_digest: StoredDigest,
    pub mechanism_digest: PopulationReleaseDigestV1,
}

impl InteractivePopulationReleaseContextV1 {
    pub fn from_request(
        request: &PopulationReleaseRequestV1,
    ) -> Result<Self, PopulationReleaseContextError> {
        request.validate()?;
        let PopulationReleaseModeV1::InteractiveDifferentialPrivacy(interactive) = &request.mode
        else {
            return Err(PopulationReleaseContextError::NotInteractiveRelease);
        };

        Ok(Self {
            schema_version: 1,
            release_policy_digest: request.policy.verified_digest()?,
            source_finding_kind: interactive.source_finding_kind,
            source_finding_digest: interactive.source_finding_digest,
            query_specification_digest: interactive.query_specification_digest,
            output_schema_digest: interactive.output_schema_digest,
            public_output_digest: interactive.public_output_digest,
            mechanism_digest: interactive.mechanism.verified_digest(&request.policy)?,
        })
    }

    pub fn verified_digest(&self) -> Result<StoredDigest, PopulationReleaseContextError> {
        if self.schema_version != 1 {
            return Err(PopulationReleaseContextError::UnsupportedVersion(
                self.schema_version,
            ));
        }
        let canonical = serde_json::to_vec(self)
            .map_err(|error| PopulationReleaseContextError::Serialization(error.to_string()))?;
        Ok(hash_canonical_bytes(
            DigestDomain::ClinicalPopulationReleaseContext,
            &canonical,
        )?
        .stored())
    }
}

pub fn interactive_release_context_digest(
    request: &PopulationReleaseRequestV1,
) -> Result<StoredDigest, PopulationReleaseContextError> {
    InteractivePopulationReleaseContextV1::from_request(request)?.verified_digest()
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PopulationReleaseContextError {
    #[error("population release request is not interactive differential privacy")]
    NotInteractiveRelease,
    #[error("unsupported population release context version {0}")]
    UnsupportedVersion(u16),
    #[error("population release context serialization failed: {0}")]
    Serialization(String),
    #[error(transparent)]
    Release(#[from] PopulationReleaseError),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
}
