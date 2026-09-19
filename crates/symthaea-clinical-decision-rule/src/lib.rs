#![deny(unsafe_code)]
//! Explicit decision-rule semantics for verified Symthaea model assertions.
//!
//! A calibrated prediction is not itself a clinical criterion. This crate makes
//! the conversion explicit: one exact model assertion may be evaluated only under
//! one exact, evidence-bound threshold policy. The output is a criterion candidate
//! (`Triggered`, `NotTriggered`, or `Indeterminate`), not a recommendation,
//! presentation permit, diagnosis, or treatment authority.

use mycelix_symthaea_clinical_admission_v2::SymthaeaAdmissionCeilingV2;
use mycelix_symthaea_clinical_model_assertion::{
    ModelAssertionKindV1, SymthaeaModelAssertionDigestV1,
    VerifiedSymthaeaModelAssertionV1,
};
use mycelix_symthaea_clinical_wire_v2::{
    SymthaeaApplicabilityV2, SymthaeaCalibrationStatusV2, SymthaeaDigestAlgorithmV2,
    SymthaeaEvidenceIdentityV2, SymthaeaEvidenceStageV2, SymthaeaIntendedUseV2,
    SymthaeaMissingCriticalityV2, SymthaeaModelIdentityV2,
    SYMTHAEA_EVIDENCE_IDENTITY_VERSION,
};
use std::collections::HashSet;
use thiserror::Error;

pub const CLINICAL_DECISION_RULE_POLICY_VERSION: u16 = 1;
pub const DECISION_RULE_EVIDENCE_IDENTITY_VERSION: u16 = 1;
const MAX_ASSERTION_AGE_MICROS: i64 = 31_536_000_000_000; // 365 days
const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000; // 5 minutes
const MAX_PREDICTION_HORIZON_MICROS: i64 = 315_576_000_000_000; // 10 years
const POLICY_DERIVE_KEY: &str = "mycelix.health.symthaea-clinical-decision-rule-policy.v1";
const RESULT_DERIVE_KEY: &str = "mycelix.health.symthaea-clinical-decision-rule-result.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProbabilityComparatorV1 {
    GreaterThanOrEqual,
    GreaterThan,
    LessThanOrEqual,
    LessThan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MissingEvidencePolicyV1 {
    /// Any producer-declared missing evidence forces abstention.
    RequireNone,
    /// Important or critical missing evidence forces abstention.
    IndeterminateOnImportantOrCritical,
    /// Only critical missing evidence forces abstention.
    IndeterminateOnCriticalOnly,
}

/// Mycelix-owned evidence identity for the clinical decision rule itself.
///
/// These identities describe validation/operating-characteristic artifacts used
/// to qualify the threshold policy. Their presence proves binding only, not that
/// the underlying studies are scientifically sufficient.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DecisionRuleEvidenceIdentityV1 {
    pub schema_version: u16,
    pub namespace: String,
    pub artifact_id: String,
    pub digest: [u8; 32],
}

impl DecisionRuleEvidenceIdentityV1 {
    pub fn validate(&self) -> Result<(), ClinicalDecisionRuleError> {
        if self.schema_version != DECISION_RULE_EVIDENCE_IDENTITY_VERSION {
            return Err(ClinicalDecisionRuleError::UnsupportedEvidenceIdentityVersion(
                self.schema_version,
            ));
        }
        validate_token(&self.namespace)?;
        validate_token(&self.artifact_id)?;
        if self.digest == [0u8; 32] {
            return Err(ClinicalDecisionRuleError::ZeroEvidenceDigest);
        }
        Ok(())
    }
}

