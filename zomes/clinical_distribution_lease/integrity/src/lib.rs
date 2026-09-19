#![deny(unsafe_code)]
//! DNA-rooted short-lived positive currentness leases for clinical distribution evaluators.
//!
//! A lease is a positive assertion by a DNA-pinned root that one exact evaluator
//! admission may be treated as current for a narrowly bounded interval. It does
//! **not** assert global absence of revocation. Consumers must still combine the
//! lease with the separately materialized known-revocation snapshot from the
//! clinical_distribution_trust zome.

use clinical_distribution_trust_integrity::{
    QualifiedDistributionEvaluatorAdmission, RuntimeArtifactIdentityV1, RuntimeContentDigestV1,
};
use hdi::prelude::*;

const ROOT_CONFIG_VERSION: u16 = 1;
const LEASE_SCHEMA_VERSION: u16 = 1;
const ABSOLUTE_MAX_LEASE_DURATION_MICROS: i64 = 300_000_000; // five minutes

#[derive(Clone, Debug, Deserialize)]
struct DistributionLeaseRootConfig {
    schema_version: u16,
    root_authorities: Vec<AgentPubKey>,
}

/// Positive bounded currentness assertion for one exact qualified admission.
///
/// Runtime/cell identity is inherited from the Holochain action and DNA in which
/// this entry validates. External adapters must retain the lease action hash and
/// DNA/cell identity when exporting evidence across process boundaries.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ClinicalDistributionEvaluatorLeaseV1 {
    pub schema_version: u16,
    pub lease_id: String,
    pub admission_hash: ActionHash,
    pub admission_proposal_digest: [u8; 32],
    pub detector: RuntimeArtifactIdentityV1,
    pub distribution_policy_digest: RuntimeContentDigestV1,
    pub trust_policy_digest: RuntimeContentDigestV1,
    pub lease_sequence: u64,
    pub supersedes_lease_hash: Option<ActionHash>,
    pub valid_from: Timestamp,
    pub valid_until: Timestamp,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    ClinicalDistributionEvaluatorLeaseV1(ClinicalDistributionEvaluatorLeaseV1),
}

#[hdk_link_types]
pub enum LinkTypes {
    AdmissionToLeases,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::ClinicalDistributionEvaluatorLeaseV1(lease) => {
                    validate_lease_create(EntryCreationAction::Create(action), &lease)
                }
            },
            OpEntry::UpdateEntry { .. } => invalid(
                "Clinical distribution evaluator leases are immutable and cannot be updated",
            ),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(_) => invalid(
            "Clinical distribution evaluator leases are immutable and cannot be updated",
        ),
        FlatOp::RegisterDelete(_) => invalid(
            "Clinical distribution evaluator leases cannot be deleted; expiry is authoritative",
        ),
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            action,
            ..
        } => validate_create_link(link_type, base_address, target_address, action),
        FlatOp::RegisterDeleteLink { .. } => invalid(
            "Clinical distribution evaluator lease links are append-only and cannot be deleted",
        ),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_lease_create(
    action: EntryCreationAction,
    lease: &ClinicalDistributionEvaluatorLeaseV1,
) -> ExternResult<ValidateCallbackResult> {
    let local = validate_lease_local_shape(lease)?;
    if !matches!(local, ValidateCallbackResult::Valid) {
        return Ok(local);
    }

    let config = distribution_lease_root_config()?;
    let root = require_root_authority(action.author(), &config)?;
    if !matches!(root, ValidateCallbackResult::Valid) {
        return Ok(root);
    }

    validate_lease_window(action.timestamp(), lease)?;
    let target = validate_lease_target(lease)?;
    if !matches!(target, ValidateCallbackResult::Valid) {
        return Ok(target);
    }

    validate_supersession(action.timestamp(), lease, &config)
}

