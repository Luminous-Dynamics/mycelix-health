#![deny(unsafe_code)]
//! Research-only target-trial protocol/emulation identities using reusable
//! anchored relative phenotype definitions.
//!
//! V2 separates protocol semantics from participant-specific time-zero anchors
//! and coverage evidence. Subject-level evaluation receipts belong downstream.

use mycelix_clinical_phenotype_v2::{
    relative_phenotype_definition_digest_v2, RelativePhenotypeDefinitionV2,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub use mycelix_target_trial_protocol::{
    BaselineConfounderV1 as BaselineConfounderV2,
    CausalContrastV1 as CausalContrastV2,
    CompetingEventStrategyV1 as CompetingEventStrategyV2,
    DataSourceV1 as DataSourceV2,
    EffectMeasureV1 as EffectMeasureV2,
    EvidenceArtifactIdentityV1 as EvidenceArtifactIdentityV2,
    FollowUpV1 as FollowUpV2,
    HypotheticalRandomAssignmentV1 as HypotheticalRandomAssignmentV2,
    IdentifyingAssumptionV1 as IdentifyingAssumptionV2,
    MaskingV1 as MaskingV2,
    SoftwareEnvironmentV1 as SoftwareEnvironmentV2,
    StrategyOperationalizationV1 as StrategyOperationalizationV2,
    TreatmentStrategyV1 as TreatmentStrategyV2,
};

pub const TARGET_TRIAL_PROTOCOL_V2_VERSION: u16 = 2;
pub const RELATIVE_PHENOTYPE_V2_NAMESPACE: &str =
    "mycelix/clinical-phenotype-definition/v2";

const PROTOCOL_TAG: &[u8] = b"mycelix/target-trial-protocol/v2";
const EMULATION_TAG: &[u8] = b"mycelix/target-trial-emulation-plan/v2";
const PROTOCOL_CONTEXT: &str = "mycelix.health.target-trial-protocol.v2";
const EMULATION_CONTEXT: &str = "mycelix.health.target-trial-emulation-plan.v2";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelativePhenotypeRefV2 {
    pub namespace: String,
    pub phenotype_id: String,
    pub version: String,
    pub digest: [u8; 32],
}

impl RelativePhenotypeRefV2 {
    pub fn from_definition(
        definition: &RelativePhenotypeDefinitionV2,
    ) -> Result<Self, TargetTrialV2Error> {
        let digest = relative_phenotype_definition_digest_v2(definition)?.into_bytes();
        Ok(Self {
            namespace: RELATIVE_PHENOTYPE_V2_NAMESPACE.to_string(),
            phenotype_id: definition.phenotype_id.clone(),
            version: definition.version.clone(),
            digest,
        })
    }

    pub fn validate(&self) -> Result<(), TargetTrialV2Error> {
        if self.namespace != RELATIVE_PHENOTYPE_V2_NAMESPACE {
            return Err(TargetTrialV2Error::WrongRelativePhenotypeNamespace);
        }
        if self.phenotype_id.trim().is_empty() || self.version.trim().is_empty() {
            return Err(TargetTrialV2Error::IncompleteRelativePhenotypeIdentity);
        }
        if self.digest == [0; 32] {
            return Err(TargetTrialV2Error::ZeroEvidenceDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutcomeV2 {
    pub outcome_id: String,
    pub phenotype: RelativePhenotypeRefV2,
    pub primary: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CausalEstimandV2 {
    pub estimand_id: String,
    pub treatment_strategy_id: String,
    pub comparator_strategy_id: String,
    pub outcome_id: String,
    pub contrast: CausalContrastV2,
    pub effect_measure: EffectMeasureV2,
    pub competing_event_strategy: CompetingEventStrategyV2,
    pub estimand_definition: EvidenceArtifactIdentityV2,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetTrialProtocolV2 {
    pub schema_version: u16,
    pub protocol_id: String,
    pub version: String,
    pub causal_question: String,
    pub rationale_evidence: Vec<EvidenceArtifactIdentityV2>,
    pub eligibility: RelativePhenotypeRefV2,
    pub treatment_strategies: Vec<TreatmentStrategyV2>,
    pub assignment: HypotheticalRandomAssignmentV2,
    pub time_zero_definition: EvidenceArtifactIdentityV2,
    pub follow_up: FollowUpV2,
    pub outcomes: Vec<OutcomeV2>,
    pub estimands: Vec<CausalEstimandV2>,
    pub identifying_assumptions: Vec<IdentifyingAssumptionV2>,
    pub analysis_plan: EvidenceArtifactIdentityV2,
    pub sensitivity_analysis_plans: Vec<EvidenceArtifactIdentityV2>,
}

impl TargetTrialProtocolV2 {
    pub fn validate(&self) -> Result<(), TargetTrialV2Error> {
        if self.schema_version != TARGET_TRIAL_PROTOCOL_V2_VERSION {
            return Err(TargetTrialV2Error::UnsupportedSchemaVersion(self.schema_version));
        }
        if self.protocol_id.trim().is_empty()
            || self.version.trim().is_empty()
            || self.causal_question.trim().is_empty()
        {
            return Err(TargetTrialV2Error::IncompleteProtocolIdentity);
        }
        if self.rationale_evidence.is_empty() {
            return Err(TargetTrialV2Error::MissingRationaleEvidence);
        }
        for artifact in &self.rationale_evidence {
            validate_artifact(artifact)?;
        }
        self.eligibility.validate()?;
        if self.treatment_strategies.len() < 2 {
            return Err(TargetTrialV2Error::InsufficientTreatmentStrategies);
        }
        let mut strategy_ids = HashSet::new();
        for strategy in &self.treatment_strategies {
            validate_strategy(strategy)?;
            if !strategy_ids.insert(strategy.strategy_id.clone()) {
                return Err(TargetTrialV2Error::DuplicateTreatmentStrategy(
                    strategy.strategy_id.clone(),
                ));
            }
        }
        validate_assignment(&self.assignment)?;
        validate_artifact(&self.time_zero_definition)?;
        validate_follow_up(&self.follow_up)?;

        if self.outcomes.is_empty() {
            return Err(TargetTrialV2Error::MissingOutcomes);
        }
        let mut outcome_ids = HashSet::new();
        let mut primary_count = 0usize;
        for outcome in &self.outcomes {
            validate_outcome(outcome)?;
            if !outcome_ids.insert(outcome.outcome_id.clone()) {
                return Err(TargetTrialV2Error::DuplicateOutcome(outcome.outcome_id.clone()));
            }
            if outcome.primary {
                primary_count += 1;
            }
        }
        if primary_count == 0 {
            return Err(TargetTrialV2Error::MissingPrimaryOutcome);
        }

        if self.estimands.is_empty() {
            return Err(TargetTrialV2Error::MissingEstimands);
        }
        let mut estimand_ids = HashSet::new();
        for estimand in &self.estimands {
            validate_estimand(estimand)?;
            if !estimand_ids.insert(estimand.estimand_id.clone()) {
                return Err(TargetTrialV2Error::DuplicateEstimand(
                    estimand.estimand_id.clone(),
                ));
            }
            if !strategy_ids.contains(&estimand.treatment_strategy_id)
                || !strategy_ids.contains(&estimand.comparator_strategy_id)
            {
                return Err(TargetTrialV2Error::EstimandReferencesUnknownStrategy);
            }
            if !outcome_ids.contains(&estimand.outcome_id) {
                return Err(TargetTrialV2Error::EstimandReferencesUnknownOutcome);
            }
        }

        let mut assumption_ids = HashSet::new();
        for assumption in &self.identifying_assumptions {
            validate_assumption(assumption)?;
            if !assumption_ids.insert(assumption.assumption_id.clone()) {
                return Err(TargetTrialV2Error::DuplicateAssumption(
                    assumption.assumption_id.clone(),
                ));
            }
        }
        validate_artifact(&self.analysis_plan)?;
        for plan in &self.sensitivity_analysis_plans {
            validate_artifact(plan)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutcomeOperationalizationV2 {
    pub outcome_id: String,
    pub operationalization: EvidenceArtifactIdentityV2,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetTrialEmulationPlanV2 {
    pub schema_version: u16,
    pub emulation_id: String,
    pub version: String,
    pub protocol_digest: [u8; 32],
    pub observational_design_statement: String,
    pub data_sources: Vec<DataSourceV2>,
    pub eligibility_operationalization: EvidenceArtifactIdentityV2,
    pub strategy_operationalizations: Vec<StrategyOperationalizationV2>,
    pub time_zero_operationalization: EvidenceArtifactIdentityV2,
    pub outcome_operationalizations: Vec<OutcomeOperationalizationV2>,
    pub baseline_confounders: Vec<BaselineConfounderV2>,
    pub censoring_plan: EvidenceArtifactIdentityV2,
    pub adherence_plan: Option<EvidenceArtifactIdentityV2>,
    pub time_varying_confounding_plan: Option<EvidenceArtifactIdentityV2>,
    pub missing_data_plan: EvidenceArtifactIdentityV2,
    pub estimator_plan: EvidenceArtifactIdentityV2,
    pub software_environment: SoftwareEnvironmentV2,
    pub vocabulary_and_mapping_artifacts: Vec<EvidenceArtifactIdentityV2>,
}

impl TargetTrialEmulationPlanV2 {
    pub fn validate_against(
        &self,
        protocol: &TargetTrialProtocolV2,
    ) -> Result<(), TargetTrialV2Error> {
        protocol.validate()?;
        if self.schema_version != TARGET_TRIAL_PROTOCOL_V2_VERSION {
            return Err(TargetTrialV2Error::UnsupportedSchemaVersion(self.schema_version));
        }
        if self.emulation_id.trim().is_empty()
            || self.version.trim().is_empty()
            || self.observational_design_statement.trim().is_empty()
        {
            return Err(TargetTrialV2Error::IncompleteEmulationIdentity);
        }
        if self.protocol_digest != target_trial_protocol_digest_v2(protocol)?.into_bytes() {
            return Err(TargetTrialV2Error::ProtocolDigestMismatch);
        }
        if self.data_sources.is_empty() {
            return Err(TargetTrialV2Error::MissingDataSources);
        }
        let mut source_ids = HashSet::new();
        for source in &self.data_sources {
            validate_data_source(source)?;
            if !source_ids.insert(source.source_id.clone()) {
                return Err(TargetTrialV2Error::DuplicateDataSource(source.source_id.clone()));
            }
        }
        validate_artifact(&self.eligibility_operationalization)?;
        validate_artifact(&self.time_zero_operationalization)?;
        validate_artifact(&self.censoring_plan)?;
        validate_artifact(&self.missing_data_plan)?;
        validate_artifact(&self.estimator_plan)?;
        validate_software_environment(&self.software_environment)?;
        if let Some(plan) = &self.adherence_plan {
            validate_artifact(plan)?;
        }
        if let Some(plan) = &self.time_varying_confounding_plan {
            validate_artifact(plan)?;
        }
        for artifact in &self.vocabulary_and_mapping_artifacts {
            validate_artifact(artifact)?;
        }

        let strategy_ids: HashSet<_> = protocol
            .treatment_strategies
            .iter()
            .map(|strategy| strategy.strategy_id.as_str())
            .collect();
        let mut mapped_strategies = HashSet::new();
        for mapping in &self.strategy_operationalizations {
            if mapping.strategy_id.trim().is_empty()
                || !strategy_ids.contains(mapping.strategy_id.as_str())
                || !mapped_strategies.insert(mapping.strategy_id.clone())
            {
                return Err(TargetTrialV2Error::InvalidStrategyOperationalization);
            }
            validate_artifact(&mapping.classification_rule)?;
        }
        if mapped_strategies.len() != strategy_ids.len() {
            return Err(TargetTrialV2Error::IncompleteStrategyOperationalization);
        }

        let outcome_ids: HashSet<_> = protocol
            .outcomes
            .iter()
            .map(|outcome| outcome.outcome_id.as_str())
            .collect();
        let mut mapped_outcomes = HashSet::new();
        for mapping in &self.outcome_operationalizations {
            if mapping.outcome_id.trim().is_empty()
                || !outcome_ids.contains(mapping.outcome_id.as_str())
                || !mapped_outcomes.insert(mapping.outcome_id.clone())
            {
                return Err(TargetTrialV2Error::InvalidOutcomeOperationalization);
            }
            validate_artifact(&mapping.operationalization)?;
        }
        if mapped_outcomes.len() != outcome_ids.len() {
            return Err(TargetTrialV2Error::IncompleteOutcomeOperationalization);
        }

        let mut confounder_ids = HashSet::new();
        for confounder in &self.baseline_confounders {
            validate_confounder(confounder)?;
            if !confounder_ids.insert(confounder.confounder_id.clone()) {
                return Err(TargetTrialV2Error::DuplicateConfounder(
                    confounder.confounder_id.clone(),
                ));
            }
        }
        Ok(())
    }
}

macro_rules! digest_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name([u8; 32]);
        impl $name {
            #[must_use]
            pub const fn into_bytes(self) -> [u8; 32] {
                self.0
            }
        }
    };
}

digest_type!(TargetTrialProtocolDigestV2);
digest_type!(TargetTrialEmulationPlanDigestV2);

pub fn target_trial_protocol_digest_v2(
    protocol: &TargetTrialProtocolV2,
) -> Result<TargetTrialProtocolDigestV2, TargetTrialV2Error> {
    protocol.validate()?;
    let mut writer = CanonicalWriter::default();
    encode_protocol(&mut writer, protocol)?;
    Ok(TargetTrialProtocolDigestV2(domain_digest(
        PROTOCOL_CONTEXT,
        PROTOCOL_TAG,
        &writer.finish(),
    )))
}

pub fn target_trial_emulation_plan_digest_v2(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
) -> Result<TargetTrialEmulationPlanDigestV2, TargetTrialV2Error> {
    plan.validate_against(protocol)?;
    let mut writer = CanonicalWriter::default();
    encode_emulation(&mut writer, plan)?;
    Ok(TargetTrialEmulationPlanDigestV2(domain_digest(
        EMULATION_CONTEXT,
        EMULATION_TAG,
        &writer.finish(),
    )))
}

fn validate_artifact(value: &EvidenceArtifactIdentityV2) -> Result<(), TargetTrialV2Error> {
    if value.namespace.trim().is_empty()
        || value.artifact_id.trim().is_empty()
        || value.version.trim().is_empty()
    {
        return Err(TargetTrialV2Error::IncompleteEvidenceIdentity);
    }
    if value.digest == [0; 32] {
        return Err(TargetTrialV2Error::ZeroEvidenceDigest);
    }
    Ok(())
}

fn validate_strategy(value: &TreatmentStrategyV2) -> Result<(), TargetTrialV2Error> {
    if value.strategy_id.trim().is_empty() || value.label.trim().is_empty() {
        return Err(TargetTrialV2Error::InvalidTreatmentStrategy);
    }
    validate_artifact(&value.intervention_definition)
}

fn validate_assignment(value: &HypotheticalRandomAssignmentV2) -> Result<(), TargetTrialV2Error> {
    if value.allocation_description.trim().is_empty() {
        return Err(TargetTrialV2Error::MissingAssignmentDescription);
    }
    Ok(())
}

fn validate_follow_up(value: &FollowUpV2) -> Result<(), TargetTrialV2Error> {
    if value.maximum_duration_micros <= 0 {
        return Err(TargetTrialV2Error::InvalidFollowUpDuration);
    }
    if value.end_conditions.is_empty() {
        return Err(TargetTrialV2Error::MissingFollowUpEndCondition);
    }
    for condition in &value.end_conditions {
        validate_artifact(condition)?;
    }
    Ok(())
}

fn validate_outcome(value: &OutcomeV2) -> Result<(), TargetTrialV2Error> {
    if value.outcome_id.trim().is_empty() {
        return Err(TargetTrialV2Error::MissingOutcomeId);
    }
    value.phenotype.validate()
}

fn validate_estimand(value: &CausalEstimandV2) -> Result<(), TargetTrialV2Error> {
    if value.estimand_id.trim().is_empty()
        || value.treatment_strategy_id.trim().is_empty()
        || value.comparator_strategy_id.trim().is_empty()
        || value.outcome_id.trim().is_empty()
        || value.treatment_strategy_id == value.comparator_strategy_id
    {
        return Err(TargetTrialV2Error::InvalidEstimand);
    }
    validate_artifact(&value.estimand_definition)
}

fn validate_assumption(value: &IdentifyingAssumptionV2) -> Result<(), TargetTrialV2Error> {
    if value.assumption_id.trim().is_empty() || value.statement.trim().is_empty() {
        return Err(TargetTrialV2Error::InvalidIdentifyingAssumption);
    }
    for artifact in &value.related_variable_definitions {
        validate_artifact(artifact)?;
    }
    Ok(())
}

fn validate_data_source(value: &DataSourceV2) -> Result<(), TargetTrialV2Error> {
    if value.source_id.trim().is_empty()
        || value.original_purpose.trim().is_empty()
        || value.source_type.trim().is_empty()
        || value.setting.trim().is_empty()
        || value.geography.trim().is_empty()
        || value.period_start_micros >= value.period_end_micros
    {
        return Err(TargetTrialV2Error::InvalidDataSource);
    }
    validate_artifact(&value.source_identity)
}

fn validate_confounder(value: &BaselineConfounderV2) -> Result<(), TargetTrialV2Error> {
    if value.confounder_id.trim().is_empty() {
        return Err(TargetTrialV2Error::InvalidConfounder);
    }
    validate_artifact(&value.definition)?;
    if value.measurement_window_end_offset_micros > 0 {
        return Err(TargetTrialV2Error::PostBaselineConfounder);
    }
    Ok(())
}

fn validate_software_environment(value: &SoftwareEnvironmentV2) -> Result<(), TargetTrialV2Error> {
    validate_artifact(&value.software)?;
    validate_artifact(&value.environment)
}

fn domain_digest(context: &str, tag: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&TARGET_TRIAL_PROTOCOL_V2_VERSION.to_be_bytes());
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
    fn i64(&mut self, value: i64) { self.bytes.extend_from_slice(&value.to_be_bytes()); }
    fn string(&mut self, value: &str) -> Result<(), TargetTrialV2Error> { self.bytes(value.as_bytes()) }
    fn bytes(&mut self, value: &[u8]) -> Result<(), TargetTrialV2Error> {
        self.u32(u32::try_from(value.len()).map_err(|_| TargetTrialV2Error::LengthOverflow)?);
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn vector_len(&mut self, len: usize) -> Result<(), TargetTrialV2Error> {
        self.u32(u32::try_from(len).map_err(|_| TargetTrialV2Error::LengthOverflow)?);
        Ok(())
    }
}

fn encode_artifact(writer: &mut CanonicalWriter, value: &EvidenceArtifactIdentityV2) -> Result<(), TargetTrialV2Error> {
    writer.string(&value.namespace)?;
    writer.string(&value.artifact_id)?;
    writer.string(&value.version)?;
    writer.bytes(&value.digest)?;
    Ok(())
}

fn encode_phenotype(writer: &mut CanonicalWriter, value: &RelativePhenotypeRefV2) -> Result<(), TargetTrialV2Error> {
    writer.string(&value.namespace)?;
    writer.string(&value.phenotype_id)?;
    writer.string(&value.version)?;
    writer.bytes(&value.digest)?;
    Ok(())
}

fn encode_strategy(writer: &mut CanonicalWriter, value: &TreatmentStrategyV2) -> Result<(), TargetTrialV2Error> {
    writer.string(&value.strategy_id)?;
    writer.string(&value.label)?;
    encode_artifact(writer, &value.intervention_definition)
}

fn encode_follow_up(writer: &mut CanonicalWriter, value: &FollowUpV2) -> Result<(), TargetTrialV2Error> {
    writer.i64(value.maximum_duration_micros);
    writer.vector_len(value.end_conditions.len())?;
    for condition in &value.end_conditions { encode_artifact(writer, condition)?; }
    Ok(())
}

fn encode_protocol(writer: &mut CanonicalWriter, value: &TargetTrialProtocolV2) -> Result<(), TargetTrialV2Error> {
    writer.u16(value.schema_version);
    writer.string(&value.protocol_id)?;
    writer.string(&value.version)?;
    writer.string(&value.causal_question)?;
    writer.vector_len(value.rationale_evidence.len())?;
    for artifact in &value.rationale_evidence { encode_artifact(writer, artifact)?; }
    encode_phenotype(writer, &value.eligibility)?;
    writer.vector_len(value.treatment_strategies.len())?;
    for strategy in &value.treatment_strategies { encode_strategy(writer, strategy)?; }
    writer.string(&value.assignment.allocation_description)?;
    writer.u8(match value.assignment.masking {
        MaskingV2::OpenLabel => 0,
        MaskingV2::SingleBlind => 1,
        MaskingV2::DoubleBlind => 2,
        MaskingV2::Other => 3,
    });
    encode_artifact(writer, &value.time_zero_definition)?;
    encode_follow_up(writer, &value.follow_up)?;
    writer.vector_len(value.outcomes.len())?;
    for outcome in &value.outcomes {
        writer.string(&outcome.outcome_id)?;
        encode_phenotype(writer, &outcome.phenotype)?;
        writer.u8(u8::from(outcome.primary));
    }
    writer.vector_len(value.estimands.len())?;
    for estimand in &value.estimands {
        writer.string(&estimand.estimand_id)?;
        writer.string(&estimand.treatment_strategy_id)?;
        writer.string(&estimand.comparator_strategy_id)?;
        writer.string(&estimand.outcome_id)?;
        writer.u8(match estimand.contrast {
            CausalContrastV2::IntentionToTreat => 0,
            CausalContrastV2::PerProtocol => 1,
        });
        writer.u8(match estimand.effect_measure {
            EffectMeasureV2::RiskDifference => 0,
            EffectMeasureV2::RiskRatio => 1,
            EffectMeasureV2::HazardRatio => 2,
            EffectMeasureV2::MeanDifference => 3,
            EffectMeasureV2::SurvivalDifference => 4,
            EffectMeasureV2::Other => 5,
        });
        writer.u8(match estimand.competing_event_strategy {
            CompetingEventStrategyV2::CensorAtCompetingEvent => 0,
            CompetingEventStrategyV2::CompositeOutcome => 1,
            CompetingEventStrategyV2::CompetingRiskEstimand => 2,
            CompetingEventStrategyV2::IgnoreWhenScientificallyJustified => 3,
            CompetingEventStrategyV2::NotApplicable => 4,
        });
        encode_artifact(writer, &estimand.estimand_definition)?;
    }
    writer.vector_len(value.identifying_assumptions.len())?;
    for assumption in &value.identifying_assumptions {
        writer.string(&assumption.assumption_id)?;
        writer.string(&assumption.statement)?;
        writer.vector_len(assumption.related_variable_definitions.len())?;
        for artifact in &assumption.related_variable_definitions { encode_artifact(writer, artifact)?; }
    }
    encode_artifact(writer, &value.analysis_plan)?;
    writer.vector_len(value.sensitivity_analysis_plans.len())?;
    for artifact in &value.sensitivity_analysis_plans { encode_artifact(writer, artifact)?; }
    Ok(())
}

fn encode_data_source(writer: &mut CanonicalWriter, value: &DataSourceV2) -> Result<(), TargetTrialV2Error> {
    writer.string(&value.source_id)?;
    writer.string(&value.original_purpose)?;
    writer.string(&value.source_type)?;
    writer.string(&value.setting)?;
    writer.string(&value.geography)?;
    writer.i64(value.period_start_micros);
    writer.i64(value.period_end_micros);
    encode_artifact(writer, &value.source_identity)
}

fn encode_optional_artifact(writer: &mut CanonicalWriter, value: Option<&EvidenceArtifactIdentityV2>) -> Result<(), TargetTrialV2Error> {
    match value {
        Some(value) => { writer.u8(1); encode_artifact(writer, value)?; }
        None => writer.u8(0),
    }
    Ok(())
}

fn encode_emulation(writer: &mut CanonicalWriter, value: &TargetTrialEmulationPlanV2) -> Result<(), TargetTrialV2Error> {
    writer.u16(value.schema_version);
    writer.string(&value.emulation_id)?;
    writer.string(&value.version)?;
    writer.bytes(&value.protocol_digest)?;
    writer.string(&value.observational_design_statement)?;
    writer.vector_len(value.data_sources.len())?;
    for source in &value.data_sources { encode_data_source(writer, source)?; }
    encode_artifact(writer, &value.eligibility_operationalization)?;
    writer.vector_len(value.strategy_operationalizations.len())?;
    for mapping in &value.strategy_operationalizations {
        writer.string(&mapping.strategy_id)?;
        encode_artifact(writer, &mapping.classification_rule)?;
    }
    encode_artifact(writer, &value.time_zero_operationalization)?;
    writer.vector_len(value.outcome_operationalizations.len())?;
    for mapping in &value.outcome_operationalizations {
        writer.string(&mapping.outcome_id)?;
        encode_artifact(writer, &mapping.operationalization)?;
    }
    writer.vector_len(value.baseline_confounders.len())?;
    for confounder in &value.baseline_confounders {
        writer.string(&confounder.confounder_id)?;
        encode_artifact(writer, &confounder.definition)?;
        writer.i64(confounder.measurement_window_end_offset_micros);
    }
    encode_artifact(writer, &value.censoring_plan)?;
    encode_optional_artifact(writer, value.adherence_plan.as_ref())?;
    encode_optional_artifact(writer, value.time_varying_confounding_plan.as_ref())?;
    encode_artifact(writer, &value.missing_data_plan)?;
    encode_artifact(writer, &value.estimator_plan)?;
    encode_artifact(writer, &value.software_environment.software)?;
    encode_artifact(writer, &value.software_environment.environment)?;
    writer.vector_len(value.vocabulary_and_mapping_artifacts.len())?;
    for artifact in &value.vocabulary_and_mapping_artifacts { encode_artifact(writer, artifact)?; }
    Ok(())
}

#[derive(Debug, Error)]
pub enum TargetTrialV2Error {
    #[error("unsupported target-trial v2 schema version: {0}")]
    UnsupportedSchemaVersion(u16),
    #[error("evidence artifact identity is incomplete")]
    IncompleteEvidenceIdentity,
    #[error("evidence digest may not be all zeroes")]
    ZeroEvidenceDigest,
    #[error("relative phenotype reference uses the wrong namespace")]
    WrongRelativePhenotypeNamespace,
    #[error("relative phenotype identity is incomplete")]
    IncompleteRelativePhenotypeIdentity,
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
    #[error("canonical framing length exceeds v2 limit")]
    LengthOverflow,
    #[error(transparent)]
    Phenotype(#[from] mycelix_clinical_phenotype_v2::PhenotypeV2Error),
}
