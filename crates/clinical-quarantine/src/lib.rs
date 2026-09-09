#![deny(unsafe_code)]
//! Privacy-preserving quarantine and reconciliation lineage for Mycelix-Health.
//!
//! The distributed quarantine record intentionally contains no raw clinical
//! payload, patient identifier, source resource identifier, source-system URL,
//! or free-form error text. Sensitive payload/context remain in a separate
//! encrypted/local store. The shared record carries only typed reason codes,
//! keyed commitments, artifact digests, state transitions, and authority lineage.

use mycelix_fhir_conformance::{ConformanceIssue, ConformanceIssueKind};
use mycelix_fhir_semantics::ProjectionIssueKind;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DigestAlgorithm {
    Sha256,
    Blake3,
}

/// Public integrity digest for non-sensitive artifacts such as a verifier binary,
/// decision record, or prior public quarantine event.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ArtifactDigest {
    pub algorithm: DigestAlgorithm,
    pub value: [u8; 32],
}

impl ArtifactDigest {
    fn validate(&self) -> Result<(), QuarantineError> {
        if self.value == [0u8; 32] {
            return Err(QuarantineError::ZeroArtifactDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CommitmentScheme {
    HmacSha256,
    Blake3Keyed,
}

/// Keyed commitment for potentially identifying material. A raw SHA-256 digest of
/// a clinical payload is deliberately not representable here because deterministic
/// public hashes can enable confirmation/dictionary attacks when an attacker has a
/// candidate payload.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct OpaqueCommitment {
    pub scheme: CommitmentScheme,
    pub value: [u8; 32],
}

impl OpaqueCommitment {
    fn validate(&self) -> Result<(), QuarantineError> {
        if self.value == [0u8; 32] {
            return Err(QuarantineError::ZeroOpaqueCommitment);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum QuarantineScope {
    Bundle,
    Resource,
    ClinicalFact,
    MedicationOrder,
    EvidenceAssertion,
}

/// Stable machine-readable reason taxonomy. No free-text extension exists in v1;
/// adding a new reason requires an explicit code change/review so PHI cannot leak
/// through an "error message" field on a distributed record.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum QuarantineReasonCode {
    EmptySourceProvenance,
    WrongResourceType,
    MissingBundleType,
    UnsupportedBundleType,
    MissingBundleEntries,
    InvalidPatientIdentity,
    MissingPatient,
    MultiplePatients,
    DuplicateResourceIdentity,
    DuplicateFullUrlVersion,
    VersionSpecificFullUrl,
    MissingObservationId,
    MissingObservationStatus,
    NonPromotableObservationStatus,
    MissingEffectiveTime,
    UnsupportedEffectiveForm,
    SubjectMismatch,
    SubjectUnresolved,
    InvalidObservation,
    SemanticValidationFailure,
    DuplicateFactIdentity,
    MedicationSemanticsFailure,
    AuthorityUnverified,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuarantineState {
    Pending,
    UnderReview,
    ResolvedPromoted,
    ResolvedDiscarded,
}

impl QuarantineState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::ResolvedPromoted | Self::ResolvedDiscarded)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolutionEvidence {
    /// Public digest of the exact decision artifact / reconciliation record.
    pub decision_artifact_digest: ArtifactDigest,
    /// Required only when a quarantined item is promoted after reconciliation.
    pub promoted_artifact_digest: Option<ArtifactDigest>,
}

impl ResolutionEvidence {
    fn validate_for_state(&self, state: QuarantineState) -> Result<(), QuarantineError> {
        self.decision_artifact_digest.validate()?;
        match state {
            QuarantineState::ResolvedPromoted => {
                self.promoted_artifact_digest
                    .as_ref()
                    .ok_or(QuarantineError::MissingPromotedArtifact)?
                    .validate()?;
            }
            QuarantineState::ResolvedDiscarded => {
                if self.promoted_artifact_digest.is_some() {
                    return Err(QuarantineError::DiscardCannotPromoteArtifact);
                }
            }
            QuarantineState::Pending | QuarantineState::UnderReview => {
                return Err(QuarantineError::ResolutionOnNonTerminalState);
            }
        }
        Ok(())
    }
}

/// Public/shared quarantine event.
///
/// `case_nonce` is random opaque case identity. `payload_commitment` must be
/// computed with a secret-keyed scheme outside this crate. The secret key and raw
/// payload never appear in this structure.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuarantineEvent {
    pub case_nonce: [u8; 16],
    pub sequence: u32,
    pub payload_commitment: OpaqueCommitment,
    pub scope: QuarantineScope,
    pub reasons: Vec<QuarantineReasonCode>,
    pub state: QuarantineState,
    pub recorded_at_micros: i64,
    /// Public digest of the software/artifact that emitted this transition.
    pub producer_artifact_digest: ArtifactDigest,
    /// Digest of the immediately preceding public event. Required after sequence 0.
    pub previous_event_digest: Option<ArtifactDigest>,
    /// Opaque commitment to reviewer/authority identity. This permits later proof
    /// without exposing a practitioner DID, NPI, agent key, or patient-visible name.
    pub reviewer_authority_commitment: Option<OpaqueCommitment>,
    pub resolution: Option<ResolutionEvidence>,
}

impl QuarantineEvent {
    pub fn new_pending(
        case_nonce: [u8; 16],
        payload_commitment: OpaqueCommitment,
        scope: QuarantineScope,
        reasons: Vec<QuarantineReasonCode>,
        producer_artifact_digest: ArtifactDigest,
        recorded_at_micros: i64,
    ) -> Result<Self, QuarantineError> {
        let event = Self {
            case_nonce,
            sequence: 0,
            payload_commitment,
            scope,
            reasons,
            state: QuarantineState::Pending,
            recorded_at_micros,
            producer_artifact_digest,
            previous_event_digest: None,
            reviewer_authority_commitment: None,
            resolution: None,
        };
        event.validate()?;
        Ok(event)
    }

    pub fn begin_review(
        previous: &Self,
        previous_event_digest: ArtifactDigest,
        producer_artifact_digest: ArtifactDigest,
        reviewer_authority_commitment: OpaqueCommitment,
        recorded_at_micros: i64,
    ) -> Result<Self, QuarantineError> {
        let next = Self {
            case_nonce: previous.case_nonce,
            sequence: previous
                .sequence
                .checked_add(1)
                .ok_or(QuarantineError::SequenceOverflow)?,
            payload_commitment: previous.payload_commitment,
            scope: previous.scope,
            reasons: previous.reasons.clone(),
            state: QuarantineState::UnderReview,
            recorded_at_micros,
            producer_artifact_digest,
            previous_event_digest: Some(previous_event_digest),
            reviewer_authority_commitment: Some(reviewer_authority_commitment),
            resolution: None,
        };
        validate_transition(previous, previous_event_digest, &next)?;
        Ok(next)
    }

    pub fn resolve(
        previous: &Self,
        previous_event_digest: ArtifactDigest,
        producer_artifact_digest: ArtifactDigest,
        reviewer_authority_commitment: OpaqueCommitment,
        disposition: ResolutionDisposition,
        decision_artifact_digest: ArtifactDigest,
        promoted_artifact_digest: Option<ArtifactDigest>,
        recorded_at_micros: i64,
    ) -> Result<Self, QuarantineError> {
        let state = match disposition {
            ResolutionDisposition::Promote => QuarantineState::ResolvedPromoted,
            ResolutionDisposition::Discard => QuarantineState::ResolvedDiscarded,
        };
        let next = Self {
            case_nonce: previous.case_nonce,
            sequence: previous
                .sequence
                .checked_add(1)
                .ok_or(QuarantineError::SequenceOverflow)?,
            payload_commitment: previous.payload_commitment,
            scope: previous.scope,
            reasons: previous.reasons.clone(),
            state,
            recorded_at_micros,
            producer_artifact_digest,
            previous_event_digest: Some(previous_event_digest),
            reviewer_authority_commitment: Some(reviewer_authority_commitment),
            resolution: Some(ResolutionEvidence {
                decision_artifact_digest,
                promoted_artifact_digest,
            }),
        };
        validate_transition(previous, previous_event_digest, &next)?;
        Ok(next)
    }

    pub fn validate(&self) -> Result<(), QuarantineError> {
        if self.case_nonce == [0u8; 16] {
            return Err(QuarantineError::ZeroCaseNonce);
        }
        self.payload_commitment.validate()?;
        self.producer_artifact_digest.validate()?;

        if self.reasons.is_empty() {
            return Err(QuarantineError::MissingReason);
        }
        let mut unique = HashSet::new();
        if self.reasons.iter().any(|reason| !unique.insert(*reason)) {
            return Err(QuarantineError::DuplicateReason);
        }

        match self.sequence {
            0 if self.previous_event_digest.is_some() => {
                return Err(QuarantineError::InitialEventHasPredecessor);
            }
            0 if self.state != QuarantineState::Pending => {
                return Err(QuarantineError::InitialEventMustBePending);
            }
            0 => {}
            _ => {
                self.previous_event_digest
                    .as_ref()
                    .ok_or(QuarantineError::MissingPredecessorDigest)?
                    .validate()?;
            }
        }

        match self.state {
            QuarantineState::Pending => {
                if self.reviewer_authority_commitment.is_some() || self.resolution.is_some() {
                    return Err(QuarantineError::PendingCarriesReviewOrResolution);
                }
            }
            QuarantineState::UnderReview => {
                self.reviewer_authority_commitment
                    .as_ref()
                    .ok_or(QuarantineError::MissingReviewerAuthority)?
                    .validate()?;
                if self.resolution.is_some() {
                    return Err(QuarantineError::ReviewCarriesResolution);
                }
            }
            QuarantineState::ResolvedPromoted | QuarantineState::ResolvedDiscarded => {
                self.reviewer_authority_commitment
                    .as_ref()
                    .ok_or(QuarantineError::MissingReviewerAuthority)?
                    .validate()?;
                self.resolution
                    .as_ref()
                    .ok_or(QuarantineError::MissingResolutionEvidence)?
                    .validate_for_state(self.state)?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResolutionDisposition {
    Promote,
    Discard,
}

/// Validate one immutable lineage transition against the digest of its exact
/// predecessor event.
pub fn validate_transition(
    previous: &QuarantineEvent,
    previous_event_digest: ArtifactDigest,
    next: &QuarantineEvent,
) -> Result<(), QuarantineError> {
    previous.validate()?;
    next.validate()?;
    previous_event_digest.validate()?;

    if previous.state.is_terminal() {
        return Err(QuarantineError::TerminalStateCannotTransition);
    }
    if previous.case_nonce != next.case_nonce {
        return Err(QuarantineError::CaseIdentityChanged);
    }
    if previous.payload_commitment != next.payload_commitment {
        return Err(QuarantineError::PayloadCommitmentChanged);
    }
    if previous.scope != next.scope {
        return Err(QuarantineError::ScopeChanged);
    }
    if previous.reasons != next.reasons {
        return Err(QuarantineError::ReasonsChanged);
    }
    if next.sequence != previous.sequence + 1 {
        return Err(QuarantineError::InvalidSequence);
    }
    if next.previous_event_digest != Some(previous_event_digest) {
        return Err(QuarantineError::PredecessorDigestMismatch);
    }
    if next.recorded_at_micros < previous.recorded_at_micros {
        return Err(QuarantineError::TimestampRegressed);
    }

    match (previous.state, next.state) {
        (QuarantineState::Pending, QuarantineState::UnderReview)
        | (QuarantineState::UnderReview, QuarantineState::UnderReview)
        | (QuarantineState::UnderReview, QuarantineState::ResolvedPromoted)
        | (QuarantineState::UnderReview, QuarantineState::ResolvedDiscarded) => Ok(()),
        _ => Err(QuarantineError::InvalidStateTransition),
    }
}

/// Convert an existing strict FHIR conformance issue into the closed quarantine
/// reason taxonomy without copying its human-readable message or resource id.
pub fn reason_from_fhir_issue(issue: &ConformanceIssue) -> QuarantineReasonCode {
    match &issue.kind {
        ConformanceIssueKind::EmptySourceSystem => QuarantineReasonCode::EmptySourceProvenance,
        ConformanceIssueKind::WrongResourceType => QuarantineReasonCode::WrongResourceType,
        ConformanceIssueKind::MissingBundleType => QuarantineReasonCode::MissingBundleType,
        ConformanceIssueKind::UnsupportedBundleType => QuarantineReasonCode::UnsupportedBundleType,
        ConformanceIssueKind::MissingEntries => QuarantineReasonCode::MissingBundleEntries,
        ConformanceIssueKind::InvalidPatient => QuarantineReasonCode::InvalidPatientIdentity,
        ConformanceIssueKind::MissingPatient => QuarantineReasonCode::MissingPatient,
        ConformanceIssueKind::MultiplePatients => QuarantineReasonCode::MultiplePatients,
        ConformanceIssueKind::DuplicateResourceIdentity => {
            QuarantineReasonCode::DuplicateResourceIdentity
        }
        ConformanceIssueKind::DuplicateFullUrlVersion => {
            QuarantineReasonCode::DuplicateFullUrlVersion
        }
        ConformanceIssueKind::VersionSpecificFullUrl => {
            QuarantineReasonCode::VersionSpecificFullUrl
        }
        ConformanceIssueKind::MissingObservationId => QuarantineReasonCode::MissingObservationId,
        ConformanceIssueKind::MissingObservationStatus => {
            QuarantineReasonCode::MissingObservationStatus
        }
        ConformanceIssueKind::NonPromotableObservationStatus => {
            QuarantineReasonCode::NonPromotableObservationStatus
        }
        ConformanceIssueKind::MissingEffectiveTime => QuarantineReasonCode::MissingEffectiveTime,
        ConformanceIssueKind::UnsupportedEffectiveForm => {
            QuarantineReasonCode::UnsupportedEffectiveForm
        }
        ConformanceIssueKind::Projection(kind) => match kind {
            ProjectionIssueKind::MissingPatient => QuarantineReasonCode::MissingPatient,
            ProjectionIssueKind::SubjectMismatch => QuarantineReasonCode::SubjectMismatch,
            ProjectionIssueKind::SubjectUnresolved => QuarantineReasonCode::SubjectUnresolved,
            ProjectionIssueKind::InvalidObservation => QuarantineReasonCode::InvalidObservation,
            ProjectionIssueKind::SemanticValidation => {
                QuarantineReasonCode::SemanticValidationFailure
            }
        },
        ConformanceIssueKind::DuplicateFactIdentity => QuarantineReasonCode::DuplicateFactIdentity,
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum QuarantineError {
    #[error("quarantine case nonce must be non-zero")]
    ZeroCaseNonce,
    #[error("opaque commitment must be non-zero")]
    ZeroOpaqueCommitment,
    #[error("artifact digest must be non-zero")]
    ZeroArtifactDigest,
    #[error("at least one typed quarantine reason is required")]
    MissingReason,
    #[error("quarantine reason codes must be unique")]
    DuplicateReason,
    #[error("initial quarantine event cannot have a predecessor")]
    InitialEventHasPredecessor,
    #[error("initial quarantine event must be pending")]
    InitialEventMustBePending,
    #[error("non-initial quarantine event requires predecessor digest")]
    MissingPredecessorDigest,
    #[error("pending event cannot carry reviewer authority or resolution")]
    PendingCarriesReviewOrResolution,
    #[error("review/terminal event requires reviewer authority commitment")]
    MissingReviewerAuthority,
    #[error("under-review event cannot carry terminal resolution")]
    ReviewCarriesResolution,
    #[error("terminal quarantine event requires resolution evidence")]
    MissingResolutionEvidence,
    #[error("promoted resolution requires promoted artifact digest")]
    MissingPromotedArtifact,
    #[error("discard resolution cannot claim a promoted artifact")]
    DiscardCannotPromoteArtifact,
    #[error("resolution evidence is only valid on terminal states")]
    ResolutionOnNonTerminalState,
    #[error("quarantine event sequence overflow")]
    SequenceOverflow,
    #[error("terminal quarantine state cannot transition")]
    TerminalStateCannotTransition,
    #[error("quarantine case identity changed across transition")]
    CaseIdentityChanged,
    #[error("payload commitment changed across transition")]
    PayloadCommitmentChanged,
    #[error("quarantine scope changed across transition")]
    ScopeChanged,
    #[error("quarantine reason set changed across transition")]
    ReasonsChanged,
    #[error("quarantine sequence is not predecessor + 1")]
    InvalidSequence,
    #[error("predecessor digest does not bind the exact prior event")]
    PredecessorDigestMismatch,
    #[error("quarantine event timestamp regressed")]
    TimestampRegressed,
    #[error("invalid quarantine state transition")]
    InvalidStateTransition,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_fhir_conformance::{ConformanceIssue, IssueSeverity};

    fn artifact(byte: u8) -> ArtifactDigest {
        ArtifactDigest {
            algorithm: DigestAlgorithm::Sha256,
            value: [byte; 32],
        }
    }

    fn commitment(byte: u8) -> OpaqueCommitment {
        OpaqueCommitment {
            scheme: CommitmentScheme::HmacSha256,
            value: [byte; 32],
        }
    }

    fn pending() -> QuarantineEvent {
        QuarantineEvent::new_pending(
            [1u8; 16],
            commitment(2),
            QuarantineScope::Resource,
            vec![QuarantineReasonCode::SubjectUnresolved],
            artifact(3),
            100,
        )
        .unwrap()
    }

    #[test]
    fn public_event_contains_only_closed_metadata_types() {
        let event = pending();
        assert_eq!(event.sequence, 0);
        assert_eq!(event.state, QuarantineState::Pending);
        assert_eq!(event.reasons, vec![QuarantineReasonCode::SubjectUnresolved]);
    }

    #[test]
    fn zero_case_nonce_is_rejected() {
        let error = QuarantineEvent::new_pending(
            [0u8; 16],
            commitment(2),
            QuarantineScope::Resource,
            vec![QuarantineReasonCode::SubjectMismatch],
            artifact(3),
            100,
        )
        .unwrap_err();
        assert_eq!(error, QuarantineError::ZeroCaseNonce);
    }

    #[test]
    fn raw_unkeyed_payload_hash_is_not_representable_as_payload_commitment() {
        let event = pending();
        assert!(matches!(
            event.payload_commitment.scheme,
            CommitmentScheme::HmacSha256 | CommitmentScheme::Blake3Keyed
        ));
    }

    #[test]
    fn cannot_resolve_before_review() {
        let previous = pending();
        let next = QuarantineEvent {
            case_nonce: previous.case_nonce,
            sequence: 1,
            payload_commitment: previous.payload_commitment,
            scope: previous.scope,
            reasons: previous.reasons.clone(),
            state: QuarantineState::ResolvedDiscarded,
            recorded_at_micros: 101,
            producer_artifact_digest: artifact(4),
            previous_event_digest: Some(artifact(5)),
            reviewer_authority_commitment: Some(commitment(6)),
            resolution: Some(ResolutionEvidence {
                decision_artifact_digest: artifact(7),
                promoted_artifact_digest: None,
            }),
        };
        let error = validate_transition(&previous, artifact(5), &next).unwrap_err();
        assert_eq!(error, QuarantineError::InvalidStateTransition);
    }

    #[test]
    fn reviewed_item_can_be_promoted_with_exact_lineage() {
        let pending = pending();
        let review = QuarantineEvent::begin_review(
            &pending,
            artifact(10),
            artifact(11),
            commitment(12),
            101,
        )
        .unwrap();
        let resolved = QuarantineEvent::resolve(
            &review,
            artifact(13),
            artifact(14),
            commitment(12),
            ResolutionDisposition::Promote,
            artifact(15),
            Some(artifact(16)),
            102,
        )
        .unwrap();
        assert_eq!(resolved.state, QuarantineState::ResolvedPromoted);
        assert_eq!(resolved.sequence, 2);
    }

    #[test]
    fn promotion_without_promoted_artifact_is_rejected() {
        let pending = pending();
        let review = QuarantineEvent::begin_review(
            &pending,
            artifact(10),
            artifact(11),
            commitment(12),
            101,
        )
        .unwrap();
        let error = QuarantineEvent::resolve(
            &review,
            artifact(13),
            artifact(14),
            commitment(12),
            ResolutionDisposition::Promote,
            artifact(15),
            None,
            102,
        )
        .unwrap_err();
        assert_eq!(error, QuarantineError::MissingPromotedArtifact);
    }

    #[test]
    fn discard_cannot_claim_promoted_artifact() {
        let pending = pending();
        let review = QuarantineEvent::begin_review(
            &pending,
            artifact(10),
            artifact(11),
            commitment(12),
            101,
        )
        .unwrap();
        let error = QuarantineEvent::resolve(
            &review,
            artifact(13),
            artifact(14),
            commitment(12),
            ResolutionDisposition::Discard,
            artifact(15),
            Some(artifact(16)),
            102,
        )
        .unwrap_err();
        assert_eq!(error, QuarantineError::DiscardCannotPromoteArtifact);
    }

    #[test]
    fn terminal_state_cannot_reopen() {
        let pending = pending();
        let review = QuarantineEvent::begin_review(
            &pending,
            artifact(10),
            artifact(11),
            commitment(12),
            101,
        )
        .unwrap();
        let resolved = QuarantineEvent::resolve(
            &review,
            artifact(13),
            artifact(14),
            commitment(12),
            ResolutionDisposition::Discard,
            artifact(15),
            None,
            102,
        )
        .unwrap();
        let error = QuarantineEvent::begin_review(
            &resolved,
            artifact(17),
            artifact(18),
            commitment(19),
            103,
        )
        .unwrap_err();
        assert_eq!(error, QuarantineError::TerminalStateCannotTransition);
    }

    #[test]
    fn predecessor_digest_must_match_exact_prior_event() {
        let previous = pending();
        let mut next = QuarantineEvent::begin_review(
            &previous,
            artifact(10),
            artifact(11),
            commitment(12),
            101,
        )
        .unwrap();
        next.previous_event_digest = Some(artifact(99));
        let error = validate_transition(&previous, artifact(10), &next).unwrap_err();
        assert_eq!(error, QuarantineError::PredecessorDigestMismatch);
    }

    #[test]
    fn fhir_reason_mapping_drops_message_and_resource_identity() {
        let issue = ConformanceIssue {
            severity: IssueSeverity::ResourceRejected,
            resource_type: Some("Observation".into()),
            resource_id: Some("patient-identifying-source-id".into()),
            kind: ConformanceIssueKind::Projection(ProjectionIssueKind::SubjectMismatch),
            message: "Patient/secret-a mismatched Patient/secret-b".into(),
        };
        assert_eq!(
            reason_from_fhir_issue(&issue),
            QuarantineReasonCode::SubjectMismatch
        );
    }
}
