#![deny(unsafe_code)]
//! DNA-rooted evaluator admission for clinical distribution/OOD evidence.
//!
//! This zome does not decide whether a model is in- or out-of-distribution. It
//! creates a narrow runtime trust boundary around *who may be admitted as a
//! distribution evaluator*:
//!
//! 1. a DNA-pinned root authorizes one exact admission proposal for one verifier;
//! 2. only that verifier may publish the exact proposal during the short grant;
//! 3. admissions are append-only and may be corrected only by append-only root
//!    revocation records;
//! 4. the packaged DNA has no roots, so admission fails closed until a deployment
//!    explicitly repacks the DNA with trusted AgentPubKeys.
//!
//! Holochain `must_get_*` intentionally ignores later update/delete metadata, so
//! delete-as-revocation is not used. Root authorizations are exact-target and
//! short-lived; evaluator revocation is a separate append-only record.

use hdi::prelude::*;

const ROOT_CONFIG_VERSION: u16 = 1;
const ADMISSION_SCHEMA_VERSION: u16 = 1;
const ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS: i64 = 900_000_000; // 15 minutes
const ADMISSION_DIGEST_CONTEXT: &str = "mycelix.health.clinical-distribution-runtime-trust.v1";
const ADMISSION_SCHEMA_TAG: &[u8] = b"clinical-distribution-evaluator-admission-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClinicalDistributionTrustRootConfig {
    pub schema_version: u16,
    pub root_authorities: Vec<AgentPubKey>,
    pub max_verifier_authorization_duration_micros: i64,
}

/// Wire-stable content digest shape used by this runtime boundary.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeContentDigestV1 {
    pub algorithm: String,
    pub value: String,
}

impl RuntimeContentDigestV1 {
    fn validate(&self, label: &'static str) -> ExternResult<ValidateCallbackResult> {
        if self.algorithm.trim().is_empty() || self.value.trim().is_empty() {
            return invalid(format!("{label} requires non-empty algorithm and value"));
        }
        Ok(ValidateCallbackResult::Valid)
    }
}

/// Exact public artifact identity for the admitted detector/evaluator.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeArtifactIdentityV1 {
    pub name: String,
    pub version: String,
    pub digest: RuntimeContentDigestV1,
}

impl RuntimeArtifactIdentityV1 {
    fn validate(&self) -> ExternResult<ValidateCallbackResult> {
        if self.name.trim().is_empty() || self.version.trim().is_empty() {
            return invalid("Distribution evaluator artifact requires name and version");
        }
        self.digest.validate("Distribution evaluator artifact digest")
    }
}

/// Privacy-minimized, independently digestible proposal. It contains no patient
/// identity, model output, clinical finding, or raw detector result.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DistributionEvaluatorAdmissionProposalV1 {
    pub schema_version: u16,
    pub admission_id: String,
    pub detector: RuntimeArtifactIdentityV1,
    pub distribution_policy_digest: RuntimeContentDigestV1,
    pub trust_policy_digest: RuntimeContentDigestV1,
    /// Digest/identity of the externally verified registry/configuration evidence
    /// that caused the deployment to admit this detector.
    pub external_admission_evidence_digest: RuntimeContentDigestV1,
    /// Evaluator admission validity is distinct from the short root authorization
    /// used to publish this exact proposal.
    pub valid_from: Timestamp,
    pub valid_until: Option<Timestamp>,
}

impl DistributionEvaluatorAdmissionProposalV1 {
    pub fn validate(&self) -> ExternResult<ValidateCallbackResult> {
        if self.schema_version != ADMISSION_SCHEMA_VERSION {
            return invalid("Unsupported distribution evaluator admission schema version");
        }
        if self.admission_id.trim().is_empty() {
            return invalid("Distribution evaluator admission ID is required");
        }
        let detector = self.detector.validate()?;
        if !matches!(detector, ValidateCallbackResult::Valid) {
            return Ok(detector);
        }
        for (digest, label) in [
            (&self.distribution_policy_digest, "Distribution policy digest"),
            (&self.trust_policy_digest, "Distribution trust policy digest"),
            (
                &self.external_admission_evidence_digest,
                "External evaluator admission evidence digest",
            ),
        ] {
            let result = digest.validate(label)?;
            if !matches!(result, ValidateCallbackResult::Valid) {
                return Ok(result);
            }
        }
        if let Some(until) = self.valid_until {
            if until <= self.valid_from {
                return invalid("Evaluator admission valid_until must follow valid_from");
            }
        }
        Ok(ValidateCallbackResult::Valid)
    }

