#![forbid(unsafe_code)]
//! Production-facing token boundary for the first Health qualification
//! anti-rollback vertical slice.
//!
//! The lower QUAL-EVID reference crates intentionally use open `Verified*`
//! traits as semantic/test seams.  This crate does not expose those traits as
//! product inputs.  Instead it accepts concrete private-field tokens and wraps
//! every authority-bearing output before it can reach the #208 product gate.
//!
//! There is deliberately no public production mint constructor yet.  Until a
//! qualified generic witness verifier and a qualified Health trusted-time
//! verifier are integrated into this crate, production minting is unavailable.
//! Reduced availability must never be converted into caller-asserted authority.

use mycelix_qualification_anti_rollback_anchor_core as health;
use mycelix_qualification_generic_transparency_adapter_core as adapter;
use mycelix_qualification_receipt_core as qual;

pub const VERIFIER_TOKEN_PROTOCOL_V1: &str =
    "mycelix-health-qualification-verifier-token-v1";

/// Concrete accepted-anchor token.  Fields are private and this crate exposes
/// no public constructor or deserializer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedGenericAcceptedAnchorTokenV1 {
    domain_id: [u8; 32],
    subject_id: [u8; 32],
    state_epoch: u64,
    state_commitment: [u8; 32],
    anchor_sequence: u64,
    previous_anchor_digest: Option<[u8; 32]>,
    anchor_nonce: [u8; 32],
    anchor_digest: [u8; 32],
    witness_set_digest: [u8; 32],
    witness_set_epoch: u64,
    witness_quorum_digest: [u8; 32],
    trust_snapshot_digest: [u8; 32],
    observed_at_micros: i64,
    expires_at_micros: i64,
    recovery_authority_epoch: u64,
    resolved_recovery_receipt_digest: Option<[u8; 32]>,
}

impl VerifiedGenericAcceptedAnchorTokenV1 {
    pub fn anchor_sequence(&self) -> u64 {
        self.anchor_sequence
    }

    pub fn anchor_digest(&self) -> [u8; 32] {
        self.anchor_digest
    }

    pub fn state_commitment(&self) -> [u8; 32] {
        self.state_commitment
    }

    pub fn expires_at_micros(&self) -> i64 {
        self.expires_at_micros
    }
}

impl adapter::VerifiedGenericAcceptedAnchorV1 for VerifiedGenericAcceptedAnchorTokenV1 {
    fn schema_version(&self) -> u16 {
        adapter::GENERIC_ACCEPTED_ANCHOR_SCHEMA_V1
    }

    fn protocol_id(&self) -> &str {
        adapter::GENERIC_PROTOCOL_V1
    }

    fn domain_id(&self) -> [u8; 32] {
        self.domain_id
    }

    fn subject_id(&self) -> [u8; 32] {
        self.subject_id
    }

    fn state_epoch(&self) -> u64 {
        self.state_epoch
    }

    fn state_commitment(&self) -> [u8; 32] {
        self.state_commitment
    }

    fn anchor_sequence(&self) -> u64 {
        self.anchor_sequence
    }

    fn previous_anchor_digest(&self) -> Option<[u8; 32]> {
        self.previous_anchor_digest
    }

    fn anchor_nonce(&self) -> [u8; 32] {
        self.anchor_nonce
    }

    fn anchor_digest(&self) -> [u8; 32] {
        self.anchor_digest
    }

    fn witness_set_digest(&self) -> [u8; 32] {
        self.witness_set_digest
    }

    fn witness_set_epoch(&self) -> u64 {
        self.witness_set_epoch
    }

    fn witness_quorum_digest(&self) -> [u8; 32] {
        self.witness_quorum_digest
    }

    fn trust_snapshot_digest(&self) -> [u8; 32] {
        self.trust_snapshot_digest
    }

    fn observed_at_micros(&self) -> i64 {
        self.observed_at_micros
    }

