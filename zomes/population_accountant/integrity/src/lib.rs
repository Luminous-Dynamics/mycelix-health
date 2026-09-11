#![deny(unsafe_code)]
//! DNA-rooted trusted privacy-accountant state.
//!
//! Full accountant receipts and query/output identities remain protected off-DHT.
//! The DHT stores only a keyed commitment to the private receipt plus the minimum
//! state needed to detect lineage gaps/forks and enforce release-policy budgets.

use hdi::prelude::*;
use mycelix_clinical_integrity::StoredDigest;
use mycelix_clinical_population_release::{
    PopulationReleaseArtifactKindV1, PopulationReleaseDigestV1, PrivacyLossV1,
    ReducedFractionV1,
};

const CONFIG_VERSION: u16 = 1;
const MAX_ID_LEN: usize = 128;
const MAX_ROOT_AUTHORITIES: usize = 64;
const ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS: i64 = 900_000_000;
const ATTESTATION_HASH_CONTEXT: &str = "mycelix.health.population-accountant-attestation.v1";
const FRAME_VERSION: u8 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PopulationAccountantRootConfig {
    pub schema_version: u16,
    pub root_authorities: Vec<AgentPubKey>,
    pub max_verifier_authorization_duration_micros: i64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AccountantCommitmentSchemeV1 {
    HmacSha256,
    Blake3Keyed,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct OpaqueAccountantReceiptCommitmentV1 {
    pub scheme: AccountantCommitmentSchemeV1,
    pub value: [u8; 32],
}

impl OpaqueAccountantReceiptCommitmentV1 {
    fn validate(&self) -> ExternResult<ValidateCallbackResult> {
        if self.value == [0u8; 32] {
            return invalid("Privacy-accountant receipt commitment cannot be zero");
        }
        Ok(ValidateCallbackResult::Valid)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PopulationAccountantAttestationDigestV1 {
    pub value: [u8; 32],
}

impl PopulationAccountantAttestationDigestV1 {
    fn validate(&self) -> ExternResult<()> {
        if self.value == [0u8; 32] {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Population accountant attestation digest cannot be zero".into()
            )));
        }
        Ok(())
    }
}

/// Public privacy-minimized projection of one protected accountant receipt.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PopulationAccountantStateProjectionV1 {
    pub schema_version: u16,
    pub state_id: String,
    pub release_policy_digest: PopulationReleaseDigestV1,
    pub accountant_instance_digest: StoredDigest,
    pub accountant_method_digest: StoredDigest,
    pub sequence: u64,
    pub query_count: u64,
    pub cumulative_privacy_loss: PrivacyLossV1,
    pub previous_state_hash: Option<ActionHash>,
    pub private_receipt_commitment: OpaqueAccountantReceiptCommitmentV1,
}

impl PopulationAccountantStateProjectionV1 {
    pub fn validate_shape(&self) -> ExternResult<ValidateCallbackResult> {
        if self.schema_version != 1 {
            return invalid("Unsupported population accountant state version");
        }
        let id = validate_id("Population accountant state ID", &self.state_id)?;
        if !matches!(id, ValidateCallbackResult::Valid) {
            return Ok(id);
        }
        validate_release_policy_digest(self.release_policy_digest)?;
        require_stored_digest(self.accountant_instance_digest, "accountant instance")?;
        require_stored_digest(self.accountant_method_digest, "accountant method")?;
        self.cumulative_privacy_loss
            .validate()
            .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?;
        let commitment = self.private_receipt_commitment.validate()?;
        if !matches!(commitment, ValidateCallbackResult::Valid) {
            return Ok(commitment);
        }
        match (self.sequence, self.previous_state_hash.as_ref()) {
            (0, None) => {
                if self.query_count != 0 || self.cumulative_privacy_loss != PrivacyLossV1::ZERO {
                    return invalid("Accountant genesis must have zero queries and zero privacy loss");
                }
            }
            (0, Some(_)) => return invalid("Accountant genesis cannot name a predecessor"),
            (_, None) => return invalid("Non-genesis accountant state requires predecessor"),
            (_, Some(_)) => {
                if self.query_count != self.sequence {
                    return invalid("Accountant sequence must equal query count in v1");
                }
                if self.cumulative_privacy_loss == PrivacyLossV1::ZERO {
                    return invalid("Non-genesis accountant state cannot have zero cumulative loss");
                }
            }
        }
        Ok(ValidateCallbackResult::Valid)
    }