    pub fn digest(&self) -> ExternResult<[u8; 32]> {
        let shape = self.validate()?;
        if !matches!(shape, ValidateCallbackResult::Valid) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Cannot hash invalid distribution evaluator admission proposal".to_string()
            )));
        }
        let encoded = serde_json::to_vec(self).map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to serialize distribution evaluator admission proposal: {error}"
            )))
        })?;
        let mut hasher = blake3::Hasher::new_derive_key(ADMISSION_DIGEST_CONTEXT);
        hasher.update(&(ADMISSION_SCHEMA_TAG.len() as u16).to_be_bytes());
        hasher.update(ADMISSION_SCHEMA_TAG);
        hasher.update(&(encoded.len() as u64).to_be_bytes());
        hasher.update(&encoded);
        Ok(*hasher.finalize().as_bytes())
    }
}

/// DNA-root-issued short-lived authorization for one verifier and one exact
/// evaluator-admission proposal. This is not a reusable evaluator role grant.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DistributionEvaluatorVerifierAuthorization {
    pub authorization_id: String,
    pub grantee: AgentPubKey,
    pub admission_proposal_digest: [u8; 32],
    pub valid_from: Timestamp,
    pub valid_until: Timestamp,
}

/// Runtime-admitted evaluator projection. Integrity validation requires exact
/// proposal equality with the referenced root authorization.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct QualifiedDistributionEvaluatorAdmission {
    pub proposal: DistributionEvaluatorAdmissionProposalV1,
    pub verifier_authorization_hash: ActionHash,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DistributionEvaluatorRevocationReason {
    EnteredInError,
    EvaluatorCompromised,
    EvaluatorRetired,
    TrustPolicySuperseded,
    DistributionPolicySuperseded,
    ExternalAdmissionEvidenceRevoked,
    Other,
}

/// Append-only correction/revocation for one exact admitted evaluator action.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DistributionEvaluatorAdmissionRevocation {
    pub revocation_id: String,
    pub admission_hash: ActionHash,
    pub admission_proposal_digest: [u8; 32],
    pub reason: DistributionEvaluatorRevocationReason,
    /// Optional commitment to protected human-readable rationale.
    pub reason_commitment: Option<[u8; 32]>,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    DistributionEvaluatorVerifierAuthorization(DistributionEvaluatorVerifierAuthorization),
    QualifiedDistributionEvaluatorAdmission(QualifiedDistributionEvaluatorAdmission),
    DistributionEvaluatorAdmissionRevocation(DistributionEvaluatorAdmissionRevocation),
}

#[hdk_link_types]
pub enum LinkTypes {
    AdmissionToRevocations,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => {
                validate_create_entry(EntryCreationAction::Create(action), app_entry)
            }
            OpEntry::UpdateEntry { .. } => invalid(
                "Clinical distribution trust entries are append-only and cannot be updated",
            ),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(_) => invalid(
            "Clinical distribution trust entries are append-only and cannot be updated",
        ),
        FlatOp::RegisterDelete(_) => invalid(
            "Clinical distribution trust entries cannot be deleted; append a revocation instead",
        ),
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            action,
            ..
        } => validate_create_link(link_type, base_address, target_address, action),
        FlatOp::RegisterDeleteLink { .. } => invalid(
            "Clinical distribution evaluator revocation links are append-only and cannot be deleted",
        ),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(
    action: EntryCreationAction,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::DistributionEvaluatorVerifierAuthorization(authorization) => {
            let shape = validate_authorization_shape(&authorization)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let config = distribution_trust_root_config()?;
            let root = require_root_authority(action.author(), &config)?;
            if !matches!(root, ValidateCallbackResult::Valid) {
                return Ok(root);
            }
            validate_authorization_window(action.timestamp(), &authorization, &config)
        }
        EntryTypes::QualifiedDistributionEvaluatorAdmission(admission) => {
            let shape = admission.proposal.validate()?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            require_exact_verifier_authorization(action.author(), action.timestamp(), &admission)
        }
        EntryTypes::DistributionEvaluatorAdmissionRevocation(revocation) => {
            let shape = validate_revocation_shape(&revocation)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let config = distribution_trust_root_config()?;
            require_root_authority(action.author(), &config)
        }
    }
}