    fn expires_at_micros(&self) -> i64 {
        self.expires_at_micros
    }

    fn recovery_authority_epoch(&self) -> u64 {
        self.recovery_authority_epoch
    }

    fn resolved_recovery_receipt_digest(&self) -> Option<[u8; 32]> {
        self.resolved_recovery_receipt_digest
    }
}

/// Concrete state-commitment derivation token.  It binds one exact canonical
/// Health transcript to one verifier-established commitment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedGenericStateCommitmentTokenV1 {
    transcript: Vec<u8>,
    commitment: [u8; 32],
}

impl VerifiedGenericStateCommitmentTokenV1 {
    pub fn commitment(&self) -> [u8; 32] {
        self.commitment
    }
}

impl adapter::VerifiedGenericStateCommitmentDeriverV1
    for VerifiedGenericStateCommitmentTokenV1
{
    fn protocol_id(&self) -> &str {
        adapter::STATE_COMMITMENT_DERIVER_PROTOCOL_V1
    }

    fn derive_state_commitment(&self, transcript: &[u8]) -> [u8; 32] {
        if transcript == self.transcript.as_slice() {
            self.commitment
        } else {
            [0; 32]
        }
    }
}

/// Concrete Health security-time token.  It is bound to one exact generic
/// anchor and generic state commitment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedHealthSecurityTimeTokenV1 {
    anchor_sequence: u64,
    anchor_digest: [u8; 32],
    state_commitment: [u8; 32],
    proof_digest: [u8; 32],
    security_time_floor_micros: i64,
    observed_at_micros: i64,
    expires_at_micros: i64,
}

impl VerifiedHealthSecurityTimeTokenV1 {
    pub fn proof_digest(&self) -> [u8; 32] {
        self.proof_digest
    }

    pub fn security_time_floor_micros(&self) -> i64 {
        self.security_time_floor_micros
    }
}

impl adapter::VerifiedHealthSecurityTimeFloorV1 for VerifiedHealthSecurityTimeTokenV1 {
    fn protocol_id(&self) -> &str {
        adapter::SECURITY_TIME_FLOOR_PROTOCOL_V1
    }

    fn anchor_sequence(&self) -> u64 {
        self.anchor_sequence
    }

    fn anchor_digest(&self) -> [u8; 32] {
        self.anchor_digest
    }

    fn state_commitment(&self) -> [u8; 32] {
        self.state_commitment
    }

    fn proof_digest(&self) -> [u8; 32] {
        self.proof_digest
    }

    fn security_time_floor_micros(&self) -> i64 {
        self.security_time_floor_micros
    }

    fn observed_at_micros(&self) -> i64 {
        self.observed_at_micros
    }

    fn expires_at_micros(&self) -> i64 {
        self.expires_at_micros
    }
}

/// Health-compatible proof reachable only through the concrete token path in
/// this crate.  The lower adapter proof is kept private.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifierBoundHealthAnchorProofV1 {
    inner: adapter::HealthGenericAnchorProofV1,
}

impl VerifierBoundHealthAnchorProofV1 {
    pub fn generic_state_commitment(&self) -> [u8; 32] {
        self.inner.generic_state_commitment()
    }

    pub fn security_time_proof_digest(&self) -> [u8; 32] {
        self.inner.security_time_proof_digest()
    }
}

impl health::VerifiedExternalAnchorProof for VerifierBoundHealthAnchorProofV1 {
    fn protocol_id(&self) -> &str {
        <adapter::HealthGenericAnchorProofV1 as health::VerifiedExternalAnchorProof>::protocol_id(
            &self.inner,
        )
    }

    fn subject_domain(&self) -> &str {
        <adapter::HealthGenericAnchorProofV1 as health::VerifiedExternalAnchorProof>::subject_domain(
            &self.inner,
        )
    }

