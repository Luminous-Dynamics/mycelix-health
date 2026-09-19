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
use std::{cmp::Ordering, collections::HashSet};
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
    pub eligibility_operationalization: EvidenceArtifactIdentityV1,
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
            pub const fn into_bytes(self) -> [u8; 32] { self.0 }
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
    let receipt = EligibilityReceiptV1 {
        schema_version: TARGET_TRIAL_COHORT_RECEIPT_VERSION,
        protocol_digest: target_trial_protocol_digest_v1(protocol)?.into_bytes(),
        emulation_plan_digest: target_trial_emulation_plan_digest_v1(protocol, plan)?.into_bytes(),
        subject: evaluation.subject,
        phenotype_definition_digest: definition_digest,
        phenotype_evaluation_digest: phenotype_evaluation_digest_v1(&evaluation)?.into_bytes(),
        evidence_fact_digests: evaluation.evidence_fact_digests,
        eligibility_operationalization: plan.eligibility_operationalization.clone(),
        eligibility_anchor_micros,
        eligibility_anchor_evidence,
    };
    validate_eligibility_receipt(&receipt)?;
    validate_eligibility_against(protocol, plan, &receipt)?;
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
    validate_time_zero_against(protocol, plan, &receipt)?;
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
    validate_time_zero_against(protocol, plan, time_zero)?;
    validate_artifact(&classification_evidence)?;
    if classified_at_micros != time_zero.time_zero_micros {
        return Err(TargetTrialCohortError::ClassificationTimeZeroMismatch);
    }
    let operationalization = plan
        .strategy_operationalizations
        .iter()
        .find(|mapping| mapping.strategy_id == strategy_id)
        .ok_or_else(|| TargetTrialCohortError::UnknownStrategyOperationalization(strategy_id.to_string()))?;
    if !protocol.treatment_strategies.iter().any(|strategy| strategy.strategy_id == strategy_id) {
        return Err(TargetTrialCohortError::UnknownTreatmentStrategy(strategy_id.to_string()));
    }
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
    validate_strategy_against(protocol, plan, time_zero, &receipt)?;
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
    validate_eligibility_against(protocol, plan, eligibility)?;
    validate_time_zero_against(protocol, plan, time_zero)?;
    validate_strategy_against(protocol, plan, time_zero, strategy)?;

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
        protocol_digest: eligibility.protocol_digest,
        emulation_plan_digest: eligibility.emulation_plan_digest,
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
    refs.sort_by(compare_entry_refs);
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

