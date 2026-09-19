#![deny(unsafe_code)]
//! Research-only fixed-horizon target-trial analysis contributions v1.
//!
//! This layer sits strictly between participant-level outcome ascertainment and
//! any estimator. It preserves indeterminate outcomes rather than silently
//! converting them into event-free observations.

use mycelix_clinical_phenotype_v2::CriterionStateV2;
use mycelix_target_trial_cohort_receipts_v2::{
    cohort_entry_receipt_digest_v2, time_zero_receipt_digest_v2, CohortEntryReceiptV2,
    TimeZeroReceiptV2,
};
use mycelix_target_trial_followup_outcome_v2::{
    follow_up_receipt_digest_v2, outcome_ascertainment_receipt_digest_v2,
    verify_follow_up_receipt_v2, verify_outcome_ascertainment_receipt_v2,
    FollowUpReceiptV2, OutcomeAscertainmentReceiptV2, OutcomeEvidenceV2,
};
use mycelix_target_trial_protocol_v2::{
    target_trial_emulation_plan_digest_v2, target_trial_protocol_digest_v2,
    CausalContrastV2, CompetingEventStrategyV2, EffectMeasureV2, EvidenceArtifactIdentityV2,
    TargetTrialEmulationPlanV2, TargetTrialProtocolV2,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const TARGET_TRIAL_ANALYSIS_CONTRIBUTION_V1_VERSION: u16 = 1;

const CONTRIBUTION_TAG: &[u8] = b"mycelix/target-trial-fixed-horizon-contribution/v1";
const MANIFEST_TAG: &[u8] = b"mycelix/target-trial-fixed-horizon-analysis-manifest/v1";
const CONTRIBUTION_CONTEXT: &str = "mycelix.health.target-trial-fixed-horizon-contribution.v1";
const MANIFEST_CONTEXT: &str = "mycelix.health.target-trial-fixed-horizon-analysis-manifest.v1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum StrategyRoleV1 {
    Treatment,
    Comparator,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum BinaryContributionStateV1 {
    ObservedEvent,
    ObservedNonEvent,
    OutcomeIndeterminate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FixedHorizonAnalysisContributionV1 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub estimand_id: String,
    pub estimand_definition: EvidenceArtifactIdentityV2,
    pub effect_measure: EffectMeasureV2,
    pub competing_event_strategy: CompetingEventStrategyV2,
    pub subject_resource_type: String,
    pub subject_id: String,
    pub strategy_id: String,
    pub strategy_role: StrategyRoleV1,
    pub cohort_entry_digest: [u8; 32],
    pub time_zero_receipt_digest: [u8; 32],
    pub follow_up_receipt_digest: [u8; 32],
    pub outcome_ascertainment_receipt_digest: [u8; 32],
    pub time_zero_micros: i64,
    pub observed_until_micros: i64,
    pub outcome_id: String,
    pub outcome_window_start_micros: i64,
    pub outcome_window_end_micros: i64,
    pub state: BinaryContributionStateV1,
}

pub struct FixedHorizonContributionEvidenceV1<'a> {
    pub protocol: &'a TargetTrialProtocolV2,
    pub plan: &'a TargetTrialEmulationPlanV2,
    pub cohort_entry: &'a CohortEntryReceiptV2,
    pub time_zero: &'a TimeZeroReceiptV2,
    pub follow_up: &'a FollowUpReceiptV2,
    pub outcome_evidence: &'a OutcomeEvidenceV2<'a>,
    pub outcome_receipt: &'a OutcomeAscertainmentReceiptV2,
    pub estimand_id: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FixedHorizonContributionDigestV1([u8; 32]);
impl FixedHorizonContributionDigestV1 {
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

pub struct VerifiedFixedHorizonContributionV1 {
    receipt: FixedHorizonAnalysisContributionV1,
    digest: FixedHorizonContributionDigestV1,
}

impl VerifiedFixedHorizonContributionV1 {
    #[must_use]
    pub fn receipt(&self) -> &FixedHorizonAnalysisContributionV1 {
        &self.receipt
    }

    #[must_use]
    pub const fn digest(&self) -> FixedHorizonContributionDigestV1 {
        self.digest
    }

    #[must_use]
    pub fn is_binary_observed(&self) -> bool {
        self.receipt.state != BinaryContributionStateV1::OutcomeIndeterminate
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FixedHorizonAnalysisManifestV1 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub estimand_id: String,
    pub estimand_definition: EvidenceArtifactIdentityV2,
    pub effect_measure: EffectMeasureV2,
    pub competing_event_strategy: CompetingEventStrategyV2,
    pub contributions: Vec<FixedHorizonAnalysisContributionV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FixedHorizonAnalysisManifestDigestV1([u8; 32]);
impl FixedHorizonAnalysisManifestDigestV1 {
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

pub fn build_fixed_horizon_contribution_v1(
    evidence: &FixedHorizonContributionEvidenceV1<'_>,
) -> Result<VerifiedFixedHorizonContributionV1, AnalysisContributionV1Error> {
    evidence.plan.validate_against(evidence.protocol)?;
    verify_follow_up_receipt_v2(
        evidence.protocol,
        evidence.plan,
        evidence.cohort_entry,
        evidence.time_zero,
        evidence.follow_up,
    )?;
    verify_outcome_ascertainment_receipt_v2(
        evidence.protocol,
        evidence.plan,
        evidence.cohort_entry,
        evidence.time_zero,
        evidence.follow_up,
        evidence.outcome_evidence,
        evidence.outcome_receipt,
    )?;

    let estimand = evidence
        .protocol
        .estimands
        .iter()
        .find(|value| value.estimand_id == evidence.estimand_id)
        .ok_or_else(|| AnalysisContributionV1Error::UnknownEstimand(evidence.estimand_id.to_string()))?;

    if estimand.contrast != CausalContrastV2::IntentionToTreat {
        return Err(AnalysisContributionV1Error::UnsupportedCausalContrast);
    }
    if !matches!(estimand.effect_measure, EffectMeasureV2::RiskDifference | EffectMeasureV2::RiskRatio) {
        return Err(AnalysisContributionV1Error::UnsupportedEffectMeasure);
    }
    if estimand.competing_event_strategy != CompetingEventStrategyV2::NotApplicable {
        return Err(AnalysisContributionV1Error::UnsupportedCompetingEventStrategy);
    }
    if estimand.outcome_id != evidence.outcome_receipt.outcome_id {
        return Err(AnalysisContributionV1Error::EstimandOutcomeMismatch);
    }

    let strategy_role = if evidence.cohort_entry.strategy_id == estimand.treatment_strategy_id {
        StrategyRoleV1::Treatment
    } else if evidence.cohort_entry.strategy_id == estimand.comparator_strategy_id {
        StrategyRoleV1::Comparator
    } else {
        return Err(AnalysisContributionV1Error::StrategyOutsideEstimand);
    };

    let state = match evidence.outcome_receipt.state {
        CriterionStateV2::Satisfied => BinaryContributionStateV1::ObservedEvent,
        CriterionStateV2::NotSatisfied => {
            if evidence.follow_up.observed_until_micros < evidence.outcome_receipt.window_end_micros {
                return Err(AnalysisContributionV1Error::NonEventWithoutCompleteOutcomeWindow);
            }
            BinaryContributionStateV1::ObservedNonEvent
        }
        CriterionStateV2::Indeterminate => BinaryContributionStateV1::OutcomeIndeterminate,
    };

    let receipt = FixedHorizonAnalysisContributionV1 {
        schema_version: TARGET_TRIAL_ANALYSIS_CONTRIBUTION_V1_VERSION,
        protocol_digest: target_trial_protocol_digest_v2(evidence.protocol)?.into_bytes(),
        emulation_plan_digest: target_trial_emulation_plan_digest_v2(evidence.protocol, evidence.plan)?.into_bytes(),
        estimand_id: estimand.estimand_id.clone(),
        estimand_definition: estimand.estimand_definition.clone(),
        effect_measure: estimand.effect_measure,
        competing_event_strategy: estimand.competing_event_strategy,
        subject_resource_type: evidence.cohort_entry.subject.resource_type.clone(),
        subject_id: evidence.cohort_entry.subject.id.clone(),
        strategy_id: evidence.cohort_entry.strategy_id.clone(),
        strategy_role,
        cohort_entry_digest: cohort_entry_receipt_digest_v2(evidence.cohort_entry)?.into_bytes(),
        time_zero_receipt_digest: time_zero_receipt_digest_v2(evidence.time_zero)?.into_bytes(),
        follow_up_receipt_digest: follow_up_receipt_digest_v2(evidence.follow_up)?.into_bytes(),
        outcome_ascertainment_receipt_digest: outcome_ascertainment_receipt_digest_v2(evidence.outcome_receipt)?.into_bytes(),
        time_zero_micros: evidence.time_zero.time_zero_micros,
        observed_until_micros: evidence.follow_up.observed_until_micros,
        outcome_id: evidence.outcome_receipt.outcome_id.clone(),
        outcome_window_start_micros: evidence.outcome_receipt.window_start_micros,
        outcome_window_end_micros: evidence.outcome_receipt.window_end_micros,
        state,
    };
    validate_contribution_shape(&receipt)?;
    let digest = fixed_horizon_contribution_digest_v1(&receipt)?;
    Ok(VerifiedFixedHorizonContributionV1 { receipt, digest })
}

pub fn verify_fixed_horizon_contribution_v1(
    evidence: &FixedHorizonContributionEvidenceV1<'_>,
    receipt: &FixedHorizonAnalysisContributionV1,
) -> Result<VerifiedFixedHorizonContributionV1, AnalysisContributionV1Error> {
    validate_contribution_shape(receipt)?;
    let rebuilt = build_fixed_horizon_contribution_v1(evidence)?;
    let supplied_digest = fixed_horizon_contribution_digest_v1(receipt)?;
    if rebuilt.digest != supplied_digest {
        return Err(AnalysisContributionV1Error::ContributionReceiptMismatch);
    }
    Ok(VerifiedFixedHorizonContributionV1 {
        receipt: receipt.clone(),
        digest: supplied_digest,
    })
}

pub fn build_fixed_horizon_analysis_manifest_v1(
    contributions: &[VerifiedFixedHorizonContributionV1],
) -> Result<FixedHorizonAnalysisManifestV1, AnalysisContributionV1Error> {
    let first = contributions
        .first()
        .ok_or(AnalysisContributionV1Error::EmptyAnalysisManifest)?
        .receipt();
    let mut values: Vec<_> = contributions.iter().map(|value| value.receipt().clone()).collect();
    for value in &values {
        validate_contribution_shape(value)?;
        if value.protocol_digest != first.protocol_digest
            || value.emulation_plan_digest != first.emulation_plan_digest
            || value.estimand_id != first.estimand_id
            || value.estimand_definition != first.estimand_definition
            || value.effect_measure != first.effect_measure
            || value.competing_event_strategy != first.competing_event_strategy
        {
            return Err(AnalysisContributionV1Error::MixedAnalysisCoordinates);
        }
    }
    values.sort_by(|left, right| {
        (&left.subject_resource_type, &left.subject_id)
            .cmp(&(&right.subject_resource_type, &right.subject_id))
    });
    let mut subjects = HashSet::new();
    for value in &values {
        if !subjects.insert((value.subject_resource_type.clone(), value.subject_id.clone())) {
            return Err(AnalysisContributionV1Error::DuplicateSubjectContribution {
                resource_type: value.subject_resource_type.clone(),
                id: value.subject_id.clone(),
            });
        }
    }
    let manifest = FixedHorizonAnalysisManifestV1 {
        schema_version: TARGET_TRIAL_ANALYSIS_CONTRIBUTION_V1_VERSION,
        protocol_digest: first.protocol_digest,
        emulation_plan_digest: first.emulation_plan_digest,
        estimand_id: first.estimand_id.clone(),
        estimand_definition: first.estimand_definition.clone(),
        effect_measure: first.effect_measure,
        competing_event_strategy: first.competing_event_strategy,
        contributions: values,
    };
    validate_manifest_shape(&manifest)?;
    Ok(manifest)
}

pub fn fixed_horizon_contribution_digest_v1(
    receipt: &FixedHorizonAnalysisContributionV1,
) -> Result<FixedHorizonContributionDigestV1, AnalysisContributionV1Error> {
    validate_contribution_shape(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_contribution(&mut writer, receipt)?;
    Ok(FixedHorizonContributionDigestV1(domain_digest(
        CONTRIBUTION_CONTEXT,
        CONTRIBUTION_TAG,
        &writer.finish(),
    )))
}

pub fn fixed_horizon_analysis_manifest_digest_v1(
    manifest: &FixedHorizonAnalysisManifestV1,
) -> Result<FixedHorizonAnalysisManifestDigestV1, AnalysisContributionV1Error> {
    validate_manifest_shape(manifest)?;
    let mut writer = CanonicalWriter::default();
    encode_manifest(&mut writer, manifest)?;
    Ok(FixedHorizonAnalysisManifestDigestV1(domain_digest(
        MANIFEST_CONTEXT,
        MANIFEST_TAG,
        &writer.finish(),
    )))
}

fn validate_contribution_shape(
    value: &FixedHorizonAnalysisContributionV1,
) -> Result<(), AnalysisContributionV1Error> {
    if value.schema_version != TARGET_TRIAL_ANALYSIS_CONTRIBUTION_V1_VERSION
        || value.protocol_digest == [0; 32]
        || value.emulation_plan_digest == [0; 32]
        || value.cohort_entry_digest == [0; 32]
        || value.time_zero_receipt_digest == [0; 32]
        || value.follow_up_receipt_digest == [0; 32]
        || value.outcome_ascertainment_receipt_digest == [0; 32]
        || value.estimand_id.trim().is_empty()
        || value.subject_resource_type.trim().is_empty()
        || value.subject_id.trim().is_empty()
        || value.strategy_id.trim().is_empty()
        || value.outcome_id.trim().is_empty()
        || value.outcome_window_start_micros >= value.outcome_window_end_micros
        || value.observed_until_micros <= value.time_zero_micros
    {
        return Err(AnalysisContributionV1Error::InvalidContributionReceipt);
    }
    validate_artifact(&value.estimand_definition)?;
    if !matches!(value.effect_measure, EffectMeasureV2::RiskDifference | EffectMeasureV2::RiskRatio) {
        return Err(AnalysisContributionV1Error::UnsupportedEffectMeasure);
    }
    if value.competing_event_strategy != CompetingEventStrategyV2::NotApplicable {
        return Err(AnalysisContributionV1Error::UnsupportedCompetingEventStrategy);
    }
    if value.state == BinaryContributionStateV1::ObservedNonEvent
        && value.observed_until_micros < value.outcome_window_end_micros
    {
        return Err(AnalysisContributionV1Error::NonEventWithoutCompleteOutcomeWindow);
    }
    Ok(())
}

fn validate_manifest_shape(
    value: &FixedHorizonAnalysisManifestV1,
) -> Result<(), AnalysisContributionV1Error> {
    if value.schema_version != TARGET_TRIAL_ANALYSIS_CONTRIBUTION_V1_VERSION
        || value.protocol_digest == [0; 32]
        || value.emulation_plan_digest == [0; 32]
        || value.estimand_id.trim().is_empty()
        || value.contributions.is_empty()
    {
        return Err(AnalysisContributionV1Error::InvalidAnalysisManifest);
    }
    validate_artifact(&value.estimand_definition)?;
    if !matches!(value.effect_measure, EffectMeasureV2::RiskDifference | EffectMeasureV2::RiskRatio)
        || value.competing_event_strategy != CompetingEventStrategyV2::NotApplicable
    {
        return Err(AnalysisContributionV1Error::InvalidAnalysisManifest);
    }
    let mut previous: Option<(&str, &str)> = None;
    let mut subjects = HashSet::new();
    for contribution in &value.contributions {
        validate_contribution_shape(contribution)?;
        if contribution.protocol_digest != value.protocol_digest
            || contribution.emulation_plan_digest != value.emulation_plan_digest
            || contribution.estimand_id != value.estimand_id
            || contribution.estimand_definition != value.estimand_definition
            || contribution.effect_measure != value.effect_measure
            || contribution.competing_event_strategy != value.competing_event_strategy
        {
            return Err(AnalysisContributionV1Error::MixedAnalysisCoordinates);
        }
        let key = (contribution.subject_resource_type.as_str(), contribution.subject_id.as_str());
        if let Some(prev) = previous {
            if prev >= key {
                return Err(AnalysisContributionV1Error::NonCanonicalManifestOrder);
            }
        }
        previous = Some(key);
        if !subjects.insert((contribution.subject_resource_type.clone(), contribution.subject_id.clone())) {
            return Err(AnalysisContributionV1Error::DuplicateSubjectContribution {
                resource_type: contribution.subject_resource_type.clone(),
                id: contribution.subject_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_artifact(value: &EvidenceArtifactIdentityV2) -> Result<(), AnalysisContributionV1Error> {
    if value.namespace.trim().is_empty()
        || value.artifact_id.trim().is_empty()
        || value.version.trim().is_empty()
    {
        return Err(AnalysisContributionV1Error::IncompleteEvidenceIdentity);
    }
    if value.digest == [0; 32] {
        return Err(AnalysisContributionV1Error::ZeroEvidenceDigest);
    }
    Ok(())
}

fn domain_digest(context: &str, tag: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&TARGET_TRIAL_ANALYSIS_CONTRIBUTION_V1_VERSION.to_be_bytes());
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
    fn string(&mut self, value: &str) -> Result<(), AnalysisContributionV1Error> { self.bytes(value.as_bytes()) }
    fn bytes(&mut self, value: &[u8]) -> Result<(), AnalysisContributionV1Error> {
        self.u32(u32::try_from(value.len()).map_err(|_| AnalysisContributionV1Error::LengthOverflow)?);
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn vector_len(&mut self, len: usize) -> Result<(), AnalysisContributionV1Error> {
        self.u32(u32::try_from(len).map_err(|_| AnalysisContributionV1Error::LengthOverflow)?);
        Ok(())
    }
}

fn encode_artifact(writer: &mut CanonicalWriter, value: &EvidenceArtifactIdentityV2) -> Result<(), AnalysisContributionV1Error> {
    writer.string(&value.namespace)?;
    writer.string(&value.artifact_id)?;
    writer.string(&value.version)?;
    writer.bytes(&value.digest)
}

fn encode_effect_measure(writer: &mut CanonicalWriter, value: EffectMeasureV2) -> Result<(), AnalysisContributionV1Error> {
    match value {
        EffectMeasureV2::RiskDifference => writer.u8(0),
        EffectMeasureV2::RiskRatio => writer.u8(1),
        _ => return Err(AnalysisContributionV1Error::UnsupportedEffectMeasure),
    }
    Ok(())
}

fn encode_contribution(writer: &mut CanonicalWriter, value: &FixedHorizonAnalysisContributionV1) -> Result<(), AnalysisContributionV1Error> {
    writer.u16(value.schema_version);
    writer.bytes(&value.protocol_digest)?;
    writer.bytes(&value.emulation_plan_digest)?;
    writer.string(&value.estimand_id)?;
    encode_artifact(writer, &value.estimand_definition)?;
    encode_effect_measure(writer, value.effect_measure)?;
    writer.u8(0); // CompetingEventStrategyV2::NotApplicable only in v1.
    writer.string(&value.subject_resource_type)?;
    writer.string(&value.subject_id)?;
    writer.string(&value.strategy_id)?;
    writer.u8(match value.strategy_role { StrategyRoleV1::Treatment => 0, StrategyRoleV1::Comparator => 1 });
    writer.bytes(&value.cohort_entry_digest)?;
    writer.bytes(&value.time_zero_receipt_digest)?;
    writer.bytes(&value.follow_up_receipt_digest)?;
    writer.bytes(&value.outcome_ascertainment_receipt_digest)?;
    writer.i64(value.time_zero_micros);
    writer.i64(value.observed_until_micros);
    writer.string(&value.outcome_id)?;
    writer.i64(value.outcome_window_start_micros);
    writer.i64(value.outcome_window_end_micros);
    writer.u8(match value.state {
        BinaryContributionStateV1::ObservedEvent => 0,
        BinaryContributionStateV1::ObservedNonEvent => 1,
        BinaryContributionStateV1::OutcomeIndeterminate => 2,
    });
    Ok(())
}

fn encode_manifest(writer: &mut CanonicalWriter, value: &FixedHorizonAnalysisManifestV1) -> Result<(), AnalysisContributionV1Error> {
    writer.u16(value.schema_version);
    writer.bytes(&value.protocol_digest)?;
    writer.bytes(&value.emulation_plan_digest)?;
    writer.string(&value.estimand_id)?;
    encode_artifact(writer, &value.estimand_definition)?;
    encode_effect_measure(writer, value.effect_measure)?;
    writer.u8(0); // CompetingEventStrategyV2::NotApplicable only in v1.
    writer.vector_len(value.contributions.len())?;
    for contribution in &value.contributions {
        let digest = fixed_horizon_contribution_digest_v1(contribution)?.into_bytes();
        writer.bytes(&digest)?;
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum AnalysisContributionV1Error {
    #[error("unknown target-trial estimand: {0}")]
    UnknownEstimand(String),
    #[error("v1 fixed-horizon contribution supports intention-to-treat estimands only")]
    UnsupportedCausalContrast,
    #[error("v1 fixed-horizon contribution supports risk difference or risk ratio only")]
    UnsupportedEffectMeasure,
    #[error("v1 fixed-horizon contribution requires competing events to be not applicable")]
    UnsupportedCompetingEventStrategy,
    #[error("estimand outcome does not match the outcome-ascertainment receipt")]
    EstimandOutcomeMismatch,
    #[error("participant strategy is outside the selected estimand contrast")]
    StrategyOutsideEstimand,
    #[error("non-event contribution requires the complete declared outcome window to be observed")]
    NonEventWithoutCompleteOutcomeWindow,
    #[error("serialized contribution does not reproduce from exact upstream evidence")]
    ContributionReceiptMismatch,
    #[error("analysis manifest may not be empty")]
    EmptyAnalysisManifest,
    #[error("analysis manifest mixes protocol/emulation/estimand coordinates")]
    MixedAnalysisCoordinates,
    #[error("duplicate contribution for subject {resource_type}/{id}")]
    DuplicateSubjectContribution { resource_type: String, id: String },
    #[error("analysis manifest is not in canonical subject order")]
    NonCanonicalManifestOrder,
    #[error("fixed-horizon analysis contribution is invalid")]
    InvalidContributionReceipt,
    #[error("fixed-horizon analysis manifest is invalid")]
    InvalidAnalysisManifest,
    #[error("evidence artifact identity is incomplete")]
    IncompleteEvidenceIdentity,
    #[error("evidence digest may not be all zeroes")]
    ZeroEvidenceDigest,
    #[error("canonical framing length exceeds v1 limit")]
    LengthOverflow,
    #[error(transparent)]
    Protocol(#[from] mycelix_target_trial_protocol_v2::TargetTrialV2Error),
    #[error(transparent)]
    Cohort(#[from] mycelix_target_trial_cohort_receipts_v2::TargetTrialCohortV2Error),
    #[error(transparent)]
    FollowUp(#[from] mycelix_target_trial_followup_outcome_v2::FollowUpOutcomeV2Error),
}