    fn state(&self) -> health::ExternalAnchorState {
        <adapter::HealthGenericAnchorProofV1 as health::VerifiedExternalAnchorProof>::state(
            &self.inner,
        )
    }

    fn anchor_sequence(&self) -> u64 {
        <adapter::HealthGenericAnchorProofV1 as health::VerifiedExternalAnchorProof>::anchor_sequence(
            &self.inner,
        )
    }

    fn previous_anchor_digest(&self) -> Option<health::ExternalAnchorDigest> {
        <adapter::HealthGenericAnchorProofV1 as health::VerifiedExternalAnchorProof>::previous_anchor_digest(
            &self.inner,
        )
    }

    fn anchor_nonce(&self) -> health::AnchorNonce {
        <adapter::HealthGenericAnchorProofV1 as health::VerifiedExternalAnchorProof>::anchor_nonce(
            &self.inner,
        )
    }

    fn anchor_digest(&self) -> health::ExternalAnchorDigest {
        <adapter::HealthGenericAnchorProofV1 as health::VerifiedExternalAnchorProof>::anchor_digest(
            &self.inner,
        )
    }

    fn security_time_floor_micros(&self) -> i64 {
        <adapter::HealthGenericAnchorProofV1 as health::VerifiedExternalAnchorProof>::security_time_floor_micros(
            &self.inner,
        )
    }

    fn witness_set_epoch(&self) -> u64 {
        <adapter::HealthGenericAnchorProofV1 as health::VerifiedExternalAnchorProof>::witness_set_epoch(
            &self.inner,
        )
    }

    fn witness_quorum_digest(&self) -> health::WitnessQuorumDigest {
        <adapter::HealthGenericAnchorProofV1 as health::VerifiedExternalAnchorProof>::witness_quorum_digest(
            &self.inner,
        )
    }

    fn trust_snapshot_digest(&self) -> health::AnchorTrustSnapshotDigest {
        <adapter::HealthGenericAnchorProofV1 as health::VerifiedExternalAnchorProof>::trust_snapshot_digest(
            &self.inner,
        )
    }

    fn observed_at_micros(&self) -> i64 {
        <adapter::HealthGenericAnchorProofV1 as health::VerifiedExternalAnchorProof>::observed_at_micros(
            &self.inner,
        )
    }

    fn expires_at_micros(&self) -> i64 {
        <adapter::HealthGenericAnchorProofV1 as health::VerifiedExternalAnchorProof>::expires_at_micros(
            &self.inner,
        )
    }
}

/// Final semantic conversion for the concrete-token path.  There is no generic
/// type parameter here for arbitrary downstream `Verified*` implementations.
pub fn bind_verifier_tokens_to_health(
    expected_health_state: health::ExternalAnchorState,
    generic: &VerifiedGenericAcceptedAnchorTokenV1,
    commitment: &VerifiedGenericStateCommitmentTokenV1,
    security_time: &VerifiedHealthSecurityTimeTokenV1,
    now_micros: i64,
) -> Result<VerifierBoundHealthAnchorProofV1, adapter::AdapterError> {
    adapter::bind_generic_anchor_to_health(
        adapter::ADAPTER_SCHEMA_V1,
        expected_health_state,
        generic,
        commitment,
        security_time,
        now_micros,
    )
    .map(|inner| VerifierBoundHealthAnchorProofV1 { inner })
}

/// Wrapper around an externally anchored qualification head.  The field is
/// private so a caller cannot take a head obtained through #220's open reference
/// trait seam and relabel it as verifier-bound authority.
pub struct VerifierAnchoredQualificationHead {
    inner: health::AnchoredQualificationHead,
}

impl VerifierAnchoredQualificationHead {
    pub fn current_head(&self) -> mycelix_qualification_product_authority_core::VerifiedCurrentHead208 {
        self.inner.current_head()
    }

    pub fn anchor_sequence(&self) -> u64 {
        self.inner.anchor_sequence()
    }

