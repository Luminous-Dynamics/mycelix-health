#![deny(unsafe_code)]
//! Verified non-authorizing model assertions from Symthaea clinical evidence context.
//!
//! A model prediction is not a four-state clinical criterion. This crate preserves
//! model output semantics (probability, uncertainty, missingness, alternatives and
//! producer distribution evidence) without converting them to
//! `Satisfied/NotSatisfied/Indeterminate/NotApplicable`.
//!
//! Conversion to a clinical criterion is intentionally deferred to a separately
//! qualified decision-rule/threshold policy.

use mycelix_clinical_evidence_v2::TypedFactEvidenceV1;
use mycelix_symthaea_clinical_admission_v2::{
    AdmittedSymthaeaInferenceV2, SymthaeaAdmissionCeilingV2,
    SymthaeaAdmissionPolicyDigestV2,
};
use mycelix_symthaea_clinical_evidence_context::{
    SymthaeaEvidenceContextDigestV1, VerifiedSymthaeaEvidenceContextV1,
};
use mycelix_symthaea_clinical_wire_v2::{
    SymthaeaAlternativeV2, SymthaeaApplicabilityV2, SymthaeaCalibrationStatusV2,
    SymthaeaClaimKindV2, SymthaeaDistributionAssessmentV2, SymthaeaEvidenceStageV2,
    SymthaeaIntendedUseV2, SymthaeaMissingEvidenceV2, SymthaeaModelIdentityV2,
    SymthaeaUncertaintyV2, VerifiedSymthaeaWireDigestV2,
};
use thiserror::Error;

const ASSERTION_DERIVE_KEY: &str = "mycelix.health.symthaea-clinical-model-assertion.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelAssertionKindV1 {
    Prediction,
    RiskEstimate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymthaeaModelAssertionDigestV1([u8; 32]);

impl SymthaeaModelAssertionDigestV1 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Non-serializable, non-authorizing model assertion.
///
/// This artifact intentionally has no `EvaluationState`, no presentation flag,
/// no recommendation authority and no therapeutic action field.
pub struct VerifiedSymthaeaModelAssertionV1 {
    wire_digest: VerifiedSymthaeaWireDigestV2,
    context_digest: SymthaeaEvidenceContextDigestV1,
    admission_policy_digest: SymthaeaAdmissionPolicyDigestV2,
    assertion_digest: SymthaeaModelAssertionDigestV1,
    qualification_ceiling: SymthaeaAdmissionCeilingV2,
    subject_id: String,
    kind: ModelAssertionKindV1,
    statement: String,
    evidence_stage: SymthaeaEvidenceStageV2,
    applicability: SymthaeaApplicabilityV2,
    intended_use: SymthaeaIntendedUseV2,
    uncertainty: SymthaeaUncertaintyV2,
    missing_evidence: Vec<SymthaeaMissingEvidenceV2>,
    alternatives: Vec<SymthaeaAlternativeV2>,
    producer_distribution: SymthaeaDistributionAssessmentV2,
    model: SymthaeaModelIdentityV2,
    operation: String,
    generated_at_micros: i64,
    typed_fact_evidence: Vec<TypedFactEvidenceV1>,
}

impl VerifiedSymthaeaModelAssertionV1 {
    pub fn wire_digest(&self) -> VerifiedSymthaeaWireDigestV2 {
        self.wire_digest
    }

    pub fn context_digest(&self) -> SymthaeaEvidenceContextDigestV1 {
        self.context_digest
    }

    pub fn admission_policy_digest(&self) -> SymthaeaAdmissionPolicyDigestV2 {
        self.admission_policy_digest
    }

    pub fn assertion_digest(&self) -> SymthaeaModelAssertionDigestV1 {
        self.assertion_digest
    }

    pub fn qualification_ceiling(&self) -> SymthaeaAdmissionCeilingV2 {
        self.qualification_ceiling
    }

    pub fn subject_id(&self) -> &str {
        &self.subject_id
    }

    pub fn kind(&self) -> ModelAssertionKindV1 {
        self.kind
    }

    pub fn statement(&self) -> &str {
        &self.statement
    }

    pub fn evidence_stage(&self) -> SymthaeaEvidenceStageV2 {
        self.evidence_stage
    }

    pub fn applicability(&self) -> SymthaeaApplicabilityV2 {
        self.applicability
    }

    pub fn intended_use(&self) -> SymthaeaIntendedUseV2 {
        self.intended_use
    }

    pub fn uncertainty(&self) -> &SymthaeaUncertaintyV2 {
        &self.uncertainty
    }

    pub fn missing_evidence(&self) -> &[SymthaeaMissingEvidenceV2] {
        &self.missing_evidence
    }

    pub fn alternatives(&self) -> &[SymthaeaAlternativeV2] {
        &self.alternatives
    }

    /// Producer-reported distribution evidence only.
    ///
    /// This is not independent Mycelix OOD trust and must never be used as an
    /// authorization predicate by itself.
    pub fn producer_distribution(&self) -> &SymthaeaDistributionAssessmentV2 {
        &self.producer_distribution
    }

    pub fn model(&self) -> &SymthaeaModelIdentityV2 {
        &self.model
    }

    pub fn operation(&self) -> &str {
        &self.operation
    }

    pub fn generated_at_micros(&self) -> i64 {
        self.generated_at_micros
    }

    pub fn typed_fact_evidence(&self) -> &[TypedFactEvidenceV1] {
        &self.typed_fact_evidence
    }
}

