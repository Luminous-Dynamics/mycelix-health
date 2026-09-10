#![deny(unsafe_code)]
//! DNA-rooted admission for medication-administration occurrence bindings.
//!
//! The private occurrence-binding artifact remains off-DHT. The DHT stores only a
//! privacy-minimized projection after a DNA-pinned root authorizes one exact verifier
//! to publish one exact administration->occurrence binding inside a short window.

use hdi::prelude::*;
use medication_administration_integrity::QualifiedMedicationAdministration;
use mycelix_clinical_integrity::{hash_canonical_bytes, DigestDomain, StoredDigest};

const CONFIG_VERSION: u16 = 1;
const ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS: i64 = 900_000_000; // 15 min
const ATTESTATION_TAG: &[u8] =
    b"mycelix-health/medication-administration-occurrence-attestation-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MedicationAdministrationOccurrenceRootConfig {
    pub schema_version: u16,
    pub root_authorities: Vec<AgentPubKey>,
    pub max_verifier_authorization_duration_micros: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MedicationAdministrationOccurrenceAttestationV1 {
    pub schema_version: u16,
    pub administration_hash: ActionHash,
    pub occurrence_binding_digest: StoredDigest,
    pub administration_receipt_digest: StoredDigest,
    pub event_digest: StoredDigest,
    pub occurrence_digest: StoredDigest,
    pub medication_artifact_digest: StoredDigest,
    pub dosage_index: u32,
}

impl MedicationAdministrationOccurrenceAttestationV1 {
    pub fn validate_shape(&self) -> ExternResult<ValidateCallbackResult> {
        if self.schema_version != 1 {
            return invalid("Unsupported administration occurrence attestation version");
        }
        require_digest_domain(
            self.occurrence_binding_digest,
            DigestDomain::MedicationAdministrationOccurrenceBinding,
        )?;
        require_digest_domain(
            self.administration_receipt_digest,
            DigestDomain::MedicationAdministrationReceipt,
        )?;
        require_digest_domain(self.event_digest, DigestDomain::MedicationAdministrationEvent)?;
        require_digest_domain(
            self.occurrence_digest,
            DigestDomain::MedicationAdministrationOccurrence,
        )?;
        require_digest_domain(
            self.medication_artifact_digest,
            DigestDomain::MedicationRequestArtifact,
        )?;
        Ok(ValidateCallbackResult::Valid)
    }

