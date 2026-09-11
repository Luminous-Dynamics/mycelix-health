use holo_hash::ActionHash;
use holochain_zome_types::prelude::Timestamp;
use mycelix_clinical_integrity::{DigestAlgorithm, DigestDomain, StoredDigest};
use mycelix_clinical_population_interactive_release::*;
use mycelix_clinical_population_release::{
    DifferentialPrivacyMechanismV1, DpMechanismKindV1, InteractiveDpReleaseRequestV1,
    PopulationFindingKindV1, PopulationReleaseArtifactKindV1, PopulationReleaseDigestV1,
    PopulationReleaseModeV1, PopulationReleasePolicyV1, PopulationReleaseRequestV1,
    PrivacyAccountantReceiptV1, PrivacyLossV1, ReducedFractionV1,
};
use population_accountant_index::{
    CanonicalAccountantObservedStateV1, CanonicalAccountantReadBoundaryV1,
    CanonicalPopulationAccountantLineageSnapshotV1,
};
use population_accountant_index_integrity::PopulationAccountantLineageAnchorV1;
use population_accountant_integrity::{
    AccountantCommitmentSchemeV1, PopulationAccountantStateProjectionV1,
    TrustedPopulationAccountantState,
};

fn action(seed: u8) -> ActionHash {
    ActionHash::from_raw_36(vec![seed; 36])
}

fn digest(domain: DigestDomain, seed: u8) -> StoredDigest {
    StoredDigest {
        algorithm: DigestAlgorithm::Blake3_256,
        domain,
        value: [seed; 32],
    }
}

fn fraction(n: u64, d: u64) -> ReducedFractionV1 {
    ReducedFractionV1 {
        numerator: n,
        denominator: d,
    }
}

fn loss(n: u64, d: u64) -> PrivacyLossV1 {
    PrivacyLossV1 {
        epsilon: fraction(n, d),
        delta: ReducedFractionV1::ZERO,
    }
}

fn policy() -> PopulationReleasePolicyV1 {
    PopulationReleasePolicyV1 {
        schema_version: 1,
        policy_id: "interactive-release-policy-v1".into(),
        static_minimum_cell_count: 11,
        max_static_report_cells: 10_000,
        allowed_dp_mechanisms: vec![DpMechanismKindV1::Gaussian],
        max_per_release_privacy_loss: loss(1, 2),
        max_cumulative_privacy_loss: loss(2, 1),
        max_interactive_queries: 100,
        accountant_instance_digest: digest(DigestDomain::ClinicalArtifact, 1),
        accountant_method_digest: digest(DigestDomain::ClinicalArtifact, 2),
    }
}

fn mechanism(policy: &PopulationReleasePolicyV1) -> DifferentialPrivacyMechanismV1 {
    let value = DifferentialPrivacyMechanismV1 {
        schema_version: 1,
        mechanism_id: "gaussian-count-v1".into(),
        mechanism_kind: DpMechanismKindV1::Gaussian,
        implementation_version: "1.0.0".into(),
        implementation_digest: digest(DigestDomain::ClinicalArtifact, 10),
        privacy_unit_definition_digest: digest(DigestDomain::ClinicalArtifact, 11),
        adjacency_definition_digest: digest(DigestDomain::ClinicalArtifact, 12),
        contribution_bounds_digest: digest(DigestDomain::ClinicalArtifact, 13),
        sensitivity_analysis_digest: digest(DigestDomain::ClinicalArtifact, 14),
        randomness_source_evidence_digest: digest(DigestDomain::ClinicalArtifact, 15),
        release_privacy_loss: loss(1, 10),
    };
    value.validate(policy).unwrap();
    value
}

struct Fixture {
    request: PopulationReleaseRequestV1,
    snapshot: CanonicalPopulationAccountantLineageSnapshotV1,
    key: AccountantReceiptCommitmentKeyV1,
}

