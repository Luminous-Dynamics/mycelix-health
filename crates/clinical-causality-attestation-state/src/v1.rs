use clinical_causality_integrity::{
    CausalAttestationCorrection, CausalAttestationCorrectionReason, CausalCommitmentSchemeV1,
    OpaqueCausalCommitmentV1, QualifiedCausalAssessmentAttestation,
    QualifiedCausalAssessmentAttestationV1,
};
use holo_hash::ActionHash;
use mycelix_clinical_causality::CausalConclusionV1;
use mycelix_clinical_integrity::{DigestDomain, StoredDigest};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

const MAX_ID_LEN: usize = 128;
type CommitmentKey = (u8, [u8; 32]);

#[derive(Clone)]
pub struct CausalAttestationPublicationRecord {
    pub action_hash: ActionHash,
    pub attestation: QualifiedCausalAssessmentAttestation,
}

#[derive(Clone)]
pub struct CausalAttestationCorrectionRecord {
    pub action_hash: ActionHash,
    pub correction: CausalAttestationCorrection,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CausalCorrectionView {
    pub correction_action_hash: ActionHash,
    pub target_attestation_action_hash: ActionHash,
    pub reason: CausalAttestationCorrectionReason,
    pub rationale_commitment: Option<OpaqueCausalCommitmentV1>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CausalProjectionView {
    pub qualification_policy_digest: StoredDigest,
    pub conclusion: CausalConclusionV1,
    pub publication_action_hashes: Vec<ActionHash>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum CausalAttestationLifecycle {
    Current {
        projection: CausalProjectionView,
    },
    ProjectionConflict {
        projections: Vec<CausalProjectionView>,
    },
    Superseded {
        corrections: Vec<CausalCorrectionView>,
    },
    Invalidated {
        corrections: Vec<CausalCorrectionView>,
    },
    ReviewRequired {
        corrections: Vec<CausalCorrectionView>,
    },
    PublicationSuppressed {
        corrections: Vec<CausalCorrectionView>,
    },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CausalAttestationLineageView {
    pub qualified_receipt_commitment: OpaqueCausalCommitmentV1,
    pub publication_action_hashes: Vec<ActionHash>,
    pub effective_publication_action_hashes: Vec<ActionHash>,
    pub corrections: Vec<CausalCorrectionView>,
    pub lifecycle: CausalAttestationLifecycle,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ClinicalCausalityAttestationView {
    pub lineages: Vec<CausalAttestationLineageView>,
}

#[derive(Clone)]
struct PublicationGroup {
    commitment: OpaqueCausalCommitmentV1,
    publications: Vec<(ActionHash, QualifiedCausalAssessmentAttestationV1)>,
    duplicate_suppressed_actions: HashSet<ActionHash>,
    corrections: Vec<CausalCorrectionView>,
}

/// Reduce a complete-for-purpose set of already DHT-valid causal attestations and
/// corrections. Reference closure is enforced inside the supplied set, but this
/// function cannot prove that an omitted correction does not exist elsewhere.
pub fn reduce_causal_attestation_state(
    publications: &[CausalAttestationPublicationRecord],
    corrections: &[CausalAttestationCorrectionRecord],
) -> Result<ClinicalCausalityAttestationView, CausalAttestationStateError> {
    let mut publication_payload_by_action: HashMap<
        ActionHash,
        QualifiedCausalAssessmentAttestationV1,
    > = HashMap::new();
    let mut publication_commitment_by_action: HashMap<ActionHash, CommitmentKey> = HashMap::new();
    let mut groups: HashMap<CommitmentKey, PublicationGroup> = HashMap::new();

    for record in publications {
        let attestation = &record.attestation.attestation;
        validate_attestation_shape(attestation)?;

        if let Some(existing) = publication_payload_by_action.get(&record.action_hash) {
            if existing != attestation {
                return Err(CausalAttestationStateError::ActionHashPayloadConflict);
            }
            continue;
        }

        let key = commitment_key(attestation.qualified_receipt_commitment);
        publication_payload_by_action.insert(record.action_hash.clone(), attestation.clone());
        publication_commitment_by_action.insert(record.action_hash.clone(), key);
        groups
            .entry(key)
            .or_insert_with(|| PublicationGroup {
                commitment: attestation.qualified_receipt_commitment,
                publications: Vec::new(),
                duplicate_suppressed_actions: HashSet::new(),
                corrections: Vec::new(),
            })
            .publications
            .push((record.action_hash.clone(), attestation.clone()));
    }

    let mut correction_payload_by_action: HashMap<ActionHash, CausalAttestationCorrection> =
        HashMap::new();
    let mut correction_id_owner: HashMap<String, ActionHash> = HashMap::new();

    for record in corrections {
        validate_correction_shape(&record.correction)?;

        if let Some(existing) = correction_payload_by_action.get(&record.action_hash) {
            if existing != &record.correction {
                return Err(CausalAttestationStateError::CorrectionActionHashPayloadConflict);
            }
            continue;
        }
        correction_payload_by_action.insert(record.action_hash.clone(), record.correction.clone());

        match correction_id_owner.get(&record.correction.correction_id) {
            Some(existing_action) if existing_action != &record.action_hash => {
                return Err(CausalAttestationStateError::DuplicateCorrectionId);
            }
            Some(_) => {}
            None => {
                correction_id_owner.insert(
                    record.correction.correction_id.clone(),
                    record.action_hash.clone(),
                );
            }
        }

        let target_key = publication_commitment_by_action
            .get(&record.correction.attestation_hash)
            .copied()
            .ok_or(CausalAttestationStateError::OrphanCorrection)?;
        if target_key != commitment_key(record.correction.qualified_receipt_commitment) {
            return Err(CausalAttestationStateError::CorrectionCommitmentMismatch);
        }

        let group = groups
            .get_mut(&target_key)
            .ok_or(CausalAttestationStateError::OrphanCorrection)?;
        let view = CausalCorrectionView {
            correction_action_hash: record.action_hash.clone(),
            target_attestation_action_hash: record.correction.attestation_hash.clone(),
            reason: record.correction.reason,
            rationale_commitment: record.correction.rationale_commitment,
        };
        if record.correction.reason == CausalAttestationCorrectionReason::DuplicateAttestation {
            group
                .duplicate_suppressed_actions
                .insert(record.correction.attestation_hash.clone());
        }
        group.corrections.push(view);
    }

    let mut lineages = Vec::with_capacity(groups.len());
    for (_, mut group) in groups {
        group
            .publications
            .sort_by(|left, right| action_hash_order(&left.0, &right.0));
        group.corrections.sort_by(correction_order);

        let publication_action_hashes: Vec<_> = group
            .publications
            .iter()
            .map(|(hash, _)| hash.clone())
            .collect();
        let effective_publications: Vec<_> = group
            .publications
            .iter()
            .filter(|(hash, _)| !group.duplicate_suppressed_actions.contains(hash))
            .cloned()
            .collect();
        let effective_publication_action_hashes: Vec<_> = effective_publications
            .iter()
            .map(|(hash, _)| hash.clone())
            .collect();

        let invalidating: Vec<_> = group
            .corrections
            .iter()
            .filter(|correction| is_invalidating(correction.reason))
            .cloned()
            .collect();
        let unknown_effect: Vec<_> = group
            .corrections
            .iter()
            .filter(|correction| correction.reason == CausalAttestationCorrectionReason::Other)
            .cloned()
            .collect();
        let superseding: Vec<_> = group
            .corrections
            .iter()
            .filter(|correction| {
                correction.reason == CausalAttestationCorrectionReason::PolicySuperseded
            })
            .cloned()
            .collect();

        // Strongest known semantic effect wins; all correction evidence remains in
        // `corrections`, so precedence never erases the lower-priority history.
        let lifecycle = if !invalidating.is_empty() {
            CausalAttestationLifecycle::Invalidated {
                corrections: invalidating,
            }
        } else if !unknown_effect.is_empty() {
            CausalAttestationLifecycle::ReviewRequired {
                corrections: unknown_effect,
            }
        } else if !superseding.is_empty() {
            CausalAttestationLifecycle::Superseded {
                corrections: superseding,
            }
        } else if effective_publications.is_empty() {
            CausalAttestationLifecycle::PublicationSuppressed {
                corrections: group
                    .corrections
                    .iter()
                    .filter(|correction| {
                        correction.reason == CausalAttestationCorrectionReason::DuplicateAttestation
                    })
                    .cloned()
                    .collect(),
            }
        } else {
            let projections = build_projections(&effective_publications);
            match projections.as_slice() {
                [one] => CausalAttestationLifecycle::Current {
                    projection: one.clone(),
                },
                _ => CausalAttestationLifecycle::ProjectionConflict { projections },
            }
        };

        lineages.push(CausalAttestationLineageView {
            qualified_receipt_commitment: group.commitment,
            publication_action_hashes,
            effective_publication_action_hashes,
            corrections: group.corrections,
            lifecycle,
        });
    }

    lineages.sort_by(|left, right| {
        commitment_key(left.qualified_receipt_commitment)
            .cmp(&commitment_key(right.qualified_receipt_commitment))
    });

    Ok(ClinicalCausalityAttestationView { lineages })
}

fn build_projections(
    publications: &[(ActionHash, QualifiedCausalAssessmentAttestationV1)],
) -> Vec<CausalProjectionView> {
    let mut groups: HashMap<(StoredDigest, u8), CausalProjectionView> = HashMap::new();
    for (action_hash, attestation) in publications {
        let key = (
            attestation.qualification_policy_digest,
            conclusion_rank(attestation.conclusion),
        );
        groups
            .entry(key)
            .or_insert_with(|| CausalProjectionView {
                qualification_policy_digest: attestation.qualification_policy_digest,
                conclusion: attestation.conclusion,
                publication_action_hashes: Vec::new(),
            })
            .publication_action_hashes
            .push(action_hash.clone());
    }

    let mut projections: Vec<_> = groups.into_values().collect();
    for projection in &mut projections {
        projection.publication_action_hashes.sort_by(action_hash_order);
        projection.publication_action_hashes.dedup();
    }
    projections.sort_by(|left, right| {
        left.qualification_policy_digest
            .cmp(&right.qualification_policy_digest)
            .then_with(|| conclusion_rank(left.conclusion).cmp(&conclusion_rank(right.conclusion)))
    });
    projections
}

fn validate_attestation_shape(
    attestation: &QualifiedCausalAssessmentAttestationV1,
) -> Result<(), CausalAttestationStateError> {
    if attestation.schema_version != 1
        || attestation.attestation_id.trim().is_empty()
        || attestation.attestation_id.len() > MAX_ID_LEN
        || attestation.qualified_receipt_commitment.value == [0u8; 32]
    {
        return Err(CausalAttestationStateError::MalformedAttestation);
    }
    attestation
        .qualification_policy_digest
        .validate_shape()
        .map_err(|_| CausalAttestationStateError::InvalidDigest)?;
    if attestation.qualification_policy_digest.domain
        != DigestDomain::ClinicalCausalQualificationPolicy
    {
        return Err(CausalAttestationStateError::WrongQualificationPolicyDomain);
    }
    Ok(())
}

fn validate_correction_shape(
    correction: &CausalAttestationCorrection,
) -> Result<(), CausalAttestationStateError> {
    if correction.correction_id.trim().is_empty()
        || correction.correction_id.len() > MAX_ID_LEN
        || correction.qualified_receipt_commitment.value == [0u8; 32]
    {
        return Err(CausalAttestationStateError::MalformedCorrection);
    }
    if correction
        .rationale_commitment
        .is_some_and(|commitment| commitment.value == [0u8; 32])
    {
        return Err(CausalAttestationStateError::MalformedCorrection);
    }
    if correction.reason == CausalAttestationCorrectionReason::Other
        && correction.rationale_commitment.is_none()
    {
        return Err(CausalAttestationStateError::MalformedCorrection);
    }
    Ok(())
}

fn is_invalidating(reason: CausalAttestationCorrectionReason) -> bool {
    matches!(
        reason,
        CausalAttestationCorrectionReason::EnteredInError
            | CausalAttestationCorrectionReason::SourceEvidenceRevoked
            | CausalAttestationCorrectionReason::AssessorAuthorityRevoked
            | CausalAttestationCorrectionReason::EvidenceTrustRevoked
    )
}

fn commitment_key(commitment: OpaqueCausalCommitmentV1) -> CommitmentKey {
    let scheme = match commitment.scheme {
        CausalCommitmentSchemeV1::HmacSha256 => 0,
        CausalCommitmentSchemeV1::Blake3Keyed => 1,
    };
    (scheme, commitment.value)
}

fn conclusion_rank(conclusion: CausalConclusionV1) -> u8 {
    match conclusion {
        CausalConclusionV1::Indeterminate => 0,
        CausalConclusionV1::TemporalAssociationOnly => 1,
        CausalConclusionV1::EvidenceSuggestsRelationship => 2,
        CausalConclusionV1::EvidenceSupportsRelationship => 3,
        CausalConclusionV1::EvidenceAgainstRelationship => 4,
    }
}

fn action_hash_order(left: &ActionHash, right: &ActionHash) -> std::cmp::Ordering {
    left.get_raw_36().cmp(right.get_raw_36())
}

fn correction_order(left: &CausalCorrectionView, right: &CausalCorrectionView) -> std::cmp::Ordering {
    action_hash_order(
        &left.correction_action_hash,
        &right.correction_action_hash,
    )
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CausalAttestationStateError {
    #[error("qualified causal attestation is malformed")]
    MalformedAttestation,
    #[error("causal attestation correction is malformed")]
    MalformedCorrection,
    #[error("stored digest is invalid")]
    InvalidDigest,
    #[error("qualification policy digest has the wrong domain")]
    WrongQualificationPolicyDomain,
    #[error("same publication action hash was supplied with conflicting payloads")]
    ActionHashPayloadConflict,
    #[error("same correction action hash was supplied with conflicting payloads")]
    CorrectionActionHashPayloadConflict,
    #[error("correction_id is reused by multiple correction actions")]
    DuplicateCorrectionId,
    #[error("causal correction target publication is missing from supplied read set")]
    OrphanCorrection,
    #[error("causal correction receipt commitment does not match target publication")]
    CorrectionCommitmentMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_integrity::{DigestAlgorithm, DigestDomain};

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
        id: &str,
        conclusion: CausalConclusionV1,
        policy_seed: u8,
    ) -> CausalAttestationPublicationRecord {
        CausalAttestationPublicationRecord {
            action_hash: action(action_seed),
            attestation: QualifiedCausalAssessmentAttestation {
                attestation: QualifiedCausalAssessmentAttestationV1 {
                    schema_version: 1,
                    attestation_id: id.into(),
                    qualified_receipt_commitment: commitment(receipt_seed),
                    qualification_policy_digest: digest(
                        DigestDomain::ClinicalCausalQualificationPolicy,
                        policy_seed,
                    ),
                    conclusion,
                },
                verifier_authorization_hash: action(200),
            },
        }
    }

    fn correction(
        action_seed: u8,
        target: &CausalAttestationPublicationRecord,
        reason: CausalAttestationCorrectionReason,
    ) -> CausalAttestationCorrectionRecord {
        CausalAttestationCorrectionRecord {
            action_hash: action(action_seed),
            correction: CausalAttestationCorrection {
                correction_id: format!("correction-{action_seed}"),
                attestation_hash: target.action_hash.clone(),
                qualified_receipt_commitment: target
                    .attestation
                    .attestation
                    .qualified_receipt_commitment,
                reason,
                rationale_commitment: if reason == CausalAttestationCorrectionReason::Other {
                    Some(commitment(99))
                } else {
                    None
                },
            },
        }
    }

    #[test]
    fn equivalent_publications_of_same_private_receipt_are_idempotent() {
        let a = publication(1, 10, "a", CausalConclusionV1::Indeterminate, 20);
        let b = publication(2, 10, "b", CausalConclusionV1::Indeterminate, 20);
        let view = reduce_causal_attestation_state(&[a, b], &[]).unwrap();
        assert_eq!(view.lineages.len(), 1);
        match &view.lineages[0].lifecycle {
            CausalAttestationLifecycle::Current { projection } => {
                assert_eq!(projection.publication_action_hashes.len(), 2);
            }
            _ => panic!("equivalent publications should remain one semantic current projection"),
        }
    }

    #[test]
    fn same_receipt_commitment_with_changed_conclusion_is_conflict() {
        let a = publication(1, 10, "a", CausalConclusionV1::Indeterminate, 20);
        let b = publication(
            2,
            10,
            "b",
            CausalConclusionV1::EvidenceSupportsRelationship,
            20,
        );
        let view = reduce_causal_attestation_state(&[a, b], &[]).unwrap();
        assert!(matches!(
            &view.lineages[0].lifecycle,
            CausalAttestationLifecycle::ProjectionConflict { .. }
        ));
    }

    #[test]
    fn duplicate_correction_suppresses_only_target_publication() {
        let a = publication(1, 10, "a", CausalConclusionV1::Indeterminate, 20);
        let b = publication(2, 10, "b", CausalConclusionV1::Indeterminate, 20);
        let c = correction(3, &b, CausalAttestationCorrectionReason::DuplicateAttestation);
        let view = reduce_causal_attestation_state(&[a, b], &[c]).unwrap();
        assert_eq!(view.lineages[0].effective_publication_action_hashes.len(), 1);
        assert!(matches!(
            &view.lineages[0].lifecycle,
            CausalAttestationLifecycle::Current { .. }
        ));
    }

    #[test]
    fn evidence_revocation_invalidates_entire_receipt_lineage() {
        let a = publication(
            1,
            10,
            "a",
            CausalConclusionV1::EvidenceSuggestsRelationship,
            20,
        );
        let b = publication(
            2,
            10,
            "b",
            CausalConclusionV1::EvidenceSuggestsRelationship,
            20,
        );
        let c = correction(3, &a, CausalAttestationCorrectionReason::SourceEvidenceRevoked);
        let view = reduce_causal_attestation_state(&[a, b], &[c]).unwrap();
        assert!(matches!(
            &view.lineages[0].lifecycle,
            CausalAttestationLifecycle::Invalidated { .. }
        ));
    }

    #[test]
    fn invalidation_dominates_unknown_effect_correction() {
        let a = publication(1, 10, "a", CausalConclusionV1::TemporalAssociationOnly, 20);
        let unknown = correction(2, &a, CausalAttestationCorrectionReason::Other);
        let invalid = correction(3, &a, CausalAttestationCorrectionReason::EnteredInError);
        let view = reduce_causal_attestation_state(&[a], &[unknown, invalid]).unwrap();
        assert!(matches!(
            &view.lineages[0].lifecycle,
            CausalAttestationLifecycle::Invalidated { .. }
        ));
        assert_eq!(view.lineages[0].corrections.len(), 2);
    }

    #[test]
    fn policy_supersession_is_not_erasure_or_generic_invalidation() {
        let a = publication(1, 10, "a", CausalConclusionV1::TemporalAssociationOnly, 20);
        let c = correction(2, &a, CausalAttestationCorrectionReason::PolicySuperseded);
        let view = reduce_causal_attestation_state(&[a], &[c]).unwrap();
        assert!(matches!(
            &view.lineages[0].lifecycle,
            CausalAttestationLifecycle::Superseded { .. }
        ));
    }

    #[test]
    fn unknown_correction_effect_forces_review() {
        let a = publication(1, 10, "a", CausalConclusionV1::TemporalAssociationOnly, 20);
        let c = correction(2, &a, CausalAttestationCorrectionReason::Other);
        let view = reduce_causal_attestation_state(&[a], &[c]).unwrap();
        assert!(matches!(
            &view.lineages[0].lifecycle,
            CausalAttestationLifecycle::ReviewRequired { .. }
        ));
    }

    #[test]
    fn missing_correction_target_fails_closed() {
        let a = publication(1, 10, "a", CausalConclusionV1::Indeterminate, 20);
        let mut c = correction(2, &a, CausalAttestationCorrectionReason::DuplicateAttestation);
        c.correction.attestation_hash = action(99);
        assert_eq!(
            reduce_causal_attestation_state(&[a], &[c]),
            Err(CausalAttestationStateError::OrphanCorrection)
        );
    }
}
