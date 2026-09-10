#![deny(unsafe_code)]
#![allow(clippy::collapsible_match)]
//! DNA-rooted qualified clinical causal-assessment attestations.
//!
//! The detailed causal assessment and qualification receipt remain protected off-DHT.
//! The DHT stores only a minimal projection after a DNA-pinned root authorizes one
//! exact verifier to publish one exact receipt + attestation in a short time window.
//! Corrections are append-only; updates/deletes never represent clinical correction.

use hdi::prelude::*;
use mycelix_clinical_causality::CausalConclusionV1;
use mycelix_clinical_integrity::{hash_canonical_bytes, DigestDomain, StoredDigest};

const CONFIG_VERSION: u16 = 1;
const ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS: i64 = 900_000_000; // 15 minutes
const ATTESTATION_SCHEMA_TAG: &[u8] = b"mycelix-health/clinical-causal-qualified-attestation-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClinicalCausalityRootConfig {
    pub schema_version: u16,
    pub root_authorities: Vec<AgentPubKey>,
    pub max_verifier_authorization_duration_micros: i64,
}

/// Privacy-minimized public projection of a protected qualified causal receipt.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QualifiedCausalAssessmentAttestationV1 {
    pub schema_version: u16,
    pub attestation_id: String,
    pub qualified_receipt_digest: StoredDigest,
    pub qualification_policy_digest: StoredDigest,
    pub conclusion: CausalConclusionV1,
}

impl QualifiedCausalAssessmentAttestationV1 {
    pub fn validate(&self) -> ExternResult<ValidateCallbackResult> {
        if self.schema_version != 1 {
            return invalid("Unsupported qualified causal attestation version");
        }
        if self.attestation_id.trim().is_empty() {
            return invalid("Causal attestation ID is required");
        }
        require_digest_domain(
            self.qualified_receipt_digest,
            DigestDomain::ClinicalCausalQualifiedReceipt,
        )?;
        require_digest_domain(
            self.qualification_policy_digest,
            DigestDomain::ClinicalCausalQualificationPolicy,
        )?;
        Ok(ValidateCallbackResult::Valid)
    }

    pub fn digest(&self) -> ExternResult<StoredDigest> {
        let shape = self.validate()?;
        if !matches!(shape, ValidateCallbackResult::Valid) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Cannot hash invalid qualified causal attestation".to_string()
            )));
        }
        let encoded = serde_json::to_vec(self).map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to serialize qualified causal attestation: {error}"
            )))
        })?;
        let mut framed = Vec::with_capacity(ATTESTATION_SCHEMA_TAG.len() + 1 + encoded.len());
        framed.extend_from_slice(ATTESTATION_SCHEMA_TAG);
        framed.push(0);
        framed.extend_from_slice(&encoded);
        Ok(hash_canonical_bytes(
            DigestDomain::ClinicalCausalQualifiedAttestation,
            &framed,
        )
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?
        .stored())
    }
}

/// Exact, short-lived authorization issued by a DNA-pinned root. It is not a role.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CausalAssessmentVerifierAuthorization {
    pub authorization_id: String,
    pub grantee: AgentPubKey,
    pub qualified_receipt_digest: StoredDigest,
    pub public_attestation_digest: StoredDigest,
    pub qualification_policy_digest: StoredDigest,
    pub conclusion: CausalConclusionV1,
    pub valid_from: Timestamp,
    pub valid_until: Timestamp,
}

