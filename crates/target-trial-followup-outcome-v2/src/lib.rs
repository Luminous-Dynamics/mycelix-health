#![deny(unsafe_code)]
//! Research-only target-trial follow-up and outcome-ascertainment receipts v2.
//!
//! Follow-up is represented as a half-open observation interval
//! `[time_zero, observed_until)`. Outcome ascertainment uses the exact relative
//! phenotype definition from the protocol, anchored to the exact time-zero
//! receipt, and never turns truncated follow-up into invented negative evidence.

use mycelix_clinical_phenotype_v2::{
    evaluate_relative_phenotype_v2, phenotype_evaluation_context_digest_v2,
    relative_phenotype_definition_digest_v2, relative_phenotype_evaluation_digest_v2,
    CoverageStatusV2, CriterionStateV2, PhenotypeEvaluationContextV2,
    RelativePhenotypeDefinitionV2,
};
use mycelix_clinical_semantics::ClinicalFact;
use mycelix_target_trial_cohort_receipts_v2::{
    cohort_entry_receipt_digest_v2, time_zero_anchor_evidence_v2,
    time_zero_receipt_digest_v2, CohortEntryReceiptV2, TimeZeroReceiptV2,
};
use mycelix_target_trial_protocol_v2::{
    target_trial_emulation_plan_digest_v2, target_trial_protocol_digest_v2,
    EvidenceArtifactIdentityV2, OutcomeV2, TargetTrialEmulationPlanV2,
    TargetTrialProtocolV2,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const TARGET_TRIAL_FOLLOWUP_OUTCOME_V2_VERSION: u16 = 2;

const FOLLOWUP_TAG: &[u8] = b"mycelix/target-trial-followup-receipt/v2";
const OUTCOME_TAG: &[u8] = b"mycelix/target-trial-outcome-ascertainment-receipt/v2";
const FOLLOWUP_CONTEXT: &str = "mycelix.health.target-trial-followup-receipt.v2";
const OUTCOME_CONTEXT: &str = "mycelix.health.target-trial-outcome-ascertainment-receipt.v2";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FollowUpEndV2 {
    PlannedHorizonComplete,
    ProtocolEndCondition {
        end_condition: EvidenceArtifactIdentityV2,
        event_evidence: EvidenceArtifactIdentityV2,
    },
    ObservationalCensoring {
        censoring_plan: EvidenceArtifactIdentityV2,
        event_evidence: EvidenceArtifactIdentityV2,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FollowUpReceiptV2 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub subject_resource_type: String,
    pub subject_id: String,
    pub strategy_id: String,
    pub cohort_entry_digest: [u8; 32],
    pub time_zero_receipt_digest: [u8; 32],
    pub time_zero_micros: i64,
    pub planned_horizon_end_micros: i64,
    pub observed_until_micros: i64,
    pub end: FollowUpEndV2,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutcomeAscertainmentReceiptV2 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub subject_resource_type: String,
    pub subject_id: String,
    pub strategy_id: String,
    pub cohort_entry_digest: [u8; 32],
    pub time_zero_receipt_digest: [u8; 32],
    pub follow_up_receipt_digest: [u8; 32],
    pub outcome_id: String,
    pub phenotype_definition_digest: [u8; 32],
    pub evaluation_context_digest: [u8; 32],
    pub phenotype_evaluation_digest: [u8; 32],
    pub outcome_operationalization: EvidenceArtifactIdentityV2,
    pub window_start_micros: i64,
    pub window_end_micros: i64,
    pub state: CriterionStateV2,
    pub evidence_fact_digests: Vec<[u8; 32]>,
}

macro_rules! digest_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name([u8; 32]);
        impl $name {
            #[must_use]
            pub const fn into_bytes(self) -> [u8; 32] { self.0 }
        }
    };
}

digest_type!(FollowUpReceiptDigestV2);
digest_type!(OutcomeAscertainmentReceiptDigestV2);

