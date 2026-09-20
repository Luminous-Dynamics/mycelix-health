#![forbid(unsafe_code)]
//! Exact semantic compatibility layer between the generic Luminous monotonic
//! transparency protocol and Mycelix Health QUAL-EVID-005.
//!
//! This crate does **not** depend on the unqualified cross-repository generic
//! implementation. Instead it freezes the accepted-head contract and exact
//! Health mapping that a future qualified generic backend must satisfy.
//!
//! There is no `verified: bool` input. Generic currentness, state-commitment
//! derivation, and the Health security-time floor cross separate typed evidence
//! boundaries and are rechecked before this crate implements the Health
//! `VerifiedExternalAnchorProof` trait.

use mycelix_qualification_anti_rollback_anchor_core as health;

pub const ADAPTER_SCHEMA_V1: u16 = 1;
pub const GENERIC_ACCEPTED_ANCHOR_SCHEMA_V1: u16 = 1;
pub const GENERIC_PROTOCOL_V1: &str = "mycelix-monotonic-transparency-v1";
pub const HEALTH_ADAPTER_PROTOCOL_V1: &str =
    "mycelix-health-generic-transparency-adapter-v1";
pub const STATE_COMMITMENT_DERIVER_PROTOCOL_V1: &str =
    "mycelix-health-generic-state-commitment-deriver-v1";
pub const SECURITY_TIME_FLOOR_PROTOCOL_V1: &str =
    "mycelix-health-qualified-security-time-floor-v1";
pub const STATE_TRANSCRIPT_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-ANTI-ROLLBACK-STATE-V1\0";

/// SHA-256("MYCELIX-HEALTH-QUALIFICATION-ANTI-ROLLBACK-V1").
///
/// The digest is frozen here as protocol identity; this semantic crate does not
/// contain a general-purpose hashing implementation.
pub const HEALTH_GENERIC_DOMAIN_ID_V1: [u8; 32] = [
    0xcd, 0xb6, 0x0b, 0xe3, 0x23, 0x75, 0x35, 0x84, 0xce, 0x36, 0x81, 0xc2, 0xbb, 0x8c,
    0xd5, 0x49, 0x9e, 0xc5, 0x69, 0x98, 0x7a, 0x83, 0x83, 0xbd, 0xe5, 0x62, 0x47, 0x34,
    0xee, 0xca, 0x3c, 0xdf,
];

/// Canonical bytes committed by the generic state commitment for one Health
/// qualification head.
///
/// All fields are fixed width after the domain prefix, so the framing is
/// unambiguous without schema-dependent delimiters.
pub fn canonical_health_state_transcript(state: health::ExternalAnchorState) -> Vec<u8> {
    let mut out = Vec::with_capacity(STATE_TRANSCRIPT_DOMAIN_V1.len() + 32 * 5 + 16 + 32);
    out.extend_from_slice(STATE_TRANSCRIPT_DOMAIN_V1);
    out.extend_from_slice(&HEALTH_GENERIC_DOMAIN_ID_V1);
    out.extend_from_slice(state.lineage_binding_digest().as_bytes());
    out.extend_from_slice(state.checkpoint_digest().as_bytes());
    out.extend_from_slice(&state.checkpoint_epoch().to_be_bytes());
    out.extend_from_slice(&state.head_sequence().to_be_bytes());
    out.extend_from_slice(state.lineage_state_commitment().as_bytes());
    out.extend_from_slice(state.verified_head_digest().as_bytes());
    out
}

