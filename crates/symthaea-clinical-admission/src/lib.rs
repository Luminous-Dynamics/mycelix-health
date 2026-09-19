#![deny(unsafe_code)]
//! Deployment-scoped admission preflight for Symthaea clinical inference artifacts.
//!
//! This crate is deliberately downstream of the independent wire verifier. It
//! answers a narrower question than clinical promotion:
//!
//! > Does this exact canonical Symthaea v1 inference match the exact engine/model/
//! > schema/runtime/use constraints this Mycelix deployment agreed to admit?
//!
//! A successful preflight is **not** clinical authority. The maximum Mycelix
//! qualification represented here intentionally has no `SupervisedClinical`
//! variant, and nothing in this crate can mint a clinician-presentation permit.

use mycelix_symthaea_clinical_wire::{
    parse_canonical_symthaea_clinical_inference_v1,
    verify_symthaea_clinical_inference_wire_digest_v1, SymthaeaArtifactIdentityV1,
    SymthaeaCalibrationStatusV1, SymthaeaClinicalClaimKindV1,
    SymthaeaClinicalEvidenceStageV1, SymthaeaClinicalInferenceEnvelopeV1,
    SymthaeaClinicalInferenceWireDigestV1, SymthaeaClinicalIntendedUseV1,
    SymthaeaClinicalWireError, SymthaeaDigestV1, SymthaeaDistributionStatusV1,
    SymthaeaMissingEvidenceCriticalityV1, SymthaeaModelIdentityV1,
    SYMTHAEA_CLINICAL_INFERENCE_SCHEMA_VERSION,
    SYMTHAEA_CLINICAL_INFERENCE_WIRE_IDENTITY_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

const POLICY_SCHEMA_VERSION: u16 = 1;
const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000; // five minutes
const MAX_INFERENCE_AGE_MICROS: i64 = 31_536_000_000_000; // 365 days
const POLICY_DERIVE_KEY_CONTEXT: &str = "mycelix.health.symthaea-clinical-admission-policy.v1";
const RECEIPT_DERIVE_KEY_CONTEXT: &str = "mycelix.health.symthaea-clinical-admission-preflight.v1";

/// Hard ceiling owned by Mycelix admission policy, never derived from a
/// Symthaea evidence-stage label.
///
/// There is intentionally no `SupervisedClinical` variant.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SymthaeaAdmissionCeilingV1 {
    Experimental,
    ValidatedOffline,
    ShadowClinical,
}

/// Exact deployment policy for admitting one class of Symthaea clinical wire
/// artifacts into Mycelix evidence workflows.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SymthaeaClinicalAdmissionPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub accepted_wire_identity_version: u16,
    pub accepted_envelope_schema_version: u16,
    pub accepted_engine: SymthaeaArtifactIdentityV1,
    pub accepted_model: SymthaeaModelIdentityV1,
    pub accepted_runtime_digest: SymthaeaDigestV1,
    pub accepted_configuration_digest: SymthaeaDigestV1,
    pub accepted_operation: String,
    pub allowed_claim_kinds: Vec<SymthaeaClinicalClaimKindV1>,
    pub allowed_evidence_stages: Vec<SymthaeaClinicalEvidenceStageV1>,
    pub required_intended_use: SymthaeaClinicalIntendedUseV1,
    pub required_subject_namespace: Option<String>,
    pub max_inference_age_micros: i64,
    pub max_future_skew_micros: i64,
    pub require_calibrated_probability: bool,
    pub qualification_ceiling: SymthaeaAdmissionCeilingV1,
}

