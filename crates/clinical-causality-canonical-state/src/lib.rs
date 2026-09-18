#![deny(unsafe_code)]
//! Bounded canonical state projection for causal attestations.
//!
//! The intended high-assurance API accepts the nested canonical runtime snapshot
//! emitted by the commitment-index coordinator. Runtime provenance still belongs to
//! the trusted conductor/adapter boundary; deserialization alone is not proof that a
//! snapshot was actually materialized from the DHT.
//!
//! The reducer preserves bounded-read semantics and deliberately renames reducer
//! `Current` to `ObservedCurrentWithinBoundedRead`.

use clinical_causality_index::{
    CanonicalCausalCommitmentReadBoundaryV1, CanonicalCausalCommitmentSnapshotV1,
    CanonicalCorrectionReadBoundaryV1,
};
use clinical_causality_index_integrity::{
    commitment_index_anchor_hash, OpaqueCausalCommitmentV1,
};
use holo_hash::{ActionHash, EntryHash};
use mycelix_clinical_causality_attestation_state::{
    reduce_causal_attestation_state, CausalAttestationCorrectionRecord,
    CausalAttestationLifecycle, CausalAttestationPublicationRecord, CausalAttestationStateError,
    CausalCorrectionView, CausalProjectionView,
};
use serde::Serialize;
use std::collections::HashSet;
use thiserror::Error;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum BoundedCanonicalCausalStateV1 {
    NoCanonicalPublicationObserved,
    ObservedCurrentWithinBoundedRead {
        projection: CausalProjectionView,
    },
    ProjectionConflictWithinBoundedRead {
        projections: Vec<CausalProjectionView>,
    },
    SupersededWithinBoundedRead {
        corrections: Vec<CausalCorrectionView>,
    },
    InvalidatedWithinBoundedRead {
        corrections: Vec<CausalCorrectionView>,
    },
    ReviewRequiredWithinBoundedRead {
        corrections: Vec<CausalCorrectionView>,
    },
    PublicationSuppressedWithinBoundedRead {
        corrections: Vec<CausalCorrectionView>,
    },
}

#[derive(Clone, Serialize, PartialEq, Eq)]
pub struct BoundedCanonicalCausalStateViewV1 {
    pub commitment: OpaqueCausalCommitmentV1,
    pub anchor_hash: EntryHash,
    pub source_read_boundary: CanonicalCausalCommitmentReadBoundaryV1,
    pub observed_attestation_target_count: usize,
    pub observed_total_correction_target_count: usize,
    pub observation_started_at_micros: i64,
    pub observation_completed_at_micros: i64,
    pub publication_action_hashes: Vec<ActionHash>,
    pub effective_publication_action_hashes: Vec<ActionHash>,
    pub corrections: Vec<CausalCorrectionView>,
    pub state: BoundedCanonicalCausalStateV1,
}

