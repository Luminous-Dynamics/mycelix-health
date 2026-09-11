use clinical_causality_index::{
    CanonicalCausalCommitmentReadBoundaryV1, CanonicalCausalCommitmentSnapshotV1,
    CanonicalCausalPublicationSnapshotV1, CanonicalCorrectionReadBoundaryV1,
};
use clinical_causality_index_integrity::{
    commitment_index_anchor_hash, OpaqueCausalCommitmentV1,
};
use clinical_causality_integrity::{
    CausalCommitmentSchemeV1, QualifiedCausalAssessmentAttestation,
    QualifiedCausalAssessmentAttestationV1,
};
use holo_hash::ActionHash;
use holochain_zome_types::prelude::Timestamp;
use mycelix_clinical_causality::CausalConclusionV1;
use mycelix_clinical_causality_canonical_state::{
    reduce_canonical_causal_snapshot, BoundedCanonicalCausalStateV1,
    CanonicalCausalStateError,
};
use mycelix_clinical_integrity::{DigestAlgorithm, DigestDomain, StoredDigest};

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

fn commitment(seed: u8) -> OpaqueCausalCommitmentV1 {
    OpaqueCausalCommitmentV1 {
        scheme: CausalCommitmentSchemeV1::Blake3Keyed,
        value: [seed; 32],
    }
}

fn publication(
    action_seed: u8,
    receipt_seed: u8,
    conclusion: CausalConclusionV1,
) -> CanonicalCausalPublicationSnapshotV1 {
    CanonicalCausalPublicationSnapshotV1 {
        attestation_action_hash: action(action_seed),
        attestation: QualifiedCausalAssessmentAttestation {
            attestation: QualifiedCausalAssessmentAttestationV1 {
                schema_version: 1,
                attestation_id: format!("attestation-{action_seed}"),
                qualified_receipt_commitment: commitment(receipt_seed),
                qualification_policy_digest: digest(
                    DigestDomain::ClinicalCausalQualificationPolicy,
                    42,
                ),
                conclusion,
            },
            verifier_authorization_hash: action(200),
        },
        corrections: Vec::new(),
        correction_read_boundary:
            CanonicalCorrectionReadBoundaryV1::NetworkBackedStableDoubleReadWithFinalClosureCheck,
        observed_correction_target_count: 0,
        correction_observation_started_at: Timestamp::from_micros(20),
        correction_observation_completed_at: Timestamp::from_micros(30),
    }
}

fn snapshot(
    publications: Vec<CanonicalCausalPublicationSnapshotV1>,
) -> CanonicalCausalCommitmentSnapshotV1 {
    let commitment = commitment(10);
    let total_corrections = publications
        .iter()
        .map(|publication| publication.corrections.len())
        .sum();
    CanonicalCausalCommitmentSnapshotV1 {
        commitment,
        anchor_hash: commitment_index_anchor_hash(commitment).unwrap(),
        observed_attestation_target_count: publications.len(),
        observed_total_correction_target_count: total_corrections,
        publications,
        read_boundary:
            CanonicalCausalCommitmentReadBoundaryV1::NestedNetworkBackedStableReadsWithFinalClosureChecks,
        observation_started_at: Timestamp::from_micros(10),
        observation_completed_at: Timestamp::from_micros(40),
    }
}

#[test]
fn empty_index_is_explicitly_no_canonical_publication_observed() {
    let view = reduce_canonical_causal_snapshot(&snapshot(Vec::new())).unwrap();
    assert!(matches!(
        view.state,
        BoundedCanonicalCausalStateV1::NoCanonicalPublicationObserved
    ));
}

#[test]
fn reducer_current_is_renamed_to_bounded_observation() {
    let input = snapshot(vec![publication(1, 10, CausalConclusionV1::Indeterminate)]);
    let view = reduce_canonical_causal_snapshot(&input).unwrap();
    assert!(matches!(
        view.state,
        BoundedCanonicalCausalStateV1::ObservedCurrentWithinBoundedRead { .. }
    ));
}

#[test]
fn conflicting_canonical_conclusions_are_preserved() {
    let input = snapshot(vec![
        publication(1, 10, CausalConclusionV1::EvidenceSuggestsRelationship),
        publication(2, 10, CausalConclusionV1::EvidenceAgainstRelationship),
    ]);
    let view = reduce_canonical_causal_snapshot(&input).unwrap();
    assert!(matches!(
        view.state,
        BoundedCanonicalCausalStateV1::ProjectionConflictWithinBoundedRead { .. }
    ));
}

#[test]
fn cross_commitment_publication_fails_closed() {
    let input = snapshot(vec![publication(1, 11, CausalConclusionV1::Indeterminate)]);
    assert!(matches!(
        reduce_canonical_causal_snapshot(&input),
        Err(CanonicalCausalStateError::CrossCommitmentPublication)
    ));
}

#[test]
fn duplicate_publication_action_fails_closed() {
    let a = publication(1, 10, CausalConclusionV1::Indeterminate);
    let mut b = publication(2, 10, CausalConclusionV1::Indeterminate);
    b.attestation_action_hash = a.attestation_action_hash.clone();
    let input = snapshot(vec![a, b]);
    assert!(matches!(
        reduce_canonical_causal_snapshot(&input),
        Err(CanonicalCausalStateError::DuplicatePublicationAction)
    ));
}

#[test]
fn publication_count_mismatch_fails_closed() {
    let mut input = snapshot(vec![publication(1, 10, CausalConclusionV1::Indeterminate)]);
    input.observed_attestation_target_count = 2;
    assert!(matches!(
        reduce_canonical_causal_snapshot(&input),
        Err(CanonicalCausalStateError::PublicationCountMismatch)
    ));
}

#[test]
fn correction_window_must_stay_inside_global_observation_window() {
    let mut p = publication(1, 10, CausalConclusionV1::Indeterminate);
    p.correction_observation_completed_at = Timestamp::from_micros(50);
    let input = snapshot(vec![p]);
    assert!(matches!(
        reduce_canonical_causal_snapshot(&input),
        Err(CanonicalCausalStateError::InvalidObservationWindow)
    ));
}

#[test]
fn commitment_anchor_mismatch_fails_closed() {
    let mut input = snapshot(vec![publication(1, 10, CausalConclusionV1::Indeterminate)]);
    input.anchor_hash = commitment_index_anchor_hash(commitment(11)).unwrap();
    assert!(matches!(
        reduce_canonical_causal_snapshot(&input),
        Err(CanonicalCausalStateError::AnchorHashMismatch)
    ));
}
