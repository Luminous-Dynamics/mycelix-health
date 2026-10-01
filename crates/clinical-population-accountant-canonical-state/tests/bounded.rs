use hdk::prelude::Timestamp;
use holo_hash::ActionHash;
use mycelix_clinical_integrity::{DigestAlgorithm, DigestDomain, StoredDigest};
use mycelix_clinical_population_accountant_canonical_state::{
    reduce_canonical_population_accountant_snapshot,
    BoundedCanonicalPopulationAccountantStateV1,
};
use mycelix_clinical_population_release::{
    PopulationReleaseArtifactKindV1, PopulationReleaseDigestV1, PrivacyLossV1,
};
use population_accountant_index::{
    CanonicalAccountantObservedStateV1, CanonicalAccountantReadBoundaryV1,
    CanonicalPopulationAccountantLineageSnapshotV1,
};
use population_accountant_index_integrity::PopulationAccountantLineageAnchorV1;
use population_accountant_integrity::{
    AccountantCommitmentSchemeV1, OpaqueAccountantReceiptCommitmentV1,
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

fn lineage() -> PopulationAccountantLineageAnchorV1 {
    PopulationAccountantLineageAnchorV1::new(policy(), stored(2), stored(3)).unwrap()
}

fn admitted(seed: u8) -> CanonicalAccountantObservedStateV1 {
    CanonicalAccountantObservedStateV1 {
        action_hash: action(seed),
        state: TrustedPopulationAccountantState {
            projection: PopulationAccountantStateProjectionV1 {
                schema_version: 1,
                state_id: format!("state-{seed}"),
                release_policy_digest: policy(),
                accountant_instance_digest: stored(2),
                accountant_method_digest: stored(3),
                sequence: 0,
                query_count: 0,
                cumulative_privacy_loss: PrivacyLossV1::ZERO,
                previous_state_hash: None,
                private_receipt_commitment: OpaqueAccountantReceiptCommitmentV1 {
                    scheme: AccountantCommitmentSchemeV1::Blake3Keyed,
                    value: [seed; 32],
                },
            },
            verifier_authorization_hash: action(seed.saturating_add(100)),
        },
        corrections: Vec::new(),
        observed_correction_target_count: 0,
    }
}

fn snapshot(states: Vec<CanonicalAccountantObservedStateV1>) -> CanonicalPopulationAccountantLineageSnapshotV1 {
    CanonicalPopulationAccountantLineageSnapshotV1 {
        lineage: lineage(),
        observed_state_count: states.len(),
        observed_total_correction_count: 0,
        admitted_states: states,
        read_boundary: CanonicalAccountantReadBoundaryV1::NestedNetworkBackedStableReadsWithFinalClosureChecks,
        observation_started_at: Timestamp::from_micros(10),
        observation_completed_at: Timestamp::from_micros(20),
    }
}

#[test]
fn empty_canonical_snapshot_is_not_an_invented_budget_state() {
    let result = reduce_canonical_population_accountant_snapshot(&snapshot(Vec::new())).unwrap();
    assert!(matches!(
        result,
        BoundedCanonicalPopulationAccountantStateV1::NoCanonicalStateObserved
    ));
}

#[test]
fn one_admitted_genesis_is_only_current_within_bounded_canonical_read() {
    let result = reduce_canonical_population_accountant_snapshot(&snapshot(vec![admitted(1)])).unwrap();
    assert!(matches!(
        result,
        BoundedCanonicalPopulationAccountantStateV1::ObservedCurrentWithinBoundedCanonicalRead {
            sequence: 0,
            query_count: 0,
            ..
        }
    ));
}

#[test]
fn two_admitted_genesis_states_are_conflict_not_latest_wins() {
    let result = reduce_canonical_population_accountant_snapshot(&snapshot(vec![admitted(1), admitted(2)])).unwrap();
    assert!(matches!(
        result,
        BoundedCanonicalPopulationAccountantStateV1::ConflictWithinBoundedCanonicalRead { .. }
    ));
}

#[test]
fn snapshot_count_mismatch_fails_closed() {
    let mut value = snapshot(vec![admitted(1)]);
    value.observed_state_count = 2;
    assert!(reduce_canonical_population_accountant_snapshot(&value).is_err());
}

#[test]
fn cross_lineage_state_fails_closed() {
    let mut value = snapshot(vec![admitted(1)]);
    value.admitted_states[0].state.projection.accountant_method_digest = stored(7);
    assert!(reduce_canonical_population_accountant_snapshot(&value).is_err());
}
