#![deny(unsafe_code)]
#![allow(clippy::collapsible_match)]
//! DNA-rooted medication activation attestation integrity zome.
//!
//! This zome intentionally does not re-run clinical reasoning. It creates a narrow
//! DHT trust boundary around the result of the pure clinical workflow stack:
//!
//! 1. a DNA-pinned root authority issues a short-lived authorization for one exact
//!    activation receipt and one exact privacy-minimized attestation payload;
//! 2. only the named verifier agent may publish that attestation;
//! 3. the activation action timestamp must fall inside the authorization window;
//! 4. activation/revocation evidence is append-only.
//!
//! Holochain `must_get_*` ignores later delete/update metadata, so authorization
//! revocation is NOT modeled as deleting an authorization. Authorizations are exact
//! target + short-lived. This bounds exposure and avoids a false delete-as-revoke
//! assumption.

use hdi::prelude::*;
use mycelix_clinical_integrity::{hash_canonical_bytes, DigestDomain, StoredDigest};

const ROOT_CONFIG_VERSION: u16 = 1;
const ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS: i64 = 900_000_000; // 15 minutes
const ATTESTATION_SCHEMA_TAG: &[u8] = b"mycelix-health/qualified-medication-activation-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MedicationActivationRootConfig {
    pub schema_version: u16,
    pub root_authorities: Vec<AgentPubKey>,
    pub max_verifier_authorization_duration_micros: i64,
}

/// Root-issued authorization for one exact activation receipt and one exact public
/// attestation projection. This is deliberately not a reusable role grant.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MedicationActivationVerifierAuthorization {
    pub authorization_id: String,
    pub grantee: AgentPubKey,
    pub activation_receipt_digest: StoredDigest,
    pub activation_attestation_digest: StoredDigest,
    pub authority_policy_digest: StoredDigest,
    pub safety_policy_digest: StoredDigest,
    pub safety_trust_policy_digest: StoredDigest,
    pub workflow_policy_digest: StoredDigest,
    pub valid_from: Timestamp,
    pub valid_until: Timestamp,
}

/// Privacy-minimized public attestation. It contains cryptographic identities and
/// policy/evidence lineage, not patient names, medication names, dosage text, or raw
/// clinical evidence. The exact activation receipt may remain encrypted/local.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct QualifiedMedicationActivation {
    pub activation_id: String,
    pub medication_artifact_digest: StoredDigest,
    pub activation_receipt_digest: StoredDigest,
    pub safety_context_digest: StoredDigest,
    pub safety_trust_receipt_digest: StoredDigest,
    pub authority_policy_digest: StoredDigest,
    pub safety_policy_digest: StoredDigest,
    pub safety_trust_policy_digest: StoredDigest,
    pub workflow_policy_digest: StoredDigest,
    pub verifier_authorization_hash: ActionHash,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ActivationRevocationReason {
    Superseded,
    MedicationStopped,
    SafetyEvidenceRevoked,
    ProfessionalAuthorityRevoked,
    SourceTrustRevoked,
    EnteredInError,
    Other,
}

/// Append-only corrective event. The receipt digest gives revocation a semantic
/// identity even if equivalent activation attestations were committed more than once.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MedicationActivationRevocation {
    pub revocation_id: String,
    pub activation_hash: ActionHash,
    pub activation_receipt_digest: StoredDigest,
    pub reason: ActivationRevocationReason,
    /// Optional keyed/content commitment to sensitive human-readable rationale.
    /// Raw rationale/PHI should remain local or encrypted.
    pub reason_commitment: Option<[u8; 32]>,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    MedicationActivationVerifierAuthorization(MedicationActivationVerifierAuthorization),
    QualifiedMedicationActivation(QualifiedMedicationActivation),
    MedicationActivationRevocation(MedicationActivationRevocation),
}

