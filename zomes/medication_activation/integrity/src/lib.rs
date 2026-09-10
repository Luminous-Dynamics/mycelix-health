#![deny(unsafe_code)]
#![allow(clippy::collapsible_match)]
//! DNA-rooted medication activation attestation integrity zome.
//!
//! This zome does not repeat the clinical reasoning performed by the pure safety
//! stack. Instead it creates a narrow DHT trust boundary: only an agent holding a
//! verifier grant issued by a DNA-pinned root authority may publish a qualified
//! medication activation attestation, and the grant pins the exact policy digests
//! that the attestation is allowed to represent.
//!
//! The attestation is privacy-minimized. It carries cryptographic identities and
//! policy/evidence lineage, not patient names, medication names, dosage text, or raw
//! clinical evidence.

use hdi::prelude::*;
use mycelix_clinical_integrity::{DigestDomain, StoredDigest};

const ROOT_CONFIG_VERSION: u16 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MedicationActivationRootConfig {
    pub schema_version: u16,
    pub root_authorities: Vec<AgentPubKey>,
    pub max_verifier_grant_duration_micros: i64,
}

/// Root-authorized delegation to a small verifier agent/cell. The verifier is
/// allowed to attest only activations produced under these exact policy identities.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MedicationActivationVerifierGrant {
    pub grant_id: String,
    pub grantee: AgentPubKey,
    pub authority_policy_digest: StoredDigest,
    pub safety_policy_digest: StoredDigest,
    pub safety_trust_policy_digest: StoredDigest,
    pub workflow_policy_digest: StoredDigest,
    pub issued_at: Timestamp,
    pub valid_from: Timestamp,
    pub valid_until: Timestamp,
}

/// Public, privacy-minimized attestation that an authorized verifier consumed a
/// complete activation receipt under the policy set pinned in its verifier grant.
///
/// This is an attestation, not the underlying clinical record. The exact receipt and
/// sensitive evidence may remain in an encrypted/local store.
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
    pub verifier_grant_hash: ActionHash,
    pub activated_at: Timestamp,
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

/// Append-only terminal/corrective event. Revocation does not delete the original
/// clinical history; high-assurance read models derive current state from activation
/// plus revocation lineage.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MedicationActivationRevocation {
    pub revocation_id: String,
    pub activation_hash: ActionHash,
    pub reason: ActivationRevocationReason,
    /// Optional keyed/content commitment to sensitive human-readable rationale.
    /// Raw rationale/PHI should remain local or encrypted.
    pub reason_commitment: Option<[u8; 32]>,
    pub revoked_at: Timestamp,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    MedicationActivationVerifierGrant(MedicationActivationVerifierGrant),
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
        FlatOp::RegisterDelete(OpDelete { action }) => validate_delete_entry(action),
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
        EntryTypes::MedicationActivationVerifierGrant(grant) => {
            let shape = validate_grant_shape(&grant)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let config = activation_root_config()?;
            let root = require_root_authority(action.author(), &config)?;
            if !matches!(root, ValidateCallbackResult::Valid) {
                return Ok(root);
            }
            validate_grant_duration(&grant, &config)
        }
        EntryTypes::QualifiedMedicationActivation(activation) => {
            let shape = validate_activation_shape(&activation)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            require_active_verifier_grant(action.author(), &activation)
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

fn validate_delete_entry(action: Delete) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(action.deletes_address.clone())?;
    let maybe_grant = record
        .entry()
        .to_app_option::<MedicationActivationVerifierGrant>()
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?;

    if maybe_grant.is_none() {
        return invalid(
            "Qualified activation and revocation evidence cannot be deleted; append a revocation/correction instead",
        );
    }

    let config = activation_root_config()?;
    require_root_authority(&action.author, &config)
}