/// Explicit threshold policy for one exact model and clinical endpoint.
///
/// There is intentionally no `SupervisedClinical` authority beyond the admission
/// ceiling type already provided by the Symthaea admission layer. v1 cannot raise
/// authority above the exact model assertion it evaluates.
#[derive(Clone, Debug, PartialEq)]
pub struct ClinicalDecisionRulePolicyV1 {
    pub schema_version: u16,
    pub rule_id: String,
    pub accepted_assertion_kind: ModelAssertionKindV1,
    pub required_model: SymthaeaModelIdentityV2,
    pub endpoint_namespace: String,
    pub endpoint_id: String,
    pub prediction_horizon_micros: i64,
    pub target_population: String,
    pub care_setting: String,
    pub required_intended_use: SymthaeaIntendedUseV2,
    pub required_minimum_applicability: SymthaeaApplicabilityV2,
    pub allowed_evidence_stages: Vec<SymthaeaEvidenceStageV2>,
    pub comparator: ProbabilityComparatorV1,
    pub probability_threshold: f64,
    /// Exact producer calibration evidence already bound into the model assertion.
    pub required_calibration_evidence: SymthaeaEvidenceIdentityV2,
    /// Independent artifact supporting threshold operating characteristics.
    pub operating_characteristics_evidence: DecisionRuleEvidenceIdentityV1,
    /// At least one external validation/replication artifact is required.
    pub external_validation_evidence: Vec<DecisionRuleEvidenceIdentityV1>,
    pub missing_evidence_policy: MissingEvidencePolicyV1,
    pub max_assertion_age_micros: i64,
    pub max_future_skew_micros: i64,
    pub qualification_ceiling: SymthaeaAdmissionCeilingV2,
}

impl ClinicalDecisionRulePolicyV1 {
    pub fn validate(&self) -> Result<(), ClinicalDecisionRuleError> {
        if self.schema_version != CLINICAL_DECISION_RULE_POLICY_VERSION {
            return Err(ClinicalDecisionRuleError::UnsupportedPolicyVersion(
                self.schema_version,
            ));
        }
        validate_token(&self.rule_id)?;
        validate_model_identity(&self.required_model)?;
        validate_token(&self.endpoint_namespace)?;
        validate_token(&self.endpoint_id)?;
        validate_token(&self.target_population)?;
        validate_token(&self.care_setting)?;
        if self.prediction_horizon_micros <= 0
            || self.prediction_horizon_micros > MAX_PREDICTION_HORIZON_MICROS
        {
            return Err(ClinicalDecisionRuleError::InvalidPredictionHorizon);
        }
        if !self.probability_threshold.is_finite()
            || !(0.0..=1.0).contains(&self.probability_threshold)
        {
            return Err(ClinicalDecisionRuleError::InvalidProbabilityThreshold);
        }
        if self.allowed_evidence_stages.is_empty() {
            return Err(ClinicalDecisionRuleError::MissingAllowedEvidenceStages);
        }
        let mut stages = HashSet::new();
        for stage in &self.allowed_evidence_stages {
            if !stages.insert(*stage) {
                return Err(ClinicalDecisionRuleError::DuplicateAllowedEvidenceStage);
            }
        }
        validate_symthaea_evidence_identity(&self.required_calibration_evidence)?;
        self.operating_characteristics_evidence.validate()?;
        if self.external_validation_evidence.is_empty() {
            return Err(ClinicalDecisionRuleError::MissingExternalValidationEvidence);
        }
        let mut validation_ids = HashSet::new();
        for evidence in &self.external_validation_evidence {
            evidence.validate()?;
            let key = (
                evidence.namespace.as_str(),
                evidence.artifact_id.as_str(),
                evidence.digest,
            );
            if !validation_ids.insert(key) {
                return Err(ClinicalDecisionRuleError::DuplicateExternalValidationEvidence);
            }
        }
        if self.max_assertion_age_micros <= 0
            || self.max_assertion_age_micros > MAX_ASSERTION_AGE_MICROS
        {
            return Err(ClinicalDecisionRuleError::InvalidMaximumAssertionAge);
        }
        if self.max_future_skew_micros < 0
            || self.max_future_skew_micros > MAX_FUTURE_SKEW_MICROS
        {
            return Err(ClinicalDecisionRuleError::InvalidMaximumFutureSkew);
        }
        Ok(())
    }

