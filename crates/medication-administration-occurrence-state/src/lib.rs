#![deny(unsafe_code)]
//! Occurrence-aware medication-administration read model.
//!
//! This layer reuses the receipt/correction semantics from
//! `mycelix-medication-administration-state`, but it never uses caller-selected
//! `administration_id` as the clinical conflict key. Only already DHT-valid occurrence
//! bindings can place a current semantic receipt into an occurrence bucket.
//!
//! Missing bindings remain `Unbound`; incompatible current bindings remain
//! `BindingConflict`. Neither condition falls back to record IDs or timestamps.

use holo_hash::ActionHash;
use medication_administration_occurrence_integrity::{
    MedicationAdministrationOccurrenceBindingCorrection, OccurrenceBindingCorrectionReason,
    QualifiedMedicationAdministrationOccurrenceBinding,
};
use mycelix_clinical_integrity::{DigestDomain, StoredDigest};
use mycelix_medication_administration_state::{
    reduce_administration_state, AdministrationCorrectionRecord, AdministrationLineageLifecycle,
    AdministrationLineageView, AdministrationPublicationRecord, AdministrationStateError,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Clone)]
pub struct OccurrenceBindingPublicationRecord {
    pub action_hash: ActionHash,
    pub binding: QualifiedMedicationAdministrationOccurrenceBinding,
}

