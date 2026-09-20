#![forbid(unsafe_code)]
//! Hardened public QUAL-EVID-005 external anti-rollback protocol.
//!
//! The external backend receives a reconstructible privacy-minimal state and
//! returns a backend-qualified proof. No caller-supplied verification boolean
//! crosses this public API.

use core::fmt;
use mycelix_qualification_product_authority_core as product;
use mycelix_qualification_receipt_core as qual;

pub use product::{
    AdapterProfileDigest, AdapterProfileDigestError, AdapterSeamId,
    Profile208CompositionCommitment, ProfileDefinitionError, VerifiedProfile208Match,
};

pub const QUALIFICATION_ANCHOR_PROTOCOL_V1: &str =
    "mycelix-health-qualification-external-anchor-v1";
pub const QUALIFICATION_ANCHOR_SUBJECT_DOMAIN_V1: &str =
    "MYCELIX-HEALTH-QUALIFICATION-EXTERNAL-ANCHOR-SUBJECT-V1";

macro_rules! digest_type {
    ($name:ident, $error:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $error {
            AllZero,
        }

        impl $name {
            pub fn new(bytes: [u8; 32]) -> Result<Self, $error> {
                if bytes == [0; 32] {
                    return Err($error::AllZero);
                }
                Ok(Self(bytes))
            }

            pub fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "([redacted])"))
            }
        }
    };
}

digest_type!(ExternalAnchorDigest, ExternalAnchorDigestError);
digest_type!(AnchorNonce, AnchorNonceError);
digest_type!(WitnessQuorumDigest, WitnessQuorumDigestError);
digest_type!(AnchorTrustSnapshotDigest, AnchorTrustSnapshotDigestError);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalAnchorStateError {
    InvalidCheckpointEpoch,
    InvalidHeadSequence,
}

/// Reconstructible, privacy-minimal state stored by the external rollback domain.
///
/// It contains only opaque commitments and monotonic counters. The backend does
/// not need raw care data, patient identity, contract text, or qualification
/// payloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalAnchorState {
    lineage_binding_digest: product::LineageBindingDigest,
    checkpoint_digest: product::CheckpointDigest,
    checkpoint_epoch: u64,
    head_sequence: u64,
    lineage_state_commitment: product::LineageStateCommitmentDigest,
    verified_head_digest: product::VerifiedHeadDigest,
}

impl ExternalAnchorState {
    pub fn new(
        lineage_binding_digest: product::LineageBindingDigest,
        checkpoint_digest: product::CheckpointDigest,
        checkpoint_epoch: u64,
        head_sequence: u64,
        lineage_state_commitment: product::LineageStateCommitmentDigest,
        verified_head_digest: product::VerifiedHeadDigest,
    ) -> Result<Self, ExternalAnchorStateError> {
        if checkpoint_epoch == 0 {
            return Err(ExternalAnchorStateError::InvalidCheckpointEpoch);
        }
        if head_sequence == 0 {
            return Err(ExternalAnchorStateError::InvalidHeadSequence);
        }
        Ok(Self {
            lineage_binding_digest,
            checkpoint_digest,
            checkpoint_epoch,
            head_sequence,
            lineage_state_commitment,
            verified_head_digest,
        })
    }

    pub fn lineage_binding_digest(&self) -> product::LineageBindingDigest {
        self.lineage_binding_digest
    }

    pub fn checkpoint_digest(&self) -> product::CheckpointDigest {
        self.checkpoint_digest
    }

    pub fn checkpoint_epoch(&self) -> u64 {
        self.checkpoint_epoch
    }

    pub fn head_sequence(&self) -> u64 {
        self.head_sequence
    }

    pub fn lineage_state_commitment(&self) -> product::LineageStateCommitmentDigest {
        self.lineage_state_commitment
    }