fn validate_lease_local_shape(
    lease: &ClinicalDistributionEvaluatorLeaseV1,
) -> ExternResult<ValidateCallbackResult> {
    if lease.schema_version != LEASE_SCHEMA_VERSION {
        return invalid("Unsupported clinical distribution evaluator lease schema version");
    }
    if lease.lease_id.trim().is_empty() {
        return invalid("Distribution evaluator lease ID is required");
    }
    if lease.admission_proposal_digest == [0u8; 32] {
        return invalid("Distribution evaluator lease proposal digest cannot be zero");
    }
    let detector = validate_runtime_artifact(&lease.detector, "Lease detector identity")?;
    if !matches!(detector, ValidateCallbackResult::Valid) {
        return Ok(detector);
    }
    for (digest, label) in [
        (&lease.distribution_policy_digest, "Lease distribution policy digest"),
        (&lease.trust_policy_digest, "Lease trust policy digest"),
    ] {
        let result = validate_runtime_digest(digest, label)?;
        if !matches!(result, ValidateCallbackResult::Valid) {
            return Ok(result);
        }
    }
    if lease.valid_until <= lease.valid_from {
        return invalid("Distribution evaluator lease valid_until must follow valid_from");
    }
    match (lease.lease_sequence, &lease.supersedes_lease_hash) {
        (0, None) => {}
        (0, Some(_)) => {
            return invalid("Lease sequence zero cannot supersede another lease");
        }
        (_, None) => {
            return invalid("Lease sequence greater than zero must supersede an exact prior lease");
        }
        (_, Some(_)) => {}
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_lease_window(
    issued_at: Timestamp,
    lease: &ClinicalDistributionEvaluatorLeaseV1,
) -> ExternResult<ValidateCallbackResult> {
    if issued_at < lease.valid_from || issued_at >= lease.valid_until {
        return invalid("Lease action timestamp must fall inside its validity window");
    }
    let duration = lease.valid_until.as_micros() as i128 - lease.valid_from.as_micros() as i128;
    if duration <= 0 || duration > ABSOLUTE_MAX_LEASE_DURATION_MICROS as i128 {
        return invalid("Distribution evaluator lease duration must be > 0 and <= 5 minutes");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_lease_target(
    lease: &ClinicalDistributionEvaluatorLeaseV1,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(lease.admission_hash.clone())?;
    let admission: QualifiedDistributionEvaluatorAdmission =
        decode_entry(&record, "qualified distribution evaluator admission")?;

    let proposal_shape = admission.proposal.validate()?;
    if !matches!(proposal_shape, ValidateCallbackResult::Valid) {
        return Ok(proposal_shape);
    }
    let proposal_digest = admission.proposal.digest()?;
    if proposal_digest != lease.admission_proposal_digest {
        return invalid("Lease does not match target evaluator admission proposal digest");
    }
    if admission.proposal.detector != lease.detector {
        return invalid("Lease detector identity differs from target evaluator admission");
    }
    if admission.proposal.distribution_policy_digest != lease.distribution_policy_digest {
        return invalid("Lease distribution policy differs from target evaluator admission");
    }
    if admission.proposal.trust_policy_digest != lease.trust_policy_digest {
        return invalid("Lease trust policy differs from target evaluator admission");
    }
    if lease.valid_from < admission.proposal.valid_from {
        return invalid("Lease cannot begin before underlying evaluator admission validity");
    }
    if let Some(admission_until) = admission.proposal.valid_until {
        if lease.valid_until > admission_until {
            return invalid("Lease cannot outlive underlying evaluator admission validity");
        }
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_supersession(
    issued_at: Timestamp,
    lease: &ClinicalDistributionEvaluatorLeaseV1,
    config: &DistributionLeaseRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    let Some(previous_hash) = &lease.supersedes_lease_hash else {
        return Ok(ValidateCallbackResult::Valid);
    };

    let previous_record = must_get_valid_record(previous_hash.clone())?;
    let previous: ClinicalDistributionEvaluatorLeaseV1 =
        decode_entry(&previous_record, "previous clinical distribution evaluator lease")?;

    let previous_root = require_root_authority(previous_record.action().author(), config)?;
    if !matches!(previous_root, ValidateCallbackResult::Valid) {
        return Ok(previous_root);
    }
    let previous_shape = validate_lease_local_shape(&previous)?;
    if !matches!(previous_shape, ValidateCallbackResult::Valid) {
        return Ok(previous_shape);
    }
    if previous_record.action().timestamp() >= issued_at {
        return invalid("Superseded evaluator lease must predate the new lease action");
    }
    if previous.lease_sequence.checked_add(1) != Some(lease.lease_sequence) {
        return invalid("Evaluator lease sequence must increment the exact superseded lease by one");
    }
    if previous.admission_hash != lease.admission_hash
        || previous.admission_proposal_digest != lease.admission_proposal_digest
        || previous.detector != lease.detector
        || previous.distribution_policy_digest != lease.distribution_policy_digest
        || previous.trust_policy_digest != lease.trust_policy_digest
    {
        return invalid("Evaluator lease supersession crosses admission or policy lineage");
    }
    if lease.valid_from < previous.valid_from {
        return invalid("Evaluator lease supersession cannot move valid_from backward");
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
        LinkTypes::AdmissionToLeases => {
            let admission_hash = require_action_hash(base_address, "lease link admission base")?;
            let lease_hash = require_action_hash(target_address, "lease link target")?;
            let lease_record = must_get_valid_record(lease_hash)?;
            let lease: ClinicalDistributionEvaluatorLeaseV1 =
                decode_entry(&lease_record, "clinical distribution evaluator lease")?;

            let config = distribution_lease_root_config()?;
            let lease_root = require_root_authority(lease_record.action().author(), &config)?;
            if !matches!(lease_root, ValidateCallbackResult::Valid) {
                return Ok(lease_root);
            }
            if lease_record.action().author() != &action.author {
                return invalid("Evaluator lease link must be authored by the lease root author");
            }
            if admission_hash != lease.admission_hash {
                return invalid("Evaluator lease link crosses admission identity");
            }
            let target = validate_lease_target(&lease)?;
            if !matches!(target, ValidateCallbackResult::Valid) {
                return Ok(target);
            }
            Ok(ValidateCallbackResult::Valid)
        }
    }
}

fn distribution_lease_root_config() -> ExternResult<DistributionLeaseRootConfig> {
    let info = dna_info()?;
    let properties: serde_json::Value = serde_json::from_slice(info.modifiers.properties.bytes())
        .map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "DNA properties are not valid JSON for clinical distribution lease trust: {error}"
            )))
        })?;
    let value = properties.get("clinical_distribution_trust").ok_or(wasm_error!(
        WasmErrorInner::Guest(
            "DNA properties do not configure clinical_distribution_trust roots".to_string()
        )
    ))?;
    let config: DistributionLeaseRootConfig = serde_json::from_value(value.clone()).map_err(
        |error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Invalid clinical_distribution_trust roots for evaluator leases: {error}"
            )))
        },
    )?;
    if config.schema_version != ROOT_CONFIG_VERSION {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Unsupported clinical_distribution_trust root config version {}",
            config.schema_version
        ))));
    }
    Ok(config)
}