#[derive(Clone)]
pub struct OccurrenceBindingCorrectionRecord {
    pub action_hash: ActionHash,
    pub correction: MedicationAdministrationOccurrenceBindingCorrection,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct OccurrenceBindingCorrectionView {
    pub correction_action_hash: ActionHash,
    pub target_binding_action_hash: ActionHash,
    pub reason: OccurrenceBindingCorrectionReason,
    pub rationale_commitment: Option<[u8; 32]>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum OccurrenceBindingLifecycle {
    Current,
    Corrected {
        semantic_corrections: Vec<OccurrenceBindingCorrectionView>,
    },
    PublicationSuppressed {
        duplicate_corrections: Vec<OccurrenceBindingCorrectionView>,
    },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct OccurrenceBindingLineageView {
    pub occurrence_binding_digest: StoredDigest,
    pub administration_receipt_digest: StoredDigest,
    pub event_digest: StoredDigest,
    pub occurrence_digest: StoredDigest,
    pub medication_artifact_digest: StoredDigest,
    pub dosage_index: u32,
    pub publication_action_hashes: Vec<ActionHash>,
    pub effective_publication_action_hashes: Vec<ActionHash>,
    pub corrections: Vec<OccurrenceBindingCorrectionView>,
    pub lifecycle: OccurrenceBindingLifecycle,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum OccurrenceCurrentState {
    One {
        administration_receipt_digest: StoredDigest,
    },
    Conflict {
        administration_receipt_digests: Vec<StoredDigest>,
    },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ClinicalAdministrationOccurrenceView {
    pub occurrence_digest: StoredDigest,
    pub current: OccurrenceCurrentState,
    pub current_receipt_digests: Vec<StoredDigest>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum UnresolvedCurrentAdministrationReason {
    /// No current DHT-admitted occurrence binding is present in the supplied read set.
    Unbound,
    /// The same semantic administration receipt has current admitted bindings to more
    /// than one distinct clinical occurrence. No occurrence is selected.
    BindingConflict {
        occurrence_digests: Vec<StoredDigest>,
    },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct UnresolvedCurrentAdministrationView {
    pub administration_receipt_digest: StoredDigest,
    pub reason: UnresolvedCurrentAdministrationReason,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct OccurrenceAwareMedicationAdministrationView {
    /// Full receipt/correction lineage from the v1 reducer, retained for audit.
    pub receipt_lineages: Vec<AdministrationLineageView>,
    /// Full admitted occurrence-binding/correction lineage, retained for audit.
    pub binding_lineages: Vec<OccurrenceBindingLineageView>,
    /// Canonical current state grouped only by computable occurrence digest.
    pub occurrences: Vec<ClinicalAdministrationOccurrenceView>,
    /// Current receipts that cannot safely be assigned to one occurrence.
    pub unresolved_current: Vec<UnresolvedCurrentAdministrationView>,
}

#[derive(Clone)]
struct BindingPublicationMeta {
    action_hash: ActionHash,
    administration_hash: ActionHash,
}

#[derive(Clone)]
struct BindingGroup {
    administration_receipt_digest: StoredDigest,
    event_digest: StoredDigest,
    occurrence_digest: StoredDigest,
    medication_artifact_digest: StoredDigest,
    dosage_index: u32,
    publications: Vec<BindingPublicationMeta>,
    duplicate_suppressed_actions: HashSet<ActionHash>,
    duplicate_corrections: Vec<OccurrenceBindingCorrectionView>,
    semantic_corrections: Vec<OccurrenceBindingCorrectionView>,
}

/// Reduce a complete-for-purpose set of already DHT-valid administration and
/// occurrence-binding records.
///
/// This function checks reference closure inside the supplied set, but cannot prove
/// that omitted DHT records do not exist. Callers must use a completeness-qualified
/// adapter before treating the result as a complete patient medication-administration
/// view.
pub fn reduce_occurrence_aware_administration_state(
    publications: &[AdministrationPublicationRecord],
    corrections: &[AdministrationCorrectionRecord],
    binding_publications: &[OccurrenceBindingPublicationRecord],
    binding_corrections: &[OccurrenceBindingCorrectionRecord],
) -> Result<OccurrenceAwareMedicationAdministrationView, OccurrenceStateError> {
    let legacy = reduce_administration_state(publications, corrections)?;

    let mut receipt_lineages: Vec<AdministrationLineageView> = legacy
        .occurrences
        .into_iter()
        .flat_map(|occurrence| occurrence.lineages)
        .collect();
    receipt_lineages.sort_by(|left, right| {
        digest_order(
            &left.administration_receipt_digest,
            &right.administration_receipt_digest,
        )
    });

    let mut administration_action_to_receipt: HashMap<ActionHash, StoredDigest> = HashMap::new();
    for publication in publications {
        let receipt = publication
            .administration
            .attestation
            .administration_receipt_digest;
        match administration_action_to_receipt.get(&publication.action_hash) {
            Some(existing) if *existing != receipt => {
                return Err(OccurrenceStateError::AdministrationActionPayloadConflict)
            }
            Some(_) => {}
            None => {
                administration_action_to_receipt.insert(publication.action_hash.clone(), receipt);
            }
        }
    }

    let (binding_lineages, binding_groups) = reduce_binding_lineages(
        &administration_action_to_receipt,
        binding_publications,
        binding_corrections,
    )?;

    let mut current_binding_groups_by_receipt: HashMap<StoredDigest, Vec<&BindingGroup>> =
        HashMap::new();
    for (binding_digest, group) in &binding_groups {
        let Some(view) = binding_lineages
            .iter()
            .find(|candidate| candidate.occurrence_binding_digest == *binding_digest)
        else {
            return Err(OccurrenceStateError::InternalBindingViewMismatch);
        };
        if matches!(&view.lifecycle, OccurrenceBindingLifecycle::Current) {
            current_binding_groups_by_receipt
                .entry(group.administration_receipt_digest)
                .or_default()
                .push(group);
        }
    }

    let mut occurrence_to_current_receipts: HashMap<StoredDigest, Vec<StoredDigest>> = HashMap::new();
    let mut unresolved_current = Vec::new();

    for lineage in &receipt_lineages {
        if !matches!(&lineage.lifecycle, AdministrationLineageLifecycle::Current) {
            continue;
        }

        let effective_admin_actions: HashSet<ActionHash> = lineage
            .effective_publication_action_hashes
            .iter()
            .cloned()
            .collect();
        let mut occurrence_digests = Vec::new();

        if let Some(groups) = current_binding_groups_by_receipt
            .get(&lineage.administration_receipt_digest)
        {
            for group in groups {
                let has_effective_source = group.publications.iter().any(|publication| {
                    !group
                        .duplicate_suppressed_actions
                        .contains(&publication.action_hash)
                        && effective_admin_actions.contains(&publication.administration_hash)
                });
                if has_effective_source {
                    push_unique(&mut occurrence_digests, group.occurrence_digest);
                }
            }
        }

        occurrence_digests.sort_by(digest_order);
        match occurrence_digests.as_slice() {
            [] => unresolved_current.push(UnresolvedCurrentAdministrationView {
                administration_receipt_digest: lineage.administration_receipt_digest,
                reason: UnresolvedCurrentAdministrationReason::Unbound,
            }),
            [one] => {
                push_unique(
                    occurrence_to_current_receipts.entry(*one).or_default(),
                    lineage.administration_receipt_digest,
                );
            }
            _ => unresolved_current.push(UnresolvedCurrentAdministrationView {
                administration_receipt_digest: lineage.administration_receipt_digest,
                reason: UnresolvedCurrentAdministrationReason::BindingConflict {
                    occurrence_digests,
                },
            }),
        }
    }

    let mut occurrences = Vec::with_capacity(occurrence_to_current_receipts.len());
    for (occurrence_digest, mut receipts) in occurrence_to_current_receipts {
        receipts.sort_by(digest_order);
        receipts.dedup();
        let current = if receipts.len() == 1 {
            OccurrenceCurrentState::One {
                administration_receipt_digest: receipts[0],
            }
        } else {
            OccurrenceCurrentState::Conflict {
                administration_receipt_digests: receipts.clone(),
            }
        };
        occurrences.push(ClinicalAdministrationOccurrenceView {
            occurrence_digest,
            current,
            current_receipt_digests: receipts,
        });
    }
    occurrences.sort_by(|left, right| digest_order(&left.occurrence_digest, &right.occurrence_digest));
    unresolved_current.sort_by(|left, right| {
        digest_order(
            &left.administration_receipt_digest,
            &right.administration_receipt_digest,
        )
    });

    Ok(OccurrenceAwareMedicationAdministrationView {
        receipt_lineages,
        binding_lineages,
        occurrences,
        unresolved_current,
    })
}

fn reduce_binding_lineages(
    administration_action_to_receipt: &HashMap<ActionHash, StoredDigest>,
    publications: &[OccurrenceBindingPublicationRecord],
    corrections: &[OccurrenceBindingCorrectionRecord],
) -> Result<
    (
        Vec<OccurrenceBindingLineageView>,
        HashMap<StoredDigest, BindingGroup>,
    ),
    OccurrenceStateError,
> {
    let mut action_payload: HashMap<ActionHash, QualifiedMedicationAdministrationOccurrenceBinding> =
        HashMap::new();
    let mut action_to_binding_digest: HashMap<ActionHash, StoredDigest> = HashMap::new();
    let mut groups: HashMap<StoredDigest, BindingGroup> = HashMap::new();

    for record in publications {
        validate_binding_attestation_shape(&record.binding)?;
        let attestation = &record.binding.attestation;

        let source_receipt = administration_action_to_receipt
            .get(&attestation.administration_hash)
            .ok_or(OccurrenceStateError::OrphanOccurrenceBinding)?;
        if *source_receipt != attestation.administration_receipt_digest {
            return Err(OccurrenceStateError::OccurrenceBindingReceiptMismatch);
        }

        if let Some(existing) = action_payload.get(&record.action_hash) {
            if existing != &record.binding {
                return Err(OccurrenceStateError::BindingActionPayloadConflict);
            }
            continue;
        }
        action_payload.insert(record.action_hash.clone(), record.binding.clone());
        action_to_binding_digest.insert(
            record.action_hash.clone(),
            attestation.occurrence_binding_digest,
        );

        match groups.get_mut(&attestation.occurrence_binding_digest) {
            Some(group) => {
                if group.administration_receipt_digest
                    != attestation.administration_receipt_digest
                    || group.event_digest != attestation.event_digest
                    || group.occurrence_digest != attestation.occurrence_digest
                    || group.medication_artifact_digest != attestation.medication_artifact_digest
                    || group.dosage_index != attestation.dosage_index
                {
                    return Err(OccurrenceStateError::BindingDigestSemanticConflict);
                }
                if !group.publications.iter().any(|publication| {
                    publication.action_hash == record.action_hash
                        && publication.administration_hash == attestation.administration_hash
                }) {
                    group.publications.push(BindingPublicationMeta {
                        action_hash: record.action_hash.clone(),
                        administration_hash: attestation.administration_hash.clone(),
                    });
                }
            }
            None => {
                groups.insert(
                    attestation.occurrence_binding_digest,
                    BindingGroup {
                        administration_receipt_digest: attestation.administration_receipt_digest,
                        event_digest: attestation.event_digest,
                        occurrence_digest: attestation.occurrence_digest,
                        medication_artifact_digest: attestation.medication_artifact_digest,
                        dosage_index: attestation.dosage_index,
                        publications: vec![BindingPublicationMeta {
                            action_hash: record.action_hash.clone(),
                            administration_hash: attestation.administration_hash.clone(),
                        }],
                        duplicate_suppressed_actions: HashSet::new(),
                        duplicate_corrections: Vec::new(),
                        semantic_corrections: Vec::new(),
                    },
                );
            }
        }
    }

    let mut correction_action_payload: HashMap<
        ActionHash,
        MedicationAdministrationOccurrenceBindingCorrection,
    > = HashMap::new();
    let mut correction_id_owner: HashMap<String, ActionHash> = HashMap::new();

    for record in corrections {
        validate_binding_correction_shape(&record.correction)?;
        if let Some(existing) = correction_action_payload.get(&record.action_hash) {
            if existing != &record.correction {
                return Err(OccurrenceStateError::BindingCorrectionActionPayloadConflict);
            }
            continue;
        }
        correction_action_payload.insert(record.action_hash.clone(), record.correction.clone());

        match correction_id_owner.get(&record.correction.correction_id) {
            Some(existing) if existing != &record.action_hash => {
                return Err(OccurrenceStateError::DuplicateBindingCorrectionId)
            }
            Some(_) => {}
            None => {
                correction_id_owner.insert(
                    record.correction.correction_id.clone(),
                    record.action_hash.clone(),
                );
            }
        }

        let target_binding_digest = action_to_binding_digest
            .get(&record.correction.binding_hash)
            .ok_or(OccurrenceStateError::OrphanBindingCorrection)?;
        if *target_binding_digest != record.correction.occurrence_binding_digest {
            return Err(OccurrenceStateError::BindingCorrectionDigestMismatch);
        }
        let group = groups
            .get_mut(target_binding_digest)
            .ok_or(OccurrenceStateError::OrphanBindingCorrection)?;
        if group.occurrence_digest != record.correction.occurrence_digest {
            return Err(OccurrenceStateError::BindingCorrectionOccurrenceMismatch);
        }

        let view = OccurrenceBindingCorrectionView {
            correction_action_hash: record.action_hash.clone(),
            target_binding_action_hash: record.correction.binding_hash.clone(),
            reason: record.correction.reason,
            rationale_commitment: record.correction.rationale_commitment,
        };
        if record.correction.reason == OccurrenceBindingCorrectionReason::DuplicateDocumentation {
            group
                .duplicate_suppressed_actions
                .insert(record.correction.binding_hash.clone());
            group.duplicate_corrections.push(view);
        } else {
            group.semantic_corrections.push(view);
        }
    }

    let mut views = Vec::with_capacity(groups.len());
    for (binding_digest, group) in &mut groups {
        group
            .publications
            .sort_by(|left, right| action_hash_order(&left.action_hash, &right.action_hash));
        group.duplicate_corrections.sort_by(binding_correction_order);
        group.semantic_corrections.sort_by(binding_correction_order);

        let publication_action_hashes: Vec<ActionHash> = group
            .publications
            .iter()
            .map(|publication| publication.action_hash.clone())
            .collect();
        let effective_publication_action_hashes: Vec<ActionHash> = group
            .publications
            .iter()
            .filter(|publication| {
                !group
                    .duplicate_suppressed_actions
                    .contains(&publication.action_hash)
            })
            .map(|publication| publication.action_hash.clone())
            .collect();

        let mut all_corrections = group.duplicate_corrections.clone();
        all_corrections.extend(group.semantic_corrections.iter().cloned());
        all_corrections.sort_by(binding_correction_order);

        let lifecycle = if !group.semantic_corrections.is_empty() {
            OccurrenceBindingLifecycle::Corrected {
                semantic_corrections: group.semantic_corrections.clone(),
            }
        } else if effective_publication_action_hashes.is_empty() {
            OccurrenceBindingLifecycle::PublicationSuppressed {
                duplicate_corrections: group.duplicate_corrections.clone(),
            }
        } else {
            OccurrenceBindingLifecycle::Current
        };

        views.push(OccurrenceBindingLineageView {
            occurrence_binding_digest: *binding_digest,
            administration_receipt_digest: group.administration_receipt_digest,
            event_digest: group.event_digest,
            occurrence_digest: group.occurrence_digest,
            medication_artifact_digest: group.medication_artifact_digest,
            dosage_index: group.dosage_index,
            publication_action_hashes,
            effective_publication_action_hashes,
            corrections: all_corrections,
            lifecycle,
        });
    }
    views.sort_by(|left, right| {
        digest_order(
            &left.occurrence_binding_digest,
            &right.occurrence_binding_digest,
        )
    });

    Ok((views, groups))
}

fn validate_binding_attestation_shape(
    binding: &QualifiedMedicationAdministrationOccurrenceBinding,
) -> Result<(), OccurrenceStateError> {
    let attestation = &binding.attestation;
    if attestation.schema_version != 1 {
        return Err(OccurrenceStateError::MalformedOccurrenceBinding);
    }
    require_domain(
        attestation.occurrence_binding_digest,
        DigestDomain::MedicationAdministrationOccurrenceBinding,
    )?;
    require_domain(
        attestation.administration_receipt_digest,
        DigestDomain::MedicationAdministrationReceipt,
    )?;
    require_domain(
        attestation.event_digest,
        DigestDomain::MedicationAdministrationEvent,
    )?;
    require_domain(
        attestation.occurrence_digest,
        DigestDomain::MedicationAdministrationOccurrence,
    )?;
    require_domain(
        attestation.medication_artifact_digest,
        DigestDomain::MedicationRequestArtifact,
    )?;
    Ok(())
}

fn validate_binding_correction_shape(
    correction: &MedicationAdministrationOccurrenceBindingCorrection,
) -> Result<(), OccurrenceStateError> {
    if correction.correction_id.trim().is_empty() {
        return Err(OccurrenceStateError::MalformedBindingCorrection);
    }
    require_domain(
        correction.occurrence_binding_digest,
        DigestDomain::MedicationAdministrationOccurrenceBinding,
    )?;
    require_domain(
        correction.occurrence_digest,
        DigestDomain::MedicationAdministrationOccurrence,
    )?;
    if correction
        .rationale_commitment
        .is_some_and(|commitment| commitment == [0u8; 32])
    {
        return Err(OccurrenceStateError::MalformedBindingCorrection);
    }
    if correction.reason == OccurrenceBindingCorrectionReason::Other
        && correction.rationale_commitment.is_none()
    {
        return Err(OccurrenceStateError::MalformedBindingCorrection);
    }
    Ok(())
}

fn require_domain(
    digest: StoredDigest,
    expected: DigestDomain,
) -> Result<(), OccurrenceStateError> {
    digest
        .validate_shape()
        .map_err(|_| OccurrenceStateError::InvalidDigest)?;
    if digest.domain != expected {
        return Err(OccurrenceStateError::WrongDigestDomain {
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

fn binding_correction_order(
    left: &OccurrenceBindingCorrectionView,
    right: &OccurrenceBindingCorrectionView,
) -> std::cmp::Ordering {
    action_hash_order(&left.correction_action_hash, &right.correction_action_hash)
}

fn digest_order(left: &StoredDigest, right: &StoredDigest) -> std::cmp::Ordering {
    left.value.cmp(&right.value)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OccurrenceStateError {
    #[error(transparent)]
    AdministrationState(#[from] AdministrationStateError),
    #[error("administration action hash was supplied with conflicting receipt payloads")]
    AdministrationActionPayloadConflict,
    #[error("qualified occurrence binding is malformed")]
    MalformedOccurrenceBinding,
    #[error("occurrence binding correction is malformed")]
    MalformedBindingCorrection,
    #[error("stored digest is invalid")]
    InvalidDigest,
    #[error("digest domain mismatch: expected {expected:?}, got {actual:?}")]
    WrongDigestDomain {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("occurrence binding references an administration absent from the supplied read set")]
    OrphanOccurrenceBinding,
    #[error("occurrence binding receipt does not match its referenced administration")]
    OccurrenceBindingReceiptMismatch,
    #[error("same occurrence-binding action hash was supplied with conflicting payloads")]
    BindingActionPayloadConflict,
    #[error("one private occurrence-binding digest is paired with conflicting semantic fields")]
    BindingDigestSemanticConflict,
    #[error("same occurrence-binding correction action hash has conflicting payloads")]
    BindingCorrectionActionPayloadConflict,
    #[error("occurrence-binding correction_id is reused by multiple actions")]
    DuplicateBindingCorrectionId,
    #[error("occurrence-binding correction target is absent from the supplied read set")]
    OrphanBindingCorrection,
    #[error("occurrence-binding correction private binding digest does not match target")]
    BindingCorrectionDigestMismatch,
    #[error("occurrence-binding correction occurrence digest does not match target")]
    BindingCorrectionOccurrenceMismatch,
    #[error("internal occurrence-binding lineage/view mismatch")]
    InternalBindingViewMismatch,
}
