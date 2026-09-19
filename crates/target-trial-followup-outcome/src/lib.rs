#![deny(unsafe_code)]
//! Research-only target-trial follow-up and outcome ascertainment receipts.
//!
//! V1 composes one exact cohort entry with bounded observational follow-up and
//! one declared outcome. It does not estimate a causal effect.

use mycelix_clinical_phenotype::{
    evaluate_phenotype_v1, phenotype_definition_digest_v1, phenotype_evaluation_digest_v1,
    CriterionStateV1, PhenotypeDefinitionV1,
};
use mycelix_clinical_semantics::{ClinicalFact, ClinicalSemanticsError, SubjectRef};
use mycelix_target_trial_cohort_receipts::{
    cohort_entry_receipt_digest_v1, CohortEntryReceiptV1,
};
use mycelix_target_trial_protocol::{
    target_trial_emulation_plan_digest_v1, target_trial_protocol_digest_v1,
    EvidenceArtifactIdentityV1, OutcomeV1, TargetTrialEmulationPlanV1,
    TargetTrialProtocolV1,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const TARGET_TRIAL_FOLLOWUP_OUTCOME_VERSION: u16 = 1;

const FOLLOWUP_TAG: &[u8] = b"mycelix/target-trial-followup-receipt/v1";
const OUTCOME_TAG: &[u8] = b"mycelix/target-trial-outcome-ascertainment-receipt/v1";
const FOLLOWUP_CONTEXT: &str = "mycelix.health.target-trial-followup-receipt.v1";
const OUTCOME_CONTEXT: &str = "mycelix.health.target-trial-outcome-ascertainment-receipt.v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FollowUpEndV1 {
    PlannedHorizonComplete,
    ProtocolEndCondition {
        definition: EvidenceArtifactIdentityV1,
        event_evidence: EvidenceArtifactIdentityV1,
    },
    Censored {
        censoring_plan: EvidenceArtifactIdentityV1,
        event_evidence: EvidenceArtifactIdentityV1,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FollowUpReceiptV1 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub cohort_entry_digest: [u8; 32],
    pub subject: SubjectRef,
    pub strategy_id: String,
    pub data_source_id: String,
    pub time_zero_micros: i64,
    pub planned_horizon_end_micros: i64,
    /// Follow-up is observed on the half-open interval [time_zero, observed_until).
    pub observed_until_micros: i64,
    pub end: FollowUpEndV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OutcomeAscertainmentReceiptV1 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub cohort_entry_digest: [u8; 32],
    pub follow_up_receipt_digest: [u8; 32],
    pub subject: SubjectRef,
    pub strategy_id: String,
    pub time_zero_micros: i64,
    pub outcome_id: String,
    pub outcome_definition_digest: [u8; 32],
    pub outcome_evaluation_digest: [u8; 32],
    pub outcome_operationalization: EvidenceArtifactIdentityV1,
    pub assessment_window_start_micros: i64,
    pub assessment_window_end_micros: i64,
    pub state: CriterionStateV1,
    pub evidence_fact_digests: Vec<[u8; 32]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FollowUpReceiptDigestV1([u8; 32]);

impl FollowUpReceiptDigestV1 {
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OutcomeAscertainmentReceiptDigestV1([u8; 32]);

impl OutcomeAscertainmentReceiptDigestV1 {
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

pub fn build_follow_up_receipt_v1(
    protocol: &TargetTrialProtocolV1,
    plan: &TargetTrialEmulationPlanV1,
    entry: &CohortEntryReceiptV1,
    observed_until_micros: i64,
    end: FollowUpEndV1,
) -> Result<FollowUpReceiptV1, FollowUpOutcomeError> {
    plan.validate_against(protocol)?;
    validate_entry_against(protocol, plan, entry)?;

    let planned_horizon_end_micros = entry
        .time_zero_micros
        .checked_add(protocol.follow_up.maximum_duration_micros)
        .ok_or(FollowUpOutcomeError::TimeOverflow)?;
    if observed_until_micros < entry.time_zero_micros
        || observed_until_micros > planned_horizon_end_micros
    {
        return Err(FollowUpOutcomeError::InvalidObservedFollowUpEnd);
    }
    validate_follow_up_end(
        protocol,
        plan,
        observed_until_micros,
        planned_horizon_end_micros,
        &end,
    )?;

    let receipt = FollowUpReceiptV1 {
        schema_version: TARGET_TRIAL_FOLLOWUP_OUTCOME_VERSION,
        protocol_digest: target_trial_protocol_digest_v1(protocol)?.into_bytes(),
        emulation_plan_digest: target_trial_emulation_plan_digest_v1(protocol, plan)?.into_bytes(),
        cohort_entry_digest: cohort_entry_receipt_digest_v1(entry)?.into_bytes(),
        subject: entry.subject.clone(),
        strategy_id: entry.strategy_id.clone(),
        data_source_id: entry.data_source_id.clone(),
        time_zero_micros: entry.time_zero_micros,
        planned_horizon_end_micros,
        observed_until_micros,
        end,
    };
    validate_follow_up_receipt(&receipt)?;
    validate_follow_up_against(protocol, plan, entry, &receipt)?;
    Ok(receipt)
}

pub fn build_outcome_ascertainment_receipt_v1(
    protocol: &TargetTrialProtocolV1,
    plan: &TargetTrialEmulationPlanV1,
    entry: &CohortEntryReceiptV1,
    follow_up: &FollowUpReceiptV1,
    outcome_id: &str,
    outcome_definition: &PhenotypeDefinitionV1,
    facts: &[ClinicalFact],
) -> Result<OutcomeAscertainmentReceiptV1, FollowUpOutcomeError> {
    plan.validate_against(protocol)?;
    validate_entry_against(protocol, plan, entry)?;
    validate_follow_up_receipt(follow_up)?;
    validate_follow_up_against(protocol, plan, entry, follow_up)?;

    let outcome = protocol
        .outcomes
        .iter()
        .find(|outcome| outcome.outcome_id == outcome_id)
        .ok_or_else(|| FollowUpOutcomeError::UnknownOutcome(outcome_id.to_string()))?;
    let operationalization = plan
        .outcome_operationalizations
        .iter()
        .find(|mapping| mapping.outcome_id == outcome_id)
        .ok_or_else(|| FollowUpOutcomeError::UnknownOutcomeOperationalization(outcome_id.to_string()))?;

    validate_outcome_definition(outcome, outcome_definition)?;
    let assessment_window_start_micros = entry
        .time_zero_micros
        .checked_add(outcome.assessment_window_start_offset_micros)
        .ok_or(FollowUpOutcomeError::TimeOverflow)?;
    let assessment_window_end_micros = entry
        .time_zero_micros
        .checked_add(outcome.assessment_window_end_offset_micros)
        .ok_or(FollowUpOutcomeError::TimeOverflow)?;
    if outcome_definition.window.start_micros != assessment_window_start_micros
        || outcome_definition.window.end_micros != assessment_window_end_micros
    {
        return Err(FollowUpOutcomeError::OutcomeWindowMismatch);
    }

    // Validate every supplied fact before filtering, so a cross-patient or malformed
    // record cannot disappear merely because it occurs after censoring.
    for fact in facts {
        fact.validate_machine_actionable()?;
        if fact.subject.resource_type != entry.subject.resource_type
            || fact.subject.id != entry.subject.id
        {
            return Err(FollowUpOutcomeError::SubjectMismatch);
        }
    }
    let observed_facts: Vec<ClinicalFact> = facts
        .iter()
        .filter(|fact| fact.effective_at_micros < follow_up.observed_until_micros)
        .cloned()
        .collect();
    let evaluation = evaluate_phenotype_v1(
        outcome_definition,
        &entry.subject.id,
        &observed_facts,
    )?;
    let state = if evaluation.state == CriterionStateV1::Satisfied {
        CriterionStateV1::Satisfied
    } else if follow_up.observed_until_micros < assessment_window_end_micros {
        CriterionStateV1::Indeterminate
    } else {
        evaluation.state
    };

    let receipt = OutcomeAscertainmentReceiptV1 {
        schema_version: TARGET_TRIAL_FOLLOWUP_OUTCOME_VERSION,
        protocol_digest: follow_up.protocol_digest,
        emulation_plan_digest: follow_up.emulation_plan_digest,
        cohort_entry_digest: follow_up.cohort_entry_digest,
        follow_up_receipt_digest: follow_up_receipt_digest_v1(follow_up)?.into_bytes(),
        subject: entry.subject.clone(),
        strategy_id: entry.strategy_id.clone(),
        time_zero_micros: entry.time_zero_micros,
        outcome_id: outcome_id.to_string(),
        outcome_definition_digest: phenotype_definition_digest_v1(outcome_definition)?.into_bytes(),
        outcome_evaluation_digest: phenotype_evaluation_digest_v1(&evaluation)?.into_bytes(),
        outcome_operationalization: operationalization.operationalization.clone(),
        assessment_window_start_micros,
        assessment_window_end_micros,
        state,
        evidence_fact_digests: evaluation.evidence_fact_digests,
    };
    validate_outcome_receipt(&receipt)?;
    validate_outcome_against(protocol, plan, entry, follow_up, &receipt)?;
    Ok(receipt)
}

pub fn follow_up_receipt_digest_v1(
    receipt: &FollowUpReceiptV1,
) -> Result<FollowUpReceiptDigestV1, FollowUpOutcomeError> {
    validate_follow_up_receipt(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_follow_up(&mut writer, receipt)?;
    Ok(FollowUpReceiptDigestV1(domain_digest(
        FOLLOWUP_CONTEXT,
        FOLLOWUP_TAG,
        &writer.finish(),
    )))
}

pub fn outcome_ascertainment_receipt_digest_v1(
    receipt: &OutcomeAscertainmentReceiptV1,
) -> Result<OutcomeAscertainmentReceiptDigestV1, FollowUpOutcomeError> {
    validate_outcome_receipt(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_outcome(&mut writer, receipt)?;
    Ok(OutcomeAscertainmentReceiptDigestV1(domain_digest(
        OUTCOME_CONTEXT,
        OUTCOME_TAG,
        &writer.finish(),
    )))
}

fn validate_entry_against(
    protocol: &TargetTrialProtocolV1,
    plan: &TargetTrialEmulationPlanV1,
    entry: &CohortEntryReceiptV1,
) -> Result<(), FollowUpOutcomeError> {
    let protocol_digest = target_trial_protocol_digest_v1(protocol)?.into_bytes();
    let emulation_plan_digest = target_trial_emulation_plan_digest_v1(protocol, plan)?.into_bytes();
    if entry.protocol_digest != protocol_digest || entry.emulation_plan_digest != emulation_plan_digest {
        return Err(FollowUpOutcomeError::ProtocolOrEmulationMismatch);
    }
    if entry.subject.resource_type.trim().is_empty()
        || entry.subject.id.trim().is_empty()
        || entry.strategy_id.trim().is_empty()
        || entry.data_source_id.trim().is_empty()
    {
        return Err(FollowUpOutcomeError::InvalidCohortEntry);
    }
    cohort_entry_receipt_digest_v1(entry)?;
    if !protocol
        .treatment_strategies
        .iter()
        .any(|strategy| strategy.strategy_id == entry.strategy_id)
    {
        return Err(FollowUpOutcomeError::InvalidCohortEntry);
    }
    let source = plan
        .data_sources
        .iter()
        .find(|source| source.source_id == entry.data_source_id)
        .ok_or(FollowUpOutcomeError::InvalidCohortEntry)?;
    if entry.time_zero_micros < source.period_start_micros
        || entry.time_zero_micros >= source.period_end_micros
    {
        return Err(FollowUpOutcomeError::InvalidCohortEntry);
    }
    Ok(())
}

fn validate_follow_up_end(
    protocol: &TargetTrialProtocolV1,
    plan: &TargetTrialEmulationPlanV1,
    observed_until_micros: i64,
    planned_horizon_end_micros: i64,
    end: &FollowUpEndV1,
) -> Result<(), FollowUpOutcomeError> {
    match end {
        FollowUpEndV1::PlannedHorizonComplete => {
            if observed_until_micros != planned_horizon_end_micros {
                return Err(FollowUpOutcomeError::PlannedHorizonMismatch);
            }
        }
        FollowUpEndV1::ProtocolEndCondition {
            definition,
            event_evidence,
        } => {
            validate_artifact(definition)?;
            validate_artifact(event_evidence)?;
            if !protocol.follow_up.end_conditions.iter().any(|item| item == definition) {
                return Err(FollowUpOutcomeError::UnknownProtocolEndCondition);
            }
        }
        FollowUpEndV1::Censored {
            censoring_plan,
            event_evidence,
        } => {
            validate_artifact(censoring_plan)?;
            validate_artifact(event_evidence)?;
            if censoring_plan != &plan.censoring_plan {
                return Err(FollowUpOutcomeError::CensoringPlanMismatch);
            }
        }
    }
    Ok(())
}

fn validate_follow_up_against(
    protocol: &TargetTrialProtocolV1,
    plan: &TargetTrialEmulationPlanV1,
    entry: &CohortEntryReceiptV1,
    receipt: &FollowUpReceiptV1,
) -> Result<(), FollowUpOutcomeError> {
    validate_entry_against(protocol, plan, entry)?;
    let expected_protocol = target_trial_protocol_digest_v1(protocol)?.into_bytes();
    let expected_plan = target_trial_emulation_plan_digest_v1(protocol, plan)?.into_bytes();
    if receipt.protocol_digest != expected_protocol || receipt.emulation_plan_digest != expected_plan {
        return Err(FollowUpOutcomeError::ProtocolOrEmulationMismatch);
    }
    if receipt.cohort_entry_digest != cohort_entry_receipt_digest_v1(entry)?.into_bytes() {
        return Err(FollowUpOutcomeError::CohortEntryDigestMismatch);
    }
    if receipt.subject != entry.subject
        || receipt.strategy_id != entry.strategy_id
        || receipt.data_source_id != entry.data_source_id
        || receipt.time_zero_micros != entry.time_zero_micros
    {
        return Err(FollowUpOutcomeError::CohortEntryBindingMismatch);
    }
    let planned = entry
        .time_zero_micros
        .checked_add(protocol.follow_up.maximum_duration_micros)
        .ok_or(FollowUpOutcomeError::TimeOverflow)?;
    if receipt.planned_horizon_end_micros != planned
        || receipt.observed_until_micros < receipt.time_zero_micros
        || receipt.observed_until_micros > planned
    {
        return Err(FollowUpOutcomeError::InvalidObservedFollowUpEnd);
    }
    validate_follow_up_end(
        protocol,
        plan,
        receipt.observed_until_micros,
        planned,
        &receipt.end,
    )
}

fn validate_outcome_definition(
    outcome: &OutcomeV1,
    definition: &PhenotypeDefinitionV1,
) -> Result<(), FollowUpOutcomeError> {
    let digest = phenotype_definition_digest_v1(definition)?.into_bytes();
    if definition.phenotype_id != outcome.phenotype.phenotype_id
        || definition.version != outcome.phenotype.version
        || digest != outcome.phenotype.digest
    {
        return Err(FollowUpOutcomeError::OutcomeDefinitionMismatch);
    }
    Ok(())
}

fn validate_outcome_against(
    protocol: &TargetTrialProtocolV1,
    plan: &TargetTrialEmulationPlanV1,
    entry: &CohortEntryReceiptV1,
    follow_up: &FollowUpReceiptV1,
    receipt: &OutcomeAscertainmentReceiptV1,
) -> Result<(), FollowUpOutcomeError> {
    validate_follow_up_against(protocol, plan, entry, follow_up)?;
    let outcome = protocol
        .outcomes
        .iter()
        .find(|outcome| outcome.outcome_id == receipt.outcome_id)
        .ok_or_else(|| FollowUpOutcomeError::UnknownOutcome(receipt.outcome_id.clone()))?;
    if receipt.protocol_digest != follow_up.protocol_digest
        || receipt.emulation_plan_digest != follow_up.emulation_plan_digest
        || receipt.cohort_entry_digest != follow_up.cohort_entry_digest
        || receipt.follow_up_receipt_digest != follow_up_receipt_digest_v1(follow_up)?.into_bytes()
        || receipt.subject != follow_up.subject
        || receipt.strategy_id != follow_up.strategy_id
        || receipt.time_zero_micros != follow_up.time_zero_micros
    {
        return Err(FollowUpOutcomeError::FollowUpBindingMismatch);
    }
    if receipt.outcome_definition_digest != outcome.phenotype.digest {
        return Err(FollowUpOutcomeError::OutcomeDefinitionMismatch);
    }
    let expected_operationalization = plan
        .outcome_operationalizations
        .iter()
        .find(|mapping| mapping.outcome_id == receipt.outcome_id)
        .ok_or_else(|| FollowUpOutcomeError::UnknownOutcomeOperationalization(receipt.outcome_id.clone()))?;
    if receipt.outcome_operationalization != expected_operationalization.operationalization {
        return Err(FollowUpOutcomeError::OutcomeOperationalizationMismatch);
    }
    let expected_start = entry
        .time_zero_micros
        .checked_add(outcome.assessment_window_start_offset_micros)
        .ok_or(FollowUpOutcomeError::TimeOverflow)?;
    let expected_end = entry
        .time_zero_micros
        .checked_add(outcome.assessment_window_end_offset_micros)
        .ok_or(FollowUpOutcomeError::TimeOverflow)?;
    if receipt.assessment_window_start_micros != expected_start
        || receipt.assessment_window_end_micros != expected_end
    {
        return Err(FollowUpOutcomeError::OutcomeWindowMismatch);
    }
    Ok(())
}

fn validate_follow_up_receipt(receipt: &FollowUpReceiptV1) -> Result<(), FollowUpOutcomeError> {
    require_schema(receipt.schema_version)?;
    for digest in [receipt.protocol_digest, receipt.emulation_plan_digest, receipt.cohort_entry_digest] {
        validate_nonzero(digest)?;
    }
    validate_subject(&receipt.subject)?;
    if receipt.strategy_id.trim().is_empty() || receipt.data_source_id.trim().is_empty() {
        return Err(FollowUpOutcomeError::InvalidFollowUpReceipt);
    }
    if receipt.observed_until_micros < receipt.time_zero_micros
        || receipt.planned_horizon_end_micros < receipt.time_zero_micros
        || receipt.observed_until_micros > receipt.planned_horizon_end_micros
    {
        return Err(FollowUpOutcomeError::InvalidObservedFollowUpEnd);
    }
    match &receipt.end {
        FollowUpEndV1::PlannedHorizonComplete => {}
        FollowUpEndV1::ProtocolEndCondition { definition, event_evidence } => {
            validate_artifact(definition)?;
            validate_artifact(event_evidence)?;
        }
        FollowUpEndV1::Censored { censoring_plan, event_evidence } => {
            validate_artifact(censoring_plan)?;
            validate_artifact(event_evidence)?;
        }
    }
    Ok(())
}

fn validate_outcome_receipt(receipt: &OutcomeAscertainmentReceiptV1) -> Result<(), FollowUpOutcomeError> {
    require_schema(receipt.schema_version)?;
    for digest in [
        receipt.protocol_digest,
        receipt.emulation_plan_digest,
        receipt.cohort_entry_digest,
        receipt.follow_up_receipt_digest,
        receipt.outcome_definition_digest,
        receipt.outcome_evaluation_digest,
    ] {
        validate_nonzero(digest)?;
    }
    validate_subject(&receipt.subject)?;
    validate_artifact(&receipt.outcome_operationalization)?;
    if receipt.strategy_id.trim().is_empty()
        || receipt.outcome_id.trim().is_empty()
        || receipt.assessment_window_start_micros >= receipt.assessment_window_end_micros
        || receipt.evidence_fact_digests.iter().any(|digest| *digest == [0; 32])
    {
        return Err(FollowUpOutcomeError::InvalidOutcomeReceipt);
    }
    Ok(())
}

fn validate_artifact(value: &EvidenceArtifactIdentityV1) -> Result<(), FollowUpOutcomeError> {
    if value.namespace.trim().is_empty()
        || value.artifact_id.trim().is_empty()
        || value.version.trim().is_empty()
        || value.digest == [0; 32]
    {
        return Err(FollowUpOutcomeError::InvalidEvidenceArtifact);
    }
    Ok(())
}

fn validate_subject(subject: &SubjectRef) -> Result<(), FollowUpOutcomeError> {
    if subject.resource_type.trim().is_empty() || subject.id.trim().is_empty() {
        return Err(FollowUpOutcomeError::InvalidSubject);
    }
    Ok(())
}

fn require_schema(version: u16) -> Result<(), FollowUpOutcomeError> {
    if version == TARGET_TRIAL_FOLLOWUP_OUTCOME_VERSION {
        Ok(())
    } else {
        Err(FollowUpOutcomeError::UnsupportedSchemaVersion(version))
    }
}

fn validate_nonzero(value: [u8; 32]) -> Result<(), FollowUpOutcomeError> {
    if value == [0; 32] {
        Err(FollowUpOutcomeError::ZeroDigest)
    } else {
        Ok(())
    }
}

fn domain_digest(context: &str, tag: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&TARGET_TRIAL_FOLLOWUP_OUTCOME_VERSION.to_be_bytes());
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
    fn bytes(&mut self, value: &[u8]) -> Result<(), FollowUpOutcomeError> {
        self.u32(u32::try_from(value.len()).map_err(|_| FollowUpOutcomeError::LengthOverflow)?);
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn string(&mut self, value: &str) -> Result<(), FollowUpOutcomeError> { self.bytes(value.as_bytes()) }
    fn vector_len(&mut self, len: usize) -> Result<(), FollowUpOutcomeError> {
        self.u32(u32::try_from(len).map_err(|_| FollowUpOutcomeError::LengthOverflow)?);
        Ok(())
    }
}

fn encode_subject(writer: &mut CanonicalWriter, subject: &SubjectRef) -> Result<(), FollowUpOutcomeError> {
    writer.string(&subject.resource_type)?;
    writer.string(&subject.id)
}

fn encode_artifact(writer: &mut CanonicalWriter, artifact: &EvidenceArtifactIdentityV1) -> Result<(), FollowUpOutcomeError> {
    writer.string(&artifact.namespace)?;
    writer.string(&artifact.artifact_id)?;
    writer.string(&artifact.version)?;
    writer.bytes(&artifact.digest)
}

fn encode_follow_up(writer: &mut CanonicalWriter, receipt: &FollowUpReceiptV1) -> Result<(), FollowUpOutcomeError> {
    writer.u16(receipt.schema_version);
    writer.bytes(&receipt.protocol_digest)?;
    writer.bytes(&receipt.emulation_plan_digest)?;
    writer.bytes(&receipt.cohort_entry_digest)?;
    encode_subject(writer, &receipt.subject)?;
    writer.string(&receipt.strategy_id)?;
    writer.string(&receipt.data_source_id)?;
    writer.i64(receipt.time_zero_micros);
    writer.i64(receipt.planned_horizon_end_micros);
    writer.i64(receipt.observed_until_micros);
    match &receipt.end {
        FollowUpEndV1::PlannedHorizonComplete => writer.u8(0),
        FollowUpEndV1::ProtocolEndCondition { definition, event_evidence } => {
            writer.u8(1);
            encode_artifact(writer, definition)?;
            encode_artifact(writer, event_evidence)?;
        }
        FollowUpEndV1::Censored { censoring_plan, event_evidence } => {
            writer.u8(2);
            encode_artifact(writer, censoring_plan)?;
            encode_artifact(writer, event_evidence)?;
        }
    }
    Ok(())
}

fn encode_outcome(writer: &mut CanonicalWriter, receipt: &OutcomeAscertainmentReceiptV1) -> Result<(), FollowUpOutcomeError> {
    writer.u16(receipt.schema_version);
    writer.bytes(&receipt.protocol_digest)?;
    writer.bytes(&receipt.emulation_plan_digest)?;
    writer.bytes(&receipt.cohort_entry_digest)?;
    writer.bytes(&receipt.follow_up_receipt_digest)?;
    encode_subject(writer, &receipt.subject)?;
    writer.string(&receipt.strategy_id)?;
    writer.i64(receipt.time_zero_micros);
    writer.string(&receipt.outcome_id)?;
    writer.bytes(&receipt.outcome_definition_digest)?;
    writer.bytes(&receipt.outcome_evaluation_digest)?;
    encode_artifact(writer, &receipt.outcome_operationalization)?;
    writer.i64(receipt.assessment_window_start_micros);
    writer.i64(receipt.assessment_window_end_micros);
    writer.u8(match receipt.state {
        CriterionStateV1::Satisfied => 0,
        CriterionStateV1::NotSatisfied => 1,
        CriterionStateV1::Indeterminate => 2,
    });
    writer.vector_len(receipt.evidence_fact_digests.len())?;
    for digest in &receipt.evidence_fact_digests {
        writer.bytes(digest)?;
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum FollowUpOutcomeError {
    #[error("unsupported follow-up/outcome schema version: {0}")]
    UnsupportedSchemaVersion(u16),
    #[error("evidence artifact identity is invalid")]
    InvalidEvidenceArtifact,
    #[error("subject identity is invalid")]
    InvalidSubject,
    #[error("digest may not be all zeroes")]
    ZeroDigest,
    #[error("protocol/emulation identity mismatch")]
    ProtocolOrEmulationMismatch,
    #[error("cohort entry is invalid for the supplied protocol/emulation plan")]
    InvalidCohortEntry,
    #[error("cohort-entry digest mismatch")]
    CohortEntryDigestMismatch,
    #[error("follow-up receipt was rebound to a different cohort entry")]
    CohortEntryBindingMismatch,
    #[error("time arithmetic overflow")]
    TimeOverflow,
    #[error("observed follow-up end is outside [time zero, planned horizon]")]
    InvalidObservedFollowUpEnd,
    #[error("planned-horizon completion does not end exactly at the planned horizon")]
    PlannedHorizonMismatch,
    #[error("protocol end condition is not declared by the target-trial protocol")]
    UnknownProtocolEndCondition,
    #[error("observational censoring plan does not match the emulation plan")]
    CensoringPlanMismatch,
    #[error("follow-up receipt is structurally invalid")]
    InvalidFollowUpReceipt,
    #[error("unknown target-trial outcome: {0}")]
    UnknownOutcome(String),
    #[error("missing observational outcome operationalization: {0}")]
    UnknownOutcomeOperationalization(String),
    #[error("outcome phenotype definition does not match the target-trial protocol")]
    OutcomeDefinitionMismatch,
    #[error("outcome phenotype window does not equal the protocol-derived absolute assessment window")]
    OutcomeWindowMismatch,
    #[error("outcome operationalization was substituted")]
    OutcomeOperationalizationMismatch,
    #[error("outcome receipt is not bound to the exact follow-up receipt")]
    FollowUpBindingMismatch,
    #[error("outcome ascertainment receipt is structurally invalid")]
    InvalidOutcomeReceipt,
    #[error("canonical framing length exceeds v1 limit")]
    LengthOverflow,
    #[error("fact subject does not match the cohort subject")]
    SubjectMismatch,
    #[error(transparent)]
    Protocol(#[from] mycelix_target_trial_protocol::TargetTrialError),
    #[error(transparent)]
    Cohort(#[from] mycelix_target_trial_cohort_receipts::TargetTrialCohortError),
    #[error(transparent)]
    Phenotype(#[from] mycelix_clinical_phenotype::PhenotypeError),
    #[error(transparent)]
    Clinical(#[from] ClinicalSemanticsError),
}
