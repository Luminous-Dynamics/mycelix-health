#![deny(unsafe_code)]
//! Deployment-scoped admission preflight for typed Symthaea clinical inference v2.
//!
//! This crate sits downstream of the independent Mycelix binary-wire verifier.
//! It answers only whether one exact verified Symthaea v2 inference matches one
//! exact Mycelix deployment admission policy. It does not verify ClinicalFact
//! truth, independent OOD trust, practitioner authority, or clinical presentation.

use mycelix_symthaea_clinical_wire_v2::{
    verify_symthaea_clinical_wire_v2, verify_symthaea_clinical_wire_v2_digest,
    SymthaeaArtifactIdentityV2, SymthaeaCalibrationStatusV2, SymthaeaClaimKindV2,
    SymthaeaClinicalInferenceV2, SymthaeaDigestAlgorithmV2, SymthaeaDigestV2,
    SymthaeaDistributionStatusV2, SymthaeaEvidenceIdentityV2, SymthaeaEvidenceStageV2,
    SymthaeaIntendedUseV2, SymthaeaMissingCriticalityV2, SymthaeaModelIdentityV2,
    SymthaeaWireV2Error, VerifiedSymthaeaWireDigestV2,
    SYMTHAEA_CLINICAL_ENVELOPE_V2_VERSION, SYMTHAEA_CLINICAL_WIRE_V2_VERSION,
};
use std::collections::HashSet;
use thiserror::Error;

pub const SYMTHAEA_CLINICAL_ADMISSION_POLICY_V2_VERSION: u16 = 1;

const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000; // five minutes
const MAX_INFERENCE_AGE_MICROS: i64 = 31_536_000_000_000; // 365 days
const MAX_POLICY_TOKEN_BYTES: usize = 4096;
const POLICY_DERIVE_KEY_CONTEXT: &str =
    "mycelix.health.symthaea-clinical-admission-v2-policy.v1";
const PREFLIGHT_DERIVE_KEY_CONTEXT: &str =
    "mycelix.health.symthaea-clinical-admission-v2-preflight.v1";

/// Hard Mycelix-owned ceiling. There is intentionally no SupervisedClinical
/// variant and no implicit ordering derived from producer evidence labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SymthaeaAdmissionCeilingV2 {
    Experimental,
    ValidatedOffline,
    ShadowClinical,
}

/// Exact deployment admission contract for one class of Symthaea v2 inference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymthaeaClinicalAdmissionPolicyV2 {
    pub schema_version: u16,
    pub policy_id: String,
    pub accepted_wire_version: u16,
    pub accepted_envelope_version: u16,
    pub accepted_engine: SymthaeaArtifactIdentityV2,
    pub accepted_model: SymthaeaModelIdentityV2,
    pub accepted_runtime_digest: SymthaeaDigestV2,
    pub accepted_configuration_digest: SymthaeaDigestV2,
    pub accepted_operation: String,
    pub allowed_claim_kinds: Vec<SymthaeaClaimKindV2>,
    pub allowed_evidence_stages: Vec<SymthaeaEvidenceStageV2>,
    pub required_intended_use: SymthaeaIntendedUseV2,
    pub required_subject_namespace: Option<String>,
    pub required_subject_binding_namespace: Option<String>,
    pub allowed_claim_evidence_namespaces: Vec<String>,
    pub required_claim_evidence_namespaces: Vec<String>,
    pub required_producer_distribution_namespace: Option<String>,
    pub max_inference_age_micros: i64,
    pub max_future_skew_micros: i64,
    pub require_calibrated_probability: bool,
    pub qualification_ceiling: SymthaeaAdmissionCeilingV2,
}

