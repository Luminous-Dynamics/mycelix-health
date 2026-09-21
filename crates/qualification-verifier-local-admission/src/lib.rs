#![forbid(unsafe_code)]
//! Linux local-caller admission for the qualification verifier daemon.
//!
//! QUAL-EVID-009C4B1 deliberately composes two independent factors:
//!
//! 1. kernel-reported `SO_PEERCRED` must match the configured UID/GID policy;
//! 2. a one-time daemon auth challenge must carry a valid HMAC-SHA256 capability
//!    proof over the exact peer credentials and exact #239 canonical RPC bytes.
//!
//! The auth challenge is consumed before peer-policy/MAC validation. Any later
//! denial therefore burns the challenge and cannot become a retry oracle.

#[cfg(not(target_os = "linux"))]
compile_error!("qualification-verifier-local-admission V1 is Linux-only");

use std::collections::HashMap;
use std::os::unix::net::UnixStream;

use hmac::{Hmac, Mac};
use mycelix_qualification_verifier_daemon_core as daemon;
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use rand::{RngCore, rngs::OsRng};
use sha2::Sha256;
use zeroize::Zeroizing;

type HmacSha256 = Hmac<Sha256>;

pub const ADMISSION_SCHEMA_V1: u16 = 1;
pub const CAPABILITY_TAG_LEN_V1: usize = 32;
pub const CAPABILITY_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-LOCAL-CAPABILITY-V1\0";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalAuthChallengeV1([u8; 32]);

impl LocalAuthChallengeV1 {
    pub fn new(bytes: [u8; 32]) -> Result<Self, LocalAdmissionError> {
        if bytes == [0; 32] {
            return Err(LocalAdmissionError::ZeroAuthChallenge);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for LocalAuthChallengeV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LocalAuthChallengeV1([redacted])")
    }
}

/// Capability material owned by an authorized local client and the daemon.
/// Secret bytes are zeroized on drop and never included in Debug output.
pub struct LocalCapabilityKeyV1(Zeroizing<[u8; 32]>);

impl LocalCapabilityKeyV1 {
    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self, LocalAdmissionError> {
        if bytes == [0; 32] {
            return Err(LocalAdmissionError::ZeroCapabilityKey);
        }
        Ok(Self(Zeroizing::new(bytes)))
    }

    fn secret(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for LocalCapabilityKeyV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LocalCapabilityKeyV1([redacted])")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxPeerCredentialsV1 {
    pid: i32,
    uid: u32,
    gid: u32,
}

impl LinuxPeerCredentialsV1 {
    pub fn new(pid: i32, uid: u32, gid: u32) -> Result<Self, LocalAdmissionError> {
        if pid <= 0 {
            return Err(LocalAdmissionError::InvalidPeerPid);
        }
        Ok(Self { pid, uid, gid })
    }

    pub fn pid(&self) -> i32 {
        self.pid
    }

    pub fn uid(&self) -> u32 {
        self.uid
    }

