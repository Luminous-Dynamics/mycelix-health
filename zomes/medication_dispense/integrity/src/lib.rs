#![deny(unsafe_code)]
#![allow(clippy::collapsible_match)]
//! DNA-pinned refill-slot adjudication and finalized medication dispense integrity.
//!
//! Holochain DHT visibility is not treated as a linearizable lock. Instead, each
//! medication artifact deterministically maps to one allocator AgentPubKey pinned in
//! DNA properties. That allocator may issue a short-lived authorization for one exact
//! preflight, one exact refill slot, one exact pharmacist, and one exact final receipt.
//!
//! This gives strong serialization under the explicit non-equivocating allocator
//! assumption. If an allocator issues conflicting authorizations for the same semantic
//! slot, anyone may publish objective `AllocatorEquivocationEvidence` referencing both
//! records. V1 detects that Byzantine failure; it does not claim to prevent a
//! compromised allocator from signing conflicting authorizations before detection.

use hdi::prelude::*;
use mycelix_clinical_integrity::{hash_canonical_bytes, DigestDomain, StoredDigest};

const CONFIG_VERSION: u16 = 1;
const ENTRY_SCHEMA_VERSION: u16 = 1;
const ABSOLUTE_MAX_SLOT_AUTHORIZATION_MICROS: i64 = 300_000_000; // 5 minutes
const FINAL_RECEIPT_SCHEMA_TAG: &[u8] = b"mycelix-health/final-medication-dispense-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MedicationDispenseRootConfig {
    pub schema_version: u16,
    /// Ordered allocator set. The exact order is part of DNA properties and therefore
    /// part of the DNA hash. One medication artifact deterministically selects one key.
    pub slot_allocators: Vec<AgentPubKey>,
    pub max_slot_authorization_duration_micros: i64,
}

/// Privacy-minimized final dispense receipt payload. Clinical/product/quantity details
/// remain bound through the exact request + preflight digests rather than duplicated.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MedicationDispenseFinalAttestationV1 {
    pub schema_version: u16,
    pub dispense_id: String,
    pub medication_artifact_digest: StoredDigest,
    pub activation_semantic_receipt_digest: StoredDigest,
    pub activation_state_digest: StoredDigest,
    pub dispense_request_digest: StoredDigest,
    pub preflight_receipt_digest: StoredDigest,
    pub slot_index: u32,
    /// Binding produced by the private clinical-authority layer. The DHT additionally
    /// requires the final action author to equal the authorization grantee AgentPubKey.
    pub pharmacist_principal_binding: [u8; 32],
    pub pharmacy_record_digest: StoredDigest,
    pub pharmacy_affiliation_evidence_digest: StoredDigest,
    pub pharmacy_status_evidence_digest: StoredDigest,
    pub authority_policy_digest: StoredDigest,
    pub dispense_policy_digest: StoredDigest,
}

impl MedicationDispenseFinalAttestationV1 {
    pub fn validate(&self) -> ExternResult<ValidateCallbackResult> {
        if self.schema_version != ENTRY_SCHEMA_VERSION {
            return invalid("Unsupported finalized medication dispense attestation version");
        }
        if self.dispense_id.trim().is_empty() {
            return invalid("Finalized medication dispense ID is required");
        }
        if self.pharmacist_principal_binding == [0u8; 32] {
            return invalid("Finalized medication dispense pharmacist principal binding cannot be zero");
        }
        require_digest_domain(
            self.medication_artifact_digest,
            DigestDomain::MedicationRequestArtifact,
        )?;
        require_activation_receipt_domain(self.activation_semantic_receipt_digest)?;
        require_digest_domain(
            self.activation_state_digest,
            DigestDomain::MedicationActivationState,
        )?;
        require_digest_domain(
            self.dispense_request_digest,
            DigestDomain::MedicationDispenseRequest,
        )?;
        require_digest_domain(
            self.preflight_receipt_digest,
            DigestDomain::MedicationDispensePreflightReceipt,
        )?;
        require_digest_domain(self.pharmacy_record_digest, DigestDomain::PharmacyRecord)?;
        require_digest_domain(
            self.pharmacy_affiliation_evidence_digest,
            DigestDomain::PharmacyAffiliationEvidence,
        )?;
        require_digest_domain(
            self.pharmacy_status_evidence_digest,
            DigestDomain::PharmacyStatusEvidence,
        )?;
        require_digest_domain(self.authority_policy_digest, DigestDomain::AuthorityPolicy)?;
        require_digest_domain(
            self.dispense_policy_digest,
            DigestDomain::MedicationDispensePolicy,
        )?;
        Ok(ValidateCallbackResult::Valid)
    }

