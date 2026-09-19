#![deny(unsafe_code)]
//! Research-only baseline confounder measurement receipts v1.
//!
//! Missing measurement means only that no qualifying measurement was available
//! under the exact extraction/policy evidence supplied here. It never means
//! clinical absence of the confounder or condition.

use mycelix_clinical_fact_snapshot::clinical_fact_snapshot_digest_v1;
use mycelix_clinical_phenotype_v2::ConceptIdentityV2;
use mycelix_clinical_semantics::ClinicalFact;
use mycelix_target_trial_cohort_receipts_v2::{
    time_zero_receipt_digest_v2, verify_time_zero_receipt_v2, TimeZeroReceiptV2,
};
use mycelix_target_trial_protocol_v2::{
    target_trial_emulation_plan_digest_v2, target_trial_protocol_digest_v2,
    BaselineConfounderV2, EvidenceArtifactIdentityV2, TargetTrialEmulationPlanV2,
    TargetTrialProtocolV2,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const BASELINE_COVARIATE_MEASUREMENT_V1_VERSION: u16 = 1;

const POLICY_TAG: &[u8] = b"mycelix/target-trial-baseline-measurement-policy/v1";
const RECEIPT_TAG: &[u8] = b"mycelix/target-trial-baseline-measurement-receipt/v1";
const POLICY_CONTEXT: &str = "mycelix.health.target-trial-baseline-measurement-policy.v1";
const RECEIPT_CONTEXT: &str = "mycelix.health.target-trial-baseline-measurement-receipt.v1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BaselineSelectionRuleV1 {
    UniqueOnly,
    LatestAtOrBeforeEnd,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineMeasurementPolicyV1 {
    pub schema_version: u16,
    pub confounder_id: String,
    pub protocol_definition: EvidenceArtifactIdentityV2,
    pub concept: ConceptIdentityV2,
    pub window_start_offset_micros: i64,
    pub window_end_offset_micros: i64,
    pub selection_rule: BaselineSelectionRuleV1,
    pub policy_evidence: EvidenceArtifactIdentityV2,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct BaselineCandidateFactV1 {
    pub effective_at_micros: i64,
    pub fact_snapshot_digest: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BaselineMeasurementStateV1 {
    Selected {
        fact_snapshot_digest: [u8; 32],
        effective_at_micros: i64,
    },
    MissingMeasurement,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineCovariateMeasurementReceiptV1 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub subject_resource_type: String,
    pub subject_id: String,
    pub time_zero_receipt_digest: [u8; 32],
    pub time_zero_micros: i64,
    pub confounder_id: String,
    pub measurement_policy_digest: [u8; 32],
    pub candidate_set_evidence: EvidenceArtifactIdentityV2,
    pub window_start_micros: i64,
    pub window_end_micros: i64,
    pub candidate_facts: Vec<BaselineCandidateFactV1>,
    pub state: BaselineMeasurementStateV1,
}

pub struct BaselineMeasurementEvidenceV1<'a> {
    pub protocol: &'a TargetTrialProtocolV2,
    pub plan: &'a TargetTrialEmulationPlanV2,
    pub time_zero: &'a TimeZeroReceiptV2,
    pub policy: &'a BaselineMeasurementPolicyV1,
    pub candidate_set_evidence: &'a EvidenceArtifactIdentityV2,
    pub candidate_facts: &'a [ClinicalFact],
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

digest_type!(BaselineMeasurementPolicyDigestV1);
digest_type!(BaselineCovariateMeasurementReceiptDigestV1);

pub struct VerifiedBaselineCovariateMeasurementV1 {
    receipt: BaselineCovariateMeasurementReceiptV1,
    digest: BaselineCovariateMeasurementReceiptDigestV1,
}

impl VerifiedBaselineCovariateMeasurementV1 {
    #[must_use]
    pub fn receipt(&self) -> &BaselineCovariateMeasurementReceiptV1 {
        &self.receipt
    }

    #[must_use]
    pub const fn digest(&self) -> BaselineCovariateMeasurementReceiptDigestV1 {
        self.digest
    }
}

pub fn build_baseline_covariate_measurement_v1(
    evidence: &BaselineMeasurementEvidenceV1<'_>,
) -> Result<VerifiedBaselineCovariateMeasurementV1, BaselineCovariateV1Error> {
    evidence.plan.validate_against(evidence.protocol)?;
    verify_time_zero_receipt_v2(evidence.protocol, evidence.plan, evidence.time_zero)?;
    let confounder = find_confounder(evidence.plan, &evidence.policy.confounder_id)?;
    validate_policy_against_confounder(evidence.policy, confounder)?;
    validate_artifact(evidence.candidate_set_evidence)?;

    let window_start = evidence
        .time_zero
        .time_zero_micros
        .checked_add(evidence.policy.window_start_offset_micros)
        .ok_or(BaselineCovariateV1Error::TimeOverflow)?;
    let window_end = evidence
        .time_zero
        .time_zero_micros
        .checked_add(evidence.policy.window_end_offset_micros)
        .ok_or(BaselineCovariateV1Error::TimeOverflow)?;

    let mut candidates = Vec::with_capacity(evidence.candidate_facts.len());
    let mut seen = HashSet::new();
    for fact in evidence.candidate_facts {
        fact.validate_machine_actionable()?;
        if fact.subject != evidence.time_zero.subject {
            return Err(BaselineCovariateV1Error::SubjectMismatch);
        }
        if !fact_matches_concept(fact, &evidence.policy.concept) {
            return Err(BaselineCovariateV1Error::CandidateConceptMismatch);
        }
        if fact.effective_at_micros < window_start || fact.effective_at_micros > window_end {
            return Err(BaselineCovariateV1Error::CandidateOutsideBaselineWindow);
        }
        let digest = clinical_fact_snapshot_digest_v1(fact)?.into_bytes();
        if !seen.insert(digest) {
            return Err(BaselineCovariateV1Error::DuplicateCandidateFact);
        }
        candidates.push(BaselineCandidateFactV1 {
            effective_at_micros: fact.effective_at_micros,
            fact_snapshot_digest: digest,
        });
    }
    candidates.sort();

    let state = select_measurement(evidence.policy.selection_rule, &candidates)?;
    let receipt = BaselineCovariateMeasurementReceiptV1 {
        schema_version: BASELINE_COVARIATE_MEASUREMENT_V1_VERSION,
        protocol_digest: target_trial_protocol_digest_v2(evidence.protocol)?.into_bytes(),
        emulation_plan_digest: target_trial_emulation_plan_digest_v2(evidence.protocol, evidence.plan)?.into_bytes(),
        subject_resource_type: evidence.time_zero.subject.resource_type.clone(),
        subject_id: evidence.time_zero.subject.id.clone(),
        time_zero_receipt_digest: time_zero_receipt_digest_v2(evidence.time_zero)?.into_bytes(),
        time_zero_micros: evidence.time_zero.time_zero_micros,
        confounder_id: evidence.policy.confounder_id.clone(),
        measurement_policy_digest: baseline_measurement_policy_digest_v1(evidence.policy)?.into_bytes(),
        candidate_set_evidence: evidence.candidate_set_evidence.clone(),
        window_start_micros: window_start,
        window_end_micros: window_end,
        candidate_facts: candidates,
        state,
    };
    validate_receipt_shape(&receipt)?;
    let digest = baseline_covariate_measurement_receipt_digest_v1(&receipt)?;
    Ok(VerifiedBaselineCovariateMeasurementV1 { receipt, digest })
}

pub fn verify_baseline_covariate_measurement_v1(
    evidence: &BaselineMeasurementEvidenceV1<'_>,
    receipt: &BaselineCovariateMeasurementReceiptV1,
) -> Result<VerifiedBaselineCovariateMeasurementV1, BaselineCovariateV1Error> {
    validate_receipt_shape(receipt)?;
    let rebuilt = build_baseline_covariate_measurement_v1(evidence)?;
    let supplied_digest = baseline_covariate_measurement_receipt_digest_v1(receipt)?;
    if rebuilt.digest != supplied_digest {
        return Err(BaselineCovariateV1Error::ReceiptMismatch);
    }
    Ok(VerifiedBaselineCovariateMeasurementV1 {
        receipt: receipt.clone(),
        digest: supplied_digest,
    })
}

pub fn baseline_measurement_policy_digest_v1(
    policy: &BaselineMeasurementPolicyV1,
) -> Result<BaselineMeasurementPolicyDigestV1, BaselineCovariateV1Error> {
    validate_policy_shape(policy)?;
    let mut writer = CanonicalWriter::default();
    encode_policy(&mut writer, policy)?;
    Ok(BaselineMeasurementPolicyDigestV1(domain_digest(
        POLICY_CONTEXT,
        POLICY_TAG,
        &writer.finish(),
    )))
}

pub fn baseline_covariate_measurement_receipt_digest_v1(
    receipt: &BaselineCovariateMeasurementReceiptV1,
) -> Result<BaselineCovariateMeasurementReceiptDigestV1, BaselineCovariateV1Error> {
    validate_receipt_shape(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_receipt(&mut writer, receipt)?;
    Ok(BaselineCovariateMeasurementReceiptDigestV1(domain_digest(
        RECEIPT_CONTEXT,
        RECEIPT_TAG,
        &writer.finish(),
    )))
}

fn find_confounder<'a>(
    plan: &'a TargetTrialEmulationPlanV2,
    confounder_id: &str,
) -> Result<&'a BaselineConfounderV2, BaselineCovariateV1Error> {
    plan.baseline_confounders
        .iter()
        .find(|value| value.confounder_id == confounder_id)
        .ok_or_else(|| BaselineCovariateV1Error::UnknownConfounder(confounder_id.to_string()))
}

fn validate_policy_against_confounder(
    policy: &BaselineMeasurementPolicyV1,
    confounder: &BaselineConfounderV2,
) -> Result<(), BaselineCovariateV1Error> {
    validate_policy_shape(policy)?;
    if policy.protocol_definition != confounder.definition {
        return Err(BaselineCovariateV1Error::ConfounderDefinitionMismatch);
    }
    if policy.window_end_offset_micros > confounder.measurement_window_end_offset_micros {
        return Err(BaselineCovariateV1Error::WindowExceedsProtocolBaselineCutoff);
    }
    Ok(())
}

fn validate_policy_shape(policy: &BaselineMeasurementPolicyV1) -> Result<(), BaselineCovariateV1Error> {
    if policy.schema_version != BASELINE_COVARIATE_MEASUREMENT_V1_VERSION
        || policy.confounder_id.trim().is_empty()
        || policy.concept.system.trim().is_empty()
        || policy.concept.code.trim().is_empty()
        || policy.concept.version.trim().is_empty()
        || policy.window_start_offset_micros > policy.window_end_offset_micros
        || policy.window_end_offset_micros > 0
    {
        return Err(BaselineCovariateV1Error::InvalidMeasurementPolicy);
    }
    validate_artifact(&policy.protocol_definition)?;
    validate_artifact(&policy.policy_evidence)
}

fn fact_matches_concept(fact: &ClinicalFact, concept: &ConceptIdentityV2) -> bool {
    fact.concept.coding.iter().any(|coding| {
        coding.system == concept.system
            && coding.code == concept.code
            && coding.version.as_deref() == Some(concept.version.as_str())
    })
}

fn select_measurement(
    rule: BaselineSelectionRuleV1,
    candidates: &[BaselineCandidateFactV1],
) -> Result<BaselineMeasurementStateV1, BaselineCovariateV1Error> {
    if candidates.is_empty() {
        return Ok(BaselineMeasurementStateV1::MissingMeasurement);
    }
    match rule {
        BaselineSelectionRuleV1::UniqueOnly => {
            if candidates.len() != 1 {
                return Err(BaselineCovariateV1Error::AmbiguousMultipleCandidates);
            }
            Ok(selected_state(&candidates[0]))
        }
        BaselineSelectionRuleV1::LatestAtOrBeforeEnd => {
            let latest_time = candidates
                .last()
                .ok_or(BaselineCovariateV1Error::AmbiguousMultipleCandidates)?
                .effective_at_micros;
            let mut latest = candidates.iter().rev().take_while(|value| value.effective_at_micros == latest_time);
            let selected = latest.next().ok_or(BaselineCovariateV1Error::AmbiguousMultipleCandidates)?;
            if latest.next().is_some() {
                return Err(BaselineCovariateV1Error::AmbiguousSelectionTie);
            }
            Ok(selected_state(selected))
        }
    }
}

fn selected_state(candidate: &BaselineCandidateFactV1) -> BaselineMeasurementStateV1 {
    BaselineMeasurementStateV1::Selected {
        fact_snapshot_digest: candidate.fact_snapshot_digest,
        effective_at_micros: candidate.effective_at_micros,
    }
}

fn validate_receipt_shape(receipt: &BaselineCovariateMeasurementReceiptV1) -> Result<(), BaselineCovariateV1Error> {
    if receipt.schema_version != BASELINE_COVARIATE_MEASUREMENT_V1_VERSION
        || receipt.protocol_digest == [0; 32]
        || receipt.emulation_plan_digest == [0; 32]
        || receipt.time_zero_receipt_digest == [0; 32]
        || receipt.measurement_policy_digest == [0; 32]
        || receipt.subject_resource_type.trim().is_empty()
        || receipt.subject_id.trim().is_empty()
        || receipt.confounder_id.trim().is_empty()
        || receipt.window_start_micros > receipt.window_end_micros
        || receipt.window_end_micros > receipt.time_zero_micros
    {
        return Err(BaselineCovariateV1Error::InvalidMeasurementReceipt);
    }
    validate_artifact(&receipt.candidate_set_evidence)?;
    let mut previous: Option<&BaselineCandidateFactV1> = None;
    let mut seen = HashSet::new();
    for candidate in &receipt.candidate_facts {
        if candidate.fact_snapshot_digest == [0; 32]
            || candidate.effective_at_micros < receipt.window_start_micros
            || candidate.effective_at_micros > receipt.window_end_micros
        {
            return Err(BaselineCovariateV1Error::InvalidMeasurementReceipt);
        }
        if let Some(prev) = previous {
            if prev >= candidate {
                return Err(BaselineCovariateV1Error::NonCanonicalCandidateOrder);
            }
        }
        previous = Some(candidate);
        if !seen.insert(candidate.fact_snapshot_digest) {
            return Err(BaselineCovariateV1Error::DuplicateCandidateFact);
        }
    }
    match &receipt.state {
        BaselineMeasurementStateV1::MissingMeasurement if !receipt.candidate_facts.is_empty() => {
            return Err(BaselineCovariateV1Error::StateCandidateMismatch);
        }
        BaselineMeasurementStateV1::Selected { fact_snapshot_digest, effective_at_micros } => {
            if !receipt.candidate_facts.iter().any(|candidate| {
                candidate.fact_snapshot_digest == *fact_snapshot_digest
                    && candidate.effective_at_micros == *effective_at_micros
            }) {
                return Err(BaselineCovariateV1Error::StateCandidateMismatch);
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_artifact(value: &EvidenceArtifactIdentityV2) -> Result<(), BaselineCovariateV1Error> {
    if value.namespace.trim().is_empty()
        || value.artifact_id.trim().is_empty()
        || value.version.trim().is_empty()
    {
        return Err(BaselineCovariateV1Error::IncompleteEvidenceIdentity);
    }
    if value.digest == [0; 32] {
        return Err(BaselineCovariateV1Error::ZeroEvidenceDigest);
    }
    Ok(())
}

fn domain_digest(context: &str, tag: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&BASELINE_COVARIATE_MEASUREMENT_V1_VERSION.to_be_bytes());
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
    fn string(&mut self, value: &str) -> Result<(), BaselineCovariateV1Error> { self.bytes(value.as_bytes()) }
    fn bytes(&mut self, value: &[u8]) -> Result<(), BaselineCovariateV1Error> {
        self.u32(u32::try_from(value.len()).map_err(|_| BaselineCovariateV1Error::LengthOverflow)?);
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn vector_len(&mut self, len: usize) -> Result<(), BaselineCovariateV1Error> {
        self.u32(u32::try_from(len).map_err(|_| BaselineCovariateV1Error::LengthOverflow)?);
        Ok(())
    }
}

fn encode_artifact(writer: &mut CanonicalWriter, value: &EvidenceArtifactIdentityV2) -> Result<(), BaselineCovariateV1Error> {
    writer.string(&value.namespace)?;
    writer.string(&value.artifact_id)?;
    writer.string(&value.version)?;
    writer.bytes(&value.digest)
}

fn encode_concept(writer: &mut CanonicalWriter, value: &ConceptIdentityV2) -> Result<(), BaselineCovariateV1Error> {
    writer.string(&value.system)?;
    writer.string(&value.code)?;
    writer.string(&value.version)
}

fn encode_policy(writer: &mut CanonicalWriter, value: &BaselineMeasurementPolicyV1) -> Result<(), BaselineCovariateV1Error> {
    writer.u16(value.schema_version);
    writer.string(&value.confounder_id)?;
    encode_artifact(writer, &value.protocol_definition)?;
    encode_concept(writer, &value.concept)?;
    writer.i64(value.window_start_offset_micros);
    writer.i64(value.window_end_offset_micros);
    writer.u8(match value.selection_rule { BaselineSelectionRuleV1::UniqueOnly => 0, BaselineSelectionRuleV1::LatestAtOrBeforeEnd => 1 });
    encode_artifact(writer, &value.policy_evidence)
}

fn encode_receipt(writer: &mut CanonicalWriter, value: &BaselineCovariateMeasurementReceiptV1) -> Result<(), BaselineCovariateV1Error> {
    writer.u16(value.schema_version);
    writer.bytes(&value.protocol_digest)?;
    writer.bytes(&value.emulation_plan_digest)?;
    writer.string(&value.subject_resource_type)?;
    writer.string(&value.subject_id)?;
    writer.bytes(&value.time_zero_receipt_digest)?;
    writer.i64(value.time_zero_micros);
    writer.string(&value.confounder_id)?;
    writer.bytes(&value.measurement_policy_digest)?;
    encode_artifact(writer, &value.candidate_set_evidence)?;
    writer.i64(value.window_start_micros);
    writer.i64(value.window_end_micros);
    writer.vector_len(value.candidate_facts.len())?;
    for candidate in &value.candidate_facts {
        writer.i64(candidate.effective_at_micros);
        writer.bytes(&candidate.fact_snapshot_digest)?;
    }
    match &value.state {
        BaselineMeasurementStateV1::Selected { fact_snapshot_digest, effective_at_micros } => {
            writer.u8(0);
            writer.bytes(fact_snapshot_digest)?;
            writer.i64(*effective_at_micros);
        }
        BaselineMeasurementStateV1::MissingMeasurement => writer.u8(1),
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum BaselineCovariateV1Error {
    #[error("unknown baseline confounder: {0}")]
    UnknownConfounder(String),
    #[error("measurement policy is invalid")]
    InvalidMeasurementPolicy,
    #[error("measurement policy does not bind the live confounder definition")]
    ConfounderDefinitionMismatch,
    #[error("measurement window exceeds the protocol baseline cutoff")]
    WindowExceedsProtocolBaselineCutoff,
    #[error("candidate fact belongs to a different subject")]
    SubjectMismatch,
    #[error("candidate fact does not match the exact confounder concept")]
    CandidateConceptMismatch,
    #[error("candidate fact is outside the declared baseline measurement window")]
    CandidateOutsideBaselineWindow,
    #[error("duplicate candidate fact snapshot")]
    DuplicateCandidateFact,
    #[error("unique-only selection received multiple candidate measurements")]
    AmbiguousMultipleCandidates,
    #[error("latest selection has multiple candidates at the same latest timestamp")]
    AmbiguousSelectionTie,
    #[error("baseline measurement receipt is invalid")]
    InvalidMeasurementReceipt,
    #[error("candidate facts are not in canonical order")]
    NonCanonicalCandidateOrder,
    #[error("selected/missing state disagrees with candidate evidence")]
    StateCandidateMismatch,
    #[error("serialized receipt does not reproduce from exact measurement evidence")]
    ReceiptMismatch,
    #[error("evidence artifact identity is incomplete")]
    IncompleteEvidenceIdentity,
    #[error("evidence digest may not be all zeroes")]
    ZeroEvidenceDigest,
    #[error("time arithmetic overflow")]
    TimeOverflow,
    #[error("canonical framing length exceeds v1 limit")]
    LengthOverflow,
    #[error(transparent)]
    Protocol(#[from] mycelix_target_trial_protocol_v2::TargetTrialV2Error),
    #[error(transparent)]
    Cohort(#[from] mycelix_target_trial_cohort_receipts_v2::TargetTrialCohortV2Error),
    #[error(transparent)]
    Clinical(#[from] mycelix_clinical_semantics::ClinicalSemanticsError),
    #[error(transparent)]
    Snapshot(#[from] mycelix_clinical_fact_snapshot::ClinicalFactSnapshotError),
}
