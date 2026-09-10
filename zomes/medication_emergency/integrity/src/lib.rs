#![deny(unsafe_code)]
#![allow(clippy::collapsible_match)]
//! DNA-rooted emergency medication override activation integrity zome.
//!
//! Emergency medication activation is intentionally a different provenance class
//! from ordinary qualified medication activation. This zome validates only the
//! privacy-minimized distributed attestation boundary; it does not re-run private
//! clinical reasoning or turn uncertainty into `Cleared`.
//!
//! A DNA-pinned root issues a short-lived authorization for one exact verifier,
//! override receipt, public attestation, safety basis, and policy set. Only that
//! verifier can publish the exact attestation during the authorization window.
//! All trust/evidence entries are append-only because Holochain `must_get_*` does
//! not make later delete metadata part of inductive entry validity.

use hdi::prelude::*;
use mycelix_clinical_integrity::{hash_canonical_bytes, DigestDomain, StoredDigest};

const ROOT_CONFIG_VERSION: u16 = 1;
const ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS: i64 = 900_000_000; // 15 minutes
const ATTESTATION_SCHEMA_TAG: &[u8] = b"mycelix-health/emergency-medication-activation-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmergencyMedicationRootConfig {
    pub schema_version: u16,
    pub root_authorities: Vec<AgentPubKey>,
    pub max_verifier_authorization_duration_micros: i64,
}

/// Emergency uncertainty basis. There is intentionally no `Cleared` or `Blocked`
/// variant in this type. Cleared medication belongs on the ordinary qualified path;
/// known-danger evidence requires a separately designed break-glass path, if any.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EmergencySafetyBasis {
    Indeterminate,
    RequiresReview,
}

/// Privacy-minimized public statement of one emergency override activation.
///
/// Patient identity, medication display text, diagnosis, dosage, laboratory data,
/// allergy data, and rationale text stay outside this DHT entry. Their exact private
/// evidence lineage is committed through the receipt/assessment/artifact digests.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EmergencyMedicationAttestationV1 {
    pub schema_version: u16,
    pub activation_id: String,
    pub medication_artifact_digest: StoredDigest,
    pub override_receipt_digest: StoredDigest,
    pub safety_assessment_digest: StoredDigest,
    pub safety_policy_digest: StoredDigest,
    pub authority_policy_digest: StoredDigest,
    pub emergency_policy_digest: StoredDigest,
    pub safety_basis: EmergencySafetyBasis,
}

impl EmergencyMedicationAttestationV1 {
    pub fn validate(&self) -> ExternResult<ValidateCallbackResult> {
        if self.schema_version != 1 {
            return invalid("Unsupported emergency medication attestation version");
        }
        if self.activation_id.trim().is_empty() {
            return invalid("Emergency medication activation ID is required");
        }
        require_digest_domain(
            self.medication_artifact_digest,
            DigestDomain::MedicationRequestArtifact,
        )?;
        require_digest_domain(
            self.override_receipt_digest,
            DigestDomain::EmergencyMedicationOverrideReceipt,
        )?;
        require_digest_domain(
            self.safety_assessment_digest,
            DigestDomain::MedicationSafetyAssessment,
        )?;
        require_digest_domain(
            self.safety_policy_digest,
            DigestDomain::MedicationSafetyPolicy,
        )?;
        require_digest_domain(self.authority_policy_digest, DigestDomain::AuthorityPolicy)?;
        require_digest_domain(
            self.emergency_policy_digest,
            DigestDomain::EmergencyMedicationOverridePolicy,
        )?;
        Ok(ValidateCallbackResult::Valid)
    }

    pub fn digest(&self) -> ExternResult<StoredDigest> {
        let shape = self.validate()?;
        if !matches!(shape, ValidateCallbackResult::Valid) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Cannot hash invalid emergency medication attestation".to_string()
            )));
        }
        let encoded = serde_json::to_vec(self).map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to serialize emergency medication attestation: {error}"
            )))
        })?;
        let mut framed = Vec::with_capacity(ATTESTATION_SCHEMA_TAG.len() + 1 + encoded.len());
        framed.extend_from_slice(ATTESTATION_SCHEMA_TAG);
        framed.push(0);
        framed.extend_from_slice(&encoded);
        Ok(hash_canonical_bytes(
            DigestDomain::EmergencyMedicationOverrideAttestation,
            &framed,
        )
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?
        .stored())
    }
}