    pub fn anchor_digest(&self) -> health::ExternalAnchorDigest {
        self.inner.anchor_digest()
    }
}

pub fn prepare_anchor_bootstrap(
    local: &health::LocalQualificationHead,
    anchor_nonce: health::AnchorNonce,
    now_micros: i64,
) -> Result<health::AnchorUpdateIntent, health::AnchorError> {
    health::prepare_anchor_bootstrap(local, anchor_nonce, now_micros)
}

pub fn prepare_anchor_successor(
    local: &health::LocalQualificationHead,
    prior: &VerifierBoundHealthAnchorProofV1,
    anchor_nonce: health::AnchorNonce,
    now_micros: i64,
) -> Result<health::AnchorUpdateIntent, health::AnchorError> {
    health::prepare_anchor_successor(local, &prior.inner, anchor_nonce, now_micros)
}

pub fn finalize_anchor_update(
    local: &health::LocalQualificationHead,
    intent: &health::AnchorUpdateIntent,
    proof: &VerifierBoundHealthAnchorProofV1,
    now_micros: i64,
) -> Result<VerifierAnchoredQualificationHead, health::AnchorError> {
    health::finalize_anchor_update(local, intent, &proof.inner, now_micros)
        .map(|inner| VerifierAnchoredQualificationHead { inner })
}

pub fn reconcile_existing_anchor(
    local: &health::LocalQualificationHead,
    proof: &VerifierBoundHealthAnchorProofV1,
    now_micros: i64,
) -> Result<VerifierAnchoredQualificationHead, health::AnchorError> {
    health::reconcile_existing_anchor(local, &proof.inner, now_micros)
        .map(|inner| VerifierAnchoredQualificationHead { inner })
}

/// Product token reachable only from a verifier-bound anchored head through the
/// wrapper gate below.
pub struct VerifierBoundProfile208ProductAuthorityToken {
    inner: health::AnchoredProfile208ProductAuthorityToken,
}

impl VerifierBoundProfile208ProductAuthorityToken {
    pub fn receipt_digest(&self) -> qual::ReceiptDigest {
        self.inner.receipt_digest()
    }

    pub fn checkpoint_epoch(&self) -> u64 {
        self.inner.checkpoint_epoch()
    }

    pub fn anchor_sequence(&self) -> u64 {
        self.inner.anchor_sequence()
    }

    pub fn anchor_digest(&self) -> health::ExternalAnchorDigest {
        self.inner.anchor_digest()
    }
}

pub struct VerifierBoundProfile208ProductAuthorityGate {
    inner: health::AnchoredProfile208ProductAuthorityGate,
}

impl VerifierBoundProfile208ProductAuthorityGate {
    pub fn new(profile: health::Profile208CompositionCommitment) -> Self {
        Self {
            inner: health::AnchoredProfile208ProductAuthorityGate::new(profile),
        }
    }