/// Public proof that an authorized verifier revalidated one private qualified receipt.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct QualifiedCausalAssessmentAttestation {
    pub attestation: QualifiedCausalAssessmentAttestationV1,
    pub verifier_authorization_hash: ActionHash,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CausalAttestationCorrectionReason {
    EnteredInError,
    SourceEvidenceRevoked,
    AssessorAuthorityRevoked,
    EvidenceTrustRevoked,
    PolicySuperseded,
    DuplicateAttestation,
    Other,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CausalAttestationCorrection {
    pub correction_id: String,
    pub attestation_hash: ActionHash,
    pub qualified_receipt_digest: StoredDigest,
    pub reason: CausalAttestationCorrectionReason,
    /// Commitment to protected rationale. Raw text remains outside the public DHT.
    pub rationale_commitment: Option<[u8; 32]>,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    CausalAssessmentVerifierAuthorization(CausalAssessmentVerifierAuthorization),
    QualifiedCausalAssessmentAttestation(QualifiedCausalAssessmentAttestation),
    CausalAttestationCorrection(CausalAttestationCorrection),
}

#[hdk_link_types]
pub enum LinkTypes {
    AttestationToCorrections,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => {
                validate_create_entry(EntryCreationAction::Create(action), app_entry)
            }
            OpEntry::UpdateEntry { .. } => invalid(
                "Clinical causality trust/evidence entries are append-only and cannot be updated",
            ),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(_) => invalid(
            "Clinical causality trust/evidence entries are append-only and cannot be updated",
        ),
        FlatOp::RegisterDelete(_) => invalid(
            "Clinical causality attestations cannot be deleted; append a correction",
        ),
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            action,
            ..
        } => validate_create_link(link_type, base_address, target_address, action),
        FlatOp::RegisterDeleteLink { .. } => invalid(
            "Clinical causality correction links are append-only and cannot be deleted",
        ),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(
    action: EntryCreationAction,
    entry: EntryTypes,
) -> ExternResult<ValidateCallbackResult> {
    match entry {
        EntryTypes::CausalAssessmentVerifierAuthorization(authorization) => {
            let shape = validate_authorization_shape(&authorization)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let config = clinical_causality_root_config()?;
            let root = require_root_authority(action.author(), &config)?;
            if !matches!(root, ValidateCallbackResult::Valid) {
                return Ok(root);
            }
            validate_authorization_window(action.timestamp(), &authorization, &config)
        }
        EntryTypes::QualifiedCausalAssessmentAttestation(attestation) => {
            let shape = attestation.attestation.validate()?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            require_exact_verifier_authorization(action.author(), action.timestamp(), &attestation)
        }
        EntryTypes::CausalAttestationCorrection(correction) => {
            let shape = validate_correction_shape(&correction)?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let config = clinical_causality_root_config()?;
            require_root_authority(action.author(), &config)
        }
    }
}

fn validate_authorization_shape(
    authorization: &CausalAssessmentVerifierAuthorization,
) -> ExternResult<ValidateCallbackResult> {
    if authorization.authorization_id.trim().is_empty() {
        return invalid("Causal verifier authorization ID is required");
    }
    require_digest_domain(
        authorization.qualified_receipt_digest,
        DigestDomain::ClinicalCausalQualifiedReceipt,
    )?;
    require_digest_domain(
        authorization.public_attestation_digest,
        DigestDomain::ClinicalCausalQualifiedAttestation,
    )?;
    require_digest_domain(
        authorization.qualification_policy_digest,
        DigestDomain::ClinicalCausalQualificationPolicy,
    )?;
    if authorization.valid_until <= authorization.valid_from {
        return invalid("Causal verifier authorization validity window is invalid");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_authorization_window(
    action_timestamp: Timestamp,
    authorization: &CausalAssessmentVerifierAuthorization,
    config: &ClinicalCausalityRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    let duration = authorization.valid_until.as_micros() as i128
        - authorization.valid_from.as_micros() as i128;
    if duration <= 0
        || duration > config.max_verifier_authorization_duration_micros as i128
        || duration > ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS as i128
    {
        return invalid("Causal verifier authorization duration exceeds configured/hard limit");
    }
    if action_timestamp > authorization.valid_until {
        return invalid("Causal verifier authorization was already expired when created");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_exact_verifier_authorization(
    author: &AgentPubKey,
    action_timestamp: Timestamp,
    value: &QualifiedCausalAssessmentAttestation,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(value.verifier_authorization_hash.clone())?;
    let authorization: CausalAssessmentVerifierAuthorization =
        decode_entry(&record, "causal verifier authorization")?;
    if &authorization.grantee != author {
        return invalid("Causal attestation author is not authorization grantee");
    }
    if action_timestamp < authorization.valid_from || action_timestamp >= authorization.valid_until {
        return invalid("Causal attestation action timestamp is outside authorization validity");
    }
    let attestation = &value.attestation;
    let attestation_digest = attestation.digest()?;
    if attestation_digest != authorization.public_attestation_digest
        || attestation.qualified_receipt_digest != authorization.qualified_receipt_digest
        || attestation.qualification_policy_digest != authorization.qualification_policy_digest
        || attestation.conclusion != authorization.conclusion
    {
        return invalid("Causal attestation does not exactly match root authorization");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_correction_shape(
    correction: &CausalAttestationCorrection,
) -> ExternResult<ValidateCallbackResult> {
    if correction.correction_id.trim().is_empty() {
        return invalid("Causal attestation correction ID is required");
    }
    require_digest_domain(
        correction.qualified_receipt_digest,
        DigestDomain::ClinicalCausalQualifiedReceipt,
    )?;
    if correction
        .rationale_commitment
        .is_some_and(|commitment| commitment == [0u8; 32])
    {
        return invalid("Causal correction rationale commitment cannot be zero");
    }
    if matches!(correction.reason, CausalAttestationCorrectionReason::Other)
        && correction.rationale_commitment.is_none()
    {
        return invalid("Other causal correction requires a rationale commitment");
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
        LinkTypes::AttestationToCorrections => {
            let attestation_hash = require_action_hash(base_address, "causal attestation link base")?;
            let correction_hash = require_action_hash(target_address, "causal correction link target")?;
            let attestation_record = must_get_valid_record(attestation_hash.clone())?;
            let attestation: QualifiedCausalAssessmentAttestation =
                decode_entry(&attestation_record, "qualified causal attestation")?;
            let correction_record = must_get_valid_record(correction_hash)?;
            let correction: CausalAttestationCorrection =
                decode_entry(&correction_record, "causal attestation correction")?;
            if correction.attestation_hash != attestation_hash
                || correction.qualified_receipt_digest
                    != attestation.attestation.qualified_receipt_digest
            {
                return invalid("Causal attestation-correction link crosses receipt lineage");
            }
            if correction_record.action().author() != &action.author {
                return invalid("Causal correction link must be authored by correction author");
            }
            let config = clinical_causality_root_config()?;
            require_root_authority(&action.author, &config)
        }
    }
}

fn clinical_causality_root_config() -> ExternResult<ClinicalCausalityRootConfig> {
    let info = dna_info()?;
    let properties: serde_json::Value = serde_json::from_slice(info.modifiers.properties.bytes())
        .map_err(|error| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "DNA properties are not valid JSON for clinical causality: {error}"
            )))
        })?;
    let value = properties.get("clinical_causality").ok_or(wasm_error!(
        WasmErrorInner::Guest(
            "DNA properties do not configure clinical_causality trust roots".to_string()
        )
    ))?;
    let config: ClinicalCausalityRootConfig = serde_json::from_value(value.clone()).map_err(|error| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Invalid clinical_causality DNA properties: {error}"
        )))
    })?;
    if config.schema_version != CONFIG_VERSION {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Unsupported clinical_causality root config version {}",
            config.schema_version
        ))));
    }
    if config.max_verifier_authorization_duration_micros <= 0
        || config.max_verifier_authorization_duration_micros
            > ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS
    {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Clinical causality authorization duration must be > 0 and <= 15 minutes".to_string()
        )));
    }
    for (index, root) in config.root_authorities.iter().enumerate() {
        if config.root_authorities[..index].contains(root) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Clinical causality root list contains duplicate AgentPubKeys".to_string()
            )));
        }
    }
    Ok(config)
}

