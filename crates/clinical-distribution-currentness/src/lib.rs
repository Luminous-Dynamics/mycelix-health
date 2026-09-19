#![deny(unsafe_code)]
//! Ephemeral runtime-currentness composition for clinical distribution evaluators.
//!
//! This crate deliberately does **not** infer current trust from absence of an
//! observed revocation. A currentness proof requires both:
//!
//! 1. one bounded network-backed admission/revocation observation with zero
//!    observed revocations; and
//! 2. one bounded network-backed positive lease observation for the exact same
//!    admission lineage.
//!
//! The resulting proof is non-serializable and short-lived. It is still a runtime
//! trust primitive, not evidence of detector science, model validity, patient
//! applicability, clinical effectiveness, or presentation authority.

use clinical_distribution_lease_zome::{
    DistributionEvaluatorLeaseReadBoundaryV1, DistributionEvaluatorPositiveLeaseObservationV1,
};
use clinical_distribution_trust_zome::{
    DistributionEvaluatorAdmissionSnapshotV1, DistributionEvaluatorSnapshotReadBoundaryV1,
};
use holo_hash::ActionHash;
use mycelix_clinical_evidence::{ArtifactIdentity, ContentDigest};
use thiserror::Error;

pub const DISTRIBUTION_EVALUATOR_CURRENTNESS_POLICY_VERSION: u16 = 1;
const ABSOLUTE_MAX_OBSERVATION_AGE_MICROS: i64 = 60_000_000; // 60 seconds
const ABSOLUTE_MAX_INTER_OBSERVATION_GAP_MICROS: i64 = 30_000_000; // 30 seconds
const POLICY_DERIVE_KEY_CONTEXT: &str =
    "mycelix.health.clinical-distribution-currentness-policy.v1";
const CURRENTNESS_DERIVE_KEY_CONTEXT: &str =
    "mycelix.health.clinical-distribution-currentness-proof.v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DistributionEvaluatorCurrentnessPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    /// Maximum age of either network observation at composition time.
    pub max_observation_age_micros: i64,
    /// Maximum difference between the two observation completion timestamps.
    pub max_inter_observation_gap_micros: i64,
}

impl DistributionEvaluatorCurrentnessPolicyV1 {
    pub fn validate(&self) -> Result<(), DistributionEvaluatorCurrentnessError> {
        if self.schema_version != DISTRIBUTION_EVALUATOR_CURRENTNESS_POLICY_VERSION {
            return Err(
                DistributionEvaluatorCurrentnessError::UnsupportedCurrentnessPolicyVersion {
                    found: self.schema_version,
                    expected: DISTRIBUTION_EVALUATOR_CURRENTNESS_POLICY_VERSION,
                },
            );
        }
        validate_token(&self.policy_id)?;
        if self.max_observation_age_micros <= 0
            || self.max_observation_age_micros > ABSOLUTE_MAX_OBSERVATION_AGE_MICROS
        {
            return Err(DistributionEvaluatorCurrentnessError::InvalidObservationAge);
        }
        if self.max_inter_observation_gap_micros < 0
            || self.max_inter_observation_gap_micros
                > ABSOLUTE_MAX_INTER_OBSERVATION_GAP_MICROS
            || self.max_inter_observation_gap_micros > self.max_observation_age_micros
        {
            return Err(DistributionEvaluatorCurrentnessError::InvalidObservationGap);
        }
        Ok(())
    }