/// Construct a verified model assertion without inventing a clinical criterion
/// state. Both inputs must refer to the same exact admitted inference.
pub fn build_verified_symthaea_model_assertion_v1(
    context: &VerifiedSymthaeaEvidenceContextV1,
    admission: &AdmittedSymthaeaInferenceV2,
) -> Result<VerifiedSymthaeaModelAssertionV1, SymthaeaModelAssertionError> {
    if context.wire_digest() != admission.wire_digest() {
        return Err(SymthaeaModelAssertionError::WireDigestMismatch);
    }
    if context.admission_policy_digest() != admission.policy_digest() {
        return Err(SymthaeaModelAssertionError::AdmissionPolicyMismatch);
    }

    let envelope = admission.envelope();
    let subject = envelope
        .subject
        .as_ref()
        .ok_or(SymthaeaModelAssertionError::SubjectRequired)?;
    if context.subject_id() != subject.subject_id {
        return Err(SymthaeaModelAssertionError::SubjectMismatch);
    }
    if context.statement() != envelope.statement {
        return Err(SymthaeaModelAssertionError::StatementMismatch);
    }
    if context.claim_kind() != envelope.semantics.claim_kind
        || context.evidence_stage() != envelope.semantics.evidence_stage
        || context.intended_use() != envelope.semantics.intended_use
    {
        return Err(SymthaeaModelAssertionError::SemanticContextMismatch);
    }

    let kind = match envelope.semantics.claim_kind {
        SymthaeaClaimKindV2::Prediction => ModelAssertionKindV1::Prediction,
        SymthaeaClaimKindV2::RiskEstimate => ModelAssertionKindV1::RiskEstimate,
        _ => return Err(SymthaeaModelAssertionError::UnsupportedModelAssertionKind),
    };

    if context.typed_fact_evidence().is_empty() {
        return Err(SymthaeaModelAssertionError::MissingVerifiedFactEvidence);
    }

    // Clinical decision-support predictions must retain calibrated numerical
    // uncertainty before any later threshold/decision-rule projection.
    if envelope.semantics.intended_use == SymthaeaIntendedUseV2::ClinicalDecisionSupport
        && (envelope.uncertainty.calibration_status != SymthaeaCalibrationStatusV2::Calibrated
            || envelope.uncertainty.calibrated_probability.is_none())
    {
        return Err(SymthaeaModelAssertionError::ClinicalPredictionMustBeCalibrated);
    }

    let assertion_digest = assertion_digest(
        admission.wire_digest(),
        context.context_digest(),
        admission.policy_digest(),
        kind,
        &subject.subject_id,
        envelope.generated_at_micros,
    );

    Ok(VerifiedSymthaeaModelAssertionV1 {
        wire_digest: admission.wire_digest(),
        context_digest: context.context_digest(),
        admission_policy_digest: admission.policy_digest(),
        assertion_digest,
        qualification_ceiling: admission.qualification_ceiling(),
        subject_id: subject.subject_id.clone(),
        kind,
        statement: envelope.statement.clone(),
        evidence_stage: envelope.semantics.evidence_stage,
        applicability: envelope.semantics.applicability,
        intended_use: envelope.semantics.intended_use,
        uncertainty: envelope.uncertainty.clone(),
        missing_evidence: envelope.missing_evidence.clone(),
        alternatives: envelope.alternatives.clone(),
        producer_distribution: envelope.distribution.clone(),
        model: envelope.execution.model.clone(),
        operation: envelope.execution.operation.clone(),
        generated_at_micros: envelope.generated_at_micros,
        typed_fact_evidence: context.typed_fact_evidence().to_vec(),
    })
}

fn assertion_digest(
    wire: VerifiedSymthaeaWireDigestV2,
    context: SymthaeaEvidenceContextDigestV1,
    policy: SymthaeaAdmissionPolicyDigestV2,
    kind: ModelAssertionKindV1,
    subject_id: &str,
    generated_at_micros: i64,
) -> SymthaeaModelAssertionDigestV1 {
    let mut hasher = blake3::Hasher::new_derive_key(ASSERTION_DERIVE_KEY);
    hasher.update(wire.as_bytes());
    hasher.update(context.as_bytes());
    hasher.update(policy.as_bytes());
    hasher.update(&[match kind {
        ModelAssertionKindV1::Prediction => 0,
        ModelAssertionKindV1::RiskEstimate => 1,
    }]);
    hasher.update(&(subject_id.len() as u32).to_be_bytes());
    hasher.update(subject_id.as_bytes());
    hasher.update(&generated_at_micros.to_be_bytes());
    SymthaeaModelAssertionDigestV1(*hasher.finalize().as_bytes())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SymthaeaModelAssertionError {
    #[error("evidence context and admission refer to different Symthaea wire artifacts")]
    WireDigestMismatch,
    #[error("evidence context and admission use different deployment admission policies")]
    AdmissionPolicyMismatch,
    #[error("model assertion requires a subject")]
    SubjectRequired,
    #[error("evidence context and admitted inference refer to different subjects")]
    SubjectMismatch,
    #[error("evidence context and admitted inference carry different statements")]
    StatementMismatch,
    #[error("evidence context and admitted inference semantic labels differ")]
    SemanticContextMismatch,
    #[error("v1 model assertion supports only Prediction and RiskEstimate claim kinds")]
    UnsupportedModelAssertionKind,
    #[error("verified model assertion requires at least one independently bound ClinicalFact")]
    MissingVerifiedFactEvidence,
    #[error("clinical-decision-support prediction requires calibrated probability")]
    ClinicalPredictionMustBeCalibrated,
}
