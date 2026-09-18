use holo_hash::ActionHash;
use medication_dispense_integrity::{
    AllocatorEquivocationEvidence, FinalizedMedicationDispense,
    MedicationDispenseFinalAttestationV1, MedicationDispenseSlotAuthorization,
};
use mycelix_clinical_integrity::{DigestDomain, StoredDigest};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Clone)]
pub struct SlotAuthorizationRecord {
    pub action_hash: ActionHash,
    pub authorization: MedicationDispenseSlotAuthorization,
}

#[derive(Clone)]
pub struct FinalizedDispenseRecord {
    pub action_hash: ActionHash,
    pub finalized: FinalizedMedicationDispense,
}

#[derive(Clone)]
pub struct EquivocationEvidenceRecord {
    pub action_hash: ActionHash,
    pub evidence: AllocatorEquivocationEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct LineageKey {
    medication_artifact_digest: StoredDigest,
    activation_semantic_receipt_digest: StoredDigest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SlotKey {
    lineage: LineageKey,
    slot_index: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct FinalizedReceiptView {
    pub receipt_digest: StoredDigest,
    pub dispense_id: String,
    pub dispense_request_digest: StoredDigest,
    pub preflight_receipt_digest: StoredDigest,
    pub activation_state_digest: StoredDigest,
    pub pharmacist_principal_binding: [u8; 32],
    pub pharmacy_record_digest: StoredDigest,
    pub pharmacy_affiliation_evidence_digest: StoredDigest,
    pub pharmacy_status_evidence_digest: StoredDigest,
    pub authority_policy_digest: StoredDigest,
    pub dispense_policy_digest: StoredDigest,
    pub publication_action_hashes: Vec<ActionHash>,
    pub authorization_action_hashes: Vec<ActionHash>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum DispenseSlotState {
    Finalized(FinalizedReceiptView),
    /// Authorization evidence alone does not consume a refill slot and may already
    /// be expired outside the supplied read set.
    AuthorizationEvidenceOnly {
        authorization_action_hashes: Vec<ActionHash>,
    },
    /// Conflicting release authority and/or finalized outcomes are visible. There is
    /// deliberately no winner field.
    Conflict {
        finalized_receipts: Vec<FinalizedReceiptView>,
        authorization_action_hashes: Vec<ActionHash>,
        equivocation_evidence_action_hashes: Vec<ActionHash>,
    },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct DispenseSlotView {
    pub slot_index: u32,
    pub state: DispenseSlotState,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum DispenseLineageHealth {
    Consistent,
    Conflict { slots: Vec<u32> },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct DispenseLineageView {
    pub medication_artifact_digest: StoredDigest,
    pub activation_semantic_receipt_digest: StoredDigest,
    pub slots: Vec<DispenseSlotView>,
    /// Highest contiguous, conflict-free finalized slot beginning at zero.
    pub contiguous_finalized_through: Option<u32>,
    pub health: DispenseLineageHealth,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct MedicationDispenseView {
    pub lineages: Vec<DispenseLineageView>,
}

#[derive(Clone)]
struct FinalReceiptGroup {
    attestation: MedicationDispenseFinalAttestationV1,
    receipt_digest: StoredDigest,
    publication_action_hashes: Vec<ActionHash>,
    authorization_action_hashes: Vec<ActionHash>,
}

#[derive(Default)]
struct SlotAccumulator {
    authorization_action_hashes: Vec<ActionHash>,
    final_receipts: HashMap<StoredDigest, FinalReceiptGroup>,
    visible_authorization_conflict: bool,
    equivocation_evidence_action_hashes: Vec<ActionHash>,
}

/// Reduce a complete-for-purpose set of already DHT-valid dispense records.
///
/// This is deliberately not an integrity substitute and makes no global-absence
/// claim. It rechecks cross-record closure so incomplete/mismatched fetched sets do
/// not become a false clean refill ledger.
pub fn reduce_dispense_state(
    authorizations: &[SlotAuthorizationRecord],
    finalized: &[FinalizedDispenseRecord],
    equivocation_evidence: &[EquivocationEvidenceRecord],
) -> Result<MedicationDispenseView, DispenseStateError> {
    let mut authorization_by_action = HashMap::new();
    let mut authorizations_by_slot: HashMap<SlotKey, Vec<&SlotAuthorizationRecord>> = HashMap::new();

    for record in authorizations {
        validate_authorization_shape(&record.authorization)?;
        if authorization_by_action
            .insert(record.action_hash.clone(), &record.authorization)
            .is_some()
        {
            return Err(DispenseStateError::DuplicateAuthorizationActionHash);
        }
        authorizations_by_slot
            .entry(slot_key_from_authorization(&record.authorization))
            .or_default()
            .push(record);
    }

    let mut finalized_by_action: HashMap<ActionHash, (&FinalizedDispenseRecord, StoredDigest)> =
        HashMap::new();
    let mut slots: HashMap<SlotKey, SlotAccumulator> = HashMap::new();

    for (slot_key, records) in &authorizations_by_slot {
        let accumulator = slots.entry(*slot_key).or_default();
        for record in records {
            push_unique(
                &mut accumulator.authorization_action_hashes,
                record.action_hash.clone(),
            );
        }
        accumulator.visible_authorization_conflict = visible_authorizations_conflict(records);
    }

    for record in finalized {
        let authorization = authorization_by_action
            .get(&record.finalized.slot_authorization_hash)
            .ok_or(DispenseStateError::OrphanFinalizedDispense)?;
        validate_final_against_authorization(&record.finalized, authorization)?;

        let receipt_digest = record
            .finalized
            .attestation
            .receipt_digest()
            .map_err(|_| DispenseStateError::InvalidFinalReceiptDigest)?;
        require_domain(receipt_digest, DigestDomain::MedicationDispenseReceipt)?;

        if finalized_by_action
            .insert(record.action_hash.clone(), (record, receipt_digest))
            .is_some()
        {
            return Err(DispenseStateError::DuplicateFinalizedActionHash);
        }

        let key = slot_key_from_attestation(&record.finalized.attestation);
        if key != slot_key_from_authorization(authorization) {
            return Err(DispenseStateError::FinalizedAuthorizationLineageMismatch);
        }

        let accumulator = slots.entry(key).or_default();
        match accumulator.final_receipts.get_mut(&receipt_digest) {
            Some(group) => {
                if group.attestation != record.finalized.attestation {
                    return Err(DispenseStateError::ConflictingSemanticReceiptDuplicate);
                }
                push_unique(&mut group.publication_action_hashes, record.action_hash.clone());
                push_unique(
                    &mut group.authorization_action_hashes,
                    record.finalized.slot_authorization_hash.clone(),
                );
            }
            None => {
                accumulator.final_receipts.insert(
                    receipt_digest,
                    FinalReceiptGroup {
                        attestation: record.finalized.attestation.clone(),
                        receipt_digest,
                        publication_action_hashes: vec![record.action_hash.clone()],
                        authorization_action_hashes: vec![
                            record.finalized.slot_authorization_hash.clone(),
                        ],
                    },
                );
            }
        }
    }

    // A later refill authorization must close over the immediately preceding final
    // publication in the supplied set. Missing predecessor data is not interpreted as
    // absence of prior dispensing.
    for record in authorizations {
        validate_predecessor_closure(&record.authorization, &finalized_by_action)?;
    }

    let mut seen_equivocation_actions = HashSet::new();
    for record in equivocation_evidence {
        if !seen_equivocation_actions.insert(record.action_hash.clone()) {
            continue;
        }
        if record.evidence.evidence_id.trim().is_empty()
            || record.evidence.left_authorization_hash == record.evidence.right_authorization_hash
        {
            return Err(DispenseStateError::MalformedEquivocationEvidence);
        }
        let left = authorization_by_action
            .get(&record.evidence.left_authorization_hash)
            .ok_or(DispenseStateError::OrphanEquivocationEvidence)?;
        let right = authorization_by_action
            .get(&record.evidence.right_authorization_hash)
            .ok_or(DispenseStateError::OrphanEquivocationEvidence)?;
        let left_key = slot_key_from_authorization(left);
        let right_key = slot_key_from_authorization(right);
        if left_key != right_key {
            return Err(DispenseStateError::EquivocationSlotMismatch);
        }
        if !release_authorizations_conflict(left, right) {
            return Err(DispenseStateError::NonConflictingEquivocationEvidence);
        }
        push_unique(
            &mut slots
                .entry(left_key)
                .or_default()
                .equivocation_evidence_action_hashes,
            record.action_hash.clone(),
        );
    }

    let mut by_lineage: HashMap<LineageKey, Vec<(u32, SlotAccumulator)>> = HashMap::new();
    for (key, accumulator) in slots {
        by_lineage
            .entry(key.lineage)
            .or_default()
            .push((key.slot_index, accumulator));
    }

    let mut lineages = Vec::with_capacity(by_lineage.len());
    for (lineage_key, mut lineage_slots) in by_lineage {
        lineage_slots.sort_by_key(|(slot, _)| *slot);
        let mut views = Vec::with_capacity(lineage_slots.len());
        let mut conflict_slots = Vec::new();

        for (slot_index, mut accumulator) in lineage_slots {
            accumulator.authorization_action_hashes.sort_by(action_hash_order);
            accumulator
                .equivocation_evidence_action_hashes
                .sort_by(action_hash_order);

            let mut final_views: Vec<FinalizedReceiptView> = accumulator
                .final_receipts
                .into_values()
                .map(final_receipt_view)
                .collect();
            final_views.sort_by(|left, right| left.receipt_digest.value.cmp(&right.receipt_digest.value));

            let conflict = accumulator.visible_authorization_conflict
                || !accumulator.equivocation_evidence_action_hashes.is_empty()
                || final_views.len() > 1;

            let state = if conflict {
                conflict_slots.push(slot_index);
                DispenseSlotState::Conflict {
                    finalized_receipts: final_views,
                    authorization_action_hashes: accumulator.authorization_action_hashes,
                    equivocation_evidence_action_hashes: accumulator
                        .equivocation_evidence_action_hashes,
                }
            } else if let Some(finalized) = final_views.into_iter().next() {
                DispenseSlotState::Finalized(finalized)
            } else {
                DispenseSlotState::AuthorizationEvidenceOnly {
                    authorization_action_hashes: accumulator.authorization_action_hashes,
                }
            };
            views.push(DispenseSlotView { slot_index, state });
        }

        // A conflicting slot containing one or more final receipts is structurally
        // present for gap detection, but it never extends the conflict-free prefix.
        validate_no_structural_finalized_gaps(&views)?;
        let contiguous_finalized_through = contiguous_conflict_free_finalized_through(&views);
        let health = if conflict_slots.is_empty() {
            DispenseLineageHealth::Consistent
        } else {
            DispenseLineageHealth::Conflict {
                slots: conflict_slots,
            }
        };

        lineages.push(DispenseLineageView {
            medication_artifact_digest: lineage_key.medication_artifact_digest,
            activation_semantic_receipt_digest: lineage_key.activation_semantic_receipt_digest,
            slots: views,
            contiguous_finalized_through,
            health,
        });
    }

    lineages.sort_by(|left, right| {
        left.medication_artifact_digest
            .value
            .cmp(&right.medication_artifact_digest.value)
            .then_with(|| {
                activation_domain_rank(left.activation_semantic_receipt_digest.domain)
                    .cmp(&activation_domain_rank(
                        right.activation_semantic_receipt_digest.domain,
                    ))
            })
            .then_with(|| {
                left.activation_semantic_receipt_digest
                    .value
                    .cmp(&right.activation_semantic_receipt_digest.value)
            })
    });

    Ok(MedicationDispenseView { lineages })
}

fn validate_authorization_shape(
    authorization: &MedicationDispenseSlotAuthorization,
) -> Result<(), DispenseStateError> {
    if authorization.schema_version != 1 || authorization.authorization_id.trim().is_empty() {
        return Err(DispenseStateError::MalformedAuthorization);
    }
    if authorization.pharmacist_principal_binding == [0u8; 32] {
        return Err(DispenseStateError::MalformedAuthorization);
    }
    require_domain(
        authorization.medication_artifact_digest,
        DigestDomain::MedicationRequestArtifact,
    )?;
    require_activation_receipt_domain(authorization.activation_semantic_receipt_digest)?;
    require_domain(
        authorization.activation_state_digest,
        DigestDomain::MedicationActivationState,
    )?;
    require_domain(
        authorization.dispense_request_digest,
        DigestDomain::MedicationDispenseRequest,
    )?;
    require_domain(
        authorization.preflight_receipt_digest,
        DigestDomain::MedicationDispensePreflightReceipt,
    )?;
    require_domain(authorization.pharmacy_record_digest, DigestDomain::PharmacyRecord)?;
    require_domain(
        authorization.pharmacy_affiliation_evidence_digest,
        DigestDomain::PharmacyAffiliationEvidence,
    )?;
    require_domain(
        authorization.pharmacy_status_evidence_digest,
        DigestDomain::PharmacyStatusEvidence,
    )?;
    require_domain(authorization.authority_policy_digest, DigestDomain::AuthorityPolicy)?;
    require_domain(
        authorization.dispense_policy_digest,
        DigestDomain::MedicationDispensePolicy,
    )?;
    require_domain(
        authorization.final_receipt_digest,
        DigestDomain::MedicationDispenseReceipt,
    )?;
    match (authorization.slot_index, &authorization.previous_final_dispense_hash) {
        (0, None) | (1.., Some(_)) => Ok(()),
        _ => Err(DispenseStateError::MalformedAuthorization),
    }
}

fn validate_final_against_authorization(
    finalized: &FinalizedMedicationDispense,
    authorization: &MedicationDispenseSlotAuthorization,
) -> Result<(), DispenseStateError> {
    validate_attestation_shape(&finalized.attestation)?;
    let digest = finalized
        .attestation
        .receipt_digest()
        .map_err(|_| DispenseStateError::InvalidFinalReceiptDigest)?;
    if digest != authorization.final_receipt_digest {
        return Err(DispenseStateError::FinalReceiptAuthorizationMismatch);
    }
    let attestation = &finalized.attestation;
    if attestation.medication_artifact_digest != authorization.medication_artifact_digest
        || attestation.activation_semantic_receipt_digest
            != authorization.activation_semantic_receipt_digest
        || attestation.activation_state_digest != authorization.activation_state_digest
        || attestation.dispense_request_digest != authorization.dispense_request_digest
        || attestation.preflight_receipt_digest != authorization.preflight_receipt_digest
        || attestation.slot_index != authorization.slot_index
        || attestation.pharmacist_principal_binding
            != authorization.pharmacist_principal_binding
        || attestation.pharmacy_record_digest != authorization.pharmacy_record_digest
        || attestation.pharmacy_affiliation_evidence_digest
            != authorization.pharmacy_affiliation_evidence_digest
        || attestation.pharmacy_status_evidence_digest
            != authorization.pharmacy_status_evidence_digest
        || attestation.authority_policy_digest != authorization.authority_policy_digest
        || attestation.dispense_policy_digest != authorization.dispense_policy_digest
    {
        return Err(DispenseStateError::FinalizedAuthorizationLineageMismatch);
    }
    Ok(())
}

fn validate_attestation_shape(
    attestation: &MedicationDispenseFinalAttestationV1,
) -> Result<(), DispenseStateError> {
    if attestation.schema_version != 1 || attestation.dispense_id.trim().is_empty() {
        return Err(DispenseStateError::MalformedFinalizedDispense);
    }
    if attestation.pharmacist_principal_binding == [0u8; 32] {
        return Err(DispenseStateError::MalformedFinalizedDispense);
    }
    require_domain(
        attestation.medication_artifact_digest,
        DigestDomain::MedicationRequestArtifact,
    )?;
    require_activation_receipt_domain(attestation.activation_semantic_receipt_digest)?;
    require_domain(
        attestation.activation_state_digest,
        DigestDomain::MedicationActivationState,
    )?;
    require_domain(
        attestation.dispense_request_digest,
        DigestDomain::MedicationDispenseRequest,
    )?;
    require_domain(
        attestation.preflight_receipt_digest,
        DigestDomain::MedicationDispensePreflightReceipt,
    )?;
    require_domain(attestation.pharmacy_record_digest, DigestDomain::PharmacyRecord)?;
    require_domain(
        attestation.pharmacy_affiliation_evidence_digest,
        DigestDomain::PharmacyAffiliationEvidence,
    )?;
    require_domain(
        attestation.pharmacy_status_evidence_digest,
        DigestDomain::PharmacyStatusEvidence,
    )?;
    require_domain(attestation.authority_policy_digest, DigestDomain::AuthorityPolicy)?;
    require_domain(
        attestation.dispense_policy_digest,
        DigestDomain::MedicationDispensePolicy,
    )
}

fn validate_predecessor_closure(
    authorization: &MedicationDispenseSlotAuthorization,
    finalized_by_action: &HashMap<ActionHash, (&FinalizedDispenseRecord, StoredDigest)>,
) -> Result<(), DispenseStateError> {
    if authorization.slot_index == 0 {
        return Ok(());
    }
    let previous_hash = authorization
        .previous_final_dispense_hash
        .as_ref()
        .ok_or(DispenseStateError::MissingPredecessor)?;
    let (previous_record, _) = finalized_by_action
        .get(previous_hash)
        .ok_or(DispenseStateError::IncompletePredecessorReadSet)?;
    let previous = &previous_record.finalized.attestation;
    let expected_slot = previous
        .slot_index
        .checked_add(1)
        .ok_or(DispenseStateError::SlotOverflow)?;
    if expected_slot != authorization.slot_index {
        return Err(DispenseStateError::PredecessorSlotMismatch);
    }
    if previous.medication_artifact_digest != authorization.medication_artifact_digest
        || previous.activation_semantic_receipt_digest
            != authorization.activation_semantic_receipt_digest
    {
        return Err(DispenseStateError::PredecessorLineageMismatch);
    }
    Ok(())
}

fn visible_authorizations_conflict(records: &[&SlotAuthorizationRecord]) -> bool {
    let Some(first) = records.first() else {
        return false;
    };
    records.iter().skip(1).any(|record| {
        release_authorizations_conflict(&first.authorization, &record.authorization)
    })
}

/// Retry authorization IDs/windows may differ. They are not contradictory when the
/// exact same grantee is authorized to publish the exact same final semantic receipt.
fn release_authorizations_conflict(
    left: &MedicationDispenseSlotAuthorization,
    right: &MedicationDispenseSlotAuthorization,
) -> bool {
    left.final_receipt_digest != right.final_receipt_digest || left.grantee != right.grantee
}

fn validate_no_structural_finalized_gaps(
    slots: &[DispenseSlotView],
) -> Result<(), DispenseStateError> {
    let finalized_slots: Vec<u32> = slots
        .iter()
        .filter(|slot| slot_has_any_finalized_receipt(&slot.state))
        .map(|slot| slot.slot_index)
        .collect();
    for (expected, actual) in finalized_slots.iter().enumerate() {
        let expected = u32::try_from(expected).map_err(|_| DispenseStateError::SlotOverflow)?;
        if *actual != expected {
            return Err(DispenseStateError::IncompleteFinalizedHistory);
        }
    }
    Ok(())
}

fn slot_has_any_finalized_receipt(state: &DispenseSlotState) -> bool {
    match state {
        DispenseSlotState::Finalized(_) => true,
        DispenseSlotState::AuthorizationEvidenceOnly { .. } => false,
        DispenseSlotState::Conflict {
            finalized_receipts, ..
        } => !finalized_receipts.is_empty(),
    }
}

fn contiguous_conflict_free_finalized_through(slots: &[DispenseSlotView]) -> Option<u32> {
    let mut expected = 0u32;
    let mut through = None;
    for slot in slots {
        if slot.slot_index != expected {
            break;
        }
        match &slot.state {
            DispenseSlotState::Finalized(_) => {
                through = Some(expected);
                let Some(next) = expected.checked_add(1) else {
                    break;
                };
                expected = next;
            }
            DispenseSlotState::AuthorizationEvidenceOnly { .. }
            | DispenseSlotState::Conflict { .. } => break,
        }
    }
    through
}

fn final_receipt_view(mut group: FinalReceiptGroup) -> FinalizedReceiptView {
    group.publication_action_hashes.sort_by(action_hash_order);
    group.authorization_action_hashes.sort_by(action_hash_order);
    let attestation = group.attestation;
    FinalizedReceiptView {
        receipt_digest: group.receipt_digest,
        dispense_id: attestation.dispense_id,
        dispense_request_digest: attestation.dispense_request_digest,
        preflight_receipt_digest: attestation.preflight_receipt_digest,
        activation_state_digest: attestation.activation_state_digest,
        pharmacist_principal_binding: attestation.pharmacist_principal_binding,
        pharmacy_record_digest: attestation.pharmacy_record_digest,
        pharmacy_affiliation_evidence_digest: attestation.pharmacy_affiliation_evidence_digest,
        pharmacy_status_evidence_digest: attestation.pharmacy_status_evidence_digest,
        authority_policy_digest: attestation.authority_policy_digest,
        dispense_policy_digest: attestation.dispense_policy_digest,
        publication_action_hashes: group.publication_action_hashes,
        authorization_action_hashes: group.authorization_action_hashes,
    }
}

fn slot_key_from_authorization(authorization: &MedicationDispenseSlotAuthorization) -> SlotKey {
    SlotKey {
        lineage: LineageKey {
            medication_artifact_digest: authorization.medication_artifact_digest,
            activation_semantic_receipt_digest: authorization.activation_semantic_receipt_digest,
        },
        slot_index: authorization.slot_index,
    }
}

fn slot_key_from_attestation(attestation: &MedicationDispenseFinalAttestationV1) -> SlotKey {
    SlotKey {
        lineage: LineageKey {
            medication_artifact_digest: attestation.medication_artifact_digest,
            activation_semantic_receipt_digest: attestation.activation_semantic_receipt_digest,
        },
        slot_index: attestation.slot_index,
    }
}

fn require_activation_receipt_domain(digest: StoredDigest) -> Result<(), DispenseStateError> {
    digest
        .validate_shape()
        .map_err(|_| DispenseStateError::InvalidDigest)?;
    if !matches!(
        digest.domain,
        DigestDomain::MedicationActivationReceipt
            | DigestDomain::EmergencyMedicationOverrideReceipt
    ) {
        return Err(DispenseStateError::WrongActivationReceiptDomain);
    }
    Ok(())
}

fn require_domain(digest: StoredDigest, expected: DigestDomain) -> Result<(), DispenseStateError> {
    digest
        .validate_shape()
        .map_err(|_| DispenseStateError::InvalidDigest)?;
    if digest.domain != expected {
        return Err(DispenseStateError::WrongDigestDomain {
            expected,
            actual: digest.domain,
        });
    }
    Ok(())
}

fn activation_domain_rank(domain: DigestDomain) -> u8 {
    match domain {
        DigestDomain::MedicationActivationReceipt => 0,
        DigestDomain::EmergencyMedicationOverrideReceipt => 1,
        _ => 2,
    }
}

fn action_hash_order(left: &ActionHash, right: &ActionHash) -> std::cmp::Ordering {
    left.get_raw_36().cmp(right.get_raw_36())
}

fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DispenseStateError {
    #[error("slot authorization is malformed")]
    MalformedAuthorization,
    #[error("finalized dispense is malformed")]
    MalformedFinalizedDispense,
    #[error("finalized dispense receipt digest could not be derived")]
    InvalidFinalReceiptDigest,
    #[error("stored digest is invalid")]
    InvalidDigest,
    #[error("digest domain mismatch: expected {expected:?}, got {actual:?}")]
    WrongDigestDomain {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("activation semantic receipt has invalid digest domain")]
    WrongActivationReceiptDomain,
    #[error("duplicate slot-authorization action hash in supplied read set")]
    DuplicateAuthorizationActionHash,
    #[error("duplicate finalized-dispense action hash in supplied read set")]
    DuplicateFinalizedActionHash,
    #[error("finalized dispense has no supplied slot authorization")]
    OrphanFinalizedDispense,
    #[error("finalized dispense does not match its exact slot authorization")]
    FinalizedAuthorizationLineageMismatch,
    #[error("final semantic receipt digest does not match slot authorization")]
    FinalReceiptAuthorizationMismatch,
    #[error("same semantic final receipt digest is paired with changed attestation fields")]
    ConflictingSemanticReceiptDuplicate,
    #[error("refill authorization is missing predecessor")]
    MissingPredecessor,
    #[error("refill predecessor is not present in the supplied finalized read set")]
    IncompletePredecessorReadSet,
    #[error("refill predecessor slot does not immediately precede authorization")]
    PredecessorSlotMismatch,
    #[error("refill predecessor belongs to another medication activation lineage")]
    PredecessorLineageMismatch,
    #[error("slot index overflow")]
    SlotOverflow,
    #[error("equivocation evidence is malformed")]
    MalformedEquivocationEvidence,
    #[error("equivocation evidence references authorization not present in supplied read set")]
    OrphanEquivocationEvidence,
    #[error("equivocation evidence references different semantic refill slots")]
    EquivocationSlotMismatch,
    #[error("equivocation evidence references retry-equivalent release authority")]
    NonConflictingEquivocationEvidence,
    #[error("supplied finalized history has a gap before a later finalized slot")]
    IncompleteFinalizedHistory,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hdi::prelude::Timestamp;
    use holo_hash::AgentPubKey;
    use mycelix_clinical_integrity::DigestAlgorithm;

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

    fn agent(seed: u8) -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![seed; 36])
    }

    fn attestation(slot: u32, final_seed: u8) -> MedicationDispenseFinalAttestationV1 {
        MedicationDispenseFinalAttestationV1 {
            schema_version: 1,
            dispense_id: format!("dispense-{slot}-{final_seed}"),
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            activation_semantic_receipt_digest: stored(DigestDomain::MedicationActivationReceipt, 2),
            activation_state_digest: stored(DigestDomain::MedicationActivationState, 3),
            dispense_request_digest: stored(DigestDomain::MedicationDispenseRequest, final_seed),
            preflight_receipt_digest: stored(
                DigestDomain::MedicationDispensePreflightReceipt,
                final_seed.wrapping_add(1),
            ),
            slot_index: slot,
            pharmacist_principal_binding: [6; 32],
            pharmacy_record_digest: stored(DigestDomain::PharmacyRecord, 7),
            pharmacy_affiliation_evidence_digest: stored(
                DigestDomain::PharmacyAffiliationEvidence,
                8,
            ),
            pharmacy_status_evidence_digest: stored(DigestDomain::PharmacyStatusEvidence, 9),
            authority_policy_digest: stored(DigestDomain::AuthorityPolicy, 10),
            dispense_policy_digest: stored(DigestDomain::MedicationDispensePolicy, 11),
        }
    }

    fn authorization(
        action_hash: ActionHash,
        slot: u32,
        final_seed: u8,
        grantee_seed: u8,
        previous: Option<ActionHash>,
    ) -> SlotAuthorizationRecord {
        let final_attestation = attestation(slot, final_seed);
        SlotAuthorizationRecord {
            action_hash,
            authorization: MedicationDispenseSlotAuthorization {
                schema_version: 1,
                authorization_id: format!("auth-{slot}-{final_seed}"),
                grantee: agent(grantee_seed),
                medication_artifact_digest: final_attestation.medication_artifact_digest,
                activation_semantic_receipt_digest: final_attestation
                    .activation_semantic_receipt_digest,
                activation_state_digest: final_attestation.activation_state_digest,
                dispense_request_digest: final_attestation.dispense_request_digest,
                preflight_receipt_digest: final_attestation.preflight_receipt_digest,
                slot_index: slot,
                previous_final_dispense_hash: previous,
                pharmacist_principal_binding: final_attestation.pharmacist_principal_binding,
                pharmacy_record_digest: final_attestation.pharmacy_record_digest,
                pharmacy_affiliation_evidence_digest: final_attestation
                    .pharmacy_affiliation_evidence_digest,
                pharmacy_status_evidence_digest: final_attestation.pharmacy_status_evidence_digest,
                authority_policy_digest: final_attestation.authority_policy_digest,
                dispense_policy_digest: final_attestation.dispense_policy_digest,
                final_receipt_digest: final_attestation.receipt_digest().unwrap(),
                valid_from: Timestamp::from_micros(10),
                valid_until: Timestamp::from_micros(100),
            },
        }
    }

    fn finalized(
        action_hash: ActionHash,
        authorization: &SlotAuthorizationRecord,
        final_seed: u8,
    ) -> FinalizedDispenseRecord {
        FinalizedDispenseRecord {
            action_hash,
            finalized: FinalizedMedicationDispense {
                attestation: attestation(authorization.authorization.slot_index, final_seed),
                slot_authorization_hash: authorization.action_hash.clone(),
            },
        }
    }

    #[test]
    fn duplicate_publication_of_same_receipt_is_idempotent() {
        let auth = authorization(action(1), 0, 20, 30, None);
        let first = finalized(action(2), &auth, 20);
        let second = finalized(action(3), &auth, 20);
        let view = reduce_dispense_state(&[auth], &[first, second], &[]).unwrap();
        match &view.lineages[0].slots[0].state {
            DispenseSlotState::Finalized(finalized) => {
                assert_eq!(finalized.publication_action_hashes.len(), 2);
            }
            _ => panic!("expected one idempotent finalized semantic receipt"),
        }
    }

    #[test]
    fn two_different_final_receipts_for_same_slot_are_conflict() {
        let auth_a = authorization(action(1), 0, 20, 30, None);
        let auth_b = authorization(action(2), 0, 21, 30, None);
        let final_a = finalized(action(3), &auth_a, 20);
        let final_b = finalized(action(4), &auth_b, 21);
        let view = reduce_dispense_state(&[auth_a, auth_b], &[final_a, final_b], &[]).unwrap();
        assert!(matches!(
            &view.lineages[0].health,
            DispenseLineageHealth::Conflict { .. }
        ));
        assert!(matches!(
            &view.lineages[0].slots[0].state,
            DispenseSlotState::Conflict { .. }
        ));
        assert_eq!(view.lineages[0].contiguous_finalized_through, None);
    }

    #[test]
    fn same_final_receipt_and_grantee_retry_is_not_equivocation() {
        let auth_a = authorization(action(1), 0, 20, 30, None);
        let mut auth_b = authorization(action(2), 0, 20, 30, None);
        auth_b.authorization.authorization_id = "retry-id".into();
        auth_b.authorization.valid_from = Timestamp::from_micros(200);
        auth_b.authorization.valid_until = Timestamp::from_micros(250);
        assert!(!release_authorizations_conflict(
            &auth_a.authorization,
            &auth_b.authorization
        ));
        auth_b.authorization.grantee = agent(31);
        assert!(release_authorizations_conflict(
            &auth_a.authorization,
            &auth_b.authorization
        ));
    }

    #[test]
    fn refill_requires_predecessor_in_supplied_read_set() {
        let auth = authorization(action(10), 1, 21, 30, Some(action(99)));
        assert!(matches!(
            reduce_dispense_state(&[auth], &[], &[]),
            Err(DispenseStateError::IncompletePredecessorReadSet)
        ));
    }

    #[test]
    fn explicit_equivocation_contaminates_slot_even_before_finalization() {
        let auth_a = authorization(action(1), 0, 20, 30, None);
        let auth_b = authorization(action(2), 0, 21, 30, None);
        let evidence = EquivocationEvidenceRecord {
            action_hash: action(3),
            evidence: AllocatorEquivocationEvidence {
                evidence_id: "eq-1".into(),
                left_authorization_hash: auth_a.action_hash.clone(),
                right_authorization_hash: auth_b.action_hash.clone(),
            },
        };
        let view = reduce_dispense_state(&[auth_a, auth_b], &[], &[evidence]).unwrap();
        assert!(matches!(
            &view.lineages[0].slots[0].state,
            DispenseSlotState::Conflict { .. }
        ));
    }

    #[test]
    fn benign_retry_cannot_be_relabelled_as_equivocation() {
        let auth_a = authorization(action(1), 0, 20, 30, None);
        let mut auth_b = authorization(action(2), 0, 20, 30, None);
        auth_b.authorization.authorization_id = "retry".into();
        let evidence = EquivocationEvidenceRecord {
            action_hash: action(3),
            evidence: AllocatorEquivocationEvidence {
                evidence_id: "eq-invalid".into(),
                left_authorization_hash: auth_a.action_hash.clone(),
                right_authorization_hash: auth_b.action_hash.clone(),
            },
        };
        assert!(matches!(
            reduce_dispense_state(&[auth_a, auth_b], &[], &[evidence]),
            Err(DispenseStateError::NonConflictingEquivocationEvidence)
        ));
    }
}