fn fixture() -> Fixture {
    let policy = policy();
    let policy_digest = policy.verified_digest().unwrap();
    let mechanism = mechanism(&policy);
    let mechanism_digest = mechanism.verified_digest(&policy).unwrap();
    let query = digest(DigestDomain::ClinicalArtifact, 20);
    let output = digest(DigestDomain::ClinicalArtifact, 21);

    let before_receipt = PrivacyAccountantReceiptV1 {
        schema_version: 1,
        accountant_instance_digest: policy.accountant_instance_digest,
        accountant_method_digest: policy.accountant_method_digest,
        sequence: 0,
        previous_receipt_digest: None,
        cumulative_privacy_loss: PrivacyLossV1::ZERO,
        query_count: 0,
        last_query_spec_digest: None,
        last_mechanism_digest: None,
        last_output_digest: None,
        accountant_evidence_digest: digest(DigestDomain::ClinicalArtifact, 30),
        observed_at_micros: 100,
    };
    let after_receipt = PrivacyAccountantReceiptV1 {
        schema_version: 1,
        accountant_instance_digest: policy.accountant_instance_digest,
        accountant_method_digest: policy.accountant_method_digest,
        sequence: 1,
        previous_receipt_digest: Some(before_receipt.verified_digest(&policy).unwrap()),
        cumulative_privacy_loss: loss(1, 10),
        query_count: 1,
        last_query_spec_digest: Some(query),
        last_mechanism_digest: Some(mechanism_digest),
        last_output_digest: Some(output),
        accountant_evidence_digest: digest(DigestDomain::ClinicalArtifact, 31),
        observed_at_micros: 110,
    };

    let key = AccountantReceiptCommitmentKeyV1::new([7; 32]).unwrap();
    let before_commitment = commit_private_accountant_receipt(
        &before_receipt,
        AccountantCommitmentSchemeV1::Blake3Keyed,
        &key,
    )
    .unwrap();
    let after_commitment = commit_private_accountant_receipt(
        &after_receipt,
        AccountantCommitmentSchemeV1::Blake3Keyed,
        &key,
    )
    .unwrap();

    let before_hash = action(1);
    let after_hash = action(2);
    let before_state = TrustedPopulationAccountantState {
        projection: PopulationAccountantStateProjectionV1 {
            schema_version: 1,
            state_id: "accountant-state-0".into(),
            release_policy_digest: policy_digest,
            accountant_instance_digest: policy.accountant_instance_digest,
            accountant_method_digest: policy.accountant_method_digest,
            sequence: 0,
            query_count: 0,
            cumulative_privacy_loss: PrivacyLossV1::ZERO,
            previous_state_hash: None,
            private_receipt_commitment: before_commitment,
        },
        verifier_authorization_hash: action(101),
    };
    let after_state = TrustedPopulationAccountantState {
        projection: PopulationAccountantStateProjectionV1 {
            schema_version: 1,
            state_id: "accountant-state-1".into(),
            release_policy_digest: policy_digest,
            accountant_instance_digest: policy.accountant_instance_digest,
            accountant_method_digest: policy.accountant_method_digest,
            sequence: 1,
            query_count: 1,
            cumulative_privacy_loss: loss(1, 10),
            previous_state_hash: Some(before_hash.clone()),
            private_receipt_commitment: after_commitment,
        },
        verifier_authorization_hash: action(102),
    };

    let request = PopulationReleaseRequestV1 {
        schema_version: 1,
        release_id: "trusted-interactive-release-1".into(),
        policy: policy.clone(),
        mode: PopulationReleaseModeV1::InteractiveDifferentialPrivacy(
            InteractiveDpReleaseRequestV1 {
                source_finding_kind: PopulationFindingKindV1::CausalEffectEstimate,
                source_finding_digest: digest(DigestDomain::ClinicalCausalEffectEstimate, 40),
                query_specification_digest: query,
                output_schema_digest: digest(DigestDomain::ClinicalArtifact, 41),
                public_output_digest: output,
                mechanism,
                accountant_before: before_receipt,
                accountant_after: after_receipt,
            },
        ),
        requested_at_micros: 120,
    };

    let lineage = PopulationAccountantLineageAnchorV1::new(
        policy_digest,
        policy.accountant_instance_digest,
        policy.accountant_method_digest,
    )
    .unwrap();
    let states = vec![
        CanonicalAccountantObservedStateV1 {
            action_hash: before_hash,
            state: before_state,
            corrections: Vec::new(),
            observed_correction_target_count: 0,
        },
        CanonicalAccountantObservedStateV1 {
            action_hash: after_hash,
            state: after_state,
            corrections: Vec::new(),
            observed_correction_target_count: 0,
        },
    ];
    let snapshot = CanonicalPopulationAccountantLineageSnapshotV1 {
        lineage,
        observed_state_count: states.len(),
        observed_total_correction_count: 0,
        admitted_states: states,
        read_boundary:
            CanonicalAccountantReadBoundaryV1::NestedNetworkBackedStableReadsWithFinalClosureChecks,
        observation_started_at: Timestamp::from_micros(115),
        observation_completed_at: Timestamp::from_micros(130),
    };

    Fixture {
        request,
        snapshot,
        key,
    }
}

