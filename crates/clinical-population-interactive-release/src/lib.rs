#![deny(unsafe_code)]
//! High-assurance interactive differential-privacy release gate.
//!
//! The lower-level population-release crate validates DP request/accountant structure.
//! This crate adds the trust conditions required before an interactive output can be
//! treated as release-authorized: the protected accountant receipts must match the
//! keyed commitments in DNA-qualified public accountant states, the `after` state must
//! be the sole bounded-canonical current leaf, and its predecessor must be the exact
//! trusted `before` state. No caller-supplied accountant vector is accepted here.

use hmac::{Hmac, Mac};
use holo_hash::ActionHash;
use mycelix_clinical_population_accountant_canonical_state::{
    reduce_canonical_population_accountant_snapshot,
    BoundedCanonicalPopulationAccountantStateV1, CanonicalAccountantStateError,
};
use mycelix_clinical_population_release::{
    InteractiveDpReleaseRequestV1, PopulationReleaseDigestV1, PopulationReleaseError,
    PopulationReleaseModeV1, PopulationReleasePolicyV1, PopulationReleaseRequestV1,
    PrivacyAccountantReceiptV1,
};
use population_accountant_index::CanonicalPopulationAccountantLineageSnapshotV1;
use population_accountant_integrity::{
    AccountantCommitmentSchemeV1, OpaqueAccountantReceiptCommitmentV1,
    TrustedPopulationAccountantState,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;

const PRIVATE_RECEIPT_COMMITMENT_CONTEXT: &[u8] =
    b"mycelix.health.population-accountant-private-receipt.v1";
const TRUSTED_RELEASE_HASH_CONTEXT: &str =
    "mycelix.health.population-interactive-release-receipt.v1";
const FRAME_VERSION: u8 = 1;
/// The canonical read must close within 60 seconds after the protected accountant
/// transition's own observation timestamp. This is a software freshness ceiling,
/// not a replacement for a trusted runtime clock.
pub const MAX_CANONICAL_READ_LAG_MICROS: i64 = 60_000_000;
/// A trusted capability is intended for immediate consumption. A runtime executor
/// should pass its trusted `sys_time()` into `into_receipt` and reject anything later.
pub const MAX_RELEASE_AFTER_CANONICAL_READ_MICROS: i64 = 60_000_000;

type HmacSha256 = Hmac<Sha256>;

/// Non-serializable secret used to recompute protected accountant-receipt commitments.
/// The bytes are zeroed on drop.
pub struct AccountantReceiptCommitmentKeyV1 {
    bytes: [u8; 32],
}

impl AccountantReceiptCommitmentKeyV1 {
    pub fn new(bytes: [u8; 32]) -> Result<Self, TrustedInteractiveReleaseError> {
        if bytes == [0u8; 32] {
            return Err(TrustedInteractiveReleaseError::ZeroCommitmentKey);
        }
        Ok(Self { bytes })
    }

    fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

impl Drop for AccountantReceiptCommitmentKeyV1 {
    fn drop(&mut self) {
        self.bytes.fill(0);
    }
}

/// Recompute the public opaque commitment to one protected accountant receipt.
/// Both DNA-root verifiers and the trusted release gate should use this exact helper.
pub fn commit_private_accountant_receipt(
    receipt: &PrivacyAccountantReceiptV1,
    scheme: AccountantCommitmentSchemeV1,
    key: &AccountantReceiptCommitmentKeyV1,
) -> Result<OpaqueAccountantReceiptCommitmentV1, TrustedInteractiveReleaseError> {
    let encoded = serde_json::to_vec(receipt)
        .map_err(|error| TrustedInteractiveReleaseError::Serialization(error.to_string()))?;
    let mut framed = Vec::with_capacity(
        PRIVATE_RECEIPT_COMMITMENT_CONTEXT.len() + 1 + 8 + encoded.len(),
    );
    framed.extend_from_slice(PRIVATE_RECEIPT_COMMITMENT_CONTEXT);
    framed.push(FRAME_VERSION);
    framed.extend_from_slice(&(encoded.len() as u64).to_be_bytes());
    framed.extend_from_slice(&encoded);

    let value = match scheme {
        AccountantCommitmentSchemeV1::HmacSha256 => {
            let mut mac = HmacSha256::new_from_slice(key.as_bytes())
                .map_err(|_| TrustedInteractiveReleaseError::InvalidHmacKey)?;
            mac.update(&framed);
            let bytes = mac.finalize().into_bytes();
            let mut value = [0u8; 32];
            value.copy_from_slice(&bytes);
            value
        }
        AccountantCommitmentSchemeV1::Blake3Keyed => {
            let mut hasher = blake3::Hasher::new_keyed(key.as_bytes());
            hasher.update(&framed);
            *hasher.finalize().as_bytes()
        }
    };

    if value == [0u8; 32] {
        return Err(TrustedInteractiveReleaseError::ZeroComputedCommitment);
    }
    Ok(OpaqueAccountantReceiptCommitmentV1 { scheme, value })
}

/// Distinct non-serializable authority for the high-assurance interactive path.
/// The lower-level `PopulationReleaseCapabilityV1` is not accepted as equivalent.
pub struct TrustedInteractivePopulationReleaseCapabilityV1 {
    request_digest: PopulationReleaseDigestV1,
    policy_digest: PopulationReleaseDigestV1,
    source_finding_digest: mycelix_clinical_integrity::StoredDigest,
    public_output_digest: mycelix_clinical_integrity::StoredDigest,
    accountant_before_state_hash: ActionHash,
    accountant_after_state_hash: ActionHash,
    accountant_sequence: u64,
    accountant_query_count: u64,
    canonical_observation_started_at_micros: i64,
    canonical_observation_completed_at_micros: i64,
    requested_at_micros: i64,
}

pub fn authorize_trusted_interactive_population_release(
    request: &PopulationReleaseRequestV1,
    canonical_snapshot: &CanonicalPopulationAccountantLineageSnapshotV1,
    commitment_key: &AccountantReceiptCommitmentKeyV1,
) -> Result<TrustedInteractivePopulationReleaseCapabilityV1, TrustedInteractiveReleaseError> {
    request.validate()?;
    let dp = match &request.mode {
        PopulationReleaseModeV1::InteractiveDifferentialPrivacy(value) => value,
        PopulationReleaseModeV1::StaticPreSpecifiedReport(_) => {
            return Err(TrustedInteractiveReleaseError::InteractiveModeRequired)
        }
    };

    let policy_digest = request.policy.verified_digest()?;
    validate_snapshot_lineage(&request.policy, policy_digest, canonical_snapshot)?;

    let bounded = reduce_canonical_population_accountant_snapshot(canonical_snapshot)?;
    let (after_hash, sequence, query_count) = match bounded {
        BoundedCanonicalPopulationAccountantStateV1::ObservedCurrentWithinBoundedCanonicalRead {
            action_hash,
            sequence,
            query_count,
        } => (action_hash, sequence, query_count),
        BoundedCanonicalPopulationAccountantStateV1::NoCanonicalStateObserved => {
            return Err(TrustedInteractiveReleaseError::NoCanonicalAccountantState)
        }
        BoundedCanonicalPopulationAccountantStateV1::ConflictWithinBoundedCanonicalRead { .. } => {
            return Err(TrustedInteractiveReleaseError::CanonicalAccountantConflict)
        }
        BoundedCanonicalPopulationAccountantStateV1::NoCurrentStateWithinBoundedCanonicalRead { .. }
        | BoundedCanonicalPopulationAccountantStateV1::ReviewRequiredWithinBoundedCanonicalRead {
            ..
        } => return Err(TrustedInteractiveReleaseError::CanonicalAccountantNotUsable),
    };

    let after = canonical_snapshot
        .admitted_states
        .iter()
        .find(|item| item.action_hash == after_hash)
        .ok_or(TrustedInteractiveReleaseError::CanonicalCurrentStateMissing)?;
    let before_hash = after
        .state
        .projection
        .previous_state_hash
        .clone()
        .ok_or(TrustedInteractiveReleaseError::InteractiveAfterStateCannotBeGenesis)?;
    let before = canonical_snapshot
        .admitted_states
        .iter()
        .find(|item| item.action_hash == before_hash)
        .ok_or(TrustedInteractiveReleaseError::TrustedPredecessorMissing)?;

    bind_private_receipt_to_public_state(
        &dp.accountant_before,
        &before.state,
        commitment_key,
    )?;
    bind_private_receipt_to_public_state(
        &dp.accountant_after,
        &after.state,
        commitment_key,
    )?;

    if before.state.projection.private_receipt_commitment.scheme
        != after.state.projection.private_receipt_commitment.scheme
    {
        return Err(TrustedInteractiveReleaseError::CommitmentSchemeChangedWithinTransition);
    }

    require_receipt_projection_match(&dp.accountant_before, &before.state)?;
    require_receipt_projection_match(&dp.accountant_after, &after.state)?;

    if sequence != dp.accountant_after.sequence
        || query_count != dp.accountant_after.query_count
        || after.state.projection.sequence != sequence
        || after.state.projection.query_count != query_count
    {
        return Err(TrustedInteractiveReleaseError::CanonicalLeafDoesNotMatchPrivateAfterReceipt);
    }
    if after.state.projection.previous_state_hash.as_ref() != Some(&before.action_hash) {
        return Err(TrustedInteractiveReleaseError::TrustedPredecessorMismatch);
    }

    validate_freshness(request, dp, canonical_snapshot)?;

    let request_digest = request.verified_digest()?;
    Ok(TrustedInteractivePopulationReleaseCapabilityV1 {
        request_digest,
        policy_digest,
        source_finding_digest: dp.source_finding_digest,
        public_output_digest: dp.public_output_digest,
        accountant_before_state_hash: before.action_hash.clone(),
        accountant_after_state_hash: after.action_hash.clone(),
        accountant_sequence: sequence,
        accountant_query_count: query_count,
        canonical_observation_started_at_micros: canonical_snapshot
            .observation_started_at
            .as_micros(),
        canonical_observation_completed_at_micros: canonical_snapshot
            .observation_completed_at
            .as_micros(),
        requested_at_micros: request.requested_at_micros,
    })
}

fn validate_snapshot_lineage(
    policy: &PopulationReleasePolicyV1,
    policy_digest: PopulationReleaseDigestV1,
    snapshot: &CanonicalPopulationAccountantLineageSnapshotV1,
) -> Result<(), TrustedInteractiveReleaseError> {
    if snapshot.lineage.release_policy_digest != policy_digest {
        return Err(TrustedInteractiveReleaseError::ReleasePolicyLineageMismatch);
    }
    if snapshot.lineage.accountant_instance_digest != policy.accountant_instance_digest
        || snapshot.lineage.accountant_method_digest != policy.accountant_method_digest
    {
        return Err(TrustedInteractiveReleaseError::AccountantLineageMismatch);
    }
    Ok(())
}

fn bind_private_receipt_to_public_state(
    receipt: &PrivacyAccountantReceiptV1,
    public_state: &TrustedPopulationAccountantState,
    commitment_key: &AccountantReceiptCommitmentKeyV1,
) -> Result<(), TrustedInteractiveReleaseError> {
    let expected = commit_private_accountant_receipt(
        receipt,
        public_state.projection.private_receipt_commitment.scheme,
        commitment_key,
    )?;
    if expected != public_state.projection.private_receipt_commitment {
        return Err(TrustedInteractiveReleaseError::PrivateReceiptCommitmentMismatch);
    }
    Ok(())
}

fn require_receipt_projection_match(
    receipt: &PrivacyAccountantReceiptV1,
    public_state: &TrustedPopulationAccountantState,
) -> Result<(), TrustedInteractiveReleaseError> {
    let projection = &public_state.projection;
    if receipt.accountant_instance_digest != projection.accountant_instance_digest
        || receipt.accountant_method_digest != projection.accountant_method_digest
        || receipt.sequence != projection.sequence
        || receipt.query_count != projection.query_count
        || receipt.cumulative_privacy_loss != projection.cumulative_privacy_loss
    {
        return Err(TrustedInteractiveReleaseError::PrivateReceiptProjectionMismatch);
    }
    Ok(())
}

fn validate_freshness(
    request: &PopulationReleaseRequestV1,
    dp: &InteractiveDpReleaseRequestV1,
    snapshot: &CanonicalPopulationAccountantLineageSnapshotV1,
) -> Result<(), TrustedInteractiveReleaseError> {
    let completed = snapshot.observation_completed_at.as_micros();
    let started = snapshot.observation_started_at.as_micros();
    if completed < started {
        return Err(TrustedInteractiveReleaseError::InvalidCanonicalObservationWindow);
    }
    if dp.accountant_after.observed_at_micros > request.requested_at_micros
        || request.requested_at_micros > completed
    {
        return Err(TrustedInteractiveReleaseError::RequestOutsideTrustedTransitionWindow);
    }
    let lag = completed
        .checked_sub(dp.accountant_after.observed_at_micros)
        .ok_or(TrustedInteractiveReleaseError::InvalidCanonicalObservationWindow)?;
    if lag > MAX_CANONICAL_READ_LAG_MICROS {
        return Err(TrustedInteractiveReleaseError::CanonicalSnapshotTooStale);
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TrustedInteractivePopulationReleaseReceiptV1 {
    pub schema_version: u16,
    pub request_digest: PopulationReleaseDigestV1,
    pub policy_digest: PopulationReleaseDigestV1,
    pub source_finding_digest: mycelix_clinical_integrity::StoredDigest,
    pub public_output_digest: mycelix_clinical_integrity::StoredDigest,
    pub accountant_before_state_hash: ActionHash,
    pub accountant_after_state_hash: ActionHash,
    pub accountant_sequence: u64,
    pub accountant_query_count: u64,
    pub canonical_observation_started_at_micros: i64,
    pub canonical_observation_completed_at_micros: i64,
    pub requested_at_micros: i64,
    pub released_at_micros: i64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TrustedInteractivePopulationReleaseReceiptDigestV1 {
    pub value: [u8; 32],
}

impl TrustedInteractivePopulationReleaseCapabilityV1 {
    /// Consume the capability into an evidence receipt. Production executors should
    /// supply a trusted runtime clock value; this pure method can only validate the
    /// timestamp claim it receives.
    pub fn into_receipt(
        self,
        released_at_micros: i64,
    ) -> Result<TrustedInteractivePopulationReleaseReceiptV1, TrustedInteractiveReleaseError> {
        if released_at_micros < self.requested_at_micros
            || released_at_micros < self.canonical_observation_completed_at_micros
        {
            return Err(TrustedInteractiveReleaseError::ReleaseTimeBeforeQualification);
        }
        let latest = self
            .canonical_observation_completed_at_micros
            .checked_add(MAX_RELEASE_AFTER_CANONICAL_READ_MICROS)
            .ok_or(TrustedInteractiveReleaseError::ReleaseWindowOverflow)?;
        if released_at_micros > latest {
            return Err(TrustedInteractiveReleaseError::TrustedReleaseWindowExpired);
        }
        Ok(TrustedInteractivePopulationReleaseReceiptV1 {
            schema_version: 1,
            request_digest: self.request_digest,
            policy_digest: self.policy_digest,
            source_finding_digest: self.source_finding_digest,
            public_output_digest: self.public_output_digest,
            accountant_before_state_hash: self.accountant_before_state_hash,
            accountant_after_state_hash: self.accountant_after_state_hash,
            accountant_sequence: self.accountant_sequence,
            accountant_query_count: self.accountant_query_count,
            canonical_observation_started_at_micros: self
                .canonical_observation_started_at_micros,
            canonical_observation_completed_at_micros: self
                .canonical_observation_completed_at_micros,
            requested_at_micros: self.requested_at_micros,
            released_at_micros,
        })
    }
}

impl TrustedInteractivePopulationReleaseReceiptV1 {
    pub fn verified_digest(
        &self,
    ) -> Result<TrustedInteractivePopulationReleaseReceiptDigestV1, TrustedInteractiveReleaseError>
    {
        if self.schema_version != 1 {
            return Err(TrustedInteractiveReleaseError::UnsupportedReceiptVersion);
        }
        if self.released_at_micros < self.requested_at_micros
            || self.released_at_micros < self.canonical_observation_completed_at_micros
        {
            return Err(TrustedInteractiveReleaseError::ReleaseTimeBeforeQualification);
        }
        let encoded = serde_json::to_vec(self)
            .map_err(|error| TrustedInteractiveReleaseError::Serialization(error.to_string()))?;
        let mut hasher = blake3::Hasher::new_derive_key(TRUSTED_RELEASE_HASH_CONTEXT);
        hasher.update(&[FRAME_VERSION]);
        hasher.update(&(encoded.len() as u64).to_be_bytes());
        hasher.update(&encoded);
        let value = *hasher.finalize().as_bytes();
        if value == [0u8; 32] {
            return Err(TrustedInteractiveReleaseError::ZeroTrustedReleaseDigest);
        }
        Ok(TrustedInteractivePopulationReleaseReceiptDigestV1 { value })
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TrustedInteractiveReleaseError {
    #[error("accountant commitment key cannot be all zero")]
    ZeroCommitmentKey,
    #[error("HMAC key initialization failed")]
    InvalidHmacKey,
    #[error("computed accountant receipt commitment cannot be all zero")]
    ZeroComputedCommitment,
    #[error("trusted interactive release gate accepts only interactive DP requests")]
    InteractiveModeRequired,
    #[error("canonical accountant snapshot uses a different release policy")]
    ReleasePolicyLineageMismatch,
    #[error("canonical accountant snapshot uses a different accountant instance/method")]
    AccountantLineageMismatch,
    #[error("no canonical accountant state was observed")]
    NoCanonicalAccountantState,
    #[error("canonical accountant lineage is forked/conflicting")]
    CanonicalAccountantConflict,
    #[error("canonical accountant state requires review or has no usable current leaf")]
    CanonicalAccountantNotUsable,
    #[error("canonical reducer current action is missing from the snapshot")]
    CanonicalCurrentStateMissing,
    #[error("interactive DP after-state cannot be accountant genesis")]
    InteractiveAfterStateCannotBeGenesis,
    #[error("trusted accountant predecessor is missing from the canonical snapshot")]
    TrustedPredecessorMissing,
    #[error("protected accountant receipt does not match the public keyed commitment")]
    PrivateReceiptCommitmentMismatch,
    #[error("commitment scheme changed inside one interactive DP transition")]
    CommitmentSchemeChangedWithinTransition,
    #[error("protected accountant receipt fields do not match public trusted state")]
    PrivateReceiptProjectionMismatch,
    #[error("canonical current leaf does not match protected after receipt")]
    CanonicalLeafDoesNotMatchPrivateAfterReceipt,
    #[error("trusted after-state predecessor does not equal the exact trusted before-state action")]
    TrustedPredecessorMismatch,
    #[error("canonical accountant observation window is invalid")]
    InvalidCanonicalObservationWindow,
    #[error("release request is outside the trusted accountant transition/canonical-read window")]
    RequestOutsideTrustedTransitionWindow,
    #[error("canonical accountant snapshot is too stale for interactive release")]
    CanonicalSnapshotTooStale,
    #[error("release timestamp precedes request or canonical qualification")]
    ReleaseTimeBeforeQualification,
    #[error("trusted release time window overflow")]
    ReleaseWindowOverflow,
    #[error("trusted interactive release capability expired")]
    TrustedReleaseWindowExpired,
    #[error("unsupported trusted interactive release receipt version")]
    UnsupportedReceiptVersion,
    #[error("trusted interactive release receipt digest cannot be zero")]
    ZeroTrustedReleaseDigest,
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error(transparent)]
    Release(#[from] PopulationReleaseError),
    #[error(transparent)]
    Canonical(#[from] CanonicalAccountantStateError),
}