/// Future generic core/backend contract required by Health #221.
///
/// The artifact must represent one already-current, conflict-free accepted
/// generic anchor. Issue Luminous-Dynamics/luminous-dynamics#2038 tracks making
/// the generic core expose this complete identity directly.
pub trait VerifiedGenericAcceptedAnchorV1 {
    fn schema_version(&self) -> u16;
    fn protocol_id(&self) -> &str;
    fn domain_id(&self) -> [u8; 32];
    fn subject_id(&self) -> [u8; 32];
    fn state_epoch(&self) -> u64;
    fn state_commitment(&self) -> [u8; 32];
    fn anchor_sequence(&self) -> u64;
    fn previous_anchor_digest(&self) -> Option<[u8; 32]>;
    fn anchor_nonce(&self) -> [u8; 32];
    fn anchor_digest(&self) -> [u8; 32];
    fn witness_set_digest(&self) -> [u8; 32];
    fn witness_set_epoch(&self) -> u64;
    fn witness_quorum_digest(&self) -> [u8; 32];
    fn trust_snapshot_digest(&self) -> [u8; 32];
    fn observed_at_micros(&self) -> i64;
    fn expires_at_micros(&self) -> i64;

    /// V1 Health interoperability deliberately refuses recovered generic
    /// lineages until recovery-policy mapping is independently qualified.
    fn recovery_authority_epoch(&self) -> u64;
    fn resolved_recovery_receipt_digest(&self) -> Option<[u8; 32]>;
}

/// Separately qualified derivation of the opaque generic state commitment from
/// the exact transcript frozen above.
pub trait VerifiedGenericStateCommitmentDeriverV1 {
    fn protocol_id(&self) -> &str;
    fn derive_state_commitment(&self, transcript: &[u8]) -> [u8; 32];
}

/// Health retains a separate security-time-floor theorem instead of pretending
/// generic checkpoint freshness supplies rollback-resistant trusted time.
pub trait VerifiedHealthSecurityTimeFloorV1 {
    fn protocol_id(&self) -> &str;
    fn anchor_sequence(&self) -> u64;
    fn anchor_digest(&self) -> [u8; 32];
    fn state_commitment(&self) -> [u8; 32];
    fn proof_digest(&self) -> [u8; 32];
    fn security_time_floor_micros(&self) -> i64;
    fn observed_at_micros(&self) -> i64;
    fn expires_at_micros(&self) -> i64;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterError {
    UnsupportedAdapterSchema,
    UnsupportedGenericSchema,
    UnsupportedGenericProtocol,
    UnsupportedCommitmentDeriver,
    UnsupportedSecurityTimeProtocol,
    GenericDomainMismatch,
    GenericSubjectMismatch,
    GenericStateEpochMismatch,
    GenericStateCommitmentMismatch,
    InvalidGenericAnchorSequence,
    InvalidGenericPredecessorShape,
    InvalidGenericWitnessSetEpoch,
    InvalidGenericWindow,
    GenericObservedInFuture,
    GenericExpired,
    RecoveredGenericLineageUnsupported,
    ZeroGenericStateCommitment,
    ZeroGenericAnchorDigest,
    ZeroGenericAnchorNonce,
    ZeroGenericWitnessSetDigest,
    ZeroGenericWitnessQuorumDigest,
    ZeroGenericTrustSnapshotDigest,
    SecurityTimeAnchorSequenceMismatch,
    SecurityTimeAnchorMismatch,
    SecurityTimeStateMismatch,
    ZeroSecurityTimeProofDigest,
    InvalidSecurityTimeWindow,
    SecurityTimeObservedInFuture,
    SecurityTimeProofExpired,
    InvalidSecurityTimeFloor,
    SecurityTimeFloorAfterObservation,
}

/// Exact Health-compatible proof produced only after all generic/Health mapping
/// checks succeed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealthGenericAnchorProofV1 {
    state: health::ExternalAnchorState,
    anchor_sequence: u64,
    previous_anchor_digest: Option<health::ExternalAnchorDigest>,
    anchor_nonce: health::AnchorNonce,
    anchor_digest: health::ExternalAnchorDigest,
    security_time_floor_micros: i64,
    witness_set_epoch: u64,
    witness_quorum_digest: health::WitnessQuorumDigest,
    trust_snapshot_digest: health::AnchorTrustSnapshotDigest,
    observed_at_micros: i64,
    expires_at_micros: i64,
    generic_state_commitment: [u8; 32],
    generic_witness_set_digest: [u8; 32],
    security_time_proof_digest: [u8; 32],
}