    pub fn digest(&self) -> ExternResult<StoredDigest> {
        let shape = self.validate_shape()?;
        if !matches!(shape, ValidateCallbackResult::Valid) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Cannot hash invalid administration occurrence attestation".to_string()
            )));
        }
        let encoded = serde_json::to_vec(self).map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to serialize administration occurrence attestation: {error}"
            )))
        })?;
        let mut framed = Vec::with_capacity(ATTESTATION_TAG.len() + 1 + encoded.len());
        framed.extend_from_slice(ATTESTATION_TAG);
        framed.push(0);
        framed.extend_from_slice(&encoded);
        Ok(hash_canonical_bytes(
            DigestDomain::MedicationAdministrationOccurrenceBinding,
            &framed,
        )
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?
        .stored())
    }
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MedicationAdministrationOccurrenceVerifierAuthorization {
    pub authorization_id: String,
    pub grantee: AgentPubKey,
    pub administration_hash: ActionHash,
    pub occurrence_attestation_digest: StoredDigest,
    pub occurrence_binding_digest: StoredDigest,
    pub administration_receipt_digest: StoredDigest,
    pub event_digest: StoredDigest,
    pub occurrence_digest: StoredDigest,
    pub medication_artifact_digest: StoredDigest,
    pub dosage_index: u32,
    pub valid_from: Timestamp,
    pub valid_until: Timestamp,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct QualifiedMedicationAdministrationOccurrenceBinding {
    pub attestation: MedicationAdministrationOccurrenceAttestationV1,
    pub verifier_authorization_hash: ActionHash,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OccurrenceBindingCorrectionReason {
    EnteredInError,
    WrongOccurrence,
    SourceEvidenceRevoked,
    DuplicateDocumentation,
    Other,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MedicationAdministrationOccurrenceBindingCorrection {
    pub correction_id: String,
    pub binding_hash: ActionHash,
    pub occurrence_binding_digest: StoredDigest,
    pub occurrence_digest: StoredDigest,
    pub reason: OccurrenceBindingCorrectionReason,
    pub rationale_commitment: Option<[u8; 32]>,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    MedicationAdministrationOccurrenceVerifierAuthorization(
        MedicationAdministrationOccurrenceVerifierAuthorization,
    ),
    QualifiedMedicationAdministrationOccurrenceBinding(
        QualifiedMedicationAdministrationOccurrenceBinding,
    ),
    MedicationAdministrationOccurrenceBindingCorrection(
        MedicationAdministrationOccurrenceBindingCorrection,
    ),
}

#[hdk_link_types]
pub enum LinkTypes {
    BindingToCorrections,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => {
                validate_create_entry(EntryCreationAction::Create(action), app_entry)
            }
            OpEntry::UpdateEntry { .. } => invalid(
                "Administration occurrence trust/evidence entries are append-only and cannot be updated",
            ),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(_) => invalid(
            "Administration occurrence trust/evidence entries are append-only and cannot be updated",
        ),
        FlatOp::RegisterDelete(_) => invalid(
            "Administration occurrence trust/evidence entries cannot be deleted; append a correction",
        ),
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            action,
            ..
        } => validate_create_link(link_type, base_address, target_address, action),
        FlatOp::RegisterDeleteLink { .. } => invalid(
            "Administration occurrence correction links are append-only and cannot be deleted",
        ),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(
    action: EntryCreationAction,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::MedicationAdministrationOccurrenceVerifierAuthorization(authorization) => {
            let shape = validate_authorization_shape(&authorization)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let config = occurrence_root_config()?;
            let root = require_root_authority(action.author(), &config)?;
            if !matches!(root, ValidateCallbackResult::Valid) {
                return Ok(root);
            }
            validate_authorization_window(action.timestamp(), &authorization, &config)
        }
        EntryTypes::QualifiedMedicationAdministrationOccurrenceBinding(binding) => {
            let shape = binding.attestation.validate_shape()?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let source = require_exact_administration_source(&binding.attestation)?;
            if !matches!(source, ValidateCallbackResult::Valid) {
                return Ok(source);
            }
            require_exact_verifier_authorization(action.author(), action.timestamp(), &binding)
        }
        EntryTypes::MedicationAdministrationOccurrenceBindingCorrection(correction) => {
            let shape = validate_correction_shape(&correction)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            require_correction_authority(action.author(), &correction)
        }
    }
}

fn validate_authorization_shape(
    authorization: &MedicationAdministrationOccurrenceVerifierAuthorization,
) -> ExternResult<ValidateCallbackResult> {
    if authorization.authorization_id.trim().is_empty() {
        return invalid("Administration occurrence verifier authorization ID is required");
    }
    require_digest_domain(
        authorization.occurrence_attestation_digest,
        DigestDomain::MedicationAdministrationOccurrenceBinding,
    )?;
    require_digest_domain(
        authorization.occurrence_binding_digest,
        DigestDomain::MedicationAdministrationOccurrenceBinding,
    )?;
    require_digest_domain(
        authorization.administration_receipt_digest,
        DigestDomain::MedicationAdministrationReceipt,
    )?;
    require_digest_domain(
        authorization.event_digest,
        DigestDomain::MedicationAdministrationEvent,
    )?;
    require_digest_domain(
        authorization.occurrence_digest,
        DigestDomain::MedicationAdministrationOccurrence,
    )?;
    require_digest_domain(
        authorization.medication_artifact_digest,
        DigestDomain::MedicationRequestArtifact,
    )?;
    if authorization.valid_until <= authorization.valid_from {
        return invalid("Administration occurrence authorization validity window is invalid");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_authorization_window(
    action_timestamp: Timestamp,
    authorization: &MedicationAdministrationOccurrenceVerifierAuthorization,
    config: &MedicationAdministrationOccurrenceRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    let duration = authorization.valid_until.as_micros() as i128
        - authorization.valid_from.as_micros() as i128;
    if duration <= 0
        || duration > config.max_verifier_authorization_duration_micros as i128
        || duration > ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS as i128
    {
        return invalid(
            "Administration occurrence authorization duration exceeds configured/hard limit",
        );
    }
    if action_timestamp > authorization.valid_until {
        return invalid("Administration occurrence authorization was expired when created");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_exact_administration_source(
    attestation: &MedicationAdministrationOccurrenceAttestationV1,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(attestation.administration_hash.clone())?;
    let administration: QualifiedMedicationAdministration =
        decode_entry(&record, "qualified medication administration")?;
    if administration.attestation.administration_receipt_digest
        != attestation.administration_receipt_digest
        || administration.attestation.event_digest != attestation.event_digest
        || administration.attestation.medication_artifact_digest
            != attestation.medication_artifact_digest
    {
        return invalid(
            "Administration occurrence binding does not match referenced qualified administration",
        );
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_exact_verifier_authorization(
    author: &AgentPubKey,
    action_timestamp: Timestamp,
    binding: &QualifiedMedicationAdministrationOccurrenceBinding,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(binding.verifier_authorization_hash.clone())?;
    let authorization: MedicationAdministrationOccurrenceVerifierAuthorization =
        decode_entry(&record, "administration occurrence verifier authorization")?;
    if &authorization.grantee != author {
        return invalid("Administration occurrence binding author is not authorization grantee");
    }
    if action_timestamp < authorization.valid_from || action_timestamp >= authorization.valid_until {
        return invalid("Administration occurrence binding is outside authorization validity");
    }

    let attestation = &binding.attestation;
    let attestation_digest = attestation.digest()?;
    if authorization.administration_hash != attestation.administration_hash
        || authorization.occurrence_attestation_digest != attestation_digest
        || authorization.occurrence_binding_digest != attestation.occurrence_binding_digest
        || authorization.administration_receipt_digest
            != attestation.administration_receipt_digest
        || authorization.event_digest != attestation.event_digest
        || authorization.occurrence_digest != attestation.occurrence_digest
        || authorization.medication_artifact_digest != attestation.medication_artifact_digest
        || authorization.dosage_index != attestation.dosage_index
    {
        return invalid(
            "Administration occurrence attestation does not exactly match root authorization",
        );
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_correction_shape(
    correction: &MedicationAdministrationOccurrenceBindingCorrection,
) -> ExternResult<ValidateCallbackResult> {
    if correction.correction_id.trim().is_empty() {
        return invalid("Administration occurrence binding correction ID is required");
    }
    require_digest_domain(
        correction.occurrence_binding_digest,
        DigestDomain::MedicationAdministrationOccurrenceBinding,
    )?;
    require_digest_domain(
        correction.occurrence_digest,
        DigestDomain::MedicationAdministrationOccurrence,
    )?;
    if correction
        .rationale_commitment
        .is_some_and(|commitment| commitment == [0u8; 32])
    {
        return invalid("Occurrence-binding correction rationale commitment cannot be zero");
    }
    if matches!(correction.reason, OccurrenceBindingCorrectionReason::Other)
        && correction.rationale_commitment.is_none()
    {
        return invalid("Other occurrence-binding correction requires rationale commitment");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_correction_authority(
    author: &AgentPubKey,
    correction: &MedicationAdministrationOccurrenceBindingCorrection,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(correction.binding_hash.clone())?;
    let binding: QualifiedMedicationAdministrationOccurrenceBinding =
        decode_entry(&record, "qualified administration occurrence binding")?;
    if binding.attestation.occurrence_binding_digest != correction.occurrence_binding_digest
        || binding.attestation.occurrence_digest != correction.occurrence_digest
    {
        return invalid("Occurrence-binding correction does not match target lineage");
    }
    if record.action().author() == author {
        return Ok(ValidateCallbackResult::Valid);
    }
    let config = occurrence_root_config()?;
    require_root_authority(author, &config)
}

fn validate_create_link(
    link_type: LinkTypes,
    base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    action: CreateLink,
) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        LinkTypes::BindingToCorrections => {
            let binding_hash = require_action_hash(base_address, "occurrence binding link base")?;
            let correction_hash = require_action_hash(target_address, "occurrence correction target")?;
            let binding_record = must_get_valid_record(binding_hash.clone())?;
            let binding: QualifiedMedicationAdministrationOccurrenceBinding =
                decode_entry(&binding_record, "qualified administration occurrence binding")?;
            let correction_record = must_get_valid_record(correction_hash)?;
            let correction: MedicationAdministrationOccurrenceBindingCorrection =
                decode_entry(&correction_record, "administration occurrence binding correction")?;
            if correction.binding_hash != binding_hash
                || correction.occurrence_binding_digest
                    != binding.attestation.occurrence_binding_digest
                || correction.occurrence_digest != binding.attestation.occurrence_digest
            {
                return invalid("Occurrence-binding correction link crosses lineage");
            }
            if correction_record.action().author() != &action.author {
                return invalid("Occurrence-binding correction link must be authored by correction author");
            }
            Ok(ValidateCallbackResult::Valid)
        }
    }
}

fn occurrence_root_config() -> ExternResult<MedicationAdministrationOccurrenceRootConfig> {
    let info = dna_info()?;
    let properties: serde_json::Value = serde_json::from_slice(info.modifiers.properties.bytes())
        .map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "DNA properties are not valid JSON for medication administration occurrence: {error}"
            )))
        })?;
    let value = properties
        .get("medication_administration_occurrence")
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "DNA properties do not configure medication_administration_occurrence trust roots"
                .to_string()
        )))?;
    let config: MedicationAdministrationOccurrenceRootConfig =
        serde_json::from_value(value.clone()).map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Invalid medication_administration_occurrence DNA properties: {error}"
            )))
        })?;
    if config.schema_version != CONFIG_VERSION {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Unsupported medication_administration_occurrence root config version {}",
            config.schema_version
        ))));
    }
    if config.max_verifier_authorization_duration_micros <= 0
        || config.max_verifier_authorization_duration_micros
            > ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS
    {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Administration occurrence authorization duration must be > 0 and <= 15 minutes"
                .to_string()
        )));
    }
    for (index, root) in config.root_authorities.iter().enumerate() {
        if config.root_authorities[..index].contains(root) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Administration occurrence root list contains duplicate AgentPubKeys".to_string()
            )));
        }
    }
    Ok(config)
}