    pub fn digest(&self) -> ExternResult<PopulationAccountantAttestationDigestV1> {
        let valid = self.validate_shape()?;
        if !matches!(valid, ValidateCallbackResult::Valid) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Cannot hash invalid population accountant state projection".into()
            )));
        }
        let encoded = serde_json::to_vec(self).map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to serialize population accountant state projection: {error}"
            )))
        })?;
        let mut hasher = blake3::Hasher::new_derive_key(ATTESTATION_HASH_CONTEXT);
        hasher.update(&[FRAME_VERSION]);
        hasher.update(&(encoded.len() as u64).to_be_bytes());
        hasher.update(&encoded);
        let value = *hasher.finalize().as_bytes();
        if value == [0u8; 32] {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Population accountant attestation digest cannot be zero".into()
            )));
        }
        Ok(PopulationAccountantAttestationDigestV1 { value })
    }
}

/// One exact short-lived authorization from a DNA-pinned root.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PopulationAccountantVerifierAuthorization {
    pub authorization_id: String,
    pub grantee: AgentPubKey,
    pub public_state_digest: PopulationAccountantAttestationDigestV1,
    pub release_policy_digest: PopulationReleaseDigestV1,
    pub accountant_instance_digest: StoredDigest,
    pub accountant_method_digest: StoredDigest,
    pub sequence: u64,
    pub previous_state_hash: Option<ActionHash>,
    pub private_receipt_commitment: OpaqueAccountantReceiptCommitmentV1,
    pub valid_from: Timestamp,
    pub valid_until: Timestamp,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TrustedPopulationAccountantState {
    pub projection: PopulationAccountantStateProjectionV1,
    pub verifier_authorization_hash: ActionHash,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PopulationAccountantCorrectionReason {
    EnteredInError,
    AccountantImplementationRevoked,
    AccountantMethodRevoked,
    PolicySuperseded,
    DuplicateState,
    Other,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PopulationAccountantStateCorrection {
    pub correction_id: String,
    pub state_hash: ActionHash,
    pub private_receipt_commitment: OpaqueAccountantReceiptCommitmentV1,
    pub reason: PopulationAccountantCorrectionReason,
    pub rationale_commitment: Option<OpaqueAccountantReceiptCommitmentV1>,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    PopulationAccountantVerifierAuthorization(PopulationAccountantVerifierAuthorization),
    TrustedPopulationAccountantState(TrustedPopulationAccountantState),
    PopulationAccountantStateCorrection(PopulationAccountantStateCorrection),
}

#[hdk_link_types]
pub enum LinkTypes {
    StateToCorrections,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => {
                validate_create_entry(EntryCreationAction::Create(action), app_entry)
            }
            OpEntry::UpdateEntry { .. } => invalid("Population accountant trust entries are append-only"),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(_) => invalid("Population accountant trust entries cannot be updated"),
        FlatOp::RegisterDelete(_) => invalid("Population accountant trust entries cannot be deleted; append a correction"),
        FlatOp::RegisterCreateLink { link_type, base_address, target_address, action, .. } => {
            validate_create_link(link_type, base_address, target_address, action)
        }
        FlatOp::RegisterDeleteLink { .. } => invalid("Population accountant correction links are append-only"),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(
    action: EntryCreationAction,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::PopulationAccountantVerifierAuthorization(value) => {
            let shape = validate_authorization_shape(&value)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let config = population_accountant_root_config()?;
            let root = require_root_authority(action.author(), &config)?;
            if !matches!(root, ValidateCallbackResult::Valid) {
                return Ok(root);
            }
            validate_authorization_window(action.timestamp(), &value, &config)
        }
        EntryTypes::TrustedPopulationAccountantState(value) => {
            let shape = value.projection.validate_shape()?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let predecessor = require_predecessor_lineage(&value.projection)?;
            if !matches!(predecessor, ValidateCallbackResult::Valid) {
                return Ok(predecessor);
            }
            require_exact_verifier_authorization(action.author(), action.timestamp(), &value)
        }
        EntryTypes::PopulationAccountantStateCorrection(value) => {
            let shape = validate_correction_shape(&value)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let config = population_accountant_root_config()?;
            let root = require_root_authority(action.author(), &config)?;
            if !matches!(root, ValidateCallbackResult::Valid) {
                return Ok(root);
            }
            require_correction_target(&value)
        }
    }
}

fn validate_authorization_shape(
    value: &PopulationAccountantVerifierAuthorization,
) -> ExternResult<ValidateCallbackResult> {
    let id = validate_id("Population accountant authorization ID", &value.authorization_id)?;
    if !matches!(id, ValidateCallbackResult::Valid) {
        return Ok(id);
    }
    value.public_state_digest.validate()?;
    validate_release_policy_digest(value.release_policy_digest)?;
    require_stored_digest(value.accountant_instance_digest, "accountant instance")?;
    require_stored_digest(value.accountant_method_digest, "accountant method")?;
    let commitment = value.private_receipt_commitment.validate()?;
    if !matches!(commitment, ValidateCallbackResult::Valid) {
        return Ok(commitment);
    }
    match (value.sequence, value.previous_state_hash.as_ref()) {
        (0, None) => {}
        (0, Some(_)) => return invalid("Genesis accountant authorization cannot name predecessor"),
        (_, None) => return invalid("Non-genesis accountant authorization requires predecessor"),
        (_, Some(_)) => {}
    }
    if value.valid_until <= value.valid_from {
        return invalid("Population accountant authorization validity window is invalid");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_authorization_window(
    action_timestamp: Timestamp,
    authorization: &PopulationAccountantVerifierAuthorization,
    config: &PopulationAccountantRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    let duration = authorization.valid_until.as_micros() as i128
        - authorization.valid_from.as_micros() as i128;
    if duration <= 0
        || duration > config.max_verifier_authorization_duration_micros as i128
        || duration > ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS as i128
    {
        return invalid("Population accountant authorization exceeds configured/hard duration");
    }
    if action_timestamp > authorization.valid_until {
        return invalid("Population accountant authorization was expired when created");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_predecessor_lineage(
    projection: &PopulationAccountantStateProjectionV1,
) -> ExternResult<ValidateCallbackResult> {
    let Some(previous_hash) = projection.previous_state_hash.as_ref() else {
        return Ok(ValidateCallbackResult::Valid);
    };
    let record = must_get_valid_record(previous_hash.clone())?;
    let previous: TrustedPopulationAccountantState =
        decode_entry(&record, "trusted population accountant predecessor state")?;
    let prior = &previous.projection;
    if prior.release_policy_digest != projection.release_policy_digest
        || prior.accountant_instance_digest != projection.accountant_instance_digest
        || prior.accountant_method_digest != projection.accountant_method_digest
    {
        return invalid("Population accountant predecessor crosses policy/accountant lineage");
    }
    let expected_sequence = prior.sequence.checked_add(1).ok_or(wasm_error!(
        WasmErrorInner::Guest("Population accountant sequence overflow".into())
    ))?;
    let expected_queries = prior.query_count.checked_add(1).ok_or(wasm_error!(
        WasmErrorInner::Guest("Population accountant query-count overflow".into())
    ))?;
    if projection.sequence != expected_sequence || projection.query_count != expected_queries {
        return invalid("Population accountant predecessor sequence/query count is not contiguous");
    }
    if !privacy_loss_non_decreasing(
        projection.cumulative_privacy_loss,
        prior.cumulative_privacy_loss,
    ) {
        return invalid("Population accountant cumulative privacy loss decreased");
    }
    if projection.private_receipt_commitment == prior.private_receipt_commitment {
        return invalid("Population accountant successor reuses predecessor receipt commitment");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_exact_verifier_authorization(
    author: &AgentPubKey,
    action_timestamp: Timestamp,
    state: &TrustedPopulationAccountantState,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(state.verifier_authorization_hash.clone())?;
    let authorization: PopulationAccountantVerifierAuthorization =
        decode_entry(&record, "population accountant verifier authorization")?;
    let config = population_accountant_root_config()?;
    let root = require_root_authority(record.action().author(), &config)?;
    if !matches!(root, ValidateCallbackResult::Valid) {
        return Ok(root);
    }
    let shape = validate_authorization_shape(&authorization)?;
    if !matches!(shape, ValidateCallbackResult::Valid) {
        return Ok(shape);
    }
    if &authorization.grantee != author {
        return invalid("Population accountant state author is not authorization grantee");
    }
    if action_timestamp < authorization.valid_from || action_timestamp >= authorization.valid_until {
        return invalid("Population accountant state action is outside authorization validity");
    }
    let projection = &state.projection;
    if authorization.public_state_digest != projection.digest()?
        || authorization.release_policy_digest != projection.release_policy_digest
        || authorization.accountant_instance_digest != projection.accountant_instance_digest
        || authorization.accountant_method_digest != projection.accountant_method_digest
        || authorization.sequence != projection.sequence
        || authorization.previous_state_hash != projection.previous_state_hash
        || authorization.private_receipt_commitment != projection.private_receipt_commitment
    {
        return invalid("Population accountant state does not exactly match root authorization");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_correction_shape(
    value: &PopulationAccountantStateCorrection,
) -> ExternResult<ValidateCallbackResult> {
    let id = validate_id("Population accountant correction ID", &value.correction_id)?;
    if !matches!(id, ValidateCallbackResult::Valid) {
        return Ok(id);
    }
    let commitment = value.private_receipt_commitment.validate()?;
    if !matches!(commitment, ValidateCallbackResult::Valid) {
        return Ok(commitment);
    }
    if let Some(rationale) = value.rationale_commitment {
        let shape = rationale.validate()?;
        if !matches!(shape, ValidateCallbackResult::Valid) {
            return Ok(shape);
        }
    }
    if matches!(value.reason, PopulationAccountantCorrectionReason::Other)
        && value.rationale_commitment.is_none()
    {
        return invalid("Other population accountant correction requires rationale commitment");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_correction_target(
    value: &PopulationAccountantStateCorrection,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(value.state_hash.clone())?;
    let state: TrustedPopulationAccountantState =
        decode_entry(&record, "trusted population accountant state")?;
    if state.projection.private_receipt_commitment != value.private_receipt_commitment {
        return invalid("Population accountant correction commitment does not match target state");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_create_link(
    link_type: LinkTypes,
    base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    action: CreateLink,
) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        LinkTypes::StateToCorrections => {
            let state_hash = require_action_hash(base_address, "accountant correction base")?;
            let correction_hash = require_action_hash(target_address, "accountant correction target")?;
            let state_record = must_get_valid_record(state_hash.clone())?;
            let state: TrustedPopulationAccountantState =
                decode_entry(&state_record, "trusted population accountant state")?;
            let correction_record = must_get_valid_record(correction_hash)?;
            let correction: PopulationAccountantStateCorrection =
                decode_entry(&correction_record, "population accountant correction")?;
            if correction.state_hash != state_hash
                || correction.private_receipt_commitment
                    != state.projection.private_receipt_commitment
            {
                return invalid("Population accountant correction link crosses state lineage");
            }
            if correction_record.action().author() != &action.author {
                return invalid("Population accountant correction link must be authored by correction author");
            }
            let config = population_accountant_root_config()?;
            require_root_authority(&action.author, &config)
        }
    }
}

fn population_accountant_root_config() -> ExternResult<PopulationAccountantRootConfig> {
    let info = dna_info()?;
    let properties: serde_json::Value = serde_json::from_slice(info.modifiers.properties.bytes())
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(format!(
            "DNA properties are not valid JSON for population accountant trust: {error}"
        ))))?;
    let value = properties.get("population_accountant").ok_or(wasm_error!(
        WasmErrorInner::Guest("DNA properties do not configure population_accountant roots".into())
    ))?;
    let config: PopulationAccountantRootConfig = serde_json::from_value(value.clone())
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(format!(
            "Invalid population_accountant DNA properties: {error}"
        ))))?;
    if config.schema_version != CONFIG_VERSION {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Unsupported population_accountant root config version {}",
            config.schema_version
        ))));
    }
    if config.root_authorities.len() > MAX_ROOT_AUTHORITIES {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Population accountant root list exceeds maximum".into()
        )));
    }
    if config.max_verifier_authorization_duration_micros <= 0
        || config.max_verifier_authorization_duration_micros
            > ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS
    {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Population accountant authorization duration must be >0 and <=15 minutes".into()
        )));
    }
    for (index, root) in config.root_authorities.iter().enumerate() {
        if config.root_authorities[..index].contains(root) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Population accountant root list contains duplicate AgentPubKeys".into()
            )));
        }
    }
    Ok(config)
}