#[hdk_link_types]
pub enum LinkTypes {
    ActivationToRevocations,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => {
                validate_create_entry(EntryCreationAction::Create(action), app_entry)
            }
            OpEntry::UpdateEntry { .. } => invalid(
                "Medication activation trust/evidence entries are append-only and cannot be updated",
            ),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(_) => invalid(
            "Medication activation trust/evidence entries are append-only and cannot be updated",
        ),
        FlatOp::RegisterDelete(_) => invalid(
            "Medication activation trust/evidence entries cannot be deleted; use expiry or append a revocation",
        ),
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            action,
            ..
        } => validate_create_link(link_type, base_address, target_address, action),
        FlatOp::RegisterDeleteLink { .. } => invalid(
            "Medication activation revocation links are append-only and cannot be deleted",
        ),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(
    action: EntryCreationAction,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::MedicationActivationVerifierAuthorization(authorization) => {
            let shape = validate_authorization_shape(&authorization)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let config = activation_root_config()?;
            let root = require_root_authority(action.author(), &config)?;
            if !matches!(root, ValidateCallbackResult::Valid) {
                return Ok(root);
            }
            validate_authorization_window(action.timestamp(), &authorization, &config)
        }
        EntryTypes::QualifiedMedicationActivation(activation) => {
            let shape = validate_activation_shape(&activation)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            require_exact_verifier_authorization(action.author(), action.timestamp(), &activation)
        }
        EntryTypes::MedicationActivationRevocation(revocation) => {
            let shape = validate_revocation_shape(&revocation)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            require_revocation_authority(action.author(), &revocation)
        }
    }
}

fn validate_authorization_shape(
    authorization: &MedicationActivationVerifierAuthorization,
) -> ExternResult<ValidateCallbackResult> {
    if authorization.authorization_id.trim().is_empty() {
        return invalid("Medication activation verifier authorization ID is required");
    }
    require_digest_domain(
        authorization.activation_receipt_digest,
        DigestDomain::MedicationActivationReceipt,
    )?;
    require_digest_domain(
        authorization.activation_attestation_digest,
        DigestDomain::MedicationActivationAttestation,
    )?;
    require_digest_domain(
        authorization.authority_policy_digest,
        DigestDomain::AuthorityPolicy,
    )?;
    require_digest_domain(
        authorization.safety_policy_digest,
        DigestDomain::MedicationSafetyPolicy,
    )?;
    require_digest_domain(
        authorization.safety_trust_policy_digest,
        DigestDomain::MedicationSafetyTrustPolicy,
    )?;
    require_digest_domain(
        authorization.workflow_policy_digest,
        DigestDomain::WorkflowPolicy,
    )?;
    if authorization.valid_until <= authorization.valid_from {
        return invalid("Verifier authorization valid_until must follow valid_from");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_authorization_window(
    issued_at: Timestamp,
    authorization: &MedicationActivationVerifierAuthorization,
    config: &MedicationActivationRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    let configured_max = config.max_verifier_authorization_duration_micros;
    if configured_max <= 0 || configured_max > ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS {
        return invalid("DNA medication activation authorization-duration policy is invalid");
    }
    if issued_at < authorization.valid_from || issued_at >= authorization.valid_until {
        return invalid("Root authorization action timestamp must fall inside its validity window");
    }
    let duration = authorization.valid_until.as_micros() as i128
        - authorization.valid_from.as_micros() as i128;
    if duration <= 0 || duration > configured_max as i128 {
        return invalid("Verifier authorization exceeds DNA-pinned maximum duration");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_activation_shape(
    activation: &QualifiedMedicationActivation,
) -> ExternResult<ValidateCallbackResult> {
    if activation.activation_id.trim().is_empty() {
        return invalid("Qualified medication activation ID is required");
    }
    require_digest_domain(
        activation.medication_artifact_digest,
        DigestDomain::MedicationRequestArtifact,
    )?;
    require_digest_domain(
        activation.activation_receipt_digest,
        DigestDomain::MedicationActivationReceipt,
    )?;
    require_digest_domain(
        activation.safety_context_digest,
        DigestDomain::MedicationSafetyContext,
    )?;
    require_digest_domain(
        activation.safety_trust_receipt_digest,
        DigestDomain::MedicationSafetyTrustReceipt,
    )?;
    require_digest_domain(
        activation.authority_policy_digest,
        DigestDomain::AuthorityPolicy,
    )?;
    require_digest_domain(
        activation.safety_policy_digest,
        DigestDomain::MedicationSafetyPolicy,
    )?;
    require_digest_domain(
        activation.safety_trust_policy_digest,
        DigestDomain::MedicationSafetyTrustPolicy,
    )?;
    require_digest_domain(
        activation.workflow_policy_digest,
        DigestDomain::WorkflowPolicy,
    )?;
    Ok(ValidateCallbackResult::Valid)
}

/// Compute the exact privacy-minimized attestation identity approved by a root.
/// `verifier_authorization_hash` is excluded to avoid a circular dependency.
pub fn activation_attestation_digest(
    activation: &QualifiedMedicationActivation,
) -> ExternResult<StoredDigest> {
    let material = ActivationAttestationMaterial {
        activation_id: &activation.activation_id,
        medication_artifact_digest: activation.medication_artifact_digest,
        activation_receipt_digest: activation.activation_receipt_digest,
        safety_context_digest: activation.safety_context_digest,
        safety_trust_receipt_digest: activation.safety_trust_receipt_digest,
        authority_policy_digest: activation.authority_policy_digest,
        safety_policy_digest: activation.safety_policy_digest,
        safety_trust_policy_digest: activation.safety_trust_policy_digest,
        workflow_policy_digest: activation.workflow_policy_digest,
    };
    let encoded = serde_json::to_vec(&material).map_err(|error| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Failed to serialize medication activation attestation: {error}"
        )))
    })?;
    let mut framed = Vec::with_capacity(ATTESTATION_SCHEMA_TAG.len() + 1 + encoded.len());
    framed.extend_from_slice(ATTESTATION_SCHEMA_TAG);
    framed.push(0);
    framed.extend_from_slice(&encoded);
    Ok(hash_canonical_bytes(
        DigestDomain::MedicationActivationAttestation,
        &framed,
    )
    .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?
    .stored())
}