fn require_root_authority(
    author: &AgentPubKey,
    config: &MedicationAdministrationOccurrenceRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    if !config.root_authorities.contains(author) {
        return invalid(
            "Administration occurrence verifier authorization requires a DNA-pinned root authority",
        );
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_digest_domain(digest: StoredDigest, expected: DigestDomain) -> ExternResult<()> {
    digest
        .validate_shape()
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?;
    if digest.domain != expected {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Administration occurrence digest domain mismatch: expected {expected:?}, got {:?}",
            digest.domain
        ))));
    }
    Ok(())
}

fn require_action_hash(hash: AnyLinkableHash, field: &'static str) -> ExternResult<ActionHash> {
    hash.into_action_hash().ok_or(wasm_error!(WasmErrorInner::Guest(format!(
        "{field} must be an ActionHash"
    ))))
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
    use mycelix_clinical_integrity::{DigestAlgorithm, DigestDomain};

    fn stored(domain: DigestDomain, seed: u8) -> StoredDigest {
        StoredDigest {
            algorithm: DigestAlgorithm::Blake3_256,
            domain,
            value: [seed; 32],
        }
    }

    fn attestation() -> MedicationAdministrationOccurrenceAttestationV1 {
        MedicationAdministrationOccurrenceAttestationV1 {
            schema_version: 1,
            administration_hash: ActionHash::from_raw_36(vec![1; 36]),
            occurrence_binding_digest: stored(
                DigestDomain::MedicationAdministrationOccurrenceBinding,
                2,
            ),
            administration_receipt_digest: stored(
                DigestDomain::MedicationAdministrationReceipt,
                3,
            ),
            event_digest: stored(DigestDomain::MedicationAdministrationEvent, 4),
            occurrence_digest: stored(DigestDomain::MedicationAdministrationOccurrence, 5),
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 6),
            dosage_index: 0,
        }
    }

    #[test]
    fn attestation_digest_changes_with_administration_source() {
        let first = attestation();
        let mut second = first.clone();
        second.administration_hash = ActionHash::from_raw_36(vec![9; 36]);
        assert_ne!(first.digest().unwrap(), second.digest().unwrap());
    }

    #[test]
    fn correction_other_requires_rationale_commitment() {
        let correction = MedicationAdministrationOccurrenceBindingCorrection {
            correction_id: "correction-a".into(),
            binding_hash: ActionHash::from_raw_36(vec![2; 36]),
            occurrence_binding_digest: stored(
                DigestDomain::MedicationAdministrationOccurrenceBinding,
                3,
            ),
            occurrence_digest: stored(DigestDomain::MedicationAdministrationOccurrence, 4),
            reason: OccurrenceBindingCorrectionReason::Other,
            rationale_commitment: None,
        };
        let result = validate_correction_shape(&correction).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }
}