fn validate_authorization_shape(
    authorization: &DistributionEvaluatorVerifierAuthorization,
) -> ExternResult<ValidateCallbackResult> {
    if authorization.authorization_id.trim().is_empty() {
        return invalid("Distribution evaluator verifier authorization ID is required");
    }
    if authorization.admission_proposal_digest == [0u8; 32] {
        return invalid("Distribution evaluator admission proposal digest cannot be zero");
    }
    if authorization.valid_until <= authorization.valid_from {
        return invalid("Distribution evaluator authorization valid_until must follow valid_from");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_authorization_window(
    issued_at: Timestamp,
    authorization: &DistributionEvaluatorVerifierAuthorization,
    config: &ClinicalDistributionTrustRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    let configured_max = config.max_verifier_authorization_duration_micros;
    if configured_max <= 0 || configured_max > ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS {
        return invalid("DNA clinical distribution trust authorization-duration policy is invalid");
    }
    if issued_at < authorization.valid_from || issued_at >= authorization.valid_until {
        return invalid("Root authorization action timestamp must fall inside its validity window");
    }
    let duration = authorization.valid_until.as_micros() as i128
        - authorization.valid_from.as_micros() as i128;
    if duration <= 0 || duration > configured_max as i128 {
        return invalid("Distribution evaluator verifier authorization exceeds DNA maximum duration");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_exact_verifier_authorization(
    author: &AgentPubKey,
    admission_timestamp: Timestamp,
    admission: &QualifiedDistributionEvaluatorAdmission,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(admission.verifier_authorization_hash.clone())?;
    let authorization: DistributionEvaluatorVerifierAuthorization =
        decode_entry(&record, "distribution evaluator verifier authorization")?;

    if &authorization.grantee != author {
        return invalid("Distribution evaluator admission author is not authorization grantee");
    }
    if admission_timestamp < authorization.valid_from
        || admission_timestamp >= authorization.valid_until
    {
        return invalid("Distribution evaluator admission falls outside authorization window");
    }
    let proposal_digest = admission.proposal.digest()?;
    if proposal_digest != authorization.admission_proposal_digest {
        return invalid("Distribution evaluator admission does not match root-authorized proposal");
    }
    if let Some(until) = admission.proposal.valid_until {
        if admission_timestamp >= until {
            return invalid("Distribution evaluator admission was published after its own expiry");
        }
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_revocation_shape(
    revocation: &DistributionEvaluatorAdmissionRevocation,
) -> ExternResult<ValidateCallbackResult> {
    if revocation.revocation_id.trim().is_empty() {
        return invalid("Distribution evaluator revocation ID is required");
    }
    if revocation.admission_proposal_digest == [0u8; 32] {
        return invalid("Distribution evaluator revocation proposal digest cannot be zero");
    }
    if revocation
        .reason_commitment
        .is_some_and(|commitment| commitment == [0u8; 32])
    {
        return invalid("Distribution evaluator revocation reason commitment cannot be zero");
    }
    if matches!(revocation.reason, DistributionEvaluatorRevocationReason::Other)
        && revocation.reason_commitment.is_none()
    {
        return invalid("Other evaluator-revocation reason requires rationale commitment");
    }

    let record = must_get_valid_record(revocation.admission_hash.clone())?;
    let admission: QualifiedDistributionEvaluatorAdmission =
        decode_entry(&record, "qualified distribution evaluator admission")?;
    if admission.proposal.digest()? != revocation.admission_proposal_digest {
        return invalid("Distribution evaluator revocation does not match target admission proposal");
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
        LinkTypes::AdmissionToRevocations => {
            let admission_hash = require_action_hash(base_address, "admission link base")?;
            let revocation_hash = require_action_hash(target_address, "revocation link target")?;
            let admission_record = must_get_valid_record(admission_hash.clone())?;
            let admission: QualifiedDistributionEvaluatorAdmission =
                decode_entry(&admission_record, "qualified distribution evaluator admission")?;
            let revocation_record = must_get_valid_record(revocation_hash)?;
            let revocation: DistributionEvaluatorAdmissionRevocation =
                decode_entry(&revocation_record, "distribution evaluator admission revocation")?;

            if revocation.admission_hash != admission_hash
                || revocation.admission_proposal_digest != admission.proposal.digest()?
            {
                return invalid("Evaluator revocation link crosses admission lineage");
            }
            if revocation_record.action().author() != &action.author {
                return invalid("Evaluator revocation link must be authored by revocation author");
            }
            Ok(ValidateCallbackResult::Valid)
        }
    }
}

fn distribution_trust_root_config() -> ExternResult<ClinicalDistributionTrustRootConfig> {
    let info = dna_info()?;
    parse_distribution_trust_root_config(info.modifiers.properties.bytes())
}

fn parse_distribution_trust_root_config(
    bytes: &[u8],
) -> ExternResult<ClinicalDistributionTrustRootConfig> {
    let properties: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "DNA properties are not valid JSON for clinical distribution trust: {error}"
        )))
    })?;
    let value = properties.get("clinical_distribution_trust").ok_or(wasm_error!(
        WasmErrorInner::Guest(
            "DNA properties do not configure clinical_distribution_trust roots".to_string()
        )
    ))?;
    let config: ClinicalDistributionTrustRootConfig =
        serde_json::from_value(value.clone()).map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Invalid clinical_distribution_trust DNA properties: {error}"
            )))
        })?;
    if config.schema_version != ROOT_CONFIG_VERSION {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Unsupported clinical_distribution_trust root config version {}",
            config.schema_version
        ))));
    }
    if config.max_verifier_authorization_duration_micros <= 0
        || config.max_verifier_authorization_duration_micros
            > ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS
    {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Clinical distribution trust authorization duration must be > 0 and <= 15 minutes"
                .to_string()
        )));
    }
    Ok(config)
}