    pub fn convert(
        &mut self,
        qualification: &qual::VerifiedQualification,
        ledger: &qual::QualificationLedger,
        profile_match: health::VerifiedProfile208Match,
        anchored_head: VerifierAnchoredQualificationHead,
        now_micros: i64,
    ) -> Result<VerifierBoundProfile208ProductAuthorityToken, health::AnchorError> {
        self.inner
            .convert(
                qualification,
                ledger,
                profile_match,
                anchored_head.inner,
                now_micros,
            )
            .map(|inner| VerifierBoundProfile208ProductAuthorityToken { inner })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_qualification_product_authority_core as product;

    fn b(value: u8) -> [u8; 32] {
        [value; 32]
    }

    fn health_state() -> health::ExternalAnchorState {
        health::ExternalAnchorState::new(
            product::LineageBindingDigest::new(b(1)).unwrap(),
            product::CheckpointDigest::new(b(2)).unwrap(),
            1,
            1,
            product::LineageStateCommitmentDigest::new(b(3)).unwrap(),
            product::VerifiedHeadDigest::new(b(4)).unwrap(),
        )
        .unwrap()
    }

    fn test_commitment(transcript: &[u8]) -> [u8; 32] {
        let mut out = [0_u8; 32];
        for (index, byte) in transcript.iter().copied().enumerate() {
            let slot = index % 32;
            out[slot] = out[slot]
                .wrapping_add(byte)
                .rotate_left((index % 7) as u32);
        }
        if out == [0; 32] {
            out[0] = 1;
        }
        out
    }

    fn verifier_tokens(
        state: health::ExternalAnchorState,
    ) -> (
        VerifiedGenericAcceptedAnchorTokenV1,
        VerifiedGenericStateCommitmentTokenV1,
        VerifiedHealthSecurityTimeTokenV1,
    ) {
        let transcript = adapter::canonical_health_state_transcript(state);
        let state_commitment = test_commitment(&transcript);
        let generic = VerifiedGenericAcceptedAnchorTokenV1 {
            domain_id: adapter::HEALTH_GENERIC_DOMAIN_ID_V1,
            subject_id: *state.lineage_binding_digest().as_bytes(),
            state_epoch: state.checkpoint_epoch(),
            state_commitment,
            anchor_sequence: 1,
            previous_anchor_digest: None,
            anchor_nonce: b(10),
            anchor_digest: b(11),
            witness_set_digest: b(12),
            witness_set_epoch: 1,
            witness_quorum_digest: b(13),
            trust_snapshot_digest: b(14),
            observed_at_micros: 100,
            expires_at_micros: 1_000,
            recovery_authority_epoch: 0,
            resolved_recovery_receipt_digest: None,
        };
        let commitment = VerifiedGenericStateCommitmentTokenV1 {
            transcript,
            commitment: state_commitment,
        };
        let time = VerifiedHealthSecurityTimeTokenV1 {
            anchor_sequence: 1,
            anchor_digest: b(11),
            state_commitment,
            proof_digest: b(15),
            security_time_floor_micros: 110,
            observed_at_micros: 120,
            expires_at_micros: 900,
        };
        (generic, commitment, time)
    }

    #[test]
    fn concrete_private_tokens_reach_the_same_semantic_mapping() {
        let state = health_state();
        let (generic, commitment, time) = verifier_tokens(state);
        let proof = bind_verifier_tokens_to_health(state, &generic, &commitment, &time, 130)
            .unwrap();
        assert_eq!(
            <VerifierBoundHealthAnchorProofV1 as health::VerifiedExternalAnchorProof>::state(
                &proof,
            ),
            state
        );
        assert_eq!(
            <VerifierBoundHealthAnchorProofV1 as health::VerifiedExternalAnchorProof>::anchor_sequence(
                &proof,
            ),
            1
        );
        assert_eq!(proof.generic_state_commitment(), generic.state_commitment());
        assert_eq!(proof.security_time_proof_digest(), time.proof_digest());
    }

    #[test]
    fn commitment_token_is_bound_to_one_exact_transcript() {
        let state = health_state();
        let (generic, mut commitment, time) = verifier_tokens(state);
        commitment.transcript.push(0xff);
        assert_eq!(
            bind_verifier_tokens_to_health(state, &generic, &commitment, &time, 130),
            Err(adapter::AdapterError::ZeroGenericStateCommitment)
        );
    }

    #[test]
    fn recovered_generic_token_remains_fail_closed_in_v1() {
        let state = health_state();
        let (mut generic, commitment, time) = verifier_tokens(state);
        generic.recovery_authority_epoch = 1;
        generic.resolved_recovery_receipt_digest = Some(b(88));
        assert_eq!(
            bind_verifier_tokens_to_health(state, &generic, &commitment, &time, 130),
            Err(adapter::AdapterError::RecoveredGenericLineageUnsupported)
        );
    }
}
