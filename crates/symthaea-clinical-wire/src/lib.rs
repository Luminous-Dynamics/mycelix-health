#![deny(unsafe_code)]
//! Independent verification of Symthaea's clinical inference v1 wire contract.
//!
//! This crate intentionally has **no dependency on Symthaea**. Mycelix therefore
//! does not inherit trust from a shared Rust type or a git revision. It mirrors
//! only the public v1 wire schema, validates it independently, requires the exact
//! canonical representation, and reproduces the Symthaea wire digest framing.
//!
//! A successful parse proves structural/wire compatibility only. It does not
//! establish scientific validity, clinical effectiveness, trusted model
//! execution, distribution validity, or clinical presentation authority.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const SYMTHAEA_CLINICAL_INFERENCE_SCHEMA_VERSION: u16 = 1;
pub const SYMTHAEA_CLINICAL_INFERENCE_WIRE_IDENTITY_VERSION: u16 = 1;

const DERIVE_KEY_CONTEXT: &str = "symthaea.clinical.inference-envelope-wire.v1";
const SCHEMA_TAG: &[u8] = b"symthaea/clinical-inference-envelope/v1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SymthaeaDigestAlgorithmV1 {
    Blake3_256,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct SymthaeaDigestV1 {
    pub algorithm: SymthaeaDigestAlgorithmV1,
    pub value: [u8; 32],
}