/// Root-issued authorization for one exact private emergency receipt and one exact
/// public attestation. This is not a reusable emergency-verifier role.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct EmergencyMedicationVerifierAuthorization {
    pub authorization_id: String,
    pub grantee: AgentPubKey,
    pub override_receipt_digest: StoredDigest,
    pub emergency_attestation_digest: StoredDigest,
    pub safety_assessment_digest: StoredDigest,
    pub safety_policy_digest: StoredDigest,
    pub authority_policy_digest: StoredDigest,
    pub emergency_policy_digest: StoredDigest,
    pub safety_basis: EmergencySafetyBasis,
    pub valid_from: Timestamp,
    pub valid_until: Timestamp,
}

/// Distributed emergency activation. The provenance class is structural: this
/// cannot be decoded as `QualifiedMedicationActivation` from the ordinary zome.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct EmergencyMedicationActivation {
    pub attestation: EmergencyMedicationAttestationV1,
    pub verifier_authorization_hash: ActionHash,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EmergencyActivationTerminationReason {
    MedicationStopped,
    QualifiedActivationSuperseded,
    SafetyEvidenceResolved,
    ProfessionalAuthorityRevoked,
    EnteredInError,
    Other,
}

/// Append-only terminal/corrective event for emergency activation state.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct EmergencyMedicationTermination {
    pub termination_id: String,
    pub activation_hash: ActionHash,
    pub override_receipt_digest: StoredDigest,
    pub reason: EmergencyActivationTerminationReason,
    /// Commitment to protected rationale when needed. No raw PHI/rationale is
    /// required on the DHT.
    pub reason_commitment: Option<[u8; 32]>,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    EmergencyMedicationVerifierAuthorization(EmergencyMedicationVerifierAuthorization),
    EmergencyMedicationActivation(EmergencyMedicationActivation),
    EmergencyMedicationTermination(EmergencyMedicationTermination),
}

#[hdk_link_types]
pub enum LinkTypes {
    ActivationToTerminations,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => {
                validate_create_entry(EntryCreationAction::Create(action), app_entry)
            }
            OpEntry::UpdateEntry { .. } => invalid(
                "Emergency medication trust/evidence entries are append-only and cannot be updated",
            ),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(_) => invalid(
            "Emergency medication trust/evidence entries are append-only and cannot be updated",
        ),
        FlatOp::RegisterDelete(_) => invalid(
            "Emergency medication trust/evidence entries cannot be deleted; append a termination/correction",
        ),
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            action,
            ..
        } => validate_create_link(link_type, base_address, target_address, action),
        FlatOp::RegisterDeleteLink { .. } => invalid(
            "Emergency medication termination links are append-only and cannot be deleted",
        ),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(
    action: EntryCreationAction,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::EmergencyMedicationVerifierAuthorization(authorization) => {
            let shape = validate_authorization_shape(&authorization)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let config = emergency_root_config()?;
            let root = require_root_authority(action.author(), &config)?;
            if !matches!(root, ValidateCallbackResult::Valid) {
                return Ok(root);
            }
            validate_authorization_window(action.timestamp(), &authorization, &config)
        }
        EntryTypes::EmergencyMedicationActivation(activation) => {
            let shape = activation.attestation.validate()?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            require_exact_verifier_authorization(action.author(), action.timestamp(), &activation)
        }
        EntryTypes::EmergencyMedicationTermination(termination) => {
            let shape = validate_termination_shape(&termination)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            require_termination_authority(action.author(), &termination)
        }
    }
}

