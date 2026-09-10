use holo_hash::ActionHash;
use medication_activation_integrity::{
    ActivationRevocationReason, MedicationActivationAttestationV1,
    MedicationActivationRevocation, QualifiedMedicationActivation,
};
use medication_emergency_integrity::{
    EmergencyActivationTerminationReason, EmergencyMedicationActivation,
    EmergencyMedicationAttestationV1, EmergencyMedicationTermination, EmergencySafetyBasis,
};
use mycelix_clinical_integrity::{DigestDomain, StoredDigest};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Clone)]
pub struct QualifiedActivationRecord {
    pub action_hash: ActionHash,
    pub activation: QualifiedMedicationActivation,
}

#[derive(Clone)]
pub struct QualifiedRevocationRecord {
    pub action_hash: ActionHash,
    pub revocation: MedicationActivationRevocation,
}

#[derive(Clone)]
pub struct EmergencyActivationRecord {
    pub action_hash: ActionHash,
    pub activation: EmergencyMedicationActivation,
}

#[derive(Clone)]
pub struct EmergencyTerminationRecord {
    pub action_hash: ActionHash,
    pub termination: EmergencyMedicationTermination,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub enum LegacyPrescriptionStatus {
    Active,
    Completed,
    Discontinued,
    OnHold,
    Cancelled,
    Expired,
    EnteredInError,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct LegacyPrescriptionRecord {
    pub action_hash: ActionHash,
    pub prescription_id: String,
    pub status: LegacyPrescriptionStatus,
}

impl LegacyPrescriptionRecord {
    pub fn validate(&self) -> Result<(), ActivationStateError> {
        if self.prescription_id.trim().is_empty() {
            return Err(ActivationStateError::MissingLegacyPrescriptionId);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum QualifiedTerminationReason {
    Superseded,
    MedicationStopped,
    SafetyEvidenceRevoked,
    ProfessionalAuthorityRevoked,
    SourceTrustRevoked,
    EnteredInError,
    Other,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum EmergencyTerminationReason {
    MedicationStopped,
    QualifiedActivationSuperseded,
    SafetyEvidenceResolved,
    ProfessionalAuthorityRevoked,
    EnteredInError,
    Other,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct QualifiedTerminationEvidence {
    pub action_hash: ActionHash,
    pub revocation_id: String,
    pub reason: QualifiedTerminationReason,
    pub reason_commitment: Option<[u8; 32]>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct EmergencyTerminationEvidence {
    pub action_hash: ActionHash,
    pub termination_id: String,
    pub reason: EmergencyTerminationReason,
    pub reason_commitment: Option<[u8; 32]>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum ActivationLifecycle<T> {
    Current,
    Terminated { events: Vec<T> },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct QualifiedActivationLineage {
    pub activation_id: String,
    pub activation_action_hashes: Vec<ActionHash>,
    pub medication_artifact_digest: StoredDigest,
    pub activation_receipt_digest: StoredDigest,
    pub safety_context_digest: StoredDigest,
    pub safety_trust_receipt_digest: StoredDigest,
    pub authority_policy_digest: StoredDigest,
    pub safety_policy_digest: StoredDigest,
    pub safety_trust_policy_digest: StoredDigest,
    pub workflow_policy_digest: StoredDigest,
    pub lifecycle: ActivationLifecycle<QualifiedTerminationEvidence>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct EmergencyActivationLineage {
    pub activation_id: String,
    pub activation_action_hashes: Vec<ActionHash>,
    pub medication_artifact_digest: StoredDigest,
    pub override_receipt_digest: StoredDigest,
    pub safety_assessment_digest: StoredDigest,
    pub safety_policy_digest: StoredDigest,
    pub authority_policy_digest: StoredDigest,
    pub emergency_policy_digest: StoredDigest,
    pub safety_basis: EmergencySafetyBasis,
    pub lifecycle: ActivationLifecycle<EmergencyTerminationEvidence>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum CurrentActivation {
    Qualified(QualifiedActivationLineage),
    EmergencyOverride(EmergencyActivationLineage),
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum CurrentActivationState {
    None,
    One(CurrentActivation),
    Conflict { current: Vec<CurrentActivation> },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum HistoricalActivation {
    Qualified(QualifiedActivationLineage),
    EmergencyOverride(EmergencyActivationLineage),
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct MedicationActivationView {
    pub current: CurrentActivationState,
    pub history: Vec<HistoricalActivation>,
    pub legacy_unqualified: Vec<LegacyPrescriptionRecord>,
}

#[derive(Clone)]
struct QualifiedGroup {
    attestation: MedicationActivationAttestationV1,
    action_hashes: Vec<ActionHash>,
    terminations: Vec<QualifiedTerminationEvidence>,
}

#[derive(Clone)]
struct EmergencyGroup {
    attestation: EmergencyMedicationAttestationV1,
    action_hashes: Vec<ActionHash>,
    terminations: Vec<EmergencyTerminationEvidence>,
}

pub fn reduce_activation_state(
    qualified: &[QualifiedActivationRecord],
    qualified_revocations: &[QualifiedRevocationRecord],
    emergency: &[EmergencyActivationRecord],
    emergency_terminations: &[EmergencyTerminationRecord],
    legacy: &[LegacyPrescriptionRecord],
) -> Result<MedicationActivationView, ActivationStateError> {
    let (mut qualified_groups, qualified_action_to_receipt) =
        collect_qualified_activations(qualified)?;
    apply_qualified_revocations(
        &mut qualified_groups,
        &qualified_action_to_receipt,
        qualified_revocations,
    )?;

    let (mut emergency_groups, emergency_action_to_receipt) =
        collect_emergency_activations(emergency)?;
    apply_emergency_terminations(
        &mut emergency_groups,
        &emergency_action_to_receipt,
        emergency_terminations,
    )?;

    let mut current = Vec::new();
    let mut history = Vec::new();

    for group in qualified_groups.into_values() {
        let lineage = qualified_lineage(group);
        if matches!(&lineage.lifecycle, ActivationLifecycle::Current) {
            current.push(CurrentActivation::Qualified(lineage));
        } else {
            history.push(HistoricalActivation::Qualified(lineage));
        }
    }

    for group in emergency_groups.into_values() {
        let lineage = emergency_lineage(group);
        if matches!(&lineage.lifecycle, ActivationLifecycle::Current) {
            current.push(CurrentActivation::EmergencyOverride(lineage));
        } else {
            history.push(HistoricalActivation::EmergencyOverride(lineage));
        }
    }

    sort_current(&mut current);
    sort_history(&mut history);

    let current = match current.len() {
        0 => CurrentActivationState::None,
        1 => CurrentActivationState::One(current.remove(0)),
        _ => CurrentActivationState::Conflict { current },
    };

    let legacy_unqualified = collect_legacy_active(legacy)?;

    Ok(MedicationActivationView {
        current,
        history,
        legacy_unqualified,
    })
}

fn collect_qualified_activations(
    records: &[QualifiedActivationRecord],
) -> Result<
    (
        HashMap<StoredDigest, QualifiedGroup>,
        HashMap<ActionHash, StoredDigest>,
    ),
    ActivationStateError,
> {
    let mut groups = HashMap::new();
    let mut action_to_receipt = HashMap::new();

    for record in records {
        validate_qualified_attestation(&record.activation.attestation)?;
        let receipt = record.activation.attestation.activation_receipt_digest;
        if action_to_receipt
            .insert(record.action_hash.clone(), receipt)
            .is_some()
        {
            return Err(ActivationStateError::DuplicateActivationActionHash);
        }

        match groups.get_mut(&receipt) {
            Some(group: &mut QualifiedGroup) => {
                if group.attestation != record.activation.attestation {
                    return Err(ActivationStateError::ConflictingQualifiedSemanticDuplicate);
                }
                group.action_hashes.push(record.action_hash.clone());
            }
            None => {
                groups.insert(
                    receipt,
                    QualifiedGroup {
                        attestation: record.activation.attestation.clone(),
                        action_hashes: vec![record.action_hash.clone()],
                        terminations: Vec::new(),
                    },
                );
            }
        }
    }

    Ok((groups, action_to_receipt))
}

fn apply_qualified_revocations(
    groups: &mut HashMap<StoredDigest, QualifiedGroup>,
    action_to_receipt: &HashMap<ActionHash, StoredDigest>,
    revocations: &[QualifiedRevocationRecord],
) -> Result<(), ActivationStateError> {
    let mut seen_actions = HashSet::new();

    for record in revocations {
        if !seen_actions.insert(record.action_hash.clone()) {
            continue;
        }
        validate_qualified_revocation(&record.revocation)?;
        let expected_receipt = action_to_receipt
            .get(&record.revocation.activation_hash)
            .ok_or(ActivationStateError::OrphanQualifiedRevocation)?;
        if *expected_receipt != record.revocation.activation_receipt_digest {
            return Err(ActivationStateError::QualifiedRevocationReceiptMismatch);
        }
        let group = groups
            .get_mut(expected_receipt)
            .ok_or(ActivationStateError::OrphanQualifiedRevocation)?;
        group.terminations.push(QualifiedTerminationEvidence {
            action_hash: record.action_hash.clone(),
            revocation_id: record.revocation.revocation_id.clone(),
            reason: map_qualified_reason(&record.revocation.reason),
            reason_commitment: record.revocation.reason_commitment,
        });
    }

    Ok(())
}

fn collect_emergency_activations(
    records: &[EmergencyActivationRecord],
) -> Result<
    (
        HashMap<StoredDigest, EmergencyGroup>,
        HashMap<ActionHash, StoredDigest>,
    ),
    ActivationStateError,
> {
    let mut groups = HashMap::new();
    let mut action_to_receipt = HashMap::new();

    for record in records {
        validate_emergency_attestation(&record.activation.attestation)?;
        let receipt = record.activation.attestation.override_receipt_digest;
        if action_to_receipt
            .insert(record.action_hash.clone(), receipt)
            .is_some()
        {
            return Err(ActivationStateError::DuplicateActivationActionHash);
        }

        match groups.get_mut(&receipt) {
            Some(group: &mut EmergencyGroup) => {
                if group.attestation != record.activation.attestation {
                    return Err(ActivationStateError::ConflictingEmergencySemanticDuplicate);
                }
                group.action_hashes.push(record.action_hash.clone());
            }
            None => {
                groups.insert(
                    receipt,
                    EmergencyGroup {
                        attestation: record.activation.attestation.clone(),
                        action_hashes: vec![record.action_hash.clone()],
                        terminations: Vec::new(),
                    },
                );
            }
        }
    }

    Ok((groups, action_to_receipt))
}

fn apply_emergency_terminations(
    groups: &mut HashMap<StoredDigest, EmergencyGroup>,
    action_to_receipt: &HashMap<ActionHash, StoredDigest>,
    terminations: &[EmergencyTerminationRecord],
) -> Result<(), ActivationStateError> {
    let mut seen_actions = HashSet::new();

    for record in terminations {
        if !seen_actions.insert(record.action_hash.clone()) {
            continue;
        }
        validate_emergency_termination(&record.termination)?;
        let expected_receipt = action_to_receipt
            .get(&record.termination.activation_hash)
            .ok_or(ActivationStateError::OrphanEmergencyTermination)?;
        if *expected_receipt != record.termination.override_receipt_digest {
            return Err(ActivationStateError::EmergencyTerminationReceiptMismatch);
        }
        let group = groups
            .get_mut(expected_receipt)
            .ok_or(ActivationStateError::OrphanEmergencyTermination)?;
        group.terminations.push(EmergencyTerminationEvidence {
            action_hash: record.action_hash.clone(),
            termination_id: record.termination.termination_id.clone(),
            reason: map_emergency_reason(&record.termination.reason),
            reason_commitment: record.termination.reason_commitment,
        });
    }

    Ok(())
}

fn collect_legacy_active(
    legacy: &[LegacyPrescriptionRecord],
) -> Result<Vec<LegacyPrescriptionRecord>, ActivationStateError> {
    let mut seen_actions = HashSet::new();
    let mut result = Vec::new();

    for record in legacy {
        record.validate()?;
        if !seen_actions.insert(record.action_hash.clone()) {
            continue;
        }
        if record.status == LegacyPrescriptionStatus::Active {
            result.push(record.clone());
        }
    }

    result.sort_by(|left, right| left.prescription_id.cmp(&right.prescription_id));
    Ok(result)
}

fn qualified_lineage(mut group: QualifiedGroup) -> QualifiedActivationLineage {
    group
        .action_hashes
        .sort_by(|left, right| left.get_raw_36().cmp(right.get_raw_36()));
    group
        .terminations
        .sort_by(|left, right| left.revocation_id.cmp(&right.revocation_id));

    let lifecycle = if group.terminations.is_empty() {
        ActivationLifecycle::Current
    } else {
        ActivationLifecycle::Terminated {
            events: group.terminations,
        }
    };
    let attestation = group.attestation;

    QualifiedActivationLineage {
        activation_id: attestation.activation_id,
        activation_action_hashes: group.action_hashes,
        medication_artifact_digest: attestation.medication_artifact_digest,
        activation_receipt_digest: attestation.activation_receipt_digest,
        safety_context_digest: attestation.safety_context_digest,
        safety_trust_receipt_digest: attestation.safety_trust_receipt_digest,
        authority_policy_digest: attestation.authority_policy_digest,
        safety_policy_digest: attestation.safety_policy_digest,
        safety_trust_policy_digest: attestation.safety_trust_policy_digest,
        workflow_policy_digest: attestation.workflow_policy_digest,
        lifecycle,
    }
}

fn emergency_lineage(mut group: EmergencyGroup) -> EmergencyActivationLineage {
    group
        .action_hashes
        .sort_by(|left, right| left.get_raw_36().cmp(right.get_raw_36()));
    group
        .terminations
        .sort_by(|left, right| left.termination_id.cmp(&right.termination_id));

    let lifecycle = if group.terminations.is_empty() {
        ActivationLifecycle::Current
    } else {
        ActivationLifecycle::Terminated {
            events: group.terminations,
        }
    };
    let attestation = group.attestation;

    EmergencyActivationLineage {
        activation_id: attestation.activation_id,
        activation_action_hashes: group.action_hashes,
        medication_artifact_digest: attestation.medication_artifact_digest,
        override_receipt_digest: attestation.override_receipt_digest,
        safety_assessment_digest: attestation.safety_assessment_digest,
        safety_policy_digest: attestation.safety_policy_digest,
        authority_policy_digest: attestation.authority_policy_digest,
        emergency_policy_digest: attestation.emergency_policy_digest,
        safety_basis: attestation.safety_basis,
        lifecycle,
    }
}

fn validate_qualified_attestation(
    attestation: &MedicationActivationAttestationV1,
) -> Result<(), ActivationStateError> {
    if attestation.schema_version != 1 || attestation.activation_id.trim().is_empty() {
        return Err(ActivationStateError::MalformedQualifiedActivation);
    }
    require_domain(
        attestation.medication_artifact_digest,
        DigestDomain::MedicationRequestArtifact,
    )?;
    require_domain(
        attestation.activation_receipt_digest,
        DigestDomain::MedicationActivationReceipt,
    )?;
    require_domain(
        attestation.safety_context_digest,
        DigestDomain::MedicationSafetyContext,
    )?;
    require_domain(
        attestation.safety_trust_receipt_digest,
        DigestDomain::MedicationSafetyTrustReceipt,
    )?;
    require_domain(
        attestation.authority_policy_digest,
        DigestDomain::AuthorityPolicy,
    )?;
    require_domain(
        attestation.safety_policy_digest,
        DigestDomain::MedicationSafetyPolicy,
    )?;
    require_domain(
        attestation.safety_trust_policy_digest,
        DigestDomain::MedicationSafetyTrustPolicy,
    )?;
    require_domain(
        attestation.workflow_policy_digest,
        DigestDomain::WorkflowPolicy,
    )
}

fn validate_emergency_attestation(
    attestation: &EmergencyMedicationAttestationV1,
) -> Result<(), ActivationStateError> {
    if attestation.schema_version != 1 || attestation.activation_id.trim().is_empty() {
        return Err(ActivationStateError::MalformedEmergencyActivation);
    }
    require_domain(
        attestation.medication_artifact_digest,
        DigestDomain::MedicationRequestArtifact,
    )?;
    require_domain(
        attestation.override_receipt_digest,
        DigestDomain::EmergencyMedicationOverrideReceipt,
    )?;
    require_domain(
        attestation.safety_assessment_digest,
        DigestDomain::MedicationSafetyAssessment,
    )?;
    require_domain(
        attestation.safety_policy_digest,
        DigestDomain::MedicationSafetyPolicy,
    )?;
    require_domain(
        attestation.authority_policy_digest,
        DigestDomain::AuthorityPolicy,
    )?;
    require_domain(
        attestation.emergency_policy_digest,
        DigestDomain::EmergencyMedicationOverridePolicy,
    )
}

fn validate_qualified_revocation(
    revocation: &MedicationActivationRevocation,
) -> Result<(), ActivationStateError> {
    if revocation.revocation_id.trim().is_empty() {
        return Err(ActivationStateError::MalformedQualifiedRevocation);
    }
    require_domain(
        revocation.activation_receipt_digest,
        DigestDomain::MedicationActivationReceipt,
    )?;
    validate_reason_commitment(
        matches!(revocation.reason, ActivationRevocationReason::Other),
        revocation.reason_commitment,
    )
}

fn validate_emergency_termination(
    termination: &EmergencyMedicationTermination,
) -> Result<(), ActivationStateError> {
    if termination.termination_id.trim().is_empty() {
        return Err(ActivationStateError::MalformedEmergencyTermination);
    }
    require_domain(
        termination.override_receipt_digest,
        DigestDomain::EmergencyMedicationOverrideReceipt,
    )?;
    validate_reason_commitment(
        matches!(termination.reason, EmergencyActivationTerminationReason::Other),
        termination.reason_commitment,
    )
}

fn validate_reason_commitment(
    is_other: bool,
    commitment: Option<[u8; 32]>,
) -> Result<(), ActivationStateError> {
    if commitment.is_some_and(|value| value == [0u8; 32]) {
        return Err(ActivationStateError::ZeroReasonCommitment);
    }
    if is_other && commitment.is_none() {
        return Err(ActivationStateError::MissingOtherReasonCommitment);
    }
    Ok(())
}

fn require_domain(digest: StoredDigest, domain: DigestDomain) -> Result<(), ActivationStateError> {
    digest.validate_shape()?;
    if digest.domain != domain {
        return Err(ActivationStateError::DigestDomainMismatch {
            expected: domain,
            actual: digest.domain,
        });
    }
    Ok(())
}

fn map_qualified_reason(reason: &ActivationRevocationReason) -> QualifiedTerminationReason {
    match reason {
        ActivationRevocationReason::Superseded => QualifiedTerminationReason::Superseded,
        ActivationRevocationReason::MedicationStopped => QualifiedTerminationReason::MedicationStopped,
        ActivationRevocationReason::SafetyEvidenceRevoked => {
            QualifiedTerminationReason::SafetyEvidenceRevoked
        }
        ActivationRevocationReason::ProfessionalAuthorityRevoked => {
            QualifiedTerminationReason::ProfessionalAuthorityRevoked
        }
        ActivationRevocationReason::SourceTrustRevoked => {
            QualifiedTerminationReason::SourceTrustRevoked
        }
        ActivationRevocationReason::EnteredInError => QualifiedTerminationReason::EnteredInError,
        ActivationRevocationReason::Other => QualifiedTerminationReason::Other,
    }
}

fn map_emergency_reason(reason: &EmergencyActivationTerminationReason) -> EmergencyTerminationReason {
    match reason {
        EmergencyActivationTerminationReason::MedicationStopped => {
            EmergencyTerminationReason::MedicationStopped
        }
        EmergencyActivationTerminationReason::QualifiedActivationSuperseded => {
            EmergencyTerminationReason::QualifiedActivationSuperseded
        }
        EmergencyActivationTerminationReason::SafetyEvidenceResolved => {
            EmergencyTerminationReason::SafetyEvidenceResolved
        }
        EmergencyActivationTerminationReason::ProfessionalAuthorityRevoked => {
            EmergencyTerminationReason::ProfessionalAuthorityRevoked
        }
        EmergencyActivationTerminationReason::EnteredInError => {
            EmergencyTerminationReason::EnteredInError
        }
        EmergencyActivationTerminationReason::Other => EmergencyTerminationReason::Other,
    }
}

fn sort_current(values: &mut [CurrentActivation]) {
    values.sort_by_key(|value| match value {
        CurrentActivation::Qualified(lineage) => {
            (0_u8, lineage.activation_receipt_digest.value)
        }
        CurrentActivation::EmergencyOverride(lineage) => {
            (1_u8, lineage.override_receipt_digest.value)
        }
    });
}

fn sort_history(values: &mut [HistoricalActivation]) {
    values.sort_by_key(|value| match value {
        HistoricalActivation::Qualified(lineage) => {
            (0_u8, lineage.activation_receipt_digest.value)
        }
        HistoricalActivation::EmergencyOverride(lineage) => {
            (1_u8, lineage.override_receipt_digest.value)
        }
    });
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ActivationStateError {
    #[error("legacy prescription id is required")]
    MissingLegacyPrescriptionId,
    #[error("malformed qualified activation attestation")]
    MalformedQualifiedActivation,
    #[error("malformed emergency activation attestation")]
    MalformedEmergencyActivation,
    #[error("malformed qualified activation revocation")]
    MalformedQualifiedRevocation,
    #[error("malformed emergency activation termination")]
    MalformedEmergencyTermination,
    #[error("duplicate activation action hash appears in input")]
    DuplicateActivationActionHash,
    #[error("same qualified receipt was published with conflicting attestation fields")]
    ConflictingQualifiedSemanticDuplicate,
    #[error("same emergency receipt was published with conflicting attestation fields")]
    ConflictingEmergencySemanticDuplicate,
    #[error("qualified revocation target activation is missing from reducer input")]
    OrphanQualifiedRevocation,
    #[error("qualified revocation receipt does not match its target activation")]
    QualifiedRevocationReceiptMismatch,
    #[error("emergency termination target activation is missing from reducer input")]
    OrphanEmergencyTermination,
    #[error("emergency termination receipt does not match its target activation")]
    EmergencyTerminationReceiptMismatch,
    #[error("reason commitment cannot be all zero")]
    ZeroReasonCommitment,
    #[error("Other termination/revocation reason requires a protected rationale commitment")]
    MissingOtherReasonCommitment,
    #[error("digest domain mismatch: expected {expected:?}, got {actual:?}")]
    DigestDomainMismatch {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("invalid stored digest: {0}")]
    Integrity(#[from] mycelix_clinical_integrity::IntegrityError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_integrity::DigestAlgorithm;

    fn action(byte: u8) -> ActionHash {
        ActionHash::from_raw_36(vec![byte; 36])
    }

    fn digest(domain: DigestDomain, byte: u8) -> StoredDigest {
        StoredDigest {
            algorithm: DigestAlgorithm::Blake3_256,
            domain,
            value: [byte; 32],
        }
    }

    fn qualified(action_byte: u8, receipt_byte: u8) -> QualifiedActivationRecord {
        QualifiedActivationRecord {
            action_hash: action(action_byte),
            activation: QualifiedMedicationActivation {
                attestation: MedicationActivationAttestationV1 {
                    schema_version: 1,
                    activation_id: format!("qa-{receipt_byte}"),
                    medication_artifact_digest: digest(
                        DigestDomain::MedicationRequestArtifact,
                        1,
                    ),
                    activation_receipt_digest: digest(
                        DigestDomain::MedicationActivationReceipt,
                        receipt_byte,
                    ),
                    safety_context_digest: digest(DigestDomain::MedicationSafetyContext, 3),
                    safety_trust_receipt_digest: digest(
                        DigestDomain::MedicationSafetyTrustReceipt,
                        4,
                    ),
                    authority_policy_digest: digest(DigestDomain::AuthorityPolicy, 5),
                    safety_policy_digest: digest(DigestDomain::MedicationSafetyPolicy, 6),
                    safety_trust_policy_digest: digest(
                        DigestDomain::MedicationSafetyTrustPolicy,
                        7,
                    ),
                    workflow_policy_digest: digest(DigestDomain::WorkflowPolicy, 8),
                },
                verifier_authorization_hash: action(200),
            },
        }
    }

    fn emergency(action_byte: u8, receipt_byte: u8) -> EmergencyActivationRecord {
        EmergencyActivationRecord {
            action_hash: action(action_byte),
            activation: EmergencyMedicationActivation {
                attestation: EmergencyMedicationAttestationV1 {
                    schema_version: 1,
                    activation_id: format!("ea-{receipt_byte}"),
                    medication_artifact_digest: digest(
                        DigestDomain::MedicationRequestArtifact,
                        1,
                    ),
                    override_receipt_digest: digest(
                        DigestDomain::EmergencyMedicationOverrideReceipt,
                        receipt_byte,
                    ),
                    safety_assessment_digest: digest(
                        DigestDomain::MedicationSafetyAssessment,
                        9,
                    ),
                    safety_policy_digest: digest(DigestDomain::MedicationSafetyPolicy, 6),
                    authority_policy_digest: digest(DigestDomain::AuthorityPolicy, 5),
                    emergency_policy_digest: digest(
                        DigestDomain::EmergencyMedicationOverridePolicy,
                        10,
                    ),
                    safety_basis: EmergencySafetyBasis::Indeterminate,
                },
                verifier_authorization_hash: action(201),
            },
        }
    }

    #[test]
    fn qualified_current_state_preserves_provenance() {
        let view = reduce_activation_state(&[qualified(10, 2)], &[], &[], &[], &[]).unwrap();
        assert!(matches!(
            view.current,
            CurrentActivationState::One(CurrentActivation::Qualified(_))
        ));
        assert!(view.history.is_empty());
    }

    #[test]
    fn emergency_current_state_is_not_qualified() {
        let view = reduce_activation_state(&[], &[], &[emergency(11, 12)], &[], &[]).unwrap();
        assert!(matches!(
            view.current,
            CurrentActivationState::One(CurrentActivation::EmergencyOverride(_))
        ));
    }

    #[test]
    fn duplicate_receipt_is_idempotent_and_one_revocation_terminates_lineage() {
        let a = qualified(10, 2);
        let b = qualified(11, 2);
        let revocation = QualifiedRevocationRecord {
            action_hash: action(30),
            revocation: MedicationActivationRevocation {
                revocation_id: "stop-1".into(),
                activation_hash: a.action_hash.clone(),
                activation_receipt_digest: a.activation.attestation.activation_receipt_digest,
                reason: ActivationRevocationReason::MedicationStopped,
                reason_commitment: None,
            },
        };

        let view = reduce_activation_state(&[a, b], &[revocation], &[], &[], &[]).unwrap();
        assert_eq!(view.current, CurrentActivationState::None);
        assert_eq!(view.history.len(), 1);
        let HistoricalActivation::Qualified(lineage) = &view.history[0] else {
            panic!("expected qualified history");
        };
        assert_eq!(lineage.activation_action_hashes.len(), 2);
        assert!(matches!(
            lineage.lifecycle,
            ActivationLifecycle::Terminated { .. }
        ));
    }

    #[test]
    fn mismatched_revocation_receipt_fails_closed() {
        let activation = qualified(10, 2);
        let revocation = QualifiedRevocationRecord {
            action_hash: action(30),
            revocation: MedicationActivationRevocation {
                revocation_id: "bad-stop".into(),
                activation_hash: activation.action_hash.clone(),
                activation_receipt_digest: digest(
                    DigestDomain::MedicationActivationReceipt,
                    99,
                ),
                reason: ActivationRevocationReason::MedicationStopped,
                reason_commitment: None,
            },
        };

        assert_eq!(
            reduce_activation_state(&[activation], &[revocation], &[], &[], &[]).unwrap_err(),
            ActivationStateError::QualifiedRevocationReceiptMismatch
        );
    }

    #[test]
    fn ordinary_and_emergency_current_states_conflict() {
        let view = reduce_activation_state(
            &[qualified(10, 2)],
            &[],
            &[emergency(11, 12)],
            &[],
            &[],
        )
        .unwrap();
        let CurrentActivationState::Conflict { current } = view.current else {
            panic!("expected conflict");
        };
        assert_eq!(current.len(), 2);
    }

    #[test]
    fn legacy_active_is_never_promoted_to_current() {
        let legacy = LegacyPrescriptionRecord {
            action_hash: action(40),
            prescription_id: "legacy-rx-1".into(),
            status: LegacyPrescriptionStatus::Active,
        };
        let view = reduce_activation_state(&[], &[], &[], &[], &[legacy]).unwrap();
        assert_eq!(view.current, CurrentActivationState::None);
        assert_eq!(view.legacy_unqualified.len(), 1);
    }

    #[test]
    fn same_receipt_with_changed_attestation_fails_closed() {
        let a = qualified(10, 2);
        let mut b = qualified(11, 2);
        b.activation.attestation.safety_policy_digest =
            digest(DigestDomain::MedicationSafetyPolicy, 77);

        assert_eq!(
            reduce_activation_state(&[a, b], &[], &[], &[], &[]).unwrap_err(),
            ActivationStateError::ConflictingQualifiedSemanticDuplicate
        );
    }

    #[test]
    fn orphan_emergency_termination_fails_closed() {
        let termination = EmergencyTerminationRecord {
            action_hash: action(31),
            termination: EmergencyMedicationTermination {
                termination_id: "term-1".into(),
                activation_hash: action(99),
                override_receipt_digest: digest(
                    DigestDomain::EmergencyMedicationOverrideReceipt,
                    12,
                ),
                reason: EmergencyActivationTerminationReason::MedicationStopped,
                reason_commitment: None,
            },
        };

        assert_eq!(
            reduce_activation_state(&[], &[], &[], &[termination], &[]).unwrap_err(),
            ActivationStateError::OrphanEmergencyTermination
        );
    }
}