pub fn build_follow_up_receipt_v2(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    cohort_entry: &CohortEntryReceiptV2,
    time_zero: &TimeZeroReceiptV2,
    observed_until_micros: i64,
    end: FollowUpEndV2,
) -> Result<FollowUpReceiptV2, FollowUpOutcomeV2Error> {
    plan.validate_against(protocol)?;
    verify_entry_coordinates(protocol, plan, cohort_entry, time_zero)?;
    let horizon = time_zero
        .time_zero_micros
        .checked_add(protocol.follow_up.maximum_duration_micros)
        .ok_or(FollowUpOutcomeV2Error::TimeOverflow)?;
    if observed_until_micros <= time_zero.time_zero_micros || observed_until_micros > horizon {
        return Err(FollowUpOutcomeV2Error::InvalidObservedFollowUpEnd);
    }
    validate_follow_up_end(protocol, plan, observed_until_micros, horizon, &end)?;
    let receipt = FollowUpReceiptV2 {
        schema_version: TARGET_TRIAL_FOLLOWUP_OUTCOME_V2_VERSION,
        protocol_digest: target_trial_protocol_digest_v2(protocol)?.into_bytes(),
        emulation_plan_digest: target_trial_emulation_plan_digest_v2(protocol, plan)?.into_bytes(),
        subject_resource_type: cohort_entry.subject.resource_type.clone(),
        subject_id: cohort_entry.subject.id.clone(),
        strategy_id: cohort_entry.strategy_id.clone(),
        cohort_entry_digest: cohort_entry_receipt_digest_v2(cohort_entry)?.into_bytes(),
        time_zero_receipt_digest: time_zero_receipt_digest_v2(time_zero)?.into_bytes(),
        time_zero_micros: time_zero.time_zero_micros,
        planned_horizon_end_micros: horizon,
        observed_until_micros,
        end,
    };
    verify_follow_up_receipt_v2(protocol, plan, cohort_entry, time_zero, &receipt)?;
    Ok(receipt)
}

pub fn verify_follow_up_receipt_v2(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    cohort_entry: &CohortEntryReceiptV2,
    time_zero: &TimeZeroReceiptV2,
    receipt: &FollowUpReceiptV2,
) -> Result<(), FollowUpOutcomeV2Error> {
    plan.validate_against(protocol)?;
    validate_follow_up_shape(receipt)?;
    verify_entry_coordinates(protocol, plan, cohort_entry, time_zero)?;
    let protocol_digest = target_trial_protocol_digest_v2(protocol)?.into_bytes();
    let plan_digest = target_trial_emulation_plan_digest_v2(protocol, plan)?.into_bytes();
    if receipt.protocol_digest != protocol_digest || receipt.emulation_plan_digest != plan_digest {
        return Err(FollowUpOutcomeV2Error::ProtocolOrEmulationMismatch);
    }
    if receipt.subject_resource_type != cohort_entry.subject.resource_type
        || receipt.subject_id != cohort_entry.subject.id
        || receipt.strategy_id != cohort_entry.strategy_id
    {
        return Err(FollowUpOutcomeV2Error::SubjectOrStrategyMismatch);
    }
    if receipt.cohort_entry_digest != cohort_entry_receipt_digest_v2(cohort_entry)?.into_bytes() {
        return Err(FollowUpOutcomeV2Error::CohortEntryDigestMismatch);
    }
    if receipt.time_zero_receipt_digest != time_zero_receipt_digest_v2(time_zero)?.into_bytes()
        || receipt.time_zero_micros != time_zero.time_zero_micros
    {
        return Err(FollowUpOutcomeV2Error::TimeZeroReceiptMismatch);
    }
    let horizon = time_zero
        .time_zero_micros
        .checked_add(protocol.follow_up.maximum_duration_micros)
        .ok_or(FollowUpOutcomeV2Error::TimeOverflow)?;
    if receipt.planned_horizon_end_micros != horizon
        || receipt.observed_until_micros <= receipt.time_zero_micros
        || receipt.observed_until_micros > horizon
    {
        return Err(FollowUpOutcomeV2Error::InvalidObservedFollowUpEnd);
    }
    validate_follow_up_end(protocol, plan, receipt.observed_until_micros, horizon, &receipt.end)
}

