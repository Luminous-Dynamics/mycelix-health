#![deny(unsafe_code)]
//! Conflict-preserving qualified medication administration read model.
//!
//! This crate reduces already DHT-valid administration publications and corrections.
//! It does not query the DHT and never interprets missing data as proof of global absence.
//! There is deliberately no `performed: bool` or last-write-wins rule.

use holo_hash::ActionHash;
use medication_administration_integrity::{
    AdministrationCorrectionReason, MedicationAdministrationCorrection,
    MedicationAdministrationAttestationV1, QualifiedMedicationAdministration,
};
use mycelix_clinical_integrity::{DigestDomain, StoredDigest};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Clone)]
pub struct AdministrationPublicationRecord {
    pub action_hash: ActionHash,
    pub administration: QualifiedMedicationAdministration,
}

#[derive(Clone)]
pub struct AdministrationCorrectionRecord {
    pub action_hash: ActionHash,
    pub correction: MedicationAdministrationCorrection,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AdministrationCorrectionView {
    pub correction_action_hash: ActionHash,
    pub target_publication_action_hash: ActionHash,
    pub reason: AdministrationCorrectionReason,
    pub rationale_commitment: Option<[u8; 32]>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub enum AdministrationLineageLifecycle {
    /// At least one unsuppressed publication remains and no semantic-invalidating
    /// correction is known in the supplied complete-for-purpose read set.
    Current,
    /// One or more corrections invalidate the semantic administration receipt.
    Corrected {
        corrections: Vec<AdministrationCorrectionView>,
    },
    /// Every publication of this semantic receipt has been marked duplicate
    /// documentation, while no semantic-invalidating correction exists.
    PublicationSuppressed {
        corrections: Vec<AdministrationCorrectionView>,
    },
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AdministrationLineageView {
    pub administration_id: String,
    pub administration_receipt_digest: StoredDigest,
    pub event_digest: StoredDigest,
    pub medication_artifact_digest: StoredDigest,
    pub activation_semantic_receipt_digest: StoredDigest,
    pub finalized_dispense_receipt_digest: StoredDigest,
    pub administrator_principal_binding: [u8; 32],
    pub authority_policy_digest: StoredDigest,
    pub administration_policy_digest: StoredDigest,
    /// All equivalent DHT publications of this exact semantic receipt.
    pub publication_action_hashes: Vec<ActionHash>,
    /// Publications not individually suppressed as duplicate documentation.
    pub effective_publication_action_hashes: Vec<ActionHash>,
    pub lifecycle: AdministrationLineageLifecycle,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum AdministrationCurrentState {
    None,
    One {
        administration_receipt_digest: StoredDigest,
    },
    Conflict {
        administration_receipt_digests: Vec<StoredDigest>,
    },
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AdministrationOccurrenceView {
    /// V1 logical occurrence key. The verifier is responsible for ensuring this ID is
    /// stable for the intended administration occurrence. A future computable schedule
    /// occurrence digest should replace reliance on caller-selected IDs.
    pub administration_id: String,
    pub current: AdministrationCurrentState,
    pub lineages: Vec<AdministrationLineageView>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct MedicationAdministrationView {
    pub occurrences: Vec<AdministrationOccurrenceView>,
}

#[derive(Clone)]
struct PublicationGroup {
    attestation: MedicationAdministrationAttestationV1,
    publication_action_hashes: Vec<ActionHash>,
    duplicate_suppressed_actions: HashSet<ActionHash>,
    duplicate_corrections: Vec<AdministrationCorrectionView>,
    semantic_corrections: Vec<AdministrationCorrectionView>,
}

/// Reduce a complete-for-purpose set of already DHT-valid administration records.
///
/// The caller must supply all relevant publications and correction records for the
/// scope it wishes to interpret. The reducer can prove reference closure inside that
/// set, but it cannot prove that an omitted correction does not exist elsewhere.
pub fn reduce_administration_state(
    publications: &[AdministrationPublicationRecord],
    corrections: &[AdministrationCorrectionRecord],
) -> Result<MedicationAdministrationView, AdministrationStateError> {
    let mut publication_by_action: HashMap<ActionHash, (String, StoredDigest)> = HashMap::new();
    let mut publication_payload_by_action: HashMap<ActionHash, MedicationAdministrationAttestationV1> =
        HashMap::new();
    let mut groups: HashMap<StoredDigest, PublicationGroup> = HashMap::new();

    for record in publications {
        validate_attestation_shape(&record.administration.attestation)?;
        let attestation = &record.administration.attestation;
        let receipt = attestation.administration_receipt_digest;

        if let Some(existing) = publication_payload_by_action.get(&record.action_hash) {
            if existing != attestation {
                return Err(AdministrationStateError::ActionHashPayloadConflict);
            }
            continue;
        }

        publication_payload_by_action.insert(record.action_hash.clone(), attestation.clone());
        publication_by_action.insert(
            record.action_hash.clone(),
            (attestation.administration_id.clone(), receipt),
        );

        match groups.get_mut(&receipt) {
            Some(group) => {
                if group.attestation != *attestation {
                    return Err(AdministrationStateError::ConflictingSemanticReceiptDuplicate);
                }
                push_unique(&mut group.publication_action_hashes, record.action_hash.clone());
            }
            None => {
                groups.insert(
                    receipt,
                    PublicationGroup {
                        attestation: attestation.clone(),
                        publication_action_hashes: vec![record.action_hash.clone()],
                        duplicate_suppressed_actions: HashSet::new(),
                        duplicate_corrections: Vec::new(),
                        semantic_corrections: Vec::new(),
                    },
                );
            }
        }
    }

    let mut seen_correction_actions: HashMap<ActionHash, MedicationAdministrationCorrection> =
        HashMap::new();
    for record in corrections {
        validate_correction_shape(&record.correction)?;
        if let Some(existing) = seen_correction_actions.get(&record.action_hash) {
            if existing != &record.correction {
                return Err(AdministrationStateError::CorrectionActionHashPayloadConflict);
            }
            continue;
        }
        seen_correction_actions.insert(record.action_hash.clone(), record.correction.clone());

        let (_, target_receipt) = publication_by_action
            .get(&record.correction.administration_hash)
            .ok_or(AdministrationStateError::OrphanCorrection)?;
        if *target_receipt != record.correction.administration_receipt_digest {
            return Err(AdministrationStateError::CorrectionReceiptMismatch);
        }

        let group = groups
            .get_mut(target_receipt)
            .ok_or(AdministrationStateError::OrphanCorrection)?;
        let view = AdministrationCorrectionView {
            correction_action_hash: record.action_hash.clone(),
            target_publication_action_hash: record.correction.administration_hash.clone(),
            reason: record.correction.reason.clone(),
            rationale_commitment: record.correction.rationale_commitment,
        };

        if matches!(record.correction.reason, AdministrationCorrectionReason::DuplicateDocumentation)
        {
            group
                .duplicate_suppressed_actions
                .insert(record.correction.administration_hash.clone());
            group.duplicate_corrections.push(view);
        } else {
            group.semantic_corrections.push(view);
        }
    }

    let mut by_occurrence: HashMap<String, Vec<AdministrationLineageView>> = HashMap::new();
    for (_, mut group) in groups {
        group.publication_action_hashes.sort_by(action_hash_order);
        group.duplicate_corrections.sort_by(correction_order);
        group.semantic_corrections.sort_by(correction_order);

        let effective_publication_action_hashes: Vec<ActionHash> = group
            .publication_action_hashes
            .iter()
            .filter(|hash| !group.duplicate_suppressed_actions.contains(*hash))
            .cloned()
            .collect();

        let lifecycle = if !group.semantic_corrections.is_empty() {
            AdministrationLineageLifecycle::Corrected {
                corrections: group.semantic_corrections.clone(),
            }
        } else if effective_publication_action_hashes.is_empty() {
            AdministrationLineageLifecycle::PublicationSuppressed {
                corrections: group.duplicate_corrections.clone(),
            }
        } else {
            AdministrationLineageLifecycle::Current
        };

        let attestation = group.attestation;
        let view = AdministrationLineageView {
            administration_id: attestation.administration_id.clone(),
            administration_receipt_digest: attestation.administration_receipt_digest,
            event_digest: attestation.event_digest,
            medication_artifact_digest: attestation.medication_artifact_digest,
            activation_semantic_receipt_digest: attestation.activation_semantic_receipt_digest,
            finalized_dispense_receipt_digest: attestation.finalized_dispense_receipt_digest,
            administrator_principal_binding: attestation.administrator_principal_binding,
            authority_policy_digest: attestation.authority_policy_digest,
            administration_policy_digest: attestation.administration_policy_digest,
            publication_action_hashes: group.publication_action_hashes,
            effective_publication_action_hashes,
            lifecycle,
        };
        by_occurrence
            .entry(view.administration_id.clone())
            .or_default()
            .push(view);
    }

    let mut occurrences = Vec::with_capacity(by_occurrence.len());
    for (administration_id, mut lineages) in by_occurrence {
        lineages.sort_by(|left, right| digest_order(
            &left.administration_receipt_digest,
            &right.administration_receipt_digest,
        ));

        let mut current_receipts: Vec<StoredDigest> = lineages
            .iter()
            .filter(|lineage| matches!(&lineage.lifecycle, AdministrationLineageLifecycle::Current))
            .map(|lineage| lineage.administration_receipt_digest)
            .collect();
        current_receipts.sort_by(digest_order);
        current_receipts.dedup();

        let current = match current_receipts.as_slice() {
            [] => AdministrationCurrentState::None,
            [one] => AdministrationCurrentState::One {
                administration_receipt_digest: *one,
            },
            _ => AdministrationCurrentState::Conflict {
                administration_receipt_digests: current_receipts,
            },
        };

        occurrences.push(AdministrationOccurrenceView {
            administration_id,
            current,
            lineages,
        });
    }
    occurrences.sort_by(|left, right| left.administration_id.cmp(&right.administration_id));

    Ok(MedicationAdministrationView { occurrences })
}

fn validate_attestation_shape(
    attestation: &MedicationAdministrationAttestationV1,
) -> Result<(), AdministrationStateError> {
    if attestation.schema_version != 1 || attestation.administration_id.trim().is_empty() {
        return Err(AdministrationStateError::MalformedAdministration);
    }
    if attestation.administrator_principal_binding == [0u8; 32] {
        return Err(AdministrationStateError::MalformedAdministration);
    }
    require_domain(
        attestation.administration_receipt_digest,
        DigestDomain::MedicationAdministrationReceipt,
    )?;
    require_domain(
        attestation.event_digest,
        DigestDomain::MedicationAdministrationEvent,
    )?;
    require_domain(
        attestation.medication_artifact_digest,
        DigestDomain::MedicationRequestArtifact,
    )?;
    require_activation_receipt_domain(attestation.activation_semantic_receipt_digest)?;
    require_domain(
        attestation.finalized_dispense_receipt_digest,
        DigestDomain::MedicationDispenseReceipt,
    )?;
    require_domain(attestation.authority_policy_digest, DigestDomain::AuthorityPolicy)?;
    require_domain(
        attestation.administration_policy_digest,
        DigestDomain::MedicationAdministrationPolicy,
    )?;
    Ok(())
}

fn validate_correction_shape(
    correction: &MedicationAdministrationCorrection,
) -> Result<(), AdministrationStateError> {
    if correction.correction_id.trim().is_empty() {
        return Err(AdministrationStateError::MalformedCorrection);
    }
    require_domain(
        correction.administration_receipt_digest,
        DigestDomain::MedicationAdministrationReceipt,
    )?;
    if correction
        .rationale_commitment
        .is_some_and(|commitment| commitment == [0u8; 32])
    {
        return Err(AdministrationStateError::MalformedCorrection);
    }
    if matches!(correction.reason, AdministrationCorrectionReason::Other)
        && correction.rationale_commitment.is_none()
    {
        return Err(AdministrationStateError::MalformedCorrection);
    }
    Ok(())
}

fn require_activation_receipt_domain(digest: StoredDigest) -> Result<(), AdministrationStateError> {
    digest
        .validate_shape()
        .map_err(|_| AdministrationStateError::InvalidDigest)?;
    if !matches!(
        digest.domain,
        DigestDomain::MedicationActivationReceipt
            | DigestDomain::EmergencyMedicationOverrideReceipt
    ) {
        return Err(AdministrationStateError::WrongActivationReceiptDomain);
    }
    Ok(())
}

fn require_domain(
    digest: StoredDigest,
    expected: DigestDomain,
) -> Result<(), AdministrationStateError> {
    digest
        .validate_shape()
        .map_err(|_| AdministrationStateError::InvalidDigest)?;
    if digest.domain != expected {
        return Err(AdministrationStateError::WrongDigestDomain {
            expected,
            actual: digest.domain,
        });
    }
    Ok(())
}

fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn action_hash_order(left: &ActionHash, right: &ActionHash) -> std::cmp::Ordering {
    left.get_raw_36().cmp(right.get_raw_36())
}

fn correction_order(
    left: &AdministrationCorrectionView,
    right: &AdministrationCorrectionView,
) -> std::cmp::Ordering {
    action_hash_order(&left.correction_action_hash, &right.correction_action_hash)
}

fn digest_domain_rank(domain: DigestDomain) -> u16 {
    match domain {
        DigestDomain::MedicationAdministrationReceipt => 0,
        DigestDomain::MedicationActivationReceipt => 1,
        DigestDomain::EmergencyMedicationOverrideReceipt => 2,
        _ => 100,
    }
}

fn digest_order(left: &StoredDigest, right: &StoredDigest) -> std::cmp::Ordering {
    digest_domain_rank(left.domain)
        .cmp(&digest_domain_rank(right.domain))
        .then_with(|| left.value.cmp(&right.value))
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AdministrationStateError {
    #[error("qualified medication administration is malformed")]
    MalformedAdministration,
    #[error("medication administration correction is malformed")]
    MalformedCorrection,
    #[error("stored digest is invalid")]
    InvalidDigest,
    #[error("digest domain mismatch: expected {expected:?}, got {actual:?}")]
    WrongDigestDomain {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("activation semantic receipt has invalid digest domain")]
    WrongActivationReceiptDomain,
    #[error("same publication action hash was supplied with conflicting payloads")]
    ActionHashPayloadConflict,
    #[error("same correction action hash was supplied with conflicting payloads")]
    CorrectionActionHashPayloadConflict,
    #[error("same semantic administration receipt is paired with changed attestation fields")]
    ConflictingSemanticReceiptDuplicate,
    #[error("administration correction target publication is missing from supplied read set")]
    OrphanCorrection,
    #[error("administration correction receipt does not match target publication")]
    CorrectionReceiptMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use holo_hash::AgentPubKey;
    use mycelix_clinical_integrity::{DigestAlgorithm, DigestDomain};

    fn stored(domain: DigestDomain, seed: u8) -> StoredDigest {
        StoredDigest {
            algorithm: DigestAlgorithm::Blake3_256,
            domain,
            value: [seed; 32],
        }
    }

    fn action(seed: u8) -> ActionHash {
        ActionHash::from_raw_36(vec![seed; 36])
    }

    fn attestation(id: &str, receipt_seed: u8) -> MedicationAdministrationAttestationV1 {
        MedicationAdministrationAttestationV1 {
            schema_version: 1,
            administration_id: id.into(),
            administration_receipt_digest: stored(
                DigestDomain::MedicationAdministrationReceipt,
                receipt_seed,
            ),
            event_digest: stored(DigestDomain::MedicationAdministrationEvent, receipt_seed + 1),
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 10),
            activation_semantic_receipt_digest: stored(DigestDomain::MedicationActivationReceipt, 11),
            finalized_dispense_receipt_digest: stored(DigestDomain::MedicationDispenseReceipt, 12),
            administrator_principal_binding: [13; 32],
            authority_policy_digest: stored(DigestDomain::AuthorityPolicy, 14),
            administration_policy_digest: stored(DigestDomain::MedicationAdministrationPolicy, 15),
        }
    }

    fn publication(action_seed: u8, id: &str, receipt_seed: u8) -> AdministrationPublicationRecord {
        AdministrationPublicationRecord {
            action_hash: action(action_seed),
            administration: QualifiedMedicationAdministration {
                attestation: attestation(id, receipt_seed),
                verifier_authorization_hash: action(200),
            },
        }
    }

    fn correction(
        action_seed: u8,
        target: &AdministrationPublicationRecord,
        reason: AdministrationCorrectionReason,
    ) -> AdministrationCorrectionRecord {
        AdministrationCorrectionRecord {
            action_hash: action(action_seed),
            correction: MedicationAdministrationCorrection {
                correction_id: format!("correction-{action_seed}"),
                administration_hash: target.action_hash.clone(),
                administration_receipt_digest: target
                    .administration
                    .attestation
                    .administration_receipt_digest,
                reason,
                rationale_commitment: None,
            },
        }
    }

    #[test]
    fn duplicate_publications_of_same_receipt_are_idempotent() {
        let a = publication(1, "dose-a", 20);
        let mut b = publication(2, "dose-a", 20);
        b.administration.attestation = a.administration.attestation.clone();
        let view = reduce_administration_state(&[a, b], &[]).unwrap();
        assert_eq!(view.occurrences.len(), 1);
        assert_eq!(view.occurrences[0].lineages.len(), 1);
        assert_eq!(view.occurrences[0].lineages[0].publication_action_hashes.len(), 2);
        assert!(matches!(
            view.occurrences[0].current,
            AdministrationCurrentState::One { .. }
        ));
    }

    #[test]
    fn distinct_current_receipts_for_same_administration_id_are_conflict() {
        let a = publication(1, "dose-a", 20);
        let b = publication(2, "dose-a", 30);
        let view = reduce_administration_state(&[a, b], &[]).unwrap();
        assert!(matches!(
            &view.occurrences[0].current,
            AdministrationCurrentState::Conflict { .. }
        ));
    }

    #[test]
    fn semantic_correction_can_resolve_conflict_without_last_write_wins() {
        let a = publication(1, "dose-a", 20);
        let b = publication(2, "dose-a", 30);
        let c = correction(3, &b, AdministrationCorrectionReason::WrongDose);
        let view = reduce_administration_state(&[a, b], &[c]).unwrap();
        match &view.occurrences[0].current {
            AdministrationCurrentState::One {
                administration_receipt_digest,
            } => assert_eq!(*administration_receipt_digest, stored(DigestDomain::MedicationAdministrationReceipt, 20)),
            _ => panic!("explicit semantic correction should leave one current receipt"),
        }
    }

    #[test]
    fn duplicate_documentation_suppresses_only_target_publication() {
        let a = publication(1, "dose-a", 20);
        let mut b = publication(2, "dose-a", 20);
        b.administration.attestation = a.administration.attestation.clone();
        let c = correction(3, &b, AdministrationCorrectionReason::DuplicateDocumentation);
        let view = reduce_administration_state(&[a, b], &[c]).unwrap();
        assert!(matches!(
            view.occurrences[0].current,
            AdministrationCurrentState::One { .. }
        ));
        assert_eq!(
            view.occurrences[0].lineages[0].effective_publication_action_hashes.len(),
            1
        );
    }

    #[test]
    fn suppressing_only_publication_leaves_no_current_claim() {
        let a = publication(1, "dose-a", 20);
        let c = correction(2, &a, AdministrationCorrectionReason::DuplicateDocumentation);
        let view = reduce_administration_state(&[a], &[c]).unwrap();
        assert_eq!(view.occurrences[0].current, AdministrationCurrentState::None);
        assert!(matches!(
            &view.occurrences[0].lineages[0].lifecycle,
            AdministrationLineageLifecycle::PublicationSuppressed { .. }
        ));
    }

    #[test]
    fn orphan_correction_fails_closed() {
        let fake = AdministrationCorrectionRecord {
            action_hash: action(2),
            correction: MedicationAdministrationCorrection {
                correction_id: "orphan".into(),
                administration_hash: action(99),
                administration_receipt_digest: stored(
                    DigestDomain::MedicationAdministrationReceipt,
                    20,
                ),
                reason: AdministrationCorrectionReason::WrongPatient,
                rationale_commitment: None,
            },
        };
        assert_eq!(
            reduce_administration_state(&[], &[fake]),
            Err(AdministrationStateError::OrphanCorrection)
        );
    }

    #[test]
    fn input_order_does_not_change_state() {
        let a = publication(1, "dose-a", 20);
        let b = publication(2, "dose-a", 30);
        let c = correction(3, &b, AdministrationCorrectionReason::WrongDose);
        let forward = reduce_administration_state(&[a.clone(), b.clone()], &[c.clone()]).unwrap();
        let reverse = reduce_administration_state(&[b, a], &[c]).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn emergency_activation_domain_is_preserved() {
        let mut a = publication(1, "dose-a", 20);
        a.administration.attestation.activation_semantic_receipt_digest = stored(
            DigestDomain::EmergencyMedicationOverrideReceipt,
            11,
        );
        let view = reduce_administration_state(&[a], &[]).unwrap();
        assert_eq!(
            view.occurrences[0].lineages[0]
                .activation_semantic_receipt_digest
                .domain,
            DigestDomain::EmergencyMedicationOverrideReceipt
        );
    }

    #[test]
    fn action_hash_constructor_fixture_is_valid() {
        let _ = AgentPubKey::from_raw_36(vec![1; 36]);
        assert_eq!(action(1).get_raw_36().len(), 36);
    }
}
