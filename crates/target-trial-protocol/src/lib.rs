#![deny(unsafe_code)]
//! Research-only target trial protocol and observational emulation plan identities.
//!
//! V1 keeps three concepts separate:
//! 1. the hypothetical randomized target trial protocol;
//! 2. the plan mapping that protocol into observational data;
//! 3. any later causal estimate produced by executing the emulation.
//!
//! This crate defines only (1) and (2). It cannot encode a causal effect estimate.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const TARGET_TRIAL_PROTOCOL_VERSION: u16 = 1;

const PROTOCOL_TAG: &[u8] = b"mycelix/target-trial-protocol/v1";
const EMULATION_TAG: &[u8] = b"mycelix/target-trial-emulation-plan/v1";
const PROTOCOL_CONTEXT: &str = "mycelix.health.target-trial-protocol.v1";
const EMULATION_CONTEXT: &str = "mycelix.health.target-trial-emulation-plan.v1";
const PHENOTYPE_NAMESPACE: &str = "mycelix/clinical-phenotype-definition/v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceArtifactIdentityV1 {
    pub namespace: String,
    pub artifact_id: String,
    pub version: String,
    pub digest: [u8; 32],
}

impl EvidenceArtifactIdentityV1 {
    fn validate(&self) -> Result<(), TargetTrialError> {
        if self.namespace.trim().is_empty()
            || self.artifact_id.trim().is_empty()
            || self.version.trim().is_empty()
        {
            return Err(TargetTrialError::IncompleteEvidenceIdentity);
        }
        if self.digest == [0; 32] {
            return Err(TargetTrialError::ZeroEvidenceDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhenotypeDefinitionRefV1 {
    pub phenotype_id: String,
    pub version: String,
    pub digest: [u8; 32],
}

impl PhenotypeDefinitionRefV1 {
    fn validate(&self) -> Result<(), TargetTrialError> {
        if self.phenotype_id.trim().is_empty() || self.version.trim().is_empty() {
            return Err(TargetTrialError::IncompletePhenotypeIdentity);
        }
        if self.digest == [0; 32] {
            return Err(TargetTrialError::ZeroEvidenceDigest);
        }
        Ok(())
    }

    pub fn as_evidence_identity(&self) -> EvidenceArtifactIdentityV1 {
        EvidenceArtifactIdentityV1 {
            namespace: PHENOTYPE_NAMESPACE.to_string(),
            artifact_id: self.phenotype_id.clone(),
            version: self.version.clone(),
            digest: self.digest,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreatmentStrategyV1 {
    pub strategy_id: String,
    pub label: String,
    pub intervention_definition: EvidenceArtifactIdentityV1,
}

impl TreatmentStrategyV1 {
    fn validate(&self) -> Result<(), TargetTrialError> {
        if self.strategy_id.trim().is_empty() || self.label.trim().is_empty() {
            return Err(TargetTrialError::InvalidTreatmentStrategy);
        }
        self.intervention_definition.validate()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MaskingV1 {
    OpenLabel,
    SingleBlind,
    DoubleBlind,
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HypotheticalRandomAssignmentV1 {
    pub allocation_description: String,
    pub masking: MaskingV1,
}

impl HypotheticalRandomAssignmentV1 {
    fn validate(&self) -> Result<(), TargetTrialError> {
        if self.allocation_description.trim().is_empty() {
            return Err(TargetTrialError::MissingAssignmentDescription);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FollowUpV1 {
    pub maximum_duration_micros: i64,
    pub end_conditions: Vec<EvidenceArtifactIdentityV1>,
}

impl FollowUpV1 {
    fn validate(&self) -> Result<(), TargetTrialError> {
        if self.maximum_duration_micros <= 0 {
            return Err(TargetTrialError::InvalidFollowUpDuration);
        }
        if self.end_conditions.is_empty() {
            return Err(TargetTrialError::MissingFollowUpEndCondition);
        }
        for condition in &self.end_conditions {
            condition.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutcomeV1 {
    pub outcome_id: String,
    pub phenotype: PhenotypeDefinitionRefV1,
    pub assessment_window_start_offset_micros: i64,
    pub assessment_window_end_offset_micros: i64,
    pub primary: bool,
}

impl OutcomeV1 {
    fn validate(&self) -> Result<(), TargetTrialError> {
        if self.outcome_id.trim().is_empty() {
            return Err(TargetTrialError::MissingOutcomeId);
        }
        self.phenotype.validate()?;
        if self.assessment_window_start_offset_micros < 0
            || self.assessment_window_start_offset_micros
                >= self.assessment_window_end_offset_micros
        {
            return Err(TargetTrialError::InvalidOutcomeAssessmentWindow);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CausalContrastV1 {
    IntentionToTreat,
    PerProtocol,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EffectMeasureV1 {
    RiskDifference,
    RiskRatio,
    HazardRatio,
    MeanDifference,
    SurvivalDifference,
    Other,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompetingEventStrategyV1 {
    CensorAtCompetingEvent,
    CompositeOutcome,
    CompetingRiskEstimand,
    IgnoreWhenScientificallyJustified,
    NotApplicable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CausalEstimandV1 {
    pub estimand_id: String,
    pub treatment_strategy_id: String,
    pub comparator_strategy_id: String,
    pub outcome_id: String,
    pub contrast: CausalContrastV1,
    pub effect_measure: EffectMeasureV1,
    pub competing_event_strategy: CompetingEventStrategyV1,
    pub estimand_definition: EvidenceArtifactIdentityV1,
}

impl CausalEstimandV1 {
    fn validate(&self) -> Result<(), TargetTrialError> {
        if self.estimand_id.trim().is_empty()
            || self.treatment_strategy_id.trim().is_empty()
            || self.comparator_strategy_id.trim().is_empty()
            || self.outcome_id.trim().is_empty()
            || self.treatment_strategy_id == self.comparator_strategy_id
        {
            return Err(TargetTrialError::InvalidEstimand);
        }
        self.estimand_definition.validate()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentifyingAssumptionV1 {
    pub assumption_id: String,
    pub statement: String,
    pub related_variable_definitions: Vec<EvidenceArtifactIdentityV1>,
}

impl IdentifyingAssumptionV1 {
    fn validate(&self) -> Result<(), TargetTrialError> {
        if self.assumption_id.trim().is_empty() || self.statement.trim().is_empty() {
            return Err(TargetTrialError::InvalidIdentifyingAssumption);
        }
        for artifact in &self.related_variable_definitions {
            artifact.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetTrialProtocolV1 {
    pub schema_version: u16,
    pub protocol_id: String,
    pub version: String,
    pub causal_question: String,
    pub rationale_evidence: Vec<EvidenceArtifactIdentityV1>,
    pub eligibility: PhenotypeDefinitionRefV1,
    pub treatment_strategies: Vec<TreatmentStrategyV1>,
    pub assignment: HypotheticalRandomAssignmentV1,
    pub time_zero_definition: EvidenceArtifactIdentityV1,
    pub follow_up: FollowUpV1,
    pub outcomes: Vec<OutcomeV1>,
    pub estimands: Vec<CausalEstimandV1>,
    pub identifying_assumptions: Vec<IdentifyingAssumptionV1>,
    pub analysis_plan: EvidenceArtifactIdentityV1,
    pub sensitivity_analysis_plans: Vec<EvidenceArtifactIdentityV1>,
}

impl TargetTrialProtocolV1 {
    pub fn validate(&self) -> Result<(), TargetTrialError> {
        if self.schema_version != TARGET_TRIAL_PROTOCOL_VERSION {
            return Err(TargetTrialError::UnsupportedSchemaVersion(self.schema_version));
        }
        if self.protocol_id.trim().is_empty()
            || self.version.trim().is_empty()
            || self.causal_question.trim().is_empty()
        {
            return Err(TargetTrialError::IncompleteProtocolIdentity);
        }
        if self.rationale_evidence.is_empty() {
            return Err(TargetTrialError::MissingRationaleEvidence);
        }
        for artifact in &self.rationale_evidence {
            artifact.validate()?;
        }
        self.eligibility.validate()?;
        if self.treatment_strategies.len() < 2 {
            return Err(TargetTrialError::InsufficientTreatmentStrategies);
        }
        let mut strategy_ids = HashSet::new();
        for strategy in &self.treatment_strategies {
            strategy.validate()?;
            if !strategy_ids.insert(strategy.strategy_id.clone()) {
                return Err(TargetTrialError::DuplicateTreatmentStrategy(
                    strategy.strategy_id.clone(),
                ));
            }
        }
        self.assignment.validate()?;
        self.time_zero_definition.validate()?;
        self.follow_up.validate()?;
        if self.outcomes.is_empty() {
            return Err(TargetTrialError::MissingOutcomes);
        }
        let mut outcome_ids = HashSet::new();
        let mut primary_count = 0usize;
        for outcome in &self.outcomes {
            outcome.validate()?;
            if !outcome_ids.insert(outcome.outcome_id.clone()) {
                return Err(TargetTrialError::DuplicateOutcome(outcome.outcome_id.clone()));
            }
            if outcome.primary {
                primary_count += 1;
            }
        }
        if primary_count == 0 {
            return Err(TargetTrialError::MissingPrimaryOutcome);
        }
        if self.estimands.is_empty() {
            return Err(TargetTrialError::MissingEstimands);
        }
        let mut estimand_ids = HashSet::new();
        for estimand in &self.estimands {
            estimand.validate()?;
            if !estimand_ids.insert(estimand.estimand_id.clone()) {
                return Err(TargetTrialError::DuplicateEstimand(estimand.estimand_id.clone()));
            }
            if !strategy_ids.contains(&estimand.treatment_strategy_id)
                || !strategy_ids.contains(&estimand.comparator_strategy_id)
            {
                return Err(TargetTrialError::EstimandReferencesUnknownStrategy);
            }
            if !outcome_ids.contains(&estimand.outcome_id) {
                return Err(TargetTrialError::EstimandReferencesUnknownOutcome);
            }
        }
        let mut assumption_ids = HashSet::new();
        for assumption in &self.identifying_assumptions {
            assumption.validate()?;
            if !assumption_ids.insert(assumption.assumption_id.clone()) {
                return Err(TargetTrialError::DuplicateAssumption(
                    assumption.assumption_id.clone(),
                ));
            }
        }
        self.analysis_plan.validate()?;
        for plan in &self.sensitivity_analysis_plans {
            plan.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataSourceV1 {
    pub source_id: String,
    pub original_purpose: String,
    pub source_type: String,
    pub setting: String,
    pub geography: String,
    pub period_start_micros: i64,
    pub period_end_micros: i64,
    pub source_identity: EvidenceArtifactIdentityV1,
}

impl DataSourceV1 {
    fn validate(&self) -> Result<(), TargetTrialError> {
        if self.source_id.trim().is_empty()
            || self.original_purpose.trim().is_empty()
            || self.source_type.trim().is_empty()
            || self.setting.trim().is_empty()
            || self.geography.trim().is_empty()
            || self.period_start_micros >= self.period_end_micros
        {
            return Err(TargetTrialError::InvalidDataSource);
        }
        self.source_identity.validate()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategyOperationalizationV1 {
    pub strategy_id: String,
    pub classification_rule: EvidenceArtifactIdentityV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutcomeOperationalizationV1 {
    pub outcome_id: String,
    pub operationalization: EvidenceArtifactIdentityV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineConfounderV1 {
    pub confounder_id: String,
    pub definition: EvidenceArtifactIdentityV1,
    /// Latest permissible measurement relative to time zero. V1 forbids > 0.
    pub measurement_window_end_offset_micros: i64,
}

impl BaselineConfounderV1 {
    fn validate(&self) -> Result<(), TargetTrialError> {
        if self.confounder_id.trim().is_empty() {
            return Err(TargetTrialError::InvalidConfounder);
        }
        self.definition.validate()?;
        if self.measurement_window_end_offset_micros > 0 {
            return Err(TargetTrialError::PostBaselineConfounder);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SoftwareEnvironmentV1 {
    pub software: EvidenceArtifactIdentityV1,
    pub environment: EvidenceArtifactIdentityV1,
}

impl SoftwareEnvironmentV1 {
    fn validate(&self) -> Result<(), TargetTrialError> {
        self.software.validate()?;
        self.environment.validate()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetTrialEmulationPlanV1 {
    pub schema_version: u16,
    pub emulation_id: String,
    pub version: String,
    pub protocol_digest: [u8; 32],
    pub observational_design_statement: String,
    pub data_sources: Vec<DataSourceV1>,
    pub eligibility_operationalization: EvidenceArtifactIdentityV1,
    pub strategy_operationalizations: Vec<StrategyOperationalizationV1>,
    pub time_zero_operationalization: EvidenceArtifactIdentityV1,
    pub outcome_operationalizations: Vec<OutcomeOperationalizationV1>,
    pub baseline_confounders: Vec<BaselineConfounderV1>,
    pub censoring_plan: EvidenceArtifactIdentityV1,
    pub adherence_plan: Option<EvidenceArtifactIdentityV1>,
    pub time_varying_confounding_plan: Option<EvidenceArtifactIdentityV1>,
    pub missing_data_plan: EvidenceArtifactIdentityV1,
    pub estimator_plan: EvidenceArtifactIdentityV1,
    pub software_environment: SoftwareEnvironmentV1,
    pub vocabulary_and_mapping_artifacts: Vec<EvidenceArtifactIdentityV1>,
}

impl TargetTrialEmulationPlanV1 {
    pub fn validate_against(&self, protocol: &TargetTrialProtocolV1) -> Result<(), TargetTrialError> {
        protocol.validate()?;
        if self.schema_version != TARGET_TRIAL_PROTOCOL_VERSION {
            return Err(TargetTrialError::UnsupportedSchemaVersion(self.schema_version));
        }
        if self.emulation_id.trim().is_empty()
            || self.version.trim().is_empty()
            || self.observational_design_statement.trim().is_empty()
        {
            return Err(TargetTrialError::IncompleteEmulationIdentity);
        }
        let protocol_digest = target_trial_protocol_digest_v1(protocol)?.into_bytes();
        if self.protocol_digest != protocol_digest {
            return Err(TargetTrialError::ProtocolDigestMismatch);
        }
        if self.data_sources.is_empty() {
            return Err(TargetTrialError::MissingDataSources);
        }
        let mut source_ids = HashSet::new();
        for source in &self.data_sources {
            source.validate()?;
            if !source_ids.insert(source.source_id.clone()) {
                return Err(TargetTrialError::DuplicateDataSource(source.source_id.clone()));
            }
        }
        self.eligibility_operationalization.validate()?;
        self.time_zero_operationalization.validate()?;
        self.censoring_plan.validate()?;
        self.missing_data_plan.validate()?;
        self.estimator_plan.validate()?;
        self.software_environment.validate()?;
        if let Some(plan) = &self.adherence_plan {
            plan.validate()?;
        }
        if let Some(plan) = &self.time_varying_confounding_plan {
            plan.validate()?;
        }
        for artifact in &self.vocabulary_and_mapping_artifacts {
            artifact.validate()?;
        }

        let strategy_ids: HashSet<_> = protocol
            .treatment_strategies
            .iter()
            .map(|strategy| strategy.strategy_id.as_str())
            .collect();
        let mut mapped_strategies = HashSet::new();
        for operationalization in &self.strategy_operationalizations {
            if !strategy_ids.contains(operationalization.strategy_id.as_str())
                || !mapped_strategies.insert(operationalization.strategy_id.clone())
            {
                return Err(TargetTrialError::InvalidStrategyOperationalization);
            }
            operationalization.classification_rule.validate()?;
        }
        if mapped_strategies.len() != strategy_ids.len() {
            return Err(TargetTrialError::IncompleteStrategyOperationalization);
        }

        let outcome_ids: HashSet<_> = protocol
            .outcomes
            .iter()
            .map(|outcome| outcome.outcome_id.as_str())
            .collect();
        let mut mapped_outcomes = HashSet::new();
        for operationalization in &self.outcome_operationalizations {
            if !outcome_ids.contains(operationalization.outcome_id.as_str())
                || !mapped_outcomes.insert(operationalization.outcome_id.clone())
            {
                return Err(TargetTrialError::InvalidOutcomeOperationalization);
            }
            operationalization.operationalization.validate()?;
        }
        if mapped_outcomes.len() != outcome_ids.len() {
            return Err(TargetTrialError::IncompleteOutcomeOperationalization);
        }

        let mut confounder_ids = HashSet::new();
        for confounder in &self.baseline_confounders {
            confounder.validate()?;
            if !confounder_ids.insert(confounder.confounder_id.clone()) {
                return Err(TargetTrialError::DuplicateConfounder(
                    confounder.confounder_id.clone(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TargetTrialProtocolDigestV1([u8; 32]);

impl TargetTrialProtocolDigestV1 {
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TargetTrialEmulationPlanDigestV1([u8; 32]);

impl TargetTrialEmulationPlanDigestV1 {
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

pub fn target_trial_protocol_digest_v1(
    protocol: &TargetTrialProtocolV1,
) -> Result<TargetTrialProtocolDigestV1, TargetTrialError> {
    protocol.validate()?;
    let mut writer = CanonicalWriter::default();
    encode_protocol(&mut writer, protocol)?;
    Ok(TargetTrialProtocolDigestV1(domain_digest(
        PROTOCOL_CONTEXT,
        PROTOCOL_TAG,
        &writer.finish(),
    )))
}

pub fn target_trial_emulation_plan_digest_v1(
    protocol: &TargetTrialProtocolV1,
    plan: &TargetTrialEmulationPlanV1,
) -> Result<TargetTrialEmulationPlanDigestV1, TargetTrialError> {
    plan.validate_against(protocol)?;
    let mut writer = CanonicalWriter::default();
    encode_emulation(&mut writer, plan)?;
    Ok(TargetTrialEmulationPlanDigestV1(domain_digest(
        EMULATION_CONTEXT,
        EMULATION_TAG,
        &writer.finish(),
    )))
}

fn domain_digest(context: &str, tag: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&TARGET_TRIAL_PROTOCOL_VERSION.to_be_bytes());
    hasher.update(&(tag.len() as u16).to_be_bytes());
    hasher.update(tag);
    hasher.update(&(payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    *hasher.finalize().as_bytes()
}

#[derive(Default)]
struct CanonicalWriter {
    bytes: Vec<u8>,
}

impl CanonicalWriter {
    fn finish(self) -> Vec<u8> { self.bytes }
    fn u8(&mut self, value: u8) { self.bytes.push(value); }
    fn u16(&mut self, value: u16) { self.bytes.extend_from_slice(&value.to_be_bytes()); }
    fn u32(&mut self, value: u32) { self.bytes.extend_from_slice(&value.to_be_bytes()); }
    fn u64(&mut self, value: u64) { self.bytes.extend_from_slice(&value.to_be_bytes()); }
    fn i64(&mut self, value: i64) { self.bytes.extend_from_slice(&value.to_be_bytes()); }
    fn string(&mut self, value: &str) -> Result<(), TargetTrialError> {
        self.bytes(value.as_bytes())
    }
    fn bytes(&mut self, value: &[u8]) -> Result<(), TargetTrialError> {
        self.u32(u32::try_from(value.len()).map_err(|_| TargetTrialError::LengthOverflow)?);
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn vector_len(&mut self, len: usize) -> Result<(), TargetTrialError> {
        self.u32(u32::try_from(len).map_err(|_| TargetTrialError::LengthOverflow)?);
        Ok(())
    }
}

fn encode_artifact(writer: &mut CanonicalWriter, value: &EvidenceArtifactIdentityV1) -> Result<(), TargetTrialError> {
    writer.string(&value.namespace)?;
    writer.string(&value.artifact_id)?;
    writer.string(&value.version)?;
    writer.bytes(&value.digest)?;
    Ok(())
}

fn encode_phenotype(writer: &mut CanonicalWriter, value: &PhenotypeDefinitionRefV1) -> Result<(), TargetTrialError> {
    writer.string(&value.phenotype_id)?;
    writer.string(&value.version)?;
    writer.bytes(&value.digest)?;
    Ok(())
}

fn encode_protocol(writer: &mut CanonicalWriter, protocol: &TargetTrialProtocolV1) -> Result<(), TargetTrialError> {
    writer.u16(protocol.schema_version);
    writer.string(&protocol.protocol_id)?;
    writer.string(&protocol.version)?;
    writer.string(&protocol.causal_question)?;
    writer.vector_len(protocol.rationale_evidence.len())?;
    for artifact in &protocol.rationale_evidence { encode_artifact(writer, artifact)?; }
    encode_phenotype(writer, &protocol.eligibility)?;
    writer.vector_len(protocol.treatment_strategies.len())?;
    for strategy in &protocol.treatment_strategies {
        writer.string(&strategy.strategy_id)?;
        writer.string(&strategy.label)?;
        encode_artifact(writer, &strategy.intervention_definition)?;
    }
    writer.string(&protocol.assignment.allocation_description)?;
    writer.u8(match protocol.assignment.masking {
        MaskingV1::OpenLabel => 0,
        MaskingV1::SingleBlind => 1,
        MaskingV1::DoubleBlind => 2,
        MaskingV1::Other => 3,
    });
    encode_artifact(writer, &protocol.time_zero_definition)?;
    writer.i64(protocol.follow_up.maximum_duration_micros);
    writer.vector_len(protocol.follow_up.end_conditions.len())?;
    for artifact in &protocol.follow_up.end_conditions { encode_artifact(writer, artifact)?; }
    writer.vector_len(protocol.outcomes.len())?;
    for outcome in &protocol.outcomes {
        writer.string(&outcome.outcome_id)?;
        encode_phenotype(writer, &outcome.phenotype)?;
        writer.i64(outcome.assessment_window_start_offset_micros);
        writer.i64(outcome.assessment_window_end_offset_micros);
        writer.u8(u8::from(outcome.primary));
    }
    writer.vector_len(protocol.estimands.len())?;
    for estimand in &protocol.estimands {
        writer.string(&estimand.estimand_id)?;
        writer.string(&estimand.treatment_strategy_id)?;
        writer.string(&estimand.comparator_strategy_id)?;
        writer.string(&estimand.outcome_id)?;
        writer.u8(match estimand.contrast { CausalContrastV1::IntentionToTreat => 0, CausalContrastV1::PerProtocol => 1 });
        writer.u8(match estimand.effect_measure {
            EffectMeasureV1::RiskDifference => 0,
            EffectMeasureV1::RiskRatio => 1,
            EffectMeasureV1::HazardRatio => 2,
            EffectMeasureV1::MeanDifference => 3,
            EffectMeasureV1::SurvivalDifference => 4,
            EffectMeasureV1::Other => 5,
        });
        writer.u8(match estimand.competing_event_strategy {
            CompetingEventStrategyV1::CensorAtCompetingEvent => 0,
            CompetingEventStrategyV1::CompositeOutcome => 1,
            CompetingEventStrategyV1::CompetingRiskEstimand => 2,
            CompetingEventStrategyV1::IgnoreWhenScientificallyJustified => 3,
            CompetingEventStrategyV1::NotApplicable => 4,
        });
        encode_artifact(writer, &estimand.estimand_definition)?;
    }
    writer.vector_len(protocol.identifying_assumptions.len())?;
    for assumption in &protocol.identifying_assumptions {
        writer.string(&assumption.assumption_id)?;
        writer.string(&assumption.statement)?;
        writer.vector_len(assumption.related_variable_definitions.len())?;
        for artifact in &assumption.related_variable_definitions { encode_artifact(writer, artifact)?; }
    }
    encode_artifact(writer, &protocol.analysis_plan)?;
    writer.vector_len(protocol.sensitivity_analysis_plans.len())?;
    for artifact in &protocol.sensitivity_analysis_plans { encode_artifact(writer, artifact)?; }
    Ok(())
}

fn encode_emulation(writer: &mut CanonicalWriter, plan: &TargetTrialEmulationPlanV1) -> Result<(), TargetTrialError> {
    writer.u16(plan.schema_version);
    writer.string(&plan.emulation_id)?;
    writer.string(&plan.version)?;
    writer.bytes(&plan.protocol_digest)?;
    writer.string(&plan.observational_design_statement)?;
    writer.vector_len(plan.data_sources.len())?;
    for source in &plan.data_sources {
        writer.string(&source.source_id)?;
        writer.string(&source.original_purpose)?;
        writer.string(&source.source_type)?;
        writer.string(&source.setting)?;
        writer.string(&source.geography)?;
        writer.i64(source.period_start_micros);
        writer.i64(source.period_end_micros);
        encode_artifact(writer, &source.source_identity)?;
    }
    encode_artifact(writer, &plan.eligibility_operationalization)?;
    writer.vector_len(plan.strategy_operationalizations.len())?;
    for value in &plan.strategy_operationalizations {
        writer.string(&value.strategy_id)?;
        encode_artifact(writer, &value.classification_rule)?;
    }
    encode_artifact(writer, &plan.time_zero_operationalization)?;
    writer.vector_len(plan.outcome_operationalizations.len())?;
    for value in &plan.outcome_operationalizations {
        writer.string(&value.outcome_id)?;
        encode_artifact(writer, &value.operationalization)?;
    }
    writer.vector_len(plan.baseline_confounders.len())?;
    for value in &plan.baseline_confounders {
        writer.string(&value.confounder_id)?;
        encode_artifact(writer, &value.definition)?;
        writer.i64(value.measurement_window_end_offset_micros);
    }
    encode_artifact(writer, &plan.censoring_plan)?;
    encode_optional_artifact(writer, plan.adherence_plan.as_ref())?;
    encode_optional_artifact(writer, plan.time_varying_confounding_plan.as_ref())?;
    encode_artifact(writer, &plan.missing_data_plan)?;
    encode_artifact(writer, &plan.estimator_plan)?;
    encode_artifact(writer, &plan.software_environment.software)?;
    encode_artifact(writer, &plan.software_environment.environment)?;
    writer.vector_len(plan.vocabulary_and_mapping_artifacts.len())?;
    for artifact in &plan.vocabulary_and_mapping_artifacts { encode_artifact(writer, artifact)?; }
    Ok(())
}

fn encode_optional_artifact(writer: &mut CanonicalWriter, value: Option<&EvidenceArtifactIdentityV1>) -> Result<(), TargetTrialError> {
    match value {
        Some(value) => { writer.u8(1); encode_artifact(writer, value)?; }
        None => writer.u8(0),
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum TargetTrialError {
    #[error("unsupported target-trial schema version: {0}")]
    UnsupportedSchemaVersion(u16),
    #[error("evidence artifact identity is incomplete")]
    IncompleteEvidenceIdentity,
    #[error("evidence digest may not be all zeroes")]
    ZeroEvidenceDigest,
    #[error("phenotype identity is incomplete")]
    IncompletePhenotypeIdentity,
    #[error("target-trial protocol identity/question is incomplete")]
    IncompleteProtocolIdentity,
    #[error("target-trial protocol lacks rationale evidence")]
    MissingRationaleEvidence,
    #[error("treatment strategy is invalid")]
    InvalidTreatmentStrategy,
    #[error("target trial requires at least two treatment strategies")]
    InsufficientTreatmentStrategies,
    #[error("duplicate treatment strategy id: {0}")]
    DuplicateTreatmentStrategy(String),
    #[error("hypothetical random assignment description is missing")]
    MissingAssignmentDescription,
    #[error("follow-up duration must be positive")]
    InvalidFollowUpDuration,
    #[error("follow-up end condition is required")]
    MissingFollowUpEndCondition,
    #[error("outcome id is missing")]
    MissingOutcomeId,
    #[error("outcome assessment window is invalid")]
    InvalidOutcomeAssessmentWindow,
    #[error("target trial requires at least one outcome")]
    MissingOutcomes,
    #[error("target trial requires at least one primary outcome")]
    MissingPrimaryOutcome,
    #[error("duplicate outcome id: {0}")]
    DuplicateOutcome(String),
    #[error("causal estimand is invalid")]
    InvalidEstimand,
    #[error("target trial requires at least one causal estimand")]
    MissingEstimands,
    #[error("duplicate estimand id: {0}")]
    DuplicateEstimand(String),
    #[error("estimand references an unknown treatment strategy")]
    EstimandReferencesUnknownStrategy,
    #[error("estimand references an unknown outcome")]
    EstimandReferencesUnknownOutcome,
    #[error("identifying assumption is invalid")]
    InvalidIdentifyingAssumption,
    #[error("duplicate identifying assumption id: {0}")]
    DuplicateAssumption(String),
    #[error("observational emulation identity/statement is incomplete")]
    IncompleteEmulationIdentity,
    #[error("emulation plan does not bind the exact protocol digest")]
    ProtocolDigestMismatch,
    #[error("emulation requires at least one observational data source")]
    MissingDataSources,
    #[error("observational data source is invalid")]
    InvalidDataSource,
    #[error("duplicate observational data source id: {0}")]
    DuplicateDataSource(String),
    #[error("strategy operationalization is invalid or duplicated")]
    InvalidStrategyOperationalization,
    #[error("not all target-trial strategies have observational operationalizations")]
    IncompleteStrategyOperationalization,
    #[error("outcome operationalization is invalid or duplicated")]
    InvalidOutcomeOperationalization,
    #[error("not all target-trial outcomes have observational operationalizations")]
    IncompleteOutcomeOperationalization,
    #[error("baseline confounder is invalid")]
    InvalidConfounder,
    #[error("baseline confounder uses post-time-zero information")]
    PostBaselineConfounder,
    #[error("duplicate baseline confounder id: {0}")]
    DuplicateConfounder(String),
    #[error("canonical framing length exceeds v1 limit")]
    LengthOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(namespace: &str, id: &str, byte: u8) -> EvidenceArtifactIdentityV1 {
        EvidenceArtifactIdentityV1 {
            namespace: namespace.to_string(),
            artifact_id: id.to_string(),
            version: "1".to_string(),
            digest: [byte; 32],
        }
    }

    fn phenotype(id: &str, byte: u8) -> PhenotypeDefinitionRefV1 {
        PhenotypeDefinitionRefV1 {
            phenotype_id: id.to_string(),
            version: "1".to_string(),
            digest: [byte; 32],
        }
    }

    fn protocol() -> TargetTrialProtocolV1 {
        TargetTrialProtocolV1 {
            schema_version: TARGET_TRIAL_PROTOCOL_VERSION,
            protocol_id: "tte:test".to_string(),
            version: "1".to_string(),
            causal_question: "What is the 1-year risk difference under strategy A versus B?".to_string(),
            rationale_evidence: vec![artifact("literature", "rationale", 1)],
            eligibility: phenotype("eligible-adults", 2),
            treatment_strategies: vec![
                TreatmentStrategyV1 {
                    strategy_id: "a".to_string(),
                    label: "initiate A".to_string(),
                    intervention_definition: artifact("treatment", "a", 3),
                },
                TreatmentStrategyV1 {
                    strategy_id: "b".to_string(),
                    label: "initiate B".to_string(),
                    intervention_definition: artifact("treatment", "b", 4),
                },
            ],
            assignment: HypotheticalRandomAssignmentV1 {
                allocation_description: "1:1 individual random assignment".to_string(),
                masking: MaskingV1::OpenLabel,
            },
            time_zero_definition: artifact("study-design", "time-zero", 5),
            follow_up: FollowUpV1 {
                maximum_duration_micros: 365 * 24 * 60 * 60 * 1_000_000,
                end_conditions: vec![artifact("study-design", "follow-up-end", 6)],
            },
            outcomes: vec![OutcomeV1 {
                outcome_id: "outcome".to_string(),
                phenotype: phenotype("outcome", 7),
                assessment_window_start_offset_micros: 0,
                assessment_window_end_offset_micros: 365 * 24 * 60 * 60 * 1_000_000,
                primary: true,
            }],
            estimands: vec![CausalEstimandV1 {
                estimand_id: "itt-risk-difference".to_string(),
                treatment_strategy_id: "a".to_string(),
                comparator_strategy_id: "b".to_string(),
                outcome_id: "outcome".to_string(),
                contrast: CausalContrastV1::IntentionToTreat,
                effect_measure: EffectMeasureV1::RiskDifference,
                competing_event_strategy: CompetingEventStrategyV1::NotApplicable,
                estimand_definition: artifact("estimand", "itt-risk-difference", 8),
            }],
            identifying_assumptions: vec![IdentifyingAssumptionV1 {
                assumption_id: "follow-up".to_string(),
                statement: "loss to follow-up is handled by the declared analysis plan".to_string(),
                related_variable_definitions: vec![],
            }],
            analysis_plan: artifact("analysis-plan", "primary", 9),
            sensitivity_analysis_plans: vec![artifact("analysis-plan", "sensitivity", 10)],
        }
    }

    fn emulation(protocol: &TargetTrialProtocolV1) -> TargetTrialEmulationPlanV1 {
        TargetTrialEmulationPlanV1 {
            schema_version: TARGET_TRIAL_PROTOCOL_VERSION,
            emulation_id: "tte:test:omop".to_string(),
            version: "1".to_string(),
            protocol_digest: target_trial_protocol_digest_v1(protocol).unwrap().into_bytes(),
            observational_design_statement: "Observational classification; participants are not randomized.".to_string(),
            data_sources: vec![DataSourceV1 {
                source_id: "omop-site-a".to_string(),
                original_purpose: "routine clinical care".to_string(),
                source_type: "EHR-derived OMOP CDM".to_string(),
                setting: "health system".to_string(),
                geography: "site-a".to_string(),
                period_start_micros: 1,
                period_end_micros: 2,
                source_identity: artifact("omop/source", "site-a", 11),
            }],
            eligibility_operationalization: artifact("operationalization", "eligibility", 12),
            strategy_operationalizations: vec![
                StrategyOperationalizationV1 {
                    strategy_id: "a".to_string(),
                    classification_rule: artifact("operationalization", "strategy-a", 13),
                },
                StrategyOperationalizationV1 {
                    strategy_id: "b".to_string(),
                    classification_rule: artifact("operationalization", "strategy-b", 14),
                },
            ],
            time_zero_operationalization: artifact("operationalization", "time-zero", 15),
            outcome_operationalizations: vec![OutcomeOperationalizationV1 {
                outcome_id: "outcome".to_string(),
                operationalization: artifact("operationalization", "outcome", 16),
            }],
            baseline_confounders: vec![BaselineConfounderV1 {
                confounder_id: "age".to_string(),
                definition: artifact("confounder", "age", 17),
                measurement_window_end_offset_micros: 0,
            }],
            censoring_plan: artifact("analysis-plan", "censoring", 18),
            adherence_plan: None,
            time_varying_confounding_plan: None,
            missing_data_plan: artifact("analysis-plan", "missing", 19),
            estimator_plan: artifact("analysis-plan", "estimator", 20),
            software_environment: SoftwareEnvironmentV1 {
                software: artifact("software", "analysis", 21),
                environment: artifact("environment", "analysis", 22),
            },
            vocabulary_and_mapping_artifacts: vec![artifact("omop/vocabulary", "snapshot", 23)],
        }
    }

    #[test]
    fn protocol_and_emulation_identities_are_deterministic() {
        let protocol = protocol();
        let p1 = target_trial_protocol_digest_v1(&protocol).unwrap();
        let p2 = target_trial_protocol_digest_v1(&protocol).unwrap();
        assert_eq!(p1, p2);

        let plan = emulation(&protocol);
        let e1 = target_trial_emulation_plan_digest_v1(&protocol, &plan).unwrap();
        let e2 = target_trial_emulation_plan_digest_v1(&protocol, &plan).unwrap();
        assert_eq!(e1, e2);
    }

    #[test]
    fn emulation_cannot_rebind_to_different_protocol() {
        let protocol = protocol();
        let mut changed = protocol.clone();
        changed.causal_question.push_str(" changed");
        let plan = emulation(&protocol);
        assert!(matches!(
            plan.validate_against(&changed),
            Err(TargetTrialError::ProtocolDigestMismatch)
        ));
    }

    #[test]
    fn observational_strategy_mapping_must_cover_every_protocol_strategy_exactly_once() {
        let protocol = protocol();
        let mut plan = emulation(&protocol);
        plan.strategy_operationalizations.pop();
        assert!(matches!(
            plan.validate_against(&protocol),
            Err(TargetTrialError::IncompleteStrategyOperationalization)
        ));
    }

    #[test]
    fn outcome_operationalization_must_cover_every_protocol_outcome() {
        let protocol = protocol();
        let mut plan = emulation(&protocol);
        plan.outcome_operationalizations.clear();
        assert!(matches!(
            plan.validate_against(&protocol),
            Err(TargetTrialError::IncompleteOutcomeOperationalization)
        ));
    }

    #[test]
    fn post_baseline_confounder_definition_is_rejected() {
        let protocol = protocol();
        let mut plan = emulation(&protocol);
        plan.baseline_confounders[0].measurement_window_end_offset_micros = 1;
        assert!(matches!(
            plan.validate_against(&protocol),
            Err(TargetTrialError::PostBaselineConfounder)
        ));
    }

    #[test]
    fn estimand_must_reference_declared_strategy_and_outcome() {
        let mut protocol = protocol();
        protocol.estimands[0].treatment_strategy_id = "unknown".to_string();
        assert!(matches!(
            protocol.validate(),
            Err(TargetTrialError::EstimandReferencesUnknownStrategy)
        ));
    }

    #[test]
    fn target_trial_requires_primary_outcome() {
        let mut protocol = protocol();
        protocol.outcomes[0].primary = false;
        assert!(matches!(
            protocol.validate(),
            Err(TargetTrialError::MissingPrimaryOutcome)
        ));
    }

    #[test]
    fn observational_statement_is_part_of_emulation_identity() {
        let protocol = protocol();
        let original = emulation(&protocol);
        let mut changed = original.clone();
        changed.observational_design_statement =
            "Observational classification with a different design statement.".to_string();
        assert_ne!(
            target_trial_emulation_plan_digest_v1(&protocol, &original).unwrap(),
            target_trial_emulation_plan_digest_v1(&protocol, &changed).unwrap()
        );
    }
}