fn require_root_authority(
    author: &AgentPubKey,
    config: &ClinicalCausalityRootConfig,
) -> ExternResult<ValidateCallbackResult> {
    if !config.root_authorities.contains(author) {
        return invalid(
            "Causal verifier authorization/correction requires a DNA-pinned root authority",
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
            "Clinical causality digest domain mismatch: expected {expected:?}, got {:?}",
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

    fn attestation() -> QualifiedCausalAssessmentAttestationV1 {
        QualifiedCausalAssessmentAttestationV1 {
            schema_version: 1,
            attestation_id: "causal-attestation-a".into(),
            qualified_receipt_digest: digest(DigestDomain::ClinicalCausalQualifiedReceipt, 2),
            qualification_policy_digest: digest(DigestDomain::ClinicalCausalQualificationPolicy, 3),
            conclusion: CausalConclusionV1::Indeterminate,
        }
    }

    fn authorization() -> CausalAssessmentVerifierAuthorization {
        let attestation = attestation();
        CausalAssessmentVerifierAuthorization {
            authorization_id: "causal-auth-a".into(),
            grantee: AgentPubKey::from_raw_36(vec![1; 36]),
            qualified_receipt_digest: attestation.qualified_receipt_digest,
            public_attestation_digest: attestation.digest().unwrap(),
            qualification_policy_digest: attestation.qualification_policy_digest,
            conclusion: attestation.conclusion,
            valid_from: Timestamp::from_micros(10),
            valid_until: Timestamp::from_micros(100),
        }
    }

    #[test]
    fn authorization_rejects_wrong_receipt_domain() {
        let mut value = authorization();
        value.qualified_receipt_digest = stored(DigestDomain::ClinicalCausalAssessment, 2);
        assert!(validate_authorization_shape(&value).is_err());
    }

    #[test]
    fn attestation_id_changes_authorized_identity() {
        let original = attestation();
        let original_digest = original.digest().unwrap();
        let mut changed = original.clone();
        changed.attestation_id = "causal-attestation-b".into();
        assert_ne!(original_digest, changed.digest().unwrap());
    }

    #[test]
    fn correction_other_requires_commitment() {
        let value = CausalAttestationCorrection {
            correction_id: "correction-a".into(),
            attestation_hash: ActionHash::from_raw_36(vec![2; 36]),
            qualified_receipt_digest: digest(DigestDomain::ClinicalCausalQualifiedReceipt, 3),
            reason: CausalAttestationCorrectionReason::Other,
            rationale_commitment: None,
        };
        let result = validate_correction_shape(&value).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn authorization_duration_is_hard_bounded() {
        let value = authorization();
        let config = ClinicalCausalityRootConfig {
            schema_version: 1,
            root_authorities: Vec::new(),
            max_verifier_authorization_duration_micros: 50,
        };
        let result = validate_authorization_window(Timestamp::from_micros(10), &value, &config)
            .unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }
}
