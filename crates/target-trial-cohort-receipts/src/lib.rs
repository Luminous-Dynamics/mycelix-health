#![deny(unsafe_code)]
//! Research-only subject-level target-trial receipts.
//!
//! These receipts bind observational eligibility, time zero, treatment-strategy
//! classification, and cohort assembly to one exact target-trial protocol and
//! emulation plan. They never represent random assignment or a causal effect.

use mycelix_clinical_phenotype::{
    evaluate_phenotype_v1, phenotype_definition_digest_v1, phenotype_evaluation_digest_v1,
    CriterionStateV1, PhenotypeDefinitionV1,
};
use mycelix_clinical_semantics::{ClinicalFact, SubjectRef};
use mycelix_target_trial_protocol::{
    target_trial_emulation_plan_digest_v1, target_trial_protocol_digest_v1,
    EvidenceArtifactIdentityV1, TargetTrialEmulationPlanV1, TargetTrialProtocolV1,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const TARGET_TRIAL_COHORT_RECEIPT_VERSION: u16 = 1;

const ELIGIBILITY_TAG: &[u8] = b"mycelix/target-trial-eligibility-receipt/v1";
const TIME_ZERO_TAG: &[u8] = b"mycelix/target-trial-time-zero-receipt/v1";
const STRATEGY_TAG: &[u8] = b"mycelix/target-trial-strategy-classification-receipt/v1";
const ENTRY_TAG: &[u8] = b"mycelix/target-trial-cohort-entry-receipt/v1";
const MANIFEST_TAG: &[u8] = b"mycelix/target-trial-cohort-manifest/v1";

const ELIGIBILITY_CONTEXT: &str = "mycelix.health.target-trial-eligibility-receipt.v1";
const TIME_ZERO_CONTEXT: &str = "mycelix.health.target-trial-time-zero-receipt.v1";
const STRATEGY_CONTEXT: &str = "mycelix.health.target-trial-strategy-classification-receipt.v1";
const ENTRY_CONTEXT: &str = "mycelix.health.target-trial-cohort-entry-receipt.v1";
const MANIFEST_CONTEXT: &str = "mycelix.health.target-trial-cohort-manifest.v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EligibilityReceiptV1 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub subject: SubjectRef,
    pub phenotype_definition_digest: [u8; 32],
    pub phenotype_evaluation_digest: [u8; 32],
    pub evidence_fact_digests: Vec<[u8; 32]>,
    pub eligibility_anchor_micros: i64,
    pub eligibility_anchor_evidence: EvidenceArtifactIdentityV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeZeroReceiptV1 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub subject: SubjectRef,
    pub data_source_id: String,
    pub data_source_identity: EvidenceArtifactIdentityV1,
    pub time_zero_micros: i64,
    pub time_zero_operationalization: EvidenceArtifactIdentityV1,
    pub time_zero_evidence: EvidenceArtifactIdentityV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StrategyClassificationReceiptV1 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub subject: SubjectRef,
    pub strategy_id: String,
    pub classification_rule: EvidenceArtifactIdentityV1,
    pub classification_evidence: EvidenceArtifactIdentityV1,
    pub classified_at_micros: i64,
    pub time_zero_receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CohortEntryReceiptV1 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub subject: SubjectRef,
    pub strategy_id: String,
    pub data_source_id: String,
    pub time_zero_micros: i64,
    pub eligibility_receipt_digest: [u8; 32],
    pub time_zero_receipt_digest: [u8; 32],
    pub strategy_classification_receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CohortEntryRefV1 {
    pub subject: SubjectRef,
    pub strategy_id: String,
    pub time_zero_micros: i64,
    pub cohort_entry_digest: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CohortManifestV1 {
    pub schema_version: u16,
    pub cohort_id: String,
    pub version: String,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub entries: Vec<CohortEntryRefV1>,
    pub assembly_software: EvidenceArtifactIdentityV1,
    pub assembly_environment: EvidenceArtifactIdentityV1,
    pub assembled_at_micros: i64,
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

digest_type!(EligibilityReceiptDigestV1);
digest_type!(TimeZeroReceiptDigestV1);
digest_type!(StrategyClassificationReceiptDigestV1);
digest_type!(CohortEntryReceiptDigestV1);
digest_type!(CohortManifestDigestV1);

pub fn build_eligibility_receipt_v1(
    protocol: &TargetTrialProtocolV1,
    plan: &TargetTrialEmulationPlanV1,
    definition: &PhenotypeDefinitionV1,
    subject_id: &str,
    facts: &[ClinicalFact],
    eligibility_anchor_micros: i64,
    eligibility_anchor_evidence: EvidenceArtifactIdentityV1,
) -> Result<EligibilityReceiptV1, TargetTrialCohortError> {
    plan.validate_against(protocol)?;
    validate_artifact(&eligibility_anchor_evidence)?;

    let definition_digest = phenotype_definition_digest_v1(definition)?.into_bytes();
    if definition.phenotype_id != protocol.eligibility.phenotype_id
        || definition.version != protocol.eligibility.version
        || definition_digest != protocol.eligibility.digest
    {
        return Err(TargetTrialCohortError::EligibilityDefinitionMismatch);
    }

    let evaluation = evaluate_phenotype_v1(definition, subject_id, facts)?;
    if evaluation.state != CriterionStateV1::Satisfied {
        return Err(TargetTrialCohortError::SubjectNotEligible(evaluation.state));
    }
    let phenotype_evaluation_digest = phenotype_evaluation_digest_v1(&evaluation)?.into_bytes();

    let receipt = EligibilityReceiptV1 {
        schema_version: TARGET_TRIAL_COHORT_RECEIPT_VERSION,
        protocol_digest: target_trial_protocol_digest_v1(protocol)?.into_bytes(),
        emulation_plan_digest: target_trial_emulation_plan_digest_v1(protocol, plan)?.into_bytes(),
        subject: evaluation.subject,
        phenotype_definition_digest: definition_digest,
        phenotype_evaluation_digest,
        evidence_fact_digests: evaluation.evidence_fact_digests,
        eligibility_anchor_micros,
        eligibility_anchor_evidence,
    };
    validate_eligibility_receipt(&receipt)?;
    Ok(receipt)
}

pub fn build_time_zero_receipt_v1(
    protocol: &TargetTrialProtocolV1,
    plan: &TargetTrialEmulationPlanV1,
    subject: SubjectRef,
    data_source_id: &str,
    time_zero_micros: i64,
    time_zero_evidence: EvidenceArtifactIdentityV1,
) -> Result<TimeZeroReceiptV1, TargetTrialCohortError> {
    plan.validate_against(protocol)?;
    validate_subject(&subject)?;
    validate_artifact(&time_zero_evidence)?;

    let source = plan
        .data_sources
        .iter()
        .find(|source| source.source_id == data_source_id)
        .ok_or_else(|| TargetTrialCohortError::UnknownDataSource(data_source_id.to_string()))?;
    if time_zero_micros < source.period_start_micros || time_zero_micros >= source.period_end_micros {
        return Err(TargetTrialCohortError::TimeZeroOutsideDataSourcePeriod);
    }

    let receipt = TimeZeroReceiptV1 {
        schema_version: TARGET_TRIAL_COHORT_RECEIPT_VERSION,
        protocol_digest: target_trial_protocol_digest_v1(protocol)?.into_bytes(),
        emulation_plan_digest: target_trial_emulation_plan_digest_v1(protocol, plan)?.into_bytes(),
        subject,
        data_source_id: source.source_id.clone(),
        data_source_identity: source.source_identity.clone(),
        time_zero_micros,
        time_zero_operationalization: plan.time_zero_operationalization.clone(),
        time_zero_evidence,
    };
    validate_time_zero_receipt(&receipt)?;
    Ok(receipt)
}

pub fn build_strategy_classification_receipt_v1(
    protocol: &TargetTrialProtocolV1,
    plan: &TargetTrialEmulationPlanV1,
    time_zero: &TimeZeroReceiptV1,
    strategy_id: &str,
    classified_at_micros: i64,
    classification_evidence: EvidenceArtifactIdentityV1,
) -> Result<StrategyClassificationReceiptV1, TargetTrialCohortError> {
    plan.validate_against(protocol)?;
    validate_time_zero_receipt(time_zero)?;
    validate_artifact(&classification_evidence)?;
    require_plan_binding(protocol, plan, time_zero.protocol_digest, time_zero.emulation_plan_digest)?;
    if classified_at_micros != time_zero.time_zero_micros {
        return Err(TargetTrialCohortError::ClassificationTimeZeroMismatch);
    }
    if !protocol
        .treatment_strategies
        .iter()
        .any(|strategy| strategy.strategy_id == strategy_id)
    {
        return Err(TargetTrialCohortError::UnknownTreatmentStrategy(strategy_id.to_string()));
    }
    let operationalization = plan
        .strategy_operationalizations
        .iter()
        .find(|mapping| mapping.strategy_id == strategy_id)
        .ok_or_else(|| TargetTrialCohortError::UnknownStrategyOperationalization(strategy_id.to_string()))?;

    let receipt = StrategyClassificationReceiptV1 {
        schema_version: TARGET_TRIAL_COHORT_RECEIPT_VERSION,
        protocol_digest: time_zero.protocol_digest,
        emulation_plan_digest: time_zero.emulation_plan_digest,
        subject: time_zero.subject.clone(),
        strategy_id: strategy_id.to_string(),
        classification_rule: operationalization.classification_rule.clone(),
        classification_evidence,
        classified_at_micros,
        time_zero_receipt_digest: time_zero_receipt_digest_v1(time_zero)?.into_bytes(),
    };
    validate_strategy_receipt(&receipt)?;
    Ok(receipt)
}

pub fn compose_cohort_entry_v1(
    protocol: &TargetTrialProtocolV1,
    plan: &TargetTrialEmulationPlanV1,
    eligibility: &EligibilityReceiptV1,
    time_zero: &TimeZeroReceiptV1,
    strategy: &StrategyClassificationReceiptV1,
) -> Result<CohortEntryReceiptV1, TargetTrialCohortError> {
    plan.validate_against(protocol)?;
    validate_eligibility_receipt(eligibility)?;
    validate_time_zero_receipt(time_zero)?;
    validate_strategy_receipt(strategy)?;

    let expected_protocol = target_trial_protocol_digest_v1(protocol)?.into_bytes();
    let expected_plan = target_trial_emulation_plan_digest_v1(protocol, plan)?.into_bytes();
    for (protocol_digest, plan_digest) in [
        (eligibility.protocol_digest, eligibility.emulation_plan_digest),
        (time_zero.protocol_digest, time_zero.emulation_plan_digest),
        (strategy.protocol_digest, strategy.emulation_plan_digest),
    ] {
        if protocol_digest != expected_protocol || plan_digest != expected_plan {
            return Err(TargetTrialCohortError::ProtocolOrEmulationMismatch);
        }
    }
    if !same_subject(&eligibility.subject, &time_zero.subject)
        || !same_subject(&eligibility.subject, &strategy.subject)
    {
        return Err(TargetTrialCohortError::SubjectMismatch);
    }
    if eligibility.eligibility_anchor_micros != time_zero.time_zero_micros
        || strategy.classified_at_micros != time_zero.time_zero_micros
    {
        return Err(TargetTrialCohortError::TimeZeroAlignmentMismatch);
    }
    let exact_time_zero_digest = time_zero_receipt_digest_v1(time_zero)?.into_bytes();
    if strategy.time_zero_receipt_digest != exact_time_zero_digest {
        return Err(TargetTrialCohortError::TimeZeroReceiptMismatch);
    }

    let receipt = CohortEntryReceiptV1 {
        schema_version: TARGET_TRIAL_COHORT_RECEIPT_VERSION,
        protocol_digest: expected_protocol,
        emulation_plan_digest: expected_plan,
        subject: eligibility.subject.clone(),
        strategy_id: strategy.strategy_id.clone(),
        data_source_id: time_zero.data_source_id.clone(),
        time_zero_micros: time_zero.time_zero_micros,
        eligibility_receipt_digest: eligibility_receipt_digest_v1(eligibility)?.into_bytes(),
        time_zero_receipt_digest: exact_time_zero_digest,
        strategy_classification_receipt_digest: strategy_classification_receipt_digest_v1(strategy)?.into_bytes(),
    };
    validate_cohort_entry(&receipt)?;
    Ok(receipt)
}

pub fn assemble_cohort_manifest_v1(
    protocol: &TargetTrialProtocolV1,
    plan: &TargetTrialEmulationPlanV1,
    cohort_id: &str,
    version: &str,
    entries: &[CohortEntryReceiptV1],
    assembly_software: EvidenceArtifactIdentityV1,
    assembly_environment: EvidenceArtifactIdentityV1,
    assembled_at_micros: i64,
) -> Result<CohortManifestV1, TargetTrialCohortError> {
    plan.validate_against(protocol)?;
    if cohort_id.trim().is_empty() || version.trim().is_empty() || entries.is_empty() {
        return Err(TargetTrialCohortError::InvalidCohortManifest);
    }
    validate_artifact(&assembly_software)?;
    validate_artifact(&assembly_environment)?;
    let expected_protocol = target_trial_protocol_digest_v1(protocol)?.into_bytes();
    let expected_plan = target_trial_emulation_plan_digest_v1(protocol, plan)?.into_bytes();

    let mut seen_subjects = HashSet::new();
    let mut refs = Vec::with_capacity(entries.len());
    for entry in entries {
        validate_cohort_entry(entry)?;
        if entry.protocol_digest != expected_protocol || entry.emulation_plan_digest != expected_plan {
            return Err(TargetTrialCohortError::ProtocolOrEmulationMismatch);
        }
        if assembled_at_micros < entry.time_zero_micros {
            return Err(TargetTrialCohortError::AssemblyPredatesTimeZero);
        }
        let subject_key = format!("{}\u{1f}{}", entry.subject.resource_type, entry.subject.id);
        if !seen_subjects.insert(subject_key) {
            return Err(TargetTrialCohortError::DuplicateCohortSubject);
        }
        refs.push(CohortEntryRefV1 {
            subject: entry.subject.clone(),
            strategy_id: entry.strategy_id.clone(),
            time_zero_micros: entry.time_zero_micros,
            cohort_entry_digest: cohort_entry_receipt_digest_v1(entry)?.into_bytes(),
        });
    }
    refs.sort_by(|left, right| {
        left.subject
            .resource_type
            .cmp(&right.subject.resource_type)
            .then(left.subject.id.cmp(&right.subject.id))
            .then(left.strategy_id.cmp(&right.strategy_id))
            .then(left.time_zero_micros.cmp(&right.time_zero_micros))
    });

    let manifest = CohortManifestV1 {
        schema_version: TARGET_TRIAL_COHORT_RECEIPT_VERSION,
        cohort_id: cohort_id.to_string(),
        version: version.to_string(),
        protocol_digest: expected_protocol,
        emulation_plan_digest: expected_plan,
        entries: refs,
        assembly_software,
        assembly_environment,
        assembled_at_micros,
    };
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub fn eligibility_receipt_digest_v1(
    receipt: &EligibilityReceiptV1,
) -> Result<EligibilityReceiptDigestV1, TargetTrialCohortError> {
    validate_eligibility_receipt(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_eligibility(&mut writer, receipt)?;
    Ok(EligibilityReceiptDigestV1(domain_digest(
        ELIGIBILITY_CONTEXT,
        ELIGIBILITY_TAG,
        &writer.finish(),
    )))
}

pub fn time_zero_receipt_digest_v1(
    receipt: &TimeZeroReceiptV1,
) -> Result<TimeZeroReceiptDigestV1, TargetTrialCohortError> {
    validate_time_zero_receipt(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_time_zero(&mut writer, receipt)?;
    Ok(TimeZeroReceiptDigestV1(domain_digest(
        TIME_ZERO_CONTEXT,
        TIME_ZERO_TAG,
        &writer.finish(),
    )))
}

pub fn strategy_classification_receipt_digest_v1(
    receipt: &StrategyClassificationReceiptV1,
) -> Result<StrategyClassificationReceiptDigestV1, TargetTrialCohortError> {
    validate_strategy_receipt(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_strategy(&mut writer, receipt)?;
    Ok(StrategyClassificationReceiptDigestV1(domain_digest(
        STRATEGY_CONTEXT,
        STRATEGY_TAG,
        &writer.finish(),
    )))
}

pub fn cohort_entry_receipt_digest_v1(
    receipt: &CohortEntryReceiptV1,
) -> Result<CohortEntryReceiptDigestV1, TargetTrialCohortError> {
    validate_cohort_entry(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_entry(&mut writer, receipt)?;
    Ok(CohortEntryReceiptDigestV1(domain_digest(
        ENTRY_CONTEXT,
        ENTRY_TAG,
        &writer.finish(),
    )))
}

pub fn cohort_manifest_digest_v1(
    manifest: &CohortManifestV1,
) -> Result<CohortManifestDigestV1, TargetTrialCohortError> {
    validate_manifest(manifest)?;
    let mut writer = CanonicalWriter::default();
    encode_manifest(&mut writer, manifest)?;
    Ok(CohortManifestDigestV1(domain_digest(
        MANIFEST_CONTEXT,
        MANIFEST_TAG,
        &writer.finish(),
    )))
}

fn validate_eligibility_receipt(receipt: &EligibilityReceiptV1) -> Result<(), TargetTrialCohortError> {
    require_schema(receipt.schema_version)?;
    validate_nonzero(receipt.protocol_digest)?;
    validate_nonzero(receipt.emulation_plan_digest)?;
    validate_nonzero(receipt.phenotype_definition_digest)?;
    validate_nonzero(receipt.phenotype_evaluation_digest)?;
    validate_subject(&receipt.subject)?;
    validate_artifact(&receipt.eligibility_anchor_evidence)?;
    if receipt.evidence_fact_digests.iter().any(|digest| *digest == [0; 32]) {
        return Err(TargetTrialCohortError::ZeroDigest);
    }
    Ok(())
}

fn validate_time_zero_receipt(receipt: &TimeZeroReceiptV1) -> Result<(), TargetTrialCohortError> {
    require_schema(receipt.schema_version)?;
    validate_nonzero(receipt.protocol_digest)?;
    validate_nonzero(receipt.emulation_plan_digest)?;
    validate_subject(&receipt.subject)?;
    if receipt.data_source_id.trim().is_empty() {
        return Err(TargetTrialCohortError::UnknownDataSource(receipt.data_source_id.clone()));
    }
    validate_artifact(&receipt.data_source_identity)?;
    validate_artifact(&receipt.time_zero_operationalization)?;
    validate_artifact(&receipt.time_zero_evidence)?;
    Ok(())
}

fn validate_strategy_receipt(receipt: &StrategyClassificationReceiptV1) -> Result<(), TargetTrialCohortError> {
    require_schema(receipt.schema_version)?;
    validate_nonzero(receipt.protocol_digest)?;
    validate_nonzero(receipt.emulation_plan_digest)?;
    validate_nonzero(receipt.time_zero_receipt_digest)?;
    validate_subject(&receipt.subject)?;
    if receipt.strategy_id.trim().is_empty() {
        return Err(TargetTrialCohortError::UnknownTreatmentStrategy(receipt.strategy_id.clone()));
    }
    validate_artifact(&receipt.classification_rule)?;
    validate_artifact(&receipt.classification_evidence)?;
    Ok(())
}

fn validate_cohort_entry(receipt: &CohortEntryReceiptV1) -> Result<(), TargetTrialCohortError> {
    require_schema(receipt.schema_version)?;
    validate_nonzero(receipt.protocol_digest)?;
    validate_nonzero(receipt.emulation_plan_digest)?;
    validate_nonzero(receipt.eligibility_receipt_digest)?;
    validate_nonzero(receipt.time_zero_receipt_digest)?;
    validate_nonzero(receipt.strategy_classification_receipt_digest)?;
    validate_subject(&receipt.subject)?;
    if receipt.strategy_id.trim().is_empty() || receipt.data_source_id.trim().is_empty() {
        return Err(TargetTrialCohortError::InvalidCohortEntry);
    }
    Ok(())
}

fn validate_manifest(manifest: &CohortManifestV1) -> Result<(), TargetTrialCohortError> {
    require_schema(manifest.schema_version)?;
    validate_nonzero(manifest.protocol_digest)?;
    validate_nonzero(manifest.emulation_plan_digest)?;
    validate_artifact(&manifest.assembly_software)?;
    validate_artifact(&manifest.assembly_environment)?;
    if manifest.cohort_id.trim().is_empty() || manifest.version.trim().is_empty() || manifest.entries.is_empty() {
        return Err(TargetTrialCohortError::InvalidCohortManifest);
    }
    let mut subjects = HashSet::new();
    for entry in &manifest.entries {
        validate_subject(&entry.subject)?;
        validate_nonzero(entry.cohort_entry_digest)?;
        if entry.strategy_id.trim().is_empty() {
            return Err(TargetTrialCohortError::InvalidCohortEntry);
        }
        let key = format!("{}\u{1f}{}", entry.subject.resource_type, entry.subject.id);
        if !subjects.insert(key) {
            return Err(TargetTrialCohortError::DuplicateCohortSubject);
        }
    }
    Ok(())
}

fn require_plan_binding(
    protocol: &TargetTrialProtocolV1,
    plan: &TargetTrialEmulationPlanV1,
    protocol_digest: [u8; 32],
    emulation_digest: [u8; 32],
) -> Result<(), TargetTrialCohortError> {
    if target_trial_protocol_digest_v1(protocol)?.into_bytes() != protocol_digest
        || target_trial_emulation_plan_digest_v1(protocol, plan)?.into_bytes() != emulation_digest
    {
        return Err(TargetTrialCohortError::ProtocolOrEmulationMismatch);
    }
    Ok(())
}

fn validate_artifact(value: &EvidenceArtifactIdentityV1) -> Result<(), TargetTrialCohortError> {
    if value.namespace.trim().is_empty()
        || value.artifact_id.trim().is_empty()
        || value.version.trim().is_empty()
        || value.digest == [0; 32]
    {
        return Err(TargetTrialCohortError::InvalidEvidenceArtifact);
    }
    Ok(())
}

fn validate_subject(subject: &SubjectRef) -> Result<(), TargetTrialCohortError> {
    if subject.resource_type.trim().is_empty() || subject.id.trim().is_empty() {
        return Err(TargetTrialCohortError::InvalidSubject);
    }
    Ok(())
}

fn same_subject(left: &SubjectRef, right: &SubjectRef) -> bool {
    left.resource_type == right.resource_type && left.id == right.id
}

fn require_schema(version: u16) -> Result<(), TargetTrialCohortError> {
    if version != TARGET_TRIAL_COHORT_RECEIPT_VERSION {
        return Err(TargetTrialCohortError::UnsupportedSchemaVersion(version));
    }
    Ok(())
}

fn validate_nonzero(value: [u8; 32]) -> Result<(), TargetTrialCohortError> {
    if value == [0; 32] {
        return Err(TargetTrialCohortError::ZeroDigest);
    }
    Ok(())
}

fn domain_digest(context: &str, tag: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&TARGET_TRIAL_COHORT_RECEIPT_VERSION.to_be_bytes());
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
    fn u16(&mut self, value: u16) { self.bytes.extend_from_slice(&value.to_be_bytes()); }
    fn u32(&mut self, value: u32) { self.bytes.extend_from_slice(&value.to_be_bytes()); }
    fn i64(&mut self, value: i64) { self.bytes.extend_from_slice(&value.to_be_bytes()); }
    fn bytes(&mut self, value: &[u8]) -> Result<(), TargetTrialCohortError> {
        self.u32(u32::try_from(value.len()).map_err(|_| TargetTrialCohortError::LengthOverflow)?);
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn string(&mut self, value: &str) -> Result<(), TargetTrialCohortError> { self.bytes(value.as_bytes()) }
    fn vector_len(&mut self, len: usize) -> Result<(), TargetTrialCohortError> {
        self.u32(u32::try_from(len).map_err(|_| TargetTrialCohortError::LengthOverflow)?);
        Ok(())
    }
}

fn encode_subject(writer: &mut CanonicalWriter, subject: &SubjectRef) -> Result<(), TargetTrialCohortError> {
    writer.string(&subject.resource_type)?;
    writer.string(&subject.id)?;
    Ok(())
}

fn encode_artifact(writer: &mut CanonicalWriter, artifact: &EvidenceArtifactIdentityV1) -> Result<(), TargetTrialCohortError> {
    writer.string(&artifact.namespace)?;
    writer.string(&artifact.artifact_id)?;
    writer.string(&artifact.version)?;
    writer.bytes(&artifact.digest)?;
    Ok(())
}

fn encode_common(
    writer: &mut CanonicalWriter,
    schema_version: u16,
    protocol_digest: [u8; 32],
    emulation_plan_digest: [u8; 32],
    subject: &SubjectRef,
) -> Result<(), TargetTrialCohortError> {
    writer.u16(schema_version);
    writer.bytes(&protocol_digest)?;
    writer.bytes(&emulation_plan_digest)?;
    encode_subject(writer, subject)
}

fn encode_eligibility(writer: &mut CanonicalWriter, receipt: &EligibilityReceiptV1) -> Result<(), TargetTrialCohortError> {
    encode_common(writer, receipt.schema_version, receipt.protocol_digest, receipt.emulation_plan_digest, &receipt.subject)?;
    writer.bytes(&receipt.phenotype_definition_digest)?;
    writer.bytes(&receipt.phenotype_evaluation_digest)?;
    writer.vector_len(receipt.evidence_fact_digests.len())?;
    for digest in &receipt.evidence_fact_digests { writer.bytes(digest)?; }
    writer.i64(receipt.eligibility_anchor_micros);
    encode_artifact(writer, &receipt.eligibility_anchor_evidence)
}

fn encode_time_zero(writer: &mut CanonicalWriter, receipt: &TimeZeroReceiptV1) -> Result<(), TargetTrialCohortError> {
    encode_common(writer, receipt.schema_version, receipt.protocol_digest, receipt.emulation_plan_digest, &receipt.subject)?;
    writer.string(&receipt.data_source_id)?;
    encode_artifact(writer, &receipt.data_source_identity)?;
    writer.i64(receipt.time_zero_micros);
    encode_artifact(writer, &receipt.time_zero_operationalization)?;
    encode_artifact(writer, &receipt.time_zero_evidence)
}

fn encode_strategy(writer: &mut CanonicalWriter, receipt: &StrategyClassificationReceiptV1) -> Result<(), TargetTrialCohortError> {
    encode_common(writer, receipt.schema_version, receipt.protocol_digest, receipt.emulation_plan_digest, &receipt.subject)?;
    writer.string(&receipt.strategy_id)?;
    encode_artifact(writer, &receipt.classification_rule)?;
    encode_artifact(writer, &receipt.classification_evidence)?;
    writer.i64(receipt.classified_at_micros);
    writer.bytes(&receipt.time_zero_receipt_digest)
}

fn encode_entry(writer: &mut CanonicalWriter, receipt: &CohortEntryReceiptV1) -> Result<(), TargetTrialCohortError> {
    encode_common(writer, receipt.schema_version, receipt.protocol_digest, receipt.emulation_plan_digest, &receipt.subject)?;
    writer.string(&receipt.strategy_id)?;
    writer.string(&receipt.data_source_id)?;
    writer.i64(receipt.time_zero_micros);
    writer.bytes(&receipt.eligibility_receipt_digest)?;
    writer.bytes(&receipt.time_zero_receipt_digest)?;
    writer.bytes(&receipt.strategy_classification_receipt_digest)
}

fn encode_manifest(writer: &mut CanonicalWriter, manifest: &CohortManifestV1) -> Result<(), TargetTrialCohortError> {
    writer.u16(manifest.schema_version);
    writer.string(&manifest.cohort_id)?;
    writer.string(&manifest.version)?;
    writer.bytes(&manifest.protocol_digest)?;
    writer.bytes(&manifest.emulation_plan_digest)?;
    writer.vector_len(manifest.entries.len())?;
    for entry in &manifest.entries {
        encode_subject(writer, &entry.subject)?;
        writer.string(&entry.strategy_id)?;
        writer.i64(entry.time_zero_micros);
        writer.bytes(&entry.cohort_entry_digest)?;
    }
    encode_artifact(writer, &manifest.assembly_software)?;
    encode_artifact(writer, &manifest.assembly_environment)?;
    writer.i64(manifest.assembled_at_micros);
    Ok(())
}

#[derive(Debug, Error)]
pub enum TargetTrialCohortError {
    #[error("unsupported cohort-receipt schema version: {0}")]
    UnsupportedSchemaVersion(u16),
    #[error("evidence artifact identity is invalid")]
    InvalidEvidenceArtifact,
    #[error("subject identity is invalid")]
    InvalidSubject,
    #[error("digest may not be all zeroes")]
    ZeroDigest,
    #[error("protocol/emulation identity mismatch")]
    ProtocolOrEmulationMismatch,
    #[error("protocol eligibility phenotype does not match the supplied definition")]
    EligibilityDefinitionMismatch,
    #[error("subject is not eligible: {0:?}")]
    SubjectNotEligible(CriterionStateV1),
    #[error("unknown observational data source: {0}")]
    UnknownDataSource(String),
    #[error("time zero falls outside the selected data-source period")]
    TimeZeroOutsideDataSourcePeriod,
    #[error("unknown target-trial treatment strategy: {0}")]
    UnknownTreatmentStrategy(String),
    #[error("missing observational strategy operationalization: {0}")]
    UnknownStrategyOperationalization(String),
    #[error("observational treatment classification must occur exactly at time zero in v1")]
    ClassificationTimeZeroMismatch,
    #[error("subject mismatch across target-trial receipts")]
    SubjectMismatch,
    #[error("eligibility, classification, and follow-up start are not aligned at one time zero")]
    TimeZeroAlignmentMismatch,
    #[error("strategy classification is not bound to the exact time-zero receipt")]
    TimeZeroReceiptMismatch,
    #[error("cohort entry is invalid")]
    InvalidCohortEntry,
    #[error("cohort manifest is invalid")]
    InvalidCohortManifest,
    #[error("cohort contains more than one entry for the same subject")]
    DuplicateCohortSubject,
    #[error("cohort assembly timestamp predates a subject time zero")]
    AssemblyPredatesTimeZero,
    #[error("canonical framing length exceeds v1 limit")]
    LengthOverflow,
    #[error(transparent)]
    Protocol(#[from] mycelix_target_trial_protocol::TargetTrialError),
    #[error(transparent)]
    Phenotype(#[from] mycelix_clinical_phenotype::PhenotypeError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_phenotype::{
        ConceptIdentityV1, CoverageEvidenceV1, CoverageStatusV1, CriterionV1,
        ObservationWindowV1,
    };
    use mycelix_clinical_semantics::{
        ClinicalValue, CodeableConcept, Coding, FactProvenance, SubjectRef,
    };
    use mycelix_target_trial_protocol::{
        BaselineConfounderV1, CausalContrastV1, CausalEstimandV1,
        CompetingEventStrategyV1, DataSourceV1, EffectMeasureV1, FollowUpV1,
        HypotheticalRandomAssignmentV1, IdentifyingAssumptionV1, MaskingV1,
        OutcomeOperationalizationV1, OutcomeV1, PhenotypeDefinitionRefV1,
        SoftwareEnvironmentV1, StrategyOperationalizationV1, TreatmentStrategyV1,
        TARGET_TRIAL_PROTOCOL_VERSION,
    };

    fn artifact(namespace: &str, id: &str, byte: u8) -> EvidenceArtifactIdentityV1 {
        EvidenceArtifactIdentityV1 {
            namespace: namespace.to_string(),
            artifact_id: id.to_string(),
            version: "1".to_string(),
            digest: [byte; 32],
        }
    }

    fn eligibility_definition() -> PhenotypeDefinitionV1 {
        PhenotypeDefinitionV1 {
            schema_version: 1,
            phenotype_id: "eligible".to_string(),
            version: "1".to_string(),
            subject_resource_type: "Patient".to_string(),
            window: ObservationWindowV1 {
                start_micros: 0,
                end_micros: 2_000,
                coverage: vec![CoverageEvidenceV1 {
                    domain: "diagnoses".to_string(),
                    status: CoverageStatusV1::Complete,
                    source: mycelix_clinical_phenotype::EvidenceArtifactIdentityV1 {
                        namespace: "coverage".to_string(),
                        artifact_id: "site-a".to_string(),
                        version: "1".to_string(),
                        digest: [31; 32],
                    },
                }],
            },
            criterion: CriterionV1::FactPresent {
                criterion_id: "has-entry-condition".to_string(),
                concept: ConceptIdentityV1 {
                    system: "http://snomed.info/sct".to_string(),
                    code: "123".to_string(),
                    version: "2026-09".to_string(),
                },
                minimum_count: 1,
                coverage_domain: "diagnoses".to_string(),
            },
            definition_evidence: mycelix_clinical_phenotype::EvidenceArtifactIdentityV1 {
                namespace: "phenotype-definition".to_string(),
                artifact_id: "eligible".to_string(),
                version: "1".to_string(),
                digest: [32; 32],
            },
        }
    }

    fn clinical_fact(subject_id: &str) -> ClinicalFact {
        ClinicalFact {
            fact_id: format!("fact:{subject_id}"),
            subject: SubjectRef { resource_type: "Patient".to_string(), id: subject_id.to_string() },
            concept: CodeableConcept {
                coding: vec![Coding {
                    system: "http://snomed.info/sct".to_string(),
                    code: "123".to_string(),
                    display: None,
                    version: Some("2026-09".to_string()),
                }],
                text: None,
            },
            value: ClinicalValue::Boolean(true),
            effective_at_micros: 1_000,
            provenance: FactProvenance {
                source_system: "test-ehr".to_string(),
                source_resource_type: "Condition".to_string(),
                source_resource_id: format!("condition:{subject_id}"),
                source_version: Some("1".to_string()),
                recorded_at_micros: 1_100,
                asserted_by: None,
                transformation: None,
            },
            uncertainty: None,
        }
    }

    fn protocol(definition: &PhenotypeDefinitionV1) -> TargetTrialProtocolV1 {
        let eligibility_digest = phenotype_definition_digest_v1(definition).unwrap().into_bytes();
        TargetTrialProtocolV1 {
            schema_version: TARGET_TRIAL_PROTOCOL_VERSION,
            protocol_id: "tte:receipt-test".to_string(),
            version: "1".to_string(),
            causal_question: "A versus B".to_string(),
            rationale_evidence: vec![artifact("literature", "rationale", 1)],
            eligibility: PhenotypeDefinitionRefV1 {
                phenotype_id: definition.phenotype_id.clone(),
                version: definition.version.clone(),
                digest: eligibility_digest,
            },
            treatment_strategies: vec![
                TreatmentStrategyV1 { strategy_id: "a".to_string(), label: "A".to_string(), intervention_definition: artifact("treatment", "a", 2) },
                TreatmentStrategyV1 { strategy_id: "b".to_string(), label: "B".to_string(), intervention_definition: artifact("treatment", "b", 3) },
            ],
            assignment: HypotheticalRandomAssignmentV1 { allocation_description: "hypothetical random assignment".to_string(), masking: MaskingV1::OpenLabel },
            time_zero_definition: artifact("design", "time-zero", 4),
            follow_up: FollowUpV1 { maximum_duration_micros: 10_000, end_conditions: vec![artifact("design", "follow-up-end", 5)] },
            outcomes: vec![OutcomeV1 {
                outcome_id: "outcome".to_string(),
                phenotype: PhenotypeDefinitionRefV1 { phenotype_id: "outcome".to_string(), version: "1".to_string(), digest: [6; 32] },
                assessment_window_start_offset_micros: 0,
                assessment_window_end_offset_micros: 10_000,
                primary: true,
            }],
            estimands: vec![CausalEstimandV1 {
                estimand_id: "itt".to_string(),
                treatment_strategy_id: "a".to_string(),
                comparator_strategy_id: "b".to_string(),
                outcome_id: "outcome".to_string(),
                contrast: CausalContrastV1::IntentionToTreat,
                effect_measure: EffectMeasureV1::RiskDifference,
                competing_event_strategy: CompetingEventStrategyV1::NotApplicable,
                estimand_definition: artifact("estimand", "itt", 7),
            }],
            identifying_assumptions: vec![IdentifyingAssumptionV1 { assumption_id: "exchangeability".to_string(), statement: "declared assumption".to_string(), related_variable_definitions: vec![] }],
            analysis_plan: artifact("analysis", "primary", 8),
            sensitivity_analysis_plans: vec![],
        }
    }

    fn plan(protocol: &TargetTrialProtocolV1) -> TargetTrialEmulationPlanV1 {
        TargetTrialEmulationPlanV1 {
            schema_version: TARGET_TRIAL_PROTOCOL_VERSION,
            emulation_id: "tte:receipt-test:site-a".to_string(),
            version: "1".to_string(),
            protocol_digest: target_trial_protocol_digest_v1(protocol).unwrap().into_bytes(),
            observational_design_statement: "Observational classification; no random assignment.".to_string(),
            data_sources: vec![DataSourceV1 {
                source_id: "site-a".to_string(),
                original_purpose: "routine care".to_string(),
                source_type: "EHR".to_string(),
                setting: "health system".to_string(),
                geography: "site-a".to_string(),
                period_start_micros: 0,
                period_end_micros: 10_000,
                source_identity: artifact("data-source", "site-a", 9),
            }],
            eligibility_operationalization: artifact("operationalization", "eligibility", 10),
            strategy_operationalizations: vec![
                StrategyOperationalizationV1 { strategy_id: "a".to_string(), classification_rule: artifact("operationalization", "strategy-a", 11) },
                StrategyOperationalizationV1 { strategy_id: "b".to_string(), classification_rule: artifact("operationalization", "strategy-b", 12) },
            ],
            time_zero_operationalization: artifact("operationalization", "time-zero", 13),
            outcome_operationalizations: vec![OutcomeOperationalizationV1 { outcome_id: "outcome".to_string(), operationalization: artifact("operationalization", "outcome", 14) }],
            baseline_confounders: vec![BaselineConfounderV1 { confounder_id: "age".to_string(), definition: artifact("confounder", "age", 15), measurement_window_end_offset_micros: 0 }],
            censoring_plan: artifact("analysis", "censoring", 16),
            adherence_plan: None,
            time_varying_confounding_plan: None,
            missing_data_plan: artifact("analysis", "missing", 17),
            estimator_plan: artifact("analysis", "estimator", 18),
            software_environment: SoftwareEnvironmentV1 { software: artifact("software", "analysis", 19), environment: artifact("environment", "analysis", 20) },
            vocabulary_and_mapping_artifacts: vec![],
        }
    }

    fn receipts(subject_id: &str) -> (TargetTrialProtocolV1, TargetTrialEmulationPlanV1, EligibilityReceiptV1, TimeZeroReceiptV1, StrategyClassificationReceiptV1) {
        let definition = eligibility_definition();
        let protocol = protocol(&definition);
        let plan = plan(&protocol);
        let eligibility = build_eligibility_receipt_v1(
            &protocol,
            &plan,
            &definition,
            subject_id,
            &[clinical_fact(subject_id)],
            2_000,
            artifact("eligibility-anchor", subject_id, 21),
        ).unwrap();
        let time_zero = build_time_zero_receipt_v1(
            &protocol,
            &plan,
            SubjectRef { resource_type: "Patient".to_string(), id: subject_id.to_string() },
            "site-a",
            2_000,
            artifact("time-zero-evidence", subject_id, 22),
        ).unwrap();
        let strategy = build_strategy_classification_receipt_v1(
            &protocol,
            &plan,
            &time_zero,
            "a",
            2_000,
            artifact("classification-evidence", subject_id, 23),
        ).unwrap();
        (protocol, plan, eligibility, time_zero, strategy)
    }

    #[test]
    fn subject_receipts_compose_only_when_time_zero_aligns() {
        let (protocol, plan, eligibility, time_zero, strategy) = receipts("p1");
        let entry = compose_cohort_entry_v1(&protocol, &plan, &eligibility, &time_zero, &strategy).unwrap();
        assert_eq!(entry.time_zero_micros, 2_000);
        assert_eq!(entry.strategy_id, "a");
    }

    #[test]
    fn classification_after_time_zero_is_rejected() {
        let definition = eligibility_definition();
        let protocol = protocol(&definition);
        let plan = plan(&protocol);
        let time_zero = build_time_zero_receipt_v1(
            &protocol,
            &plan,
            SubjectRef { resource_type: "Patient".to_string(), id: "p1".to_string() },
            "site-a",
            2_000,
            artifact("time-zero-evidence", "p1", 22),
        ).unwrap();
        assert!(matches!(
            build_strategy_classification_receipt_v1(&protocol, &plan, &time_zero, "a", 2_001, artifact("classification", "p1", 23)),
            Err(TargetTrialCohortError::ClassificationTimeZeroMismatch)
        ));
    }

    #[test]
    fn cross_subject_receipt_composition_is_rejected() {
        let (protocol, plan, eligibility, time_zero, _) = receipts("p1");
        let (_, _, _, time_zero_p2, strategy_p2) = receipts("p2");
        let _ = time_zero_p2;
        assert!(matches!(
            compose_cohort_entry_v1(&protocol, &plan, &eligibility, &time_zero, &strategy_p2),
            Err(TargetTrialCohortError::SubjectMismatch)
        ));
    }

    #[test]
    fn time_zero_receipt_substitution_is_rejected() {
        let (protocol, plan, eligibility, time_zero, mut strategy) = receipts("p1");
        strategy.time_zero_receipt_digest = [99; 32];
        assert!(matches!(
            compose_cohort_entry_v1(&protocol, &plan, &eligibility, &time_zero, &strategy),
            Err(TargetTrialCohortError::TimeZeroReceiptMismatch)
        ));
    }

    #[test]
    fn duplicate_subject_cannot_enter_cohort_twice() {
        let (protocol, plan, eligibility, time_zero, strategy) = receipts("p1");
        let entry = compose_cohort_entry_v1(&protocol, &plan, &eligibility, &time_zero, &strategy).unwrap();
        assert!(matches!(
            assemble_cohort_manifest_v1(
                &protocol,
                &plan,
                "cohort-1",
                "1",
                &[entry.clone(), entry],
                artifact("software", "assembler", 24),
                artifact("environment", "assembler", 25),
                3_000,
            ),
            Err(TargetTrialCohortError::DuplicateCohortSubject)
        ));
    }

    #[test]
    fn cohort_manifest_identity_is_order_independent_after_canonical_sort() {
        let (protocol, plan, e1, t1, s1) = receipts("p1");
        let (_, _, e2, t2, s2) = receipts("p2");
        let entry1 = compose_cohort_entry_v1(&protocol, &plan, &e1, &t1, &s1).unwrap();
        let entry2 = compose_cohort_entry_v1(&protocol, &plan, &e2, &t2, &s2).unwrap();
        let first = assemble_cohort_manifest_v1(
            &protocol, &plan, "cohort", "1", &[entry1.clone(), entry2.clone()],
            artifact("software", "assembler", 24), artifact("environment", "assembler", 25), 3_000,
        ).unwrap();
        let second = assemble_cohort_manifest_v1(
            &protocol, &plan, "cohort", "1", &[entry2, entry1],
            artifact("software", "assembler", 24), artifact("environment", "assembler", 25), 3_000,
        ).unwrap();
        assert_eq!(cohort_manifest_digest_v1(&first).unwrap(), cohort_manifest_digest_v1(&second).unwrap());
    }
}
