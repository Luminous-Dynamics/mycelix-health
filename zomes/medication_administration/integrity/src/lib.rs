#![deny(unsafe_code)]
#![allow(clippy::collapsible_match)]
//! DNA-rooted qualified clinician medication administration attestations.
//!
//! The detailed administration event and receipt remain protected off-DHT. The DHT
//! stores only a privacy-minimized projection after a DNA-pinned root has authorized
//! one exact verifier to publish one exact receipt + attestation inside a short window.
//! Corrections are append-only; update/delete are never used as clinical revocation.

use hdi::prelude::*;
use mycelix_clinical_integrity::{hash_canonical_bytes, DigestDomain, StoredDigest};

const CONFIG_VERSION: u16 = 1;
const ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS: i64 = 900_000_000; // 15 min
const ATTESTATION_SCHEMA_TAG: &[u8] = b"mycelix-health/medication-administration-attestation-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MedicationAdministrationRootConfig {
    pub schema_version: u16,
    pub root_authorities: Vec<AgentPubKey>,
    pub max_verifier_authorization_duration_micros: i64,
}

/// Privacy-minimized public projection of one protected administration receipt.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MedicationAdministrationAttestationV1 {
    pub schema_version: u16,
    pub administration_id: String,
    pub administration_receipt_digest: StoredDigest,
    pub event_digest: StoredDigest,
    pub medication_artifact_digest: StoredDigest,
    pub activation_semantic_receipt_digest: StoredDigest,
    pub finalized_dispense_receipt_digest: StoredDigest,
    pub administrator_principal_binding: [u8; 32],
    pub authority_policy_digest: StoredDigest,
    pub administration_policy_digest: StoredDigest,
}

impl MedicationAdministrationAttestationV1 {
    pub fn validate(&self) -> ExternResult<ValidateCallbackResult> {
        if self.schema_version != 1 {
            return invalid("Unsupported medication administration attestation version");
        }
        if self.administration_id.trim().is_empty() {
            return invalid("Medication administration ID is required");
        }
        if self.administrator_principal_binding == [0u8; 32] {
            return invalid("Administrator principal binding cannot be zero");
        }
        require_digest_domain(
            self.administration_receipt_digest,
            DigestDomain::MedicationAdministrationReceipt,
        )?;
        require_digest_domain(
            self.event_digest,
            DigestDomain::MedicationAdministrationEvent,
        )?;
        require_digest_domain(
            self.medication_artifact_digest,
            DigestDomain::MedicationRequestArtifact,
        )?;
        require_activation_receipt_domain(self.activation_semantic_receipt_digest)?;
        require_digest_domain(
            self.finalized_dispense_receipt_digest,
            DigestDomain::MedicationDispenseReceipt,
        )?;
        require_digest_domain(self.authority_policy_digest, DigestDomain::AuthorityPolicy)?;
        require_digest_domain(
            self.administration_policy_digest,
            DigestDomain::MedicationAdministrationPolicy,
        )?;
        Ok(ValidateCallbackResult::Valid)
    }

    pub fn digest(&self) -> ExternResult<StoredDigest> {
        let shape = self.validate()?;
        if !matches!(shape, ValidateCallbackResult::Valid) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Cannot hash invalid medication administration attestation".to_string()
            )));
        }
        let encoded = serde_json::to_vec(self).map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to serialize medication administration attestation: {error}"
            )))
        })?;
        let mut framed = Vec::with_capacity(ATTESTATION_SCHEMA_TAG.len() + 1 + encoded.len());
        framed.extend_from_slice(ATTESTATION_SCHEMA_TAG);
        framed.push(0);
        framed.extend_from_slice(&encoded);
        Ok(hash_canonical_bytes(
            DigestDomain::MedicationAdministrationAttestation,
            &framed,
        )
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?
        .stored())
    }
}