pub fn build_outcome_ascertainment_receipt_v2(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    cohort_entry: &CohortEntryReceiptV2,
    time_zero: &TimeZeroReceiptV2,
    follow_up: &FollowUpReceiptV2,
    outcome_id: &str,
    definition: &RelativePhenotypeDefinitionV2,
    context: &PhenotypeEvaluationContextV2,
    facts: &[ClinicalFact],
) -> Result<OutcomeAscertainmentReceiptV2, FollowUpOutcomeV2Error> {
    verify_follow_up_receipt_v2(protocol, plan, cohort_entry, time_zero, follow_up)?;
    let outcome = find_outcome(protocol, outcome_id)?;
    verify_outcome_definition(outcome, definition)?;
    let operationalization = plan
        .outcome_operationalizations
        .iter()
        .find(|mapping| mapping.outcome_id == outcome_id)
        .ok_or_else(|| FollowUpOutcomeV2Error::UnknownOutcomeOperationalization(outcome_id.to_string()))?;

    if context.anchor_micros != time_zero.time_zero_micros {
        return Err(FollowUpOutcomeV2Error::OutcomeAnchorTimeMismatch);
    }
    if context.anchor_evidence != time_zero_anchor_evidence_v2(time_zero)? {
        return Err(FollowUpOutcomeV2Error::OutcomeAnchorEvidenceMismatch);
    }
    let window_start = context
        .anchor_micros
        .checked_add(definition.window.start_offset_micros)
        .ok_or(FollowUpOutcomeV2Error::TimeOverflow)?;
    let window_end = context
        .anchor_micros
        .checked_add(definition.window.end_offset_micros)
        .ok_or(FollowUpOutcomeV2Error::TimeOverflow)?;
    if window_start >= window_end {
        return Err(FollowUpOutcomeV2Error::InvalidDerivedOutcomeWindow);
    }
    if follow_up.observed_until_micros < window_end
        && context.coverage.iter().any(|coverage| coverage.status == CoverageStatusV2::Complete)
    {
        return Err(FollowUpOutcomeV2Error::CompleteCoverageAfterTruncatedFollowUp);
    }

    let mut observable_facts = Vec::new();
    for fact in facts {
        fact.validate_machine_actionable()?;
        if fact.subject.resource_type != cohort_entry.subject.resource_type
            || fact.subject.id != cohort_entry.subject.id
        {
            return Err(FollowUpOutcomeV2Error::MixedSubjectFacts);
        }
        if fact.effective_at_micros < follow_up.observed_until_micros {
            observable_facts.push(fact.clone());
        }
    }

    let evaluation = evaluate_relative_phenotype_v2(
        definition,
        context,
        &cohort_entry.subject.id,
        &observable_facts,
    )?;
    if evaluation.window_start_micros != window_start || evaluation.window_end_micros != window_end {
        return Err(FollowUpOutcomeV2Error::OutcomeWindowMismatch);
    }
    let receipt = OutcomeAscertainmentReceiptV2 {
        schema_version: TARGET_TRIAL_FOLLOWUP_OUTCOME_V2_VERSION,
        protocol_digest: follow_up.protocol_digest,
        emulation_plan_digest: follow_up.emulation_plan_digest,
        subject_resource_type: follow_up.subject_resource_type.clone(),
        subject_id: follow_up.subject_id.clone(),
        strategy_id: follow_up.strategy_id.clone(),
        cohort_entry_digest: follow_up.cohort_entry_digest,
        time_zero_receipt_digest: follow_up.time_zero_receipt_digest,
        follow_up_receipt_digest: follow_up_receipt_digest_v2(follow_up)?.into_bytes(),
        outcome_id: outcome_id.to_string(),
        phenotype_definition_digest: relative_phenotype_definition_digest_v2(definition)?.into_bytes(),
        evaluation_context_digest: phenotype_evaluation_context_digest_v2(definition, context)?.into_bytes(),
        phenotype_evaluation_digest: relative_phenotype_evaluation_digest_v2(&evaluation)?.into_bytes(),
        outcome_operationalization: operationalization.operationalization.clone(),
        window_start_micros: window_start,
        window_end_micros: window_end,
        state: evaluation.state,
        evidence_fact_digests: evaluation.evidence_fact_digests,
    };
    validate_outcome_shape(&receipt)?;
    Ok(receipt)
}

