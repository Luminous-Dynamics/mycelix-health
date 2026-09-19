#![forbid(unsafe_code)]
//! Fail-closed audit-chain reference semantics.
//!
//! The reference deliberately models **per-author linear chains** plus explicit
//! multi-author checkpoints. It does not infer a single total order from an
//! unordered distributed collection.

use core::fmt;

pub const AUDIT_CHAIN_CONTRACT_V1: u16 = 1;
const ENTRY_TRANSCRIPT_DOMAIN: &[u8] = b"mycelix-health/audit-chain-entry/v1\0";
const CHECKPOINT_TRANSCRIPT_DOMAIN: &[u8] = b"mycelix-health/audit-checkpoint/v1\0";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChainId([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthorId([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectContext([u8; 32]);
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

opaque32!(ChainId);
opaque32!(AuthorId);
opaque32!(SubjectContext);
opaque32!(EntryId);
opaque32!(EventDigest);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainHead {
    pub chain_id: ChainId,
    pub author: AuthorId,
    pub sequence: u64,
    pub entry_id: EntryId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppendCandidate {
    pub chain_id: ChainId,
    pub author: AuthorId,
    pub sequence: u64,
    pub predecessor: Option<EntryId>,
    pub event_digest: EventDigest,
    pub occurred_at_micros: i64,
    /// Cryptographic digest/commitment supplied by an external qualified hash
    /// adapter over `transcript_bytes()`. This crate validates structure only.
    pub entry_id: EntryId,
}

impl AppendCandidate {
    pub fn transcript_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(ENTRY_TRANSCRIPT_DOMAIN.len() + 32 * 4 + 8 + 1 + 8);
        out.extend_from_slice(ENTRY_TRANSCRIPT_DOMAIN);
        out.extend_from_slice(&AUDIT_CHAIN_CONTRACT_V1.to_be_bytes());
        out.extend_from_slice(self.chain_id.bytes());
        out.extend_from_slice(self.author.bytes());
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
            chain_id: self.chain_id,
            author: self.author,
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
    /// Positive proof that no prior entry exists for this exact chain identity.
    KnownAbsent {
        chain_id: ChainId,
        author: AuthorId,
    },
    KnownHead(ChainHead),
    Unavailable(UnavailableReason),
    Invalid(InvalidStateReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppendError {
    PreviousStateUnavailable,
    PreviousStateInvalid,
    ChainMismatch,
    AuthorMismatch,
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

/// Validate one append against an explicitly known prior state.
///
/// `Unavailable` and `Invalid` can never be interpreted as genesis.
pub fn validate_append(
    previous: &PreviousState,
    candidate: &AppendCandidate,
) -> Result<AppendReceipt, AppendError> {
    match previous {
        PreviousState::Unavailable(_) => return Err(AppendError::PreviousStateUnavailable),
        PreviousState::Invalid(_) => return Err(AppendError::PreviousStateInvalid),
        PreviousState::KnownAbsent { chain_id, author } => {
            if candidate.chain_id != *chain_id {
                return Err(AppendError::ChainMismatch);
            }
            if candidate.author != *author {
                return Err(AppendError::AuthorMismatch);
            }
            if candidate.sequence != 0 {
                return Err(AppendError::GenesisMustBeSequenceZero);
            }
            if candidate.predecessor.is_some() {
                return Err(AppendError::GenesisMustHaveNoPredecessor);
            }
        }
        PreviousState::KnownHead(head) => {
            if candidate.chain_id != head.chain_id {
                return Err(AppendError::ChainMismatch);
            }
            if candidate.author != head.author {
                return Err(AppendError::AuthorMismatch);
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

/// Identify two competing children of the same predecessor/sequence.
pub fn compare_candidates(a: &AppendCandidate, b: &AppendCandidate) -> CandidateRelation {
    if a == b {
        return CandidateRelation::SameEntry;
    }
    if a.chain_id == b.chain_id
        && a.author == b.author
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
    KnownAbsent {
        chain_id: ChainId,
        author: AuthorId,
    },
    /// Complete but potentially unordered set of entries for one chain.
    Complete(Vec<AppendCandidate>),
    Unavailable(UnavailableReason),
    Invalid(InvalidStateReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainVerificationError {
    QueryUnavailable,
    QueryInvalid,
    EmptyCompleteSetIsAmbiguous,
    MixedChainIds,
    MixedAuthors,
    DuplicateSequence,
    ForkDetected,
    InvalidGenesis,
    SequenceGap,
    PredecessorMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifiedChain {
    KnownEmpty {
        chain_id: ChainId,
        author: AuthorId,
    },
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
        ChainQueryResult::KnownAbsent { chain_id, author } => {
            Ok(VerifiedChain::KnownEmpty { chain_id, author })
        }
        ChainQueryResult::Complete(mut entries) => {
            if entries.is_empty() {
                return Err(ChainVerificationError::EmptyCompleteSetIsAmbiguous);
            }

            let chain_id = entries[0].chain_id;
            let author = entries[0].author;
            if entries.iter().any(|entry| entry.chain_id != chain_id) {
                return Err(ChainVerificationError::MixedChainIds);
            }
            if entries.iter().any(|entry| entry.author != author) {
                return Err(ChainVerificationError::MixedAuthors);
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

/// A multi-author checkpoint records one verified head per author/chain. It does
/// not claim that events across authors have a single linear total order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditCheckpoint {
    pub subject: SubjectContext,
    heads: Vec<ChainHead>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointError {
    NoHeads,
    DuplicateAuthor,
    DuplicateChain,
}

impl AuditCheckpoint {
    pub fn new(subject: SubjectContext, mut heads: Vec<ChainHead>) -> Result<Self, CheckpointError> {
        if heads.is_empty() {
            return Err(CheckpointError::NoHeads);
        }
        heads.sort_by_key(|head| (head.author, head.chain_id));
        for pair in heads.windows(2) {
            if pair[0].author == pair[1].author {
                return Err(CheckpointError::DuplicateAuthor);
            }
            if pair[0].chain_id == pair[1].chain_id {
                return Err(CheckpointError::DuplicateChain);
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
            out.extend_from_slice(head.author.bytes());
            out.extend_from_slice(head.chain_id.bytes());
            out.extend_from_slice(&head.sequence.to_be_bytes());
            out.extend_from_slice(head.entry_id.bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain(byte: u8) -> ChainId {
        ChainId::new([byte; 32]).unwrap()
    }
    fn author(byte: u8) -> AuthorId {
        AuthorId::new([byte; 32]).unwrap()
    }
    fn subject(byte: u8) -> SubjectContext {
        SubjectContext::new([byte; 32]).unwrap()
    }
    fn entry(byte: u8) -> EntryId {
        EntryId::new([byte; 32]).unwrap()
    }
    fn event(byte: u8) -> EventDigest {
        EventDigest::new([byte; 32]).unwrap()
    }

    fn candidate(sequence: u64, predecessor: Option<EntryId>, entry_byte: u8) -> AppendCandidate {
        AppendCandidate {
            chain_id: chain(1),
            author: author(2),
            sequence,
            predecessor,
            event_digest: event(entry_byte.saturating_add(20)),
            occurred_at_micros: 100 + sequence as i64,
            entry_id: entry(entry_byte),
        }
    }

    #[test]
    fn genesis_requires_positive_known_absence() {
        let genesis = candidate(0, None, 10);
        assert!(validate_append(
            &PreviousState::KnownAbsent {
                chain_id: chain(1),
                author: author(2),
            },
            &genesis,
        )
        .is_ok());
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
    fn non_genesis_requires_exact_predecessor_and_sequence() {
        let head = candidate(0, None, 10).head();
        let good = candidate(1, Some(entry(10)), 11);
        assert!(validate_append(&PreviousState::KnownHead(head.clone()), &good).is_ok());

        let wrong_pred = candidate(1, Some(entry(99)), 12);
        assert_eq!(
            validate_append(&PreviousState::KnownHead(head.clone()), &wrong_pred),
            Err(AppendError::PredecessorMismatch)
        );

        let gap = candidate(2, Some(entry(10)), 13);
        assert_eq!(
            validate_append(&PreviousState::KnownHead(head), &gap),
            Err(AppendError::SequenceMismatch)
        );
    }

    #[test]
    fn canonical_entry_transcript_changes_when_chain_fields_change() {
        let a = candidate(1, Some(entry(10)), 11);
        let mut b = a.clone();
        b.sequence = 2;
        assert_ne!(a.transcript_bytes(), b.transcript_bytes());
        b = a.clone();
        b.event_digest = event(88);
        assert_ne!(a.transcript_bytes(), b.transcript_bytes());
    }

    #[test]
    fn competing_children_of_same_predecessor_are_a_fork() {
        let a = candidate(1, Some(entry(10)), 11);
        let b = candidate(1, Some(entry(10)), 12);
        assert_eq!(compare_candidates(&a, &b), CandidateRelation::Fork);
        assert_eq!(compare_candidates(&a, &a), CandidateRelation::SameEntry);
    }

    #[test]
    fn verification_is_independent_of_retrieval_order() {
        let a = candidate(0, None, 10);
        let b = candidate(1, Some(entry(10)), 11);
        let c = candidate(2, Some(entry(11)), 12);
        let forward = verify_chain(ChainQueryResult::Complete(vec![a.clone(), b.clone(), c.clone()]))
            .unwrap();
        let reverse = verify_chain(ChainQueryResult::Complete(vec![c, b, a])).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn query_failure_never_becomes_empty_genesis() {
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
    fn known_absence_is_distinct_from_empty_complete_result() {
        assert_eq!(
            verify_chain(ChainQueryResult::KnownAbsent {
                chain_id: chain(1),
                author: author(2),
            }),
            Ok(VerifiedChain::KnownEmpty {
                chain_id: chain(1),
                author: author(2),
            })
        );
    }

    #[test]
    fn sequence_gap_fails_verification() {
        let a = candidate(0, None, 10);
        let c = candidate(2, Some(entry(10)), 12);
        assert_eq!(
            verify_chain(ChainQueryResult::Complete(vec![a, c])),
            Err(ChainVerificationError::SequenceGap)
        );
    }

    #[test]
    fn predecessor_mismatch_fails_verification() {
        let a = candidate(0, None, 10);
        let b = candidate(1, Some(entry(99)), 11);
        assert_eq!(
            verify_chain(ChainQueryResult::Complete(vec![a, b])),
            Err(ChainVerificationError::PredecessorMismatch)
        );
    }

    #[test]
    fn fork_at_same_sequence_is_detected() {
        let a = candidate(0, None, 10);
        let b1 = candidate(1, Some(entry(10)), 11);
        let b2 = candidate(1, Some(entry(10)), 12);
        assert_eq!(
            verify_chain(ChainQueryResult::Complete(vec![a, b1, b2])),
            Err(ChainVerificationError::ForkDetected)
        );
    }

    #[test]
    fn mixed_author_or_chain_is_rejected() {
        let a = candidate(0, None, 10);
        let mut other_author = candidate(1, Some(entry(10)), 11);
        other_author.author = author(9);
        assert_eq!(
            verify_chain(ChainQueryResult::Complete(vec![a.clone(), other_author])),
            Err(ChainVerificationError::MixedAuthors)
        );

        let mut other_chain = candidate(1, Some(entry(10)), 11);
        other_chain.chain_id = chain(9);
        assert_eq!(
            verify_chain(ChainQueryResult::Complete(vec![a, other_chain])),
            Err(ChainVerificationError::MixedChainIds)
        );
    }

    #[test]
    fn checkpoint_is_deterministic_across_input_order() {
        let head_a = ChainHead {
            chain_id: chain(1),
            author: author(2),
            sequence: 3,
            entry_id: entry(10),
        };
        let head_b = ChainHead {
            chain_id: chain(3),
            author: author(4),
            sequence: 7,
            entry_id: entry(11),
        };
        let a = AuditCheckpoint::new(subject(5), vec![head_a.clone(), head_b.clone()]).unwrap();
        let b = AuditCheckpoint::new(subject(5), vec![head_b, head_a]).unwrap();
        assert_eq!(a.transcript_bytes(), b.transcript_bytes());
    }

    #[test]
    fn checkpoint_rejects_duplicate_author() {
        let head_a = ChainHead {
            chain_id: chain(1),
            author: author(2),
            sequence: 3,
            entry_id: entry(10),
        };
        let head_b = ChainHead {
            chain_id: chain(3),
            author: author(2),
            sequence: 7,
            entry_id: entry(11),
        };
        assert_eq!(
            AuditCheckpoint::new(subject(5), vec![head_a, head_b]),
            Err(CheckpointError::DuplicateAuthor)
        );
    }

    #[test]
    fn checkpoint_does_not_accept_empty_head_set() {
        assert_eq!(
            AuditCheckpoint::new(subject(5), vec![]),
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