    /// Semantic finalized-dispense identity. Duplicate DHT publications of this exact
    /// attestation are one idempotent clinical dispense event in later read models.
    pub fn receipt_digest(&self) -> ExternResult<StoredDigest> {
        let shape = self.validate()?;
        if !matches!(shape, ValidateCallbackResult::Valid) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Cannot hash invalid finalized medication dispense attestation".to_string()
            )));
        }
        let encoded = serde_json::to_vec(self).map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to serialize finalized medication dispense attestation: {error}"
            )))
        })?;
        let mut framed = Vec::with_capacity(FINAL_RECEIPT_SCHEMA_TAG.len() + 1 + encoded.len());
        framed.extend_from_slice(FINAL_RECEIPT_SCHEMA_TAG);
        framed.push(0);
        framed.extend_from_slice(&encoded);
        Ok(hash_canonical_bytes(DigestDomain::MedicationDispenseReceipt, &framed)
            .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?
            .stored())
    }
}

/// Exact, short-lived authorization issued by the deterministic allocator selected for
/// this medication artifact. This is not a reusable pharmacy or pharmacist role.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MedicationDispenseSlotAuthorization {
    pub schema_version: u16,
    pub authorization_id: String,
    pub grantee: AgentPubKey,
    pub medication_artifact_digest: StoredDigest,
    pub activation_semantic_receipt_digest: StoredDigest,
    pub activation_state_digest: StoredDigest,
    pub dispense_request_digest: StoredDigest,
    pub preflight_receipt_digest: StoredDigest,
    pub slot_index: u32,
    /// Slot N>0 must reference a finalized dispense for slot N-1 in the same lineage.
    pub previous_final_dispense_hash: Option<ActionHash>,
    pub pharmacist_principal_binding: [u8; 32],
    pub pharmacy_record_digest: StoredDigest,
    pub pharmacy_affiliation_evidence_digest: StoredDigest,
    pub pharmacy_status_evidence_digest: StoredDigest,
    pub authority_policy_digest: StoredDigest,
    pub dispense_policy_digest: StoredDigest,
    /// Exact semantic receipt the grantee is allowed to publish.
    pub final_receipt_digest: StoredDigest,
    pub valid_from: Timestamp,
    pub valid_until: Timestamp,
}

/// Pharmacist-authored finalization of one exact allocator-authorized slot.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FinalizedMedicationDispense {
    pub attestation: MedicationDispenseFinalAttestationV1,
    pub slot_authorization_hash: ActionHash,
}

/// Objective proof that one allocator issued two different authorizations for the
/// same medication + activation lineage + refill slot.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AllocatorEquivocationEvidence {
    pub evidence_id: String,
    pub left_authorization_hash: ActionHash,
    pub right_authorization_hash: ActionHash,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    MedicationDispenseSlotAuthorization(MedicationDispenseSlotAuthorization),
    FinalizedMedicationDispense(FinalizedMedicationDispense),
    AllocatorEquivocationEvidence(AllocatorEquivocationEvidence),
}