fn require_root_authority(
    author: &AgentPubKey,
    config: &DistributionLeaseRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    if !config.root_authorities.contains(author) {
        return invalid("Distribution evaluator lease requires a DNA-pinned root authority");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_runtime_digest(
    digest: &RuntimeContentDigestV1,
    label: &'static str,
) -> ExternResult<ValidateCallbackResult> {
    if digest.algorithm.trim().is_empty() || digest.value.trim().is_empty() {
        return invalid(format!("{label} requires non-empty algorithm and value"));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_runtime_artifact(
    artifact: &RuntimeArtifactIdentityV1,
    label: &'static str,
) -> ExternResult<ValidateCallbackResult> {
    if artifact.name.trim().is_empty() || artifact.version.trim().is_empty() {
        return invalid(format!("{label} requires name and version"));
    }
    validate_runtime_digest(&artifact.digest, label)
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

    fn digest(seed: &str) -> RuntimeContentDigestV1 {
        RuntimeContentDigestV1 {
            algorithm: "blake3-256".into(),
            value: seed.into(),
        }
    }

    fn lease(sequence: u64, supersedes: Option<ActionHash>) -> ClinicalDistributionEvaluatorLeaseV1 {
        ClinicalDistributionEvaluatorLeaseV1 {
            schema_version: LEASE_SCHEMA_VERSION,
            lease_id: format!("lease-{sequence}"),
            admission_hash: ActionHash::from_raw_36(vec![1; 36]),
            admission_proposal_digest: [2u8; 32],
            detector: RuntimeArtifactIdentityV1 {
                name: "detector".into(),
                version: "1".into(),
                digest: digest("detector"),
            },
            distribution_policy_digest: digest("distribution"),
            trust_policy_digest: digest("trust"),
            lease_sequence: sequence,
            supersedes_lease_hash: supersedes,
            valid_from: Timestamp::from_micros(100),
            valid_until: Timestamp::from_micros(200),
        }
    }

    #[test]
    fn lease_duration_is_hard_capped_at_five_minutes() {
        let mut lease = lease(0, None);
        lease.valid_until = Timestamp::from_micros(
            lease.valid_from.as_micros() + ABSOLUTE_MAX_LEASE_DURATION_MICROS + 1,
        );
        let result = validate_lease_window(lease.valid_from, &lease).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn future_dated_lease_is_rejected() {
        let lease = lease(0, None);
        let result = validate_lease_window(Timestamp::from_micros(99), &lease).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn sequence_zero_cannot_claim_supersession() {
        let previous = ActionHash::from_raw_36(vec![3; 36]);
        let lease = lease(0, Some(previous));
        let result = validate_lease_local_shape(&lease).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn later_sequence_requires_superseded_hash() {
        let lease = lease(1, None);
        let result = validate_lease_local_shape(&lease).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }
}