#[allow(clippy::too_many_arguments)]
pub fn verify_outcome_ascertainment_receipt_v2(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    cohort_entry: &CohortEntryReceiptV2,
    time_zero: &TimeZeroReceiptV2,
    follow_up: &FollowUpReceiptV2,
    outcome_id: &str,
    definition: &RelativePhenotypeDefinitionV2,
    context: &PhenotypeEvaluationContextV2,
    facts: &[ClinicalFact],
    receipt: &OutcomeAscertainmentReceiptV2,
) -> Result<(), FollowUpOutcomeV2Error> {
    validate_outcome_shape(receipt)?;
    let rebuilt = build_outcome_ascertainment_receipt_v2(
        protocol,
        plan,
        cohort_entry,
        time_zero,
        follow_up,
        outcome_id,
        definition,
        context,
        facts,
    )?;
    if outcome_ascertainment_receipt_digest_v2(&rebuilt)?
        != outcome_ascertainment_receipt_digest_v2(receipt)?
    {
        return Err(FollowUpOutcomeV2Error::OutcomeReceiptMismatch);
    }
    Ok(())
}

pub fn follow_up_receipt_digest_v2(
    receipt: &FollowUpReceiptV2,
) -> Result<FollowUpReceiptDigestV2, FollowUpOutcomeV2Error> {
    validate_follow_up_shape(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_follow_up(&mut writer, receipt)?;
    Ok(FollowUpReceiptDigestV2(domain_digest(
        FOLLOWUP_CONTEXT,
        FOLLOWUP_TAG,
        &writer.finish(),
    )))
}

pub fn outcome_ascertainment_receipt_digest_v2(
    receipt: &OutcomeAscertainmentReceiptV2,
) -> Result<OutcomeAscertainmentReceiptDigestV2, FollowUpOutcomeV2Error> {
    validate_outcome_shape(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_outcome(&mut writer, receipt)?;
    Ok(OutcomeAscertainmentReceiptDigestV2(domain_digest(
        OUTCOME_CONTEXT,
        OUTCOME_TAG,
        &writer.finish(),
    )))
}

fn verify_entry_coordinates(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    entry: &CohortEntryReceiptV2,
    time_zero: &TimeZeroReceiptV2,
) -> Result<(), FollowUpOutcomeV2Error> {
    let protocol_digest = target_trial_protocol_digest_v2(protocol)?.into_bytes();
    let plan_digest = target_trial_emulation_plan_digest_v2(protocol, plan)?.into_bytes();
    if entry.protocol_digest != protocol_digest
        || entry.emulation_plan_digest != plan_digest
        || time_zero.protocol_digest != protocol_digest
        || time_zero.emulation_plan_digest != plan_digest
    {
        return Err(FollowUpOutcomeV2Error::ProtocolOrEmulationMismatch);
    }
    if entry.subject != time_zero.subject
        || entry.time_zero_micros != time_zero.time_zero_micros
        || entry.data_source_id != time_zero.data_source_id
        || entry.time_zero_receipt_digest != time_zero_receipt_digest_v2(time_zero)?.into_bytes()
    {
        return Err(FollowUpOutcomeV2Error::TimeZeroReceiptMismatch);
    }
    Ok(())
}

fn validate_follow_up_end(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    observed_until: i64,
    horizon: i64,
    end: &FollowUpEndV2,
) -> Result<(), FollowUpOutcomeV2Error> {
    match end {
        FollowUpEndV2::PlannedHorizonComplete => {
            if observed_until != horizon {
                return Err(FollowUpOutcomeV2Error::PlannedHorizonMismatch);
            }
        }
        FollowUpEndV2::ProtocolEndCondition {
            end_condition,
            event_evidence,
        } => {
            validate_artifact(end_condition)?;
            validate_artifact(event_evidence)?;
            if !protocol.follow_up.end_conditions.iter().any(|declared| declared == end_condition) {
                return Err(FollowUpOutcomeV2Error::UndeclaredProtocolEndCondition);
            }
        }
        FollowUpEndV2::ObservationalCensoring {
            censoring_plan,
            event_evidence,
        } => {
            validate_artifact(censoring_plan)?;
            validate_artifact(event_evidence)?;
            if censoring_plan != &plan.censoring_plan {
                return Err(FollowUpOutcomeV2Error::CensoringPlanMismatch);
            }
        }
    }
    Ok(())
}

fn find_outcome<'a>(
    protocol: &'a TargetTrialProtocolV2,
    outcome_id: &str,
) -> Result<&'a OutcomeV2, FollowUpOutcomeV2Error> {
    protocol
        .outcomes
        .iter()
        .find(|outcome| outcome.outcome_id == outcome_id)
        .ok_or_else(|| FollowUpOutcomeV2Error::UnknownOutcome(outcome_id.to_string()))
}