#[hdk_link_types]
pub enum LinkTypes {}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => {
                validate_create_entry(EntryCreationAction::Create(action), app_entry)
            }
            OpEntry::UpdateEntry { .. } => invalid(
                "Medication dispense adjudication/finalization records are append-only",
            ),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(_) => invalid(
            "Medication dispense adjudication/finalization records cannot be updated",
        ),
        FlatOp::RegisterDelete(_) => invalid(
            "Medication dispense adjudication/finalization records cannot be deleted",
        ),
        FlatOp::RegisterCreateLink { .. } | FlatOp::RegisterDeleteLink { .. } => {
            invalid("Medication dispense v1 defines no DHT link surface")
        }
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(
    action: EntryCreationAction,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::MedicationDispenseSlotAuthorization(authorization) => {
            let shape = validate_slot_authorization_shape(&authorization)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let config = dispense_root_config()?;
            let allocator = selected_allocator(&config, authorization.medication_artifact_digest)?;
            if action.author() != &allocator {
                return invalid(
                    "Refill-slot authorization author is not the DNA-selected allocator for this medication",
                );
            }
            let window = validate_authorization_window(action.timestamp(), &authorization, &config)?;
            if !matches!(window, ValidateCallbackResult::Valid) {
                return Ok(window);
            }
            validate_previous_final_dispense(&authorization)
        }
        EntryTypes::FinalizedMedicationDispense(finalized) => {
            let shape = finalized.attestation.validate()?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            require_exact_slot_authorization(action.author(), action.timestamp(), &finalized)
        }
        EntryTypes::AllocatorEquivocationEvidence(evidence) => {
            validate_allocator_equivocation(&evidence)
        }
    }
}