impl SymthaeaClinicalAdmissionPolicyV2 {
    pub fn validate(&self) -> Result<(), SymthaeaAdmissionV2Error> {
        if self.schema_version != SYMTHAEA_CLINICAL_ADMISSION_POLICY_V2_VERSION {
            return Err(SymthaeaAdmissionV2Error::UnsupportedPolicyVersion(
                self.schema_version,
            ));
        }
        validate_token(&self.policy_id)?;
        if self.accepted_wire_version != SYMTHAEA_CLINICAL_WIRE_V2_VERSION {
            return Err(SymthaeaAdmissionV2Error::UnsupportedWireVersion(
                self.accepted_wire_version,
            ));
        }
        if self.accepted_envelope_version != SYMTHAEA_CLINICAL_ENVELOPE_V2_VERSION {
            return Err(SymthaeaAdmissionV2Error::UnsupportedEnvelopeVersion(
                self.accepted_envelope_version,
            ));
        }
        validate_artifact(&self.accepted_engine)?;
        validate_model(&self.accepted_model)?;
        validate_digest(&self.accepted_runtime_digest)?;
        validate_digest(&self.accepted_configuration_digest)?;
        validate_token(&self.accepted_operation)?;
        validate_nonempty_unique(
            &self.allowed_claim_kinds,
            SymthaeaAdmissionV2Error::MissingClaimKinds,
            SymthaeaAdmissionV2Error::DuplicateClaimKind,
        )?;
        validate_nonempty_unique(
            &self.allowed_evidence_stages,
            SymthaeaAdmissionV2Error::MissingEvidenceStages,
            SymthaeaAdmissionV2Error::DuplicateEvidenceStage,
        )?;
        validate_namespace_list(&self.allowed_claim_evidence_namespaces, true)?;
        validate_namespace_list(&self.required_claim_evidence_namespaces, false)?;

        let allowed: HashSet<&str> = self
            .allowed_claim_evidence_namespaces
            .iter()
            .map(String::as_str)
            .collect();
        for namespace in &self.required_claim_evidence_namespaces {
            if !allowed.contains(namespace.as_str()) {
                return Err(SymthaeaAdmissionV2Error::RequiredNamespaceNotAllowed);
            }
        }

        for namespace in [
            self.required_subject_namespace.as_deref(),
            self.required_subject_binding_namespace.as_deref(),
            self.required_producer_distribution_namespace.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            validate_token(namespace)?;
        }

        if self.required_intended_use == SymthaeaIntendedUseV2::ClinicalDecisionSupport {
            if self.required_subject_namespace.is_none() {
                return Err(SymthaeaAdmissionV2Error::ClinicalPolicyRequiresSubjectNamespace);
            }
            if self.required_subject_binding_namespace.is_none() {
                return Err(
                    SymthaeaAdmissionV2Error::ClinicalPolicyRequiresSubjectBindingNamespace,
                );
            }
            if self.required_claim_evidence_namespaces.is_empty() {
                return Err(
                    SymthaeaAdmissionV2Error::ClinicalPolicyRequiresEvidenceNamespace,
                );
            }
        }

        if self.max_inference_age_micros <= 0
            || self.max_inference_age_micros > MAX_INFERENCE_AGE_MICROS
        {
            return Err(SymthaeaAdmissionV2Error::InvalidMaximumInferenceAge);
        }
        if self.max_future_skew_micros < 0
            || self.max_future_skew_micros > MAX_FUTURE_SKEW_MICROS
        {
            return Err(SymthaeaAdmissionV2Error::InvalidMaximumFutureSkew);
        }
        Ok(())
    }