fn verify_outcome_definition(
    outcome: &OutcomeV2,
    definition: &RelativePhenotypeDefinitionV2,
) -> Result<(), FollowUpOutcomeV2Error> {
    let digest = relative_phenotype_definition_digest_v2(definition)?.into_bytes();
    if outcome.phenotype.phenotype_id != definition.phenotype_id
        || outcome.phenotype.version != definition.version
        || outcome.phenotype.digest != digest
    {
        return Err(FollowUpOutcomeV2Error::OutcomeDefinitionMismatch);
    }
    Ok(())
}

fn validate_artifact(value: &EvidenceArtifactIdentityV2) -> Result<(), FollowUpOutcomeV2Error> {
    if value.namespace.trim().is_empty()
        || value.artifact_id.trim().is_empty()
        || value.version.trim().is_empty()
    {
        return Err(FollowUpOutcomeV2Error::IncompleteEvidenceIdentity);
    }
    if value.digest == [0; 32] {
        return Err(FollowUpOutcomeV2Error::ZeroEvidenceDigest);
    }
    Ok(())
}

fn validate_follow_up_shape(value: &FollowUpReceiptV2) -> Result<(), FollowUpOutcomeV2Error> {
    if value.schema_version != TARGET_TRIAL_FOLLOWUP_OUTCOME_V2_VERSION
        || value.protocol_digest == [0; 32]
        || value.emulation_plan_digest == [0; 32]
        || value.cohort_entry_digest == [0; 32]
        || value.time_zero_receipt_digest == [0; 32]
        || value.subject_resource_type.trim().is_empty()
        || value.subject_id.trim().is_empty()
        || value.strategy_id.trim().is_empty()
        || value.observed_until_micros <= value.time_zero_micros
        || value.planned_horizon_end_micros < value.observed_until_micros
    {
        return Err(FollowUpOutcomeV2Error::InvalidFollowUpReceipt);
    }
    Ok(())
}

fn validate_outcome_shape(value: &OutcomeAscertainmentReceiptV2) -> Result<(), FollowUpOutcomeV2Error> {
    if value.schema_version != TARGET_TRIAL_FOLLOWUP_OUTCOME_V2_VERSION
        || value.protocol_digest == [0; 32]
        || value.emulation_plan_digest == [0; 32]
        || value.cohort_entry_digest == [0; 32]
        || value.time_zero_receipt_digest == [0; 32]
        || value.follow_up_receipt_digest == [0; 32]
        || value.phenotype_definition_digest == [0; 32]
        || value.evaluation_context_digest == [0; 32]
        || value.phenotype_evaluation_digest == [0; 32]
        || value.subject_resource_type.trim().is_empty()
        || value.subject_id.trim().is_empty()
        || value.strategy_id.trim().is_empty()
        || value.outcome_id.trim().is_empty()
        || value.window_start_micros >= value.window_end_micros
    {
        return Err(FollowUpOutcomeV2Error::InvalidOutcomeReceipt);
    }
    validate_artifact(&value.outcome_operationalization)
}