fn validate_slot_authorization_shape(
    authorization: &MedicationDispenseSlotAuthorization,
) -> ExternResult<ValidateCallbackResult> {
    if authorization.schema_version != ENTRY_SCHEMA_VERSION {
        return invalid("Unsupported medication dispense slot authorization version");
    }
    if authorization.authorization_id.trim().is_empty() {
        return invalid("Medication dispense slot authorization ID is required");
    }
    if authorization.pharmacist_principal_binding == [0u8; 32] {
        return invalid("Slot authorization pharmacist principal binding cannot be zero");
    }
    require_digest_domain(
        authorization.medication_artifact_digest,
        DigestDomain::MedicationRequestArtifact,
    )?;
    require_activation_receipt_domain(authorization.activation_semantic_receipt_digest)?;
    require_digest_domain(
        authorization.activation_state_digest,
        DigestDomain::MedicationActivationState,
    )?;
    require_digest_domain(
        authorization.dispense_request_digest,
        DigestDomain::MedicationDispenseRequest,
    )?;
    require_digest_domain(
        authorization.preflight_receipt_digest,
        DigestDomain::MedicationDispensePreflightReceipt,
    )?;
    require_digest_domain(authorization.pharmacy_record_digest, DigestDomain::PharmacyRecord)?;
    require_digest_domain(
        authorization.pharmacy_affiliation_evidence_digest,
        DigestDomain::PharmacyAffiliationEvidence,
    )?;
    require_digest_domain(
        authorization.pharmacy_status_evidence_digest,
        DigestDomain::PharmacyStatusEvidence,
    )?;
    require_digest_domain(authorization.authority_policy_digest, DigestDomain::AuthorityPolicy)?;
    require_digest_domain(
        authorization.dispense_policy_digest,
        DigestDomain::MedicationDispensePolicy,
    )?;
    require_digest_domain(
        authorization.final_receipt_digest,
        DigestDomain::MedicationDispenseReceipt,
    )?;
    if authorization.valid_until <= authorization.valid_from {
        return invalid("Slot authorization valid_until must follow valid_from");
    }
    match (authorization.slot_index, &authorization.previous_final_dispense_hash) {
        (0, None) => {}
        (0, Some(_)) => {
            return invalid("Initial dispense slot must not reference a previous finalized dispense")
        }
        (_, None) => {
            return invalid("Refill slot requires the immediately previous finalized dispense")
        }
        (_, Some(_)) => {}
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_authorization_window(
    issued_at: Timestamp,
    authorization: &MedicationDispenseSlotAuthorization,
    config: &MedicationDispenseRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    let configured_max = config.max_slot_authorization_duration_micros;
    if configured_max <= 0 || configured_max > ABSOLUTE_MAX_SLOT_AUTHORIZATION_MICROS {
        return invalid("DNA dispense slot authorization-duration policy is invalid");
    }
    if issued_at < authorization.valid_from || issued_at >= authorization.valid_until {
        return invalid("Allocator authorization action timestamp must fall inside validity window");
    }
    let duration = authorization.valid_until.as_micros() as i128
        - authorization.valid_from.as_micros() as i128;
    if duration <= 0 || duration > configured_max as i128 {
        return invalid("Refill-slot authorization exceeds DNA-pinned maximum duration");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_previous_final_dispense(
    authorization: &MedicationDispenseSlotAuthorization,
) -> ExternResult<ValidateCallbackResult> {
    if authorization.slot_index == 0 {
        return Ok(ValidateCallbackResult::Valid);
    }
    let previous_hash = authorization
        .previous_final_dispense_hash
        .clone()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Refill slot is missing previous finalized dispense hash".to_string()
        )))?;
    let record = must_get_valid_record(previous_hash)?;
    let previous: FinalizedMedicationDispense = decode_entry(&record, "previous finalized dispense")?;
    let previous_slot = previous
        .attestation
        .slot_index
        .checked_add(1)
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Previous finalized dispense slot index overflow".to_string()
        )))?;
    if previous_slot != authorization.slot_index {
        return invalid("Refill authorization does not immediately follow previous finalized slot");
    }
    if previous.attestation.medication_artifact_digest != authorization.medication_artifact_digest
        || previous.attestation.activation_semantic_receipt_digest
            != authorization.activation_semantic_receipt_digest
    {
        return invalid("Previous finalized dispense belongs to another medication activation lineage");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_exact_slot_authorization(
    author: &AgentPubKey,
    finalized_at: Timestamp,
    finalized: &FinalizedMedicationDispense,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(finalized.slot_authorization_hash.clone())?;
    let authorization: MedicationDispenseSlotAuthorization =
        decode_entry(&record, "medication dispense slot authorization")?;

    if &authorization.grantee != author {
        return invalid("Finalized dispense author is not the slot authorization grantee");
    }
    if finalized_at < authorization.valid_from || finalized_at >= authorization.valid_until {
        return invalid("Finalized dispense action timestamp falls outside slot authorization window");
    }
    let actual_receipt_digest = finalized.attestation.receipt_digest()?;
    if actual_receipt_digest != authorization.final_receipt_digest {
        return invalid("Finalized dispense payload does not match allocator-authorized receipt");
    }
    if finalized.attestation.medication_artifact_digest != authorization.medication_artifact_digest
        || finalized.attestation.activation_semantic_receipt_digest
            != authorization.activation_semantic_receipt_digest
        || finalized.attestation.activation_state_digest != authorization.activation_state_digest
        || finalized.attestation.dispense_request_digest != authorization.dispense_request_digest
        || finalized.attestation.preflight_receipt_digest != authorization.preflight_receipt_digest
        || finalized.attestation.slot_index != authorization.slot_index
        || finalized.attestation.pharmacist_principal_binding
            != authorization.pharmacist_principal_binding
        || finalized.attestation.pharmacy_record_digest != authorization.pharmacy_record_digest
        || finalized.attestation.pharmacy_affiliation_evidence_digest
            != authorization.pharmacy_affiliation_evidence_digest
        || finalized.attestation.pharmacy_status_evidence_digest
            != authorization.pharmacy_status_evidence_digest
        || finalized.attestation.authority_policy_digest != authorization.authority_policy_digest
        || finalized.attestation.dispense_policy_digest != authorization.dispense_policy_digest
    {
        return invalid("Finalized dispense evidence set does not match exact slot authorization");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_allocator_equivocation(
    evidence: &AllocatorEquivocationEvidence,
) -> ExternResult<ValidateCallbackResult> {
    if evidence.evidence_id.trim().is_empty() {
        return invalid("Allocator equivocation evidence ID is required");
    }
    if evidence.left_authorization_hash == evidence.right_authorization_hash {
        return invalid("Allocator equivocation requires two distinct authorization actions");
    }
    let left_record = must_get_valid_record(evidence.left_authorization_hash.clone())?;
    let right_record = must_get_valid_record(evidence.right_authorization_hash.clone())?;
    let left: MedicationDispenseSlotAuthorization =
        decode_entry(&left_record, "left slot authorization")?;
    let right: MedicationDispenseSlotAuthorization =
        decode_entry(&right_record, "right slot authorization")?;

    if left_record.action().author() != right_record.action().author() {
        return invalid("Equivocation evidence authorizations were issued by different allocators");
    }
    if left.medication_artifact_digest != right.medication_artifact_digest
        || left.activation_semantic_receipt_digest != right.activation_semantic_receipt_digest
        || left.slot_index != right.slot_index
    {
        return invalid("Equivocation evidence does not target the same semantic refill slot");
    }
    if left == right {
        return invalid("Equivalent duplicate authorization payloads are idempotent, not equivocation");
    }

    let config = dispense_root_config()?;
    let expected = selected_allocator(&config, left.medication_artifact_digest)?;
    if left_record.action().author() != &expected {
        return invalid("Equivocation evidence does not involve the DNA-selected allocator");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn dispense_root_config() -> ExternResult<MedicationDispenseRootConfig> {
    let info = dna_info()?;
    parse_dispense_root_config(info.modifiers.properties.bytes())
}

fn parse_dispense_root_config(bytes: &[u8]) -> ExternResult<MedicationDispenseRootConfig> {
    let properties: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "DNA properties are not valid JSON for medication dispense: {error}"
        )))
    })?;
    let value = properties.get("medication_dispense").ok_or(wasm_error!(
        WasmErrorInner::Guest(
            "DNA properties do not configure medication_dispense slot allocators".to_string()
        )
    ))?;
    let config: MedicationDispenseRootConfig = serde_json::from_value(value.clone()).map_err(|error| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Invalid medication_dispense DNA properties: {error}"
        )))
    })?;
    if config.schema_version != CONFIG_VERSION {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Unsupported medication_dispense config version {}",
            config.schema_version
        ))));
    }
    if config.max_slot_authorization_duration_micros <= 0
        || config.max_slot_authorization_duration_micros
            > ABSOLUTE_MAX_SLOT_AUTHORIZATION_MICROS
    {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Medication dispense slot authorization duration must be > 0 and <= 5 minutes"
                .to_string()
        )));
    }
    for (index, allocator) in config.slot_allocators.iter().enumerate() {
        if config.slot_allocators[..index].contains(allocator) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Medication dispense allocator list contains duplicate AgentPubKeys".to_string()
            )));
        }
    }
    Ok(config)
}

