#![deny(unsafe_code)]
//! Conflict-preserving read model for DNA-qualified privacy-accountant state.
//!
//! This reducer never chooses a winner by timestamp or insertion order. A stable
//! chain is current only within the supplied read set. Forks, missing predecessors,
//! mixed lineages, and semantically invalidating corrections fail closed.

use holo_hash::ActionHash;
use population_accountant_integrity::{
    PopulationAccountantCorrectionReason, PopulationAccountantStateCorrection,
    TrustedPopulationAccountantState,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_STATES: usize = 4096;
const MAX_CORRECTIONS_PER_STATE: usize = 256;

#[derive(Clone)]
pub struct ObservedPopulationAccountantStateV1 {
    pub action_hash: ActionHash,
    pub state: TrustedPopulationAccountantState,
    pub corrections: Vec<ObservedPopulationAccountantCorrectionV1>,
}

#[derive(Clone)]
pub struct ObservedPopulationAccountantCorrectionV1 {
    pub action_hash: ActionHash,
    pub correction: PopulationAccountantStateCorrection,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PopulationAccountantStateLifecycleV1 {
    Eligible,
    DuplicatePublicationSuppressed,
    Superseded,
    Invalidated,
    ReviewRequired,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PopulationAccountantStateLineageV1 {
    pub action_hash: ActionHash,
    pub sequence: u64,
    pub query_count: u64,
    pub lifecycle: PopulationAccountantStateLifecycleV1,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CanonicalPopulationAccountantStateV1 {
    NoTrustedStateObserved,
    ObservedCurrentWithinRead {
        action_hash: ActionHash,
        sequence: u64,
        query_count: u64,
    },
    ConflictWithinRead {
        competing_state_hashes: Vec<ActionHash>,
    },
    NoCurrentStateWithinRead {
        lineage: Vec<PopulationAccountantStateLineageV1>,
    },
    ReviewRequiredWithinRead {
        lineage: Vec<PopulationAccountantStateLineageV1>,
    },
}

pub fn reduce_population_accountant_state(
    observed: &[ObservedPopulationAccountantStateV1],
) -> Result<CanonicalPopulationAccountantStateV1, PopulationAccountantStateError> {
    if observed.is_empty() {
        return Ok(CanonicalPopulationAccountantStateV1::NoTrustedStateObserved);
    }
    if observed.len() > MAX_STATES {
        return Err(PopulationAccountantStateError::TooManyStates);
    }
    reject_duplicate_state_actions(observed)?;
    require_one_lineage(observed)?;
    require_predecessor_closure(observed)?;

    let mut lineage = Vec::with_capacity(observed.len());
    for item in observed {
        let lifecycle = lifecycle_for(item, observed)?;
        lineage.push(PopulationAccountantStateLineageV1 {
            action_hash: item.action_hash.clone(),
            sequence: item.state.projection.sequence,
            query_count: item.state.projection.query_count,
            lifecycle,
        });
    }
    lineage.sort_by(|left, right| {
        left.sequence
            .cmp(&right.sequence)
            .then_with(|| left.action_hash.get_raw_36().cmp(right.action_hash.get_raw_36()))
    });

    if lineage.iter().any(|item| {
        matches!(
            item.lifecycle,
            PopulationAccountantStateLifecycleV1::ReviewRequired
        )
    }) {
        return Ok(CanonicalPopulationAccountantStateV1::ReviewRequiredWithinRead {
            lineage,
        });
    }

    let active_indices: Vec<usize> = lineage
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            matches!(item.lifecycle, PopulationAccountantStateLifecycleV1::Eligible)
                .then_some(index)
        })
        .collect();

    if active_indices.is_empty() {
        return Ok(CanonicalPopulationAccountantStateV1::NoCurrentStateWithinRead {
            lineage,
        });
    }

    let live_actions: Vec<ActionHash> = active_indices
        .iter()
        .map(|index| lineage[*index].action_hash.clone())
        .collect();

    // More than one eligible genesis is an explicit fork.
    let eligible_genesis: Vec<ActionHash> = live_actions
        .iter()
        .filter_map(|hash| {
            let item = find_observed(observed, hash)?;
            item.state.projection.previous_state_hash.is_none().then_some(hash.clone())
        })
        .collect();
    if eligible_genesis.len() > 1 {
        return Ok(CanonicalPopulationAccountantStateV1::ConflictWithinRead {
            competing_state_hashes: sorted_hashes(eligible_genesis),
        });
    }

    // Any predecessor with multiple eligible children is a sibling-state fork.
    for parent_hash in &live_actions {
        let children: Vec<ActionHash> = live_actions
            .iter()
            .filter_map(|candidate_hash| {
                let candidate = find_observed(observed, candidate_hash)?;
                (candidate.state.projection.previous_state_hash.as_ref() == Some(parent_hash))
                    .then_some(candidate_hash.clone())
            })
            .collect();
        if children.len() > 1 {
            return Ok(CanonicalPopulationAccountantStateV1::ConflictWithinRead {
                competing_state_hashes: sorted_hashes(children),
            });
        }
    }

    // A current state is an eligible state that is not the predecessor of another
    // eligible state. More than one leaf means competing live branches.
    let leaves: Vec<ActionHash> = live_actions
        .iter()
        .filter(|candidate| {
            !live_actions.iter().any(|other| {
                find_observed(observed, other)
                    .and_then(|item| item.state.projection.previous_state_hash.as_ref())
                    == Some(*candidate)
            })
        })
        .cloned()
        .collect();

    if leaves.len() != 1 {
        return Ok(CanonicalPopulationAccountantStateV1::ConflictWithinRead {
            competing_state_hashes: sorted_hashes(leaves),
        });
    }

    let leaf = find_observed(observed, &leaves[0]).ok_or(
        PopulationAccountantStateError::InternalMissingState,
    )?;

    // A live leaf whose ancestry contains semantic invalidation cannot be upgraded
    // to trusted-current merely because the leaf itself lacks a correction.
    let mut cursor = leaf.state.projection.previous_state_hash.clone();
    while let Some(parent_hash) = cursor {
        let parent = find_observed(observed, &parent_hash).ok_or(
            PopulationAccountantStateError::MissingPredecessor,
        )?;
        match lifecycle_for(parent, observed)? {
            PopulationAccountantStateLifecycleV1::Eligible
            | PopulationAccountantStateLifecycleV1::DuplicatePublicationSuppressed => {}
            PopulationAccountantStateLifecycleV1::Superseded
            | PopulationAccountantStateLifecycleV1::Invalidated
            | PopulationAccountantStateLifecycleV1::ReviewRequired => {
                return Ok(CanonicalPopulationAccountantStateV1::ReviewRequiredWithinRead {
                    lineage,
                });
            }
        }
        cursor = parent.state.projection.previous_state_hash.clone();
    }

    Ok(CanonicalPopulationAccountantStateV1::ObservedCurrentWithinRead {
        action_hash: leaf.action_hash.clone(),
        sequence: leaf.state.projection.sequence,
        query_count: leaf.state.projection.query_count,
    })
}

fn lifecycle_for(
    item: &ObservedPopulationAccountantStateV1,
    all: &[ObservedPopulationAccountantStateV1],
) -> Result<PopulationAccountantStateLifecycleV1, PopulationAccountantStateError> {
    if item.corrections.len() > MAX_CORRECTIONS_PER_STATE {
        return Err(PopulationAccountantStateError::TooManyCorrections);
    }
    reject_duplicate_corrections(item)?;

    let mut saw_duplicate = false;
    let mut saw_superseded = false;
    let mut saw_review = false;
    let mut saw_invalidation = false;

    for observed in &item.corrections {
        let correction = &observed.correction;
        if correction.state_hash != item.action_hash
            || correction.private_receipt_commitment
                != item.state.projection.private_receipt_commitment
        {
            return Err(PopulationAccountantStateError::CorrectionLineageMismatch);
        }
        match correction.reason {
            PopulationAccountantCorrectionReason::EnteredInError
            | PopulationAccountantCorrectionReason::AccountantImplementationRevoked
            | PopulationAccountantCorrectionReason::AccountantMethodRevoked => {
                saw_invalidation = true;
            }
            PopulationAccountantCorrectionReason::Other => saw_review = true,
            PopulationAccountantCorrectionReason::PolicySuperseded => saw_superseded = true,
            PopulationAccountantCorrectionReason::DuplicateState => saw_duplicate = true,
        }
    }

    if saw_invalidation {
        return Ok(PopulationAccountantStateLifecycleV1::Invalidated);
    }
    if saw_review {
        return Ok(PopulationAccountantStateLifecycleV1::ReviewRequired);
    }
    if saw_superseded {
        return Ok(PopulationAccountantStateLifecycleV1::Superseded);
    }
    if saw_duplicate {
        let equivalents = all
            .iter()
            .filter(|candidate| {
                candidate.action_hash != item.action_hash
                    && candidate.state.projection.private_receipt_commitment
                        == item.state.projection.private_receipt_commitment
                    && candidate.state.projection == item.state.projection
            })
            .count();
        if equivalents == 0 {
            return Ok(PopulationAccountantStateLifecycleV1::ReviewRequired);
        }
        return Ok(PopulationAccountantStateLifecycleV1::DuplicatePublicationSuppressed);
    }
    Ok(PopulationAccountantStateLifecycleV1::Eligible)
}

fn require_one_lineage(
    observed: &[ObservedPopulationAccountantStateV1],
) -> Result<(), PopulationAccountantStateError> {
    let first = &observed[0].state.projection;
    for item in &observed[1..] {
        let current = &item.state.projection;
        if current.release_policy_digest != first.release_policy_digest
            || current.accountant_instance_digest != first.accountant_instance_digest
            || current.accountant_method_digest != first.accountant_method_digest
        {
            return Err(PopulationAccountantStateError::MixedAccountantLineage);
        }
    }
    Ok(())
}

fn require_predecessor_closure(
    observed: &[ObservedPopulationAccountantStateV1],
) -> Result<(), PopulationAccountantStateError> {
    for item in observed {
        if let Some(parent_hash) = &item.state.projection.previous_state_hash {
            let parent = find_observed(observed, parent_hash).ok_or(
                PopulationAccountantStateError::MissingPredecessor,
            )?;
            if parent.state.projection.sequence.checked_add(1)
                != Some(item.state.projection.sequence)
                || parent.state.projection.query_count.checked_add(1)
                    != Some(item.state.projection.query_count)
            {
                return Err(PopulationAccountantStateError::NonContiguousLineage);
            }
        }
    }
    Ok(())
}

fn reject_duplicate_state_actions(
    observed: &[ObservedPopulationAccountantStateV1],
) -> Result<(), PopulationAccountantStateError> {
    for (index, item) in observed.iter().enumerate() {
        if observed[..index]
            .iter()
            .any(|previous| previous.action_hash == item.action_hash)
        {
            return Err(PopulationAccountantStateError::DuplicateStateAction);
        }
    }
    Ok(())
}

fn reject_duplicate_corrections(
    item: &ObservedPopulationAccountantStateV1,
) -> Result<(), PopulationAccountantStateError> {
    for (index, correction) in item.corrections.iter().enumerate() {
        if item.corrections[..index].iter().any(|previous| {
            previous.action_hash == correction.action_hash
                || previous.correction.correction_id == correction.correction.correction_id
        }) {
            return Err(PopulationAccountantStateError::DuplicateCorrection);
        }
    }
    Ok(())
}

fn find_observed<'a>(
    observed: &'a [ObservedPopulationAccountantStateV1],
    action_hash: &ActionHash,
) -> Option<&'a ObservedPopulationAccountantStateV1> {
    observed.iter().find(|item| &item.action_hash == action_hash)
}

fn sorted_hashes(mut values: Vec<ActionHash>) -> Vec<ActionHash> {
    values.sort_by(|left, right| left.get_raw_36().cmp(right.get_raw_36()));
    values
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PopulationAccountantStateError {
    #[error("too many accountant states for one bounded reduction")]
    TooManyStates,
    #[error("too many corrections attached to one accountant state")]
    TooManyCorrections,
    #[error("duplicate accountant state action")]
    DuplicateStateAction,
    #[error("duplicate correction action or correction ID")]
    DuplicateCorrection,
    #[error("correction crosses accountant-state lineage")]
    CorrectionLineageMismatch,
    #[error("supplied accountant states mix policy/instance/method lineages")]
    MixedAccountantLineage,
    #[error("supplied accountant read set is missing a predecessor")]
    MissingPredecessor,
    #[error("accountant predecessor sequence/query lineage is not contiguous")]
    NonContiguousLineage,
    #[error("internal reducer lookup unexpectedly lost a state")]
    InternalMissingState,
}
