#![deny(unsafe_code)]
//! Independent verifier for Symthaea clinical inference binary wire v2.
//!
//! This crate intentionally has **no dependency on Symthaea**. It mirrors only the
//! published binary contract so cross-repository compatibility must be reproduced,
//! not inherited through shared implementation code.
//!
//! Successful verification proves only that exact bytes satisfy the independently
//! implemented wire/semantic contract. It grants no clinical authority.

use std::collections::{HashMap, HashSet};

pub const SYMTHAEA_CLINICAL_WIRE_V2_VERSION: u16 = 1;
pub const SYMTHAEA_CLINICAL_ENVELOPE_V2_VERSION: u16 = 2;
pub const SYMTHAEA_EVIDENCE_IDENTITY_VERSION: u16 = 1;

const MAGIC: &[u8; 8] = b"SYMCLN2\0";
const DERIVE_KEY_CONTEXT: &str = "symthaea.clinical.inference-wire-v2.v1";
const DOMAIN_TAG: &[u8] = b"symthaea/clinical-inference-wire/v2/v1";
const MAX_WIRE_BYTES: usize = 4 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 256 * 1024;
const MAX_ID_BYTES: usize = 4 * 1024;
const MAX_VECTOR_ITEMS: usize = 4096;
const MAX_SEMANTIC_NAMESPACE: usize = 256;
const MAX_SEMANTIC_ARTIFACT_ID: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SymthaeaDigestAlgorithmV2 {
    Blake3_256,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymthaeaDigestV2 {
    pub algorithm: SymthaeaDigestAlgorithmV2,
    pub value: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SymthaeaEvidenceIdentityV2 {
    pub identity_version: u16,
    pub namespace: String,
    pub artifact_id: String,
    pub digest: SymthaeaDigestV2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SymthaeaClaimKindV2 {
    CandidateSignal,
    Association,
    Prediction,
    RiskEstimate,
    CausalHypothesis,
    CausalEffectEstimate,
    DiagnosticSupport,
    TreatmentSupport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SymthaeaEvidenceStageV2 {
    MechanisticHypothesis,
    SyntheticDemonstration,
    RetrospectiveInternal,
    RetrospectiveExternal,
    ProspectiveShadow,
    ProspectiveClinicalStudy,
    ReplicatedClinicalEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SymthaeaApplicabilityV2 {
    Unestablished,
    EvaluatedCohortOnly,
    DefinedTargetPopulation,
    ValidatedTargetPopulation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SymthaeaIntendedUseV2 {
    ResearchOnly,
    ClinicalDecisionSupport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymthaeaClaimSemanticsV2 {
    pub schema_version: u16,
    pub claim_kind: SymthaeaClaimKindV2,
    pub evidence_stage: SymthaeaEvidenceStageV2,
    pub applicability: SymthaeaApplicabilityV2,
    pub intended_use: SymthaeaIntendedUseV2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymthaeaArtifactIdentityV2 {
    pub name: String,
    pub version: String,
    pub digest: SymthaeaDigestV2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymthaeaModelIdentityV2 {
    pub model: SymthaeaArtifactIdentityV2,
    pub input_schema_digest: SymthaeaDigestV2,
    pub output_schema_digest: SymthaeaDigestV2,
    pub training_lineage: Option<SymthaeaEvidenceIdentityV2>,
    pub evaluation_lineage: Option<SymthaeaEvidenceIdentityV2>,
    pub calibration_evidence: Option<SymthaeaEvidenceIdentityV2>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymthaeaSubjectBindingV2 {
    pub subject_namespace: String,
    pub subject_id: String,
    pub binding_evidence: SymthaeaEvidenceIdentityV2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymthaeaEvidenceRoleV2 {
    Supports,
    Opposes,
    Context,
    Contraindication,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymthaeaEvidenceRefV2 {
    pub identity: SymthaeaEvidenceIdentityV2,
    pub role: SymthaeaEvidenceRoleV2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymthaeaAlternativeV2 {
    pub statement: String,
    pub semantics: SymthaeaClaimSemanticsV2,
    pub rationale: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymthaeaMissingCriticalityV2 {
    Informational,
    Important,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymthaeaMissingEvidenceV2 {
    pub requirement_id: String,
    pub description: String,
    pub criticality: SymthaeaMissingCriticalityV2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymthaeaCalibrationStatusV2 {
    NotAssessed,
    Uncalibrated,
    Calibrated,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SymthaeaUncertaintyV2 {
    pub epistemic: Option<f64>,
    pub aleatoric: Option<f64>,
    pub calibrated_probability: Option<f64>,
    pub calibration_status: SymthaeaCalibrationStatusV2,
    pub calibration_evidence: Option<SymthaeaEvidenceIdentityV2>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymthaeaDistributionStatusV2 {
    Unknown,
    InDistribution,
    OutOfDistribution,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymthaeaDistributionAssessmentV2 {
    pub status: SymthaeaDistributionStatusV2,
    pub detector_evidence: Option<SymthaeaEvidenceIdentityV2>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymthaeaExecutionIdentityV2 {
    pub engine: SymthaeaArtifactIdentityV2,
    pub model: SymthaeaModelIdentityV2,
    pub runtime_digest: SymthaeaDigestV2,
    pub configuration_digest: SymthaeaDigestV2,
    pub input_evidence: Vec<SymthaeaEvidenceIdentityV2>,
    pub operation: String,
    pub executed_at_micros: i64,
    pub execution_nonce: [u8; 16],
}

#[derive(Clone, Debug, PartialEq)]
pub struct SymthaeaClinicalInferenceV2 {
    pub schema_version: u16,
    pub semantics: SymthaeaClaimSemanticsV2,
    pub subject: Option<SymthaeaSubjectBindingV2>,
    pub statement: String,
    pub evidence: Vec<SymthaeaEvidenceRefV2>,
    pub alternatives: Vec<SymthaeaAlternativeV2>,
    pub missing_evidence: Vec<SymthaeaMissingEvidenceV2>,
    pub uncertainty: SymthaeaUncertaintyV2,
    pub distribution: SymthaeaDistributionAssessmentV2,
    pub execution: SymthaeaExecutionIdentityV2,
    pub generated_at_micros: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VerifiedSymthaeaWireDigestV2([u8; 32]);

impl VerifiedSymthaeaWireDigestV2 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Independent parse + semantic verification of the published Symthaea binary contract.
pub fn verify_symthaea_clinical_wire_v2(
    bytes: &[u8],
) -> Result<SymthaeaClinicalInferenceV2, SymthaeaWireV2Error> {
    if bytes.is_empty() || bytes.len() > MAX_WIRE_BYTES {
        return Err(SymthaeaWireV2Error::InvalidWireLength);
    }
    let mut reader = Reader::new(bytes);
    if reader.take_exact(MAGIC.len())? != MAGIC {
        return Err(SymthaeaWireV2Error::WrongMagic);
    }
    let wire_version = reader.u16()?;
    if wire_version != SYMTHAEA_CLINICAL_WIRE_V2_VERSION {
        return Err(SymthaeaWireV2Error::UnsupportedWireVersion(wire_version));
    }
    let envelope = decode_envelope(&mut reader)?;
    if !reader.is_finished() {
        return Err(SymthaeaWireV2Error::TrailingBytes);
    }
    validate_envelope(&envelope)?;
    Ok(envelope)
}

/// Verify exact wire semantics, then compute the published Symthaea v2 wire identity
/// over the exact accepted bytes. This digest is Symthaea-wire-specific and must not
/// be reinterpreted as a Mycelix clinical artifact digest.
pub fn verify_symthaea_clinical_wire_v2_digest(
    bytes: &[u8],
) -> Result<VerifiedSymthaeaWireDigestV2, SymthaeaWireV2Error> {
    verify_symthaea_clinical_wire_v2(bytes)?;
    let mut hasher = blake3::Hasher::new_derive_key(DERIVE_KEY_CONTEXT);
    hasher.update(&SYMTHAEA_CLINICAL_WIRE_V2_VERSION.to_be_bytes());
    hasher.update(&(DOMAIN_TAG.len() as u16).to_be_bytes());
    hasher.update(DOMAIN_TAG);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    Ok(VerifiedSymthaeaWireDigestV2(*hasher.finalize().as_bytes()))
}

fn validate_envelope(value: &SymthaeaClinicalInferenceV2) -> Result<(), SymthaeaWireV2Error> {
    if value.schema_version != SYMTHAEA_CLINICAL_ENVELOPE_V2_VERSION {
        return Err(SymthaeaWireV2Error::UnsupportedEnvelopeVersion(value.schema_version));
    }
    validate_semantics(&value.semantics)?;
    validate_text(&value.statement, MAX_TEXT_BYTES)?;
    if value.semantics.intended_use == SymthaeaIntendedUseV2::ClinicalDecisionSupport
        && value.subject.is_none()
    {
        return Err(SymthaeaWireV2Error::ClinicalUseWithoutSubject);
    }
    validate_execution(&value.execution)?;
    if let Some(subject) = &value.subject {
        validate_identifier(&subject.subject_namespace, MAX_SEMANTIC_NAMESPACE)?;
        validate_identifier(&subject.subject_id, MAX_SEMANTIC_ARTIFACT_ID)?;
        validate_evidence_identity(&subject.binding_evidence)?;
        if !value.execution.input_evidence.contains(&subject.binding_evidence) {
            return Err(SymthaeaWireV2Error::SubjectBindingNotExecutionBound);
        }
    }
    if value.evidence.is_empty() {
        return Err(SymthaeaWireV2Error::MissingEvidence);
    }
    let mut named = HashMap::new();
    for evidence in &value.evidence {
        validate_evidence_identity(&evidence.identity)?;
        let key = (
            evidence.identity.namespace.as_str(),
            evidence.identity.artifact_id.as_str(),
        );
        if named.insert(key, evidence.identity.digest).is_some() {
            return Err(SymthaeaWireV2Error::DuplicateEvidenceIdentity);
        }
        if !value.execution.input_evidence.contains(&evidence.identity) {
            return Err(SymthaeaWireV2Error::EvidenceNotExecutionBound);
        }
    }
    for alternative in &value.alternatives {
        validate_text(&alternative.statement, MAX_TEXT_BYTES)?;
        validate_semantics(&alternative.semantics)?;
        validate_text(&alternative.rationale, MAX_TEXT_BYTES)?;
    }
    for missing in &value.missing_evidence {
        validate_identifier(&missing.requirement_id, MAX_SEMANTIC_ARTIFACT_ID)?;
        validate_text(&missing.description, MAX_TEXT_BYTES)?;
    }
    validate_uncertainty(&value.uncertainty, &value.execution.model)?;
    validate_distribution(&value.distribution)?;
    Ok(())
}

fn validate_semantics(value: &SymthaeaClaimSemanticsV2) -> Result<(), SymthaeaWireV2Error> {
    if value.schema_version != 1 {
        return Err(SymthaeaWireV2Error::UnsupportedClaimVocabularyVersion(
            value.schema_version,
        ));
    }
    Ok(())
}

fn validate_execution(value: &SymthaeaExecutionIdentityV2) -> Result<(), SymthaeaWireV2Error> {
    validate_artifact(&value.engine)?;
    validate_model(&value.model)?;
    validate_digest(&value.runtime_digest)?;
    validate_digest(&value.configuration_digest)?;
    validate_identifier(&value.operation, MAX_SEMANTIC_ARTIFACT_ID)?;
    if value.execution_nonce == [0u8; 16] {
        return Err(SymthaeaWireV2Error::ZeroExecutionNonce);
    }
    if value.input_evidence.is_empty() {
        return Err(SymthaeaWireV2Error::MissingExecutionInputs);
    }
    let mut seen = HashSet::new();
    for input in &value.input_evidence {
        validate_evidence_identity(input)?;
        if !seen.insert(input.clone()) {
            return Err(SymthaeaWireV2Error::DuplicateExecutionInput);
        }
    }
    Ok(())
}

fn validate_model(value: &SymthaeaModelIdentityV2) -> Result<(), SymthaeaWireV2Error> {
    validate_artifact(&value.model)?;
    validate_digest(&value.input_schema_digest)?;
    validate_digest(&value.output_schema_digest)?;
    for identity in [
        value.training_lineage.as_ref(),
        value.evaluation_lineage.as_ref(),
        value.calibration_evidence.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        validate_evidence_identity(identity)?;
    }
    Ok(())
}

fn validate_uncertainty(
    value: &SymthaeaUncertaintyV2,
    model: &SymthaeaModelIdentityV2,
) -> Result<(), SymthaeaWireV2Error> {
    for number in [value.epistemic, value.aleatoric].into_iter().flatten() {
        if !number.is_finite() || number < 0.0 {
            return Err(SymthaeaWireV2Error::InvalidUncertainty);
        }
    }
    if let Some(probability) = value.calibrated_probability {
        if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
            return Err(SymthaeaWireV2Error::InvalidCalibratedProbability);
        }
    }
    if let Some(identity) = &value.calibration_evidence {
        validate_evidence_identity(identity)?;
    }
    if value.calibration_status == SymthaeaCalibrationStatusV2::Calibrated {
        if value.calibrated_probability.is_none() || value.calibration_evidence.is_none() {
            return Err(SymthaeaWireV2Error::IncompleteCalibrationEvidence);
        }
        let model_evidence = model
            .calibration_evidence
            .as_ref()
            .ok_or(SymthaeaWireV2Error::ModelCalibrationEvidenceRequired)?;
        if value.calibration_evidence.as_ref() != Some(model_evidence) {
            return Err(SymthaeaWireV2Error::CalibrationEvidenceMismatch);
        }
    }
    Ok(())
}

fn validate_distribution(value: &SymthaeaDistributionAssessmentV2) -> Result<(), SymthaeaWireV2Error> {
    if let Some(identity) = &value.detector_evidence {
        validate_evidence_identity(identity)?;
    }
    if value.status != SymthaeaDistributionStatusV2::Unknown && value.detector_evidence.is_none() {
        return Err(SymthaeaWireV2Error::MissingDistributionEvidence);
    }
    Ok(())
}

fn validate_artifact(value: &SymthaeaArtifactIdentityV2) -> Result<(), SymthaeaWireV2Error> {
    validate_identifier(&value.name, MAX_SEMANTIC_ARTIFACT_ID)?;
    validate_identifier(&value.version, MAX_SEMANTIC_ARTIFACT_ID)?;
    validate_digest(&value.digest)
}

fn validate_evidence_identity(value: &SymthaeaEvidenceIdentityV2) -> Result<(), SymthaeaWireV2Error> {
    if value.identity_version != SYMTHAEA_EVIDENCE_IDENTITY_VERSION {
        return Err(SymthaeaWireV2Error::UnsupportedEvidenceIdentityVersion(
            value.identity_version,
        ));
    }
    validate_identifier(&value.namespace, MAX_SEMANTIC_NAMESPACE)?;
    if value.namespace.chars().any(char::is_whitespace) {
        return Err(SymthaeaWireV2Error::InvalidNamespace);
    }
    validate_identifier(&value.artifact_id, MAX_SEMANTIC_ARTIFACT_ID)?;
    validate_digest(&value.digest)
}

fn validate_digest(value: &SymthaeaDigestV2) -> Result<(), SymthaeaWireV2Error> {
    if value.value == [0u8; 32] {
        return Err(SymthaeaWireV2Error::ZeroDigest);
    }
    Ok(())
}

fn validate_identifier(value: &str, max: usize) -> Result<(), SymthaeaWireV2Error> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(SymthaeaWireV2Error::InvalidIdentifier);
    }
    if value.len() > max {
        return Err(SymthaeaWireV2Error::FieldTooLarge);
    }
    Ok(())
}

fn validate_text(value: &str, max: usize) -> Result<(), SymthaeaWireV2Error> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(SymthaeaWireV2Error::InvalidText);
    }
    if value.len() > max {
        return Err(SymthaeaWireV2Error::FieldTooLarge);
    }
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn is_finished(&self) -> bool {
        self.offset == self.bytes.len()
    }

    fn take_exact(&mut self, len: usize) -> Result<&'a [u8], SymthaeaWireV2Error> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(SymthaeaWireV2Error::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(SymthaeaWireV2Error::TruncatedWire);
        }
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, SymthaeaWireV2Error> {
        Ok(self.take_exact(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, SymthaeaWireV2Error> {
        Ok(u16::from_be_bytes(self.fixed()?))
    }

    fn u32(&mut self) -> Result<u32, SymthaeaWireV2Error> {
        Ok(u32::from_be_bytes(self.fixed()?))
    }

    fn u64(&mut self) -> Result<u64, SymthaeaWireV2Error> {
        Ok(u64::from_be_bytes(self.fixed()?))
    }

    fn i64(&mut self) -> Result<i64, SymthaeaWireV2Error> {
        Ok(i64::from_be_bytes(self.fixed()?))
    }

    fn f64(&mut self) -> Result<f64, SymthaeaWireV2Error> {
        Ok(f64::from_bits(self.u64()?))
    }

    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], SymthaeaWireV2Error> {
        self.take_exact(N)?
            .try_into()
            .map_err(|_| SymthaeaWireV2Error::TruncatedWire)
    }

    fn string(&mut self, max: usize) -> Result<String, SymthaeaWireV2Error> {
        let len = usize::try_from(self.u32()?).map_err(|_| SymthaeaWireV2Error::LengthOverflow)?;
        if len > max {
            return Err(SymthaeaWireV2Error::FieldTooLarge);
        }
        let bytes = self.take_exact(len)?;
        let text = std::str::from_utf8(bytes).map_err(|_| SymthaeaWireV2Error::InvalidUtf8)?;
        Ok(text.to_owned())
    }

    fn vector_len(&mut self) -> Result<usize, SymthaeaWireV2Error> {
        let len = usize::try_from(self.u32()?).map_err(|_| SymthaeaWireV2Error::LengthOverflow)?;
        if len > MAX_VECTOR_ITEMS {
            return Err(SymthaeaWireV2Error::TooManyItems);
        }
        Ok(len)
    }

    fn option<T>(
        &mut self,
        decode: impl FnOnce(&mut Self) -> Result<T, SymthaeaWireV2Error>,
    ) -> Result<Option<T>, SymthaeaWireV2Error> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(decode(self)?)),
            tag => Err(SymthaeaWireV2Error::InvalidOptionTag(tag)),
        }
    }
}

fn decode_envelope(reader: &mut Reader<'_>) -> Result<SymthaeaClinicalInferenceV2, SymthaeaWireV2Error> {
    let schema_version = reader.u16()?;
    let semantics = decode_semantics(reader)?;
    let subject = reader.option(decode_subject)?;
    let statement = reader.string(MAX_TEXT_BYTES)?;
    let evidence_len = reader.vector_len()?;
    let mut evidence = Vec::with_capacity(evidence_len);
    for _ in 0..evidence_len {
        evidence.push(decode_evidence_ref(reader)?);
    }
    let alternative_len = reader.vector_len()?;
    let mut alternatives = Vec::with_capacity(alternative_len);
    for _ in 0..alternative_len {
        alternatives.push(decode_alternative(reader)?);
    }
    let missing_len = reader.vector_len()?;
    let mut missing_evidence = Vec::with_capacity(missing_len);
    for _ in 0..missing_len {
        missing_evidence.push(decode_missing(reader)?);
    }
    let uncertainty = decode_uncertainty(reader)?;
    let distribution = decode_distribution(reader)?;
    let execution = decode_execution(reader)?;
    let generated_at_micros = reader.i64()?;
    Ok(SymthaeaClinicalInferenceV2 {
        schema_version,
        semantics,
        subject,
        statement,
        evidence,
        alternatives,
        missing_evidence,
        uncertainty,
        distribution,
        execution,
        generated_at_micros,
    })
}

fn decode_digest(reader: &mut Reader<'_>) -> Result<SymthaeaDigestV2, SymthaeaWireV2Error> {
    let algorithm = match reader.u8()? {
        0 => SymthaeaDigestAlgorithmV2::Blake3_256,
        tag => return Err(SymthaeaWireV2Error::InvalidEnumTag("digest algorithm", tag)),
    };
    Ok(SymthaeaDigestV2 {
        algorithm,
        value: reader.fixed()?,
    })
}

fn decode_evidence_identity(reader: &mut Reader<'_>) -> Result<SymthaeaEvidenceIdentityV2, SymthaeaWireV2Error> {
    Ok(SymthaeaEvidenceIdentityV2 {
        identity_version: reader.u16()?,
        namespace: reader.string(MAX_ID_BYTES)?,
        artifact_id: reader.string(MAX_ID_BYTES)?,
        digest: decode_digest(reader)?,
    })
}

fn decode_semantics(reader: &mut Reader<'_>) -> Result<SymthaeaClaimSemanticsV2, SymthaeaWireV2Error> {
    let schema_version = reader.u16()?;
    Ok(SymthaeaClaimSemanticsV2 {
        schema_version,
        claim_kind: match reader.u8()? {
            0 => SymthaeaClaimKindV2::CandidateSignal,
            1 => SymthaeaClaimKindV2::Association,
            2 => SymthaeaClaimKindV2::Prediction,
            3 => SymthaeaClaimKindV2::RiskEstimate,
            4 => SymthaeaClaimKindV2::CausalHypothesis,
            5 => SymthaeaClaimKindV2::CausalEffectEstimate,
            6 => SymthaeaClaimKindV2::DiagnosticSupport,
            7 => SymthaeaClaimKindV2::TreatmentSupport,
            tag => return Err(SymthaeaWireV2Error::InvalidEnumTag("claim kind", tag)),
        },
        evidence_stage: match reader.u8()? {
            0 => SymthaeaEvidenceStageV2::MechanisticHypothesis,
            1 => SymthaeaEvidenceStageV2::SyntheticDemonstration,
            2 => SymthaeaEvidenceStageV2::RetrospectiveInternal,
            3 => SymthaeaEvidenceStageV2::RetrospectiveExternal,
            4 => SymthaeaEvidenceStageV2::ProspectiveShadow,
            5 => SymthaeaEvidenceStageV2::ProspectiveClinicalStudy,
            6 => SymthaeaEvidenceStageV2::ReplicatedClinicalEvidence,
            tag => return Err(SymthaeaWireV2Error::InvalidEnumTag("evidence stage", tag)),
        },
        applicability: match reader.u8()? {
            0 => SymthaeaApplicabilityV2::Unestablished,
            1 => SymthaeaApplicabilityV2::EvaluatedCohortOnly,
            2 => SymthaeaApplicabilityV2::DefinedTargetPopulation,
            3 => SymthaeaApplicabilityV2::ValidatedTargetPopulation,
            tag => return Err(SymthaeaWireV2Error::InvalidEnumTag("applicability", tag)),
        },
        intended_use: match reader.u8()? {
            0 => SymthaeaIntendedUseV2::ResearchOnly,
            1 => SymthaeaIntendedUseV2::ClinicalDecisionSupport,
            tag => return Err(SymthaeaWireV2Error::InvalidEnumTag("intended use", tag)),
        },
    })
}

fn decode_artifact(reader: &mut Reader<'_>) -> Result<SymthaeaArtifactIdentityV2, SymthaeaWireV2Error> {
    Ok(SymthaeaArtifactIdentityV2 {
        name: reader.string(MAX_ID_BYTES)?,
        version: reader.string(MAX_ID_BYTES)?,
        digest: decode_digest(reader)?,
    })
}

fn decode_subject(reader: &mut Reader<'_>) -> Result<SymthaeaSubjectBindingV2, SymthaeaWireV2Error> {
    Ok(SymthaeaSubjectBindingV2 {
        subject_namespace: reader.string(MAX_ID_BYTES)?,
        subject_id: reader.string(MAX_ID_BYTES)?,
        binding_evidence: decode_evidence_identity(reader)?,
    })
}

fn decode_evidence_ref(reader: &mut Reader<'_>) -> Result<SymthaeaEvidenceRefV2, SymthaeaWireV2Error> {
    Ok(SymthaeaEvidenceRefV2 {
        identity: decode_evidence_identity(reader)?,
        role: match reader.u8()? {
            0 => SymthaeaEvidenceRoleV2::Supports,
            1 => SymthaeaEvidenceRoleV2::Opposes,
            2 => SymthaeaEvidenceRoleV2::Context,
            3 => SymthaeaEvidenceRoleV2::Contraindication,
            tag => return Err(SymthaeaWireV2Error::InvalidEnumTag("evidence role", tag)),
        },
    })
}

fn decode_alternative(reader: &mut Reader<'_>) -> Result<SymthaeaAlternativeV2, SymthaeaWireV2Error> {
    Ok(SymthaeaAlternativeV2 {
        statement: reader.string(MAX_TEXT_BYTES)?,
        semantics: decode_semantics(reader)?,
        rationale: reader.string(MAX_TEXT_BYTES)?,
    })
}

fn decode_missing(reader: &mut Reader<'_>) -> Result<SymthaeaMissingEvidenceV2, SymthaeaWireV2Error> {
    Ok(SymthaeaMissingEvidenceV2 {
        requirement_id: reader.string(MAX_ID_BYTES)?,
        description: reader.string(MAX_TEXT_BYTES)?,
        criticality: match reader.u8()? {
            0 => SymthaeaMissingCriticalityV2::Informational,
            1 => SymthaeaMissingCriticalityV2::Important,
            2 => SymthaeaMissingCriticalityV2::Critical,
            tag => return Err(SymthaeaWireV2Error::InvalidEnumTag("missing criticality", tag)),
        },
    })
}

fn decode_model(reader: &mut Reader<'_>) -> Result<SymthaeaModelIdentityV2, SymthaeaWireV2Error> {
    Ok(SymthaeaModelIdentityV2 {
        model: decode_artifact(reader)?,
        input_schema_digest: decode_digest(reader)?,
        output_schema_digest: decode_digest(reader)?,
        training_lineage: reader.option(decode_evidence_identity)?,
        evaluation_lineage: reader.option(decode_evidence_identity)?,
        calibration_evidence: reader.option(decode_evidence_identity)?,
    })
}

fn decode_optional_f64(reader: &mut Reader<'_>) -> Result<Option<f64>, SymthaeaWireV2Error> {
    match reader.u8()? {
        0 => Ok(None),
        1 => Ok(Some(reader.f64()?)),
        tag => Err(SymthaeaWireV2Error::InvalidOptionTag(tag)),
    }
}

fn decode_uncertainty(reader: &mut Reader<'_>) -> Result<SymthaeaUncertaintyV2, SymthaeaWireV2Error> {
    Ok(SymthaeaUncertaintyV2 {
        epistemic: decode_optional_f64(reader)?,
        aleatoric: decode_optional_f64(reader)?,
        calibrated_probability: decode_optional_f64(reader)?,
        calibration_status: match reader.u8()? {
            0 => SymthaeaCalibrationStatusV2::NotAssessed,
            1 => SymthaeaCalibrationStatusV2::Uncalibrated,
            2 => SymthaeaCalibrationStatusV2::Calibrated,
            tag => return Err(SymthaeaWireV2Error::InvalidEnumTag("calibration status", tag)),
        },
        calibration_evidence: reader.option(decode_evidence_identity)?,
    })
}

fn decode_distribution(reader: &mut Reader<'_>) -> Result<SymthaeaDistributionAssessmentV2, SymthaeaWireV2Error> {
    Ok(SymthaeaDistributionAssessmentV2 {
        status: match reader.u8()? {
            0 => SymthaeaDistributionStatusV2::Unknown,
            1 => SymthaeaDistributionStatusV2::InDistribution,
            2 => SymthaeaDistributionStatusV2::OutOfDistribution,
            tag => return Err(SymthaeaWireV2Error::InvalidEnumTag("distribution status", tag)),
        },
        detector_evidence: reader.option(decode_evidence_identity)?,
    })
}

fn decode_execution(reader: &mut Reader<'_>) -> Result<SymthaeaExecutionIdentityV2, SymthaeaWireV2Error> {
    let engine = decode_artifact(reader)?;
    let model = decode_model(reader)?;
    let runtime_digest = decode_digest(reader)?;
    let configuration_digest = decode_digest(reader)?;
    let input_len = reader.vector_len()?;
    let mut input_evidence = Vec::with_capacity(input_len);
    for _ in 0..input_len {
        input_evidence.push(decode_evidence_identity(reader)?);
    }
    Ok(SymthaeaExecutionIdentityV2 {
        engine,
        model,
        runtime_digest,
        configuration_digest,
        input_evidence,
        operation: reader.string(MAX_ID_BYTES)?,
        executed_at_micros: reader.i64()?,
        execution_nonce: reader.fixed()?,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymthaeaWireV2Error {
    WrongMagic,
    UnsupportedWireVersion(u16),
    UnsupportedEnvelopeVersion(u16),
    UnsupportedClaimVocabularyVersion(u16),
    UnsupportedEvidenceIdentityVersion(u16),
    InvalidWireLength,
    FieldTooLarge,
    TooManyItems,
    LengthOverflow,
    TruncatedWire,
    InvalidUtf8,
    InvalidOptionTag(u8),
    InvalidEnumTag(&'static str, u8),
    TrailingBytes,
    InvalidIdentifier,
    InvalidText,
    InvalidNamespace,
    ZeroDigest,
    ZeroExecutionNonce,
    MissingExecutionInputs,
    DuplicateExecutionInput,
    ClinicalUseWithoutSubject,
    SubjectBindingNotExecutionBound,
    MissingEvidence,
    DuplicateEvidenceIdentity,
    EvidenceNotExecutionBound,
    InvalidUncertainty,
    InvalidCalibratedProbability,
    IncompleteCalibrationEvidence,
    ModelCalibrationEvidenceRequired,
    CalibrationEvidenceMismatch,
    MissingDistributionEvidence,
}

#[cfg(test)]
mod tests {
    use super::*;

    const VECTOR_HEX: &str = include_str!("../fixtures/clinical_inference_wire_v2.hex");

    fn fixture() -> Vec<u8> {
        let compact: Vec<u8> = VECTOR_HEX
            .bytes()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect();
        assert_eq!(compact.len() % 2, 0);
        compact
            .chunks_exact(2)
            .map(|pair| (hex(pair[0]) << 4) | hex(pair[1]))
            .collect()
    }

    fn hex(value: u8) -> u8 {
        match value {
            b'0'..=b'9' => value - b'0',
            b'a'..=b'f' => value - b'a' + 10,
            b'A'..=b'F' => value - b'A' + 10,
            _ => panic!("invalid fixture hex"),
        }
    }

    #[test]
    fn independently_decodes_frozen_symthaea_vector() {
        let bytes = fixture();
        assert_eq!(bytes.len(), 1_260);
        let value = verify_symthaea_clinical_wire_v2(&bytes).unwrap();
        assert_eq!(value.schema_version, 2);
        assert_eq!(value.semantics.claim_kind, SymthaeaClaimKindV2::Prediction);
        assert_eq!(value.semantics.evidence_stage, SymthaeaEvidenceStageV2::RetrospectiveExternal);
        assert_eq!(value.subject.as_ref().unwrap().subject_id, "patient-a");
        assert_eq!(value.evidence[0].identity.namespace, "mycelix/clinical-fact-snapshot/v1");
        assert_eq!(value.evidence[0].identity.artifact_id, "fact-1");
        assert_eq!(value.execution.input_evidence.len(), 2);
        assert_eq!(value.execution.operation, "evaluate");
        assert_eq!(value.generated_at_micros, 1_001);
    }

    #[test]
    fn independently_reproduces_nonzero_symthaea_wire_identity() {
        let digest = verify_symthaea_clinical_wire_v2_digest(&fixture()).unwrap();
        assert_ne!(digest.into_bytes(), [0u8; 32]);
    }

    #[test]
    fn trailing_byte_is_rejected() {
        let mut bytes = fixture();
        bytes.push(0);
        assert_eq!(
            verify_symthaea_clinical_wire_v2(&bytes),
            Err(SymthaeaWireV2Error::TrailingBytes)
        );
    }

    #[test]
    fn truncation_is_rejected() {
        let mut bytes = fixture();
        bytes.pop();
        assert!(matches!(
            verify_symthaea_clinical_wire_v2(&bytes),
            Err(SymthaeaWireV2Error::TruncatedWire)
        ));
    }

    #[test]
    fn wrong_magic_is_rejected() {
        let mut bytes = fixture();
        bytes[0] ^= 0xff;
        assert_eq!(
            verify_symthaea_clinical_wire_v2(&bytes),
            Err(SymthaeaWireV2Error::WrongMagic)
        );
    }

    #[test]
    fn parsed_fixture_preserves_typed_calibration_identity() {
        let value = verify_symthaea_clinical_wire_v2(&fixture()).unwrap();
        let inference_calibration = value.uncertainty.calibration_evidence.unwrap();
        let model_calibration = value.execution.model.calibration_evidence.unwrap();
        assert_eq!(inference_calibration, model_calibration);
        assert_eq!(inference_calibration.namespace, "symthaea/model-calibration-evidence/v1");
        assert_eq!(inference_calibration.artifact_id, "cal-1");
    }

    #[test]
    fn parsed_fixture_preserves_typed_distribution_identity() {
        let value = verify_symthaea_clinical_wire_v2(&fixture()).unwrap();
        assert_eq!(value.distribution.status, SymthaeaDistributionStatusV2::InDistribution);
        let detector = value.distribution.detector_evidence.unwrap();
        assert_eq!(detector.namespace, "symthaea/ood-detector-evidence/v1");
        assert_eq!(detector.artifact_id, "ood-1");
    }
}