    pub fn verified_head_digest(&self) -> product::VerifiedHeadDigest {
        self.verified_head_digest
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalQualificationHead {
    head: product::VerifiedCurrentHead208,
    state: ExternalAnchorState,
}

impl LocalQualificationHead {
    pub fn from_verified_current_head(head: product::VerifiedCurrentHead208) -> Self {
        let state = ExternalAnchorState::new(
            head.lineage_binding_digest(),
            head.checkpoint_digest(),
            head.checkpoint_epoch(),
            head.head_sequence(),
            head.lineage_state_commitment(),
            head.verified_head_digest(),
        )
        .expect("qualified #217 head has nonzero epoch and sequence");
        Self { head, state }
    }

    pub fn state(&self) -> ExternalAnchorState {
        self.state
    }

    pub fn current_head(&self) -> product::VerifiedCurrentHead208 {
        self.head
    }
}

/// Authority adapter implemented by an independently qualified external backend.
///
/// A production implementation is responsible for cryptographic signatures,
/// witness quorum, trust/revocation state, monotonic external history,
/// equivocation detection and recovery. The generic Luminous implementation is
/// tracked in Luminous-Dynamics/luminous-dynamics#1962.
pub trait VerifiedExternalAnchorProof {
    fn protocol_id(&self) -> &str;
    fn subject_domain(&self) -> &str;
    fn state(&self) -> ExternalAnchorState;
    fn anchor_sequence(&self) -> u64;
    fn previous_anchor_digest(&self) -> Option<ExternalAnchorDigest>;
    fn anchor_nonce(&self) -> AnchorNonce;
    fn anchor_digest(&self) -> ExternalAnchorDigest;
    fn security_time_floor_micros(&self) -> i64;
    fn witness_set_epoch(&self) -> u64;
    fn witness_quorum_digest(&self) -> WitnessQuorumDigest;
    fn trust_snapshot_digest(&self) -> AnchorTrustSnapshotDigest;
    fn observed_at_micros(&self) -> i64;
    fn expires_at_micros(&self) -> i64;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnchorError {
    UnsupportedProtocol,
    UnsupportedSubjectDomain,
    InvalidAnchorSequence,
    InvalidPredecessorShape,
    InvalidWitnessSetEpoch,
    InvalidAnchorWindow,
    AnchorObservedInFuture,
    AnchorExpired,
    InvalidSecurityTimeFloor,
    SecurityTimeFloorInFuture,
    SecurityTimeFloorAfterObservation,
    CurrentHeadNotYetValid,
    CurrentHeadExpired,
    BootstrapRequiresInitialHead,
    SecurityTimeRollback,
    LocalRollbackDetected,
    ExternalAnchorBehind,
    LineageMismatch,
    SameEpochStateConflict,
    CheckpointEpochGap,
    HeadSequenceRollback,
    AnchorSequenceOverflow,
    ReusedAnchorNonce,
    AlreadyAnchored,
    IntentPreparedInFuture,
    IntentStateMismatch,
    ProofStateMismatch,
    ProofSequenceMismatch,
    ProofPredecessorMismatch,
    ProofNonceMismatch,
    ProofObservedBeforePrepare,
    ProofSecurityTimeMismatch,
    ProductAuthority(product::ProductAuthorityError),
}

impl From<product::ProductAuthorityError> for AnchorError {
    fn from(value: product::ProductAuthorityError) -> Self {
        Self::ProductAuthority(value)
    }
}

fn validate_local_head(
    local: &LocalQualificationHead,
    now_micros: i64,
) -> Result<(), AnchorError> {
    if now_micros < local.head.verified_at_micros() {
        return Err(AnchorError::CurrentHeadNotYetValid);
    }
    if now_micros >= local.head.expires_at_micros() {
        return Err(AnchorError::CurrentHeadExpired);
    }
    Ok(())
}

fn validate_external_proof<P: VerifiedExternalAnchorProof>(
    proof: &P,
    now_micros: i64,
) -> Result<(), AnchorError> {
    if proof.protocol_id() != QUALIFICATION_ANCHOR_PROTOCOL_V1 {
        return Err(AnchorError::UnsupportedProtocol);
    }
    if proof.subject_domain() != QUALIFICATION_ANCHOR_SUBJECT_DOMAIN_V1 {
        return Err(AnchorError::UnsupportedSubjectDomain);
    }
    if proof.anchor_sequence() == 0 {
        return Err(AnchorError::InvalidAnchorSequence);
    }
    if (proof.anchor_sequence() == 1 && proof.previous_anchor_digest().is_some())
        || (proof.anchor_sequence() > 1 && proof.previous_anchor_digest().is_none())
    {
        return Err(AnchorError::InvalidPredecessorShape);
    }
    if proof.witness_set_epoch() == 0 {
        return Err(AnchorError::InvalidWitnessSetEpoch);
    }
    if proof.observed_at_micros() < 0
        || proof.expires_at_micros() <= proof.observed_at_micros()
    {
        return Err(AnchorError::InvalidAnchorWindow);
    }
    if proof.observed_at_micros() > now_micros {
        return Err(AnchorError::AnchorObservedInFuture);
    }
    if now_micros >= proof.expires_at_micros() {
        return Err(AnchorError::AnchorExpired);
    }
    if proof.security_time_floor_micros() < 0 {
        return Err(AnchorError::InvalidSecurityTimeFloor);
    }
    if proof.security_time_floor_micros() > now_micros {
        return Err(AnchorError::SecurityTimeFloorInFuture);
    }
    if proof.security_time_floor_micros() > proof.observed_at_micros() {
        return Err(AnchorError::SecurityTimeFloorAfterObservation);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnchorUpdateIntent {
    state: ExternalAnchorState,
    anchor_sequence: u64,
    previous_anchor_digest: Option<ExternalAnchorDigest>,
    anchor_nonce: AnchorNonce,
    security_time_floor_micros: i64,
    prepared_at_micros: i64,
}

impl AnchorUpdateIntent {
    pub fn state(&self) -> ExternalAnchorState {
        self.state
    }
    pub fn anchor_sequence(&self) -> u64 {
        self.anchor_sequence
    }
    pub fn previous_anchor_digest(&self) -> Option<ExternalAnchorDigest> {
        self.previous_anchor_digest
    }
    pub fn anchor_nonce(&self) -> AnchorNonce {
        self.anchor_nonce
    }
    pub fn security_time_floor_micros(&self) -> i64 {
        self.security_time_floor_micros
    }
    pub fn prepared_at_micros(&self) -> i64 {
        self.prepared_at_micros
    }
}

/// Bootstrap is deliberately strict. A later pre-existing lineage needs an
/// explicit migration/root-of-trust ceremony rather than retroactive blessing.
pub fn prepare_anchor_bootstrap(
    local: &LocalQualificationHead,
    anchor_nonce: AnchorNonce,
    now_micros: i64,
) -> Result<AnchorUpdateIntent, AnchorError> {
    validate_local_head(local, now_micros)?;
    if local.state.checkpoint_epoch != 1 || local.state.head_sequence != 1 {
        return Err(AnchorError::BootstrapRequiresInitialHead);
    }
    Ok(AnchorUpdateIntent {
        state: local.state,
        anchor_sequence: 1,
        previous_anchor_digest: None,
        anchor_nonce,
        security_time_floor_micros: now_micros,
        prepared_at_micros: now_micros,
    })
}

/// Prepare an exact successor, or an identical-state refresh that only raises
/// the externally witnessed security-time floor.
pub fn prepare_anchor_successor<P: VerifiedExternalAnchorProof>(
    local: &LocalQualificationHead,
    prior: &P,
    anchor_nonce: AnchorNonce,
    now_micros: i64,
) -> Result<AnchorUpdateIntent, AnchorError> {
    validate_local_head(local, now_micros)?;
    validate_external_proof(prior, now_micros)?;
    let prior_state = prior.state();
    if local.state.lineage_binding_digest != prior_state.lineage_binding_digest {
        return Err(AnchorError::LineageMismatch);
    }
    if anchor_nonce == prior.anchor_nonce() {
        return Err(AnchorError::ReusedAnchorNonce);
    }
    if now_micros < prior.security_time_floor_micros() {
        return Err(AnchorError::SecurityTimeRollback);
    }

    if local.state.checkpoint_epoch < prior_state.checkpoint_epoch {
        return Err(AnchorError::LocalRollbackDetected);
    }
    if local.state.checkpoint_epoch == prior_state.checkpoint_epoch {
        if local.state != prior_state {
            return Err(AnchorError::SameEpochStateConflict);
        }
        if now_micros <= prior.security_time_floor_micros() {
            return Err(AnchorError::AlreadyAnchored);
        }
    } else {
        let expected_epoch = prior_state
            .checkpoint_epoch
            .checked_add(1)
            .ok_or(AnchorError::CheckpointEpochGap)?;
        if local.state.checkpoint_epoch != expected_epoch {
            return Err(AnchorError::CheckpointEpochGap);
        }
        if local.state.head_sequence < prior_state.head_sequence {
            return Err(AnchorError::HeadSequenceRollback);
        }
    }

    let next_sequence = prior
        .anchor_sequence()
        .checked_add(1)
        .ok_or(AnchorError::AnchorSequenceOverflow)?;
    Ok(AnchorUpdateIntent {
        state: local.state,
        anchor_sequence: next_sequence,
        previous_anchor_digest: Some(prior.anchor_digest()),
        anchor_nonce,
        security_time_floor_micros: now_micros,
        prepared_at_micros: now_micros,
    })
}

pub struct AnchoredQualificationHead {
    current_head: product::VerifiedCurrentHead208,
    anchor_digest: ExternalAnchorDigest,
    anchor_sequence: u64,
    anchor_nonce: AnchorNonce,
    security_time_floor_micros: i64,
    witness_set_epoch: u64,
    witness_quorum_digest: WitnessQuorumDigest,
    trust_snapshot_digest: AnchorTrustSnapshotDigest,
    observed_at_micros: i64,
    expires_at_micros: i64,
}

impl fmt::Debug for AnchoredQualificationHead {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnchoredQualificationHead")
            .field("checkpoint_epoch", &self.current_head.checkpoint_epoch())
            .field("head_sequence", &self.current_head.head_sequence())
            .field("anchor_sequence", &self.anchor_sequence)
            .field("anchor_digest", &self.anchor_digest)
            .field("witness_set_epoch", &self.witness_set_epoch)
            .field("observed_at_micros", &self.observed_at_micros)
            .field("expires_at_micros", &self.expires_at_micros)
            .finish_non_exhaustive()
    }
}

impl AnchoredQualificationHead {
    pub fn current_head(&self) -> product::VerifiedCurrentHead208 {
        self.current_head
    }
    pub fn anchor_digest(&self) -> ExternalAnchorDigest {
        self.anchor_digest
    }
    pub fn anchor_sequence(&self) -> u64 {
        self.anchor_sequence
    }
    pub fn anchor_nonce(&self) -> AnchorNonce {
        self.anchor_nonce
    }
    pub fn security_time_floor_micros(&self) -> i64 {
        self.security_time_floor_micros
    }
    pub fn witness_set_epoch(&self) -> u64 {
        self.witness_set_epoch
    }
    pub fn witness_quorum_digest(&self) -> WitnessQuorumDigest {
        self.witness_quorum_digest
    }
    pub fn trust_snapshot_digest(&self) -> AnchorTrustSnapshotDigest {
        self.trust_snapshot_digest
    }
    pub fn observed_at_micros(&self) -> i64 {
        self.observed_at_micros
    }
    pub fn expires_at_micros(&self) -> i64 {
        self.expires_at_micros
    }
}

fn anchored_from_proof<P: VerifiedExternalAnchorProof>(
    local: &LocalQualificationHead,
    proof: &P,
) -> AnchoredQualificationHead {
    AnchoredQualificationHead {
        current_head: local.head,
        anchor_digest: proof.anchor_digest(),
        anchor_sequence: proof.anchor_sequence(),
        anchor_nonce: proof.anchor_nonce(),
        security_time_floor_micros: proof.security_time_floor_micros(),
        witness_set_epoch: proof.witness_set_epoch(),
        witness_quorum_digest: proof.witness_quorum_digest(),
        trust_snapshot_digest: proof.trust_snapshot_digest(),
        observed_at_micros: proof.observed_at_micros(),
        expires_at_micros: proof.expires_at_micros(),
    }
}

/// Finalize only after the external backend returns proof for the exact prepare
/// record. Prepared-but-uncommitted local state carries no authority.
pub fn finalize_anchor_update<P: VerifiedExternalAnchorProof>(
    local: &LocalQualificationHead,
    intent: &AnchorUpdateIntent,
    proof: &P,
    now_micros: i64,
) -> Result<AnchoredQualificationHead, AnchorError> {
    validate_local_head(local, now_micros)?;
    validate_external_proof(proof, now_micros)?;
    if intent.prepared_at_micros > now_micros {
        return Err(AnchorError::IntentPreparedInFuture);
    }
    if local.state != intent.state {
        return Err(AnchorError::IntentStateMismatch);
    }
    if proof.state() != intent.state {
        return Err(AnchorError::ProofStateMismatch);
    }
    if proof.anchor_sequence() != intent.anchor_sequence {
        return Err(AnchorError::ProofSequenceMismatch);
    }
    if proof.previous_anchor_digest() != intent.previous_anchor_digest {
        return Err(AnchorError::ProofPredecessorMismatch);
    }
    if proof.anchor_nonce() != intent.anchor_nonce {
        return Err(AnchorError::ProofNonceMismatch);
    }
    if proof.observed_at_micros() < intent.prepared_at_micros {
        return Err(AnchorError::ProofObservedBeforePrepare);
    }
    if proof.security_time_floor_micros() != intent.security_time_floor_micros {
        return Err(AnchorError::ProofSecurityTimeMismatch);
    }
    Ok(anchored_from_proof(local, proof))
}

/// Restart/recovery path. Only exact equality restores authority; arrival order
/// never resolves disagreement.
pub fn reconcile_existing_anchor<P: VerifiedExternalAnchorProof>(
    local: &LocalQualificationHead,
    proof: &P,
    now_micros: i64,
) -> Result<AnchoredQualificationHead, AnchorError> {
    validate_local_head(local, now_micros)?;
    validate_external_proof(proof, now_micros)?;
    let external = proof.state();
    if local.state.lineage_binding_digest != external.lineage_binding_digest {
        return Err(AnchorError::LineageMismatch);
    }
    if local.state.checkpoint_epoch < external.checkpoint_epoch {
        return Err(AnchorError::LocalRollbackDetected);
    }
    if local.state.checkpoint_epoch > external.checkpoint_epoch {
        return Err(AnchorError::ExternalAnchorBehind);
    }
    if local.state != external {
        return Err(AnchorError::SameEpochStateConflict);
    }
    if now_micros < proof.security_time_floor_micros() {
        return Err(AnchorError::SecurityTimeRollback);
    }
    Ok(anchored_from_proof(local, proof))
}

pub struct AnchoredProfile208ProductAuthorityToken {
    inner: product::Profile208ProductAuthorityToken,
    anchor_digest: ExternalAnchorDigest,
    anchor_sequence: u64,
    witness_set_epoch: u64,
    witness_quorum_digest: WitnessQuorumDigest,
    trust_snapshot_digest: AnchorTrustSnapshotDigest,
    security_time_floor_micros: i64,
    anchor_observed_at_micros: i64,
    anchor_expires_at_micros: i64,
}

impl fmt::Debug for AnchoredProfile208ProductAuthorityToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnchoredProfile208ProductAuthorityToken")
            .field("seam", &self.inner.seam())
            .field("checkpoint_epoch", &self.inner.checkpoint_epoch())
            .field("anchor_sequence", &self.anchor_sequence)
            .field("anchor_digest", &self.anchor_digest)
            .field("witness_set_epoch", &self.witness_set_epoch)
            .finish_non_exhaustive()
    }
}

impl AnchoredProfile208ProductAuthorityToken {
    pub fn seam(&self) -> AdapterSeamId {
        self.inner.seam()
    }
    pub fn receipt_digest(&self) -> qual::ReceiptDigest {
        self.inner.receipt_digest()
    }
    pub fn checkpoint_epoch(&self) -> u64 {
        self.inner.checkpoint_epoch()
    }
    pub fn verified_head_digest(&self) -> [u8; 32] {
        self.inner.verified_head_digest()
    }
    pub fn anchor_digest(&self) -> ExternalAnchorDigest {
        self.anchor_digest
    }
    pub fn anchor_sequence(&self) -> u64 {
        self.anchor_sequence
    }
    pub fn witness_set_epoch(&self) -> u64 {
        self.witness_set_epoch
    }
    pub fn witness_quorum_digest(&self) -> WitnessQuorumDigest {
        self.witness_quorum_digest
    }
    pub fn trust_snapshot_digest(&self) -> AnchorTrustSnapshotDigest {
        self.trust_snapshot_digest
    }
    pub fn security_time_floor_micros(&self) -> i64 {
        self.security_time_floor_micros
    }
    pub fn anchor_observed_at_micros(&self) -> i64 {
        self.anchor_observed_at_micros
    }
    pub fn anchor_expires_at_micros(&self) -> i64 {
        self.anchor_expires_at_micros
    }
}

/// Stronger successor to #219: a bare #217 current head is not accepted.
pub struct AnchoredProfile208ProductAuthorityGate {
    inner: product::Profile208ProductAuthorityGate,
    latest_time_micros: Option<i64>,
}

impl AnchoredProfile208ProductAuthorityGate {
    pub fn new(profile: Profile208CompositionCommitment) -> Self {
        Self {
            inner: product::Profile208ProductAuthorityGate::new(profile),
            latest_time_micros: None,
        }
    }

    fn observe_time(&mut self, now_micros: i64) -> Result<(), AnchorError> {
        if self
            .latest_time_micros
            .is_some_and(|previous| now_micros < previous)
        {
            return Err(AnchorError::SecurityTimeRollback);
        }
        self.latest_time_micros = Some(now_micros);
        Ok(())
    }

    pub fn convert(
        &mut self,
        qualification: &qual::VerifiedQualification,
        ledger: &qual::QualificationLedger,
        profile_match: VerifiedProfile208Match,
        anchored_head: AnchoredQualificationHead,
        now_micros: i64,
    ) -> Result<AnchoredProfile208ProductAuthorityToken, AnchorError> {
        self.observe_time(now_micros)?;
        if now_micros < anchored_head.security_time_floor_micros {
            return Err(AnchorError::SecurityTimeRollback);
        }
        if now_micros < anchored_head.observed_at_micros {
            return Err(AnchorError::AnchorObservedInFuture);
        }
        if now_micros >= anchored_head.expires_at_micros {
            return Err(AnchorError::AnchorExpired);
        }
        let inner = self.inner.convert(
            qualification,
            ledger,
            profile_match,
            anchored_head.current_head,
            now_micros,
        )?;
        Ok(AnchoredProfile208ProductAuthorityToken {
            inner,
            anchor_digest: anchored_head.anchor_digest,
            anchor_sequence: anchored_head.anchor_sequence,
            witness_set_epoch: anchored_head.witness_set_epoch,
            witness_quorum_digest: anchored_head.witness_quorum_digest,
            trust_snapshot_digest: anchored_head.trust_snapshot_digest,
            security_time_floor_micros: anchored_head.security_time_floor_micros,
            anchor_observed_at_micros: anchored_head.observed_at_micros,
            anchor_expires_at_micros: anchored_head.expires_at_micros,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(value: u8) -> [u8; 32] {
        [value; 32]
    }

    fn binding() -> qual::QualificationLineageBinding {
        qual::QualificationLineageBinding::new(
            qual::QualificationLineageId::new(bytes(1)).unwrap(),
            qual::ContractRef::new(qual::ContractDigest::new(bytes(2)).unwrap(), 1).unwrap(),
            qual::SubjectDigest::new(bytes(3)).unwrap(),
            Some(qual::ContextDigest::new(bytes(4)).unwrap()),
            qual::AdmissionScope::Cutover,
            qual::ClaimProfileDigest::new(bytes(5)).unwrap(),
        )
    }

    fn head(
        epoch: u64,
        head_sequence: u64,
        checkpoint: u8,
        state: u8,
        verified: u8,
        verified_at: i64,
        expires_at: i64,
    ) -> product::VerifiedCurrentHead208 {
        product::VerifiedCurrentHead208::from_qualified_checkpoint_report(
            product::VERIFIED_HEAD_SCHEMA_V1,
            product::VERIFIED_HEAD_REPORT_KIND_V1,
            binding(),
            product::LineageBindingDigest::new(bytes(10)).unwrap(),
            qual::ReceiptDigest::new(bytes(11)).unwrap(),
            head_sequence,
            product::CheckpointDigest::new(bytes(checkpoint)).unwrap(),
            product::CheckpointBundleDigest::new(bytes(13)).unwrap(),
            epoch,
            product::ProfileMatchDigest::new(bytes(14)).unwrap(),
            product::LineageStateCommitmentDigest::new(bytes(state)).unwrap(),
            qual::GovernancePolicyDigest::new(bytes(16)).unwrap(),
            qual::TrustStoreDigest::new(bytes(17)).unwrap(),
            qual::DeploymentEvidenceDigest::new(bytes(18)).unwrap(),
            verified_at,
            expires_at,
            product::VerifiedHeadDigest::new(bytes(verified)).unwrap(),
        )
        .unwrap()
    }

    #[derive(Clone, Copy)]
    struct Proof {
        state: ExternalAnchorState,
        seq: u64,
        prev: Option<ExternalAnchorDigest>,
        nonce: AnchorNonce,
        digest: ExternalAnchorDigest,
        floor: i64,
        witness_epoch: u64,
        observed: i64,
        expires: i64,
    }

    impl VerifiedExternalAnchorProof for Proof {
        fn protocol_id(&self) -> &str {
            QUALIFICATION_ANCHOR_PROTOCOL_V1
        }
        fn subject_domain(&self) -> &str {
            QUALIFICATION_ANCHOR_SUBJECT_DOMAIN_V1
        }
        fn state(&self) -> ExternalAnchorState {
            self.state
        }
        fn anchor_sequence(&self) -> u64 {
            self.seq
        }
        fn previous_anchor_digest(&self) -> Option<ExternalAnchorDigest> {
            self.prev
        }
        fn anchor_nonce(&self) -> AnchorNonce {
            self.nonce
        }
        fn anchor_digest(&self) -> ExternalAnchorDigest {
            self.digest
        }
        fn security_time_floor_micros(&self) -> i64 {
            self.floor
        }
        fn witness_set_epoch(&self) -> u64 {
            self.witness_epoch
        }
        fn witness_quorum_digest(&self) -> WitnessQuorumDigest {
            WitnessQuorumDigest::new(bytes(90)).unwrap()
        }
        fn trust_snapshot_digest(&self) -> AnchorTrustSnapshotDigest {
            AnchorTrustSnapshotDigest::new(bytes(91)).unwrap()
        }
        fn observed_at_micros(&self) -> i64 {
            self.observed
        }
        fn expires_at_micros(&self) -> i64 {
            self.expires
        }
    }

    fn proof(local: &LocalQualificationHead, seq: u64, floor: i64) -> Proof {
        Proof {
            state: local.state(),
            seq,
            prev: if seq == 1 {
                None
            } else {
                Some(ExternalAnchorDigest::new(bytes(70)).unwrap())
            },
            nonce: AnchorNonce::new(bytes((40 + seq) as u8)).unwrap(),
            digest: ExternalAnchorDigest::new(bytes((70 + seq) as u8)).unwrap(),
            floor,
            witness_epoch: 1,
            observed: floor + 10,
            expires: 1_900,
        }
    }

    fn local(epoch: u64, seq: u64, checkpoint: u8, state: u8, verified: u8) -> LocalQualificationHead {
        LocalQualificationHead::from_verified_current_head(head(
            epoch, seq, checkpoint, state, verified, 100, 2_000,
        ))
    }

    #[test]
    fn external_state_is_reconstructible_but_not_authority() {
        let state = ExternalAnchorState::new(
            product::LineageBindingDigest::new(bytes(1)).unwrap(),
            product::CheckpointDigest::new(bytes(2)).unwrap(),
            1,
            1,
            product::LineageStateCommitmentDigest::new(bytes(3)).unwrap(),
            product::VerifiedHeadDigest::new(bytes(4)).unwrap(),
        )
        .unwrap();
        assert_eq!(state.checkpoint_epoch(), 1);
        assert_eq!(state.head_sequence(), 1);
    }

    #[test]
    fn bootstrap_requires_initial_checkpoint_and_head_sequence() {
        let first = local(1, 1, 12, 15, 19);
        assert!(prepare_anchor_bootstrap(
            &first,
            AnchorNonce::new(bytes(40)).unwrap(),
            200,
        )
        .is_ok());

        let later = local(2, 2, 22, 25, 29);
        assert_eq!(
            prepare_anchor_bootstrap(&later, AnchorNonce::new(bytes(40)).unwrap(), 200),
            Err(AnchorError::BootstrapRequiresInitialHead)
        );
    }

    #[test]
    fn predecessor_shape_is_canonical() {
        let first = local(1, 1, 12, 15, 19);
        let mut malformed = proof(&first, 1, 200);
        malformed.prev = Some(ExternalAnchorDigest::new(bytes(60)).unwrap());
        assert_eq!(
            reconcile_existing_anchor(&first, &malformed, 300).map(|_| ()),
            Err(AnchorError::InvalidPredecessorShape)
        );

        let mut later = proof(&first, 2, 200);
        later.prev = None;
        assert_eq!(
            reconcile_existing_anchor(&first, &later, 300).map(|_| ()),
            Err(AnchorError::InvalidPredecessorShape)
        );
    }

    #[test]
    fn exact_external_match_recovers_after_restart() {
        let local = local(1, 1, 12, 15, 19);
        let external = proof(&local, 1, 200);
        let anchored = reconcile_existing_anchor(&local, &external, 300).unwrap();
        assert_eq!(anchored.anchor_sequence(), 1);
        assert_eq!(anchored.current_head(), local.current_head());
    }

    #[test]
    fn whole_volume_local_rollback_is_detected() {
        let old = local(1, 1, 12, 15, 19);
        let new = local(2, 2, 22, 25, 29);
        let external = proof(&new, 2, 250);
        assert_eq!(
            reconcile_existing_anchor(&old, &external, 300).map(|_| ()),
            Err(AnchorError::LocalRollbackDetected)
        );
    }

    #[test]
    fn external_anchor_behind_local_state_denies_authority() {
        let old = local(1, 1, 12, 15, 19);
        let new = local(2, 2, 22, 25, 29);
        let external = proof(&old, 1, 200);
        assert_eq!(
            reconcile_existing_anchor(&new, &external, 300).map(|_| ()),
            Err(AnchorError::ExternalAnchorBehind)
        );
    }

    #[test]
    fn same_epoch_substitution_is_explicit_conflict() {
        let a = local(1, 1, 12, 15, 19);
        let b = local(1, 1, 13, 15, 19);
        let external = proof(&a, 1, 200);
        assert_eq!(
            reconcile_existing_anchor(&b, &external, 300).map(|_| ()),
            Err(AnchorError::SameEpochStateConflict)
        );
    }

    #[test]
    fn successor_rejects_epoch_gap_and_head_sequence_rollback() {
        let old = local(1, 4, 12, 15, 19);
        let prior = proof(&old, 1, 200);
        let skipped = local(3, 5, 32, 35, 39);
        assert_eq!(
            prepare_anchor_successor(
                &skipped,
                &prior,
                AnchorNonce::new(bytes(50)).unwrap(),
                300,
            ),
            Err(AnchorError::CheckpointEpochGap)
        );

        let rollback = local(2, 3, 22, 25, 29);
        assert_eq!(
            prepare_anchor_successor(
                &rollback,
                &prior,
                AnchorNonce::new(bytes(51)).unwrap(),
                300,
            ),
            Err(AnchorError::HeadSequenceRollback)
        );
    }

    #[test]
    fn same_state_refreshes_only_time_floor_with_new_anchor_record() {
        let local = local(1, 1, 12, 15, 19);
        let prior = proof(&local, 1, 200);
        let intent = prepare_anchor_successor(
            &local,
            &prior,
            AnchorNonce::new(bytes(50)).unwrap(),
            300,
        )
        .unwrap();
        assert_eq!(intent.state(), prior.state());
        assert_eq!(intent.anchor_sequence(), 2);
        assert_eq!(intent.previous_anchor_digest(), Some(prior.anchor_digest()));
        assert_eq!(intent.security_time_floor_micros(), 300);
    }

    #[test]
    fn finalization_binds_the_exact_prepare_commit_identity() {
        let local = local(1, 1, 12, 15, 19);
        let intent =
            prepare_anchor_bootstrap(&local, AnchorNonce::new(bytes(40)).unwrap(), 200).unwrap();
        let mut external = proof(&local, 1, 200);
        external.nonce = intent.anchor_nonce();
        external.observed = 210;
        assert!(finalize_anchor_update(&local, &intent, &external, 300).is_ok());

        let mut wrong = external;
        wrong.seq = 2;
        wrong.prev = Some(ExternalAnchorDigest::new(bytes(70)).unwrap());
        assert_eq!(
            finalize_anchor_update(&local, &intent, &wrong, 300).map(|_| ()),
            Err(AnchorError::ProofSequenceMismatch)
        );
    }

    #[test]
    fn expired_current_head_cannot_be_resurrected_by_fresh_anchor() {
        let local = LocalQualificationHead::from_verified_current_head(head(
            1, 1, 12, 15, 19, 100, 250,
        ));
        let mut external = proof(&local, 1, 200);
        external.expires = 1_000;
        assert_eq!(
            reconcile_existing_anchor(&local, &external, 300).map(|_| ()),
            Err(AnchorError::CurrentHeadExpired)
        );
    }

    #[test]
    fn stale_or_future_external_proof_never_authorizes() {
        let local = local(1, 1, 12, 15, 19);
        let mut expired = proof(&local, 1, 200);
        expired.expires = 250;
        assert_eq!(
            reconcile_existing_anchor(&local, &expired, 300).map(|_| ()),
            Err(AnchorError::AnchorExpired)
        );

        let mut future = proof(&local, 1, 400);
        future.observed = 410;
        future.expires = 500;
        assert_eq!(
            reconcile_existing_anchor(&local, &future, 300).map(|_| ()),
            Err(AnchorError::AnchorObservedInFuture)
        );
    }
}