    pub fn digest(
        &self,
    ) -> Result<ClinicalDecisionRulePolicyDigestV1, ClinicalDecisionRuleError> {
        self.validate()?;
        let mut framed = Vec::new();
        framed.extend_from_slice(&self.schema_version.to_be_bytes());
        append_string(&mut framed, &self.rule_id)?;
        framed.push(assertion_kind_tag(self.accepted_assertion_kind));
        append_model_identity(&mut framed, &self.required_model)?;
        append_string(&mut framed, &self.endpoint_namespace)?;
        append_string(&mut framed, &self.endpoint_id)?;
        framed.extend_from_slice(&self.prediction_horizon_micros.to_be_bytes());
        append_string(&mut framed, &self.target_population)?;
        append_string(&mut framed, &self.care_setting)?;
        framed.push(intended_use_tag(self.required_intended_use));
        framed.push(applicability_tag(self.required_minimum_applicability));
        append_len(&mut framed, self.allowed_evidence_stages.len())?;
        for stage in &self.allowed_evidence_stages {
            framed.push(evidence_stage_tag(*stage));
        }
        framed.push(comparator_tag(self.comparator));
        framed.extend_from_slice(&self.probability_threshold.to_bits().to_be_bytes());
        append_symthaea_evidence_identity(&mut framed, &self.required_calibration_evidence)?;
        append_decision_rule_evidence(&mut framed, &self.operating_characteristics_evidence)?;
        append_len(&mut framed, self.external_validation_evidence.len())?;
        for evidence in &self.external_validation_evidence {
            append_decision_rule_evidence(&mut framed, evidence)?;
        }
        framed.push(missing_policy_tag(self.missing_evidence_policy));
        framed.extend_from_slice(&self.max_assertion_age_micros.to_be_bytes());
        framed.extend_from_slice(&self.max_future_skew_micros.to_be_bytes());
        framed.push(ceiling_tag(self.qualification_ceiling));

        let mut hasher = blake3::Hasher::new_derive_key(POLICY_DERIVE_KEY);
        hasher.update(&(framed.len() as u64).to_be_bytes());
        hasher.update(&framed);
        Ok(ClinicalDecisionRulePolicyDigestV1(
            *hasher.finalize().as_bytes(),
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClinicalDecisionRulePolicyDigestV1([u8; 32]);

impl ClinicalDecisionRulePolicyDigestV1 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClinicalDecisionRuleStateV1 {
    Triggered,
    NotTriggered,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClinicalDecisionRuleResultDigestV1([u8; 32]);

impl ClinicalDecisionRuleResultDigestV1 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Non-serializable result of applying one exact threshold policy to one exact
/// verified model assertion.
///
/// This object is not a recommendation or presentation capability. `Triggered`
/// means only that the declared numerical rule evaluated true.
pub struct VerifiedClinicalDecisionRuleResultV1 {
    assertion_digest: SymthaeaModelAssertionDigestV1,
    policy_digest: ClinicalDecisionRulePolicyDigestV1,
    result_digest: ClinicalDecisionRuleResultDigestV1,
    state: ClinicalDecisionRuleStateV1,
    subject_id: String,
    endpoint_namespace: String,
    endpoint_id: String,
    prediction_horizon_micros: i64,
    calibrated_probability: f64,
    comparator: ProbabilityComparatorV1,
    probability_threshold: f64,
    qualification_ceiling: SymthaeaAdmissionCeilingV2,
    evaluated_at_micros: i64,
}

impl VerifiedClinicalDecisionRuleResultV1 {
    pub fn assertion_digest(&self) -> SymthaeaModelAssertionDigestV1 {
        self.assertion_digest
    }

    pub fn policy_digest(&self) -> ClinicalDecisionRulePolicyDigestV1 {
        self.policy_digest
    }

    pub fn result_digest(&self) -> ClinicalDecisionRuleResultDigestV1 {
        self.result_digest
    }

    pub fn state(&self) -> ClinicalDecisionRuleStateV1 {
        self.state
    }

    pub fn subject_id(&self) -> &str {
        &self.subject_id
    }

    pub fn endpoint_namespace(&self) -> &str {
        &self.endpoint_namespace
    }

    pub fn endpoint_id(&self) -> &str {
        &self.endpoint_id
    }

    pub fn prediction_horizon_micros(&self) -> i64 {
        self.prediction_horizon_micros
    }

    pub fn calibrated_probability(&self) -> f64 {
        self.calibrated_probability
    }

    pub fn comparator(&self) -> ProbabilityComparatorV1 {
        self.comparator
    }

    pub fn probability_threshold(&self) -> f64 {
        self.probability_threshold
    }

    pub fn qualification_ceiling(&self) -> SymthaeaAdmissionCeilingV2 {
        self.qualification_ceiling
    }

    pub fn evaluated_at_micros(&self) -> i64 {
        self.evaluated_at_micros
    }
}

/// Apply one explicit, evidence-bound probability rule to one verified model
/// assertion without granting any clinical action authority.
pub fn evaluate_clinical_decision_rule_v1(
    assertion: &VerifiedSymthaeaModelAssertionV1,
    policy: &ClinicalDecisionRulePolicyV1,
    evaluated_at_micros: i64,
) -> Result<VerifiedClinicalDecisionRuleResultV1, ClinicalDecisionRuleError> {
    policy.validate()?;
    if evaluated_at_micros <= 0 {
        return Err(ClinicalDecisionRuleError::InvalidEvaluationTime);
    }
    if assertion.kind() != policy.accepted_assertion_kind {
        return Err(ClinicalDecisionRuleError::AssertionKindMismatch);
    }
    if assertion.model() != &policy.required_model {
        return Err(ClinicalDecisionRuleError::ModelIdentityMismatch);
    }
    if assertion.intended_use() != policy.required_intended_use {
        return Err(ClinicalDecisionRuleError::IntendedUseMismatch);
    }
    if applicability_rank(assertion.applicability())
        < applicability_rank(policy.required_minimum_applicability)
    {
        return Err(ClinicalDecisionRuleError::InsufficientApplicability);
    }
    if !policy.allowed_evidence_stages.contains(&assertion.evidence_stage()) {
        return Err(ClinicalDecisionRuleError::EvidenceStageNotAllowed);
    }
    if ceiling_rank(policy.qualification_ceiling) > ceiling_rank(assertion.qualification_ceiling()) {
        return Err(ClinicalDecisionRuleError::QualificationEscalation);
    }

    let age = evaluated_at_micros.saturating_sub(assertion.generated_at_micros());
    if assertion.generated_at_micros() > evaluated_at_micros {
        let skew = assertion.generated_at_micros().saturating_sub(evaluated_at_micros);
        if skew > policy.max_future_skew_micros {
            return Err(ClinicalDecisionRuleError::AssertionTooFarInFuture);
        }
    } else if age > policy.max_assertion_age_micros {
        return Err(ClinicalDecisionRuleError::AssertionTooOld);
    }

    let uncertainty = assertion.uncertainty();
    if uncertainty.calibration_status != SymthaeaCalibrationStatusV2::Calibrated {
        return Err(ClinicalDecisionRuleError::AssertionNotCalibrated);
    }
    let probability = uncertainty
        .calibrated_probability
        .ok_or(ClinicalDecisionRuleError::MissingCalibratedProbability)?;
    if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
        return Err(ClinicalDecisionRuleError::InvalidCalibratedProbability);
    }
    let calibration_evidence = uncertainty
        .calibration_evidence
        .as_ref()
        .ok_or(ClinicalDecisionRuleError::MissingCalibrationEvidence)?;
    if calibration_evidence != &policy.required_calibration_evidence {
        return Err(ClinicalDecisionRuleError::CalibrationEvidenceMismatch);
    }
    if assertion.model().calibration_evidence.as_ref() != Some(calibration_evidence) {
        return Err(ClinicalDecisionRuleError::ModelCalibrationEvidenceMismatch);
    }

    let state = if missingness_requires_abstention(assertion, policy.missing_evidence_policy) {
        ClinicalDecisionRuleStateV1::Indeterminate
    } else if compare_probability(probability, policy.probability_threshold, policy.comparator) {
        ClinicalDecisionRuleStateV1::Triggered
    } else {
        ClinicalDecisionRuleStateV1::NotTriggered
    };

    let policy_digest = policy.digest()?;
    let result_digest = result_digest(
        assertion.assertion_digest(),
        policy_digest,
        state,
        probability,
        evaluated_at_micros,
    );

    Ok(VerifiedClinicalDecisionRuleResultV1 {
        assertion_digest: assertion.assertion_digest(),
        policy_digest,
        result_digest,
        state,
        subject_id: assertion.subject_id().to_string(),
        endpoint_namespace: policy.endpoint_namespace.clone(),
        endpoint_id: policy.endpoint_id.clone(),
        prediction_horizon_micros: policy.prediction_horizon_micros,
        calibrated_probability: probability,
        comparator: policy.comparator,
        probability_threshold: policy.probability_threshold,
        qualification_ceiling: policy.qualification_ceiling,
        evaluated_at_micros,
    })
}

fn missingness_requires_abstention(
    assertion: &VerifiedSymthaeaModelAssertionV1,
    policy: MissingEvidencePolicyV1,
) -> bool {
    assertion.missing_evidence().iter().any(|missing| match policy {
        MissingEvidencePolicyV1::RequireNone => true,
        MissingEvidencePolicyV1::IndeterminateOnImportantOrCritical => matches!(
            missing.criticality,
            SymthaeaMissingCriticalityV2::Important | SymthaeaMissingCriticalityV2::Critical
        ),
        MissingEvidencePolicyV1::IndeterminateOnCriticalOnly => {
            missing.criticality == SymthaeaMissingCriticalityV2::Critical
        }
    })
}

fn compare_probability(probability: f64, threshold: f64, comparator: ProbabilityComparatorV1) -> bool {
    match comparator {
        ProbabilityComparatorV1::GreaterThanOrEqual => probability >= threshold,
        ProbabilityComparatorV1::GreaterThan => probability > threshold,
        ProbabilityComparatorV1::LessThanOrEqual => probability <= threshold,
        ProbabilityComparatorV1::LessThan => probability < threshold,
    }
}

fn result_digest(
    assertion: SymthaeaModelAssertionDigestV1,
    policy: ClinicalDecisionRulePolicyDigestV1,
    state: ClinicalDecisionRuleStateV1,
    probability: f64,
    evaluated_at_micros: i64,
) -> ClinicalDecisionRuleResultDigestV1 {
    let mut hasher = blake3::Hasher::new_derive_key(RESULT_DERIVE_KEY);
    hasher.update(assertion.as_bytes());
    hasher.update(policy.as_bytes());
    hasher.update(&[state_tag(state)]);
    hasher.update(&probability.to_bits().to_be_bytes());
    hasher.update(&evaluated_at_micros.to_be_bytes());
    ClinicalDecisionRuleResultDigestV1(*hasher.finalize().as_bytes())
}

fn validate_symthaea_evidence_identity(
    identity: &SymthaeaEvidenceIdentityV2,
) -> Result<(), ClinicalDecisionRuleError> {
    if identity.identity_version != SYMTHAEA_EVIDENCE_IDENTITY_VERSION {
        return Err(ClinicalDecisionRuleError::UnsupportedCalibrationEvidenceVersion(
            identity.identity_version,
        ));
    }
    validate_token(&identity.namespace)?;
    validate_token(&identity.artifact_id)?;
    if identity.digest.algorithm != SymthaeaDigestAlgorithmV2::Blake3_256
        || identity.digest.value == [0u8; 32]
    {
        return Err(ClinicalDecisionRuleError::InvalidCalibrationEvidenceDigest);
    }
    Ok(())
}

fn validate_model_identity(model: &SymthaeaModelIdentityV2) -> Result<(), ClinicalDecisionRuleError> {
    validate_token(&model.model.name)?;
    validate_token(&model.model.version)?;
    validate_symthaea_digest(model.model.digest.algorithm, &model.model.digest.value)?;
    validate_symthaea_digest(
        model.input_schema_digest.algorithm,
        &model.input_schema_digest.value,
    )?;
    validate_symthaea_digest(
        model.output_schema_digest.algorithm,
        &model.output_schema_digest.value,
    )?;
    for identity in [
        model.training_lineage.as_ref(),
        model.evaluation_lineage.as_ref(),
        model.calibration_evidence.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        validate_symthaea_evidence_identity(identity)?;
    }
    Ok(())
}

fn validate_symthaea_digest(
    algorithm: SymthaeaDigestAlgorithmV2,
    value: &[u8; 32],
) -> Result<(), ClinicalDecisionRuleError> {
    if algorithm != SymthaeaDigestAlgorithmV2::Blake3_256 || *value == [0u8; 32] {
        return Err(ClinicalDecisionRuleError::InvalidModelDigest);
    }
    Ok(())
}

fn append_model_identity(
    target: &mut Vec<u8>,
    model: &SymthaeaModelIdentityV2,
) -> Result<(), ClinicalDecisionRuleError> {
    validate_model_identity(model)?;
    append_string(target, &model.model.name)?;
    append_string(target, &model.model.version)?;
    append_symthaea_digest(target, model.model.digest.algorithm, &model.model.digest.value);
    append_symthaea_digest(
        target,
        model.input_schema_digest.algorithm,
        &model.input_schema_digest.value,
    );
    append_symthaea_digest(
        target,
        model.output_schema_digest.algorithm,
        &model.output_schema_digest.value,
    );
    for identity in [
        model.training_lineage.as_ref(),
        model.evaluation_lineage.as_ref(),
        model.calibration_evidence.as_ref(),
    ] {
        match identity {
            Some(identity) => {
                target.push(1);
                append_symthaea_evidence_identity(target, identity)?;
            }
            None => target.push(0),
        }
    }
    Ok(())
}

fn append_symthaea_evidence_identity(
    target: &mut Vec<u8>,
    identity: &SymthaeaEvidenceIdentityV2,
) -> Result<(), ClinicalDecisionRuleError> {
    validate_symthaea_evidence_identity(identity)?;
    target.extend_from_slice(&identity.identity_version.to_be_bytes());
    append_string(target, &identity.namespace)?;
    append_string(target, &identity.artifact_id)?;
    append_symthaea_digest(target, identity.digest.algorithm, &identity.digest.value);
    Ok(())
}

fn append_symthaea_digest(
    target: &mut Vec<u8>,
    algorithm: SymthaeaDigestAlgorithmV2,
    value: &[u8; 32],
) {
    target.push(match algorithm {
        SymthaeaDigestAlgorithmV2::Blake3_256 => 1,
    });
    target.extend_from_slice(value);
}

fn append_decision_rule_evidence(
    target: &mut Vec<u8>,
    evidence: &DecisionRuleEvidenceIdentityV1,
) -> Result<(), ClinicalDecisionRuleError> {
    evidence.validate()?;
    target.extend_from_slice(&evidence.schema_version.to_be_bytes());
    append_string(target, &evidence.namespace)?;
    append_string(target, &evidence.artifact_id)?;
    target.extend_from_slice(&evidence.digest);
    Ok(())
}

fn append_len(target: &mut Vec<u8>, len: usize) -> Result<(), ClinicalDecisionRuleError> {
    let len = u32::try_from(len).map_err(|_| ClinicalDecisionRuleError::PolicyTooLarge)?;
    target.extend_from_slice(&len.to_be_bytes());
    Ok(())
}

fn append_string(target: &mut Vec<u8>, value: &str) -> Result<(), ClinicalDecisionRuleError> {
    validate_token(value)?;
    append_len(target, value.len())?;
    target.extend_from_slice(value.as_bytes());
    Ok(())
}

fn validate_token(value: &str) -> Result<(), ClinicalDecisionRuleError> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(ClinicalDecisionRuleError::InvalidPolicyToken);
    }
    if value.len() > 4096 {
        return Err(ClinicalDecisionRuleError::PolicyTooLarge);
    }
    Ok(())
}

fn applicability_rank(value: SymthaeaApplicabilityV2) -> u8 {
    match value {
        SymthaeaApplicabilityV2::Unestablished => 0,
        SymthaeaApplicabilityV2::EvaluatedCohortOnly => 1,
        SymthaeaApplicabilityV2::DefinedTargetPopulation => 2,
        SymthaeaApplicabilityV2::ValidatedTargetPopulation => 3,
    }
}

fn ceiling_rank(value: SymthaeaAdmissionCeilingV2) -> u8 {
    match value {
        SymthaeaAdmissionCeilingV2::Experimental => 0,
        SymthaeaAdmissionCeilingV2::ValidatedOffline => 1,
        SymthaeaAdmissionCeilingV2::ShadowClinical => 2,
    }
}

fn assertion_kind_tag(value: ModelAssertionKindV1) -> u8 {
    match value {
        ModelAssertionKindV1::Prediction => 0,
        ModelAssertionKindV1::RiskEstimate => 1,
    }
}

fn intended_use_tag(value: SymthaeaIntendedUseV2) -> u8 {
    match value {
        SymthaeaIntendedUseV2::ResearchOnly => 0,
        SymthaeaIntendedUseV2::ClinicalDecisionSupport => 1,
    }
}

fn applicability_tag(value: SymthaeaApplicabilityV2) -> u8 {
    applicability_rank(value)
}

fn evidence_stage_tag(value: SymthaeaEvidenceStageV2) -> u8 {
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

fn comparator_tag(value: ProbabilityComparatorV1) -> u8 {
    match value {
        ProbabilityComparatorV1::GreaterThanOrEqual => 0,
        ProbabilityComparatorV1::GreaterThan => 1,
        ProbabilityComparatorV1::LessThanOrEqual => 2,
        ProbabilityComparatorV1::LessThan => 3,
    }
}

fn missing_policy_tag(value: MissingEvidencePolicyV1) -> u8 {
    match value {
        MissingEvidencePolicyV1::RequireNone => 0,
        MissingEvidencePolicyV1::IndeterminateOnImportantOrCritical => 1,
        MissingEvidencePolicyV1::IndeterminateOnCriticalOnly => 2,
    }
}

fn ceiling_tag(value: SymthaeaAdmissionCeilingV2) -> u8 {
    ceiling_rank(value)
}

fn state_tag(value: ClinicalDecisionRuleStateV1) -> u8 {
    match value {
        ClinicalDecisionRuleStateV1::Triggered => 0,
        ClinicalDecisionRuleStateV1::NotTriggered => 1,
        ClinicalDecisionRuleStateV1::Indeterminate => 2,
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClinicalDecisionRuleError {
    #[error("unsupported clinical decision rule policy version {0}")]
    UnsupportedPolicyVersion(u16),
    #[error("unsupported decision rule evidence identity version {0}")]
    UnsupportedEvidenceIdentityVersion(u16),
    #[error("unsupported producer calibration evidence identity version {0}")]
    UnsupportedCalibrationEvidenceVersion(u16),
    #[error("decision rule policy token is invalid")]
    InvalidPolicyToken,
    #[error("decision rule policy exceeds framing limits")]
    PolicyTooLarge,
    #[error("prediction horizon is invalid")]
    InvalidPredictionHorizon,
    #[error("probability threshold must be finite and within [0, 1]")]
    InvalidProbabilityThreshold,
    #[error("decision rule requires at least one allowed evidence stage")]
    MissingAllowedEvidenceStages,
    #[error("decision rule allowed evidence stages contain a duplicate")]
    DuplicateAllowedEvidenceStage,
    #[error("decision rule evidence digest cannot be zero")]
    ZeroEvidenceDigest,
    #[error("decision rule requires external validation evidence")]
    MissingExternalValidationEvidence,
    #[error("decision rule external validation evidence contains a duplicate")]
    DuplicateExternalValidationEvidence,
    #[error("assertion age bound is invalid")]
    InvalidMaximumAssertionAge,
    #[error("assertion future-skew bound is invalid")]
    InvalidMaximumFutureSkew,
    #[error("producer calibration evidence digest is invalid")]
    InvalidCalibrationEvidenceDigest,
    #[error("model identity contains an invalid digest")]
    InvalidModelDigest,
    #[error("decision rule evaluation time must be positive")]
    InvalidEvaluationTime,
    #[error("model assertion kind does not match the decision rule")]
    AssertionKindMismatch,
    #[error("model assertion model identity does not match the decision rule")]
    ModelIdentityMismatch,
    #[error("model assertion intended use does not match the decision rule")]
    IntendedUseMismatch,
    #[error("model assertion population applicability is below the rule requirement")]
    InsufficientApplicability,
    #[error("model assertion evidence stage is not allowed by the decision rule")]
    EvidenceStageNotAllowed,
    #[error("decision rule would raise authority above the model assertion ceiling")]
    QualificationEscalation,
    #[error("model assertion is too old for this decision rule")]
    AssertionTooOld,
    #[error("model assertion is too far in the future for this decision rule")]
    AssertionTooFarInFuture,
    #[error("model assertion is not calibrated")]
    AssertionNotCalibrated,
    #[error("model assertion has no calibrated probability")]
    MissingCalibratedProbability,
    #[error("model assertion calibrated probability is invalid")]
    InvalidCalibratedProbability,
    #[error("model assertion has no calibration evidence")]
    MissingCalibrationEvidence,
    #[error("model assertion calibration evidence does not match the decision rule")]
    CalibrationEvidenceMismatch,
    #[error("model identity calibration evidence differs from assertion calibration evidence")]
    ModelCalibrationEvidenceMismatch,
}
