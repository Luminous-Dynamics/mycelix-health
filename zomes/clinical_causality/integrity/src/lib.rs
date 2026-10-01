#![deny(unsafe_code)]
#![allow(clippy::collapsible_match)]
//! DNA-rooted qualified clinical causal-assessment attestations.
//!
//! Detailed causal evidence and qualification receipts remain protected off-DHT.
//! The public DHT stores only a secret-keyed commitment to the private receipt,
//! qualification-policy identity, and conservative conclusion after a DNA-pinned
//! root authorizes one exact verifier to publish one exact projection in a short window.
//! Corrections are append-only; updates/deletes never represent clinical correction.

use hdi::prelude::*;
use mycelix_clinical_causality::CausalConclusionV1;
use mycelix_clinical_integrity::{hash_canonical_bytes, DigestDomain, StoredDigest};

const CONFIG_VERSION: u16 = 1;
const MAX_ID_LEN: usize = 128;
const MAX_ROOT_AUTHORITIES: usize = 64;
const ABSOLUTE_MAX_AUTHORIZATION_DURATION_MICROS: i64 = 900_000_000; // 15 minutes
const ATTESTATION_SCHEMA_TAG: &[u8] = b"mycelix-health/clinical-causal-qualified-attestation-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClinicalCausalityRootConfig {
    pub schema_version: u16,
    pub root_authorities: Vec<AgentPubKey>,
    pub max_verifier_authorization_duration_micros: i64,
}

/// Secret-keyed commitment scheme. The secret key never appears on the DHT.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CausalCommitmentSchemeV1 {
    HmacSha256,
    Blake3Keyed,
}

/// Opaque commitment to protected material.
///
/// Validators establish shape and exact authorization/lineage binding only. The
/// DNA-rooted institutional verifier is responsible for recomputing the commitment
/// from the exact protected artifact before authorization/publication.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OpaqueCausalCommitmentV1 {
    pub scheme: CausalCommitmentSchemeV1,
    pub value: [u8; 32],
}

impl OpaqueCausalCommitmentV1 {
    fn validate(&self, label: &'static str) -> ExternResult<ValidateCallbackResult> {
        if self.value == [0u8; 32] {
            return invalid(format!("{label} commitment cannot be zero"));
        }
        Ok(ValidateCallbackResult::Valid)
    }
}

/// Privacy-minimized public projection of one protected qualified causal receipt.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QualifiedCausalAssessmentAttestationV1 {
    pub schema_version: u16,
    pub attestation_id: String,
    pub qualified_receipt_commitment: OpaqueCausalCommitmentV1,
    pub qualification_policy_digest: StoredDigest,
    pub conclusion: CausalConclusionV1,
}

impl QualifiedCausalAssessmentAttestationV1 {
    pub fn validate(&self) -> ExternResult<ValidateCallbackResult> {
        if self.schema_version != 1 {
            return invalid("Unsupported qualified causal attestation version");
        }
        let id = validate_id("Causal attestation ID", &self.attestation_id)?;
        if !matches!(id, ValidateCallbackResult::Valid) {
            return Ok(id);
        }
        let commitment = self
            .qualified_receipt_commitment
            .validate("Qualified causal receipt")?;
        if !matches!(commitment, ValidateCallbackResult::Valid) {
            return Ok(commitment);
        }
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
/// The raw private receipt digest is intentionally absent from this public entry.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CausalAssessmentVerifierAuthorization {
    pub authorization_id: String,
    pub grantee: AgentPubKey,
    pub qualified_receipt_commitment: OpaqueCausalCommitmentV1,
    pub public_attestation_digest: StoredDigest,
    pub qualification_policy_digest: StoredDigest,
    pub conclusion: CausalConclusionV1,
    pub valid_from: Timestamp,
    pub valid_until: Timestamp,
}

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
    pub qualified_receipt_commitment: OpaqueCausalCommitmentV1,
    pub reason: CausalAttestationCorrectionReason,
    /// Commitment to protected human-readable rationale. Raw text stays private.
    pub rationale_commitment: Option<OpaqueCausalCommitmentV1>,
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
            let root = require_root_authority(action.author(), &config)?;
            if !matches!(root, ValidateCallbackResult::Valid) {
                return Ok(root);
            }
            require_correction_target(&correction)
        }
    }
}