fn validate_authorization_shape(
    authorization: &EmergencyMedicationVerifierAuthorization,
) -> ExternResult<ValidateCallbackResult> {
    if authorization.authorization_id.trim().is_empty() {
        return invalid("Emergency medication verifier authorization ID is required");
    }
    require_digest_domain(
        authorization.override_receipt_digest,
        DigestDomain::EmergencyMedicationOverrideReceipt,
    )?;
    require_digest_domain(
        authorization.emergency_attestation_digest,
        DigestDomain::EmergencyMedicationOverrideAttestation,
    )?;
    require_digest_domain(
        authorization.safety_assessment_digest,
        DigestDomain::MedicationSafetyAssessment,
    )?;
    require_digest_domain(
        authorization.safety_policy_digest,
        DigestDomain::MedicationSafetyPolicy,
    )?;
    require_digest_domain(
        authorization.authority_policy_digest,
        DigestDomain::AuthorityPolicy,
    )?;
    require_digest_domain(
        authorization.emergency_policy_digest,
        DigestDomain::EmergencyMedicationOverridePolicy,
    )?;
    if authorization.valid_until <= authorization.valid_from {
        return invalid("Emergency verifier authorization valid_until must follow valid_from");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_authorization_window(
    issued_at: Timestamp,
    authorization: &EmergencyMedicationVerifierAuthorization,
    config: &EmergencyMedicationRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    let configured_max = config.max_verifier_authorization_duration_micros;
    if configured_max <= 0 || configured_max > ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS {
        return invalid("DNA emergency medication authorization-duration policy is invalid");
    }
    if issued_at < authorization.valid_from || issued_at >= authorization.valid_until {
        return invalid("Root emergency authorization action timestamp must fall inside its validity window");
    }
    let duration = authorization.valid_until.as_micros() as i128
        - authorization.valid_from.as_micros() as i128;
    if duration <= 0 || duration > configured_max as i128 {
        return invalid("Emergency verifier authorization exceeds DNA-pinned maximum duration");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_termination_shape(
    termination: &EmergencyMedicationTermination,
) -> ExternResult<ValidateCallbackResult> {
    if termination.termination_id.trim().is_empty() {
        return invalid("Emergency medication termination ID is required");
    }
    require_digest_domain(
        termination.override_receipt_digest,
        DigestDomain::EmergencyMedicationOverrideReceipt,
    )?;
    if termination
        .reason_commitment
        .is_some_and(|commitment| commitment == [0u8; 32])
    {
        return invalid("Emergency termination reason commitment cannot be zero");
    }
    if matches!(termination.reason, EmergencyActivationTerminationReason::Other)
        && termination.reason_commitment.is_none()
    {
        return invalid("Other emergency termination reason requires a rationale commitment");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_exact_verifier_authorization(
    author: &AgentPubKey,
    activation_timestamp: Timestamp,
    activation: &EmergencyMedicationActivation,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(activation.verifier_authorization_hash.clone())?;
    let authorization: EmergencyMedicationVerifierAuthorization =
        decode_entry(&record, "emergency verifier authorization")?;

    if &authorization.grantee != author {
        return invalid("Emergency activation author is not the verifier authorization grantee");
    }
    if activation_timestamp < authorization.valid_from
        || activation_timestamp >= authorization.valid_until
    {
        return invalid("Emergency activation action timestamp falls outside authorization validity");
    }
    let attestation = &activation.attestation;
    if attestation.override_receipt_digest != authorization.override_receipt_digest {
        return invalid("Emergency activation receipt does not match root authorization");
    }
    let actual_attestation_digest = attestation.digest()?;
    if actual_attestation_digest != authorization.emergency_attestation_digest {
        return invalid("Emergency activation payload does not match root-authorized attestation");
    }
    if attestation.safety_assessment_digest != authorization.safety_assessment_digest
        || attestation.safety_policy_digest != authorization.safety_policy_digest
        || attestation.authority_policy_digest != authorization.authority_policy_digest
        || attestation.emergency_policy_digest != authorization.emergency_policy_digest
        || attestation.safety_basis != authorization.safety_basis
    {
        return invalid("Emergency activation evidence/policy set does not match root authorization");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_termination_authority(
    author: &AgentPubKey,
    termination: &EmergencyMedicationTermination,
) -> ExternResult<ValidateCallbackResult> {
    let activation_record = must_get_valid_record(termination.activation_hash.clone())?;
    let activation: EmergencyMedicationActivation =
        decode_entry(&activation_record, "emergency medication activation")?;
    if activation.attestation.override_receipt_digest != termination.override_receipt_digest {
        return invalid("Emergency termination receipt digest does not match target activation");
    }

    if activation_record.action().author() == author {
        return Ok(ValidateCallbackResult::Valid);
    }

    let config = emergency_root_config()?;
    require_root_authority(author, &config)
}

fn validate_create_link(
    link_type: LinkTypes,
    base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    action: CreateLink,
) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        LinkTypes::ActivationToTerminations => {
            let activation_hash = require_action_hash(base_address, "emergency activation link base")?;
            let termination_hash =
                require_action_hash(target_address, "emergency termination link target")?;
            let activation_record = must_get_valid_record(activation_hash.clone())?;
            let activation: EmergencyMedicationActivation =
                decode_entry(&activation_record, "emergency medication activation")?;
            let termination_record = must_get_valid_record(termination_hash)?;
            let termination: EmergencyMedicationTermination =
                decode_entry(&termination_record, "emergency medication termination")?;
            if termination.activation_hash != activation_hash
                || termination.override_receipt_digest
                    != activation.attestation.override_receipt_digest
            {
                return invalid(
                    "Emergency activation-termination link references another activation lineage",
                );
            }
            if termination_record.action().author() != &action.author {
                return invalid("Emergency termination link must be authored by termination author");
            }
            Ok(ValidateCallbackResult::Valid)
        }
    }
}

fn emergency_root_config() -> ExternResult<EmergencyMedicationRootConfig> {
    let info = dna_info()?;
    parse_emergency_root_config(info.modifiers.properties.bytes())
}

fn parse_emergency_root_config(bytes: &[u8]) -> ExternResult<EmergencyMedicationRootConfig> {
    let properties: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "DNA properties are not valid JSON for emergency medication activation: {error}"
        )))
    })?;
    let value = properties.get("medication_emergency").ok_or(wasm_error!(
        WasmErrorInner::Guest(
            "DNA properties do not configure medication_emergency trust roots".to_string()
        )
    ))?;
    let config: EmergencyMedicationRootConfig =
        serde_json::from_value(value.clone()).map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Invalid medication_emergency DNA properties: {error}"
            )))
        })?;
    if config.schema_version != ROOT_CONFIG_VERSION {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Unsupported medication_emergency root config version {}",
            config.schema_version
        ))));
    }
    if config.max_verifier_authorization_duration_micros <= 0
        || config.max_verifier_authorization_duration_micros
            > ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS
    {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Medication emergency authorization duration must be positive and no more than 15 minutes"
                .to_string()
        )));
    }
    Ok(config)
}