fn domain_digest(context: &str, tag: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&TARGET_TRIAL_FOLLOWUP_OUTCOME_V2_VERSION.to_be_bytes());
    hasher.update(&(tag.len() as u16).to_be_bytes());
    hasher.update(tag);
    hasher.update(&(payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    *hasher.finalize().as_bytes()
}

#[derive(Default)]
struct CanonicalWriter { bytes: Vec<u8> }
impl CanonicalWriter {
    fn finish(self) -> Vec<u8> { self.bytes }
    fn u8(&mut self, value: u8) { self.bytes.push(value); }
    fn u16(&mut self, value: u16) { self.bytes.extend_from_slice(&value.to_be_bytes()); }
    fn u32(&mut self, value: u32) { self.bytes.extend_from_slice(&value.to_be_bytes()); }
    fn i64(&mut self, value: i64) { self.bytes.extend_from_slice(&value.to_be_bytes()); }
    fn string(&mut self, value: &str) -> Result<(), FollowUpOutcomeV2Error> { self.bytes(value.as_bytes()) }
    fn bytes(&mut self, value: &[u8]) -> Result<(), FollowUpOutcomeV2Error> { self.u32(u32::try_from(value.len()).map_err(|_| FollowUpOutcomeV2Error::LengthOverflow)?); self.bytes.extend_from_slice(value); Ok(()) }
    fn vector_len(&mut self, len: usize) -> Result<(), FollowUpOutcomeV2Error> { self.u32(u32::try_from(len).map_err(|_| FollowUpOutcomeV2Error::LengthOverflow)?); Ok(()) }
}

fn encode_artifact(writer: &mut CanonicalWriter, value: &EvidenceArtifactIdentityV2) -> Result<(), FollowUpOutcomeV2Error> { writer.string(&value.namespace)?; writer.string(&value.artifact_id)?; writer.string(&value.version)?; writer.bytes(&value.digest) }
fn encode_follow_up(writer: &mut CanonicalWriter, value: &FollowUpReceiptV2) -> Result<(), FollowUpOutcomeV2Error> {
    writer.u16(value.schema_version); writer.bytes(&value.protocol_digest)?; writer.bytes(&value.emulation_plan_digest)?; writer.string(&value.subject_resource_type)?; writer.string(&value.subject_id)?; writer.string(&value.strategy_id)?; writer.bytes(&value.cohort_entry_digest)?; writer.bytes(&value.time_zero_receipt_digest)?; writer.i64(value.time_zero_micros); writer.i64(value.planned_horizon_end_micros); writer.i64(value.observed_until_micros);
    match &value.end {
        FollowUpEndV2::PlannedHorizonComplete => writer.u8(0),
        FollowUpEndV2::ProtocolEndCondition { end_condition, event_evidence } => { writer.u8(1); encode_artifact(writer, end_condition)?; encode_artifact(writer, event_evidence)?; },
        FollowUpEndV2::ObservationalCensoring { censoring_plan, event_evidence } => { writer.u8(2); encode_artifact(writer, censoring_plan)?; encode_artifact(writer, event_evidence)?; },
    }
    Ok(())
}
fn encode_outcome(writer: &mut CanonicalWriter, value: &OutcomeAscertainmentReceiptV2) -> Result<(), FollowUpOutcomeV2Error> {
    writer.u16(value.schema_version); writer.bytes(&value.protocol_digest)?; writer.bytes(&value.emulation_plan_digest)?; writer.string(&value.subject_resource_type)?; writer.string(&value.subject_id)?; writer.string(&value.strategy_id)?; writer.bytes(&value.cohort_entry_digest)?; writer.bytes(&value.time_zero_receipt_digest)?; writer.bytes(&value.follow_up_receipt_digest)?; writer.string(&value.outcome_id)?; writer.bytes(&value.phenotype_definition_digest)?; writer.bytes(&value.evaluation_context_digest)?; writer.bytes(&value.phenotype_evaluation_digest)?; encode_artifact(writer, &value.outcome_operationalization)?; writer.i64(value.window_start_micros); writer.i64(value.window_end_micros); writer.u8(match value.state { CriterionStateV2::Satisfied => 0, CriterionStateV2::NotSatisfied => 1, CriterionStateV2::Indeterminate => 2 }); writer.vector_len(value.evidence_fact_digests.len())?; for digest in &value.evidence_fact_digests { writer.bytes(digest)?; } Ok(())
}

#[derive(Debug, Error)]
pub enum FollowUpOutcomeV2Error {
    #[error("protocol or emulation identity mismatch")]
    ProtocolOrEmulationMismatch,
    #[error("cohort entry and time-zero receipt do not identify the same contribution")]
    TimeZeroReceiptMismatch,
    #[error("follow-up subject or strategy differs from cohort entry")]
    SubjectOrStrategyMismatch,
    #[error("cohort-entry digest mismatch")]
    CohortEntryDigestMismatch,
    #[error("follow-up end is outside the valid interval")]
    InvalidObservedFollowUpEnd,
    #[error("planned-horizon completion does not equal the protocol horizon")]
    PlannedHorizonMismatch,
    #[error("protocol end condition was not declared by the target-trial protocol")]
    UndeclaredProtocolEndCondition,
    #[error("observational censoring receipt uses a different censoring plan")]
    CensoringPlanMismatch,
    #[error("unknown target-trial outcome: {0}")]
    UnknownOutcome(String),
    #[error("unknown outcome operationalization: {0}")]
    UnknownOutcomeOperationalization(String),
    #[error("outcome definition does not match the protocol-relative phenotype identity")]
    OutcomeDefinitionMismatch,
    #[error("outcome context is anchored to a different time zero")]
    OutcomeAnchorTimeMismatch,
    #[error("outcome context does not bind the exact time-zero receipt")]
    OutcomeAnchorEvidenceMismatch,
    #[error("derived outcome window is invalid")]
    InvalidDerivedOutcomeWindow,
    #[error("outcome evaluation window does not match the relative phenotype")]
    OutcomeWindowMismatch,
    #[error("complete coverage cannot be claimed for an outcome window truncated by follow-up")]
    CompleteCoverageAfterTruncatedFollowUp,
    #[error("facts from multiple subjects were supplied")]
    MixedSubjectFacts,
    #[error("follow-up receipt is invalid")]
    InvalidFollowUpReceipt,
    #[error("outcome receipt is invalid")]
    InvalidOutcomeReceipt,
    #[error("outcome receipt does not reproduce from supplied evidence")]
    OutcomeReceiptMismatch,
    #[error("evidence artifact identity is incomplete")]
    IncompleteEvidenceIdentity,
    #[error("evidence digest may not be all zeroes")]
    ZeroEvidenceDigest,
    #[error("time arithmetic overflow")]
    TimeOverflow,
    #[error("canonical framing length exceeds v2 limit")]
    LengthOverflow,
    #[error(transparent)]
    Protocol(#[from] mycelix_target_trial_protocol_v2::TargetTrialV2Error),
    #[error(transparent)]
    Cohort(#[from] mycelix_target_trial_cohort_receipts_v2::TargetTrialCohortV2Error),
    #[error(transparent)]
    Phenotype(#[from] mycelix_clinical_phenotype_v2::PhenotypeV2Error),
    #[error(transparent)]
    Clinical(#[from] mycelix_clinical_semantics::ClinicalSemanticsError),
}