fn selected_allocator(
    config: &MedicationDispenseRootConfig,
    medication_artifact_digest: StoredDigest,
) -> ExternResult<AgentPubKey> {
    require_digest_domain(
        medication_artifact_digest,
        DigestDomain::MedicationRequestArtifact,
    )?;
    if config.slot_allocators.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Medication dispense is disabled because no slot allocators are pinned in DNA properties"
                .to_string()
        )));
    }
    let mut prefix = [0u8; 8];
    prefix.copy_from_slice(&medication_artifact_digest.value[..8]);
    let index = (u64::from_be_bytes(prefix) % config.slot_allocators.len() as u64) as usize;
    Ok(config.slot_allocators[index].clone())
}

fn require_activation_receipt_domain(digest: StoredDigest) -> ExternResult<()> {
    digest
        .validate_shape()
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?;
    if !matches!(
        digest.domain,
        DigestDomain::MedicationActivationReceipt
            | DigestDomain::EmergencyMedicationOverrideReceipt
    ) {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Medication dispense activation receipt has invalid domain {:?}",
            digest.domain
        ))));
    }
    Ok(())
}

fn require_digest_domain(digest: StoredDigest, expected: DigestDomain) -> ExternResult<()> {
    digest
        .validate_shape()
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?;
    if digest.domain != expected {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Medication dispense digest domain mismatch: expected {expected:?}, got {:?}",
            digest.domain
        ))));
    }
    Ok(())
}

fn decode_entry<T>(record: &Record, label: &'static str) -> ExternResult<T>
where
    T: TryFrom<SerializedBytes, Error = SerializedBytesError>,
{
    record
        .entry()
        .to_app_option::<T>()
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(format!(
            "Referenced {label} entry is missing or wrong type"
        ))))
}

