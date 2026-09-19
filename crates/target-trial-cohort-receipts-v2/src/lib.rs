#![deny(unsafe_code)]
//! Research-only target-trial subject receipts v2.
//!
//! V2 establishes observational time zero first, then anchors reusable relative
//! eligibility semantics to that exact receipt before strategy classification.
//! These artifacts never represent random assignment or a causal effect.

use mycelix_clinical_phenotype_v2::{
    evaluate_relative_phenotype_v2, phenotype_evaluation_context_digest_v2,
    relative_phenotype_definition_digest_v2, relative_phenotype_evaluation_digest_v2,
    CriterionStateV2, EvidenceArtifactIdentityV2 as PhenotypeEvidenceIdentityV2,
    PhenotypeEvaluationContextV2, RelativePhenotypeDefinitionV2,
};
use mycelix_clinical_semantics::{ClinicalFact, SubjectRef};
use mycelix_target_trial_protocol_v2::{
    target_trial_emulation_plan_digest_v2, target_trial_protocol_digest_v2,
    EvidenceArtifactIdentityV2, RelativePhenotypeRefV2, TargetTrialEmulationPlanV2,
    TargetTrialProtocolV2,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const TARGET_TRIAL_COHORT_RECEIPT_V2_VERSION: u16 = 2;
pub const TIME_ZERO_RECEIPT_NAMESPACE: &str = "mycelix/target-trial-time-zero-receipt/v2";

const TIME_ZERO_TAG: &[u8] = b"mycelix/target-trial-time-zero-receipt/v2";
const ELIGIBILITY_TAG: &[u8] = b"mycelix/target-trial-eligibility-receipt/v2";
const STRATEGY_TAG: &[u8] = b"mycelix/target-trial-strategy-classification-receipt/v2";
const ENTRY_TAG: &[u8] = b"mycelix/target-trial-cohort-entry-receipt/v2";
const MANIFEST_TAG: &[u8] = b"mycelix/target-trial-cohort-manifest/v2";

const TIME_ZERO_CONTEXT: &str = "mycelix.health.target-trial-time-zero-receipt.v2";
const ELIGIBILITY_CONTEXT: &str = "mycelix.health.target-trial-eligibility-receipt.v2";
const STRATEGY_CONTEXT: &str = "mycelix.health.target-trial-strategy-classification-receipt.v2";
const ENTRY_CONTEXT: &str = "mycelix.health.target-trial-cohort-entry-receipt.v2";
const MANIFEST_CONTEXT: &str = "mycelix.health.target-trial-cohort-manifest.v2";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeZeroReceiptV2 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub subject: SubjectRef,
    pub data_source_id: String,
    pub data_source_identity: EvidenceArtifactIdentityV2,
    pub time_zero_micros: i64,
    pub time_zero_operationalization: EvidenceArtifactIdentityV2,
    pub time_zero_evidence: EvidenceArtifactIdentityV2,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EligibilityReceiptV2 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub subject: SubjectRef,
    pub time_zero_receipt_digest: [u8; 32],
    pub phenotype_definition_digest: [u8; 32],
    pub evaluation_context_digest: [u8; 32],
    pub phenotype_evaluation_digest: [u8; 32],
    pub eligibility_operationalization: EvidenceArtifactIdentityV2,
    pub evidence_fact_digests: Vec<[u8; 32]>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategyClassificationReceiptV2 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub subject: SubjectRef,
    pub time_zero_receipt_digest: [u8; 32],
    pub strategy_id: String,
    pub classification_rule: EvidenceArtifactIdentityV2,
    pub classification_evidence: EvidenceArtifactIdentityV2,
    pub classified_at_micros: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CohortEntryReceiptV2 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub subject: SubjectRef,
    pub strategy_id: String,
    pub data_source_id: String,
    pub time_zero_micros: i64,
    pub time_zero_receipt_digest: [u8; 32],
    pub eligibility_receipt_digest: [u8; 32],
    pub strategy_classification_receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CohortEntryRefV2 {
    pub subject: SubjectRef,
    pub strategy_id: String,
    pub time_zero_micros: i64,
    pub cohort_entry_digest: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CohortManifestV2 {
    pub schema_version: u16,
    pub cohort_id: String,
    pub version: String,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub entries: Vec<CohortEntryRefV2>,
    pub assembly_software: EvidenceArtifactIdentityV2,
    pub assembly_environment: EvidenceArtifactIdentityV2,
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

digest_type!(TimeZeroReceiptDigestV2);
digest_type!(EligibilityReceiptDigestV2);
digest_type!(StrategyClassificationReceiptDigestV2);
digest_type!(CohortEntryReceiptDigestV2);
digest_type!(CohortManifestDigestV2);

pub fn build_time_zero_receipt_v2(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    subject: SubjectRef,
    data_source_id: &str,
    time_zero_micros: i64,
    time_zero_evidence: EvidenceArtifactIdentityV2,
) -> Result<TimeZeroReceiptV2, TargetTrialCohortV2Error> {
    plan.validate_against(protocol)?;
    validate_subject(&subject)?;
    validate_artifact(&time_zero_evidence)?;
    let source = plan
        .data_sources
        .iter()
        .find(|source| source.source_id == data_source_id)
        .ok_or_else(|| TargetTrialCohortV2Error::UnknownDataSource(data_source_id.to_string()))?;
    if time_zero_micros < source.period_start_micros || time_zero_micros >= source.period_end_micros {
        return Err(TargetTrialCohortV2Error::TimeZeroOutsideDataSourcePeriod);
    }
    let receipt = TimeZeroReceiptV2 {
        schema_version: TARGET_TRIAL_COHORT_RECEIPT_V2_VERSION,
        protocol_digest: target_trial_protocol_digest_v2(protocol)?.into_bytes(),
        emulation_plan_digest: target_trial_emulation_plan_digest_v2(protocol, plan)?.into_bytes(),
        subject,
        data_source_id: source.source_id.clone(),
        data_source_identity: source.source_identity.clone(),
        time_zero_micros,
        time_zero_operationalization: plan.time_zero_operationalization.clone(),
        time_zero_evidence,
    };
    verify_time_zero_receipt_v2(protocol, plan, &receipt)?;
    Ok(receipt)
}

pub fn verify_time_zero_receipt_v2(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    receipt: &TimeZeroReceiptV2,
) -> Result<(), TargetTrialCohortV2Error> {
    plan.validate_against(protocol)?;
    validate_time_zero_shape(receipt)?;
    require_plan_binding(protocol, plan, receipt.protocol_digest, receipt.emulation_plan_digest)?;
    let source = plan
        .data_sources
        .iter()
        .find(|source| source.source_id == receipt.data_source_id)
        .ok_or_else(|| TargetTrialCohortV2Error::UnknownDataSource(receipt.data_source_id.clone()))?;
    if source.source_identity != receipt.data_source_identity {
        return Err(TargetTrialCohortV2Error::DataSourceIdentityMismatch);
    }
    if plan.time_zero_operationalization != receipt.time_zero_operationalization {
        return Err(TargetTrialCohortV2Error::TimeZeroOperationalizationMismatch);
    }
    if receipt.time_zero_micros < source.period_start_micros
        || receipt.time_zero_micros >= source.period_end_micros
    {
        return Err(TargetTrialCohortV2Error::TimeZeroOutsideDataSourcePeriod);
    }
    Ok(())
}

pub fn time_zero_anchor_evidence_v2(
    receipt: &TimeZeroReceiptV2,
) -> Result<PhenotypeEvidenceIdentityV2, TargetTrialCohortV2Error> {
    validate_time_zero_shape(receipt)?;
    Ok(PhenotypeEvidenceIdentityV2 {
        namespace: TIME_ZERO_RECEIPT_NAMESPACE.to_string(),
        artifact_id: format!(
            "{}/{}@{}",
            receipt.subject.resource_type, receipt.subject.id, receipt.time_zero_micros
        ),
        version: TARGET_TRIAL_COHORT_RECEIPT_V2_VERSION.to_string(),
        digest: time_zero_receipt_digest_v2(receipt)?.into_bytes(),
    })
}

pub fn build_eligibility_receipt_v2(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    time_zero: &TimeZeroReceiptV2,
    definition: &RelativePhenotypeDefinitionV2,
    context: &PhenotypeEvaluationContextV2,
    facts: &[ClinicalFact],
) -> Result<EligibilityReceiptV2, TargetTrialCohortV2Error> {
    verify_time_zero_receipt_v2(protocol, plan, time_zero)?;
    verify_definition_ref(&protocol.eligibility, definition)?;
    if context.anchor_micros != time_zero.time_zero_micros {
        return Err(TargetTrialCohortV2Error::EligibilityAnchorTimeMismatch);
    }
    if context.anchor_evidence != time_zero_anchor_evidence_v2(time_zero)? {
        return Err(TargetTrialCohortV2Error::EligibilityAnchorEvidenceMismatch);
    }
    let evaluation = evaluate_relative_phenotype_v2(
        definition,
        context,
        &time_zero.subject.id,
        facts,
    )?;
    if evaluation.subject != time_zero.subject {
        return Err(TargetTrialCohortV2Error::SubjectMismatch);
    }
    if evaluation.state != CriterionStateV2::Satisfied {
        return Err(TargetTrialCohortV2Error::SubjectNotEligible(evaluation.state));
    }
    let receipt = EligibilityReceiptV2 {
        schema_version: TARGET_TRIAL_COHORT_RECEIPT_V2_VERSION,
        protocol_digest: time_zero.protocol_digest,
        emulation_plan_digest: time_zero.emulation_plan_digest,
        subject: time_zero.subject.clone(),
        time_zero_receipt_digest: time_zero_receipt_digest_v2(time_zero)?.into_bytes(),
        phenotype_definition_digest: relative_phenotype_definition_digest_v2(definition)?.into_bytes(),
        evaluation_context_digest: phenotype_evaluation_context_digest_v2(definition, context)?.into_bytes(),
        phenotype_evaluation_digest: relative_phenotype_evaluation_digest_v2(&evaluation)?.into_bytes(),
        eligibility_operationalization: plan.eligibility_operationalization.clone(),
        evidence_fact_digests: evaluation.evidence_fact_digests,
    };
    validate_eligibility_shape(&receipt)?;
    Ok(receipt)
}

pub fn verify_eligibility_receipt_v2(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    time_zero: &TimeZeroReceiptV2,
    definition: &RelativePhenotypeDefinitionV2,
    context: &PhenotypeEvaluationContextV2,
    facts: &[ClinicalFact],
    receipt: &EligibilityReceiptV2,
) -> Result<(), TargetTrialCohortV2Error> {
    validate_eligibility_shape(receipt)?;
    let rebuilt = build_eligibility_receipt_v2(protocol, plan, time_zero, definition, context, facts)?;
    if eligibility_receipt_digest_v2(&rebuilt)? != eligibility_receipt_digest_v2(receipt)? {
        return Err(TargetTrialCohortV2Error::EligibilityReceiptMismatch);
    }
    Ok(())
}

pub fn build_strategy_classification_receipt_v2(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    time_zero: &TimeZeroReceiptV2,
    strategy_id: &str,
    classified_at_micros: i64,
    classification_evidence: EvidenceArtifactIdentityV2,
) -> Result<StrategyClassificationReceiptV2, TargetTrialCohortV2Error> {
    verify_time_zero_receipt_v2(protocol, plan, time_zero)?;
    validate_artifact(&classification_evidence)?;
    if classified_at_micros != time_zero.time_zero_micros {
        return Err(TargetTrialCohortV2Error::ClassificationTimeZeroMismatch);
    }
    if !protocol.treatment_strategies.iter().any(|strategy| strategy.strategy_id == strategy_id) {
        return Err(TargetTrialCohortV2Error::UnknownTreatmentStrategy(strategy_id.to_string()));
    }
    let mapping = plan
        .strategy_operationalizations
        .iter()
        .find(|mapping| mapping.strategy_id == strategy_id)
        .ok_or_else(|| TargetTrialCohortV2Error::UnknownStrategyOperationalization(strategy_id.to_string()))?;
    let receipt = StrategyClassificationReceiptV2 {
        schema_version: TARGET_TRIAL_COHORT_RECEIPT_V2_VERSION,
        protocol_digest: time_zero.protocol_digest,
        emulation_plan_digest: time_zero.emulation_plan_digest,
        subject: time_zero.subject.clone(),
        time_zero_receipt_digest: time_zero_receipt_digest_v2(time_zero)?.into_bytes(),
        strategy_id: strategy_id.to_string(),
        classification_rule: mapping.classification_rule.clone(),
        classification_evidence,
        classified_at_micros,
    };
    verify_strategy_receipt_v2(protocol, plan, time_zero, &receipt)?;
    Ok(receipt)
}

pub fn verify_strategy_receipt_v2(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    time_zero: &TimeZeroReceiptV2,
    receipt: &StrategyClassificationReceiptV2,
) -> Result<(), TargetTrialCohortV2Error> {
    verify_time_zero_receipt_v2(protocol, plan, time_zero)?;
    validate_strategy_shape(receipt)?;
    require_plan_binding(protocol, plan, receipt.protocol_digest, receipt.emulation_plan_digest)?;
    if receipt.subject != time_zero.subject {
        return Err(TargetTrialCohortV2Error::SubjectMismatch);
    }
    let expected_tz = time_zero_receipt_digest_v2(time_zero)?.into_bytes();
    if receipt.time_zero_receipt_digest != expected_tz {
        return Err(TargetTrialCohortV2Error::TimeZeroReceiptMismatch);
    }
    if receipt.classified_at_micros != time_zero.time_zero_micros {
        return Err(TargetTrialCohortV2Error::ClassificationTimeZeroMismatch);
    }
    if !protocol.treatment_strategies.iter().any(|strategy| strategy.strategy_id == receipt.strategy_id) {
        return Err(TargetTrialCohortV2Error::UnknownTreatmentStrategy(receipt.strategy_id.clone()));
    }
    let mapping = plan
        .strategy_operationalizations
        .iter()
        .find(|mapping| mapping.strategy_id == receipt.strategy_id)
        .ok_or_else(|| TargetTrialCohortV2Error::UnknownStrategyOperationalization(receipt.strategy_id.clone()))?;
    if mapping.classification_rule != receipt.classification_rule {
        return Err(TargetTrialCohortV2Error::StrategyRuleMismatch);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn compose_cohort_entry_v2(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    time_zero: &TimeZeroReceiptV2,
    eligibility_definition: &RelativePhenotypeDefinitionV2,
    eligibility_context: &PhenotypeEvaluationContextV2,
    eligibility_facts: &[ClinicalFact],
    eligibility: &EligibilityReceiptV2,
    strategy: &StrategyClassificationReceiptV2,
) -> Result<CohortEntryReceiptV2, TargetTrialCohortV2Error> {
    verify_time_zero_receipt_v2(protocol, plan, time_zero)?;
    verify_eligibility_receipt_v2(
        protocol,
        plan,
        time_zero,
        eligibility_definition,
        eligibility_context,
        eligibility_facts,
        eligibility,
    )?;
    verify_strategy_receipt_v2(protocol, plan, time_zero, strategy)?;
    if eligibility.subject != time_zero.subject || strategy.subject != time_zero.subject {
        return Err(TargetTrialCohortV2Error::SubjectMismatch);
    }
    let receipt = CohortEntryReceiptV2 {
        schema_version: TARGET_TRIAL_COHORT_RECEIPT_V2_VERSION,
        protocol_digest: time_zero.protocol_digest,
        emulation_plan_digest: time_zero.emulation_plan_digest,
        subject: time_zero.subject.clone(),
        strategy_id: strategy.strategy_id.clone(),
        data_source_id: time_zero.data_source_id.clone(),
        time_zero_micros: time_zero.time_zero_micros,
        time_zero_receipt_digest: time_zero_receipt_digest_v2(time_zero)?.into_bytes(),
        eligibility_receipt_digest: eligibility_receipt_digest_v2(eligibility)?.into_bytes(),
        strategy_classification_receipt_digest: strategy_classification_receipt_digest_v2(strategy)?.into_bytes(),
    };
    validate_entry_shape(&receipt)?;
    Ok(receipt)
}

pub fn assemble_cohort_manifest_v2(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    cohort_id: &str,
    version: &str,
    entries: &[CohortEntryReceiptV2],
    assembly_software: EvidenceArtifactIdentityV2,
    assembly_environment: EvidenceArtifactIdentityV2,
    assembled_at_micros: i64,
) -> Result<CohortManifestV2, TargetTrialCohortV2Error> {
    plan.validate_against(protocol)?;
    if cohort_id.trim().is_empty() || version.trim().is_empty() || entries.is_empty() {
        return Err(TargetTrialCohortV2Error::InvalidCohortManifest);
    }
    validate_artifact(&assembly_software)?;
    validate_artifact(&assembly_environment)?;
    let protocol_digest = target_trial_protocol_digest_v2(protocol)?.into_bytes();
    let plan_digest = target_trial_emulation_plan_digest_v2(protocol, plan)?.into_bytes();
    let mut seen_subjects = HashSet::new();
    let mut refs = Vec::with_capacity(entries.len());
    for entry in entries {
        validate_entry_shape(entry)?;
        if entry.protocol_digest != protocol_digest || entry.emulation_plan_digest != plan_digest {
            return Err(TargetTrialCohortV2Error::ProtocolOrEmulationMismatch);
        }
        if assembled_at_micros < entry.time_zero_micros {
            return Err(TargetTrialCohortV2Error::AssemblyPredatesTimeZero);
        }
        let key = format!("{}\u{1f}{}", entry.subject.resource_type, entry.subject.id);
        if !seen_subjects.insert(key) {
            return Err(TargetTrialCohortV2Error::DuplicateCohortSubject);
        }
        refs.push(CohortEntryRefV2 {
            subject: entry.subject.clone(),
            strategy_id: entry.strategy_id.clone(),
            time_zero_micros: entry.time_zero_micros,
            cohort_entry_digest: cohort_entry_receipt_digest_v2(entry)?.into_bytes(),
        });
    }
    refs.sort_by(|left, right| {
        left.subject.resource_type.cmp(&right.subject.resource_type)
            .then(left.subject.id.cmp(&right.subject.id))
            .then(left.strategy_id.cmp(&right.strategy_id))
            .then(left.time_zero_micros.cmp(&right.time_zero_micros))
    });
    let manifest = CohortManifestV2 {
        schema_version: TARGET_TRIAL_COHORT_RECEIPT_V2_VERSION,
        cohort_id: cohort_id.to_string(),
        version: version.to_string(),
        protocol_digest,
        emulation_plan_digest: plan_digest,
        entries: refs,
        assembly_software,
        assembly_environment,
        assembled_at_micros,
    };
    validate_manifest_shape(&manifest)?;
    Ok(manifest)
}

pub fn time_zero_receipt_digest_v2(receipt: &TimeZeroReceiptV2) -> Result<TimeZeroReceiptDigestV2, TargetTrialCohortV2Error> {
    validate_time_zero_shape(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_time_zero(&mut writer, receipt)?;
    Ok(TimeZeroReceiptDigestV2(domain_digest(TIME_ZERO_CONTEXT, TIME_ZERO_TAG, &writer.finish())))
}

pub fn eligibility_receipt_digest_v2(receipt: &EligibilityReceiptV2) -> Result<EligibilityReceiptDigestV2, TargetTrialCohortV2Error> {
    validate_eligibility_shape(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_eligibility(&mut writer, receipt)?;
    Ok(EligibilityReceiptDigestV2(domain_digest(ELIGIBILITY_CONTEXT, ELIGIBILITY_TAG, &writer.finish())))
}

pub fn strategy_classification_receipt_digest_v2(receipt: &StrategyClassificationReceiptV2) -> Result<StrategyClassificationReceiptDigestV2, TargetTrialCohortV2Error> {
    validate_strategy_shape(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_strategy(&mut writer, receipt)?;
    Ok(StrategyClassificationReceiptDigestV2(domain_digest(STRATEGY_CONTEXT, STRATEGY_TAG, &writer.finish())))
}

pub fn cohort_entry_receipt_digest_v2(receipt: &CohortEntryReceiptV2) -> Result<CohortEntryReceiptDigestV2, TargetTrialCohortV2Error> {
    validate_entry_shape(receipt)?;
    let mut writer = CanonicalWriter::default();
    encode_entry(&mut writer, receipt)?;
    Ok(CohortEntryReceiptDigestV2(domain_digest(ENTRY_CONTEXT, ENTRY_TAG, &writer.finish())))
}

pub fn cohort_manifest_digest_v2(manifest: &CohortManifestV2) -> Result<CohortManifestDigestV2, TargetTrialCohortV2Error> {
    validate_manifest_shape(manifest)?;
    let mut writer = CanonicalWriter::default();
    encode_manifest(&mut writer, manifest)?;
    Ok(CohortManifestDigestV2(domain_digest(MANIFEST_CONTEXT, MANIFEST_TAG, &writer.finish())))
}

fn verify_definition_ref(reference: &RelativePhenotypeRefV2, definition: &RelativePhenotypeDefinitionV2) -> Result<(), TargetTrialCohortV2Error> {
    let digest = relative_phenotype_definition_digest_v2(definition)?.into_bytes();
    if reference.phenotype_id != definition.phenotype_id
        || reference.version != definition.version
        || reference.digest != digest
    {
        return Err(TargetTrialCohortV2Error::EligibilityDefinitionMismatch);
    }
    Ok(())
}

fn require_plan_binding(protocol: &TargetTrialProtocolV2, plan: &TargetTrialEmulationPlanV2, protocol_digest: [u8; 32], plan_digest: [u8; 32]) -> Result<(), TargetTrialCohortV2Error> {
    if protocol_digest != target_trial_protocol_digest_v2(protocol)?.into_bytes()
        || plan_digest != target_trial_emulation_plan_digest_v2(protocol, plan)?.into_bytes()
    {
        return Err(TargetTrialCohortV2Error::ProtocolOrEmulationMismatch);
    }
    Ok(())
}

fn validate_subject(subject: &SubjectRef) -> Result<(), TargetTrialCohortV2Error> {
    if subject.resource_type.trim().is_empty() || subject.id.trim().is_empty() {
        return Err(TargetTrialCohortV2Error::InvalidSubject);
    }
    Ok(())
}

fn validate_artifact(value: &EvidenceArtifactIdentityV2) -> Result<(), TargetTrialCohortV2Error> {
    if value.namespace.trim().is_empty() || value.artifact_id.trim().is_empty() || value.version.trim().is_empty() {
        return Err(TargetTrialCohortV2Error::IncompleteEvidenceIdentity);
    }
    if value.digest == [0; 32] { return Err(TargetTrialCohortV2Error::ZeroEvidenceDigest); }
    Ok(())
}

fn validate_time_zero_shape(value: &TimeZeroReceiptV2) -> Result<(), TargetTrialCohortV2Error> {
    if value.schema_version != TARGET_TRIAL_COHORT_RECEIPT_V2_VERSION { return Err(TargetTrialCohortV2Error::UnsupportedSchemaVersion(value.schema_version)); }
    validate_subject(&value.subject)?;
    if value.protocol_digest == [0; 32] || value.emulation_plan_digest == [0; 32] || value.data_source_id.trim().is_empty() { return Err(TargetTrialCohortV2Error::InvalidTimeZeroReceipt); }
    validate_artifact(&value.data_source_identity)?;
    validate_artifact(&value.time_zero_operationalization)?;
    validate_artifact(&value.time_zero_evidence)
}

fn validate_eligibility_shape(value: &EligibilityReceiptV2) -> Result<(), TargetTrialCohortV2Error> {
    if value.schema_version != TARGET_TRIAL_COHORT_RECEIPT_V2_VERSION { return Err(TargetTrialCohortV2Error::UnsupportedSchemaVersion(value.schema_version)); }
    validate_subject(&value.subject)?;
    if [value.protocol_digest, value.emulation_plan_digest, value.time_zero_receipt_digest, value.phenotype_definition_digest, value.evaluation_context_digest, value.phenotype_evaluation_digest].iter().any(|digest| *digest == [0; 32]) { return Err(TargetTrialCohortV2Error::InvalidEligibilityReceipt); }
    validate_artifact(&value.eligibility_operationalization)
}

fn validate_strategy_shape(value: &StrategyClassificationReceiptV2) -> Result<(), TargetTrialCohortV2Error> {
    if value.schema_version != TARGET_TRIAL_COHORT_RECEIPT_V2_VERSION { return Err(TargetTrialCohortV2Error::UnsupportedSchemaVersion(value.schema_version)); }
    validate_subject(&value.subject)?;
    if value.protocol_digest == [0; 32] || value.emulation_plan_digest == [0; 32] || value.time_zero_receipt_digest == [0; 32] || value.strategy_id.trim().is_empty() { return Err(TargetTrialCohortV2Error::InvalidStrategyReceipt); }
    validate_artifact(&value.classification_rule)?;
    validate_artifact(&value.classification_evidence)
}

fn validate_entry_shape(value: &CohortEntryReceiptV2) -> Result<(), TargetTrialCohortV2Error> {
    if value.schema_version != TARGET_TRIAL_COHORT_RECEIPT_V2_VERSION { return Err(TargetTrialCohortV2Error::UnsupportedSchemaVersion(value.schema_version)); }
    validate_subject(&value.subject)?;
    if value.protocol_digest == [0; 32] || value.emulation_plan_digest == [0; 32] || value.time_zero_receipt_digest == [0; 32] || value.eligibility_receipt_digest == [0; 32] || value.strategy_classification_receipt_digest == [0; 32] || value.strategy_id.trim().is_empty() || value.data_source_id.trim().is_empty() { return Err(TargetTrialCohortV2Error::InvalidCohortEntry); }
    Ok(())
}

fn validate_manifest_shape(value: &CohortManifestV2) -> Result<(), TargetTrialCohortV2Error> {
    if value.schema_version != TARGET_TRIAL_COHORT_RECEIPT_V2_VERSION || value.cohort_id.trim().is_empty() || value.version.trim().is_empty() || value.entries.is_empty() || value.protocol_digest == [0; 32] || value.emulation_plan_digest == [0; 32] { return Err(TargetTrialCohortV2Error::InvalidCohortManifest); }
    validate_artifact(&value.assembly_software)?;
    validate_artifact(&value.assembly_environment)?;
    let mut previous: Option<(&str, &str, &str, i64)> = None;
    let mut seen = HashSet::new();
    for entry in &value.entries {
        validate_subject(&entry.subject)?;
        if entry.strategy_id.trim().is_empty() || entry.cohort_entry_digest == [0; 32] { return Err(TargetTrialCohortV2Error::InvalidCohortManifest); }
        let key = format!("{}\u{1f}{}", entry.subject.resource_type, entry.subject.id);
        if !seen.insert(key) { return Err(TargetTrialCohortV2Error::DuplicateCohortSubject); }
        let current = (entry.subject.resource_type.as_str(), entry.subject.id.as_str(), entry.strategy_id.as_str(), entry.time_zero_micros);
        if let Some(last) = previous { if last > current { return Err(TargetTrialCohortV2Error::NonCanonicalManifestOrder); } }
        previous = Some(current);
    }
    Ok(())
}

fn domain_digest(context: &str, tag: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&TARGET_TRIAL_COHORT_RECEIPT_V2_VERSION.to_be_bytes());
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
    fn string(&mut self, value: &str) -> Result<(), TargetTrialCohortV2Error> { self.bytes(value.as_bytes()) }
    fn bytes(&mut self, value: &[u8]) -> Result<(), TargetTrialCohortV2Error> { self.u32(u32::try_from(value.len()).map_err(|_| TargetTrialCohortV2Error::LengthOverflow)?); self.bytes.extend_from_slice(value); Ok(()) }
    fn vector_len(&mut self, len: usize) -> Result<(), TargetTrialCohortV2Error> { self.u32(u32::try_from(len).map_err(|_| TargetTrialCohortV2Error::LengthOverflow)?); Ok(()) }
}

fn encode_subject(writer: &mut CanonicalWriter, value: &SubjectRef) -> Result<(), TargetTrialCohortV2Error> { writer.string(&value.resource_type)?; writer.string(&value.id) }
fn encode_artifact(writer: &mut CanonicalWriter, value: &EvidenceArtifactIdentityV2) -> Result<(), TargetTrialCohortV2Error> { writer.string(&value.namespace)?; writer.string(&value.artifact_id)?; writer.string(&value.version)?; writer.bytes(&value.digest) }
fn encode_time_zero(writer: &mut CanonicalWriter, value: &TimeZeroReceiptV2) -> Result<(), TargetTrialCohortV2Error> { writer.u16(value.schema_version); writer.bytes(&value.protocol_digest)?; writer.bytes(&value.emulation_plan_digest)?; encode_subject(writer, &value.subject)?; writer.string(&value.data_source_id)?; encode_artifact(writer, &value.data_source_identity)?; writer.i64(value.time_zero_micros); encode_artifact(writer, &value.time_zero_operationalization)?; encode_artifact(writer, &value.time_zero_evidence) }
fn encode_eligibility(writer: &mut CanonicalWriter, value: &EligibilityReceiptV2) -> Result<(), TargetTrialCohortV2Error> { writer.u16(value.schema_version); writer.bytes(&value.protocol_digest)?; writer.bytes(&value.emulation_plan_digest)?; encode_subject(writer, &value.subject)?; writer.bytes(&value.time_zero_receipt_digest)?; writer.bytes(&value.phenotype_definition_digest)?; writer.bytes(&value.evaluation_context_digest)?; writer.bytes(&value.phenotype_evaluation_digest)?; encode_artifact(writer, &value.eligibility_operationalization)?; writer.vector_len(value.evidence_fact_digests.len())?; for digest in &value.evidence_fact_digests { writer.bytes(digest)?; } Ok(()) }
fn encode_strategy(writer: &mut CanonicalWriter, value: &StrategyClassificationReceiptV2) -> Result<(), TargetTrialCohortV2Error> { writer.u16(value.schema_version); writer.bytes(&value.protocol_digest)?; writer.bytes(&value.emulation_plan_digest)?; encode_subject(writer, &value.subject)?; writer.bytes(&value.time_zero_receipt_digest)?; writer.string(&value.strategy_id)?; encode_artifact(writer, &value.classification_rule)?; encode_artifact(writer, &value.classification_evidence)?; writer.i64(value.classified_at_micros); Ok(()) }
fn encode_entry(writer: &mut CanonicalWriter, value: &CohortEntryReceiptV2) -> Result<(), TargetTrialCohortV2Error> { writer.u16(value.schema_version); writer.bytes(&value.protocol_digest)?; writer.bytes(&value.emulation_plan_digest)?; encode_subject(writer, &value.subject)?; writer.string(&value.strategy_id)?; writer.string(&value.data_source_id)?; writer.i64(value.time_zero_micros); writer.bytes(&value.time_zero_receipt_digest)?; writer.bytes(&value.eligibility_receipt_digest)?; writer.bytes(&value.strategy_classification_receipt_digest)?; Ok(()) }
fn encode_manifest(writer: &mut CanonicalWriter, value: &CohortManifestV2) -> Result<(), TargetTrialCohortV2Error> { writer.u16(value.schema_version); writer.string(&value.cohort_id)?; writer.string(&value.version)?; writer.bytes(&value.protocol_digest)?; writer.bytes(&value.emulation_plan_digest)?; writer.vector_len(value.entries.len())?; for entry in &value.entries { encode_subject(writer, &entry.subject)?; writer.string(&entry.strategy_id)?; writer.i64(entry.time_zero_micros); writer.bytes(&entry.cohort_entry_digest)?; } encode_artifact(writer, &value.assembly_software)?; encode_artifact(writer, &value.assembly_environment)?; writer.i64(value.assembled_at_micros); Ok(()) }

#[derive(Debug, Error)]
pub enum TargetTrialCohortV2Error {
    #[error("unsupported target-trial cohort receipt v2 schema version: {0}")]
    UnsupportedSchemaVersion(u16),
    #[error("subject identity is invalid")]
    InvalidSubject,
    #[error("evidence artifact identity is incomplete")]
    IncompleteEvidenceIdentity,
    #[error("evidence digest may not be all zeroes")]
    ZeroEvidenceDigest,
    #[error("unknown observational data source: {0}")]
    UnknownDataSource(String),
    #[error("time zero lies outside the selected data-source period")]
    TimeZeroOutsideDataSourcePeriod,
    #[error("data-source identity does not match the live emulation plan")]
    DataSourceIdentityMismatch,
    #[error("time-zero operationalization does not match the live emulation plan")]
    TimeZeroOperationalizationMismatch,
    #[error("time-zero receipt is invalid")]
    InvalidTimeZeroReceipt,
    #[error("protocol or emulation identity mismatch")]
    ProtocolOrEmulationMismatch,
    #[error("eligibility definition does not match the protocol v2 phenotype identity")]
    EligibilityDefinitionMismatch,
    #[error("eligibility evaluation is anchored to a different timestamp")]
    EligibilityAnchorTimeMismatch,
    #[error("eligibility evaluation does not bind the exact time-zero receipt as anchor evidence")]
    EligibilityAnchorEvidenceMismatch,
    #[error("subject is not eligible: {0:?}")]
    SubjectNotEligible(CriterionStateV2),
    #[error("eligibility receipt is invalid")]
    InvalidEligibilityReceipt,
    #[error("eligibility receipt does not reproduce from the supplied evidence")]
    EligibilityReceiptMismatch,
    #[error("subject identity mismatch")]
    SubjectMismatch,
    #[error("unknown treatment strategy: {0}")]
    UnknownTreatmentStrategy(String),
    #[error("unknown strategy operationalization: {0}")]
    UnknownStrategyOperationalization(String),
    #[error("observational strategy classification is not aligned to time zero")]
    ClassificationTimeZeroMismatch,
    #[error("strategy classification rule does not match the live emulation plan")]
    StrategyRuleMismatch,
    #[error("strategy receipt is invalid")]
    InvalidStrategyReceipt,
    #[error("strategy receipt binds a different time-zero receipt")]
    TimeZeroReceiptMismatch,
    #[error("cohort entry is invalid")]
    InvalidCohortEntry,
    #[error("cohort manifest is invalid")]
    InvalidCohortManifest,
    #[error("duplicate cohort subject is forbidden in simple v2")]
    DuplicateCohortSubject,
    #[error("cohort manifest entries are not in canonical order")]
    NonCanonicalManifestOrder,
    #[error("cohort assembly time predates a participant time zero")]
    AssemblyPredatesTimeZero,
    #[error("canonical framing length exceeds v2 limit")]
    LengthOverflow,
    #[error(transparent)]
    Protocol(#[from] mycelix_target_trial_protocol_v2::TargetTrialV2Error),
    #[error(transparent)]
    Phenotype(#[from] mycelix_clinical_phenotype_v2::PhenotypeV2Error),
}