    /// Serializer-independent policy identity.
    pub fn digest(&self) -> Result<SymthaeaAdmissionPolicyDigestV2, SymthaeaAdmissionV2Error> {
        self.validate()?;
        let bytes = canonical_policy_bytes(self)?;
        let mut hasher = blake3::Hasher::new_derive_key(POLICY_DERIVE_KEY_CONTEXT);
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
        Ok(SymthaeaAdmissionPolicyDigestV2(*hasher.finalize().as_bytes()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymthaeaAdmissionPolicyDigestV2([u8; 32]);

impl SymthaeaAdmissionPolicyDigestV2 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymthaeaAdmissionPreflightDigestV2([u8; 32]);

impl SymthaeaAdmissionPreflightDigestV2 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Non-serializable proof that one exact independently verified v2 inference
/// satisfied one exact deployment policy at one evaluation time.
pub struct AdmittedSymthaeaInferenceV2 {
    envelope: SymthaeaClinicalInferenceV2,
    wire_digest: VerifiedSymthaeaWireDigestV2,
    policy_digest: SymthaeaAdmissionPolicyDigestV2,
    preflight_digest: SymthaeaAdmissionPreflightDigestV2,
    qualification_ceiling: SymthaeaAdmissionCeilingV2,
    evaluated_at_micros: i64,
}

impl AdmittedSymthaeaInferenceV2 {
    pub fn envelope(&self) -> &SymthaeaClinicalInferenceV2 {
        &self.envelope
    }

    pub fn wire_digest(&self) -> VerifiedSymthaeaWireDigestV2 {
        self.wire_digest
    }

    pub fn policy_digest(&self) -> SymthaeaAdmissionPolicyDigestV2 {
        self.policy_digest
    }

    pub fn preflight_digest(&self) -> SymthaeaAdmissionPreflightDigestV2 {
        self.preflight_digest
    }

    pub fn qualification_ceiling(&self) -> SymthaeaAdmissionCeilingV2 {
        self.qualification_ceiling
    }

    pub fn evaluated_at_micros(&self) -> i64 {
        self.evaluated_at_micros
    }
}

/// Independently verify the binary wire artifact, then apply the exact Mycelix
/// deployment admission contract. No clinical promotion occurs here.
pub fn admit_symthaea_clinical_inference_v2(
    wire_bytes: &[u8],
    policy: &SymthaeaClinicalAdmissionPolicyV2,
    evaluated_at_micros: i64,
) -> Result<AdmittedSymthaeaInferenceV2, SymthaeaAdmissionV2Error> {
    policy.validate()?;
    let envelope = verify_symthaea_clinical_wire_v2(wire_bytes)?;
    let wire_digest = verify_symthaea_clinical_wire_v2_digest(wire_bytes)?;

    if envelope.execution.engine != policy.accepted_engine {
        return Err(SymthaeaAdmissionV2Error::EngineIdentityMismatch);
    }
    if envelope.execution.model != policy.accepted_model {
        return Err(SymthaeaAdmissionV2Error::ModelIdentityMismatch);
    }
    if envelope.execution.runtime_digest != policy.accepted_runtime_digest {
        return Err(SymthaeaAdmissionV2Error::RuntimeIdentityMismatch);
    }
    if envelope.execution.configuration_digest != policy.accepted_configuration_digest {
        return Err(SymthaeaAdmissionV2Error::ConfigurationIdentityMismatch);
    }
    if envelope.execution.operation != policy.accepted_operation {
        return Err(SymthaeaAdmissionV2Error::OperationMismatch);
    }
    if !policy
        .allowed_claim_kinds
        .contains(&envelope.semantics.claim_kind)
    {
        return Err(SymthaeaAdmissionV2Error::ClaimKindNotAdmitted);
    }
    if !policy
        .allowed_evidence_stages
        .contains(&envelope.semantics.evidence_stage)
    {
        return Err(SymthaeaAdmissionV2Error::EvidenceStageNotAdmitted);
    }
    if envelope.semantics.intended_use != policy.required_intended_use {
        return Err(SymthaeaAdmissionV2Error::IntendedUseMismatch);
    }

    if let Some(required_namespace) = &policy.required_subject_namespace {
        let subject = envelope
            .subject
            .as_ref()
            .ok_or(SymthaeaAdmissionV2Error::MissingRequiredSubject)?;
        if &subject.subject_namespace != required_namespace {
            return Err(SymthaeaAdmissionV2Error::SubjectNamespaceMismatch);
        }
    }
    if let Some(required_namespace) = &policy.required_subject_binding_namespace {
        let subject = envelope
            .subject
            .as_ref()
            .ok_or(SymthaeaAdmissionV2Error::MissingRequiredSubject)?;
        if &subject.binding_evidence.namespace != required_namespace {
            return Err(SymthaeaAdmissionV2Error::SubjectBindingNamespaceMismatch);
        }
    }

    let allowed_namespaces: HashSet<&str> = policy
        .allowed_claim_evidence_namespaces
        .iter()
        .map(String::as_str)
        .collect();
    let mut observed_namespaces = HashSet::new();
    for evidence in &envelope.evidence {
        if !allowed_namespaces.contains(evidence.identity.namespace.as_str()) {
            return Err(SymthaeaAdmissionV2Error::EvidenceNamespaceNotAdmitted(
                evidence.identity.namespace.clone(),
            ));
        }
        observed_namespaces.insert(evidence.identity.namespace.as_str());
    }
    for required in &policy.required_claim_evidence_namespaces {
        if !observed_namespaces.contains(required.as_str()) {
            return Err(SymthaeaAdmissionV2Error::RequiredEvidenceNamespaceMissing(
                required.clone(),
            ));
        }
    }

    if let Some(required_namespace) = &policy.required_producer_distribution_namespace {
        match &envelope.distribution.detector_evidence {
            Some(evidence) if &evidence.namespace == required_namespace => {}
            Some(_) => {
                return Err(SymthaeaAdmissionV2Error::DistributionNamespaceMismatch)
            }
            None if envelope.distribution.status == SymthaeaDistributionStatusV2::Unknown => {}
            None => return Err(SymthaeaAdmissionV2Error::DistributionEvidenceRequired),
        }
    }

    if envelope.generated_at_micros < envelope.execution.executed_at_micros {
        return Err(SymthaeaAdmissionV2Error::InvalidTemporalOrder);
    }
    if envelope.generated_at_micros
        > evaluated_at_micros.saturating_add(policy.max_future_skew_micros)
    {
        return Err(SymthaeaAdmissionV2Error::InferenceTooFarInFuture);
    }
    let age = evaluated_at_micros.saturating_sub(envelope.generated_at_micros);
    if age > policy.max_inference_age_micros {
        return Err(SymthaeaAdmissionV2Error::StaleInference);
    }

    if policy.require_calibrated_probability {
        if envelope.uncertainty.calibration_status != SymthaeaCalibrationStatusV2::Calibrated
            || envelope.uncertainty.calibrated_probability.is_none()
        {
            return Err(SymthaeaAdmissionV2Error::CalibratedInferenceRequired);
        }
        let model = envelope
            .execution
            .model
            .calibration_evidence
            .as_ref()
            .ok_or(SymthaeaAdmissionV2Error::ModelCalibrationEvidenceRequired)?;
        let inference = envelope
            .uncertainty
            .calibration_evidence
            .as_ref()
            .ok_or(SymthaeaAdmissionV2Error::InferenceCalibrationEvidenceRequired)?;
        if model != inference {
            return Err(SymthaeaAdmissionV2Error::CalibrationEvidenceMismatch);
        }
    }

    if envelope.semantics.intended_use == SymthaeaIntendedUseV2::ClinicalDecisionSupport {
        if envelope
            .missing_evidence
            .iter()
            .any(|missing| missing.criticality == SymthaeaMissingCriticalityV2::Critical)
        {
            return Err(SymthaeaAdmissionV2Error::CriticalEvidenceMissing);
        }
        if envelope.distribution.status == SymthaeaDistributionStatusV2::OutOfDistribution {
            return Err(SymthaeaAdmissionV2Error::ProducerReportsOutOfDistribution);
        }
    }

    let policy_digest = policy.digest()?;
    let preflight_digest = preflight_digest(
        wire_digest,
        policy_digest,
        policy.qualification_ceiling,
        evaluated_at_micros,
    );

    Ok(AdmittedSymthaeaInferenceV2 {
        envelope,
        wire_digest,
        policy_digest,
        preflight_digest,
        qualification_ceiling: policy.qualification_ceiling,
        evaluated_at_micros,
    })
}

fn preflight_digest(
    wire: VerifiedSymthaeaWireDigestV2,
    policy: SymthaeaAdmissionPolicyDigestV2,
    ceiling: SymthaeaAdmissionCeilingV2,
    evaluated_at_micros: i64,
) -> SymthaeaAdmissionPreflightDigestV2 {
    let mut hasher = blake3::Hasher::new_derive_key(PREFLIGHT_DERIVE_KEY_CONTEXT);
    hasher.update(wire.as_bytes());
    hasher.update(policy.as_bytes());
    hasher.update(&[ceiling_tag(ceiling)]);
    hasher.update(&evaluated_at_micros.to_be_bytes());
    SymthaeaAdmissionPreflightDigestV2(*hasher.finalize().as_bytes())
}

fn canonical_policy_bytes(
    policy: &SymthaeaClinicalAdmissionPolicyV2,
) -> Result<Vec<u8>, SymthaeaAdmissionV2Error> {
    let mut writer = PolicyWriter::default();
    writer.u16(policy.schema_version);
    writer.string(&policy.policy_id)?;
    writer.u16(policy.accepted_wire_version);
    writer.u16(policy.accepted_envelope_version);
    writer.artifact(&policy.accepted_engine)?;
    writer.model(&policy.accepted_model)?;
    writer.digest(&policy.accepted_runtime_digest)?;
    writer.digest(&policy.accepted_configuration_digest)?;
    writer.string(&policy.accepted_operation)?;
    writer.u32_len(policy.allowed_claim_kinds.len())?;
    for kind in &policy.allowed_claim_kinds {
        writer.u8(claim_kind_tag(*kind));
    }
    writer.u32_len(policy.allowed_evidence_stages.len())?;
    for stage in &policy.allowed_evidence_stages {
        writer.u8(evidence_stage_tag(*stage));
    }
    writer.u8(intended_use_tag(policy.required_intended_use));
    writer.option_string(policy.required_subject_namespace.as_deref())?;
    writer.option_string(policy.required_subject_binding_namespace.as_deref())?;
    writer.string_vec(&policy.allowed_claim_evidence_namespaces)?;
    writer.string_vec(&policy.required_claim_evidence_namespaces)?;
    writer.option_string(policy.required_producer_distribution_namespace.as_deref())?;
    writer.i64(policy.max_inference_age_micros);
    writer.i64(policy.max_future_skew_micros);
    writer.u8(u8::from(policy.require_calibrated_probability));
    writer.u8(ceiling_tag(policy.qualification_ceiling));
    Ok(writer.finish())
}

#[derive(Default)]
struct PolicyWriter {
    bytes: Vec<u8>,
}

impl PolicyWriter {
    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u32_len(&mut self, len: usize) -> Result<(), SymthaeaAdmissionV2Error> {
        let value = u32::try_from(len).map_err(|_| SymthaeaAdmissionV2Error::PolicyLengthOverflow)?;
        self.bytes.extend_from_slice(&value.to_be_bytes());
        Ok(())
    }

    fn string(&mut self, value: &str) -> Result<(), SymthaeaAdmissionV2Error> {
        self.u32_len(value.len())?;
        self.bytes.extend_from_slice(value.as_bytes());
        Ok(())
    }

    fn option_string(&mut self, value: Option<&str>) -> Result<(), SymthaeaAdmissionV2Error> {
        match value {
            None => self.u8(0),
            Some(value) => {
                self.u8(1);
                self.string(value)?;
            }
        }
        Ok(())
    }

    fn string_vec(&mut self, values: &[String]) -> Result<(), SymthaeaAdmissionV2Error> {
        self.u32_len(values.len())?;
        for value in values {
            self.string(value)?;
        }
        Ok(())
    }

    fn digest(&mut self, digest: &SymthaeaDigestV2) -> Result<(), SymthaeaAdmissionV2Error> {
        self.u8(match digest.algorithm {
            SymthaeaDigestAlgorithmV2::Blake3_256 => 0,
        });
        self.bytes.extend_from_slice(&digest.value);
        Ok(())
    }

    fn evidence_identity(
        &mut self,
        identity: &SymthaeaEvidenceIdentityV2,
    ) -> Result<(), SymthaeaAdmissionV2Error> {
        self.u16(identity.identity_version);
        self.string(&identity.namespace)?;
        self.string(&identity.artifact_id)?;
        self.digest(&identity.digest)
    }

    fn option_evidence_identity(
        &mut self,
        identity: Option<&SymthaeaEvidenceIdentityV2>,
    ) -> Result<(), SymthaeaAdmissionV2Error> {
        match identity {
            None => self.u8(0),
            Some(identity) => {
                self.u8(1);
                self.evidence_identity(identity)?;
            }
        }
        Ok(())
    }

    fn artifact(
        &mut self,
        artifact: &SymthaeaArtifactIdentityV2,
    ) -> Result<(), SymthaeaAdmissionV2Error> {
        self.string(&artifact.name)?;
        self.string(&artifact.version)?;
        self.digest(&artifact.digest)
    }

    fn model(&mut self, model: &SymthaeaModelIdentityV2) -> Result<(), SymthaeaAdmissionV2Error> {
        self.artifact(&model.model)?;
        self.digest(&model.input_schema_digest)?;
        self.digest(&model.output_schema_digest)?;
        self.option_evidence_identity(model.training_lineage.as_ref())?;
        self.option_evidence_identity(model.evaluation_lineage.as_ref())?;
        self.option_evidence_identity(model.calibration_evidence.as_ref())?;
        Ok(())
    }
}

fn validate_artifact(
    artifact: &SymthaeaArtifactIdentityV2,
) -> Result<(), SymthaeaAdmissionV2Error> {
    validate_token(&artifact.name)?;
    validate_token(&artifact.version)?;
    validate_digest(&artifact.digest)
}

fn validate_model(model: &SymthaeaModelIdentityV2) -> Result<(), SymthaeaAdmissionV2Error> {
    validate_artifact(&model.model)?;
    validate_digest(&model.input_schema_digest)?;
    validate_digest(&model.output_schema_digest)?;
    for identity in [
        model.training_lineage.as_ref(),
        model.evaluation_lineage.as_ref(),
        model.calibration_evidence.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        validate_evidence_identity(identity)?;
    }
    Ok(())
}

fn validate_evidence_identity(
    identity: &SymthaeaEvidenceIdentityV2,
) -> Result<(), SymthaeaAdmissionV2Error> {
    if identity.identity_version != 1 {
        return Err(SymthaeaAdmissionV2Error::UnsupportedEvidenceIdentityVersion(
            identity.identity_version,
        ));
    }
    validate_token(&identity.namespace)?;
    validate_token(&identity.artifact_id)?;
    validate_digest(&identity.digest)
}

fn validate_digest(digest: &SymthaeaDigestV2) -> Result<(), SymthaeaAdmissionV2Error> {
    if digest.value == [0u8; 32] {
        return Err(SymthaeaAdmissionV2Error::ZeroDigest);
    }
    Ok(())
}

fn validate_token(value: &str) -> Result<(), SymthaeaAdmissionV2Error> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().any(char::is_control)
        || value.len() > MAX_POLICY_TOKEN_BYTES
    {
        return Err(SymthaeaAdmissionV2Error::InvalidPolicyToken);
    }
    Ok(())
}

fn validate_namespace_list(
    values: &[String],
    require_nonempty: bool,
) -> Result<(), SymthaeaAdmissionV2Error> {
    if require_nonempty && values.is_empty() {
        return Err(SymthaeaAdmissionV2Error::MissingAllowedEvidenceNamespaces);
    }
    let mut seen = HashSet::new();
    for value in values {
        validate_token(value)?;
        if !seen.insert(value.as_str()) {
            return Err(SymthaeaAdmissionV2Error::DuplicateEvidenceNamespace);
        }
    }
    Ok(())
}

fn validate_nonempty_unique<T>(
    values: &[T],
    missing: SymthaeaAdmissionV2Error,
    duplicate: SymthaeaAdmissionV2Error,
) -> Result<(), SymthaeaAdmissionV2Error>
where
    T: Eq + std::hash::Hash,
{
    if values.is_empty() {
        return Err(missing);
    }
    let mut seen = HashSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(duplicate);
        }
    }
    Ok(())
}

const fn claim_kind_tag(value: SymthaeaClaimKindV2) -> u8 {
    match value {
        SymthaeaClaimKindV2::CandidateSignal => 0,
        SymthaeaClaimKindV2::Association => 1,
        SymthaeaClaimKindV2::Prediction => 2,
        SymthaeaClaimKindV2::RiskEstimate => 3,
        SymthaeaClaimKindV2::CausalHypothesis => 4,
        SymthaeaClaimKindV2::CausalEffectEstimate => 5,
        SymthaeaClaimKindV2::DiagnosticSupport => 6,
        SymthaeaClaimKindV2::TreatmentSupport => 7,
    }
}

const fn evidence_stage_tag(value: SymthaeaEvidenceStageV2) -> u8 {
    match value {
        SymthaeaEvidenceStageV2::MechanisticHypothesis => 0,
        SymthaeaEvidenceStageV2::SyntheticDemonstration => 1,
        SymthaeaEvidenceStageV2::RetrospectiveInternal => 2,
        SymthaeaEvidenceStageV2::RetrospectiveExternal => 3,
        SymthaeaEvidenceStageV2::ProspectiveShadow => 4,
        SymthaeaEvidenceStageV2::ProspectiveClinicalStudy => 5,
        SymthaeaEvidenceStageV2::ReplicatedClinicalEvidence => 6,
    }
}

const fn intended_use_tag(value: SymthaeaIntendedUseV2) -> u8 {
    match value {
        SymthaeaIntendedUseV2::ResearchOnly => 0,
        SymthaeaIntendedUseV2::ClinicalDecisionSupport => 1,
    }
}

const fn ceiling_tag(value: SymthaeaAdmissionCeilingV2) -> u8 {
    match value {
        SymthaeaAdmissionCeilingV2::Experimental => 0,
        SymthaeaAdmissionCeilingV2::ValidatedOffline => 1,
        SymthaeaAdmissionCeilingV2::ShadowClinical => 2,
    }
}

#[derive(Debug, Error)]
pub enum SymthaeaAdmissionV2Error {
    #[error("Symthaea v2 wire verification failed: {0:?}")]
    Wire(SymthaeaWireV2Error),
    #[error("unsupported admission policy version {0}")]
    UnsupportedPolicyVersion(u16),
    #[error("unsupported Symthaea wire version {0}")]
    UnsupportedWireVersion(u16),
    #[error("unsupported Symthaea envelope version {0}")]
    UnsupportedEnvelopeVersion(u16),
    #[error("unsupported typed evidence identity version {0}")]
    UnsupportedEvidenceIdentityVersion(u16),
    #[error("policy token is empty, non-canonical, control-bearing, or too long")]
    InvalidPolicyToken,
    #[error("digest cannot be all zero")]
    ZeroDigest,
    #[error("at least one claim kind must be admitted")]
    MissingClaimKinds,
    #[error("duplicate admitted claim kind")]
    DuplicateClaimKind,
    #[error("at least one evidence stage must be admitted")]
    MissingEvidenceStages,
    #[error("duplicate admitted evidence stage")]
    DuplicateEvidenceStage,
    #[error("at least one claim-evidence namespace must be allowed")]
    MissingAllowedEvidenceNamespaces,
    #[error("duplicate evidence namespace")]
    DuplicateEvidenceNamespace,
    #[error("required evidence namespace is not in the allowed set")]
    RequiredNamespaceNotAllowed,
    #[error("clinical decision support policy requires a subject namespace")]
    ClinicalPolicyRequiresSubjectNamespace,
    #[error("clinical decision support policy requires a subject-binding evidence namespace")]
    ClinicalPolicyRequiresSubjectBindingNamespace,
    #[error("clinical decision support policy requires at least one exact claim-evidence namespace")]
    ClinicalPolicyRequiresEvidenceNamespace,
    #[error("maximum inference age is invalid")]
    InvalidMaximumInferenceAge,
    #[error("maximum future skew is invalid")]
    InvalidMaximumFutureSkew,
    #[error("policy framing length overflow")]
    PolicyLengthOverflow,
    #[error("Symthaea engine identity does not match deployment policy")]
    EngineIdentityMismatch,
    #[error("Symthaea model/schema/typed-lineage identity does not match deployment policy")]
    ModelIdentityMismatch,
    #[error("Symthaea runtime identity does not match deployment policy")]
    RuntimeIdentityMismatch,
    #[error("Symthaea configuration identity does not match deployment policy")]
    ConfigurationIdentityMismatch,
    #[error("Symthaea execution operation does not match deployment policy")]
    OperationMismatch,
    #[error("Symthaea claim kind is not admitted")]
    ClaimKindNotAdmitted,
    #[error("Symthaea evidence stage is not admitted")]
    EvidenceStageNotAdmitted,
    #[error("Symthaea intended use does not match deployment policy")]
    IntendedUseMismatch,
    #[error("required subject binding is missing")]
    MissingRequiredSubject,
    #[error("subject namespace does not match deployment policy")]
    SubjectNamespaceMismatch,
    #[error("subject-binding evidence namespace does not match deployment policy")]
    SubjectBindingNamespaceMismatch,
    #[error("claim evidence namespace is not admitted: {0}")]
    EvidenceNamespaceNotAdmitted(String),
    #[error("required claim evidence namespace is absent: {0}")]
    RequiredEvidenceNamespaceMissing(String),
    #[error("producer distribution evidence namespace does not match deployment policy")]
    DistributionNamespaceMismatch,
    #[error("producer distribution evidence is required by deployment policy")]
    DistributionEvidenceRequired,
    #[error("inference generation predates execution")]
    InvalidTemporalOrder,
    #[error("inference is dated beyond allowed future skew")]
    InferenceTooFarInFuture,
    #[error("inference is older than deployment policy allows")]
    StaleInference,
    #[error("deployment policy requires calibrated probability")]
    CalibratedInferenceRequired,
    #[error("model identity lacks required typed calibration evidence")]
    ModelCalibrationEvidenceRequired,
    #[error("inference lacks required typed calibration evidence")]
    InferenceCalibrationEvidenceRequired,
    #[error("model and inference typed calibration identities do not match")]
    CalibrationEvidenceMismatch,
    #[error("critical evidence is missing for clinical decision support")]
    CriticalEvidenceMissing,
    #[error("producer explicitly reports the inference as out-of-distribution")]
    ProducerReportsOutOfDistribution,
}

impl From<SymthaeaWireV2Error> for SymthaeaAdmissionV2Error {
    fn from(value: SymthaeaWireV2Error) -> Self {
        Self::Wire(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VECTOR_HEX: &str = include_str!(
        "../../symthaea-clinical-wire-v2/fixtures/clinical_inference_wire_v2.hex"
    );

    fn fixture() -> Vec<u8> {
        let compact: Vec<u8> = VECTOR_HEX
            .bytes()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect();
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

    fn policy() -> SymthaeaClinicalAdmissionPolicyV2 {
        let envelope = verify_symthaea_clinical_wire_v2(&fixture()).unwrap();
        SymthaeaClinicalAdmissionPolicyV2 {
            schema_version: SYMTHAEA_CLINICAL_ADMISSION_POLICY_V2_VERSION,
            policy_id: "symthaea-clinical-v2-test".into(),
            accepted_wire_version: SYMTHAEA_CLINICAL_WIRE_V2_VERSION,
            accepted_envelope_version: SYMTHAEA_CLINICAL_ENVELOPE_V2_VERSION,
            accepted_engine: envelope.execution.engine.clone(),
            accepted_model: envelope.execution.model.clone(),
            accepted_runtime_digest: envelope.execution.runtime_digest,
            accepted_configuration_digest: envelope.execution.configuration_digest,
            accepted_operation: envelope.execution.operation.clone(),
            allowed_claim_kinds: vec![envelope.semantics.claim_kind],
            allowed_evidence_stages: vec![envelope.semantics.evidence_stage],
            required_intended_use: envelope.semantics.intended_use,
            required_subject_namespace: Some("fhir/Patient".into()),
            required_subject_binding_namespace: Some(
                "mycelix/patient-subject-binding-evidence/v1".into(),
            ),
            allowed_claim_evidence_namespaces: vec![
                "mycelix/clinical-fact-snapshot/v1".into(),
            ],
            required_claim_evidence_namespaces: vec![
                "mycelix/clinical-fact-snapshot/v1".into(),
            ],
            required_producer_distribution_namespace: Some(
                "symthaea/ood-detector-evidence/v1".into(),
            ),
            max_inference_age_micros: 10_000,
            max_future_skew_micros: 100,
            require_calibrated_probability: true,
            qualification_ceiling: SymthaeaAdmissionCeilingV2::ShadowClinical,
        }
    }

    #[test]
    fn exact_policy_admits_only_as_preflight() {
        let admitted = admit_symthaea_clinical_inference_v2(&fixture(), &policy(), 1_100).unwrap();
        assert_eq!(
            admitted.qualification_ceiling(),
            SymthaeaAdmissionCeilingV2::ShadowClinical
        );
        assert_eq!(admitted.envelope().statement, "Candidate risk prediction");
    }

    #[test]
    fn model_policy_substitution_is_rejected() {
        let mut policy = policy();
        policy.accepted_model.model.version = "2.0.0".into();
        assert!(matches!(
            admit_symthaea_clinical_inference_v2(&fixture(), &policy, 1_100),
            Err(SymthaeaAdmissionV2Error::ModelIdentityMismatch)
        ));
    }

    #[test]
    fn unapproved_claim_namespace_is_rejected() {
        let mut policy = policy();
        policy.allowed_claim_evidence_namespaces = vec!["mycelix/other/v1".into()];
        policy.required_claim_evidence_namespaces = vec!["mycelix/other/v1".into()];
        assert!(matches!(
            admit_symthaea_clinical_inference_v2(&fixture(), &policy, 1_100),
            Err(SymthaeaAdmissionV2Error::EvidenceNamespaceNotAdmitted(_))
        ));
    }

    #[test]
    fn required_claim_namespace_must_be_observed() {
        let mut policy = policy();
        policy.allowed_claim_evidence_namespaces.push("mycelix/other/v1".into());
        policy.required_claim_evidence_namespaces.push("mycelix/other/v1".into());
        assert!(matches!(
            admit_symthaea_clinical_inference_v2(&fixture(), &policy, 1_100),
            Err(SymthaeaAdmissionV2Error::RequiredEvidenceNamespaceMissing(_))
        ));
    }

    #[test]
    fn stale_inference_is_rejected() {
        let mut policy = policy();
        policy.max_inference_age_micros = 10;
        assert!(matches!(
            admit_symthaea_clinical_inference_v2(&fixture(), &policy, 2_000),
            Err(SymthaeaAdmissionV2Error::StaleInference)
        ));
    }

    #[test]
    fn subject_binding_namespace_is_policy_bound() {
        let mut policy = policy();
        policy.required_subject_binding_namespace = Some("other/binding/v1".into());
        assert!(matches!(
            admit_symthaea_clinical_inference_v2(&fixture(), &policy, 1_100),
            Err(SymthaeaAdmissionV2Error::SubjectBindingNamespaceMismatch)
        ));
    }

    #[test]
    fn clinical_policy_cannot_omit_required_evidence_namespace() {
        let mut policy = policy();
        policy.required_claim_evidence_namespaces.clear();
        assert!(matches!(
            policy.validate(),
            Err(SymthaeaAdmissionV2Error::ClinicalPolicyRequiresEvidenceNamespace)
        ));
    }

    #[test]
    fn policy_digest_binds_namespace_contract() {
        let policy = policy();
        let first = policy.digest().unwrap();
        let mut changed = policy;
        changed.allowed_claim_evidence_namespaces[0] = "mycelix/other/v1".into();
        changed.required_claim_evidence_namespaces[0] = "mycelix/other/v1".into();
        let second = changed.digest().unwrap();
        assert_ne!(first, second);
    }
}
