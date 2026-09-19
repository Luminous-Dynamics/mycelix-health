#![forbid(unsafe_code)]
//! Fail-closed audit-chain reference semantics.
//!
//! Identity theorem:
//! `chain identity = (subject context, author, author-scoped chain id)`.
//!
//! Each subject checkpoint contains exactly one verified head per author. A
//! `ChainId` is not assumed globally unique across authors.

use core::fmt;

pub const AUDIT_CHAIN_CONTRACT_V1: u16 = 1;
const ENTRY_TRANSCRIPT_DOMAIN: &[u8] = b"mycelix-health/audit-chain-entry/v1\0";
const CHECKPOINT_TRANSCRIPT_DOMAIN: &[u8] = b"mycelix-health/audit-checkpoint/v1\0";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectContext([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthorId([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChainId([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryId([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventDigest([u8; 32]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdError {
    AllZero,
}

macro_rules! opaque32 {
    ($ty:ident) => {
        impl $ty {
            pub fn new(bytes: [u8; 32]) -> Result<Self, IdError> {
                if bytes == [0u8; 32] {
                    return Err(IdError::AllZero);
                }
                Ok(Self(bytes))
            }

            fn bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl fmt::Debug for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($ty), "([redacted])"))
            }
        }
    };
}

opaque32!(SubjectContext);
opaque32!(AuthorId);
opaque32!(ChainId);
opaque32!(EntryId);
opaque32!(EventDigest);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainIdentity {
    pub subject: SubjectContext,
    pub author: AuthorId,
    pub chain_id: ChainId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainHead {
    pub identity: ChainIdentity,
    pub sequence: u64,
    pub entry_id: EntryId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppendCandidate {
    pub identity: ChainIdentity,
    pub sequence: u64,
    pub predecessor: Option<EntryId>,
    pub event_digest: EventDigest,
    pub occurred_at_micros: i64,
    /// Commitment supplied by a separately qualified digest/signature adapter.
    /// This structural crate does not verify that commitment cryptographically.
    pub entry_id: EntryId,
}

impl AppendCandidate {
    /// Canonical serializer-independent transcript for an external digest/signature adapter.
    pub fn transcript_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(ENTRY_TRANSCRIPT_DOMAIN.len() + 2 + 32 * 5 + 8 + 1 + 8);
        out.extend_from_slice(ENTRY_TRANSCRIPT_DOMAIN);
        out.extend_from_slice(&AUDIT_CHAIN_CONTRACT_V1.to_be_bytes());
        out.extend_from_slice(self.identity.subject.bytes());
        out.extend_from_slice(self.identity.author.bytes());
        out.extend_from_slice(self.identity.chain_id.bytes());
        out.extend_from_slice(&self.sequence.to_be_bytes());
        match self.predecessor {
            Some(predecessor) => {
                out.push(1);
                out.extend_from_slice(predecessor.bytes());
            }
            None => {
                out.push(0);
                out.extend_from_slice(&[0u8; 32]);
            }
        }
        out.extend_from_slice(self.event_digest.bytes());
        out.extend_from_slice(&self.occurred_at_micros.to_be_bytes());
        out
    }

    pub fn head(&self) -> ChainHead {
        ChainHead {
            identity: self.identity,
            sequence: self.sequence,
            entry_id: self.entry_id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnavailableReason {
    Network,
    Authorization,
    Authentication,
    Countersigning,
    DependencyUnavailable,
    PartialResult,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidStateReason {
    Decode,
    MalformedEntry,
    UnsupportedVersion,
    AmbiguousPredecessor,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviousState {
    /// Positive evidence that this exact chain identity has no prior entry.
    KnownAbsent(ChainIdentity),
    KnownHead(ChainHead),
    Unavailable(UnavailableReason),
    Invalid(InvalidStateReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppendError {
    PreviousStateUnavailable,
    PreviousStateInvalid,
    IdentityMismatch,
    GenesisMustBeSequenceZero,
    GenesisMustHaveNoPredecessor,
    NonGenesisMustHavePredecessor,
    SequenceOverflow,
    SequenceMismatch,
    PredecessorMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppendReceipt {
    pub head: ChainHead,
    pub predecessor: Option<EntryId>,
    pub contract_version: u16,
}

/// Validate an append against explicit predecessor evidence.
/// Unavailable/invalid predecessor state can never be interpreted as genesis.
pub fn validate_append(
    previous: &PreviousState,
    candidate: &AppendCandidate,
) -> Result<AppendReceipt, AppendError> {
    match previous {
        PreviousState::Unavailable(_) => return Err(AppendError::PreviousStateUnavailable),
        PreviousState::Invalid(_) => return Err(AppendError::PreviousStateInvalid),
        PreviousState::KnownAbsent(identity) => {
            if candidate.identity != *identity {
                return Err(AppendError::IdentityMismatch);
            }
            if candidate.sequence != 0 {
                return Err(AppendError::GenesisMustBeSequenceZero);
            }
            if candidate.predecessor.is_some() {
                return Err(AppendError::GenesisMustHaveNoPredecessor);
            }
        }
        PreviousState::KnownHead(head) => {
            if candidate.identity != head.identity {
                return Err(AppendError::IdentityMismatch);
            }
            let expected = head
                .sequence
                .checked_add(1)
                .ok_or(AppendError::SequenceOverflow)?;
            if candidate.sequence != expected {
                return Err(AppendError::SequenceMismatch);
            }
            let predecessor = candidate
                .predecessor
                .ok_or(AppendError::NonGenesisMustHavePredecessor)?;
            if predecessor != head.entry_id {
                return Err(AppendError::PredecessorMismatch);
            }
        }
    }

    Ok(AppendReceipt {
        head: candidate.head(),
        predecessor: candidate.predecessor,
        contract_version: AUDIT_CHAIN_CONTRACT_V1,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateRelation {
    SameEntry,
    Fork,
    Unrelated,
}

/// Competing children of the same exact predecessor/sequence are a fork.
pub fn compare_candidates(a: &AppendCandidate, b: &AppendCandidate) -> CandidateRelation {
    if a == b {
        return CandidateRelation::SameEntry;
    }
    if a.identity == b.identity
        && a.sequence == b.sequence
        && a.predecessor == b.predecessor
    {
        CandidateRelation::Fork
    } else {
        CandidateRelation::Unrelated
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChainQueryResult {
    KnownAbsent(ChainIdentity),
    /// Complete but potentially unordered retrieval for exactly one chain.
    Complete(Vec<AppendCandidate>),
    Unavailable(UnavailableReason),
    Invalid(InvalidStateReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainVerificationError {
    QueryUnavailable,
    QueryInvalid,
    EmptyCompleteSetIsAmbiguous,
    MixedIdentity,
    DuplicateSequence,
    ForkDetected,
    InvalidGenesis,
    SequenceGap,
    PredecessorMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifiedChain {
    KnownEmpty(ChainIdentity),
    Complete {
        head: ChainHead,
        entry_count: usize,
    },
}

/// Verify a complete chain independent of retrieval order.
pub fn verify_chain(query: ChainQueryResult) -> Result<VerifiedChain, ChainVerificationError> {
    match query {
        ChainQueryResult::Unavailable(_) => Err(ChainVerificationError::QueryUnavailable),
        ChainQueryResult::Invalid(_) => Err(ChainVerificationError::QueryInvalid),
        ChainQueryResult::KnownAbsent(identity) => Ok(VerifiedChain::KnownEmpty(identity)),
        ChainQueryResult::Complete(mut entries) => {
            if entries.is_empty() {
                return Err(ChainVerificationError::EmptyCompleteSetIsAmbiguous);
            }

            let identity = entries[0].identity;
            if entries.iter().any(|entry| entry.identity != identity) {
                return Err(ChainVerificationError::MixedIdentity);
            }

            entries.sort_by_key(|entry| entry.sequence);
            for pair in entries.windows(2) {
                if pair[0].sequence == pair[1].sequence {
                    return if compare_candidates(&pair[0], &pair[1]) == CandidateRelation::Fork {
                        Err(ChainVerificationError::ForkDetected)
                    } else {
                        Err(ChainVerificationError::DuplicateSequence)
                    };
                }
            }

            let first = &entries[0];
            if first.sequence != 0 || first.predecessor.is_some() {
                return Err(ChainVerificationError::InvalidGenesis);
            }

            for pair in entries.windows(2) {
                let expected = pair[0]
                    .sequence
                    .checked_add(1)
                    .ok_or(ChainVerificationError::SequenceGap)?;
                if pair[1].sequence != expected {
                    return Err(ChainVerificationError::SequenceGap);
                }
                if pair[1].predecessor != Some(pair[0].entry_id) {
                    return Err(ChainVerificationError::PredecessorMismatch);
                }
            }

            let head = entries.last().expect("non-empty checked above").head();
            Ok(VerifiedChain::Complete {
                head,
                entry_count: entries.len(),
            })
        }
    }
}

/// One verified per-author head for a single subject context.
/// Chain IDs are author-scoped; equal chain-id bytes across distinct authors are allowed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditCheckpoint {
    pub subject: SubjectContext,
    heads: Vec<ChainHead>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointError {
    NoHeads,
    SubjectMismatch,
    DuplicateAuthor,
}

impl AuditCheckpoint {
    pub fn new(subject: SubjectContext, mut heads: Vec<ChainHead>) -> Result<Self, CheckpointError> {
        if heads.is_empty() {
            return Err(CheckpointError::NoHeads);
        }
        if heads.iter().any(|head| head.identity.subject != subject) {
            return Err(CheckpointError::SubjectMismatch);
        }

        heads.sort_by_key(|head| (head.identity.author, head.identity.chain_id));
        for pair in heads.windows(2) {
            if pair[0].identity.author == pair[1].identity.author {
                return Err(CheckpointError::DuplicateAuthor);
            }
        }

        Ok(Self { subject, heads })
    }

    pub fn heads(&self) -> &[ChainHead] {
        &self.heads
    }

    pub fn transcript_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(CHECKPOINT_TRANSCRIPT_DOMAIN);
        out.extend_from_slice(&AUDIT_CHAIN_CONTRACT_V1.to_be_bytes());
        out.extend_from_slice(self.subject.bytes());
        out.extend_from_slice(&(self.heads.len() as u64).to_be_bytes());
        for head in &self.heads {
            out.extend_from_slice(head.identity.author.bytes());
            out.extend_from_slice(head.identity.chain_id.bytes());
            out.extend_from_slice(&head.sequence.to_be_bytes());
            out.extend_from_slice(head.entry_id.bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject(byte: u8) -> SubjectContext {
        SubjectContext::new([byte; 32]).unwrap()
    }
    fn author(byte: u8) -> AuthorId {
        AuthorId::new([byte; 32]).unwrap()
    }
    fn chain(byte: u8) -> ChainId {
        ChainId::new([byte; 32]).unwrap()
    }
    fn entry(byte: u8) -> EntryId {
        EntryId::new([byte; 32]).unwrap()
    }
    fn event(byte: u8) -> EventDigest {
        EventDigest::new([byte; 32]).unwrap()
    }
    fn identity(subject_byte: u8, author_byte: u8, chain_byte: u8) -> ChainIdentity {
        ChainIdentity {
            subject: subject(subject_byte),
            author: author(author_byte),
            chain_id: chain(chain_byte),
        }
    }
    fn candidate(
        id: ChainIdentity,
        sequence: u64,
        predecessor: Option<EntryId>,
        entry_byte: u8,
    ) -> AppendCandidate {
        AppendCandidate {
            identity: id,
            sequence,
            predecessor,
            event_digest: event(entry_byte.saturating_add(20)),
            occurred_at_micros: 100 + sequence as i64,
            entry_id: entry(entry_byte),
        }
    }

    #[test]
    fn genesis_requires_positive_known_absence_for_exact_identity() {
        let id = identity(1, 2, 3);
        let genesis = candidate(id, 0, None, 10);
        assert!(validate_append(&PreviousState::KnownAbsent(id), &genesis).is_ok());
        assert_eq!(
            validate_append(
                &PreviousState::Unavailable(UnavailableReason::Network),
                &genesis,
            ),
            Err(AppendError::PreviousStateUnavailable)
        );
        assert_eq!(
            validate_append(
                &PreviousState::Invalid(InvalidStateReason::Decode),
                &genesis,
            ),
            Err(AppendError::PreviousStateInvalid)
        );
    }

    #[test]
    fn subject_substitution_is_identity_mismatch() {
        let old = identity(1, 2, 3);
        let mut candidate = candidate(old, 1, Some(entry(10)), 11);
        candidate.identity = identity(9, 2, 3);
        let head = ChainHead {
            identity: old,
            sequence: 0,
            entry_id: entry(10),
        };
        assert_eq!(
            validate_append(&PreviousState::KnownHead(head), &candidate),
            Err(AppendError::IdentityMismatch)
        );
    }

    #[test]
    fn non_genesis_requires_exact_sequence_and_predecessor() {
        let id = identity(1, 2, 3);
        let head = candidate(id, 0, None, 10).head();
        assert!(validate_append(
            &PreviousState::KnownHead(head.clone()),
            &candidate(id, 1, Some(entry(10)), 11),
        )
        .is_ok());
        assert_eq!(
            validate_append(
                &PreviousState::KnownHead(head.clone()),
                &candidate(id, 1, Some(entry(99)), 12),
            ),
            Err(AppendError::PredecessorMismatch)
        );
        assert_eq!(
            validate_append(
                &PreviousState::KnownHead(head),
                &candidate(id, 2, Some(entry(10)), 13),
            ),
            Err(AppendError::SequenceMismatch)
        );
    }

    #[test]
    fn transcript_binds_subject_and_chain_fields() {
        let id = identity(1, 2, 3);
        let a = candidate(id, 1, Some(entry(10)), 11);
        let mut b = a.clone();
        b.identity.subject = subject(9);
        assert_ne!(a.transcript_bytes(), b.transcript_bytes());
        b = a.clone();
        b.event_digest = event(88);
        assert_ne!(a.transcript_bytes(), b.transcript_bytes());
    }

    #[test]
    fn competing_children_are_a_fork() {
        let id = identity(1, 2, 3);
        let a = candidate(id, 1, Some(entry(10)), 11);
        let b = candidate(id, 1, Some(entry(10)), 12);
        assert_eq!(compare_candidates(&a, &b), CandidateRelation::Fork);
    }

    #[test]
    fn verification_is_independent_of_retrieval_order() {
        let id = identity(1, 2, 3);
        let a = candidate(id, 0, None, 10);
        let b = candidate(id, 1, Some(entry(10)), 11);
        let c = candidate(id, 2, Some(entry(11)), 12);
        let forward = verify_chain(ChainQueryResult::Complete(vec![a.clone(), b.clone(), c.clone()]))
            .unwrap();
        let reverse = verify_chain(ChainQueryResult::Complete(vec![c, b, a])).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn unavailable_invalid_or_ambiguous_empty_never_becomes_genesis() {
        assert_eq!(
            verify_chain(ChainQueryResult::Unavailable(UnavailableReason::Network)),
            Err(ChainVerificationError::QueryUnavailable)
        );
        assert_eq!(
            verify_chain(ChainQueryResult::Invalid(InvalidStateReason::Decode)),
            Err(ChainVerificationError::QueryInvalid)
        );
        assert_eq!(
            verify_chain(ChainQueryResult::Complete(vec![])),
            Err(ChainVerificationError::EmptyCompleteSetIsAmbiguous)
        );
    }

    #[test]
    fn known_absence_is_explicitly_distinct_from_empty_complete() {
        let id = identity(1, 2, 3);
        assert_eq!(
            verify_chain(ChainQueryResult::KnownAbsent(id)),
            Ok(VerifiedChain::KnownEmpty(id))
        );
    }

    #[test]
    fn gap_predecessor_mismatch_and_fork_are_rejected() {
        let id = identity(1, 2, 3);
        let a = candidate(id, 0, None, 10);
        assert_eq!(
            verify_chain(ChainQueryResult::Complete(vec![
                a.clone(),
                candidate(id, 2, Some(entry(10)), 12),
            ])),
            Err(ChainVerificationError::SequenceGap)
        );
        assert_eq!(
            verify_chain(ChainQueryResult::Complete(vec![
                a.clone(),
                candidate(id, 1, Some(entry(99)), 11),
            ])),
            Err(ChainVerificationError::PredecessorMismatch)
        );
        assert_eq!(
            verify_chain(ChainQueryResult::Complete(vec![
                a,
                candidate(id, 1, Some(entry(10)), 11),
                candidate(id, 1, Some(entry(10)), 12),
            ])),
            Err(ChainVerificationError::ForkDetected)
        );
    }

    #[test]
    fn mixed_subject_author_or_chain_identity_is_rejected() {
        let a = candidate(identity(1, 2, 3), 0, None, 10);
        let b = candidate(identity(9, 2, 3), 1, Some(entry(10)), 11);
        assert_eq!(
            verify_chain(ChainQueryResult::Complete(vec![a, b])),
            Err(ChainVerificationError::MixedIdentity)
        );
    }

    fn head(subject_byte: u8, author_byte: u8, chain_byte: u8, entry_byte: u8) -> ChainHead {
        ChainHead {
            identity: identity(subject_byte, author_byte, chain_byte),
            sequence: 1,
            entry_id: entry(entry_byte),
        }
    }

    #[test]
    fn checkpoint_is_deterministic_across_input_order() {
        let a = head(1, 2, 7, 10);
        let b = head(1, 3, 8, 11);
        let first = AuditCheckpoint::new(subject(1), vec![a.clone(), b.clone()]).unwrap();
        let second = AuditCheckpoint::new(subject(1), vec![b, a]).unwrap();
        assert_eq!(first.transcript_bytes(), second.transcript_bytes());
    }

    #[test]
    fn checkpoint_rejects_subject_mismatch() {
        assert_eq!(
            AuditCheckpoint::new(subject(1), vec![head(9, 2, 7, 10)]),
            Err(CheckpointError::SubjectMismatch)
        );
    }

    #[test]
    fn checkpoint_rejects_duplicate_author() {
        assert_eq!(
            AuditCheckpoint::new(
                subject(1),
                vec![head(1, 2, 7, 10), head(1, 2, 8, 11)],
            ),
            Err(CheckpointError::DuplicateAuthor)
        );
    }

    #[test]
    fn author_scoped_chain_ids_may_repeat_across_different_authors() {
        let checkpoint = AuditCheckpoint::new(
            subject(1),
            vec![head(1, 2, 7, 10), head(1, 3, 7, 11)],
        )
        .unwrap();
        assert_eq!(checkpoint.heads().len(), 2);
    }

    #[test]
    fn checkpoint_rejects_empty_head_set() {
        assert_eq!(
            AuditCheckpoint::new(subject(1), vec![]),
            Err(CheckpointError::NoHeads)
        );
    }

    #[test]
    fn opaque_ids_reject_zero_and_redact_debug() {
        assert_eq!(ChainId::new([0; 32]), Err(IdError::AllZero));
        assert_eq!(format!("{:?}", chain(7)), "ChainId([redacted])");
        assert_eq!(format!("{:?}", event(8)), "EventDigest([redacted])");
    }
}
