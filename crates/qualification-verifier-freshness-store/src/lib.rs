#![forbid(unsafe_code)]
//! Restart-durable freshness state for the isolated Health qualification verifier.
//!
//! V1 is intentionally Unix/Linux-only for its durability theorem because the
//! exact write protocol relies on `fsync(file) -> rename -> fsync(directory)`.
//! It detects malformed/corrupt/internally inconsistent local history, but it
//! does **not** detect restoration of an older complete, internally valid store.
//! Whole-volume rollback resistance therefore remains an external-anchor or
//! hardware-monotonic theorem.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use mycelix_qualification_verifier_mint_contract as contract;
use sha2::{Digest as _, Sha256};

const FILE_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-FRESHNESS-STORE-V1\0";
const EVENT_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-FRESHNESS-EVENT-V1\0";
const EVENT_DIGEST_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-FRESHNESS-EVENT-DIGEST-V1\0";
const FILE_SCHEMA_V1: u16 = 1;
const EVENT_SCHEMA_V1: u16 = 1;
const MAX_EVENT_BYTES: usize = 512;
const MAX_EVENT_COUNT: usize = 1_000_000;

const KIND_GENESIS: u8 = 1;
const KIND_ISSUE_CHALLENGE: u8 = 2;
const KIND_CONSUME_CHALLENGE: u8 = 3;
const KIND_ROTATE_POLICY: u8 = 4;
const KIND_ADVANCE_TIME: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveChallengeV1 {
    issued_at_micros: i64,
    expires_at_micros: i64,
}

impl ActiveChallengeV1 {
    pub fn issued_at_micros(&self) -> i64 {
        self.issued_at_micros
    }