/// Exact, short-lived root authorization. This is not a reusable verifier role.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MedicationAdministrationVerifierAuthorization {
    pub authorization_id: String,
    pub grantee: AgentPubKey,
    pub administration_receipt_digest: StoredDigest,
    pub administration_attestation_digest: StoredDigest,
    pub event_digest: StoredDigest,
    pub medication_artifact_digest: StoredDigest,
    pub activation_semantic_receipt_digest: StoredDigest,
    pub finalized_dispense_receipt_digest: StoredDigest,
    pub administrator_principal_binding: [u8; 32],
    pub authority_policy_digest: StoredDigest,
    pub administration_policy_digest: StoredDigest,
    pub valid_from: Timestamp,
    pub valid_until: Timestamp,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct QualifiedMedicationAdministration {
    pub attestation: MedicationAdministrationAttestationV1,
    pub verifier_authorization_hash: ActionHash,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdministrationCorrectionReason {
    EnteredInError,
    WrongPatient,
    WrongMedication,
    WrongDose,
    WrongRoute,
    DuplicateDocumentation,
    SourceEvidenceRevoked,
    Other,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MedicationAdministrationCorrection {
    pub correction_id: String,
    pub administration_hash: ActionHash,
    pub administration_receipt_digest: StoredDigest,
    pub reason: AdministrationCorrectionReason,
    /// Commitment to protected human-readable rationale. Raw rationale stays private.
    pub rationale_commitment: Option<[u8; 32]>,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    MedicationAdministrationVerifierAuthorization(MedicationAdministrationVerifierAuthorization),
    QualifiedMedicationAdministration(QualifiedMedicationAdministration),
    MedicationAdministrationCorrection(MedicationAdministrationCorrection),
}

#[hdk_link_types]
pub enum LinkTypes {
    AdministrationToCorrections,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => {
                validate_create_entry(EntryCreationAction::Create(action), app_entry)
            }
            OpEntry::UpdateEntry { .. } => invalid(
                "Medication administration trust/evidence entries are append-only and cannot be updated",
            ),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(_) => invalid(
            "Medication administration trust/evidence entries are append-only and cannot be updated",
        ),
        FlatOp::RegisterDelete(_) => invalid(
            "Medication administration trust/evidence entries cannot be deleted; append a correction",
        ),
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            action,
            ..
        } => validate_create_link(link_type, base_address, target_address, action),
        FlatOp::RegisterDeleteLink { .. } => invalid(
            "Medication administration correction links are append-only and cannot be deleted",
        ),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(
    action: EntryCreationAction,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::MedicationAdministrationVerifierAuthorization(authorization) => {
            let shape = validate_authorization_shape(&authorization)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let config = administration_root_config()?;
            let root = require_root_authority(action.author(), &config)?;
            if !matches!(root, ValidateCallbackResult::Valid) {
                return Ok(root);
            }
            validate_authorization_window(action.timestamp(), &authorization, &config)
        }
        EntryTypes::QualifiedMedicationAdministration(administration) => {
            let shape = administration.attestation.validate()?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            require_exact_verifier_authorization(
                action.author(),
                action.timestamp(),
                &administration,
            )
        }
        EntryTypes::MedicationAdministrationCorrection(correction) => {
            let shape = validate_correction_shape(&correction)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            require_correction_authority(action.author(), &correction)
        }
    }
}

fn validate_authorization_shape(
    authorization: &MedicationAdministrationVerifierAuthorization,
) -> ExternResult<ValidateCallbackResult> {
    if authorization.authorization_id.trim().is_empty() {
        return invalid("Medication administration verifier authorization ID is required");
    }
    if authorization.administrator_principal_binding == [0u8; 32] {
        return invalid("Administrator principal binding cannot be zero");
    }
    require_digest_domain(
        authorization.administration_receipt_digest,
        DigestDomain::MedicationAdministrationReceipt,
    )?;
    require_digest_domain(
        authorization.administration_attestation_digest,
        DigestDomain::MedicationAdministrationAttestation,
    )?;
    require_digest_domain(
        authorization.event_digest,
        DigestDomain::MedicationAdministrationEvent,
    )?;
    require_digest_domain(
        authorization.medication_artifact_digest,
        DigestDomain::MedicationRequestArtifact,
    )?;
    require_activation_receipt_domain(authorization.activation_semantic_receipt_digest)?;
    require_digest_domain(
        authorization.finalized_dispense_receipt_digest,
        DigestDomain::MedicationDispenseReceipt,
    )?;
    require_digest_domain(authorization.authority_policy_digest, DigestDomain::AuthorityPolicy)?;
    require_digest_domain(
        authorization.administration_policy_digest,
        DigestDomain::MedicationAdministrationPolicy,
    )?;
    if authorization.valid_until <= authorization.valid_from {
        return invalid("Administration verifier authorization validity window is invalid");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_authorization_window(
    action_timestamp: Timestamp,
    authorization: &MedicationAdministrationVerifierAuthorization,
    config: &MedicationAdministrationRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    let duration = authorization.valid_until.as_micros() as i128
        - authorization.valid_from.as_micros() as i128;
    if duration <= 0
        || duration > config.max_verifier_authorization_duration_micros as i128
        || duration > ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS as i128
    {
        return invalid("Administration verifier authorization duration exceeds configured/hard limit");
    }
    if action_timestamp > authorization.valid_until {
        return invalid("Administration verifier authorization was already expired when created");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_exact_verifier_authorization(
    author: &AgentPubKey,
    action_timestamp: Timestamp,
    administration: &QualifiedMedicationAdministration,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(administration.verifier_authorization_hash.clone())?;
    let authorization: MedicationAdministrationVerifierAuthorization =
        decode_entry(&record, "administration verifier authorization")?;
    if &authorization.grantee != author {
        return invalid("Administration author is not authorization grantee");
    }
    if action_timestamp < authorization.valid_from || action_timestamp >= authorization.valid_until {
        return invalid("Administration action timestamp is outside authorization validity");
    }

    let attestation = &administration.attestation;
    let attestation_digest = attestation.digest()?;
    if attestation_digest != authorization.administration_attestation_digest
        || attestation.administration_receipt_digest != authorization.administration_receipt_digest
        || attestation.event_digest != authorization.event_digest
        || attestation.medication_artifact_digest != authorization.medication_artifact_digest
        || attestation.activation_semantic_receipt_digest
            != authorization.activation_semantic_receipt_digest
        || attestation.finalized_dispense_receipt_digest
            != authorization.finalized_dispense_receipt_digest
        || attestation.administrator_principal_binding
            != authorization.administrator_principal_binding
        || attestation.authority_policy_digest != authorization.authority_policy_digest
        || attestation.administration_policy_digest != authorization.administration_policy_digest
    {
        return invalid("Administration attestation does not exactly match root authorization");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_correction_shape(
    correction: &MedicationAdministrationCorrection,
) -> ExternResult<ValidateCallbackResult> {
    if correction.correction_id.trim().is_empty() {
        return invalid("Medication administration correction ID is required");
    }
    require_digest_domain(
        correction.administration_receipt_digest,
        DigestDomain::MedicationAdministrationReceipt,
    )?;
    if correction
        .rationale_commitment
        .is_some_and(|commitment| commitment == [0u8; 32])
    {
        return invalid("Administration correction rationale commitment cannot be zero");
    }
    if matches!(correction.reason, AdministrationCorrectionReason::Other)
        && correction.rationale_commitment.is_none()
    {
        return invalid("Other administration correction requires a rationale commitment");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_correction_authority(
    author: &AgentPubKey,
    correction: &MedicationAdministrationCorrection,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(correction.administration_hash.clone())?;
    let administration: QualifiedMedicationAdministration =
        decode_entry(&record, "qualified medication administration")?;
    if administration.attestation.administration_receipt_digest
        != correction.administration_receipt_digest
    {
        return invalid("Administration correction receipt digest does not match target");
    }
    if record.action().author() == author {
        return Ok(ValidateCallbackResult::Valid);
    }
    let config = administration_root_config()?;
    require_root_authority(author, &config)
}

fn validate_create_link(
    link_type: LinkTypes,
    base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    action: CreateLink,
) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        LinkTypes::AdministrationToCorrections => {
            let administration_hash = require_action_hash(base_address, "administration link base")?;
            let correction_hash = require_action_hash(target_address, "correction link target")?;
            let administration_record = must_get_valid_record(administration_hash.clone())?;
            let administration: QualifiedMedicationAdministration =
                decode_entry(&administration_record, "qualified medication administration")?;
            let correction_record = must_get_valid_record(correction_hash)?;
            let correction: MedicationAdministrationCorrection =
                decode_entry(&correction_record, "administration correction")?;
            if correction.administration_hash != administration_hash
                || correction.administration_receipt_digest
                    != administration.attestation.administration_receipt_digest
            {
                return invalid("Administration-correction link crosses receipt lineage");
            }
            if correction_record.action().author() != &action.author {
                return invalid("Administration-correction link must be authored by correction author");
            }
            Ok(ValidateCallbackResult::Valid)
        }
    }
}

fn administration_root_config() -> ExternResult<MedicationAdministrationRootConfig> {
    let info = dna_info()?;
    let properties: serde_json::Value = serde_json::from_slice(info.modifiers.properties.bytes())
        .map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "DNA properties are not valid JSON for medication administration: {error}"
            )))
        })?;
    let value = properties.get("medication_administration").ok_or(wasm_error!(
        WasmErrorInner::Guest(
            "DNA properties do not configure medication_administration trust roots".to_string()
        )
    ))?;
    let config: MedicationAdministrationRootConfig = serde_json::from_value(value.clone())
        .map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Invalid medication_administration DNA properties: {error}"
            )))
        })?;
    if config.schema_version != CONFIG_VERSION {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Unsupported medication_administration root config version {}",
            config.schema_version
        ))));
    }
    if config.max_verifier_authorization_duration_micros <= 0
        || config.max_verifier_authorization_duration_micros
            > ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS
    {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Medication administration authorization duration must be > 0 and <= 15 minutes"
                .to_string()
        )));
    }
    for (index, root) in config.root_authorities.iter().enumerate() {
        if config.root_authorities[..index].contains(root) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Medication administration root list contains duplicate AgentPubKeys".to_string()
            )));
        }
    }
    Ok(config)
}

