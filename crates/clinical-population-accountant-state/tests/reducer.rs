use holo_hash::ActionHash;
use mycelix_clinical_integrity::{DigestAlgorithm, DigestDomain, StoredDigest};
use mycelix_clinical_population_accountant_state::{
    reduce_population_accountant_state, CanonicalPopulationAccountantStateV1,
    ObservedPopulationAccountantCorrectionV1, ObservedPopulationAccountantStateV1,
    PopulationAccountantStateError,
};
use mycelix_clinical_population_release::{
    PopulationReleaseArtifactKindV1, PopulationReleaseDigestV1, PrivacyLossV1,
    ReducedFractionV1,
};
use population_accountant_integrity::{
    AccountantCommitmentSchemeV1, OpaqueAccountantReceiptCommitmentV1,
    PopulationAccountantCorrectionReason, PopulationAccountantStateCorrection,
    PopulationAccountantStateProjectionV1, TrustedPopulationAccountantState,
};

fn action(seed: u8) -> ActionHash {
    ActionHash::from_raw_36(vec![seed; 36])
}

fn stored(seed: u8) -> StoredDigest {
    StoredDigest {
        algorithm: DigestAlgorithm::Blake3_256,
        domain: DigestDomain::ClinicalArtifact,
        value: [seed; 32],
    }
}

fn policy() -> PopulationReleaseDigestV1 {
    PopulationReleaseDigestV1 {
        kind: PopulationReleaseArtifactKindV1::ReleasePolicy,
        value: [9; 32],
    }
}

fn loss(sequence: u64) -> PrivacyLossV1 {
    if sequence == 0 {
        PrivacyLossV1::ZERO
    } else {
        PrivacyLossV1 {
            epsilon: ReducedFractionV1 {
                numerator: sequence,
                denominator: 10,
            },
            delta: ReducedFractionV1::ZERO,
        }
    }
}

fn observed(seed: u8, sequence: u64, previous: Option<ActionHash>) -> ObservedPopulationAccountantStateV1 {
    ObservedPopulationAccountantStateV1 {
        action_hash: action(seed),
        state: TrustedPopulationAccountantState {
            projection: PopulationAccountantStateProjectionV1 {
                schema_version: 1,
                state_id: format!("state-{seed}"),
                release_policy_digest: policy(),
                accountant_instance_digest: stored(2),
                accountant_method_digest: stored(3),
                sequence,
                query_count: sequence,
                cumulative_privacy_loss: loss(sequence),
                previous_state_hash: previous,
                private_receipt_commitment: OpaqueAccountantReceiptCommitmentV1 {
                    scheme: AccountantCommitmentSchemeV1::Blake3Keyed,
                    value: [seed; 32],
                },
            },
            verifier_authorization_hash: action(seed.saturating_add(100)),
        },
        corrections: Vec::new(),
    }
}

fn correction(
    correction_seed: u8,
    item: &ObservedPopulationAccountantStateV1,
    reason: PopulationAccountantCorrectionReason,
) -> ObservedPopulationAccountantCorrectionV1 {
    ObservedPopulationAccountantCorrectionV1 {
        action_hash: action(correction_seed),
        correction: PopulationAccountantStateCorrection {
            correction_id: format!("correction-{correction_seed}"),
            state_hash: item.action_hash.clone(),
            private_receipt_commitment: item.state.projection.private_receipt_commitment,
            reason,
            rationale_commitment: None,
        },
    }
}

#[test]
fn one_genesis_is_observed_current_within_read() {
    let states = vec![observed(1, 0, None)];
    let result = reduce_population_accountant_state(&states).unwrap();
    assert!(matches!(
        result,
        CanonicalPopulationAccountantStateV1::ObservedCurrentWithinRead { sequence: 0, .. }
    ));
}

#[test]
fn two_genesis_states_are_conflict() {
    let states = vec![observed(1, 0, None), observed(2, 0, None)];
    let result = reduce_population_accountant_state(&states).unwrap();
    assert!(matches!(
        result,
        CanonicalPopulationAccountantStateV1::ConflictWithinRead { .. }
    ));
}

#[test]
fn sibling_successors_are_conflict() {
    let root = observed(1, 0, None);
    let root_hash = root.action_hash.clone();
    let states = vec![
        root,
        observed(2, 1, Some(root_hash.clone())),
        observed(3, 1, Some(root_hash)),
    ];
    let result = reduce_population_accountant_state(&states).unwrap();
    assert!(matches!(
        result,
        CanonicalPopulationAccountantStateV1::ConflictWithinRead { .. }
    ));
}

#[test]
fn missing_predecessor_fails_closed() {
    let states = vec![observed(2, 1, Some(action(1)))];
    let result = reduce_population_accountant_state(&states);
    assert!(matches!(result, Err(PopulationAccountantStateError::MissingPredecessor)));
}

#[test]
fn invalidated_ancestor_contaminates_descendant_current_claim() {
    let mut root = observed(1, 0, None);
    root.corrections.push(correction(
        20,
        &root,
        PopulationAccountantCorrectionReason::AccountantImplementationRevoked,
    ));
    let child = observed(2, 1, Some(root.action_hash.clone()));
    let result = reduce_population_accountant_state(&[root, child]).unwrap();
    assert!(matches!(
        result,
        CanonicalPopulationAccountantStateV1::ReviewRequiredWithinRead { .. }
    ));
}

#[test]
fn clean_contiguous_chain_returns_leaf() {
    let root = observed(1, 0, None);
    let child = observed(2, 1, Some(root.action_hash.clone()));
    let child_hash = child.action_hash.clone();
    let result = reduce_population_accountant_state(&[root, child]).unwrap();
    match result {
        CanonicalPopulationAccountantStateV1::ObservedCurrentWithinRead {
            action_hash,
            sequence,
            query_count,
        } => {
            assert_eq!(action_hash, child_hash);
            assert_eq!(sequence, 1);
            assert_eq!(query_count, 1);
        }
        _ => panic!("expected current child state"),
    }
}