impl SymthaeaDigestV1 {
    fn validate(&self) -> Result<(), SymthaeaClinicalWireError> {
        if self.value == [0u8; 32] {
            return Err(SymthaeaClinicalWireError::ZeroDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SymthaeaArtifactIdentityV1 {
    pub name: String,
    pub version: String,
    pub digest: SymthaeaDigestV1,
}

impl SymthaeaArtifactIdentityV1 {
    fn validate(&self) -> Result<(), SymthaeaClinicalWireError> {
        require_nonempty(&self.name, "artifact.name")?;
        require_nonempty(&self.version, "artifact.version")?;
        self.digest.validate()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SymthaeaModelIdentityV1 {
    pub model: SymthaeaArtifactIdentityV1,
    pub input_schema_digest: SymthaeaDigestV1,
    pub output_schema_digest: SymthaeaDigestV1,
    pub training_lineage_digest: Option<SymthaeaDigestV1>,
    pub evaluation_lineage_digest: Option<SymthaeaDigestV1>,
    pub calibration_evidence_digest: Option<SymthaeaDigestV1>,
}

impl SymthaeaModelIdentityV1 {
    fn validate(&self) -> Result<(), SymthaeaClinicalWireError> {
        self.model.validate()?;
        self.input_schema_digest.validate()?;
        self.output_schema_digest.validate()?;
        for digest in [
            self.training_lineage_digest,
            self.evaluation_lineage_digest,
            self.calibration_evidence_digest,
        ]
        .into_iter()
        .flatten()
        {
            digest.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SymthaeaExecutionIdentityV1 {
    pub engine: SymthaeaArtifactIdentityV1,
    pub model: SymthaeaModelIdentityV1,
    pub runtime_digest: SymthaeaDigestV1,
    pub configuration_digest: SymthaeaDigestV1,
    pub input_evidence_digests: Vec<SymthaeaDigestV1>,
    pub operation: String,
    pub executed_at_micros: i64,
    pub execution_nonce: [u8; 16],
}

impl SymthaeaExecutionIdentityV1 {
    fn validate(&self) -> Result<(), SymthaeaClinicalWireError> {
        self.engine.validate()?;
        self.model.validate()?;
        self.runtime_digest.validate()?;
        self.configuration_digest.validate()?;
        require_nonempty(&self.operation, "execution.operation")?;
        if self.execution_nonce == [0u8; 16] {
            return Err(SymthaeaClinicalWireError::ZeroExecutionNonce);
        }
        if self.input_evidence_digests.is_empty() {
            return Err(SymthaeaClinicalWireError::MissingExecutionInputs);
        }
        let mut seen = HashSet::new();
        for digest in &self.input_evidence_digests {
            digest.validate()?;
            if !seen.insert(*digest) {
                return Err(SymthaeaClinicalWireError::DuplicateExecutionInput);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SymthaeaClinicalClaimKindV1 {
    CandidateSignal,
    Association,
    Prediction,
    RiskEstimate,
    CausalHypothesis,
    CausalEffectEstimate,
    DiagnosticSupport,
    TreatmentSupport,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SymthaeaClinicalEvidenceStageV1 {
    MechanisticHypothesis,
    SyntheticDemonstration,
    RetrospectiveInternal,
    RetrospectiveExternal,
    ProspectiveShadow,
    ProspectiveClinicalStudy,
    ReplicatedClinicalEvidence,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SymthaeaClinicalApplicabilityV1 {
    Unestablished,
    EvaluatedCohortOnly,
    DefinedTargetPopulation,
    ValidatedTargetPopulation,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SymthaeaClinicalIntendedUseV1 {
    ResearchOnly,
    ClinicalDecisionSupport,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct SymthaeaClinicalClaimSemanticsV1 {
    pub schema_version: u16,
    pub claim_kind: SymthaeaClinicalClaimKindV1,
    pub evidence_stage: SymthaeaClinicalEvidenceStageV1,
    pub applicability: SymthaeaClinicalApplicabilityV1,
    pub intended_use: SymthaeaClinicalIntendedUseV1,
}

impl SymthaeaClinicalClaimSemanticsV1 {
    fn validate(&self) -> Result<(), SymthaeaClinicalWireError> {
        if self.schema_version != 1 {
            return Err(SymthaeaClinicalWireError::UnsupportedClaimSchemaVersion(
                self.schema_version,
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SymthaeaSubjectBindingV1 {
    pub namespace: String,
    pub subject_id: String,
    pub binding_evidence_digest: SymthaeaDigestV1,
}

impl SymthaeaSubjectBindingV1 {
    fn validate(&self) -> Result<(), SymthaeaClinicalWireError> {
        require_nonempty(&self.namespace, "subject.namespace")?;
        require_nonempty(&self.subject_id, "subject.subject_id")?;
        self.binding_evidence_digest.validate()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SymthaeaClinicalEvidenceRoleV1 {
    Supports,
    Opposes,
    Context,
    Contraindication,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SymthaeaClinicalEvidenceRefV1 {
    pub evidence_id: String,
    pub digest: SymthaeaDigestV1,
    pub role: SymthaeaClinicalEvidenceRoleV1,
}

impl SymthaeaClinicalEvidenceRefV1 {
    fn validate(&self) -> Result<(), SymthaeaClinicalWireError> {
        require_nonempty(&self.evidence_id, "evidence.evidence_id")?;
        self.digest.validate()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SymthaeaCalibrationStatusV1 {
    NotAssessed,
    Uncalibrated,
    Calibrated,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SymthaeaClinicalUncertaintyV1 {
    pub epistemic: Option<f64>,
    pub aleatoric: Option<f64>,
    pub calibrated_probability: Option<f64>,
    pub calibration_status: SymthaeaCalibrationStatusV1,
    pub calibration_evidence_digest: Option<SymthaeaDigestV1>,
}

impl SymthaeaClinicalUncertaintyV1 {
    fn validate(&self) -> Result<(), SymthaeaClinicalWireError> {
        for value in [self.epistemic, self.aleatoric].into_iter().flatten() {
            if !value.is_finite() || value < 0.0 {
                return Err(SymthaeaClinicalWireError::InvalidUncertainty);
            }
        }
        if let Some(probability) = self.calibrated_probability {
            if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
                return Err(SymthaeaClinicalWireError::InvalidCalibratedProbability);
            }
        }
        if let Some(digest) = self.calibration_evidence_digest {
            digest.validate()?;
        }
        if self.calibration_status == SymthaeaCalibrationStatusV1::Calibrated
            && (self.calibrated_probability.is_none()
                || self.calibration_evidence_digest.is_none())
        {
            return Err(SymthaeaClinicalWireError::IncompleteCalibrationEvidence);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SymthaeaDistributionStatusV1 {
    Unknown,
    InDistribution,
    OutOfDistribution,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SymthaeaDistributionAssessmentV1 {
    pub status: SymthaeaDistributionStatusV1,
    pub detector_evidence_digest: Option<SymthaeaDigestV1>,
}

impl SymthaeaDistributionAssessmentV1 {
    fn validate(&self) -> Result<(), SymthaeaClinicalWireError> {
        if let Some(digest) = self.detector_evidence_digest {
            digest.validate()?;
        }
        if self.status != SymthaeaDistributionStatusV1::Unknown
            && self.detector_evidence_digest.is_none()
        {
            return Err(SymthaeaClinicalWireError::MissingDistributionEvidence);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SymthaeaMissingEvidenceCriticalityV1 {
    Informational,
    Important,
    Critical,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SymthaeaMissingClinicalEvidenceV1 {
    pub requirement_id: String,
    pub description: String,
    pub criticality: SymthaeaMissingEvidenceCriticalityV1,
}

impl SymthaeaMissingClinicalEvidenceV1 {
    fn validate(&self) -> Result<(), SymthaeaClinicalWireError> {
        require_nonempty(&self.requirement_id, "missing_evidence.requirement_id")?;
        require_nonempty(&self.description, "missing_evidence.description")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SymthaeaAlternativeClinicalHypothesisV1 {
    pub statement: String,
    pub semantics: SymthaeaClinicalClaimSemanticsV1,
    pub rationale: String,
}

impl SymthaeaAlternativeClinicalHypothesisV1 {
    fn validate(&self) -> Result<(), SymthaeaClinicalWireError> {
        require_nonempty(&self.statement, "alternative.statement")?;
        require_nonempty(&self.rationale, "alternative.rationale")?;
        self.semantics.validate()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SymthaeaClinicalInferenceEnvelopeV1 {
    pub schema_version: u16,
    pub semantics: SymthaeaClinicalClaimSemanticsV1,
    pub subject: Option<SymthaeaSubjectBindingV1>,
    pub statement: String,
    pub evidence: Vec<SymthaeaClinicalEvidenceRefV1>,
    pub alternatives: Vec<SymthaeaAlternativeClinicalHypothesisV1>,
    pub missing_evidence: Vec<SymthaeaMissingClinicalEvidenceV1>,
    pub uncertainty: SymthaeaClinicalUncertaintyV1,
    pub distribution: SymthaeaDistributionAssessmentV1,
    pub execution: SymthaeaExecutionIdentityV1,
    pub generated_at_micros: i64,
}

impl SymthaeaClinicalInferenceEnvelopeV1 {
    pub fn validate(&self) -> Result<(), SymthaeaClinicalWireError> {
        if self.schema_version != SYMTHAEA_CLINICAL_INFERENCE_SCHEMA_VERSION {
            return Err(SymthaeaClinicalWireError::UnsupportedEnvelopeSchemaVersion(
                self.schema_version,
            ));
        }
        self.semantics.validate()?;
        require_nonempty(&self.statement, "statement")?;
        if self.semantics.intended_use == SymthaeaClinicalIntendedUseV1::ClinicalDecisionSupport
            && self.subject.is_none()
        {
            return Err(SymthaeaClinicalWireError::ClinicalUseWithoutSubjectBinding);
        }
        if let Some(subject) = &self.subject {
            subject.validate()?;
        }
        if self.evidence.is_empty() {
            return Err(SymthaeaClinicalWireError::MissingEvidence);
        }
        let mut evidence_ids = HashSet::new();
        for evidence in &self.evidence {
            evidence.validate()?;
            if !evidence_ids.insert(evidence.evidence_id.as_str()) {
                return Err(SymthaeaClinicalWireError::DuplicateEvidenceId);
            }
        }
        for alternative in &self.alternatives {
            alternative.validate()?;
        }
        for missing in &self.missing_evidence {
            missing.validate()?;
        }
        self.uncertainty.validate()?;
        self.distribution.validate()?;
        self.execution.validate()?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymthaeaClinicalInferenceWireDigestV1([u8; 32]);

impl SymthaeaClinicalInferenceWireDigestV1 {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Independently parse and validate the exact canonical Symthaea v1 wire bytes.
pub fn parse_canonical_symthaea_clinical_inference_v1(
    bytes: &[u8],
) -> Result<SymthaeaClinicalInferenceEnvelopeV1, SymthaeaClinicalWireError> {
    let envelope: SymthaeaClinicalInferenceEnvelopeV1 = serde_json::from_slice(bytes)
        .map_err(|_| SymthaeaClinicalWireError::DeserializationFailure)?;
    envelope.validate()?;
    let canonical = serde_json::to_vec(&envelope)
        .map_err(|_| SymthaeaClinicalWireError::SerializationFailure)?;
    if canonical != bytes {
        return Err(SymthaeaClinicalWireError::NonCanonicalEncoding);
    }
    Ok(envelope)
}

/// Independently reproduce Symthaea's v1 domain-separated wire digest after
/// canonical parsing and structural validation.
pub fn verify_symthaea_clinical_inference_wire_digest_v1(
    bytes: &[u8],
) -> Result<SymthaeaClinicalInferenceWireDigestV1, SymthaeaClinicalWireError> {
    let _ = parse_canonical_symthaea_clinical_inference_v1(bytes)?;
    let schema_len = u16::try_from(SCHEMA_TAG.len())
        .map_err(|_| SymthaeaClinicalWireError::SerializationFailure)?;
    let byte_len = u64::try_from(bytes.len())
        .map_err(|_| SymthaeaClinicalWireError::SerializationFailure)?;

    let mut hasher = blake3::Hasher::new_derive_key(DERIVE_KEY_CONTEXT);
    hasher.update(&SYMTHAEA_CLINICAL_INFERENCE_WIRE_IDENTITY_VERSION.to_be_bytes());
    hasher.update(&schema_len.to_be_bytes());
    hasher.update(SCHEMA_TAG);
    hasher.update(&byte_len.to_be_bytes());
    hasher.update(bytes);
    Ok(SymthaeaClinicalInferenceWireDigestV1(
        *hasher.finalize().as_bytes(),
    ))
}

fn require_nonempty(value: &str, field: &'static str) -> Result<(), SymthaeaClinicalWireError> {
    if value.trim().is_empty() {
        return Err(SymthaeaClinicalWireError::MissingField(field));
    }
    Ok(())
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SymthaeaClinicalWireError {
    #[error("unsupported Symthaea clinical inference envelope schema version {0}")]
    UnsupportedEnvelopeSchemaVersion(u16),
    #[error("unsupported Symthaea clinical claim schema version {0}")]
    UnsupportedClaimSchemaVersion(u16),
    #[error("required field is empty: {0}")]
    MissingField(&'static str),
    #[error("digest cannot be zero")]
    ZeroDigest,
    #[error("execution nonce cannot be zero")]
    ZeroExecutionNonce,
    #[error("execution inputs are required")]
    MissingExecutionInputs,
    #[error("duplicate execution input digest")]
    DuplicateExecutionInput,
    #[error("clinical decision support requires an explicit subject binding")]
    ClinicalUseWithoutSubjectBinding,
    #[error("at least one evidence reference is required")]
    MissingEvidence,
    #[error("duplicate evidence id")]
    DuplicateEvidenceId,
    #[error("uncertainty value is invalid")]
    InvalidUncertainty,
    #[error("calibrated probability must be finite and in [0,1]")]
    InvalidCalibratedProbability,
    #[error("calibrated status requires probability and calibration evidence")]
    IncompleteCalibrationEvidence,
    #[error("known distribution status requires detector evidence")]
    MissingDistributionEvidence,
    #[error("wire bytes could not be deserialized under the strict v1 schema")]
    DeserializationFailure,
    #[error("wire bytes could not be serialized")]
    SerializationFailure,
    #[error("wire representation is not the exact canonical v1 encoding")]
    NonCanonicalEncoding,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> SymthaeaDigestV1 {
        SymthaeaDigestV1 {
            algorithm: SymthaeaDigestAlgorithmV1::Blake3_256,
            value: [byte; 32],
        }
    }

    fn artifact(name: &str, version: &str, byte: u8) -> SymthaeaArtifactIdentityV1 {
        SymthaeaArtifactIdentityV1 {
            name: name.into(),
            version: version.into(),
            digest: digest(byte),
        }
    }

    fn envelope() -> SymthaeaClinicalInferenceEnvelopeV1 {
        SymthaeaClinicalInferenceEnvelopeV1 {
            schema_version: 1,
            semantics: SymthaeaClinicalClaimSemanticsV1 {
                schema_version: 1,
                claim_kind: SymthaeaClinicalClaimKindV1::Prediction,
                evidence_stage: SymthaeaClinicalEvidenceStageV1::RetrospectiveExternal,
                applicability: SymthaeaClinicalApplicabilityV1::DefinedTargetPopulation,
                intended_use: SymthaeaClinicalIntendedUseV1::ClinicalDecisionSupport,
            },
            subject: Some(SymthaeaSubjectBindingV1 {
                namespace: "fhir/Patient".into(),
                subject_id: "patient-a".into(),
                binding_evidence_digest: digest(10),
            }),
            statement: "Candidate risk prediction".into(),
            evidence: vec![SymthaeaClinicalEvidenceRefV1 {
                evidence_id: "fact-1".into(),
                digest: digest(11),
                role: SymthaeaClinicalEvidenceRoleV1::Supports,
            }],
            alternatives: vec![SymthaeaAlternativeClinicalHypothesisV1 {
                statement: "Alternative explanation".into(),
                semantics: SymthaeaClinicalClaimSemanticsV1 {
                    schema_version: 1,
                    claim_kind: SymthaeaClinicalClaimKindV1::CausalHypothesis,
                    evidence_stage: SymthaeaClinicalEvidenceStageV1::MechanisticHypothesis,
                    applicability: SymthaeaClinicalApplicabilityV1::Unestablished,
                    intended_use: SymthaeaClinicalIntendedUseV1::ResearchOnly,
                },
                rationale: "Preserve competing hypothesis".into(),
            }],
            missing_evidence: vec![],
            uncertainty: SymthaeaClinicalUncertaintyV1 {
                epistemic: Some(0.2),
                aleatoric: Some(0.1),
                calibrated_probability: Some(0.7),
                calibration_status: SymthaeaCalibrationStatusV1::Calibrated,
                calibration_evidence_digest: Some(digest(12)),
            },
            distribution: SymthaeaDistributionAssessmentV1 {
                status: SymthaeaDistributionStatusV1::InDistribution,
                detector_evidence_digest: Some(digest(13)),
            },
            execution: SymthaeaExecutionIdentityV1 {
                engine: artifact("symthaea", "0.1.0", 1),
                model: SymthaeaModelIdentityV1 {
                    model: artifact("clinical-model", "1.0.0", 2),
                    input_schema_digest: digest(3),
                    output_schema_digest: digest(4),
                    training_lineage_digest: Some(digest(5)),
                    evaluation_lineage_digest: Some(digest(6)),
                    calibration_evidence_digest: Some(digest(12)),
                },
                runtime_digest: digest(7),
                configuration_digest: digest(8),
                input_evidence_digests: vec![digest(11)],
                operation: "evaluate".into(),
                executed_at_micros: 1_000,
                execution_nonce: [1u8; 16],
            },
            generated_at_micros: 1_001,
        }
    }

    fn canonical_bytes(envelope: &SymthaeaClinicalInferenceEnvelopeV1) -> Vec<u8> {
        serde_json::to_vec(envelope).unwrap()
    }

    #[test]
    fn canonical_v1_parses_independently() {
        let expected = envelope();
        let bytes = canonical_bytes(&expected);
        let parsed = parse_canonical_symthaea_clinical_inference_v1(&bytes).unwrap();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn unknown_field_fails_closed() {
        let bytes = canonical_bytes(&envelope());
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("hidden_authority".into(), serde_json::json!(true));
        let mutated = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            parse_canonical_symthaea_clinical_inference_v1(&mutated),
            Err(SymthaeaClinicalWireError::DeserializationFailure)
        );
    }

    #[test]
    fn whitespace_variant_is_not_canonical() {
        let bytes = canonical_bytes(&envelope());
        let mut mutated = vec![b' '];
        mutated.extend_from_slice(&bytes);
        assert_eq!(
            parse_canonical_symthaea_clinical_inference_v1(&mutated),
            Err(SymthaeaClinicalWireError::NonCanonicalEncoding)
        );
    }

    #[test]
    fn digest_is_deterministic_and_model_bound() {
        let a = envelope();
        let bytes_a = canonical_bytes(&a);
        let digest_a = verify_symthaea_clinical_inference_wire_digest_v1(&bytes_a).unwrap();
        assert_eq!(
            digest_a,
            verify_symthaea_clinical_inference_wire_digest_v1(&bytes_a).unwrap()
        );

        let mut b = a;
        b.execution.model.model.version = "2.0.0".into();
        let bytes_b = canonical_bytes(&b);
        let digest_b = verify_symthaea_clinical_inference_wire_digest_v1(&bytes_b).unwrap();
        assert_ne!(digest_a, digest_b);
    }

    #[test]
    fn zero_digest_fails_closed() {
        let mut value = envelope();
        value.execution.runtime_digest.value = [0u8; 32];
        let bytes = canonical_bytes(&value);
        assert_eq!(
            parse_canonical_symthaea_clinical_inference_v1(&bytes),
            Err(SymthaeaClinicalWireError::ZeroDigest)
        );
    }

    #[test]
    fn clinical_use_without_subject_fails_closed() {
        let mut value = envelope();
        value.subject = None;
        let bytes = canonical_bytes(&value);
        assert_eq!(
            parse_canonical_symthaea_clinical_inference_v1(&bytes),
            Err(SymthaeaClinicalWireError::ClinicalUseWithoutSubjectBinding)
        );
    }

    #[test]
    fn calibrated_status_requires_calibration_evidence() {
        let mut value = envelope();
        value.uncertainty.calibration_evidence_digest = None;
        let bytes = canonical_bytes(&value);
        assert_eq!(
            parse_canonical_symthaea_clinical_inference_v1(&bytes),
            Err(SymthaeaClinicalWireError::IncompleteCalibrationEvidence)
        );
    }
}