/// Reduce one exact nested canonical runtime snapshot.
///
/// The intended path supplies the exact index-admitted runtime snapshot. Every
/// publication/correction entering the underlying conflict reducer must be present in
/// that supplied snapshot. The result remains bounded to the observed snapshot and
/// must never be interpreted as globally complete DHT state.
pub fn reduce_canonical_causal_snapshot(
    snapshot: &CanonicalCausalCommitmentSnapshotV1,
) -> Result<BoundedCanonicalCausalStateViewV1, CanonicalCausalStateError> {
    validate_snapshot_shape(snapshot)?;

    let mut publications = Vec::with_capacity(snapshot.publications.len());
    let mut corrections = Vec::new();
    let mut publication_hashes = HashSet::new();
    let mut correction_hashes = HashSet::new();

    for publication in &snapshot.publications {
        if !publication_hashes.insert(publication.attestation_action_hash.clone()) {
            return Err(CanonicalCausalStateError::DuplicatePublicationAction);
        }
        if publication.attestation.attestation.qualified_receipt_commitment != snapshot.commitment {
            return Err(CanonicalCausalStateError::CrossCommitmentPublication);
        }
        if publication.observed_correction_target_count != publication.corrections.len() {
            return Err(CanonicalCausalStateError::CorrectionCountMismatch);
        }
        if publication.correction_read_boundary
            != CanonicalCorrectionReadBoundaryV1::NetworkBackedStableDoubleReadWithFinalClosureCheck
        {
            return Err(CanonicalCausalStateError::UnsupportedCorrectionReadBoundary);
        }

        let correction_start = publication.correction_observation_started_at.as_micros();
        let correction_end = publication.correction_observation_completed_at.as_micros();
        if correction_start > correction_end
            || correction_start < snapshot.observation_started_at.as_micros()
            || correction_end > snapshot.observation_completed_at.as_micros()
        {
            return Err(CanonicalCausalStateError::InvalidObservationWindow);
        }

        publications.push(CausalAttestationPublicationRecord {
            action_hash: publication.attestation_action_hash.clone(),
            attestation: publication.attestation.clone(),
        });

        for correction in &publication.corrections {
            if !correction_hashes.insert(correction.action_hash.clone()) {
                return Err(CanonicalCausalStateError::DuplicateCorrectionAction);
            }
            if correction.correction.attestation_hash != publication.attestation_action_hash {
                return Err(CanonicalCausalStateError::CrossPublicationCorrection);
            }
            if correction.correction.qualified_receipt_commitment != snapshot.commitment {
                return Err(CanonicalCausalStateError::CrossCommitmentCorrection);
            }
            corrections.push(CausalAttestationCorrectionRecord {
                action_hash: correction.action_hash.clone(),
                correction: correction.correction.clone(),
            });
        }
    }

    let reduced = reduce_causal_attestation_state(&publications, &corrections)?;

    if snapshot.publications.is_empty() {
        if !reduced.lineages.is_empty() {
            return Err(CanonicalCausalStateError::UnexpectedReducerLineageCount);
        }
        return Ok(BoundedCanonicalCausalStateViewV1 {
            commitment: snapshot.commitment,
            anchor_hash: snapshot.anchor_hash.clone(),
            source_read_boundary: snapshot.read_boundary,
            observed_attestation_target_count: 0,
            observed_total_correction_target_count: 0,
            observation_started_at_micros: snapshot.observation_started_at.as_micros(),
            observation_completed_at_micros: snapshot.observation_completed_at.as_micros(),
            publication_action_hashes: Vec::new(),
            effective_publication_action_hashes: Vec::new(),
            corrections: Vec::new(),
            state: BoundedCanonicalCausalStateV1::NoCanonicalPublicationObserved,
        });
    }

    if reduced.lineages.len() != 1 {
        return Err(CanonicalCausalStateError::UnexpectedReducerLineageCount);
    }
    let lineage = reduced
        .lineages
        .into_iter()
        .next()
        .ok_or(CanonicalCausalStateError::UnexpectedReducerLineageCount)?;
    if lineage.qualified_receipt_commitment != snapshot.commitment {
        return Err(CanonicalCausalStateError::ReducerCommitmentMismatch);
    }

    let state = match lineage.lifecycle {
        CausalAttestationLifecycle::Current { projection } => {
            BoundedCanonicalCausalStateV1::ObservedCurrentWithinBoundedRead { projection }
        }
        CausalAttestationLifecycle::ProjectionConflict { projections } => {
            BoundedCanonicalCausalStateV1::ProjectionConflictWithinBoundedRead { projections }
        }
        CausalAttestationLifecycle::Superseded { corrections } => {
            BoundedCanonicalCausalStateV1::SupersededWithinBoundedRead { corrections }
        }
        CausalAttestationLifecycle::Invalidated { corrections } => {
            BoundedCanonicalCausalStateV1::InvalidatedWithinBoundedRead { corrections }
        }
        CausalAttestationLifecycle::ReviewRequired { corrections } => {
            BoundedCanonicalCausalStateV1::ReviewRequiredWithinBoundedRead { corrections }
        }
        CausalAttestationLifecycle::PublicationSuppressed { corrections } => {
            BoundedCanonicalCausalStateV1::PublicationSuppressedWithinBoundedRead { corrections }
        }
    };

    Ok(BoundedCanonicalCausalStateViewV1 {
        commitment: snapshot.commitment,
        anchor_hash: snapshot.anchor_hash.clone(),
        source_read_boundary: snapshot.read_boundary,
        observed_attestation_target_count: snapshot.observed_attestation_target_count,
        observed_total_correction_target_count: snapshot.observed_total_correction_target_count,
        observation_started_at_micros: snapshot.observation_started_at.as_micros(),
        observation_completed_at_micros: snapshot.observation_completed_at.as_micros(),
        publication_action_hashes: lineage.publication_action_hashes,
        effective_publication_action_hashes: lineage.effective_publication_action_hashes,
        corrections: lineage.corrections,
        state,
    })
}