#[derive(Serialize)]
struct ActivationAttestationMaterial<'a> {
    activation_id: &'a str,
    medication_artifact_digest: StoredDigest,
    activation_receipt_digest: StoredDigest,
    safety_context_digest: StoredDigest,
    safety_trust_receipt_digest: StoredDigest,
    authority_policy_digest: StoredDigest,
    safety_policy_digest: StoredDigest,
    safety_trust_policy_digest: StoredDigest,
    workflow_policy_digest: StoredDigest,
}

fn validate_revocation_shape(
    revocation: &MedicationActivationRevocation,
) -> ExternResult<ValidateCallbackResult> {
    if revocation.revocation_id.trim().is_empty() {
        return invalid("Medication activation revocation ID is required");
    }
    require_digest_domain(
        revocation.activation_receipt_digest,
        DigestDomain::MedicationActivationReceipt,
    )?;
    if revocation
        .reason_commitment
        .is_some_and(|commitment| commitment == [0u8; 32])
    {
        return invalid("Medication activation revocation reason commitment cannot be zero");
    }
    if matches!(revocation.reason, ActivationRevocationReason::Other)
        && revocation.reason_commitment.is_none()
    {
        return invalid("Other activation-revocation reason requires a rationale commitment");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_exact_verifier_authorization(
    author: &AgentPubKey,
    activation_timestamp: Timestamp,
    activation: &QualifiedMedicationActivation,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(activation.verifier_authorization_hash.clone())?;
    let authorization: MedicationActivationVerifierAuthorization =
        decode_entry(&record, "verifier authorization")?;

    if &authorization.grantee != author {
        return invalid("Qualified activation author is not the verifier authorization grantee");
    }
    if activation_timestamp < authorization.valid_from
        || activation_timestamp >= authorization.valid_until
    {
        return invalid("Qualified activation action timestamp falls outside authorization validity");
    }
    if activation.activation_receipt_digest != authorization.activation_receipt_digest {
        return invalid("Qualified activation receipt does not match root authorization");
    }
    let actual_attestation_digest = activation_attestation_digest(activation)?;
    if actual_attestation_digest != authorization.activation_attestation_digest {
        return invalid("Qualified activation payload does not match root-authorized attestation");
    }
    if activation.authority_policy_digest != authorization.authority_policy_digest
        || activation.safety_policy_digest != authorization.safety_policy_digest
        || activation.safety_trust_policy_digest != authorization.safety_trust_policy_digest
        || activation.workflow_policy_digest != authorization.workflow_policy_digest
    {
        return invalid("Qualified activation policy set does not match root authorization");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_revocation_authority(
    author: &AgentPubKey,
    revocation: &MedicationActivationRevocation,
) -> ExternResult<ValidateCallbackResult> {
    let activation_record = must_get_valid_record(revocation.activation_hash.clone())?;
    let activation: QualifiedMedicationActivation =
        decode_entry(&activation_record, "qualified medication activation")?;
    if activation.activation_receipt_digest != revocation.activation_receipt_digest {
        return invalid("Activation revocation receipt digest does not match target activation");
    }

    if activation_record.action().author() == author {
        return Ok(ValidateCallbackResult::Valid);
    }

    let config = activation_root_config()?;
    require_root_authority(author, &config)
}

fn validate_create_link(
    link_type: LinkTypes,
    base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    action: CreateLink,
) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        LinkTypes::ActivationToRevocations => {
            let activation_hash = require_action_hash(base_address, "activation link base")?;
            let revocation_hash = require_action_hash(target_address, "revocation link target")?;
            let activation_record = must_get_valid_record(activation_hash.clone())?;
            let activation: QualifiedMedicationActivation =
                decode_entry(&activation_record, "qualified medication activation")?;
            let revocation_record = must_get_valid_record(revocation_hash)?;
            let revocation: MedicationActivationRevocation =
                decode_entry(&revocation_record, "medication activation revocation")?;
            if revocation.activation_hash != activation_hash
                || revocation.activation_receipt_digest != activation.activation_receipt_digest
            {
                return invalid(
                    "Activation-revocation link target references another activation lineage",
                );
            }
            if revocation_record.action().author() != &action.author {
                return invalid("Activation-revocation link must be authored by revocation author");
            }
            Ok(ValidateCallbackResult::Valid)
        }
    }
}

fn activation_root_config() -> ExternResult<MedicationActivationRootConfig> {
    let info = dna_info()?;
    parse_activation_root_config(info.modifiers.properties.bytes())
}

fn parse_activation_root_config(bytes: &[u8]) -> ExternResult<MedicationActivationRootConfig> {
    let properties: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "DNA properties are not valid JSON for medication activation: {error}"
        )))
    })?;
    let value = properties.get("medication_activation").ok_or(wasm_error!(
        WasmErrorInner::Guest(
            "DNA properties do not configure medication_activation trust roots".to_string()
        )
    ))?;
    let config: MedicationActivationRootConfig =
        serde_json::from_value(value.clone()).map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Invalid medication_activation DNA properties: {error}"
            )))
        })?;
    if config.schema_version != ROOT_CONFIG_VERSION {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Unsupported medication_activation root config version {}",
            config.schema_version
        ))));
    }
    if config.max_verifier_authorization_duration_micros <= 0
        || config.max_verifier_authorization_duration_micros
            > ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS
    {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Medication activation root config authorization duration must be > 0 and <= 15 minutes"
                .to_string()
        )));
    }
    Ok(config)
}