fn require_root_authority(
    author: &AgentPubKey,
    config: &PopulationAccountantRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    if !config.root_authorities.contains(author) {
        return invalid("Population accountant authorization/correction requires DNA-pinned root");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_release_policy_digest(digest: PopulationReleaseDigestV1) -> ExternResult<()> {
    digest.validate().map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?;
    if digest.kind != PopulationReleaseArtifactKindV1::ReleasePolicy {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Population accountant state requires a release-policy digest".into()
        )));
    }
    Ok(())
}

fn require_stored_digest(digest: StoredDigest, label: &'static str) -> ExternResult<()> {
    digest.validate_shape()
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(format!("Invalid {label} digest: {error}"))))
}

fn privacy_loss_non_decreasing(next: PrivacyLossV1, previous: PrivacyLossV1) -> bool {
    fraction_le(previous.epsilon, next.epsilon) && fraction_le(previous.delta, next.delta)
}

fn fraction_le(left: ReducedFractionV1, right: ReducedFractionV1) -> bool {
    (left.numerator as u128) * (right.denominator as u128)
        <= (right.numerator as u128) * (left.denominator as u128)
}

fn validate_id(label: &'static str, value: &str) -> ExternResult<ValidateCallbackResult> {
    if value.trim().is_empty() {
        return invalid(format!("{label} is required"));
    }
    if value.len() > MAX_ID_LEN {
        return invalid(format!("{label} exceeds {MAX_ID_LEN} bytes"));
    }
    Ok(ValidateCallbackResult::Valid)
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
    record.entry().to_app_option::<T>()
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
        StoredDigest { algorithm: DigestAlgorithm::Blake3_256, domain, value: [seed; 32] }
    }

    fn release_policy_digest(seed: u8) -> PopulationReleaseDigestV1 {
        PopulationReleaseDigestV1 { kind: PopulationReleaseArtifactKindV1::ReleasePolicy, value: [seed; 32] }
    }

    fn projection(sequence: u64) -> PopulationAccountantStateProjectionV1 {
        PopulationAccountantStateProjectionV1 {
            schema_version: 1,
            state_id: format!("accountant-state-{sequence}"),
            release_policy_digest: release_policy_digest(2),
            accountant_instance_digest: stored(DigestDomain::ClinicalArtifact, 3),
            accountant_method_digest: stored(DigestDomain::ClinicalArtifact, 4),
            sequence,
            query_count: sequence,
            cumulative_privacy_loss: if sequence == 0 { PrivacyLossV1::ZERO } else { PrivacyLossV1 { epsilon: ReducedFractionV1 { numerator: sequence, denominator: 10 }, delta: ReducedFractionV1::ZERO } },
            previous_state_hash: if sequence == 0 { None } else { Some(ActionHash::from_raw_36(vec![5; 36])) },
            private_receipt_commitment: OpaqueAccountantReceiptCommitmentV1 { scheme: AccountantCommitmentSchemeV1::Blake3Keyed, value: [6 + sequence as u8; 32] },
        }
    }

    #[test]
    fn genesis_requires_zero_budget_and_no_predecessor() {
        assert!(matches!(projection(0).validate_shape().unwrap(), ValidateCallbackResult::Valid));
        let mut bad = projection(0);
        bad.query_count = 1;
        assert!(matches!(bad.validate_shape().unwrap(), ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn non_genesis_requires_predecessor() {
        let mut bad = projection(1);
        bad.previous_state_hash = None;
        assert!(matches!(bad.validate_shape().unwrap(), ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn state_id_changes_authorized_projection_digest() {
        let first = projection(0);
        let first_digest = first.digest().unwrap();
        let mut second = first.clone();
        second.state_id = "another-state".into();
        assert_ne!(first_digest, second.digest().unwrap());
    }

    #[test]
    fn zero_private_commitment_is_rejected() {
        let mut bad = projection(0);
        bad.private_receipt_commitment.value = [0; 32];
        assert!(matches!(bad.validate_shape().unwrap(), ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn privacy_loss_comparison_is_exact_rational_arithmetic() {
        let previous = PrivacyLossV1 { epsilon: ReducedFractionV1 { numerator: 1, denominator: 10 }, delta: ReducedFractionV1::ZERO };
        let next = PrivacyLossV1 { epsilon: ReducedFractionV1 { numerator: 2, denominator: 10 }, delta: ReducedFractionV1::ZERO };
        assert!(privacy_loss_non_decreasing(next, previous));
        assert!(!privacy_loss_non_decreasing(previous, next));
    }
}