impl HealthGenericAnchorProofV1 {
    pub fn generic_state_commitment(&self) -> [u8; 32] {
        self.generic_state_commitment
    }

    pub fn generic_witness_set_digest(&self) -> [u8; 32] {
        self.generic_witness_set_digest
    }

    pub fn security_time_proof_digest(&self) -> [u8; 32] {
        self.security_time_proof_digest
    }
}

impl health::VerifiedExternalAnchorProof for HealthGenericAnchorProofV1 {
    fn protocol_id(&self) -> &str {
        health::QUALIFICATION_ANCHOR_PROTOCOL_V1
    }

    fn subject_domain(&self) -> &str {
        health::QUALIFICATION_ANCHOR_SUBJECT_DOMAIN_V1
    }

    fn state(&self) -> health::ExternalAnchorState {
        self.state
    }

    fn anchor_sequence(&self) -> u64 {
        self.anchor_sequence
    }

    fn previous_anchor_digest(&self) -> Option<health::ExternalAnchorDigest> {
        self.previous_anchor_digest
    }

    fn anchor_nonce(&self) -> health::AnchorNonce {
        self.anchor_nonce
    }

    fn anchor_digest(&self) -> health::ExternalAnchorDigest {
        self.anchor_digest
    }

    fn security_time_floor_micros(&self) -> i64 {
        self.security_time_floor_micros
    }

    fn witness_set_epoch(&self) -> u64 {
        self.witness_set_epoch
    }

    fn witness_quorum_digest(&self) -> health::WitnessQuorumDigest {
        self.witness_quorum_digest
    }

    fn trust_snapshot_digest(&self) -> health::AnchorTrustSnapshotDigest {
        self.trust_snapshot_digest
    }

    fn observed_at_micros(&self) -> i64 {
        self.observed_at_micros
    }

    fn expires_at_micros(&self) -> i64 {
        self.expires_at_micros
    }
}