fn require_root_authority(
    author: &AgentPubKey,
    config: &MedicationActivationRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    if !config.root_authorities.contains(author) {
        return invalid(
            "Medication activation verifier authorization requires a DNA-pinned root authority",
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
            "Medication activation digest domain mismatch: expected {expected:?}, got {:?}",
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

    fn digest(domain: DigestDomain, seed: u8) -> StoredDigest {
        hash_canonical_bytes(domain, &[seed]).unwrap().stored()
    }

    fn authorization() -> MedicationActivationVerifierAuthorization {
        MedicationActivationVerifierAuthorization {
            authorization_id: "auth-a".into(),
            grantee: AgentPubKey::from_raw_36(vec![1; 36]),
            activation_receipt_digest: digest(DigestDomain::MedicationActivationReceipt, 1),
            activation_attestation_digest: digest(
                DigestDomain::MedicationActivationAttestation,
                2,
            ),
            authority_policy_digest: digest(DigestDomain::AuthorityPolicy, 3),
            safety_policy_digest: digest(DigestDomain::MedicationSafetyPolicy, 4),
            safety_trust_policy_digest: digest(DigestDomain::MedicationSafetyTrustPolicy, 5),
            workflow_policy_digest: digest(DigestDomain::WorkflowPolicy, 6),
            valid_from: Timestamp::from_micros(10),
            valid_until: Timestamp::from_micros(100),
        }
    }

    fn activation() -> QualifiedMedicationActivation {
        QualifiedMedicationActivation {
            activation_id: "activation-a".into(),
            medication_artifact_digest: digest(DigestDomain::MedicationRequestArtifact, 7),
            activation_receipt_digest: digest(DigestDomain::MedicationActivationReceipt, 8),
            safety_context_digest: digest(DigestDomain::MedicationSafetyContext, 9),
            safety_trust_receipt_digest: digest(DigestDomain::MedicationSafetyTrustReceipt, 10),
            authority_policy_digest: digest(DigestDomain::AuthorityPolicy, 11),
            safety_policy_digest: digest(DigestDomain::MedicationSafetyPolicy, 12),
            safety_trust_policy_digest: digest(DigestDomain::MedicationSafetyTrustPolicy, 13),
            workflow_policy_digest: digest(DigestDomain::WorkflowPolicy, 14),
            verifier_authorization_hash: ActionHash::from_raw_36(vec![15; 36]),
        }
    }

    #[test]
    fn authorization_shape_requires_exact_policy_domains() {
        let mut authorization = authorization();
        authorization.safety_policy_digest = digest(DigestDomain::AuthorityPolicy, 9);
        assert!(validate_authorization_shape(&authorization).is_err());
    }

    #[test]
    fn authorization_duration_is_hard_bounded() {
        let authorization = authorization();
        let config = MedicationActivationRootConfig {
            schema_version: 1,
            root_authorities: vec![],
            max_verifier_authorization_duration_micros: 50,
        };
        let result = validate_authorization_window(
            Timestamp::from_micros(10),
            &authorization,
            &config,
        )
        .unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn attestation_digest_changes_when_safety_policy_changes() {
        let a = activation();
        let mut b = activation();
        b.safety_policy_digest = digest(DigestDomain::MedicationSafetyPolicy, 99);
        assert_ne!(
            activation_attestation_digest(&a).unwrap(),
            activation_attestation_digest(&b).unwrap()
        );
    }

    #[test]
    fn other_revocation_requires_private_reason_commitment() {
        let revocation = MedicationActivationRevocation {
            revocation_id: "rev-a".into(),
            activation_hash: ActionHash::from_raw_36(vec![2; 36]),
            activation_receipt_digest: digest(DigestDomain::MedicationActivationReceipt, 3),
            reason: ActivationRevocationReason::Other,
            reason_commitment: None,
        };
        let result = validate_revocation_shape(&revocation).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }
}