impl SymthaeaClinicalAdmissionPolicyV1 {
    pub fn validate(&self) -> Result<(), SymthaeaAdmissionError> {
        if self.schema_version != POLICY_SCHEMA_VERSION {
            return Err(SymthaeaAdmissionError::UnsupportedPolicyVersion(
                self.schema_version,
            ));
        }
        require_nonempty(&self.policy_id, "policy_id")?;
        if self.accepted_wire_identity_version
            != SYMTHAEA_CLINICAL_INFERENCE_WIRE_IDENTITY_VERSION
        {
            return Err(SymthaeaAdmissionError::UnsupportedWireIdentityVersion(
                self.accepted_wire_identity_version,
            ));
        }
        if self.accepted_envelope_schema_version
            != SYMTHAEA_CLINICAL_INFERENCE_SCHEMA_VERSION
        {
            return Err(SymthaeaAdmissionError::UnsupportedEnvelopeSchemaVersion(
                self.accepted_envelope_schema_version,
            ));
        }
        validate_artifact(&self.accepted_engine)?;
        validate_model(&self.accepted_model)?;
        validate_digest(&self.accepted_runtime_digest)?;
        validate_digest(&self.accepted_configuration_digest)?;
        require_nonempty(&self.accepted_operation, "accepted_operation")?;
        validate_nonempty_unique(&self.allowed_claim_kinds, SymthaeaAdmissionError::MissingClaimKinds, SymthaeaAdmissionError::DuplicateClaimKind)?;
        validate_nonempty_unique(&self.allowed_evidence_stages, SymthaeaAdmissionError::MissingEvidenceStages, SymthaeaAdmissionError::DuplicateEvidenceStage)?;

        if let Some(namespace) = &self.required_subject_namespace {
            require_nonempty(namespace, "required_subject_namespace")?;
        }
        if self.required_intended_use == SymthaeaClinicalIntendedUseV1::ClinicalDecisionSupport
            && self.required_subject_namespace.is_none()
        {
            return Err(SymthaeaAdmissionError::ClinicalPolicyRequiresSubjectNamespace);
        }
        if self.max_inference_age_micros <= 0
            || self.max_inference_age_micros > MAX_INFERENCE_AGE_MICROS
        {
            return Err(SymthaeaAdmissionError::InvalidMaximumInferenceAge);
        }
        if self.max_future_skew_micros < 0
            || self.max_future_skew_micros > MAX_FUTURE_SKEW_MICROS
        {
            return Err(SymthaeaAdmissionError::InvalidMaximumFutureSkew);
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<SymthaeaAdmissionPolicyDigestV1, SymthaeaAdmissionError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|_| SymthaeaAdmissionError::PolicySerializationFailure)?;
        let mut hasher = blake3::Hasher::new_derive_key(POLICY_DERIVE_KEY_CONTEXT);
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
        Ok(SymthaeaAdmissionPolicyDigestV1(*hasher.finalize().as_bytes()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymthaeaAdmissionPolicyDigestV1([u8; 32]);

impl SymthaeaAdmissionPolicyDigestV1 {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymthaeaAdmissionPreflightDigestV1([u8; 32]);

impl SymthaeaAdmissionPreflightDigestV1 {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Non-serializable, single-owner proof that one exact canonical Symthaea wire
/// artifact satisfied one exact Mycelix admission policy at one evaluation time.
///
/// This remains preflight evidence. It is not a clinical authority capability.
pub struct AdmittedSymthaeaInferenceV1 {
    envelope: SymthaeaClinicalInferenceEnvelopeV1,
    wire_digest: SymthaeaClinicalInferenceWireDigestV1,
    policy_digest: SymthaeaAdmissionPolicyDigestV1,
    preflight_digest: SymthaeaAdmissionPreflightDigestV1,
    qualification_ceiling: SymthaeaAdmissionCeilingV1,
    evaluated_at_micros: i64,
}

impl AdmittedSymthaeaInferenceV1 {
    pub fn envelope(&self) -> &SymthaeaClinicalInferenceEnvelopeV1 {
        &self.envelope
    }

    pub fn wire_digest(&self) -> SymthaeaClinicalInferenceWireDigestV1 {
        self.wire_digest
    }

    pub fn policy_digest(&self) -> SymthaeaAdmissionPolicyDigestV1 {
        self.policy_digest
    }

    pub fn preflight_digest(&self) -> SymthaeaAdmissionPreflightDigestV1 {
        self.preflight_digest
    }

    pub fn qualification_ceiling(&self) -> SymthaeaAdmissionCeilingV1 {
        self.qualification_ceiling
    }

    pub fn evaluated_at_micros(&self) -> i64 {
        self.evaluated_at_micros
    }
}

#[derive(Serialize)]
struct PreflightDigestMaterial {
    wire_digest: [u8; 32],
    policy_digest: [u8; 32],
    qualification_ceiling: SymthaeaAdmissionCeilingV1,
    evaluated_at_micros: i64,
}

/// Verify canonical Symthaea bytes and apply exact Mycelix deployment admission
/// policy. No clinical promotion occurs here.
pub fn admit_symthaea_clinical_inference_v1(
    wire_bytes: &[u8],
    policy: &SymthaeaClinicalAdmissionPolicyV1,
    evaluated_at_micros: i64,
) -> Result<AdmittedSymthaeaInferenceV1, SymthaeaAdmissionError> {
    policy.validate()?;
    let envelope = parse_canonical_symthaea_clinical_inference_v1(wire_bytes)?;
    let wire_digest = verify_symthaea_clinical_inference_wire_digest_v1(wire_bytes)?;

    if envelope.execution.engine != policy.accepted_engine {
        return Err(SymthaeaAdmissionError::EngineIdentityMismatch);
    }
    if envelope.execution.model != policy.accepted_model {
        return Err(SymthaeaAdmissionError::ModelIdentityMismatch);
    }
    if envelope.execution.runtime_digest != policy.accepted_runtime_digest {
        return Err(SymthaeaAdmissionError::RuntimeIdentityMismatch);
    }
    if envelope.execution.configuration_digest != policy.accepted_configuration_digest {
        return Err(SymthaeaAdmissionError::ConfigurationIdentityMismatch);
    }
    if envelope.execution.operation != policy.accepted_operation {
        return Err(SymthaeaAdmissionError::OperationMismatch);
    }
    if !policy.allowed_claim_kinds.contains(&envelope.semantics.claim_kind) {
        return Err(SymthaeaAdmissionError::ClaimKindNotAdmitted);
    }
    if !policy
        .allowed_evidence_stages
        .contains(&envelope.semantics.evidence_stage)
    {
        return Err(SymthaeaAdmissionError::EvidenceStageNotAdmitted);
    }
    if envelope.semantics.intended_use != policy.required_intended_use {
        return Err(SymthaeaAdmissionError::IntendedUseMismatch);
    }

    if let Some(required_namespace) = &policy.required_subject_namespace {
        let subject = envelope
            .subject
            .as_ref()
            .ok_or(SymthaeaAdmissionError::MissingRequiredSubject)?;
        if &subject.namespace != required_namespace {
            return Err(SymthaeaAdmissionError::SubjectNamespaceMismatch);
        }
    }

    if envelope.generated_at_micros < envelope.execution.executed_at_micros {
        return Err(SymthaeaAdmissionError::InvalidTemporalOrder);
    }
    if envelope.generated_at_micros
        > evaluated_at_micros.saturating_add(policy.max_future_skew_micros)
    {
        return Err(SymthaeaAdmissionError::InferenceTooFarInFuture);
    }
    let age = evaluated_at_micros.saturating_sub(envelope.generated_at_micros);
    if age > policy.max_inference_age_micros {
        return Err(SymthaeaAdmissionError::StaleInference);
    }

    bind_claimed_evidence_to_execution(&envelope)?;

    if policy.require_calibrated_probability {
        if envelope.uncertainty.calibration_status != SymthaeaCalibrationStatusV1::Calibrated {
            return Err(SymthaeaAdmissionError::CalibratedInferenceRequired);
        }
        let model_calibration = envelope
            .execution
            .model
            .calibration_evidence_digest
            .ok_or(SymthaeaAdmissionError::ModelCalibrationEvidenceRequired)?;
        let inference_calibration = envelope
            .uncertainty
            .calibration_evidence_digest
            .ok_or(SymthaeaAdmissionError::InferenceCalibrationEvidenceRequired)?;
        if model_calibration != inference_calibration {
            return Err(SymthaeaAdmissionError::CalibrationEvidenceMismatch);
        }
    }

    if envelope.semantics.intended_use == SymthaeaClinicalIntendedUseV1::ClinicalDecisionSupport {
        if envelope.missing_evidence.iter().any(|missing| {
            missing.criticality == SymthaeaMissingEvidenceCriticalityV1::Critical
        }) {
            return Err(SymthaeaAdmissionError::CriticalEvidenceMissing);
        }
        if envelope.distribution.status == SymthaeaDistributionStatusV1::OutOfDistribution {
            return Err(SymthaeaAdmissionError::ProducerReportsOutOfDistribution);
        }
    }

    let policy_digest = policy.digest()?;
    let material = PreflightDigestMaterial {
        wire_digest: wire_digest.into_bytes(),
        policy_digest: policy_digest.0,
        qualification_ceiling: policy.qualification_ceiling,
        evaluated_at_micros,
    };
    let material_bytes = serde_json::to_vec(&material)
        .map_err(|_| SymthaeaAdmissionError::PreflightSerializationFailure)?;
    let mut hasher = blake3::Hasher::new_derive_key(RECEIPT_DERIVE_KEY_CONTEXT);
    hasher.update(&(material_bytes.len() as u64).to_be_bytes());
    hasher.update(&material_bytes);
    let preflight_digest = SymthaeaAdmissionPreflightDigestV1(*hasher.finalize().as_bytes());

    Ok(AdmittedSymthaeaInferenceV1 {
        envelope,
        wire_digest,
        policy_digest,
        preflight_digest,
        qualification_ceiling: policy.qualification_ceiling,
        evaluated_at_micros,
    })
}

fn bind_claimed_evidence_to_execution(
    envelope: &SymthaeaClinicalInferenceEnvelopeV1,
) -> Result<(), SymthaeaAdmissionError> {
    let inputs = &envelope.execution.input_evidence_digests;
    for evidence in &envelope.evidence {
        if !inputs.contains(&evidence.digest) {
            return Err(SymthaeaAdmissionError::EvidenceNotExecutionBound);
        }
    }
    if let Some(subject) = &envelope.subject {
        if !inputs.contains(&subject.binding_evidence_digest) {
            return Err(SymthaeaAdmissionError::SubjectBindingNotExecutionBound);
        }
    }
    Ok(())
}

fn validate_artifact(
    artifact: &SymthaeaArtifactIdentityV1,
) -> Result<(), SymthaeaAdmissionError> {
    require_nonempty(&artifact.name, "accepted artifact name")?;
    require_nonempty(&artifact.version, "accepted artifact version")?;
    validate_digest(&artifact.digest)
}

fn validate_model(model: &SymthaeaModelIdentityV1) -> Result<(), SymthaeaAdmissionError> {
    validate_artifact(&model.model)?;
    validate_digest(&model.input_schema_digest)?;
    validate_digest(&model.output_schema_digest)?;
    for digest in [
        model.training_lineage_digest,
        model.evaluation_lineage_digest,
        model.calibration_evidence_digest,
    ]
    .into_iter()
    .flatten()
    {
        validate_digest(&digest)?;
    }
    Ok(())
}

fn validate_digest(digest: &SymthaeaDigestV1) -> Result<(), SymthaeaAdmissionError> {
    if digest.value == [0u8; 32] {
        return Err(SymthaeaAdmissionError::ZeroPolicyDigestComponent);
    }
    Ok(())
}

fn validate_nonempty_unique<T>(
    values: &[T],
    missing: SymthaeaAdmissionError,
    duplicate: SymthaeaAdmissionError,
) -> Result<(), SymthaeaAdmissionError>
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

fn require_nonempty(value: &str, field: &'static str) -> Result<(), SymthaeaAdmissionError> {
    if value.trim().is_empty() {
        return Err(SymthaeaAdmissionError::MissingField(field));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum SymthaeaAdmissionError {
    #[error(transparent)]
    Wire(#[from] SymthaeaClinicalWireError),
    #[error("unsupported admission policy version {0}")]
    UnsupportedPolicyVersion(u16),
    #[error("unsupported Symthaea wire identity version {0}")]
    UnsupportedWireIdentityVersion(u16),
    #[error("unsupported Symthaea envelope schema version {0}")]
    UnsupportedEnvelopeSchemaVersion(u16),
    #[error("required policy field is empty: {0}")]
    MissingField(&'static str),
    #[error("policy digest component cannot be zero")]
    ZeroPolicyDigestComponent,
    #[error("at least one claim kind must be admitted")]
    MissingClaimKinds,
    #[error("duplicate admitted claim kind")]
    DuplicateClaimKind,
    #[error("at least one evidence stage must be admitted")]
    MissingEvidenceStages,
    #[error("duplicate admitted evidence stage")]
    DuplicateEvidenceStage,
    #[error("clinical-decision-support admission requires a subject namespace")]
    ClinicalPolicyRequiresSubjectNamespace,
    #[error("maximum inference age is invalid")]
    InvalidMaximumInferenceAge,
    #[error("maximum future skew is invalid")]
    InvalidMaximumFutureSkew,
    #[error("policy could not be serialized")]
    PolicySerializationFailure,
    #[error("preflight receipt material could not be serialized")]
    PreflightSerializationFailure,
    #[error("Symthaea engine identity does not match deployment policy")]
    EngineIdentityMismatch,
    #[error("Symthaea model/schema/lineage identity does not match deployment policy")]
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
    #[error("inference generation predates execution")]
    InvalidTemporalOrder,
    #[error("inference is dated beyond allowed future skew")]
    InferenceTooFarInFuture,
    #[error("inference is older than deployment policy allows")]
    StaleInference,
    #[error("claimed evidence is not bound to execution inputs")]
    EvidenceNotExecutionBound,
    #[error("subject binding evidence is not bound to execution inputs")]
    SubjectBindingNotExecutionBound,
    #[error("deployment policy requires calibrated probability")]
    CalibratedInferenceRequired,
    #[error("model identity lacks required calibration evidence")]
    ModelCalibrationEvidenceRequired,
    #[error("inference uncertainty lacks required calibration evidence")]
    InferenceCalibrationEvidenceRequired,
    #[error("model and inference calibration evidence do not match")]
    CalibrationEvidenceMismatch,
    #[error("critical evidence is missing for clinical decision support")]
    CriticalEvidenceMissing,
    #[error("producer explicitly reports the inference as out-of-distribution")]
    ProducerReportsOutOfDistribution,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_symthaea_clinical_wire::{
        SymthaeaClinicalApplicabilityV1, SymthaeaClinicalClaimSemanticsV1,
        SymthaeaClinicalEvidenceRoleV1, SymthaeaDistributionAssessmentV1,
        SymthaeaExecutionIdentityV1, SymthaeaMissingClinicalEvidenceV1,
        SymthaeaSubjectBindingV1,
    };

    const VECTOR_V1: &[u8] = include_bytes!(
        "../../symthaea-clinical-wire/fixtures/clinical_inference_wire_v1.json"
    );

    fn parsed_vector() -> SymthaeaClinicalInferenceEnvelopeV1 {
        parse_canonical_symthaea_clinical_inference_v1(VECTOR_V1).unwrap()
    }

    fn policy_for(envelope: &SymthaeaClinicalInferenceEnvelopeV1) -> SymthaeaClinicalAdmissionPolicyV1 {
        SymthaeaClinicalAdmissionPolicyV1 {
            schema_version: 1,
            policy_id: "symthaea-clinical-admission-test-v1".into(),
            accepted_wire_identity_version: 1,
            accepted_envelope_schema_version: 1,
            accepted_engine: envelope.execution.engine.clone(),
            accepted_model: envelope.execution.model.clone(),
            accepted_runtime_digest: envelope.execution.runtime_digest,
            accepted_configuration_digest: envelope.execution.configuration_digest,
            accepted_operation: envelope.execution.operation.clone(),
            allowed_claim_kinds: vec![envelope.semantics.claim_kind],
            allowed_evidence_stages: vec![envelope.semantics.evidence_stage],
            required_intended_use: envelope.semantics.intended_use,
            required_subject_namespace: Some("fhir/Patient".into()),
            max_inference_age_micros: 10_000,
            max_future_skew_micros: 100,
            require_calibrated_probability: true,
            qualification_ceiling: SymthaeaAdmissionCeilingV1::ShadowClinical,
        }
    }

    fn canonical_bytes(envelope: &SymthaeaClinicalInferenceEnvelopeV1) -> Vec<u8> {
        serde_json::to_vec(envelope).unwrap()
    }

    fn execution_input_bound_vector() -> SymthaeaClinicalInferenceEnvelopeV1 {
        let mut envelope = parsed_vector();
        let subject_digest = envelope.subject.as_ref().unwrap().binding_evidence_digest;
        envelope.execution.input_evidence_digests.push(subject_digest);
        envelope
    }

    #[test]
    fn exact_policy_admits_only_as_preflight() {
        let envelope = execution_input_bound_vector();
        let policy = policy_for(&envelope);
        let bytes = canonical_bytes(&envelope);
        let admitted = admit_symthaea_clinical_inference_v1(&bytes, &policy, 1_100).unwrap();
        assert_eq!(
            admitted.qualification_ceiling(),
            SymthaeaAdmissionCeilingV1::ShadowClinical
        );
        assert_eq!(admitted.envelope().statement, "Candidate risk prediction");
    }

    #[test]
    fn stronger_symthaea_evidence_label_does_not_raise_mycelix_ceiling() {
        let mut envelope = execution_input_bound_vector();
        envelope.semantics.evidence_stage = SymthaeaClinicalEvidenceStageV1::ReplicatedClinicalEvidence;
        let mut policy = policy_for(&envelope);
        policy.allowed_evidence_stages = vec![SymthaeaClinicalEvidenceStageV1::ReplicatedClinicalEvidence];
        policy.qualification_ceiling = SymthaeaAdmissionCeilingV1::ValidatedOffline;
        let admitted = admit_symthaea_clinical_inference_v1(
            &canonical_bytes(&envelope),
            &policy,
            1_100,
        )
        .unwrap();
        assert_eq!(
            admitted.qualification_ceiling(),
            SymthaeaAdmissionCeilingV1::ValidatedOffline
        );
    }

    #[test]
    fn model_substitution_is_rejected() {
        let mut envelope = execution_input_bound_vector();
        let policy = policy_for(&envelope);
        envelope.execution.model.model.version = "2.0.0".into();
        assert!(matches!(
            admit_symthaea_clinical_inference_v1(&canonical_bytes(&envelope), &policy, 1_100),
            Err(SymthaeaAdmissionError::ModelIdentityMismatch)
        ));
    }

    #[test]
    fn runtime_substitution_is_rejected() {
        let mut envelope = execution_input_bound_vector();
        let policy = policy_for(&envelope);
        envelope.execution.runtime_digest.value = [99u8; 32];
        assert!(matches!(
            admit_symthaea_clinical_inference_v1(&canonical_bytes(&envelope), &policy, 1_100),
            Err(SymthaeaAdmissionError::RuntimeIdentityMismatch)
        ));
    }

    #[test]
    fn stale_inference_is_rejected() {
        let envelope = execution_input_bound_vector();
        let mut policy = policy_for(&envelope);
        policy.max_inference_age_micros = 10;
        assert!(matches!(
            admit_symthaea_clinical_inference_v1(&canonical_bytes(&envelope), &policy, 2_000),
            Err(SymthaeaAdmissionError::StaleInference)
        ));
    }

    #[test]
    fn subject_namespace_substitution_is_rejected() {
        let mut envelope = execution_input_bound_vector();
        let policy = policy_for(&envelope);
        envelope.subject.as_mut().unwrap().namespace = "local/Patient".into();
        assert!(matches!(
            admit_symthaea_clinical_inference_v1(&canonical_bytes(&envelope), &policy, 1_100),
            Err(SymthaeaAdmissionError::SubjectNamespaceMismatch)
        ));
    }

    #[test]
    fn post_hoc_evidence_not_used_by_execution_is_rejected() {
        let mut envelope = execution_input_bound_vector();
        let policy = policy_for(&envelope);
        envelope.evidence.push(mycelix_symthaea_clinical_wire::SymthaeaClinicalEvidenceRefV1 {
            evidence_id: "post-hoc".into(),
            digest: SymthaeaDigestV1 {
                algorithm: mycelix_symthaea_clinical_wire::SymthaeaDigestAlgorithmV1::Blake3_256,
                value: [44u8; 32],
            },
            role: SymthaeaClinicalEvidenceRoleV1::Supports,
        });
        assert!(matches!(
            admit_symthaea_clinical_inference_v1(&canonical_bytes(&envelope), &policy, 1_100),
            Err(SymthaeaAdmissionError::EvidenceNotExecutionBound)
        ));
    }

    #[test]
    fn subject_binding_must_be_execution_bound() {
        let envelope = parsed_vector();
        let policy = policy_for(&envelope);
        assert!(matches!(
            admit_symthaea_clinical_inference_v1(&canonical_bytes(&envelope), &policy, 1_100),
            Err(SymthaeaAdmissionError::SubjectBindingNotExecutionBound)
        ));
    }

    #[test]
    fn calibration_lineage_mismatch_is_rejected() {
        let mut envelope = execution_input_bound_vector();
        let policy = policy_for(&envelope);
        envelope.execution.model.calibration_evidence_digest = Some(SymthaeaDigestV1 {
            algorithm: mycelix_symthaea_clinical_wire::SymthaeaDigestAlgorithmV1::Blake3_256,
            value: [55u8; 32],
        });
        assert!(matches!(
            admit_symthaea_clinical_inference_v1(&canonical_bytes(&envelope), &policy, 1_100),
            Err(SymthaeaAdmissionError::ModelIdentityMismatch)
        ));
    }

    #[test]
    fn critical_missing_evidence_blocks_clinical_preflight() {
        let mut envelope = execution_input_bound_vector();
        let policy = policy_for(&envelope);
        envelope.missing_evidence.push(SymthaeaMissingClinicalEvidenceV1 {
            requirement_id: "renal-function".into(),
            description: "renal function is required".into(),
            criticality: SymthaeaMissingEvidenceCriticalityV1::Critical,
        });
        assert!(matches!(
            admit_symthaea_clinical_inference_v1(&canonical_bytes(&envelope), &policy, 1_100),
            Err(SymthaeaAdmissionError::CriticalEvidenceMissing)
        ));
    }

    #[test]
    fn producer_reported_ood_blocks_clinical_preflight() {
        let mut envelope = execution_input_bound_vector();
        let policy = policy_for(&envelope);
        envelope.distribution.status = SymthaeaDistributionStatusV1::OutOfDistribution;
        assert!(matches!(
            admit_symthaea_clinical_inference_v1(&canonical_bytes(&envelope), &policy, 1_100),
            Err(SymthaeaAdmissionError::ProducerReportsOutOfDistribution)
        ));
    }
}