#[allow(clippy::too_many_arguments)]
pub fn bind_generic_anchor_to_health<G, D, T>(
    adapter_schema: u16,
    expected_health_state: health::ExternalAnchorState,
    generic: &G,
    commitment_deriver: &D,
    security_time: &T,
    now_micros: i64,
) -> Result<HealthGenericAnchorProofV1, AdapterError>
where
    G: VerifiedGenericAcceptedAnchorV1,
    D: VerifiedGenericStateCommitmentDeriverV1,
    T: VerifiedHealthSecurityTimeFloorV1,
{
    if adapter_schema != ADAPTER_SCHEMA_V1 {
        return Err(AdapterError::UnsupportedAdapterSchema);
    }
    if generic.schema_version() != GENERIC_ACCEPTED_ANCHOR_SCHEMA_V1 {
        return Err(AdapterError::UnsupportedGenericSchema);
    }
    if generic.protocol_id() != GENERIC_PROTOCOL_V1 {
        return Err(AdapterError::UnsupportedGenericProtocol);
    }
    if commitment_deriver.protocol_id() != STATE_COMMITMENT_DERIVER_PROTOCOL_V1 {
        return Err(AdapterError::UnsupportedCommitmentDeriver);
    }
    if security_time.protocol_id() != SECURITY_TIME_FLOOR_PROTOCOL_V1 {
        return Err(AdapterError::UnsupportedSecurityTimeProtocol);
    }

    if generic.domain_id() != HEALTH_GENERIC_DOMAIN_ID_V1 {
        return Err(AdapterError::GenericDomainMismatch);
    }
    if generic.subject_id() != *expected_health_state.lineage_binding_digest().as_bytes() {
        return Err(AdapterError::GenericSubjectMismatch);
    }
    if generic.state_epoch() != expected_health_state.checkpoint_epoch() {
        return Err(AdapterError::GenericStateEpochMismatch);
    }

    let transcript = canonical_health_state_transcript(expected_health_state);
    let expected_commitment = commitment_deriver.derive_state_commitment(&transcript);
    if expected_commitment == [0; 32] {
        return Err(AdapterError::ZeroGenericStateCommitment);
    }
    if generic.state_commitment() != expected_commitment {
        return Err(AdapterError::GenericStateCommitmentMismatch);
    }

    if generic.anchor_sequence() == 0 {
        return Err(AdapterError::InvalidGenericAnchorSequence);
    }
    if (generic.anchor_sequence() == 1 && generic.previous_anchor_digest().is_some())
        || (generic.anchor_sequence() > 1 && generic.previous_anchor_digest().is_none())
    {
        return Err(AdapterError::InvalidGenericPredecessorShape);
    }
    if generic.witness_set_epoch() == 0 {
        return Err(AdapterError::InvalidGenericWitnessSetEpoch);
    }
    if generic.observed_at_micros() < 0
        || generic.expires_at_micros() <= generic.observed_at_micros()
    {
        return Err(AdapterError::InvalidGenericWindow);
    }
    if generic.observed_at_micros() > now_micros {
        return Err(AdapterError::GenericObservedInFuture);
    }
    if now_micros >= generic.expires_at_micros() {
        return Err(AdapterError::GenericExpired);
    }
    if generic.recovery_authority_epoch() != 0
        || generic.resolved_recovery_receipt_digest().is_some()
    {
        return Err(AdapterError::RecoveredGenericLineageUnsupported);
    }

    let generic_anchor_digest = generic.anchor_digest();
    let generic_anchor_nonce = generic.anchor_nonce();
    let generic_witness_set_digest = generic.witness_set_digest();
    let generic_witness_quorum_digest = generic.witness_quorum_digest();
    let generic_trust_snapshot_digest = generic.trust_snapshot_digest();

    if generic_anchor_digest == [0; 32] {
        return Err(AdapterError::ZeroGenericAnchorDigest);
    }
    if generic_anchor_nonce == [0; 32] {
        return Err(AdapterError::ZeroGenericAnchorNonce);
    }
    if generic_witness_set_digest == [0; 32] {
        return Err(AdapterError::ZeroGenericWitnessSetDigest);
    }
    if generic_witness_quorum_digest == [0; 32] {
        return Err(AdapterError::ZeroGenericWitnessQuorumDigest);
    }
    if generic_trust_snapshot_digest == [0; 32] {
        return Err(AdapterError::ZeroGenericTrustSnapshotDigest);
    }

    if security_time.anchor_sequence() != generic.anchor_sequence() {
        return Err(AdapterError::SecurityTimeAnchorSequenceMismatch);
    }
    if security_time.anchor_digest() != generic_anchor_digest {
        return Err(AdapterError::SecurityTimeAnchorMismatch);
    }
    if security_time.state_commitment() != expected_commitment {
        return Err(AdapterError::SecurityTimeStateMismatch);
    }
    if security_time.proof_digest() == [0; 32] {
        return Err(AdapterError::ZeroSecurityTimeProofDigest);
    }
    if security_time.observed_at_micros() < 0
        || security_time.expires_at_micros() <= security_time.observed_at_micros()
    {
        return Err(AdapterError::InvalidSecurityTimeWindow);
    }
    if security_time.observed_at_micros() > now_micros {
        return Err(AdapterError::SecurityTimeObservedInFuture);
    }
    if now_micros >= security_time.expires_at_micros() {
        return Err(AdapterError::SecurityTimeProofExpired);
    }
    let floor = security_time.security_time_floor_micros();
    if floor < 0 {
        return Err(AdapterError::InvalidSecurityTimeFloor);
    }
    if floor > security_time.observed_at_micros() {
        return Err(AdapterError::SecurityTimeFloorAfterObservation);
    }

    let previous_anchor_digest = match generic.previous_anchor_digest() {
        Some(bytes) => Some(
            health::ExternalAnchorDigest::new(bytes)
                .map_err(|_| AdapterError::ZeroGenericAnchorDigest)?,
        ),
        None => None,
    };
    let anchor_digest = health::ExternalAnchorDigest::new(generic_anchor_digest)
        .map_err(|_| AdapterError::ZeroGenericAnchorDigest)?;
    let anchor_nonce = health::AnchorNonce::new(generic_anchor_nonce)
        .map_err(|_| AdapterError::ZeroGenericAnchorNonce)?;
    let witness_quorum_digest = health::WitnessQuorumDigest::new(generic_witness_quorum_digest)
        .map_err(|_| AdapterError::ZeroGenericWitnessQuorumDigest)?;
    let trust_snapshot_digest = health::AnchorTrustSnapshotDigest::new(generic_trust_snapshot_digest)
        .map_err(|_| AdapterError::ZeroGenericTrustSnapshotDigest)?;

    // Both evidence objects must be current.  The resulting Health proof is only
    // valid for the intersection of their validity windows and not before both
    // were observed.
    let observed_at_micros = generic
        .observed_at_micros()
        .max(security_time.observed_at_micros());
    let expires_at_micros = generic
        .expires_at_micros()
        .min(security_time.expires_at_micros());

    Ok(HealthGenericAnchorProofV1 {
        state: expected_health_state,
        anchor_sequence: generic.anchor_sequence(),
        previous_anchor_digest,
        anchor_nonce,
        anchor_digest,
        security_time_floor_micros: floor,
        witness_set_epoch: generic.witness_set_epoch(),
        witness_quorum_digest,
        trust_snapshot_digest,
        observed_at_micros,
        expires_at_micros,
        generic_state_commitment: expected_commitment,
        generic_witness_set_digest,
        security_time_proof_digest: security_time.proof_digest(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_qualification_product_authority_core as product;
    use health::VerifiedExternalAnchorProof as _;

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

    #[derive(Clone, Copy)]
    struct TestDeriver;

    impl VerifiedGenericStateCommitmentDeriverV1 for TestDeriver {
        fn protocol_id(&self) -> &str {
            STATE_COMMITMENT_DERIVER_PROTOCOL_V1
        }

        fn derive_state_commitment(&self, transcript: &[u8]) -> [u8; 32] {
            // Test-only deterministic commitment. Production derivation is an
            // independently qualified adapter and is intentionally not modeled
            // as cryptography in this semantic crate.
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
    }

    #[derive(Clone, Copy)]
    struct GenericHead {
        schema: u16,
        protocol: &'static str,
        domain: [u8; 32],
        subject: [u8; 32],
        epoch: u64,
        commitment: [u8; 32],
        sequence: u64,
        previous: Option<[u8; 32]>,
        nonce: [u8; 32],
        anchor: [u8; 32],
        witness_set: [u8; 32],
        witness_epoch: u64,
        quorum: [u8; 32],
        trust: [u8; 32],
        observed: i64,
        expires: i64,
        recovery_epoch: u64,
        recovery_receipt: Option<[u8; 32]>,
    }

    impl VerifiedGenericAcceptedAnchorV1 for GenericHead {
        fn schema_version(&self) -> u16 { self.schema }
        fn protocol_id(&self) -> &str { self.protocol }
        fn domain_id(&self) -> [u8; 32] { self.domain }
        fn subject_id(&self) -> [u8; 32] { self.subject }
        fn state_epoch(&self) -> u64 { self.epoch }
        fn state_commitment(&self) -> [u8; 32] { self.commitment }
        fn anchor_sequence(&self) -> u64 { self.sequence }
        fn previous_anchor_digest(&self) -> Option<[u8; 32]> { self.previous }
        fn anchor_nonce(&self) -> [u8; 32] { self.nonce }
        fn anchor_digest(&self) -> [u8; 32] { self.anchor }
        fn witness_set_digest(&self) -> [u8; 32] { self.witness_set }
        fn witness_set_epoch(&self) -> u64 { self.witness_epoch }
        fn witness_quorum_digest(&self) -> [u8; 32] { self.quorum }
        fn trust_snapshot_digest(&self) -> [u8; 32] { self.trust }
        fn observed_at_micros(&self) -> i64 { self.observed }
        fn expires_at_micros(&self) -> i64 { self.expires }
        fn recovery_authority_epoch(&self) -> u64 { self.recovery_epoch }
        fn resolved_recovery_receipt_digest(&self) -> Option<[u8; 32]> { self.recovery_receipt }
    }

    #[derive(Clone, Copy)]
    struct TimeFloor {
        protocol: &'static str,
        sequence: u64,
        anchor: [u8; 32],
        state: [u8; 32],
        proof: [u8; 32],
        floor: i64,
        observed: i64,
        expires: i64,
    }

    impl VerifiedHealthSecurityTimeFloorV1 for TimeFloor {
        fn protocol_id(&self) -> &str { self.protocol }
        fn anchor_sequence(&self) -> u64 { self.sequence }
        fn anchor_digest(&self) -> [u8; 32] { self.anchor }
        fn state_commitment(&self) -> [u8; 32] { self.state }
        fn proof_digest(&self) -> [u8; 32] { self.proof }
        fn security_time_floor_micros(&self) -> i64 { self.floor }
        fn observed_at_micros(&self) -> i64 { self.observed }
        fn expires_at_micros(&self) -> i64 { self.expires }
    }

    fn valid() -> (health::ExternalAnchorState, GenericHead, TimeFloor) {
        let state = health_state();
        let commitment = TestDeriver.derive_state_commitment(&canonical_health_state_transcript(state));
        let generic = GenericHead {
            schema: 1,
            protocol: GENERIC_PROTOCOL_V1,
            domain: HEALTH_GENERIC_DOMAIN_ID_V1,
            subject: *state.lineage_binding_digest().as_bytes(),
            epoch: state.checkpoint_epoch(),
            commitment,
            sequence: 1,
            previous: None,
            nonce: b(10),
            anchor: b(11),
            witness_set: b(12),
            witness_epoch: 1,
            quorum: b(13),
            trust: b(14),
            observed: 100,
            expires: 1_000,
            recovery_epoch: 0,
            recovery_receipt: None,
        };
        let time = TimeFloor {
            protocol: SECURITY_TIME_FLOOR_PROTOCOL_V1,
            sequence: 1,
            anchor: generic.anchor,
            state: commitment,
            proof: b(15),
            floor: 110,
            observed: 120,
            expires: 900,
        };
        (state, generic, time)
    }

    #[test]
    fn exact_mapping_produces_health_proof_with_intersection_window() {
        let (state, generic, time) = valid();
        let proof = bind_generic_anchor_to_health(
            ADAPTER_SCHEMA_V1,
            state,
            &generic,
            &TestDeriver,
            &time,
            130,
        )
        .unwrap();
        assert_eq!(proof.state(), state);
        assert_eq!(proof.anchor_sequence(), 1);
        assert_eq!(proof.previous_anchor_digest(), None);
        assert_eq!(*proof.anchor_digest().as_bytes(), generic.anchor);
        assert_eq!(proof.security_time_floor_micros(), 110);
        assert_eq!(proof.observed_at_micros(), 120);
        assert_eq!(proof.expires_at_micros(), 900);
        assert_eq!(proof.generic_state_commitment(), generic.commitment);
        assert_eq!(proof.generic_witness_set_digest(), generic.witness_set);
        assert_eq!(proof.security_time_proof_digest(), time.proof);
    }

    #[test]
    fn wrong_domain_subject_epoch_or_commitment_are_independent_denials() {
        let (state, generic, time) = valid();
        let cases = [
            (GenericHead { domain: b(99), ..generic }, AdapterError::GenericDomainMismatch),
            (GenericHead { subject: b(99), ..generic }, AdapterError::GenericSubjectMismatch),
            (GenericHead { epoch: 2, ..generic }, AdapterError::GenericStateEpochMismatch),
            (GenericHead { commitment: b(99), ..generic }, AdapterError::GenericStateCommitmentMismatch),
        ];
        for (candidate, expected) in cases {
            assert_eq!(
                bind_generic_anchor_to_health(1, state, &candidate, &TestDeriver, &time, 130),
                Err(expected)
            );
        }
    }

    #[test]
    fn schema_protocol_and_predecessor_shape_fail_closed() {
        let (state, generic, time) = valid();
        assert_eq!(
            bind_generic_anchor_to_health(2, state, &generic, &TestDeriver, &time, 130),
            Err(AdapterError::UnsupportedAdapterSchema)
        );
        let wrong_protocol = GenericHead { protocol: "wrong", ..generic };
        assert_eq!(
            bind_generic_anchor_to_health(1, state, &wrong_protocol, &TestDeriver, &time, 130),
            Err(AdapterError::UnsupportedGenericProtocol)
        );
        let bad_predecessor = GenericHead { previous: Some(b(20)), ..generic };
        assert_eq!(
            bind_generic_anchor_to_health(1, state, &bad_predecessor, &TestDeriver, &time, 130),
            Err(AdapterError::InvalidGenericPredecessorShape)
        );
    }

    #[test]
    fn recovered_generic_lineage_requires_future_policy_mapping() {
        let (state, generic, time) = valid();
        let recovered = GenericHead {
            recovery_epoch: 1,
            recovery_receipt: Some(b(88)),
            ..generic
        };
        assert_eq!(
            bind_generic_anchor_to_health(1, state, &recovered, &TestDeriver, &time, 130),
            Err(AdapterError::RecoveredGenericLineageUnsupported)
        );
    }

    #[test]
    fn stale_or_future_generic_head_never_becomes_health_authority() {
        let (state, generic, time) = valid();
        let future = GenericHead { observed: 200, ..generic };
        assert_eq!(
            bind_generic_anchor_to_health(1, state, &future, &TestDeriver, &time, 130),
            Err(AdapterError::GenericObservedInFuture)
        );
        let expired = GenericHead { expires: 120, ..generic };
        assert_eq!(
            bind_generic_anchor_to_health(1, state, &expired, &TestDeriver, &time, 130),
            Err(AdapterError::GenericExpired)
        );
    }

    #[test]
    fn time_floor_must_bind_exact_anchor_sequence_and_state() {
        let (state, generic, time) = valid();
        let wrong_sequence = TimeFloor { sequence: 2, ..time };
        assert_eq!(
            bind_generic_anchor_to_health(1, state, &generic, &TestDeriver, &wrong_sequence, 130),
            Err(AdapterError::SecurityTimeAnchorSequenceMismatch)
        );
        let wrong_anchor = TimeFloor { anchor: b(99), ..time };
        assert_eq!(
            bind_generic_anchor_to_health(1, state, &generic, &TestDeriver, &wrong_anchor, 130),
            Err(AdapterError::SecurityTimeAnchorMismatch)
        );
        let wrong_state = TimeFloor { state: b(99), ..time };
        assert_eq!(
            bind_generic_anchor_to_health(1, state, &generic, &TestDeriver, &wrong_state, 130),
            Err(AdapterError::SecurityTimeStateMismatch)
        );
    }

    #[test]
    fn invalid_time_floor_window_and_floor_fail_closed() {
        let (state, generic, time) = valid();
        let negative = TimeFloor { floor: -1, ..time };
        assert_eq!(
            bind_generic_anchor_to_health(1, state, &generic, &TestDeriver, &negative, 130),
            Err(AdapterError::InvalidSecurityTimeFloor)
        );
        let after_observation = TimeFloor { floor: 121, ..time };
        assert_eq!(
            bind_generic_anchor_to_health(1, state, &generic, &TestDeriver, &after_observation, 130),
            Err(AdapterError::SecurityTimeFloorAfterObservation)
        );
        let expired = TimeFloor { expires: 125, ..time };
        assert_eq!(
            bind_generic_anchor_to_health(1, state, &generic, &TestDeriver, &expired, 130),
            Err(AdapterError::SecurityTimeProofExpired)
        );
    }
}