    pub fn gid(&self) -> u32 {
        self.gid
    }
}

pub fn linux_peer_credentials_v1(
    stream: &UnixStream,
) -> Result<LinuxPeerCredentialsV1, LocalAdmissionError> {
    let credentials = getsockopt(stream, PeerCredentials)
        .map_err(|_| LocalAdmissionError::PeerCredentialLookupFailed)?;
    LinuxPeerCredentialsV1::new(credentials.pid(), credentials.uid(), credentials.gid())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IssuedLocalAuthChallengeV1 {
    challenge: LocalAuthChallengeV1,
    issued_at_micros: i64,
    expires_at_micros: i64,
}

impl IssuedLocalAuthChallengeV1 {
    pub fn challenge(&self) -> LocalAuthChallengeV1 {
        self.challenge
    }

    pub fn issued_at_micros(&self) -> i64 {
        self.issued_at_micros
    }

    pub fn expires_at_micros(&self) -> i64 {
        self.expires_at_micros
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AuthChallengeRecordV1 {
    issued_at_micros: i64,
    expires_at_micros: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalCapabilityProofV1 {
    challenge: LocalAuthChallengeV1,
    tag: [u8; CAPABILITY_TAG_LEN_V1],
}

impl LocalCapabilityProofV1 {
    pub fn new(
        challenge: LocalAuthChallengeV1,
        tag: [u8; CAPABILITY_TAG_LEN_V1],
    ) -> Result<Self, LocalAdmissionError> {
        if tag == [0; CAPABILITY_TAG_LEN_V1] {
            return Err(LocalAdmissionError::ZeroCapabilityTag);
        }
        Ok(Self { challenge, tag })
    }

    pub fn challenge(&self) -> LocalAuthChallengeV1 {
        self.challenge
    }

    pub fn tag(&self) -> &[u8; CAPABILITY_TAG_LEN_V1] {
        &self.tag
    }
}

/// Positive local-admission result. Fields are private; construction occurs
/// only after challenge consumption, peer-policy match, and MAC verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedLocalRpcRequestV1 {
    peer: LinuxPeerCredentialsV1,
    request: daemon::DaemonRpcRequestV1,
    challenge: LocalAuthChallengeV1,
}

impl AdmittedLocalRpcRequestV1 {
    pub fn peer(&self) -> LinuxPeerCredentialsV1 {
        self.peer
    }

    pub fn request(&self) -> &daemon::DaemonRpcRequestV1 {
        &self.request
    }

    pub fn challenge(&self) -> LocalAuthChallengeV1 {
        self.challenge
    }

    pub fn into_request(self) -> daemon::DaemonRpcRequestV1 {
        self.request
    }
}

pub struct LocalAdmissionPolicyV1 {
    allowed_uid: u32,
    allowed_gid: u32,
    capability: LocalCapabilityKeyV1,
    max_challenge_lifetime_micros: i64,
}

impl LocalAdmissionPolicyV1 {
    pub fn new(
        allowed_uid: u32,
        allowed_gid: u32,
        capability: LocalCapabilityKeyV1,
        max_challenge_lifetime_micros: i64,
    ) -> Result<Self, LocalAdmissionError> {
        if max_challenge_lifetime_micros <= 0 {
            return Err(LocalAdmissionError::InvalidChallengeLifetime);
        }
        Ok(Self {
            allowed_uid,
            allowed_gid,
            capability,
            max_challenge_lifetime_micros,
        })
    }

    pub fn allowed_uid(&self) -> u32 {
        self.allowed_uid
    }

    pub fn allowed_gid(&self) -> u32 {
        self.allowed_gid
    }
}

pub struct LocalAdmissionGateV1 {
    policy: LocalAdmissionPolicyV1,
    challenges: HashMap<LocalAuthChallengeV1, AuthChallengeRecordV1>,
    last_observed_micros: Option<i64>,
}

impl LocalAdmissionGateV1 {
    pub fn new(policy: LocalAdmissionPolicyV1) -> Self {
        Self {
            policy,
            challenges: HashMap::new(),
            last_observed_micros: None,
        }
    }

    pub fn issue_challenge(
        &mut self,
        now_micros: i64,
        requested_lifetime_micros: i64,
    ) -> Result<IssuedLocalAuthChallengeV1, LocalAdmissionError> {
        self.observe_time(now_micros)?;
        if requested_lifetime_micros <= 0
            || requested_lifetime_micros > self.policy.max_challenge_lifetime_micros
        {
            return Err(LocalAdmissionError::InvalidChallengeLifetime);
        }
        let expires_at_micros = now_micros
            .checked_add(requested_lifetime_micros)
            .ok_or(LocalAdmissionError::TimeOverflow)?;

        for _ in 0..16 {
            let mut bytes = [0u8; 32];
            OsRng.fill_bytes(&mut bytes);
            let Ok(challenge) = LocalAuthChallengeV1::new(bytes) else {
                continue;
            };
            if self.challenges.contains_key(&challenge) {
                continue;
            }
            self.challenges.insert(
                challenge,
                AuthChallengeRecordV1 {
                    issued_at_micros: now_micros,
                    expires_at_micros,
                },
            );
            return Ok(IssuedLocalAuthChallengeV1 {
                challenge,
                issued_at_micros: now_micros,
                expires_at_micros,
            });
        }
        Err(LocalAdmissionError::ChallengeGenerationExhausted)
    }

    /// Authenticate one exact RPC request from a connected Linux Unix stream.
    ///
    /// The challenge is removed before peer-policy or HMAC verification. Any
    /// later error leaves it consumed.
    pub fn admit_stream(
        &mut self,
        stream: &UnixStream,
        request: daemon::DaemonRpcRequestV1,
        proof: LocalCapabilityProofV1,
        now_micros: i64,
    ) -> Result<AdmittedLocalRpcRequestV1, LocalAdmissionError> {
        self.observe_time(now_micros)?;
        let record = self
            .challenges
            .remove(&proof.challenge)
            .ok_or(LocalAdmissionError::UnknownOrConsumedChallenge)?;

        if now_micros < record.issued_at_micros {
            return Err(LocalAdmissionError::ChallengeUsedBeforeIssue);
        }
        if now_micros >= record.expires_at_micros {
            return Err(LocalAdmissionError::ChallengeExpired);
        }

        let peer = linux_peer_credentials_v1(stream)?;
        if peer.uid != self.policy.allowed_uid {
            return Err(LocalAdmissionError::PeerUidDenied);
        }
        if peer.gid != self.policy.allowed_gid {
            return Err(LocalAdmissionError::PeerGidDenied);
        }

        verify_capability_mac_v1(
            &self.policy.capability,
            proof.challenge,
            peer,
            &request,
            proof.tag,
        )?;

        Ok(AdmittedLocalRpcRequestV1 {
            peer,
            request,
            challenge: proof.challenge,
        })
    }

    fn observe_time(&mut self, now_micros: i64) -> Result<(), LocalAdmissionError> {
        if now_micros < 0 {
            return Err(LocalAdmissionError::InvalidObservedTime);
        }
        if let Some(previous) = self.last_observed_micros {
            if now_micros < previous {
                return Err(LocalAdmissionError::ClockRollback);
            }
        }
        self.last_observed_micros = Some(now_micros);
        Ok(())
    }

    #[cfg(test)]
    fn issue_exact_for_test(
        &mut self,
        challenge: LocalAuthChallengeV1,
        now_micros: i64,
        lifetime_micros: i64,
    ) -> Result<IssuedLocalAuthChallengeV1, LocalAdmissionError> {
        self.observe_time(now_micros)?;
        if self.challenges.contains_key(&challenge) {
            return Err(LocalAdmissionError::ChallengeCollision);
        }
        if lifetime_micros <= 0 || lifetime_micros > self.policy.max_challenge_lifetime_micros {
            return Err(LocalAdmissionError::InvalidChallengeLifetime);
        }
        let expires_at_micros = now_micros
            .checked_add(lifetime_micros)
            .ok_or(LocalAdmissionError::TimeOverflow)?;
        self.challenges.insert(
            challenge,
            AuthChallengeRecordV1 {
                issued_at_micros: now_micros,
                expires_at_micros,
            },
        );
        Ok(IssuedLocalAuthChallengeV1 {
            challenge,
            issued_at_micros: now_micros,
            expires_at_micros,
        })
    }
}

pub fn capability_transcript_v1(
    challenge: LocalAuthChallengeV1,
    peer: LinuxPeerCredentialsV1,
    request: &daemon::DaemonRpcRequestV1,
) -> Result<Vec<u8>, LocalAdmissionError> {
    let request_bytes = request.canonical_bytes();
    let request_len = u32::try_from(request_bytes.len())
        .map_err(|_| LocalAdmissionError::RequestTooLargeForTranscript)?;
    let mut out = Vec::with_capacity(CAPABILITY_DOMAIN_V1.len() + 2 + 32 + 4 * 4 + request_bytes.len());
    out.extend_from_slice(CAPABILITY_DOMAIN_V1);
    out.extend_from_slice(&ADMISSION_SCHEMA_V1.to_be_bytes());
    out.extend_from_slice(challenge.as_bytes());
    out.extend_from_slice(&peer.pid.to_be_bytes());
    out.extend_from_slice(&peer.uid.to_be_bytes());
    out.extend_from_slice(&peer.gid.to_be_bytes());
    out.extend_from_slice(&request_len.to_be_bytes());
    out.extend_from_slice(&request_bytes);
    Ok(out)
}

pub fn compute_capability_mac_v1(
    capability: &LocalCapabilityKeyV1,
    challenge: LocalAuthChallengeV1,
    peer: LinuxPeerCredentialsV1,
    request: &daemon::DaemonRpcRequestV1,
) -> Result<[u8; CAPABILITY_TAG_LEN_V1], LocalAdmissionError> {
    let transcript = capability_transcript_v1(challenge, peer, request)?;
    let mut mac = HmacSha256::new_from_slice(capability.secret())
        .map_err(|_| LocalAdmissionError::InvalidCapabilityKey)?;
    mac.update(&transcript);
    Ok(mac.finalize().into_bytes().into())
}

fn verify_capability_mac_v1(
    capability: &LocalCapabilityKeyV1,
    challenge: LocalAuthChallengeV1,
    peer: LinuxPeerCredentialsV1,
    request: &daemon::DaemonRpcRequestV1,
    tag: [u8; CAPABILITY_TAG_LEN_V1],
) -> Result<(), LocalAdmissionError> {
    let transcript = capability_transcript_v1(challenge, peer, request)?;
    let mut mac = HmacSha256::new_from_slice(capability.secret())
        .map_err(|_| LocalAdmissionError::InvalidCapabilityKey)?;
    mac.update(&transcript);
    mac.verify_slice(&tag)
        .map_err(|_| LocalAdmissionError::CapabilityMacMismatch)
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LocalAdmissionError {
    #[error("local auth challenge must be nonzero")]
    ZeroAuthChallenge,
    #[error("local capability key must be nonzero")]
    ZeroCapabilityKey,
    #[error("local capability tag must be nonzero")]
    ZeroCapabilityTag,
    #[error("peer pid must be positive")]
    InvalidPeerPid,
    #[error("failed to read Linux peer credentials")]
    PeerCredentialLookupFailed,
    #[error("invalid auth-challenge lifetime")]
    InvalidChallengeLifetime,
    #[error("auth-challenge generation exhausted")]
    ChallengeGenerationExhausted,
    #[error("auth-challenge collision")]
    ChallengeCollision,
    #[error("unknown or already-consumed auth challenge")]
    UnknownOrConsumedChallenge,
    #[error("auth challenge used before issue time")]
    ChallengeUsedBeforeIssue,
    #[error("auth challenge expired")]
    ChallengeExpired,
    #[error("kernel peer uid is not allowed")]
    PeerUidDenied,
    #[error("kernel peer gid is not allowed")]
    PeerGidDenied,
    #[error("local capability MAC did not verify")]
    CapabilityMacMismatch,
    #[error("invalid HMAC capability key")]
    InvalidCapabilityKey,
    #[error("RPC request is too large for V1 admission transcript")]
    RequestTooLargeForTranscript,
    #[error("invalid observed time")]
    InvalidObservedTime,
    #[error("local admission clock rollback")]
    ClockRollback,
    #[error("time overflow")]
    TimeOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability(value: u8) -> LocalCapabilityKeyV1 {
        LocalCapabilityKeyV1::from_bytes([value; 32]).expect("capability")
    }

    fn auth_challenge(value: u8) -> LocalAuthChallengeV1 {
        LocalAuthChallengeV1::new([value; 32]).expect("challenge")
    }

    fn request(id_value: u8, payload: &[u8]) -> daemon::DaemonRpcRequestV1 {
        daemon::DaemonRpcRequestV1::new(
            daemon::RpcOperationV1::Mint,
            daemon::RpcRequestIdV1::new([id_value; 32]).expect("id"),
            payload.to_vec(),
        )
        .expect("request")
    }

    fn pair_and_peer() -> (UnixStream, UnixStream, LinuxPeerCredentialsV1) {
        let (client, server) = UnixStream::pair().expect("pair");
        let peer = linux_peer_credentials_v1(&server).expect("peer credentials");
        (client, server, peer)
    }

    fn gate(peer: LinuxPeerCredentialsV1, key_byte: u8) -> LocalAdmissionGateV1 {
        LocalAdmissionGateV1::new(
            LocalAdmissionPolicyV1::new(peer.uid(), peer.gid(), capability(key_byte), 1_000)
                .expect("policy"),
        )
    }

    #[test]
    fn exact_peer_challenge_request_and_mac_succeeds_once() {
        let (_client, server, peer) = pair_and_peer();
        let challenge = auth_challenge(7);
        let request = request(8, b"mint-body");
        let client_key = capability(9);
        let tag = compute_capability_mac_v1(&client_key, challenge, peer, &request).expect("mac");
        let proof = LocalCapabilityProofV1::new(challenge, tag).expect("proof");
        let mut gate = gate(peer, 9);
        gate.issue_exact_for_test(challenge, 1_000, 100).expect("issue");

        let admitted = gate
            .admit_stream(&server, request.clone(), proof, 1_010)
            .expect("admit");
        assert_eq!(admitted.peer(), peer);
        assert_eq!(admitted.request(), &request);

        assert_eq!(
            gate.admit_stream(&server, request, proof, 1_011),
            Err(LocalAdmissionError::UnknownOrConsumedChallenge)
        );
    }

    #[test]
    fn invalid_mac_burns_challenge() {
        let (_client, server, peer) = pair_and_peer();
        let challenge = auth_challenge(10);
        let request = request(11, b"payload");
        let wrong_key = capability(12);
        let tag = compute_capability_mac_v1(&wrong_key, challenge, peer, &request).expect("mac");
        let proof = LocalCapabilityProofV1::new(challenge, tag).expect("proof");
        let mut gate = gate(peer, 13);
        gate.issue_exact_for_test(challenge, 1_000, 100).expect("issue");

        assert_eq!(
            gate.admit_stream(&server, request.clone(), proof, 1_010),
            Err(LocalAdmissionError::CapabilityMacMismatch)
        );
        assert_eq!(
            gate.admit_stream(&server, request, proof, 1_011),
            Err(LocalAdmissionError::UnknownOrConsumedChallenge)
        );
    }

    #[test]
    fn wrong_uid_burns_challenge_before_mac() {
        let (_client, server, peer) = pair_and_peer();
        let challenge = auth_challenge(14);
        let request = request(15, b"payload");
        let client_key = capability(16);
        let tag = compute_capability_mac_v1(&client_key, challenge, peer, &request).expect("mac");
        let proof = LocalCapabilityProofV1::new(challenge, tag).expect("proof");
        let mut gate = LocalAdmissionGateV1::new(
            LocalAdmissionPolicyV1::new(
                peer.uid().wrapping_add(1),
                peer.gid(),
                capability(16),
                100,
            )
            .expect("policy"),
        );
        gate.issue_exact_for_test(challenge, 1_000, 100).expect("issue");

        assert_eq!(
            gate.admit_stream(&server, request.clone(), proof, 1_010),
            Err(LocalAdmissionError::PeerUidDenied)
        );
        assert_eq!(
            gate.admit_stream(&server, request, proof, 1_011),
            Err(LocalAdmissionError::UnknownOrConsumedChallenge)
        );
    }

    #[test]
    fn wrong_gid_burns_challenge_before_mac() {
        let (_client, server, peer) = pair_and_peer();
        let challenge = auth_challenge(17);
        let request = request(18, b"payload");
        let client_key = capability(19);
        let tag = compute_capability_mac_v1(&client_key, challenge, peer, &request).expect("mac");
        let proof = LocalCapabilityProofV1::new(challenge, tag).expect("proof");
        let mut gate = LocalAdmissionGateV1::new(
            LocalAdmissionPolicyV1::new(
                peer.uid(),
                peer.gid().wrapping_add(1),
                capability(19),
                100,
            )
            .expect("policy"),
        );
        gate.issue_exact_for_test(challenge, 1_000, 100).expect("issue");

        assert_eq!(
            gate.admit_stream(&server, request, proof, 1_010),
            Err(LocalAdmissionError::PeerGidDenied)
        );
    }

    #[test]
    fn request_mutations_and_peer_pid_change_mac_transcript() {
        let (_client, _server, peer) = pair_and_peer();
        let challenge = auth_challenge(20);
        let baseline = request(21, b"one");
        let changed_payload = request(21, b"two");
        let changed_id = request(22, b"one");
        let changed_peer = LinuxPeerCredentialsV1::new(peer.pid() + 1, peer.uid(), peer.gid())
            .expect("changed peer");
        let key = capability(23);

        let a = compute_capability_mac_v1(&key, challenge, peer, &baseline).expect("mac");
        let b = compute_capability_mac_v1(&key, challenge, peer, &changed_payload).expect("mac");
        let c = compute_capability_mac_v1(&key, challenge, peer, &changed_id).expect("mac");
        let d = compute_capability_mac_v1(&key, challenge, changed_peer, &baseline).expect("mac");
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn operation_mutation_changes_mac() {
        let (_client, _server, peer) = pair_and_peer();
        let challenge = auth_challenge(24);
        let key = capability(25);
        let id = daemon::RpcRequestIdV1::new([26; 32]).expect("id");
        let mint = daemon::DaemonRpcRequestV1::new(
            daemon::RpcOperationV1::Mint,
            id,
            vec![1],
        )
        .expect("mint");
        let status = daemon::DaemonRpcRequestV1::new(
            daemon::RpcOperationV1::Status,
            id,
            vec![],
        )
        .expect("status");
        let a = compute_capability_mac_v1(&key, challenge, peer, &mint).expect("mac");
        let b = compute_capability_mac_v1(&key, challenge, peer, &status).expect("mac");
        assert_ne!(a, b);
    }

    #[test]
    fn capability_debug_does_not_reveal_secret() {
        let key = capability(0x5a);
        let rendered = format!("{key:?}");
        assert!(rendered.contains("redacted"));
        assert!(!rendered.contains("5a"));
    }

    #[test]
    fn clock_rollback_denies_without_reusing_previous_time() {
        let (_client, _server, peer) = pair_and_peer();
        let mut gate = gate(peer, 27);
        gate.issue_exact_for_test(auth_challenge(28), 2_000, 100)
            .expect("issue");
        assert_eq!(
            gate.issue_challenge(1_999, 10),
            Err(LocalAdmissionError::ClockRollback)
        );
    }
}