#[test]
fn exact_canonical_transition_mints_distinct_trusted_capability() {
    let fixture = fixture();
    let receipt = authorize_trusted_interactive_population_release(
        &fixture.request,
        &fixture.snapshot,
        &fixture.key,
    )
    .unwrap()
    .into_receipt(140)
    .unwrap();
    assert_eq!(receipt.accountant_sequence, 1);
    assert_eq!(receipt.accountant_query_count, 1);
    assert_eq!(receipt.accountant_before_state_hash, action(1));
    assert_eq!(receipt.accountant_after_state_hash, action(2));
    assert_ne!(receipt.verified_digest().unwrap().value, [0; 32]);
}

#[test]
fn wrong_commitment_key_fails_closed() {
    let fixture = fixture();
    let wrong = AccountantReceiptCommitmentKeyV1::new([8; 32]).unwrap();
    assert!(matches!(
        authorize_trusted_interactive_population_release(
            &fixture.request,
            &fixture.snapshot,
            &wrong,
        ),
        Err(TrustedInteractiveReleaseError::PrivateReceiptCommitmentMismatch)
    ));
}

#[test]
fn admitted_sibling_successor_is_conflict_not_latest_wins() {
    let mut fixture = fixture();
    let after = fixture.snapshot.admitted_states[1].clone();
    let mut sibling = after;
    sibling.action_hash = action(3);
    sibling.state.projection.state_id = "accountant-state-1-sibling".into();
    sibling.state.projection.private_receipt_commitment.value = [55; 32];
    fixture.snapshot.admitted_states.push(sibling);
    fixture.snapshot.observed_state_count = 3;

    assert!(matches!(
        authorize_trusted_interactive_population_release(
            &fixture.request,
            &fixture.snapshot,
            &fixture.key,
        ),
        Err(TrustedInteractiveReleaseError::CanonicalAccountantConflict)
    ));
}

#[test]
fn stale_canonical_snapshot_cannot_authorize_release() {
    let mut fixture = fixture();
    fixture.snapshot.observation_completed_at =
        Timestamp::from_micros(110 + MAX_CANONICAL_READ_LAG_MICROS + 1);
    assert!(matches!(
        authorize_trusted_interactive_population_release(
            &fixture.request,
            &fixture.snapshot,
            &fixture.key,
        ),
        Err(TrustedInteractiveReleaseError::CanonicalSnapshotTooStale)
    ));
}

#[test]
fn different_policy_lineage_fails_before_release_authority() {
    let mut fixture = fixture();
    fixture.snapshot.lineage.release_policy_digest = PopulationReleaseDigestV1 {
        kind: PopulationReleaseArtifactKindV1::ReleasePolicy,
        value: [99; 32],
    };
    assert!(matches!(
        authorize_trusted_interactive_population_release(
            &fixture.request,
            &fixture.snapshot,
            &fixture.key,
        ),
        Err(TrustedInteractiveReleaseError::ReleasePolicyLineageMismatch)
    ));
}

#[test]
fn hmac_and_blake3_commitments_are_domain_separated_by_scheme() {
    let fixture = fixture();
    let PopulationReleaseModeV1::InteractiveDifferentialPrivacy(dp) = &fixture.request.mode else {
        unreachable!()
    };
    let hmac = commit_private_accountant_receipt(
        &dp.accountant_after,
        AccountantCommitmentSchemeV1::HmacSha256,
        &fixture.key,
    )
    .unwrap();
    let blake = commit_private_accountant_receipt(
        &dp.accountant_after,
        AccountantCommitmentSchemeV1::Blake3Keyed,
        &fixture.key,
    )
    .unwrap();
    assert_ne!(hmac.value, blake.value);
}