    pub fn expires_at_micros(&self) -> i64 {
        self.expires_at_micros
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChallengeConsumptionStatusV1 {
    Fresh,
    Expired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FreshnessStoreHeadV1 {
    lineage_id: contract::Digest,
    sequence: u64,
    head_digest: contract::Digest,
    security_time_floor_micros: i64,
    policy_generation: u64,
    key_lineage_commitment: contract::Digest,
}

impl FreshnessStoreHeadV1 {
    pub fn lineage_id(&self) -> contract::Digest {
        self.lineage_id
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn head_digest(&self) -> contract::Digest {
        self.head_digest
    }

    pub fn security_time_floor_micros(&self) -> i64 {
        self.security_time_floor_micros
    }

    pub fn policy_generation(&self) -> u64 {
        self.policy_generation
    }

    pub fn key_lineage_commitment(&self) -> contract::Digest {
        self.key_lineage_commitment
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FreshnessEventKindV1 {
    Genesis {
        policy_generation: u64,
        key_lineage_commitment: contract::Digest,
    },
    IssueChallenge {
        nonce: contract::Nonce,
        expires_at_micros: i64,
    },
    ConsumeChallenge {
        nonce: contract::Nonce,
    },
    RotatePolicy {
        policy_generation: u64,
        key_lineage_commitment: contract::Digest,
    },
    AdvanceTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FreshnessEventV1 {
    lineage_id: contract::Digest,
    sequence: u64,
    previous_digest: Option<contract::Digest>,
    observed_at_micros: i64,
    kind: FreshnessEventKindV1,
}

impl FreshnessEventV1 {
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256);
        out.extend_from_slice(EVENT_DOMAIN_V1);
        out.extend_from_slice(&EVENT_SCHEMA_V1.to_be_bytes());
        out.extend_from_slice(self.lineage_id.as_bytes());
        out.extend_from_slice(&self.sequence.to_be_bytes());
        push_optional_digest(&mut out, self.previous_digest);
        out.extend_from_slice(&self.observed_at_micros.to_be_bytes());
        match self.kind {
            FreshnessEventKindV1::Genesis {
                policy_generation,
                key_lineage_commitment,
            } => {
                out.push(KIND_GENESIS);
                out.extend_from_slice(&policy_generation.to_be_bytes());
                out.extend_from_slice(key_lineage_commitment.as_bytes());
            }
            FreshnessEventKindV1::IssueChallenge {
                nonce,
                expires_at_micros,
            } => {
                out.push(KIND_ISSUE_CHALLENGE);
                out.extend_from_slice(nonce.as_bytes());
                out.extend_from_slice(&expires_at_micros.to_be_bytes());
            }
            FreshnessEventKindV1::ConsumeChallenge { nonce } => {
                out.push(KIND_CONSUME_CHALLENGE);
                out.extend_from_slice(nonce.as_bytes());
            }
            FreshnessEventKindV1::RotatePolicy {
                policy_generation,
                key_lineage_commitment,
            } => {
                out.push(KIND_ROTATE_POLICY);
                out.extend_from_slice(&policy_generation.to_be_bytes());
                out.extend_from_slice(key_lineage_commitment.as_bytes());
            }
            FreshnessEventKindV1::AdvanceTime => out.push(KIND_ADVANCE_TIME),
        }
        out
    }

    fn digest(&self) -> Result<contract::Digest, FreshnessStoreError> {
        let mut hasher = Sha256::new();
        hasher.update(EVENT_DIGEST_DOMAIN_V1);
        hasher.update(self.canonical_bytes());
        digest_from_sha256(hasher.finalize().into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DerivedFreshnessStateV1 {
    lineage_id: contract::Digest,
    sequence: u64,
    head_digest: contract::Digest,
    security_time_floor_micros: i64,
    policy_generation: u64,
    key_lineage_commitment: contract::Digest,
    active: BTreeMap<contract::Nonce, ActiveChallengeV1>,
    consumed: BTreeSet<contract::Nonce>,
}

impl DerivedFreshnessStateV1 {
    fn head(&self) -> FreshnessStoreHeadV1 {
        FreshnessStoreHeadV1 {
            lineage_id: self.lineage_id,
            sequence: self.sequence,
            head_digest: self.head_digest,
            security_time_floor_micros: self.security_time_floor_micros,
            policy_generation: self.policy_generation,
            key_lineage_commitment: self.key_lineage_commitment,
        }
    }
}

pub struct DurableFreshnessStoreV1 {
    path: PathBuf,
    events: Vec<FreshnessEventV1>,
    state: DerivedFreshnessStateV1,
}

impl DurableFreshnessStoreV1 {
    pub fn create(
        path: impl Into<PathBuf>,
        lineage_id: contract::Digest,
        observed_at_micros: i64,
        policy_generation: u64,
        key_lineage_commitment: contract::Digest,
    ) -> Result<Self, FreshnessStoreError> {
        require_supported_platform()?;
        if observed_at_micros < 0 {
            return Err(FreshnessStoreError::InvalidObservedTime);
        }
        if policy_generation == 0 {
            return Err(FreshnessStoreError::InvalidPolicyGeneration);
        }
        let path = path.into();
        if path.exists() {
            return Err(FreshnessStoreError::AlreadyExists);
        }
        let genesis = FreshnessEventV1 {
            lineage_id,
            sequence: 1,
            previous_digest: None,
            observed_at_micros,
            kind: FreshnessEventKindV1::Genesis {
                policy_generation,
                key_lineage_commitment,
            },
        };
        let events = vec![genesis];
        let state = replay_events(&events)?;
        persist_events_atomic(&path, lineage_id, &events)?;
        Ok(Self { path, events, state })
    }

    pub fn load(path: impl Into<PathBuf>) -> Result<Self, FreshnessStoreError> {
        require_supported_platform()?;
        let path = path.into();
        let mut bytes = Vec::new();
        File::open(&path)?.read_to_end(&mut bytes)?;
        let (lineage_id, events) = decode_store_bytes(&bytes)?;
        let state = replay_events(&events)?;
        if state.lineage_id != lineage_id {
            return Err(FreshnessStoreError::LineageMismatch);
        }
        Ok(Self { path, events, state })
    }

    pub fn head(&self) -> FreshnessStoreHeadV1 {
        self.state.head()
    }

    pub fn is_challenge_active(&self, nonce: contract::Nonce) -> bool {
        self.state.active.contains_key(&nonce)
    }

    pub fn is_challenge_consumed(&self, nonce: contract::Nonce) -> bool {
        self.state.consumed.contains(&nonce)
    }

    pub fn active_challenge(&self, nonce: contract::Nonce) -> Option<ActiveChallengeV1> {
        self.state.active.get(&nonce).copied()
    }

    pub fn issue_challenge(
        &mut self,
        nonce: contract::Nonce,
        observed_at_micros: i64,
        expires_at_micros: i64,
    ) -> Result<FreshnessStoreHeadV1, FreshnessStoreError> {
        if self.state.active.contains_key(&nonce) {
            return Err(FreshnessStoreError::ChallengeAlreadyActive);
        }
        if self.state.consumed.contains(&nonce) {
            return Err(FreshnessStoreError::ChallengePreviouslyConsumed);
        }
        if observed_at_micros < self.state.security_time_floor_micros {
            return Err(FreshnessStoreError::TimeRollback);
        }
        if expires_at_micros <= observed_at_micros {
            return Err(FreshnessStoreError::InvalidChallengeWindow);
        }
        let event = self.next_event(
            observed_at_micros,
            FreshnessEventKindV1::IssueChallenge {
                nonce,
                expires_at_micros,
            },
        )?;
        self.commit_event(event)?;
        Ok(self.head())
    }

    pub fn consume_challenge(
        &mut self,
        nonce: contract::Nonce,
        observed_at_micros: i64,
    ) -> Result<ChallengeConsumptionStatusV1, FreshnessStoreError> {
        if self.state.consumed.contains(&nonce) {
            return Err(FreshnessStoreError::ChallengeReplay);
        }
        let record = self
            .state
            .active
            .get(&nonce)
            .copied()
            .ok_or(FreshnessStoreError::UnknownChallenge)?;
        if observed_at_micros < self.state.security_time_floor_micros {
            return Err(FreshnessStoreError::TimeRollback);
        }
        let status = if observed_at_micros < record.expires_at_micros {
            ChallengeConsumptionStatusV1::Fresh
        } else {
            ChallengeConsumptionStatusV1::Expired
        };
        let event = self.next_event(
            observed_at_micros,
            FreshnessEventKindV1::ConsumeChallenge { nonce },
        )?;
        // The consume event is made durable before the caller sees Fresh/Expired.
        self.commit_event(event)?;
        Ok(status)
    }

    pub fn rotate_policy(
        &mut self,
        observed_at_micros: i64,
        new_policy_generation: u64,
        new_key_lineage_commitment: contract::Digest,
    ) -> Result<FreshnessStoreHeadV1, FreshnessStoreError> {
        if observed_at_micros < self.state.security_time_floor_micros {
            return Err(FreshnessStoreError::TimeRollback);
        }
        let expected_generation = self
            .state
            .policy_generation
            .checked_add(1)
            .ok_or(FreshnessStoreError::SequenceOverflow)?;
        if new_policy_generation != expected_generation {
            return Err(FreshnessStoreError::PolicyGenerationNotContiguous);
        }
        if new_key_lineage_commitment == self.state.key_lineage_commitment {
            return Err(FreshnessStoreError::PolicyRotationDidNotChangeKeyLineage);
        }
        let event = self.next_event(
            observed_at_micros,
            FreshnessEventKindV1::RotatePolicy {
                policy_generation: new_policy_generation,
                key_lineage_commitment: new_key_lineage_commitment,
            },
        )?;
        self.commit_event(event)?;
        Ok(self.head())
    }

    pub fn advance_time(
        &mut self,
        observed_at_micros: i64,
    ) -> Result<FreshnessStoreHeadV1, FreshnessStoreError> {
        if observed_at_micros < self.state.security_time_floor_micros {
            return Err(FreshnessStoreError::TimeRollback);
        }
        if observed_at_micros == self.state.security_time_floor_micros {
            return Ok(self.head());
        }
        let event = self.next_event(observed_at_micros, FreshnessEventKindV1::AdvanceTime)?;
        self.commit_event(event)?;
        Ok(self.head())
    }

    fn next_event(
        &self,
        observed_at_micros: i64,
        kind: FreshnessEventKindV1,
    ) -> Result<FreshnessEventV1, FreshnessStoreError> {
        let sequence = self
            .state
            .sequence
            .checked_add(1)
            .ok_or(FreshnessStoreError::SequenceOverflow)?;
        Ok(FreshnessEventV1 {
            lineage_id: self.state.lineage_id,
            sequence,
            previous_digest: Some(self.state.head_digest),
            observed_at_micros,
            kind,
        })
    }

    fn commit_event(&mut self, event: FreshnessEventV1) -> Result<(), FreshnessStoreError> {
        let mut candidate_events = self.events.clone();
        candidate_events.push(event);
        let candidate_state = replay_events(&candidate_events)?;
        persist_events_atomic(&self.path, self.state.lineage_id, &candidate_events)?;
        self.events = candidate_events;
        self.state = candidate_state;
        Ok(())
    }

    #[cfg(test)]
    fn write_candidate_temp_only_for_test(
        &self,
        event: FreshnessEventV1,
    ) -> Result<PathBuf, FreshnessStoreError> {
        let mut candidate = self.events.clone();
        candidate.push(event);
        let _ = replay_events(&candidate)?;
        let bytes = encode_store_bytes(self.state.lineage_id, &candidate)?;
        let temp = temp_path_for(&self.path, candidate.len() as u64);
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        Ok(temp)
    }
}

fn replay_events(events: &[FreshnessEventV1]) -> Result<DerivedFreshnessStateV1, FreshnessStoreError> {
    if events.is_empty() {
        return Err(FreshnessStoreError::EmptyHistory);
    }
    if events.len() > MAX_EVENT_COUNT {
        return Err(FreshnessStoreError::TooManyEvents);
    }

    let first = events[0];
    if first.sequence != 1 || first.previous_digest.is_some() {
        return Err(FreshnessStoreError::InvalidGenesisShape);
    }
    let FreshnessEventKindV1::Genesis {
        policy_generation,
        key_lineage_commitment,
    } = first.kind
    else {
        return Err(FreshnessStoreError::InvalidGenesisShape);
    };
    if first.observed_at_micros < 0 {
        return Err(FreshnessStoreError::InvalidObservedTime);
    }
    if policy_generation == 0 {
        return Err(FreshnessStoreError::InvalidPolicyGeneration);
    }
    let first_digest = first.digest()?;
    let mut state = DerivedFreshnessStateV1 {
        lineage_id: first.lineage_id,
        sequence: 1,
        head_digest: first_digest,
        security_time_floor_micros: first.observed_at_micros,
        policy_generation,
        key_lineage_commitment,
        active: BTreeMap::new(),
        consumed: BTreeSet::new(),
    };

    for event in &events[1..] {
        if event.lineage_id != state.lineage_id {
            return Err(FreshnessStoreError::LineageMismatch);
        }
        let expected_sequence = state
            .sequence
            .checked_add(1)
            .ok_or(FreshnessStoreError::SequenceOverflow)?;
        if event.sequence != expected_sequence {
            return Err(FreshnessStoreError::SequenceNotContiguous);
        }
        if event.previous_digest != Some(state.head_digest) {
            return Err(FreshnessStoreError::PreviousDigestMismatch);
        }
        if event.observed_at_micros < state.security_time_floor_micros {
            return Err(FreshnessStoreError::TimeRollback);
        }
        match event.kind {
            FreshnessEventKindV1::Genesis { .. } => {
                return Err(FreshnessStoreError::UnexpectedGenesis)
            }
            FreshnessEventKindV1::IssueChallenge {
                nonce,
                expires_at_micros,
            } => {
                if expires_at_micros <= event.observed_at_micros {
                    return Err(FreshnessStoreError::InvalidChallengeWindow);
                }
                if state.active.contains_key(&nonce) {
                    return Err(FreshnessStoreError::ChallengeAlreadyActive);
                }
                if state.consumed.contains(&nonce) {
                    return Err(FreshnessStoreError::ChallengePreviouslyConsumed);
                }
                state.active.insert(
                    nonce,
                    ActiveChallengeV1 {
                        issued_at_micros: event.observed_at_micros,
                        expires_at_micros,
                    },
                );
            }
            FreshnessEventKindV1::ConsumeChallenge { nonce } => {
                if state.consumed.contains(&nonce) {
                    return Err(FreshnessStoreError::ChallengeReplay);
                }
                if state.active.remove(&nonce).is_none() {
                    return Err(FreshnessStoreError::UnknownChallenge);
                }
                state.consumed.insert(nonce);
            }
            FreshnessEventKindV1::RotatePolicy {
                policy_generation,
                key_lineage_commitment,
            } => {
                let expected = state
                    .policy_generation
                    .checked_add(1)
                    .ok_or(FreshnessStoreError::SequenceOverflow)?;
                if policy_generation != expected {
                    return Err(FreshnessStoreError::PolicyGenerationNotContiguous);
                }
                if key_lineage_commitment == state.key_lineage_commitment {
                    return Err(FreshnessStoreError::PolicyRotationDidNotChangeKeyLineage);
                }
                state.policy_generation = policy_generation;
                state.key_lineage_commitment = key_lineage_commitment;
            }
            FreshnessEventKindV1::AdvanceTime => {}
        }
        state.security_time_floor_micros = event.observed_at_micros;
        state.sequence = event.sequence;
        state.head_digest = event.digest()?;
    }
    Ok(state)
}

fn encode_store_bytes(
    lineage_id: contract::Digest,
    events: &[FreshnessEventV1],
) -> Result<Vec<u8>, FreshnessStoreError> {
    if events.is_empty() || events.len() > MAX_EVENT_COUNT {
        return Err(FreshnessStoreError::InvalidEventCount);
    }
    let mut out = Vec::new();
    out.extend_from_slice(FILE_DOMAIN_V1);
    out.extend_from_slice(&FILE_SCHEMA_V1.to_be_bytes());
    out.extend_from_slice(lineage_id.as_bytes());
    out.extend_from_slice(&(events.len() as u64).to_be_bytes());
    for event in events {
        let bytes = event.canonical_bytes();
        if bytes.len() > MAX_EVENT_BYTES {
            return Err(FreshnessStoreError::EventTooLarge);
        }
        let length = u32::try_from(bytes.len()).map_err(|_| FreshnessStoreError::EventTooLarge)?;
        out.extend_from_slice(&length.to_be_bytes());
        out.extend_from_slice(&bytes);
        out.extend_from_slice(event.digest()?.as_bytes());
    }
    Ok(out)
}

fn decode_store_bytes(
    bytes: &[u8],
) -> Result<(contract::Digest, Vec<FreshnessEventV1>), FreshnessStoreError> {
    let mut reader = ByteReader::new(bytes);
    reader.expect_bytes(FILE_DOMAIN_V1)?;
    let schema = reader.read_u16()?;
    if schema != FILE_SCHEMA_V1 {
        return Err(FreshnessStoreError::UnsupportedSchema);
    }
    let lineage_id = reader.read_digest()?;
    let event_count_u64 = reader.read_u64()?;
    let event_count = usize::try_from(event_count_u64).map_err(|_| FreshnessStoreError::TooManyEvents)?;
    if event_count == 0 || event_count > MAX_EVENT_COUNT {
        return Err(FreshnessStoreError::InvalidEventCount);
    }
    let mut events = Vec::with_capacity(event_count);
    for _ in 0..event_count {
        let event_len = usize::try_from(reader.read_u32()?).map_err(|_| FreshnessStoreError::EventTooLarge)?;
        if event_len == 0 || event_len > MAX_EVENT_BYTES {
            return Err(FreshnessStoreError::EventTooLarge);
        }
        let event_bytes = reader.read_exact(event_len)?;
        let stored_digest = reader.read_digest()?;
        let event = decode_event_bytes(event_bytes)?;
        if event.lineage_id != lineage_id {
            return Err(FreshnessStoreError::LineageMismatch);
        }
        if event.digest()? != stored_digest {
            return Err(FreshnessStoreError::EventDigestMismatch);
        }
        events.push(event);
    }
    if !reader.is_finished() {
        return Err(FreshnessStoreError::TrailingBytes);
    }
    Ok((lineage_id, events))
}

fn decode_event_bytes(bytes: &[u8]) -> Result<FreshnessEventV1, FreshnessStoreError> {
    let mut reader = ByteReader::new(bytes);
    reader.expect_bytes(EVENT_DOMAIN_V1)?;
    if reader.read_u16()? != EVENT_SCHEMA_V1 {
        return Err(FreshnessStoreError::UnsupportedSchema);
    }
    let lineage_id = reader.read_digest()?;
    let sequence = reader.read_u64()?;
    let previous_digest = reader.read_optional_digest()?;
    let observed_at_micros = reader.read_i64()?;
    let kind_tag = reader.read_u8()?;
    let kind = match kind_tag {
        KIND_GENESIS => FreshnessEventKindV1::Genesis {
            policy_generation: reader.read_u64()?,
            key_lineage_commitment: reader.read_digest()?,
        },
        KIND_ISSUE_CHALLENGE => FreshnessEventKindV1::IssueChallenge {
            nonce: reader.read_nonce()?,
            expires_at_micros: reader.read_i64()?,
        },
        KIND_CONSUME_CHALLENGE => FreshnessEventKindV1::ConsumeChallenge {
            nonce: reader.read_nonce()?,
        },
        KIND_ROTATE_POLICY => FreshnessEventKindV1::RotatePolicy {
            policy_generation: reader.read_u64()?,
            key_lineage_commitment: reader.read_digest()?,
        },
        KIND_ADVANCE_TIME => FreshnessEventKindV1::AdvanceTime,
        _ => return Err(FreshnessStoreError::UnknownEventKind),
    };
    if !reader.is_finished() {
        return Err(FreshnessStoreError::TrailingEventBytes);
    }
    Ok(FreshnessEventV1 {
        lineage_id,
        sequence,
        previous_digest,
        observed_at_micros,
        kind,
    })
}

#[cfg(unix)]
fn persist_events_atomic(
    path: &Path,
    lineage_id: contract::Digest,
    events: &[FreshnessEventV1],
) -> Result<(), FreshnessStoreError> {
    let bytes = encode_store_bytes(lineage_id, events)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let sequence = events
        .last()
        .ok_or(FreshnessStoreError::EmptyHistory)?
        .sequence;
    let temp = temp_path_for(path, sequence);
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temp)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    fs::rename(&temp, path)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn persist_events_atomic(
    _path: &Path,
    _lineage_id: contract::Digest,
    _events: &[FreshnessEventV1],
) -> Result<(), FreshnessStoreError> {
    Err(FreshnessStoreError::UnsupportedPlatform)
}

fn temp_path_for(path: &Path, sequence: u64) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("freshness-store");
    path.with_file_name(format!(".{file_name}.tmp.{sequence}"))
}

fn require_supported_platform() -> Result<(), FreshnessStoreError> {
    #[cfg(unix)]
    {
        Ok(())
    }
    #[cfg(not(unix))]
    {
        Err(FreshnessStoreError::UnsupportedPlatform)
    }
}

fn push_optional_digest(out: &mut Vec<u8>, value: Option<contract::Digest>) {
    match value {
        None => out.push(0),
        Some(value) => {
            out.push(1);
            out.extend_from_slice(value.as_bytes());
        }
    }
}

fn digest_from_sha256(bytes: [u8; 32]) -> Result<contract::Digest, FreshnessStoreError> {
    contract::Digest::new(bytes).map_err(|_| FreshnessStoreError::ImpossibleZeroDigest)
}

struct ByteReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn is_finished(&self) -> bool {
        self.pos == self.bytes.len()
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], FreshnessStoreError> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or(FreshnessStoreError::TruncatedInput)?;
        if end > self.bytes.len() {
            return Err(FreshnessStoreError::TruncatedInput);
        }
        let out = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn expect_bytes(&mut self, expected: &[u8]) -> Result<(), FreshnessStoreError> {
        if self.read_exact(expected.len())? != expected {
            return Err(FreshnessStoreError::DomainMismatch);
        }
        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8, FreshnessStoreError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, FreshnessStoreError> {
        let bytes: [u8; 2] = self
            .read_exact(2)?
            .try_into()
            .map_err(|_| FreshnessStoreError::TruncatedInput)?;
        Ok(u16::from_be_bytes(bytes))
    }

    fn read_u32(&mut self) -> Result<u32, FreshnessStoreError> {
        let bytes: [u8; 4] = self
            .read_exact(4)?
            .try_into()
            .map_err(|_| FreshnessStoreError::TruncatedInput)?;
        Ok(u32::from_be_bytes(bytes))
    }

    fn read_u64(&mut self) -> Result<u64, FreshnessStoreError> {
        let bytes: [u8; 8] = self
            .read_exact(8)?
            .try_into()
            .map_err(|_| FreshnessStoreError::TruncatedInput)?;
        Ok(u64::from_be_bytes(bytes))
    }

    fn read_i64(&mut self) -> Result<i64, FreshnessStoreError> {
        let bytes: [u8; 8] = self
            .read_exact(8)?
            .try_into()
            .map_err(|_| FreshnessStoreError::TruncatedInput)?;
        Ok(i64::from_be_bytes(bytes))
    }

    fn read_digest(&mut self) -> Result<contract::Digest, FreshnessStoreError> {
        let bytes: [u8; 32] = self
            .read_exact(32)?
            .try_into()
            .map_err(|_| FreshnessStoreError::TruncatedInput)?;
        contract::Digest::new(bytes).map_err(|_| FreshnessStoreError::ZeroDigest)
    }

    fn read_nonce(&mut self) -> Result<contract::Nonce, FreshnessStoreError> {
        let bytes: [u8; 32] = self
            .read_exact(32)?
            .try_into()
            .map_err(|_| FreshnessStoreError::TruncatedInput)?;
        contract::Nonce::new(bytes).map_err(|_| FreshnessStoreError::ZeroNonce)
    }

    fn read_optional_digest(
        &mut self,
    ) -> Result<Option<contract::Digest>, FreshnessStoreError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_digest()?)),
            _ => Err(FreshnessStoreError::InvalidOptionalTag),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FreshnessStoreError {
    #[error("freshness store is supported only on Unix-like platforms in v1")]
    UnsupportedPlatform,
    #[error("freshness store already exists")]
    AlreadyExists,
    #[error("freshness history is empty")]
    EmptyHistory,
    #[error("too many freshness events")]
    TooManyEvents,
    #[error("invalid event count")]
    InvalidEventCount,
    #[error("event exceeds v1 maximum size")]
    EventTooLarge,
    #[error("unsupported freshness-store schema")]
    UnsupportedSchema,
    #[error("freshness domain mismatch")]
    DomainMismatch,
    #[error("freshness lineage mismatch")]
    LineageMismatch,
    #[error("invalid genesis shape")]
    InvalidGenesisShape,
    #[error("unexpected second genesis event")]
    UnexpectedGenesis,
    #[error("event sequence is not contiguous")]
    SequenceNotContiguous,
    #[error("event predecessor digest does not equal current head")]
    PreviousDigestMismatch,
    #[error("event digest mismatch")]
    EventDigestMismatch,
    #[error("unknown event kind")]
    UnknownEventKind,
    #[error("invalid observed time")]
    InvalidObservedTime,
    #[error("security-time floor rollback")]
    TimeRollback,
    #[error("invalid verifier policy generation")]
    InvalidPolicyGeneration,
    #[error("verifier policy generation is not contiguous")]
    PolicyGenerationNotContiguous,
    #[error("policy rotation did not change key lineage")]
    PolicyRotationDidNotChangeKeyLineage,
    #[error("challenge already active")]
    ChallengeAlreadyActive,
    #[error("challenge was previously consumed")]
    ChallengePreviouslyConsumed,
    #[error("challenge replay")]
    ChallengeReplay,
    #[error("unknown challenge")]
    UnknownChallenge,
    #[error("invalid challenge window")]
    InvalidChallengeWindow,
    #[error("sequence overflow")]
    SequenceOverflow,
    #[error("truncated input")]
    TruncatedInput,
    #[error("unexpected trailing store bytes")]
    TrailingBytes,
    #[error("unexpected trailing event bytes")]
    TrailingEventBytes,
    #[error("invalid optional digest tag")]
    InvalidOptionalTag,
    #[error("all-zero digest is forbidden")]
    ZeroDigest,
    #[error("all-zero nonce is forbidden")]
    ZeroNonce,
    #[error("derived SHA-256 digest was unexpectedly zero")]
    ImpossibleZeroDigest,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(1);

    fn d(value: u8) -> contract::Digest {
        contract::Digest::new([value; 32]).expect("nonzero digest")
    }

    fn n(value: u8) -> contract::Nonce {
        contract::Nonce::new([value; 32]).expect("nonzero nonce")
    }

    fn test_path(name: &str) -> PathBuf {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "mycelix-health-freshness-{name}-{}-{id}.bin",
            std::process::id()
        ))
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_file(path);
        if let Some(parent) = path.parent() {
            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                for sequence in 1..16 {
                    let _ = fs::remove_file(parent.join(format!(".{file_name}.tmp.{sequence}")));
                }
            }
        }
    }

    fn create_store(path: &Path) -> DurableFreshnessStoreV1 {
        DurableFreshnessStoreV1::create(path, d(1), 1_000, 1, d(2)).expect("create")
    }

    #[test]
    fn consume_persist_restart_replay_is_denied() {
        let path = test_path("consume-restart");
        cleanup(&path);
        let mut store = create_store(&path);
        store.issue_challenge(n(3), 1_010, 1_100).expect("issue");
        assert_eq!(
            store.consume_challenge(n(3), 1_020).expect("consume"),
            ChallengeConsumptionStatusV1::Fresh
        );
        drop(store);

        let mut loaded = DurableFreshnessStoreV1::load(&path).expect("reload");
        assert!(loaded.is_challenge_consumed(n(3)));
        assert!(matches!(
            loaded.consume_challenge(n(3), 1_030),
            Err(FreshnessStoreError::ChallengeReplay)
        ));
        cleanup(&path);
    }

    #[test]
    fn expired_challenge_is_durably_consumed() {
        let path = test_path("expired");
        cleanup(&path);
        let mut store = create_store(&path);
        store.issue_challenge(n(3), 1_010, 1_020).expect("issue");
        assert_eq!(
            store.consume_challenge(n(3), 1_021).expect("consume expired"),
            ChallengeConsumptionStatusV1::Expired
        );
        drop(store);
        let loaded = DurableFreshnessStoreV1::load(&path).expect("reload");
        assert!(loaded.is_challenge_consumed(n(3)));
        cleanup(&path);
    }

    #[test]
    fn time_floor_survives_restart_and_cannot_regress() {
        let path = test_path("time-floor");
        cleanup(&path);
        let mut store = create_store(&path);
        store.advance_time(1_500).expect("advance");
        drop(store);
        let mut loaded = DurableFreshnessStoreV1::load(&path).expect("reload");
        assert_eq!(loaded.head().security_time_floor_micros(), 1_500);
        assert!(matches!(
            loaded.issue_challenge(n(3), 1_499, 1_600),
            Err(FreshnessStoreError::TimeRollback)
        ));
        cleanup(&path);
    }

    #[test]
    fn policy_generation_and_key_lineage_are_forward_only() {
        let path = test_path("policy");
        cleanup(&path);
        let mut store = create_store(&path);
        store.rotate_policy(1_100, 2, d(4)).expect("rotate");
        assert_eq!(store.head().policy_generation(), 2);
        assert_eq!(store.head().key_lineage_commitment(), d(4));
        assert!(matches!(
            store.rotate_policy(1_200, 2, d(5)),
            Err(FreshnessStoreError::PolicyGenerationNotContiguous)
        ));
        assert!(matches!(
            store.rotate_policy(1_200, 3, d(4)),
            Err(FreshnessStoreError::PolicyRotationDidNotChangeKeyLineage)
        ));
        cleanup(&path);
    }

    #[test]
    fn stale_temp_file_does_not_replace_last_durable_state() {
        let path = test_path("temp-only");
        cleanup(&path);
        let store = create_store(&path);
        let event = store
            .next_event(
                1_010,
                FreshnessEventKindV1::IssueChallenge {
                    nonce: n(3),
                    expires_at_micros: 1_100,
                },
            )
            .expect("event");
        let temp = store
            .write_candidate_temp_only_for_test(event)
            .expect("temp write");
        assert!(temp.exists());
        drop(store);

        let loaded = DurableFreshnessStoreV1::load(&path).expect("reload durable main file");
        assert!(!loaded.is_challenge_active(n(3)));
        let _ = fs::remove_file(temp);
        cleanup(&path);
    }

    #[test]
    fn corrupted_historical_event_is_rejected() {
        let path = test_path("corruption");
        cleanup(&path);
        let mut store = create_store(&path);
        store.issue_challenge(n(3), 1_010, 1_100).expect("issue");
        drop(store);

        let mut bytes = fs::read(&path).expect("read");
        let index = bytes.len() / 2;
        bytes[index] ^= 0x01;
        fs::write(&path, bytes).expect("corrupt");
        assert!(DurableFreshnessStoreV1::load(&path).is_err());
        cleanup(&path);
    }

    #[test]
    fn wrong_predecessor_with_recomputed_digest_is_rejected() {
        let path = test_path("wrong-parent");
        cleanup(&path);
        let store = create_store(&path);
        let bad = FreshnessEventV1 {
            lineage_id: store.state.lineage_id,
            sequence: 2,
            previous_digest: Some(d(99)),
            observed_at_micros: 1_010,
            kind: FreshnessEventKindV1::AdvanceTime,
        };
        let events = vec![store.events[0], bad];
        let bytes = encode_store_bytes(store.state.lineage_id, &events).expect("encode");
        fs::write(&path, bytes).expect("write forged internally hashed history");
        assert!(matches!(
            DurableFreshnessStoreV1::load(&path),
            Err(FreshnessStoreError::PreviousDigestMismatch)
        ));
        cleanup(&path);
    }

    #[test]
    fn unknown_schema_is_rejected() {
        let path = test_path("schema");
        cleanup(&path);
        let store = create_store(&path);
        drop(store);
        let mut bytes = fs::read(&path).expect("read");
        let offset = FILE_DOMAIN_V1.len();
        bytes[offset..offset + 2].copy_from_slice(&2u16.to_be_bytes());
        fs::write(&path, bytes).expect("write");
        assert!(matches!(
            DurableFreshnessStoreV1::load(&path),
            Err(FreshnessStoreError::UnsupportedSchema)
        ));
        cleanup(&path);
    }

    #[test]
    fn exact_restart_is_idempotent() {
        let path = test_path("idempotent");
        cleanup(&path);
        let mut store = create_store(&path);
        store.issue_challenge(n(3), 1_010, 1_100).expect("issue");
        let expected = store.head();
        drop(store);
        let loaded = DurableFreshnessStoreV1::load(&path).expect("reload");
        assert_eq!(loaded.head(), expected);
        assert!(loaded.is_challenge_active(n(3)));
        cleanup(&path);
    }

    #[test]
    fn whole_volume_rollback_is_an_explicit_nonclaim() {
        let path = test_path("rollback-nonclaim");
        cleanup(&path);
        let mut store = create_store(&path);
        store.issue_challenge(n(3), 1_010, 1_100).expect("issue");
        let older_valid_snapshot = fs::read(&path).expect("snapshot");
        store.consume_challenge(n(3), 1_020).expect("consume");
        assert!(store.is_challenge_consumed(n(3)));
        drop(store);

        // Simulate an attacker restoring the complete older valid storage
        // volume. The local file is internally authentic to its own history,
        // so V1 cannot know that a newer state once existed.
        fs::write(&path, older_valid_snapshot).expect("restore old volume snapshot");
        let loaded = DurableFreshnessStoreV1::load(&path).expect("old snapshot is internally valid");
        assert!(loaded.is_challenge_active(n(3)));
        assert!(!loaded.is_challenge_consumed(n(3)));
        cleanup(&path);
    }
}