    pub fn digest(
        &self,
    ) -> Result<DistributionEvaluatorCurrentnessPolicyDigestV1, DistributionEvaluatorCurrentnessError>
    {
        self.validate()?;
        let mut framed = Vec::new();
        framed.extend_from_slice(&self.schema_version.to_be_bytes());
        append_string(&mut framed, &self.policy_id)?;
        framed.extend_from_slice(&self.max_observation_age_micros.to_be_bytes());
        framed.extend_from_slice(&self.max_inter_observation_gap_micros.to_be_bytes());
        let mut hasher = blake3::Hasher::new_derive_key(POLICY_DERIVE_KEY_CONTEXT);
        hasher.update(&(framed.len() as u64).to_be_bytes());
        hasher.update(&framed);
        Ok(DistributionEvaluatorCurrentnessPolicyDigestV1(
            *hasher.finalize().as_bytes(),
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DistributionEvaluatorCurrentnessPolicyDigestV1([u8; 32]);

impl DistributionEvaluatorCurrentnessPolicyDigestV1 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DistributionEvaluatorCurrentnessDigestV1([u8; 32]);

impl DistributionEvaluatorCurrentnessDigestV1 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Non-serializable proof that one exact evaluator admission had both:
///
/// - no revocation observed in one bounded stable double read; and
/// - a positive root-issued lease observed in another bounded stable double read.
///
/// This proof expires quickly and must not be persisted as a bearer credential.
pub struct VerifiedDistributionEvaluatorCurrentnessV1 {
    admission_action_hash: ActionHash,
    selected_lease_action_hash: ActionHash,
    admission_proposal_digest: [u8; 32],
    detector: ArtifactIdentity,
    distribution_policy_digest: ContentDigest,
    trust_policy_digest: ContentDigest,
    admission_evidence_digest: ContentDigest,
    policy_digest: DistributionEvaluatorCurrentnessPolicyDigestV1,
    verified_at_micros: i64,
    valid_until_micros: i64,
    currentness_digest: DistributionEvaluatorCurrentnessDigestV1,
}

impl VerifiedDistributionEvaluatorCurrentnessV1 {
    pub fn admission_action_hash(&self) -> &ActionHash {
        &self.admission_action_hash
    }

    pub fn selected_lease_action_hash(&self) -> &ActionHash {
        &self.selected_lease_action_hash
    }

    #[must_use]
    pub const fn admission_proposal_digest(&self) -> &[u8; 32] {
        &self.admission_proposal_digest
    }

    pub fn detector(&self) -> &ArtifactIdentity {
        &self.detector
    }

    pub fn distribution_policy_digest(&self) -> &ContentDigest {
        &self.distribution_policy_digest
    }

    pub fn trust_policy_digest(&self) -> &ContentDigest {
        &self.trust_policy_digest
    }

    pub fn admission_evidence_digest(&self) -> &ContentDigest {
        &self.admission_evidence_digest
    }

    #[must_use]
    pub const fn policy_digest(&self) -> DistributionEvaluatorCurrentnessPolicyDigestV1 {
        self.policy_digest
    }

    #[must_use]
    pub const fn verified_at_micros(&self) -> i64 {
        self.verified_at_micros
    }

    #[must_use]
    pub const fn valid_until_micros(&self) -> i64 {
        self.valid_until_micros
    }

    #[must_use]
    pub const fn currentness_digest(&self) -> DistributionEvaluatorCurrentnessDigestV1 {
        self.currentness_digest
    }

    #[must_use]
    pub fn active_at(&self, at_micros: i64) -> bool {
        at_micros >= self.verified_at_micros && at_micros < self.valid_until_micros
    }
}

/// Compose two independently obtained conductor observations into one short-lived
/// evaluator-currentness proof.
///
/// The caller remains responsible for establishing that both input observations
/// actually came from the intended conductor/cell/DNA and from the exact zome
/// functions qualified for this deployment. The composition below prevents those
/// already-verified observations from being mixed across admission lineages,
/// policies, detectors, or stale time windows.
pub fn compose_distribution_evaluator_currentness_v1(
    revocation: &DistributionEvaluatorAdmissionSnapshotV1,
    lease: &DistributionEvaluatorPositiveLeaseObservationV1,
    policy: &DistributionEvaluatorCurrentnessPolicyV1,
    verified_at_micros: i64,
) -> Result<VerifiedDistributionEvaluatorCurrentnessV1, DistributionEvaluatorCurrentnessError> {
    policy.validate()?;
    if verified_at_micros <= 0 {
        return Err(DistributionEvaluatorCurrentnessError::InvalidVerificationTime);
    }
    if revocation.read_boundary
        != DistributionEvaluatorSnapshotReadBoundaryV1::NetworkBackedStableDoubleRead
    {
        return Err(DistributionEvaluatorCurrentnessError::UnexpectedRevocationReadBoundary);
    }
    if lease.read_boundary
        != DistributionEvaluatorLeaseReadBoundaryV1::NetworkBackedStableDoubleRead
    {
        return Err(DistributionEvaluatorCurrentnessError::UnexpectedLeaseReadBoundary);
    }

    let revocation_started = revocation.observation_started_at.as_micros();
    let revocation_completed = revocation.observation_completed_at.as_micros();
    let lease_started = lease.observation_started_at.as_micros();
    let lease_completed = lease.observation_completed_at.as_micros();
    if revocation_started > revocation_completed || lease_started > lease_completed {
        return Err(DistributionEvaluatorCurrentnessError::InvalidObservationWindow);
    }
    if revocation_completed > verified_at_micros || lease_completed > verified_at_micros {
        return Err(DistributionEvaluatorCurrentnessError::ObservationAfterVerificationTime);
    }

    let revocation_age = verified_at_micros.saturating_sub(revocation_completed);
    let lease_age = verified_at_micros.saturating_sub(lease_completed);
    if revocation_age > policy.max_observation_age_micros {
        return Err(DistributionEvaluatorCurrentnessError::StaleRevocationObservation);
    }
    if lease_age > policy.max_observation_age_micros {
        return Err(DistributionEvaluatorCurrentnessError::StaleLeaseObservation);
    }
    if revocation_completed.abs_diff(lease_completed)
        > policy.max_inter_observation_gap_micros as u64
    {
        return Err(DistributionEvaluatorCurrentnessError::ObservationGapTooLarge);
    }

    if revocation.observed_revocation_target_count != revocation.revocations.len() {
        return Err(DistributionEvaluatorCurrentnessError::MalformedRevocationObservation);
    }
    if !revocation.revocations.is_empty() {
        return Err(DistributionEvaluatorCurrentnessError::ObservedRevocationPresent);
    }
    if lease.observed_lease_target_count == 0 {
        return Err(DistributionEvaluatorCurrentnessError::MissingPositiveLeaseEvidence);
    }

    if revocation.admission_action_hash != lease.admission_action_hash {
        return Err(DistributionEvaluatorCurrentnessError::AdmissionActionMismatch);
    }
    if lease.selected_lease.admission_hash != revocation.admission_action_hash {
        return Err(DistributionEvaluatorCurrentnessError::LeaseAdmissionMismatch);
    }

    let proposal_digest = revocation
        .admission
        .proposal
        .digest()
        .map_err(|_| DistributionEvaluatorCurrentnessError::InvalidAdmissionProposal)?;
    if proposal_digest != lease.admission_proposal_digest
        || proposal_digest != lease.selected_lease.admission_proposal_digest
    {
        return Err(DistributionEvaluatorCurrentnessError::AdmissionProposalMismatch);
    }

    if lease.selected_lease.detector != revocation.admission.proposal.detector {
        return Err(DistributionEvaluatorCurrentnessError::DetectorMismatch);
    }
    if lease.selected_lease.distribution_policy_digest
        != revocation.admission.proposal.distribution_policy_digest
    {
        return Err(DistributionEvaluatorCurrentnessError::DistributionPolicyMismatch);
    }
    if lease.selected_lease.trust_policy_digest
        != revocation.admission.proposal.trust_policy_digest
    {
        return Err(DistributionEvaluatorCurrentnessError::TrustPolicyMismatch);
    }

    let admission_valid_from = revocation.admission.proposal.valid_from.as_micros();
    if verified_at_micros < admission_valid_from {
        return Err(DistributionEvaluatorCurrentnessError::AdmissionNotCurrent);
    }
    if revocation
        .admission
        .proposal
        .valid_until
        .is_some_and(|until| verified_at_micros >= until.as_micros())
    {
        return Err(DistributionEvaluatorCurrentnessError::AdmissionNotCurrent);
    }

    let lease_valid_from = lease.selected_lease.valid_from.as_micros();
    let lease_valid_until = lease.selected_lease.valid_until.as_micros();
    if verified_at_micros < lease_valid_from || verified_at_micros >= lease_valid_until {
        return Err(DistributionEvaluatorCurrentnessError::PositiveLeaseNotCurrent);
    }

    validate_runtime_identity(
        &revocation.admission.proposal.detector.name,
        &revocation.admission.proposal.detector.version,
        &revocation.admission.proposal.detector.digest.algorithm,
        &revocation.admission.proposal.detector.digest.value,
    )?;
    validate_runtime_digest(
        &revocation.admission.proposal.distribution_policy_digest.algorithm,
        &revocation.admission.proposal.distribution_policy_digest.value,
    )?;
    validate_runtime_digest(
        &revocation.admission.proposal.trust_policy_digest.algorithm,
        &revocation.admission.proposal.trust_policy_digest.value,
    )?;
    validate_runtime_digest(
        &revocation
            .admission
            .proposal
            .external_admission_evidence_digest
            .algorithm,
        &revocation
            .admission
            .proposal
            .external_admission_evidence_digest
            .value,
    )?;

    let revocation_fresh_until = revocation_completed
        .checked_add(policy.max_observation_age_micros)
        .ok_or(DistributionEvaluatorCurrentnessError::TimeOverflow)?;
    let lease_fresh_until = lease_completed
        .checked_add(policy.max_observation_age_micros)
        .ok_or(DistributionEvaluatorCurrentnessError::TimeOverflow)?;
    let mut valid_until_micros = lease_valid_until
        .min(revocation_fresh_until)
        .min(lease_fresh_until);
    if let Some(admission_until) = revocation.admission.proposal.valid_until {
        valid_until_micros = valid_until_micros.min(admission_until.as_micros());
    }
    if valid_until_micros <= verified_at_micros {
        return Err(DistributionEvaluatorCurrentnessError::CurrentnessAlreadyExpired);
    }

    let detector = ArtifactIdentity {
        name: revocation.admission.proposal.detector.name.clone(),
        version: revocation.admission.proposal.detector.version.clone(),
        digest: ContentDigest {
            algorithm: revocation.admission.proposal.detector.digest.algorithm.clone(),
            value: revocation.admission.proposal.detector.digest.value.clone(),
        },
    };
    let distribution_policy_digest = ContentDigest {
        algorithm: revocation
            .admission
            .proposal
            .distribution_policy_digest
            .algorithm
            .clone(),
        value: revocation
            .admission
            .proposal
            .distribution_policy_digest
            .value
            .clone(),
    };
    let trust_policy_digest = ContentDigest {
        algorithm: revocation.admission.proposal.trust_policy_digest.algorithm.clone(),
        value: revocation.admission.proposal.trust_policy_digest.value.clone(),
    };
    let admission_evidence_digest = ContentDigest {
        algorithm: revocation
            .admission
            .proposal
            .external_admission_evidence_digest
            .algorithm
            .clone(),
        value: revocation
            .admission
            .proposal
            .external_admission_evidence_digest
            .value
            .clone(),
    };
    let policy_digest = policy.digest()?;

    let currentness_digest = compute_currentness_digest(
        &revocation.admission_action_hash,
        &lease.selected_lease_action_hash,
        &proposal_digest,
        &detector,
        &distribution_policy_digest,
        &trust_policy_digest,
        &admission_evidence_digest,
        &policy_digest,
        revocation_completed,
        lease_completed,
        verified_at_micros,
        valid_until_micros,
    )?;

    Ok(VerifiedDistributionEvaluatorCurrentnessV1 {
        admission_action_hash: revocation.admission_action_hash.clone(),
        selected_lease_action_hash: lease.selected_lease_action_hash.clone(),
        admission_proposal_digest: proposal_digest,
        detector,
        distribution_policy_digest,
        trust_policy_digest,
        admission_evidence_digest,
        policy_digest,
        verified_at_micros,
        valid_until_micros,
        currentness_digest,
    })
}

#[allow(clippy::too_many_arguments)]
fn compute_currentness_digest(
    admission_action_hash: &ActionHash,
    selected_lease_action_hash: &ActionHash,
    admission_proposal_digest: &[u8; 32],
    detector: &ArtifactIdentity,
    distribution_policy_digest: &ContentDigest,
    trust_policy_digest: &ContentDigest,
    admission_evidence_digest: &ContentDigest,
    policy_digest: &DistributionEvaluatorCurrentnessPolicyDigestV1,
    revocation_completed_at_micros: i64,
    lease_completed_at_micros: i64,
    verified_at_micros: i64,
    valid_until_micros: i64,
) -> Result<DistributionEvaluatorCurrentnessDigestV1, DistributionEvaluatorCurrentnessError> {
    let mut framed = Vec::new();
    append_bytes(&mut framed, admission_action_hash.get_raw_36())?;
    append_bytes(&mut framed, selected_lease_action_hash.get_raw_36())?;
    append_bytes(&mut framed, admission_proposal_digest)?;
    append_string(&mut framed, &detector.name)?;
    append_string(&mut framed, &detector.version)?;
    append_content_digest(&mut framed, &detector.digest)?;
    append_content_digest(&mut framed, distribution_policy_digest)?;
    append_content_digest(&mut framed, trust_policy_digest)?;
    append_content_digest(&mut framed, admission_evidence_digest)?;
    append_bytes(&mut framed, policy_digest.as_bytes())?;
    framed.extend_from_slice(&revocation_completed_at_micros.to_be_bytes());
    framed.extend_from_slice(&lease_completed_at_micros.to_be_bytes());
    framed.extend_from_slice(&verified_at_micros.to_be_bytes());
    framed.extend_from_slice(&valid_until_micros.to_be_bytes());

    let mut hasher = blake3::Hasher::new_derive_key(CURRENTNESS_DERIVE_KEY_CONTEXT);
    hasher.update(&(framed.len() as u64).to_be_bytes());
    hasher.update(&framed);
    Ok(DistributionEvaluatorCurrentnessDigestV1(
        *hasher.finalize().as_bytes(),
    ))
}

fn append_content_digest(
    target: &mut Vec<u8>,
    digest: &ContentDigest,
) -> Result<(), DistributionEvaluatorCurrentnessError> {
    validate_runtime_digest(&digest.algorithm, &digest.value)?;
    append_string(target, &digest.algorithm)?;
    append_string(target, &digest.value)
}

fn append_string(
    target: &mut Vec<u8>,
    value: &str,
) -> Result<(), DistributionEvaluatorCurrentnessError> {
    validate_token(value)?;
    append_bytes(target, value.as_bytes())
}

fn append_bytes(
    target: &mut Vec<u8>,
    value: &[u8],
) -> Result<(), DistributionEvaluatorCurrentnessError> {
    let len = u32::try_from(value.len())
        .map_err(|_| DistributionEvaluatorCurrentnessError::IdentityTooLarge)?;
    target.extend_from_slice(&len.to_be_bytes());
    target.extend_from_slice(value);
    Ok(())
}

fn validate_runtime_identity(
    name: &str,
    version: &str,
    algorithm: &str,
    value: &str,
) -> Result<(), DistributionEvaluatorCurrentnessError> {
    validate_token(name)?;
    validate_token(version)?;
    validate_runtime_digest(algorithm, value)
}

fn validate_runtime_digest(
    algorithm: &str,
    value: &str,
) -> Result<(), DistributionEvaluatorCurrentnessError> {
    validate_token(algorithm)?;
    validate_token(value)?;
    Ok(())
}

fn validate_token(value: &str) -> Result<(), DistributionEvaluatorCurrentnessError> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(DistributionEvaluatorCurrentnessError::InvalidIdentityToken);
    }
    if value.len() > 4096 {
        return Err(DistributionEvaluatorCurrentnessError::IdentityTooLarge);
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DistributionEvaluatorCurrentnessError {
    #[error("unsupported evaluator-currentness policy version {found}; expected {expected}")]
    UnsupportedCurrentnessPolicyVersion { found: u16, expected: u16 },
    #[error("currentness observation-age policy is invalid")]
    InvalidObservationAge,
    #[error("currentness inter-observation gap policy is invalid")]
    InvalidObservationGap,
    #[error("currentness verification time must be positive")]
    InvalidVerificationTime,
    #[error("observation start/completion ordering is invalid")]
    InvalidObservationWindow,
    #[error("an observation completes after currentness verification time")]
    ObservationAfterVerificationTime,
    #[error("admission/revocation observation is stale")]
    StaleRevocationObservation,
    #[error("positive lease observation is stale")]
    StaleLeaseObservation,
    #[error("observation completion timestamps are too far apart")]
    ObservationGapTooLarge,
    #[error("unexpected admission/revocation read boundary")]
    UnexpectedRevocationReadBoundary,
    #[error("unexpected positive-lease read boundary")]
    UnexpectedLeaseReadBoundary,
    #[error("admission/revocation observation is internally inconsistent")]
    MalformedRevocationObservation,
    #[error("at least one evaluator revocation was observed")]
    ObservedRevocationPresent,
    #[error("positive lease observation contains no lease evidence")]
    MissingPositiveLeaseEvidence,
    #[error("revocation and lease observations target different evaluator admissions")]
    AdmissionActionMismatch,
    #[error("selected positive lease targets another admission")]
    LeaseAdmissionMismatch,
    #[error("revocation and lease observations disagree on admission proposal identity")]
    AdmissionProposalMismatch,
    #[error("positive lease detector identity differs from evaluator admission")]
    DetectorMismatch,
    #[error("positive lease distribution-policy identity differs from evaluator admission")]
    DistributionPolicyMismatch,
    #[error("positive lease trust-policy identity differs from evaluator admission")]
    TrustPolicyMismatch,
    #[error("evaluator admission is not current at composition time")]
    AdmissionNotCurrent,
    #[error("positive evaluator lease is not current at composition time")]
    PositiveLeaseNotCurrent,
    #[error("evaluator admission proposal could not be independently digested")]
    InvalidAdmissionProposal,
    #[error("runtime identity token is invalid")]
    InvalidIdentityToken,
    #[error("runtime identity exceeds framing limits")]
    IdentityTooLarge,
    #[error("time arithmetic overflowed")]
    TimeOverflow,
    #[error("composed currentness proof would already be expired")]
    CurrentnessAlreadyExpired,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clinical_distribution_lease_integrity::ClinicalDistributionEvaluatorLeaseV1;
    use clinical_distribution_trust_integrity::{
        DistributionEvaluatorAdmissionProposalV1, QualifiedDistributionEvaluatorAdmission,
        RuntimeArtifactIdentityV1, RuntimeContentDigestV1,
    };
    use hdi::prelude::Timestamp;

    fn digest(value: &str) -> RuntimeContentDigestV1 {
        RuntimeContentDigestV1 {
            algorithm: "blake3-256".into(),
            value: value.into(),
        }
    }

    fn proposal() -> DistributionEvaluatorAdmissionProposalV1 {
        DistributionEvaluatorAdmissionProposalV1 {
            schema_version: 1,
            admission_id: "admission-a".into(),
            detector: RuntimeArtifactIdentityV1 {
                name: "ood-detector".into(),
                version: "1.0.0".into(),
                digest: digest("detector"),
            },
            distribution_policy_digest: digest("distribution-policy"),
            trust_policy_digest: digest("trust-policy"),
            external_admission_evidence_digest: digest("registry-evidence"),
            valid_from: Timestamp::from_micros(100),
            valid_until: Some(Timestamp::from_micros(10_000_000)),
        }
    }

    fn admission_hash(seed: u8) -> ActionHash {
        ActionHash::from_raw_36(vec![seed; 36])
    }

    fn snapshot(hash: ActionHash) -> DistributionEvaluatorAdmissionSnapshotV1 {
        DistributionEvaluatorAdmissionSnapshotV1 {
            admission_action_hash: hash,
            admission: QualifiedDistributionEvaluatorAdmission {
                proposal: proposal(),
                verifier_authorization_hash: admission_hash(7),
            },
            revocations: vec![],
            read_boundary:
                DistributionEvaluatorSnapshotReadBoundaryV1::NetworkBackedStableDoubleRead,
            observed_revocation_target_count: 0,
            observation_started_at: Timestamp::from_micros(900),
            observation_completed_at: Timestamp::from_micros(1_000),
        }
    }

    fn lease_observation(hash: ActionHash) -> DistributionEvaluatorPositiveLeaseObservationV1 {
        let proposal = proposal();
        let proposal_digest = proposal.digest().unwrap();
        DistributionEvaluatorPositiveLeaseObservationV1 {
            admission_action_hash: hash.clone(),
            admission_proposal_digest: proposal_digest,
            selected_lease_action_hash: admission_hash(8),
            selected_lease: ClinicalDistributionEvaluatorLeaseV1 {
                schema_version: 1,
                lease_id: "lease-1".into(),
                admission_hash: hash,
                admission_proposal_digest: proposal_digest,
                detector: proposal.detector,
                distribution_policy_digest: proposal.distribution_policy_digest,
                trust_policy_digest: proposal.trust_policy_digest,
                lease_sequence: 0,
                supersedes_lease_hash: None,
                valid_from: Timestamp::from_micros(800),
                valid_until: Timestamp::from_micros(5_000_000),
            },
            observed_lease_target_count: 1,
            read_boundary:
                DistributionEvaluatorLeaseReadBoundaryV1::NetworkBackedStableDoubleRead,
            observation_started_at: Timestamp::from_micros(950),
            observation_completed_at: Timestamp::from_micros(1_100),
        }
    }

    fn policy() -> DistributionEvaluatorCurrentnessPolicyV1 {
        DistributionEvaluatorCurrentnessPolicyV1 {
            schema_version: DISTRIBUTION_EVALUATOR_CURRENTNESS_POLICY_VERSION,
            policy_id: "runtime-currentness-v1".into(),
            max_observation_age_micros: 1_000_000,
            max_inter_observation_gap_micros: 500_000,
        }
    }

    #[test]
    fn exact_revocation_and_positive_lease_observations_compose() {
        let hash = admission_hash(4);
        let currentness = compose_distribution_evaluator_currentness_v1(
            &snapshot(hash.clone()),
            &lease_observation(hash),
            &policy(),
            1_200,
        )
        .unwrap();
        assert!(currentness.active_at(1_200));
        assert_eq!(currentness.detector().name, "ood-detector");
        assert_ne!(currentness.currentness_digest().as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn observed_revocation_blocks_currentness() {
        let hash = admission_hash(4);
        let mut snapshot = snapshot(hash.clone());
        snapshot.observed_revocation_target_count = 1;
        snapshot.revocations.push(
            clinical_distribution_trust_zome::ObservedDistributionEvaluatorRevocationV1 {
                action_hash: admission_hash(9),
                revocation:
                    clinical_distribution_trust_integrity::DistributionEvaluatorAdmissionRevocation {
                        revocation_id: "revoked".into(),
                        admission_hash: hash.clone(),
                        admission_proposal_digest: proposal().digest().unwrap(),
                        reason: clinical_distribution_trust_integrity::DistributionEvaluatorRevocationReason::EvaluatorRetired,
                        reason_commitment: None,
                    },
            },
        );
        assert_eq!(
            compose_distribution_evaluator_currentness_v1(
                &snapshot,
                &lease_observation(hash),
                &policy(),
                1_200,
            )
            .err(),
            Some(DistributionEvaluatorCurrentnessError::ObservedRevocationPresent)
        );
    }

    #[test]
    fn cross_admission_observation_mix_is_rejected() {
        assert_eq!(
            compose_distribution_evaluator_currentness_v1(
                &snapshot(admission_hash(4)),
                &lease_observation(admission_hash(5)),
                &policy(),
                1_200,
            )
            .err(),
            Some(DistributionEvaluatorCurrentnessError::AdmissionActionMismatch)
        );
    }

    #[test]
    fn stale_revocation_observation_is_rejected_even_with_current_lease() {
        let hash = admission_hash(4);
        let mut policy = policy();
        policy.max_observation_age_micros = 100;
        assert_eq!(
            compose_distribution_evaluator_currentness_v1(
                &snapshot(hash.clone()),
                &lease_observation(hash),
                &policy,
                1_200,
            )
            .err(),
            Some(DistributionEvaluatorCurrentnessError::StaleRevocationObservation)
        );
    }

    #[test]
    fn currentness_validity_is_capped_by_observation_freshness() {
        let hash = admission_hash(4);
        let currentness = compose_distribution_evaluator_currentness_v1(
            &snapshot(hash.clone()),
            &lease_observation(hash),
            &policy(),
            1_200,
        )
        .unwrap();
        assert_eq!(currentness.valid_until_micros(), 1_001_000);
        assert!(currentness.active_at(1_001_000 - 1));
        assert!(!currentness.active_at(1_001_000));
    }
}
