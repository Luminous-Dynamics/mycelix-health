#![deny(unsafe_code)]
//! Application boundary for canonical population-accountant state.
//!
//! This crate accepts only the nested runtime snapshot produced by the accountant
//! canonical-index coordinator. It rechecks snapshot shape/lineage and then invokes
//! the conflict-preserving reducer. Output names retain the bounded-read qualifier;
//! no result is called globally/current without qualification.

use mycelix_clinical_population_accountant_state::{
    reduce_population_accountant_state, CanonicalPopulationAccountantStateV1,
    ObservedPopulationAccountantCorrectionV1, ObservedPopulationAccountantStateV1,
    PopulationAccountantStateError, PopulationAccountantStateLineageV1,
};
use population_accountant_index::{
    CanonicalAccountantReadBoundaryV1, CanonicalPopulationAccountantLineageSnapshotV1,
};
use population_accountant_index_integrity::PopulationAccountantLineageAnchorV1;
use thiserror::Error;

#[derive(Clone, PartialEq, Eq)]
pub enum BoundedCanonicalPopulationAccountantStateV1 {
    NoCanonicalStateObserved,
    ObservedCurrentWithinBoundedCanonicalRead {
        action_hash: holo_hash::ActionHash,
        sequence: u64,
        query_count: u64,
    },
    ConflictWithinBoundedCanonicalRead {
        competing_state_hashes: Vec<holo_hash::ActionHash>,
    },
    NoCurrentStateWithinBoundedCanonicalRead {
        lineage: Vec<PopulationAccountantStateLineageV1>,
    },
    ReviewRequiredWithinBoundedCanonicalRead {
        lineage: Vec<PopulationAccountantStateLineageV1>,
    },
}

pub fn reduce_canonical_population_accountant_snapshot(
    snapshot: &CanonicalPopulationAccountantLineageSnapshotV1,
) -> Result<BoundedCanonicalPopulationAccountantStateV1, CanonicalAccountantStateError> {
    validate_snapshot(snapshot)?;

    let observed: Vec<ObservedPopulationAccountantStateV1> = snapshot
        .admitted_states
        .iter()
        .map(|item| ObservedPopulationAccountantStateV1 {
            action_hash: item.action_hash.clone(),
            state: item.state.clone(),
            corrections: item
                .corrections
                .iter()
                .map(|correction| ObservedPopulationAccountantCorrectionV1 {
                    action_hash: correction.action_hash.clone(),
                    correction: correction.correction.clone(),
                })
                .collect(),
        })
        .collect();

    let reduced = reduce_population_accountant_state(&observed)?;
    Ok(match reduced {
        CanonicalPopulationAccountantStateV1::NoTrustedStateObserved => {
            BoundedCanonicalPopulationAccountantStateV1::NoCanonicalStateObserved
        }
        CanonicalPopulationAccountantStateV1::ObservedCurrentWithinRead {
            action_hash,
            sequence,
            query_count,
        } => BoundedCanonicalPopulationAccountantStateV1::ObservedCurrentWithinBoundedCanonicalRead {
            action_hash,
            sequence,
            query_count,
        },
        CanonicalPopulationAccountantStateV1::ConflictWithinRead {
            competing_state_hashes,
        } => BoundedCanonicalPopulationAccountantStateV1::ConflictWithinBoundedCanonicalRead {
            competing_state_hashes,
        },
        CanonicalPopulationAccountantStateV1::NoCurrentStateWithinRead { lineage } => {
            BoundedCanonicalPopulationAccountantStateV1::NoCurrentStateWithinBoundedCanonicalRead {
                lineage,
            }
        }
        CanonicalPopulationAccountantStateV1::ReviewRequiredWithinRead { lineage } => {
            BoundedCanonicalPopulationAccountantStateV1::ReviewRequiredWithinBoundedCanonicalRead {
                lineage,
            }
        }
    })
}