fn require_root_authority(
    author: &AgentPubKey,
    config: &EmergencyMedicationRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    if !config.root_authorities.contains(author) {
        return invalid(
            "Emergency medication verifier authorization/termination requires a DNA-pinned root authority",
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
            "Emergency medication digest domain mismatch: expected {expected:?}, got {:?}",
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
    use mycelix_clinical_integrity::DigestAlgorithm;

    fn stored(domain: DigestDomain, seed: u8) -> StoredDigest {
        StoredDigest {
            algorithm: DigestAlgorithm::Blake3_256,
            domain,
            value: [seed; 32],
        }
    }

    fn attestation() -> EmergencyMedicationAttestationV1 {
        EmergencyMedicationAttestationV1 {
            schema_version: 1,
            activation_id: "emergency-1".into(),
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            override_receipt_digest: stored(DigestDomain::EmergencyMedicationOverrideReceipt, 2),
            safety_assessment_digest: stored(DigestDomain::MedicationSafetyAssessment, 3),
            safety_policy_digest: stored(DigestDomain::MedicationSafetyPolicy, 4),
            authority_policy_digest: stored(DigestDomain::AuthorityPolicy, 5),
            emergency_policy_digest: stored(DigestDomain::EmergencyMedicationOverridePolicy, 6),
            safety_basis: EmergencySafetyBasis::Indeterminate,
        }
    }

    #[test]
    fn valid_attestation_has_dedicated_domain() {
        let digest = attestation().digest().unwrap();
        assert_eq!(
            digest.domain,
            DigestDomain::EmergencyMedicationOverrideAttestation
        );
    }

    #[test]
    fn ordinary_activation_receipt_cannot_substitute_for_emergency_receipt() {
        let mut value = attestation();
        value.override_receipt_digest = stored(DigestDomain::MedicationActivationReceipt, 2);
        assert!(value.digest().is_err());
    }

    #[test]
    fn root_config_rejects_duration_over_absolute_cap() {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "medication_emergency": {
                "schema_version": 1,
                "root_authorities": [],
                "max_verifier_authorization_duration_micros": 900000001_i64
            }
        }))
        .unwrap();
        assert!(parse_emergency_root_config(&bytes).is_err());
    }
}