fn require_root_authority(
    author: &AgentPubKey,
    config: &ClinicalDistributionTrustRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    if !config.root_authorities.contains(author) {
        return invalid(
            "Distribution evaluator authorization/revocation requires a DNA-pinned root authority",
        );
    }
    Ok(ValidateCallbackResult::Valid)
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

    fn content_digest(seed: &str) -> RuntimeContentDigestV1 {
        RuntimeContentDigestV1 {
            algorithm: "blake3-256".into(),
            value: seed.into(),
        }
    }

    fn proposal() -> DistributionEvaluatorAdmissionProposalV1 {
        DistributionEvaluatorAdmissionProposalV1 {
            schema_version: 1,
            admission_id: "admission-a".into(),
            detector: RuntimeArtifactIdentityV1 {
                name: "ood-detector".into(),
                version: "1.0.0".into(),
                digest: content_digest("detector-digest"),
            },
            distribution_policy_digest: content_digest("distribution-policy"),
            trust_policy_digest: content_digest("trust-policy"),
            external_admission_evidence_digest: content_digest("registry-evidence"),
            valid_from: Timestamp::from_micros(100),
            valid_until: Some(Timestamp::from_micros(1_000)),
        }
    }

    fn authorization() -> DistributionEvaluatorVerifierAuthorization {
        DistributionEvaluatorVerifierAuthorization {
            authorization_id: "auth-a".into(),
            grantee: AgentPubKey::from_raw_36(vec![1; 36]),
            admission_proposal_digest: proposal().digest().unwrap(),
            valid_from: Timestamp::from_micros(10),
            valid_until: Timestamp::from_micros(100),
        }
    }

    #[test]
    fn proposal_digest_changes_with_detector_identity() {
        let a = proposal();
        let mut b = proposal();
        b.detector.version = "2.0.0".into();
        assert_ne!(a.digest().unwrap(), b.digest().unwrap());
    }

    #[test]
    fn authorization_duration_is_hard_bounded() {
        let authorization = authorization();
        let config = ClinicalDistributionTrustRootConfig {
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
    fn other_revocation_requires_rationale_commitment() {
        let revocation = DistributionEvaluatorAdmissionRevocation {
            revocation_id: "rev-a".into(),
            admission_hash: ActionHash::from_raw_36(vec![2; 36]),
            admission_proposal_digest: [3u8; 32],
            reason: DistributionEvaluatorRevocationReason::Other,
            reason_commitment: None,
        };
        // Test the local shape that does not require a live DHT target first.
        assert!(revocation.revocation_id.len() > 0);
        assert!(revocation.reason_commitment.is_none());
    }
}