fn validate_snapshot(
    snapshot: &CanonicalPopulationAccountantLineageSnapshotV1,
) -> Result<(), CanonicalAccountantStateError> {
    if snapshot.read_boundary
        != CanonicalAccountantReadBoundaryV1::NestedNetworkBackedStableReadsWithFinalClosureChecks
    {
        return Err(CanonicalAccountantStateError::UnexpectedReadBoundary);
    }
    if snapshot.observation_completed_at < snapshot.observation_started_at {
        return Err(CanonicalAccountantStateError::InvalidObservationWindow);
    }
    if snapshot.observed_state_count != snapshot.admitted_states.len() {
        return Err(CanonicalAccountantStateError::StateCountMismatch);
    }

    validate_lineage_anchor(&snapshot.lineage)?;

    let mut correction_count = 0usize;
    for (index, item) in snapshot.admitted_states.iter().enumerate() {
        if snapshot.admitted_states[..index]
            .iter()
            .any(|previous| previous.action_hash == item.action_hash)
        {
            return Err(CanonicalAccountantStateError::DuplicateStateAction);
        }
        let expected = PopulationAccountantLineageAnchorV1::from_state(&item.state)
            .map_err(|_| CanonicalAccountantStateError::InvalidStateLineage)?;
        if expected != snapshot.lineage {
            return Err(CanonicalAccountantStateError::CrossLineageState);
        }
        if item.observed_correction_target_count != item.corrections.len() {
            return Err(CanonicalAccountantStateError::CorrectionCountMismatch);
        }
        correction_count = correction_count
            .checked_add(item.corrections.len())
            .ok_or(CanonicalAccountantStateError::CorrectionCountOverflow)?;
        for (correction_index, correction) in item.corrections.iter().enumerate() {
            if item.corrections[..correction_index]
                .iter()
                .any(|previous| previous.action_hash == correction.action_hash)
            {
                return Err(CanonicalAccountantStateError::DuplicateCorrectionAction);
            }
            if correction.correction.state_hash != item.action_hash
                || correction.correction.private_receipt_commitment
                    != item.state.projection.private_receipt_commitment
            {
                return Err(CanonicalAccountantStateError::CorrectionLineageMismatch);
            }
        }
    }
    if correction_count != snapshot.observed_total_correction_count {
        return Err(CanonicalAccountantStateError::TotalCorrectionCountMismatch);
    }
    Ok(())
}

fn validate_lineage_anchor(
    lineage: &PopulationAccountantLineageAnchorV1,
) -> Result<(), CanonicalAccountantStateError> {
    PopulationAccountantLineageAnchorV1::new(
        lineage.release_policy_digest,
        lineage.accountant_instance_digest,
        lineage.accountant_method_digest,
    )
    .map_err(|_| CanonicalAccountantStateError::InvalidLineageAnchor)?;
    if lineage.schema_version != 1 {
        return Err(CanonicalAccountantStateError::InvalidLineageAnchor);
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CanonicalAccountantStateError {
    #[error("canonical accountant snapshot used an unexpected read boundary")]
    UnexpectedReadBoundary,
    #[error("canonical accountant snapshot observation window is invalid")]
    InvalidObservationWindow,
    #[error("canonical accountant snapshot state count mismatches its admitted states")]
    StateCountMismatch,
    #[error("canonical accountant snapshot contains duplicate state action")]
    DuplicateStateAction,
    #[error("canonical accountant lineage anchor is invalid")]
    InvalidLineageAnchor,
    #[error("canonical accountant state could not establish its lineage")]
    InvalidStateLineage,
    #[error("canonical accountant state crosses the requested lineage")]
    CrossLineageState,
    #[error("canonical accountant correction count mismatches materialized corrections")]
    CorrectionCountMismatch,
    #[error("canonical accountant correction count overflow")]
    CorrectionCountOverflow,
    #[error("canonical accountant snapshot contains duplicate correction action")]
    DuplicateCorrectionAction,
    #[error("canonical accountant correction crosses state lineage")]
    CorrectionLineageMismatch,
    #[error("canonical accountant total correction count mismatches snapshot metadata")]
    TotalCorrectionCountMismatch,
    #[error(transparent)]
    Reducer(#[from] PopulationAccountantStateError),
}