fn require_root_authority(
    author: &AgentPubKey,
    config: &MedicationAdministrationRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    if !config.root_authorities.contains(author) {
        return invalid(
            "Medication administration verifier authorization requires a DNA-pinned root authority",
        );
    }
    Ok(ValidateCallbackResult::Valid)
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
            "Medication administration activation receipt has invalid domain {:?}",
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
            "Medication administration digest domain mismatch: expected {expected:?}, got {:?}",
            digest.domain
        ))));
    }
    Ok(())
}

fn require_action_hash(hash: AnyLinkableHash, field: &'static str) -> ExternResult<ActionHash> {
    hash.into_action_hash().ok_or(wasm_error!(WasmErrorInner::Guest(
        format!("{field} must be an ActionHash")
    )))
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
    use mycelix_clinical_integrity::{hash_canonical_bytes, DigestAlgorithm};

    fn digest(domain: DigestDomain, seed: u8) -> StoredDigest {
        hash_canonical_bytes(domain, &[seed]).unwrap().stored()
    }

    fn stored(domain: DigestDomain, seed: u8) -> StoredDigest {
        StoredDigest {
            algorithm: DigestAlgorithm::Blake3_256,
            domain,
            value: [seed; 32],
        }
    }

    fn attestation() -> MedicationAdministrationAttestationV1 {
        MedicationAdministrationAttestationV1 {
            schema_version: 1,
            administration_id: "admin-a".into(),
            administration_receipt_digest: digest(DigestDomain::MedicationAdministrationReceipt, 2),
            event_digest: digest(DigestDomain::MedicationAdministrationEvent, 3),
            medication_artifact_digest: digest(DigestDomain::MedicationRequestArtifact, 4),
            activation_semantic_receipt_digest: digest(DigestDomain::MedicationActivationReceipt, 5),
            finalized_dispense_receipt_digest: digest(DigestDomain::MedicationDispenseReceipt, 6),
            administrator_principal_binding: [7; 32],
            authority_policy_digest: digest(DigestDomain::AuthorityPolicy, 8),
            administration_policy_digest: digest(DigestDomain::MedicationAdministrationPolicy, 9),
        }
    }

    fn authorization() -> MedicationAdministrationVerifierAuthorization {
        let attestation = attestation();
        MedicationAdministrationVerifierAuthorization {
            authorization_id: "admin-auth-a".into(),
            grantee: AgentPubKey::from_raw_36(vec![1; 36]),
            administration_receipt_digest: attestation.administration_receipt_digest,
            administration_attestation_digest: attestation.digest().unwrap(),
            event_digest: attestation.event_digest,
            medication_artifact_digest: attestation.medication_artifact_digest,
            activation_semantic_receipt_digest: attestation.activation_semantic_receipt_digest,
            finalized_dispense_receipt_digest: attestation.finalized_dispense_receipt_digest,
            administrator_principal_binding: attestation.administrator_principal_binding,
            authority_policy_digest: attestation.authority_policy_digest,
            administration_policy_digest: attestation.administration_policy_digest,
            valid_from: Timestamp::from_micros(10),
            valid_until: Timestamp::from_micros(100),
        }
    }

    #[test]
    fn authorization_rejects_wrong_receipt_domain() {
        let mut value = authorization();
        value.administration_receipt_digest = stored(DigestDomain::MedicationDispenseReceipt, 2);
        assert!(validate_authorization_shape(&value).is_err());
    }

    #[test]
    fn attestation_id_changes_authorized_identity() {
        let original = attestation();
        let original_digest = original.digest().unwrap();
        let mut changed = original.clone();
        changed.administration_id = "admin-b".into();
        assert_ne!(original_digest, changed.digest().unwrap());
    }

    #[test]
    fn correction_other_requires_commitment() {
        let value = MedicationAdministrationCorrection {
            correction_id: "correction-a".into(),
            administration_hash: ActionHash::from_raw_36(vec![2; 36]),
            administration_receipt_digest: digest(DigestDomain::MedicationAdministrationReceipt, 3),
            reason: AdministrationCorrectionReason::Other,
            rationale_commitment: None,
        };
        let result = validate_correction_shape(&value).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn authorization_duration_is_hard_bounded() {
        let value = authorization();
        let config = MedicationAdministrationRootConfig {
            schema_version: 1,
            root_authorities: Vec::new(),
            max_verifier_authorization_duration_micros: 50,
        };
        let result = validate_authorization_window(Timestamp::from_micros(10), &value, &config)
            .unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }
}