fn validate_snapshot_shape(
    snapshot: &CanonicalCausalCommitmentSnapshotV1,
) -> Result<(), CanonicalCausalStateError> {
    if snapshot.commitment.value == [0u8; 32] {
        return Err(CanonicalCausalStateError::ZeroCommitment);
    }
    let expected_anchor = commitment_index_anchor_hash(snapshot.commitment)
        .map_err(|_| CanonicalCausalStateError::AnchorHashDerivationFailed)?;
    if snapshot.anchor_hash != expected_anchor {
        return Err(CanonicalCausalStateError::AnchorHashMismatch);
    }
    if snapshot.read_boundary
        != CanonicalCausalCommitmentReadBoundaryV1::NestedNetworkBackedStableReadsWithFinalClosureChecks
    {
        return Err(CanonicalCausalStateError::UnsupportedCanonicalReadBoundary);
    }
    if snapshot.observation_started_at > snapshot.observation_completed_at {
        return Err(CanonicalCausalStateError::InvalidObservationWindow);
    }
    if snapshot.observed_attestation_target_count != snapshot.publications.len() {
        return Err(CanonicalCausalStateError::PublicationCountMismatch);
    }

    let total = snapshot
        .publications
        .iter()
        .try_fold(0usize, |total, publication| {
            total.checked_add(publication.corrections.len())
        })
        .ok_or(CanonicalCausalStateError::CorrectionCountOverflow)?;
    if total != snapshot.observed_total_correction_target_count {
        return Err(CanonicalCausalStateError::TotalCorrectionCountMismatch);
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CanonicalCausalStateError {
    #[error("canonical causal snapshot uses a zero opaque commitment")]
    ZeroCommitment,
    #[error("canonical causal snapshot anchor hash could not be derived")]
    AnchorHashDerivationFailed,
    #[error("canonical causal snapshot anchor hash does not match its opaque commitment")]
    AnchorHashMismatch,
    #[error("canonical causal snapshot has an unsupported read boundary")]
    UnsupportedCanonicalReadBoundary,
    #[error("canonical publication has an unsupported correction read boundary")]
    UnsupportedCorrectionReadBoundary,
    #[error("canonical causal snapshot observation window is invalid")]
    InvalidObservationWindow,
    #[error("canonical causal snapshot publication count does not match materialized publications")]
    PublicationCountMismatch,
    #[error("canonical publication correction count does not match materialized corrections")]
    CorrectionCountMismatch,
    #[error("canonical causal snapshot total correction count overflowed")]
    CorrectionCountOverflow,
    #[error("canonical causal snapshot total correction count does not match publications")]
    TotalCorrectionCountMismatch,
    #[error("canonical causal snapshot contains duplicate publication action")]
    DuplicatePublicationAction,
    #[error("canonical causal snapshot contains duplicate correction action")]
    DuplicateCorrectionAction,
    #[error("canonical publication crosses opaque receipt-commitment lineage")]
    CrossCommitmentPublication,
    #[error("canonical correction targets a different publication")]
    CrossPublicationCorrection,
    #[error("canonical correction crosses opaque receipt-commitment lineage")]
    CrossCommitmentCorrection,
    #[error("underlying causal reducer returned an unexpected lineage count")]
    UnexpectedReducerLineageCount,
    #[error("underlying causal reducer returned the wrong commitment lineage")]
    ReducerCommitmentMismatch,
    #[error(transparent)]
    Reducer(#[from] CausalAttestationStateError),
}