pub fn eligibility_receipt_digest_v1(receipt: &EligibilityReceiptV1) -> Result<EligibilityReceiptDigestV1, TargetTrialCohortError> {
    validate_eligibility_receipt(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_eligibility(&mut writer, receipt)?;
    Ok(EligibilityReceiptDigestV1(domain_digest(ELIGIBILITY_CONTEXT, ELIGIBILITY_TAG, &writer.finish())))
}

pub fn time_zero_receipt_digest_v1(receipt: &TimeZeroReceiptV1) -> Result<TimeZeroReceiptDigestV1, TargetTrialCohortError> {
    validate_time_zero_receipt(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_time_zero(&mut writer, receipt)?;
    Ok(TimeZeroReceiptDigestV1(domain_digest(TIME_ZERO_CONTEXT, TIME_ZERO_TAG, &writer.finish())))
}

pub fn strategy_classification_receipt_digest_v1(receipt: &StrategyClassificationReceiptV1) -> Result<StrategyClassificationReceiptDigestV1, TargetTrialCohortError> {
    validate_strategy_receipt(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_strategy(&mut writer, receipt)?;
    Ok(StrategyClassificationReceiptDigestV1(domain_digest(STRATEGY_CONTEXT, STRATEGY_TAG, &writer.finish())))
}

pub fn cohort_entry_receipt_digest_v1(receipt: &CohortEntryReceiptV1) -> Result<CohortEntryReceiptDigestV1, TargetTrialCohortError> {
    validate_cohort_entry(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_entry(&mut writer, receipt)?;
    Ok(CohortEntryReceiptDigestV1(domain_digest(ENTRY_CONTEXT, ENTRY_TAG, &writer.finish())))
}

pub fn cohort_manifest_digest_v1(manifest: &CohortManifestV1) -> Result<CohortManifestDigestV1, TargetTrialCohortError> {
    validate_manifest(manifest)?;
    let mut writer = CanonicalWriter::default();
    encode_manifest(&mut writer, manifest)?;
    Ok(CohortManifestDigestV1(domain_digest(MANIFEST_CONTEXT, MANIFEST_TAG, &writer.finish())))
}

fn validate_eligibility_against(protocol: &TargetTrialProtocolV1, plan: &TargetTrialEmulationPlanV1, receipt: &EligibilityReceiptV1) -> Result<(), TargetTrialCohortError> {
    require_plan_binding(protocol, plan, receipt.protocol_digest, receipt.emulation_plan_digest)?;
    if receipt.phenotype_definition_digest != protocol.eligibility.digest {
        return Err(TargetTrialCohortError::EligibilityDefinitionMismatch);
    }
    if receipt.eligibility_operationalization != plan.eligibility_operationalization {
        return Err(TargetTrialCohortError::EligibilityOperationalizationMismatch);
    }
    Ok(())
}

fn validate_time_zero_against(protocol: &TargetTrialProtocolV1, plan: &TargetTrialEmulationPlanV1, receipt: &TimeZeroReceiptV1) -> Result<(), TargetTrialCohortError> {
    require_plan_binding(protocol, plan, receipt.protocol_digest, receipt.emulation_plan_digest)?;
    let source = plan.data_sources.iter().find(|source| source.source_id == receipt.data_source_id)
        .ok_or_else(|| TargetTrialCohortError::UnknownDataSource(receipt.data_source_id.clone()))?;
    if receipt.data_source_identity != source.source_identity {
        return Err(TargetTrialCohortError::DataSourceIdentityMismatch);
    }
    if receipt.time_zero_operationalization != plan.time_zero_operationalization {
        return Err(TargetTrialCohortError::TimeZeroOperationalizationMismatch);
    }
    if receipt.time_zero_micros < source.period_start_micros || receipt.time_zero_micros >= source.period_end_micros {
        return Err(TargetTrialCohortError::TimeZeroOutsideDataSourcePeriod);
    }
    Ok(())
}

fn validate_strategy_against(protocol: &TargetTrialProtocolV1, plan: &TargetTrialEmulationPlanV1, time_zero: &TimeZeroReceiptV1, receipt: &StrategyClassificationReceiptV1) -> Result<(), TargetTrialCohortError> {
    require_plan_binding(protocol, plan, receipt.protocol_digest, receipt.emulation_plan_digest)?;
    if !same_subject(&time_zero.subject, &receipt.subject) {
        return Err(TargetTrialCohortError::SubjectMismatch);
    }
    if receipt.classified_at_micros != time_zero.time_zero_micros {
        return Err(TargetTrialCohortError::ClassificationTimeZeroMismatch);
    }
    if receipt.time_zero_receipt_digest != time_zero_receipt_digest_v1(time_zero)?.into_bytes() {
        return Err(TargetTrialCohortError::TimeZeroReceiptMismatch);
    }
    if !protocol.treatment_strategies.iter().any(|strategy| strategy.strategy_id == receipt.strategy_id) {
        return Err(TargetTrialCohortError::UnknownTreatmentStrategy(receipt.strategy_id.clone()));
    }
    let expected = plan.strategy_operationalizations.iter().find(|mapping| mapping.strategy_id == receipt.strategy_id)
        .ok_or_else(|| TargetTrialCohortError::UnknownStrategyOperationalization(receipt.strategy_id.clone()))?;
    if receipt.classification_rule != expected.classification_rule {
        return Err(TargetTrialCohortError::StrategyOperationalizationMismatch);
    }
    Ok(())
}

fn validate_eligibility_receipt(receipt: &EligibilityReceiptV1) -> Result<(), TargetTrialCohortError> {
    require_schema(receipt.schema_version)?;
    for digest in [receipt.protocol_digest, receipt.emulation_plan_digest, receipt.phenotype_definition_digest, receipt.phenotype_evaluation_digest] { validate_nonzero(digest)?; }
    validate_subject(&receipt.subject)?;
    validate_artifact(&receipt.eligibility_operationalization)?;
    validate_artifact(&receipt.eligibility_anchor_evidence)?;
    if receipt.evidence_fact_digests.iter().any(|digest| *digest == [0; 32]) { return Err(TargetTrialCohortError::ZeroDigest); }
    Ok(())
}

fn validate_time_zero_receipt(receipt: &TimeZeroReceiptV1) -> Result<(), TargetTrialCohortError> {
    require_schema(receipt.schema_version)?;
    validate_nonzero(receipt.protocol_digest)?;
    validate_nonzero(receipt.emulation_plan_digest)?;
    validate_subject(&receipt.subject)?;
    if receipt.data_source_id.trim().is_empty() { return Err(TargetTrialCohortError::UnknownDataSource(receipt.data_source_id.clone())); }
    validate_artifact(&receipt.data_source_identity)?;
    validate_artifact(&receipt.time_zero_operationalization)?;
    validate_artifact(&receipt.time_zero_evidence)
}

fn validate_strategy_receipt(receipt: &StrategyClassificationReceiptV1) -> Result<(), TargetTrialCohortError> {
    require_schema(receipt.schema_version)?;
    validate_nonzero(receipt.protocol_digest)?;
    validate_nonzero(receipt.emulation_plan_digest)?;
    validate_nonzero(receipt.time_zero_receipt_digest)?;
    validate_subject(&receipt.subject)?;
    if receipt.strategy_id.trim().is_empty() { return Err(TargetTrialCohortError::UnknownTreatmentStrategy(receipt.strategy_id.clone())); }
    validate_artifact(&receipt.classification_rule)?;
    validate_artifact(&receipt.classification_evidence)
}

fn validate_cohort_entry(receipt: &CohortEntryReceiptV1) -> Result<(), TargetTrialCohortError> {
    require_schema(receipt.schema_version)?;
    for digest in [receipt.protocol_digest, receipt.emulation_plan_digest, receipt.eligibility_receipt_digest, receipt.time_zero_receipt_digest, receipt.strategy_classification_receipt_digest] { validate_nonzero(digest)?; }
    validate_subject(&receipt.subject)?;
    if receipt.strategy_id.trim().is_empty() || receipt.data_source_id.trim().is_empty() { return Err(TargetTrialCohortError::InvalidCohortEntry); }
    Ok(())
}

fn validate_manifest(manifest: &CohortManifestV1) -> Result<(), TargetTrialCohortError> {
    require_schema(manifest.schema_version)?;
    validate_nonzero(manifest.protocol_digest)?;
    validate_nonzero(manifest.emulation_plan_digest)?;
    validate_artifact(&manifest.assembly_software)?;
    validate_artifact(&manifest.assembly_environment)?;
    if manifest.cohort_id.trim().is_empty() || manifest.version.trim().is_empty() || manifest.entries.is_empty() { return Err(TargetTrialCohortError::InvalidCohortManifest); }
    let mut subjects = HashSet::new();
    for entry in &manifest.entries {
        validate_subject(&entry.subject)?;
        validate_nonzero(entry.cohort_entry_digest)?;
        if entry.strategy_id.trim().is_empty() { return Err(TargetTrialCohortError::InvalidCohortEntry); }
        let key = format!("{}\u{1f}{}", entry.subject.resource_type, entry.subject.id);
        if !subjects.insert(key) { return Err(TargetTrialCohortError::DuplicateCohortSubject); }
    }
    if manifest.entries.windows(2).any(|pair| compare_entry_refs(&pair[0], &pair[1]) == Ordering::Greater) {
        return Err(TargetTrialCohortError::NonCanonicalManifestOrder);
    }
    Ok(())
}

fn require_plan_binding(protocol: &TargetTrialProtocolV1, plan: &TargetTrialEmulationPlanV1, protocol_digest: [u8; 32], emulation_digest: [u8; 32]) -> Result<(), TargetTrialCohortError> {
    if target_trial_protocol_digest_v1(protocol)?.into_bytes() != protocol_digest
        || target_trial_emulation_plan_digest_v1(protocol, plan)?.into_bytes() != emulation_digest
    { return Err(TargetTrialCohortError::ProtocolOrEmulationMismatch); }
    Ok(())
}

fn compare_entry_refs(left: &CohortEntryRefV1, right: &CohortEntryRefV1) -> Ordering {
    left.subject.resource_type.cmp(&right.subject.resource_type)
        .then(left.subject.id.cmp(&right.subject.id))
        .then(left.strategy_id.cmp(&right.strategy_id))
        .then(left.time_zero_micros.cmp(&right.time_zero_micros))
        .then(left.cohort_entry_digest.cmp(&right.cohort_entry_digest))
}

fn validate_artifact(value: &EvidenceArtifactIdentityV1) -> Result<(), TargetTrialCohortError> {
    if value.namespace.trim().is_empty() || value.artifact_id.trim().is_empty() || value.version.trim().is_empty() || value.digest == [0; 32] {
        return Err(TargetTrialCohortError::InvalidEvidenceArtifact);
    }
    Ok(())
}

fn validate_subject(subject: &SubjectRef) -> Result<(), TargetTrialCohortError> {
    if subject.resource_type.trim().is_empty() || subject.id.trim().is_empty() { return Err(TargetTrialCohortError::InvalidSubject); }
    Ok(())
}

fn same_subject(left: &SubjectRef, right: &SubjectRef) -> bool { left.resource_type == right.resource_type && left.id == right.id }
fn require_schema(version: u16) -> Result<(), TargetTrialCohortError> { if version == TARGET_TRIAL_COHORT_RECEIPT_VERSION { Ok(()) } else { Err(TargetTrialCohortError::UnsupportedSchemaVersion(version)) } }
fn validate_nonzero(value: [u8; 32]) -> Result<(), TargetTrialCohortError> { if value == [0; 32] { Err(TargetTrialCohortError::ZeroDigest) } else { Ok(()) } }

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
struct CanonicalWriter { bytes: Vec<u8> }
impl CanonicalWriter {
    fn finish(self) -> Vec<u8> { self.bytes }
    fn u16(&mut self, value: u16) { self.bytes.extend_from_slice(&value.to_be_bytes()); }
    fn u32(&mut self, value: u32) { self.bytes.extend_from_slice(&value.to_be_bytes()); }
    fn i64(&mut self, value: i64) { self.bytes.extend_from_slice(&value.to_be_bytes()); }
    fn bytes(&mut self, value: &[u8]) -> Result<(), TargetTrialCohortError> { self.u32(u32::try_from(value.len()).map_err(|_| TargetTrialCohortError::LengthOverflow)?); self.bytes.extend_from_slice(value); Ok(()) }
    fn string(&mut self, value: &str) -> Result<(), TargetTrialCohortError> { self.bytes(value.as_bytes()) }
    fn vector_len(&mut self, len: usize) -> Result<(), TargetTrialCohortError> { self.u32(u32::try_from(len).map_err(|_| TargetTrialCohortError::LengthOverflow)?); Ok(()) }
}

fn encode_subject(writer: &mut CanonicalWriter, subject: &SubjectRef) -> Result<(), TargetTrialCohortError> { writer.string(&subject.resource_type)?; writer.string(&subject.id) }
fn encode_artifact(writer: &mut CanonicalWriter, artifact: &EvidenceArtifactIdentityV1) -> Result<(), TargetTrialCohortError> { writer.string(&artifact.namespace)?; writer.string(&artifact.artifact_id)?; writer.string(&artifact.version)?; writer.bytes(&artifact.digest) }
fn encode_common(writer: &mut CanonicalWriter, schema_version: u16, protocol_digest: [u8; 32], emulation_plan_digest: [u8; 32], subject: &SubjectRef) -> Result<(), TargetTrialCohortError> { writer.u16(schema_version); writer.bytes(&protocol_digest)?; writer.bytes(&emulation_plan_digest)?; encode_subject(writer, subject) }
fn encode_eligibility(writer: &mut CanonicalWriter, receipt: &EligibilityReceiptV1) -> Result<(), TargetTrialCohortError> { encode_common(writer, receipt.schema_version, receipt.protocol_digest, receipt.emulation_plan_digest, &receipt.subject)?; writer.bytes(&receipt.phenotype_definition_digest)?; writer.bytes(&receipt.phenotype_evaluation_digest)?; writer.vector_len(receipt.evidence_fact_digests.len())?; for digest in &receipt.evidence_fact_digests { writer.bytes(digest)?; } encode_artifact(writer, &receipt.eligibility_operationalization)?; writer.i64(receipt.eligibility_anchor_micros); encode_artifact(writer, &receipt.eligibility_anchor_evidence) }
fn encode_time_zero(writer: &mut CanonicalWriter, receipt: &TimeZeroReceiptV1) -> Result<(), TargetTrialCohortError> { encode_common(writer, receipt.schema_version, receipt.protocol_digest, receipt.emulation_plan_digest, &receipt.subject)?; writer.string(&receipt.data_source_id)?; encode_artifact(writer, &receipt.data_source_identity)?; writer.i64(receipt.time_zero_micros); encode_artifact(writer, &receipt.time_zero_operationalization)?; encode_artifact(writer, &receipt.time_zero_evidence) }
fn encode_strategy(writer: &mut CanonicalWriter, receipt: &StrategyClassificationReceiptV1) -> Result<(), TargetTrialCohortError> { encode_common(writer, receipt.schema_version, receipt.protocol_digest, receipt.emulation_plan_digest, &receipt.subject)?; writer.string(&receipt.strategy_id)?; encode_artifact(writer, &receipt.classification_rule)?; encode_artifact(writer, &receipt.classification_evidence)?; writer.i64(receipt.classified_at_micros); writer.bytes(&receipt.time_zero_receipt_digest) }
fn encode_entry(writer: &mut CanonicalWriter, receipt: &CohortEntryReceiptV1) -> Result<(), TargetTrialCohortError> { encode_common(writer, receipt.schema_version, receipt.protocol_digest, receipt.emulation_plan_digest, &receipt.subject)?; writer.string(&receipt.strategy_id)?; writer.string(&receipt.data_source_id)?; writer.i64(receipt.time_zero_micros); writer.bytes(&receipt.eligibility_receipt_digest)?; writer.bytes(&receipt.time_zero_receipt_digest)?; writer.bytes(&receipt.strategy_classification_receipt_digest) }
fn encode_manifest(writer: &mut CanonicalWriter, manifest: &CohortManifestV1) -> Result<(), TargetTrialCohortError> { writer.u16(manifest.schema_version); writer.string(&manifest.cohort_id)?; writer.string(&manifest.version)?; writer.bytes(&manifest.protocol_digest)?; writer.bytes(&manifest.emulation_plan_digest)?; writer.vector_len(manifest.entries.len())?; for entry in &manifest.entries { encode_subject(writer, &entry.subject)?; writer.string(&entry.strategy_id)?; writer.i64(entry.time_zero_micros); writer.bytes(&entry.cohort_entry_digest)?; } encode_artifact(writer, &manifest.assembly_software)?; encode_artifact(writer, &manifest.assembly_environment)?; writer.i64(manifest.assembled_at_micros); Ok(()) }

#[derive(Debug, Error)]
pub enum TargetTrialCohortError {
    #[error("unsupported cohort-receipt schema version: {0}")] UnsupportedSchemaVersion(u16),
    #[error("evidence artifact identity is invalid")] InvalidEvidenceArtifact,
    #[error("subject identity is invalid")] InvalidSubject,
    #[error("digest may not be all zeroes")] ZeroDigest,
    #[error("protocol/emulation identity mismatch")] ProtocolOrEmulationMismatch,
    #[error("protocol eligibility phenotype does not match the supplied definition")] EligibilityDefinitionMismatch,
    #[error("eligibility operationalization was substituted")] EligibilityOperationalizationMismatch,
    #[error("subject is not eligible: {0:?}")] SubjectNotEligible(CriterionStateV1),
    #[error("unknown observational data source: {0}")] UnknownDataSource(String),
    #[error("data-source identity was substituted")] DataSourceIdentityMismatch,
    #[error("time-zero operationalization was substituted")] TimeZeroOperationalizationMismatch,
    #[error("time zero falls outside the selected data-source period")] TimeZeroOutsideDataSourcePeriod,
    #[error("unknown target-trial treatment strategy: {0}")] UnknownTreatmentStrategy(String),
    #[error("missing observational strategy operationalization: {0}")] UnknownStrategyOperationalization(String),
    #[error("strategy-classification operationalization was substituted")] StrategyOperationalizationMismatch,
    #[error("observational treatment classification must occur exactly at time zero in v1")] ClassificationTimeZeroMismatch,
    #[error("subject mismatch across target-trial receipts")] SubjectMismatch,
    #[error("eligibility, classification, and follow-up start are not aligned at one time zero")] TimeZeroAlignmentMismatch,
    #[error("strategy classification is not bound to the exact time-zero receipt")] TimeZeroReceiptMismatch,
    #[error("cohort entry is invalid")] InvalidCohortEntry,
    #[error("cohort manifest is invalid")] InvalidCohortManifest,
    #[error("cohort contains more than one entry for the same subject")] DuplicateCohortSubject,
    #[error("cohort manifest entries are not in canonical order")] NonCanonicalManifestOrder,
    #[error("cohort assembly timestamp predates a subject time zero")] AssemblyPredatesTimeZero,
    #[error("canonical framing length exceeds v1 limit")] LengthOverflow,
    #[error(transparent)] Protocol(#[from] mycelix_target_trial_protocol::TargetTrialError),
    #[error(transparent)] Phenotype(#[from] mycelix_clinical_phenotype::PhenotypeError),
}