fn validate_authorization_shape(
    authorization: &CausalAssessmentVerifierAuthorization,
) -> ExternResult<ValidateCallbackResult> {
    let id = validate_id("Causal verifier authorization ID", &authorization.authorization_id)?;
    if !matches!(id, ValidateCallbackResult::Valid) {
        return Ok(id);
    }
    let commitment = authorization
        .qualified_receipt_commitment
        .validate("Qualified causal receipt")?;
    if !matches!(commitment, ValidateCallbackResult::Valid) {
        return Ok(commitment);
    }
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

    // Containment for P0 #72: even before exact source-entry-definition proof,
    // a structurally compatible dependency must still be authored by a current
    // DNA-pinned causal root and satisfy the causal authorization shape.
    let config = clinical_causality_root_config()?;
    let root = require_root_authority(record.action().author(), &config)?;
    if !matches!(root, ValidateCallbackResult::Valid) {
        return Ok(root);
    }
    let shape = validate_authorization_shape(&authorization)?;
    if !matches!(shape, ValidateCallbackResult::Valid) {
        return Ok(shape);
    }

    if &authorization.grantee != author {
        return invalid("Causal attestation author is not authorization grantee");
    }
    if action_timestamp < authorization.valid_from || action_timestamp >= authorization.valid_until {
        return invalid("Causal attestation action timestamp is outside authorization validity");
    }
    let attestation = &value.attestation;
    let attestation_digest = attestation.digest()?;
    if attestation_digest != authorization.public_attestation_digest
        || attestation.qualified_receipt_commitment != authorization.qualified_receipt_commitment
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
    let id = validate_id("Causal correction ID", &correction.correction_id)?;
    if !matches!(id, ValidateCallbackResult::Valid) {
        return Ok(id);
    }
    let receipt = correction
        .qualified_receipt_commitment
        .validate("Qualified causal receipt")?;
    if !matches!(receipt, ValidateCallbackResult::Valid) {
        return Ok(receipt);
    }
    if let Some(rationale) = &correction.rationale_commitment {
        let shape = rationale.validate("Causal correction rationale")?;
        if !matches!(shape, ValidateCallbackResult::Valid) {
            return Ok(shape);
        }
    }
    if matches!(correction.reason, CausalAttestationCorrectionReason::Other)
        && correction.rationale_commitment.is_none()
    {
        return invalid("Other causal correction requires a rationale commitment");
    }
    Ok(ValidateCallbackResult::Valid)
}

fn require_correction_target(
    correction: &CausalAttestationCorrection,
) -> ExternResult<ValidateCallbackResult> {
    let record = must_get_valid_record(correction.attestation_hash.clone())?;
    let attestation: QualifiedCausalAssessmentAttestation =
        decode_entry(&record, "qualified causal attestation")?;
    if attestation.attestation.qualified_receipt_commitment
        != correction.qualified_receipt_commitment
    {
        return invalid("Causal correction receipt commitment does not match target attestation");
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
                || correction.qualified_receipt_commitment
                    != attestation.attestation.qualified_receipt_commitment
            {
                return invalid("Causal attestation-correction link crosses receipt commitment lineage");
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
    if config.root_authorities.len() > MAX_ROOT_AUTHORITIES {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Clinical causality root list exceeds maximum of {MAX_ROOT_AUTHORITIES}"
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

fn validate_id(label: &'static str, value: &str) -> ExternResult<ValidateCallbackResult> {
    if value.trim().is_empty() {
        return invalid(format!("{label} is required"));
    }
    if value.len() > MAX_ID_LEN {
        return invalid(format!("{label} exceeds {MAX_ID_LEN} bytes"));
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

    fn commitment(seed: u8) -> OpaqueCausalCommitmentV1 {
        OpaqueCausalCommitmentV1 {
            scheme: CausalCommitmentSchemeV1::Blake3Keyed,
            value: [seed; 32],
        }
    }

    fn attestation() -> QualifiedCausalAssessmentAttestationV1 {
        QualifiedCausalAssessmentAttestationV1 {
            schema_version: 1,
            attestation_id: "causal-attestation-a".into(),
            qualified_receipt_commitment: commitment(2),
            qualification_policy_digest: digest(DigestDomain::ClinicalCausalQualificationPolicy, 3),
            conclusion: CausalConclusionV1::Indeterminate,
        }
    }

    fn authorization() -> CausalAssessmentVerifierAuthorization {
        let attestation = attestation();
        CausalAssessmentVerifierAuthorization {
            authorization_id: "causal-auth-a".into(),
            grantee: AgentPubKey::from_raw_36(vec![1; 36]),
            qualified_receipt_commitment: attestation.qualified_receipt_commitment,
            public_attestation_digest: attestation.digest().unwrap(),
            qualification_policy_digest: attestation.qualification_policy_digest,
            conclusion: attestation.conclusion,
            valid_from: Timestamp::from_micros(10),
            valid_until: Timestamp::from_micros(100),
        }
    }

    #[test]
    fn zero_receipt_commitment_is_rejected() {
        let mut value = attestation();
        value.qualified_receipt_commitment.value = [0u8; 32];
        let result = value.validate().unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn authorization_rejects_wrong_attestation_domain() {
        let mut value = authorization();
        value.public_attestation_digest = stored(DigestDomain::ClinicalCausalAssessment, 2);
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
    fn conclusion_changes_authorized_identity() {
        let original = attestation();
        let original_digest = original.digest().unwrap();
        let mut changed = original.clone();
        changed.conclusion = CausalConclusionV1::EvidenceAgainstRelationship;
        assert_ne!(original_digest, changed.digest().unwrap());
    }

    #[test]
    fn oversized_public_id_is_rejected() {
        let mut value = attestation();
        value.attestation_id = "x".repeat(MAX_ID_LEN + 1);
        let result = value.validate().unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn correction_other_requires_commitment() {
        let value = CausalAttestationCorrection {
            correction_id: "correction-a".into(),
            attestation_hash: ActionHash::from_raw_36(vec![2; 36]),
            qualified_receipt_commitment: commitment(3),
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