fn validate_grant_shape(
    grant: &MedicationActivationVerifierGrant,
) -> ExternResult<ValidateCallbackResult> {
    if grant.grant_id.trim().is_empty() {
        return invalid("Medication activation verifier grant ID is required");
    }
    require_digest_domain(grant.authority_policy_digest, DigestDomain::AuthorityPolicy)?;
    require_digest_domain(
        grant.safety_policy_digest,
        DigestDomain::MedicationSafetyPolicy,
    )?;
    require_digest_domain(
        grant.safety_trust_policy_digest,
        DigestDomain::MedicationSafetyTrustPolicy,
    )?;
    require_digest_domain(grant.workflow_policy_digest, DigestDomain::WorkflowPolicy)?;

    if grant.valid_from < grant.issued_at {
        return invalid("Verifier grant validity cannot begin before issuance");
    }
    if grant.valid_until <= grant.valid_from {
        return invalid("Verifier grant valid_until must follow valid_from");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_grant_duration(
    grant: &MedicationActivationVerifierGrant,
    config: &MedicationActivationRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    if config.max_verifier_grant_duration_micros <= 0 {
        return invalid("DNA medication activation root config has invalid grant-duration policy");
    }
    let duration = grant.valid_until.as_micros() as i128 - grant.valid_from.as_micros() as i128;
    if duration <= 0 || duration > config.max_verifier_grant_duration_micros as i128 {
        return invalid("Verifier grant exceeds DNA-pinned maximum duration");
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
    require_digest_domain(activation.workflow_policy_digest, DigestDomain::WorkflowPolicy)?;
    Ok(ValidateCallbackResult::Valid)
}

fn validate_revocation_shape(
    revocation: &MedicationActivationRevocation,
) -> ExternResult<ValidateCallbackResult> {
    if revocation.revocation_id.trim().is_empty() {
        return invalid("Medication activation revocation ID is required");
    }
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

fn require_active_verifier_grant(
    author: &AgentPubKey,
    activation: &QualifiedMedicationActivation,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(activation.verifier_grant_hash.clone())?;
    let grant: MedicationActivationVerifierGrant = decode_entry(&record, "verifier grant")?;

    if &grant.grantee != author {
        return invalid("Qualified activation author is not the verifier grant grantee");
    }
    if activation.activated_at < grant.valid_from || activation.activated_at >= grant.valid_until {
        return invalid("Qualified activation falls outside verifier grant validity");
    }
    if activation.authority_policy_digest != grant.authority_policy_digest
        || activation.safety_policy_digest != grant.safety_policy_digest
        || activation.safety_trust_policy_digest != grant.safety_trust_policy_digest
        || activation.workflow_policy_digest != grant.workflow_policy_digest
    {
        return invalid("Qualified activation policy set does not match verifier grant");
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
    if revocation.revoked_at < activation.activated_at {
        return invalid("Activation revocation cannot predate activation");
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
            let _: QualifiedMedicationActivation =
                decode_entry(&activation_record, "qualified medication activation")?;
            let revocation_record = must_get_valid_record(revocation_hash)?;
            let revocation: MedicationActivationRevocation =
                decode_entry(&revocation_record, "medication activation revocation")?;
            if revocation.activation_hash != activation_hash {
                return invalid("Activation-revocation link target references another activation");
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
    if config.max_verifier_grant_duration_micros <= 0 {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Medication activation root config maximum grant duration must be positive".to_string()
        )));
    }
    Ok(config)
}

fn require_root_authority(
    author: &AgentPubKey,
    config: &MedicationActivationRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    if !config.root_authorities.contains(author) {
        return invalid("Medication activation verifier grant/revocation requires a DNA-pinned root authority");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_digest_domain(
    digest: StoredDigest,
    expected: DigestDomain,
) -> ExternResult<()> {
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

fn require_action_hash(
    hash: AnyLinkableHash,
    field: &'static str,
) -> ExternResult<ActionHash> {
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
    use mycelix_clinical_integrity::{hash_canonical_bytes, DigestDomain};

    fn digest(domain: DigestDomain, seed: u8) -> StoredDigest {
        hash_canonical_bytes(domain, &[seed]).unwrap().stored()
    }

    fn grant() -> MedicationActivationVerifierGrant {
        MedicationActivationVerifierGrant {
            grant_id: "grant-a".into(),
            grantee: AgentPubKey::from_raw_36(vec![1; 36]),
            authority_policy_digest: digest(DigestDomain::AuthorityPolicy, 1),
            safety_policy_digest: digest(DigestDomain::MedicationSafetyPolicy, 2),
            safety_trust_policy_digest: digest(DigestDomain::MedicationSafetyTrustPolicy, 3),
            workflow_policy_digest: digest(DigestDomain::WorkflowPolicy, 4),
            issued_at: Timestamp::from_micros(10),
            valid_from: Timestamp::from_micros(10),
            valid_until: Timestamp::from_micros(100),
        }
    }

    #[test]
    fn grant_shape_requires_exact_policy_domains() {
        let mut grant = grant();
        grant.safety_policy_digest = digest(DigestDomain::AuthorityPolicy, 9);
        assert!(matches!(
            validate_grant_shape(&grant),
            Err(WasmError { .. })
        ));
    }

    #[test]
    fn grant_duration_is_bounded_by_dna_policy() {
        let grant = grant();
        let config = MedicationActivationRootConfig {
            schema_version: 1,
            root_authorities: vec![],
            max_verifier_grant_duration_micros: 50,
        };
        let result = validate_grant_duration(&grant, &config).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn other_revocation_requires_private_reason_commitment() {
        let revocation = MedicationActivationRevocation {
            revocation_id: "rev-a".into(),
            activation_hash: ActionHash::from_raw_36(vec![2; 36]),
            reason: ActivationRevocationReason::Other,
            reason_commitment: None,
            revoked_at: Timestamp::from_micros(100),
        };
        let result = validate_revocation_shape(&revocation).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }
}