fn invalid(message: impl Into<String>) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(domain: DigestDomain, seed: u8) -> StoredDigest {
        hash_canonical_bytes(domain, &[seed]).unwrap().stored()
    }

    fn agent(seed: u8) -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![seed; 36])
    }

    fn final_attestation(slot_index: u32) -> MedicationDispenseFinalAttestationV1 {
        MedicationDispenseFinalAttestationV1 {
            schema_version: 1,
            dispense_id: format!("dispense-{slot_index}"),
            medication_artifact_digest: digest(DigestDomain::MedicationRequestArtifact, 1),
            activation_semantic_receipt_digest: digest(DigestDomain::MedicationActivationReceipt, 2),
            activation_state_digest: digest(DigestDomain::MedicationActivationState, 3),
            dispense_request_digest: digest(DigestDomain::MedicationDispenseRequest, 4),
            preflight_receipt_digest: digest(DigestDomain::MedicationDispensePreflightReceipt, 5),
            slot_index,
            pharmacist_principal_binding: [6u8; 32],
            pharmacy_record_digest: digest(DigestDomain::PharmacyRecord, 7),
            pharmacy_affiliation_evidence_digest: digest(
                DigestDomain::PharmacyAffiliationEvidence,
                8,
            ),
            pharmacy_status_evidence_digest: digest(DigestDomain::PharmacyStatusEvidence, 9),
            authority_policy_digest: digest(DigestDomain::AuthorityPolicy, 10),
            dispense_policy_digest: digest(DigestDomain::MedicationDispensePolicy, 11),
        }
    }

    fn authorization(slot_index: u32) -> MedicationDispenseSlotAuthorization {
        let attestation = final_attestation(slot_index);
        MedicationDispenseSlotAuthorization {
            schema_version: 1,
            authorization_id: format!("slot-auth-{slot_index}"),
            grantee: agent(12),
            medication_artifact_digest: attestation.medication_artifact_digest,
            activation_semantic_receipt_digest: attestation.activation_semantic_receipt_digest,
            activation_state_digest: attestation.activation_state_digest,
            dispense_request_digest: attestation.dispense_request_digest,
            preflight_receipt_digest: attestation.preflight_receipt_digest,
            slot_index,
            previous_final_dispense_hash: if slot_index == 0 {
                None
            } else {
                Some(ActionHash::from_raw_36(vec![13; 36]))
            },
            pharmacist_principal_binding: attestation.pharmacist_principal_binding,
            pharmacy_record_digest: attestation.pharmacy_record_digest,
            pharmacy_affiliation_evidence_digest: attestation.pharmacy_affiliation_evidence_digest,
            pharmacy_status_evidence_digest: attestation.pharmacy_status_evidence_digest,
            authority_policy_digest: attestation.authority_policy_digest,
            dispense_policy_digest: attestation.dispense_policy_digest,
            final_receipt_digest: attestation.receipt_digest().unwrap(),
            valid_from: Timestamp::from_micros(100),
            valid_until: Timestamp::from_micros(200),
        }
    }

    #[test]
    fn initial_slot_cannot_claim_previous_final_dispense() {
        let mut auth = authorization(0);
        auth.previous_final_dispense_hash = Some(ActionHash::from_raw_36(vec![1; 36]));
        let result = validate_slot_authorization_shape(&auth).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn refill_slot_requires_previous_final_dispense() {
        let mut auth = authorization(1);
        auth.previous_final_dispense_hash = None;
        let result = validate_slot_authorization_shape(&auth).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn allocator_selection_is_deterministic_and_dna_ordered() {
        let config = MedicationDispenseRootConfig {
            schema_version: 1,
            slot_allocators: vec![agent(1), agent(2), agent(3)],
            max_slot_authorization_duration_micros: 100,
        };
        let medication = digest(DigestDomain::MedicationRequestArtifact, 42);
        assert_eq!(
            selected_allocator(&config, medication).unwrap(),
            selected_allocator(&config, medication).unwrap()
        );
    }

    #[test]
    fn empty_allocator_set_fails_closed() {
        let config = MedicationDispenseRootConfig {
            schema_version: 1,
            slot_allocators: vec![],
            max_slot_authorization_duration_micros: 100,
        };
        assert!(selected_allocator(
            &config,
            digest(DigestDomain::MedicationRequestArtifact, 1)
        )
        .is_err());
    }

    #[test]
    fn authorization_duration_is_hard_bounded() {
        let auth = authorization(0);
        let config = MedicationDispenseRootConfig {
            schema_version: 1,
            slot_allocators: vec![agent(1)],
            max_slot_authorization_duration_micros: 50,
        };
        let result = validate_authorization_window(Timestamp::from_micros(100), &auth, &config)
            .unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn final_receipt_digest_changes_with_slot() {
        let a = final_attestation(0).receipt_digest().unwrap();
        let b = final_attestation(1).receipt_digest().unwrap();
        assert_ne!(a, b);
    }
}
