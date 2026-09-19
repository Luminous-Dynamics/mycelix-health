#![deny(unsafe_code)]
//! Reusable anchored/relative longitudinal phenotype definitions for Mycelix Health.
//!
//! V2 separates reusable phenotype semantics from subject/window-specific coverage
//! evidence. Definitions contain relative offsets and required coverage domains;
//! evaluation contexts contain the anchor and concrete completeness evidence.

use mycelix_clinical_fact_snapshot::clinical_fact_snapshot_digest_v1;
use mycelix_clinical_semantics::{ClinicalFact, ClinicalSemanticsError, Coding, SubjectRef};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use thiserror::Error;

pub const PHENOTYPE_V2_VERSION: u16 = 2;
const MAX_CRITERIA: usize = 1024;
const MAX_DEPTH: usize = 32;
const DEFINITION_TAG: &[u8] = b"mycelix/clinical-phenotype-definition/v2";
const CONTEXT_TAG: &[u8] = b"mycelix/clinical-phenotype-evaluation-context/v2";
const EVALUATION_TAG: &[u8] = b"mycelix/clinical-phenotype-evaluation/v2";
const DEFINITION_CONTEXT: &str = "mycelix.health.clinical-phenotype-definition.v2";
const CONTEXT_CONTEXT: &str = "mycelix.health.clinical-phenotype-evaluation-context.v2";
const EVALUATION_CONTEXT: &str = "mycelix.health.clinical-phenotype-evaluation.v2";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EvidenceArtifactIdentityV2 {
    pub namespace: String,
    pub artifact_id: String,
    pub version: String,
    pub digest: [u8; 32],
}

impl EvidenceArtifactIdentityV2 {
    fn validate(&self) -> Result<(), PhenotypeV2Error> {
        if self.namespace.trim().is_empty()
            || self.artifact_id.trim().is_empty()
            || self.version.trim().is_empty()
        {
            return Err(PhenotypeV2Error::IncompleteEvidenceIdentity);
        }
        if self.digest == [0; 32] {
            return Err(PhenotypeV2Error::ZeroEvidenceDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConceptIdentityV2 {
    pub system: String,
    pub code: String,
    pub version: String,
}

impl ConceptIdentityV2 {
    fn validate(&self) -> Result<(), PhenotypeV2Error> {
        if self.system.trim().is_empty() || self.code.trim().is_empty() {
            return Err(PhenotypeV2Error::IncompleteConceptIdentity);
        }
        if self.version.trim().is_empty() {
            return Err(PhenotypeV2Error::MissingTerminologyVersion);
        }
        Ok(())
    }

    fn matches(&self, coding: &Coding) -> bool {
        coding.system == self.system
            && coding.code == self.code
            && coding.version.as_deref() == Some(self.version.as_str())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoverageStatusV2 {
    Complete,
    Incomplete,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverageEvidenceV2 {
    pub domain: String,
    pub status: CoverageStatusV2,
    pub source: EvidenceArtifactIdentityV2,
}

impl CoverageEvidenceV2 {
    fn validate(&self) -> Result<(), PhenotypeV2Error> {
        if self.domain.trim().is_empty() {
            return Err(PhenotypeV2Error::MissingCoverageDomain);
        }
        self.source.validate()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelativeObservationWindowV2 {
    pub start_offset_micros: i64,
    pub end_offset_micros: i64,
    pub required_coverage_domains: Vec<String>,
}

impl RelativeObservationWindowV2 {
    fn validate(&self) -> Result<(), PhenotypeV2Error> {
        if self.start_offset_micros >= self.end_offset_micros {
            return Err(PhenotypeV2Error::InvalidRelativeWindow);
        }
        let mut seen = HashSet::new();
        for domain in &self.required_coverage_domains {
            if domain.trim().is_empty() {
                return Err(PhenotypeV2Error::MissingCoverageDomain);
            }
            if !seen.insert(domain.clone()) {
                return Err(PhenotypeV2Error::DuplicateCoverageDomain(domain.clone()));
            }
        }
        Ok(())
    }

    fn declares(&self, domain: &str) -> bool {
        self.required_coverage_domains.iter().any(|item| item == domain)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelativeCriterionV2 {
    FactPresent {
        criterion_id: String,
        concept: ConceptIdentityV2,
        minimum_count: u32,
        coverage_domain: String,
    },
    FactAbsent {
        criterion_id: String,
        concept: ConceptIdentityV2,
        coverage_domain: String,
    },
    All {
        criterion_id: String,
        children: Vec<RelativeCriterionV2>,
    },
    Any {
        criterion_id: String,
        children: Vec<RelativeCriterionV2>,
    },
}

impl RelativeCriterionV2 {
    fn id(&self) -> &str {
        match self {
            Self::FactPresent { criterion_id, .. }
            | Self::FactAbsent { criterion_id, .. }
            | Self::All { criterion_id, .. }
            | Self::Any { criterion_id, .. } => criterion_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelativePhenotypeDefinitionV2 {
    pub schema_version: u16,
    pub phenotype_id: String,
    pub version: String,
    pub subject_resource_type: String,
    pub window: RelativeObservationWindowV2,
    pub criterion: RelativeCriterionV2,
    pub definition_evidence: EvidenceArtifactIdentityV2,
}

impl RelativePhenotypeDefinitionV2 {
    pub fn validate(&self) -> Result<(), PhenotypeV2Error> {
        if self.schema_version != PHENOTYPE_V2_VERSION {
            return Err(PhenotypeV2Error::UnsupportedSchemaVersion(self.schema_version));
        }
        if self.phenotype_id.trim().is_empty()
            || self.version.trim().is_empty()
            || self.subject_resource_type.trim().is_empty()
        {
            return Err(PhenotypeV2Error::IncompleteDefinitionIdentity);
        }
        self.window.validate()?;
        self.definition_evidence.validate()?;
        let mut ids = HashSet::new();
        let mut count = 0usize;
        validate_criterion(&self.criterion, &self.window, 0, &mut count, &mut ids)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhenotypeEvaluationContextV2 {
    pub schema_version: u16,
    pub anchor_micros: i64,
    pub anchor_evidence: EvidenceArtifactIdentityV2,
    pub coverage: Vec<CoverageEvidenceV2>,
    pub context_evidence: Vec<EvidenceArtifactIdentityV2>,
}

impl PhenotypeEvaluationContextV2 {
    pub fn validate_against(
        &self,
        definition: &RelativePhenotypeDefinitionV2,
    ) -> Result<(), PhenotypeV2Error> {
        definition.validate()?;
        if self.schema_version != PHENOTYPE_V2_VERSION {
            return Err(PhenotypeV2Error::UnsupportedSchemaVersion(self.schema_version));
        }
        self.anchor_evidence.validate()?;
        for evidence in &self.context_evidence {
            evidence.validate()?;
        }
        let required: BTreeSet<_> = definition
            .window
            .required_coverage_domains
            .iter()
            .cloned()
            .collect();
        let mut observed = BTreeSet::new();
        for item in &self.coverage {
            item.validate()?;
            if !observed.insert(item.domain.clone()) {
                return Err(PhenotypeV2Error::DuplicateCoverageDomain(item.domain.clone()));
            }
        }
        if observed != required {
            return Err(PhenotypeV2Error::CoverageDomainSetMismatch);
        }
        Ok(())
    }

    fn coverage_for(&self, domain: &str) -> Option<&CoverageEvidenceV2> {
        self.coverage.iter().find(|item| item.domain == domain)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CriterionStateV2 {
    Satisfied,
    NotSatisfied,
    Indeterminate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CriterionEvaluationV2 {
    pub criterion_id: String,
    pub state: CriterionStateV2,
    pub evidence_fact_digests: Vec<[u8; 32]>,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelativePhenotypeEvaluationV2 {
    pub schema_version: u16,
    pub phenotype_id: String,
    pub phenotype_version: String,
    pub definition_digest: [u8; 32],
    pub evaluation_context_digest: [u8; 32],
    pub subject: SubjectRef,
    pub anchor_micros: i64,
    pub window_start_micros: i64,
    pub window_end_micros: i64,
    pub state: CriterionStateV2,
    pub criterion_evaluations: Vec<CriterionEvaluationV2>,
    pub evidence_fact_digests: Vec<[u8; 32]>,
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

digest_type!(RelativePhenotypeDefinitionDigestV2);
digest_type!(PhenotypeEvaluationContextDigestV2);
digest_type!(RelativePhenotypeEvaluationDigestV2);

pub fn relative_phenotype_definition_digest_v2(
    definition: &RelativePhenotypeDefinitionV2,
) -> Result<RelativePhenotypeDefinitionDigestV2, PhenotypeV2Error> {
    definition.validate()?;
    let mut writer = CanonicalWriter::default();
    encode_definition(&mut writer, definition)?;
    Ok(RelativePhenotypeDefinitionDigestV2(domain_digest(
        DEFINITION_CONTEXT,
        DEFINITION_TAG,
        &writer.finish(),
    )))
}

pub fn phenotype_evaluation_context_digest_v2(
    definition: &RelativePhenotypeDefinitionV2,
    context: &PhenotypeEvaluationContextV2,
) -> Result<PhenotypeEvaluationContextDigestV2, PhenotypeV2Error> {
    context.validate_against(definition)?;
    let mut writer = CanonicalWriter::default();
    writer.bytes(&relative_phenotype_definition_digest_v2(definition)?.into_bytes())?;
    encode_context(&mut writer, context)?;
    Ok(PhenotypeEvaluationContextDigestV2(domain_digest(
        CONTEXT_CONTEXT,
        CONTEXT_TAG,
        &writer.finish(),
    )))
}

pub fn evaluate_relative_phenotype_v2(
    definition: &RelativePhenotypeDefinitionV2,
    context: &PhenotypeEvaluationContextV2,
    subject_id: &str,
    facts: &[ClinicalFact],
) -> Result<RelativePhenotypeEvaluationV2, PhenotypeV2Error> {
    context.validate_against(definition)?;
    if subject_id.trim().is_empty() {
        return Err(PhenotypeV2Error::InvalidSubject);
    }
    let window_start_micros = context
        .anchor_micros
        .checked_add(definition.window.start_offset_micros)
        .ok_or(PhenotypeV2Error::TimeOverflow)?;
    let window_end_micros = context
        .anchor_micros
        .checked_add(definition.window.end_offset_micros)
        .ok_or(PhenotypeV2Error::TimeOverflow)?;
    if window_start_micros >= window_end_micros {
        return Err(PhenotypeV2Error::InvalidDerivedWindow);
    }

    let mut prepared = Vec::new();
    let mut seen_digests = HashSet::new();
    for fact in facts {
        fact.validate_machine_actionable()?;
        if fact.subject.resource_type != definition.subject_resource_type || fact.subject.id != subject_id {
            return Err(PhenotypeV2Error::MixedSubjectFacts {
                expected: format!("{}/{}", definition.subject_resource_type, subject_id),
                found: format!("{}/{}", fact.subject.resource_type, fact.subject.id),
            });
        }
        let digest = clinical_fact_snapshot_digest_v1(fact)?.into_bytes();
        if !seen_digests.insert(digest) {
            return Err(PhenotypeV2Error::DuplicateFactIdentity);
        }
        if fact.effective_at_micros >= window_start_micros
            && fact.effective_at_micros < window_end_micros
        {
            prepared.push(PreparedFact { fact, digest });
        }
    }

    let mut evaluations = Vec::new();
    let (state, evidence) = evaluate_criterion(
        &definition.criterion,
        context,
        &prepared,
        &mut evaluations,
    )?;
    let definition_digest = relative_phenotype_definition_digest_v2(definition)?.into_bytes();
    let context_digest = phenotype_evaluation_context_digest_v2(definition, context)?.into_bytes();

    Ok(RelativePhenotypeEvaluationV2 {
        schema_version: PHENOTYPE_V2_VERSION,
        phenotype_id: definition.phenotype_id.clone(),
        phenotype_version: definition.version.clone(),
        definition_digest,
        evaluation_context_digest: context_digest,
        subject: SubjectRef {
            resource_type: definition.subject_resource_type.clone(),
            id: subject_id.to_string(),
        },
        anchor_micros: context.anchor_micros,
        window_start_micros,
        window_end_micros,
        state,
        criterion_evaluations: evaluations,
        evidence_fact_digests: sorted_unique(evidence),
    })
}

pub fn relative_phenotype_evaluation_digest_v2(
    evaluation: &RelativePhenotypeEvaluationV2,
) -> Result<RelativePhenotypeEvaluationDigestV2, PhenotypeV2Error> {
    validate_evaluation(evaluation)?;
    let mut writer = CanonicalWriter::default();
    encode_evaluation(&mut writer, evaluation)?;
    Ok(RelativePhenotypeEvaluationDigestV2(domain_digest(
        EVALUATION_CONTEXT,
        EVALUATION_TAG,
        &writer.finish(),
    )))
}

fn validate_criterion(
    criterion: &RelativeCriterionV2,
    window: &RelativeObservationWindowV2,
    depth: usize,
    count: &mut usize,
    ids: &mut HashSet<String>,
) -> Result<(), PhenotypeV2Error> {
    if depth > MAX_DEPTH {
        return Err(PhenotypeV2Error::CriterionDepthExceeded);
    }
    *count += 1;
    if *count > MAX_CRITERIA {
        return Err(PhenotypeV2Error::TooManyCriteria);
    }
    if criterion.id().trim().is_empty() {
        return Err(PhenotypeV2Error::MissingCriterionId);
    }
    if !ids.insert(criterion.id().to_string()) {
        return Err(PhenotypeV2Error::DuplicateCriterionId(criterion.id().to_string()));
    }
    match criterion {
        RelativeCriterionV2::FactPresent {
            concept,
            minimum_count,
            coverage_domain,
            ..
        } => {
            concept.validate()?;
            if *minimum_count == 0 {
                return Err(PhenotypeV2Error::ZeroMinimumCount);
            }
            require_declared_domain(window, coverage_domain)?;
        }
        RelativeCriterionV2::FactAbsent {
            concept,
            coverage_domain,
            ..
        } => {
            concept.validate()?;
            require_declared_domain(window, coverage_domain)?;
        }
        RelativeCriterionV2::All { children, .. } | RelativeCriterionV2::Any { children, .. } => {
            if children.is_empty() {
                return Err(PhenotypeV2Error::EmptyCriterionGroup);
            }
            for child in children {
                validate_criterion(child, window, depth + 1, count, ids)?;
            }
        }
    }
    Ok(())
}

fn require_declared_domain(
    window: &RelativeObservationWindowV2,
    domain: &str,
) -> Result<(), PhenotypeV2Error> {
    if domain.trim().is_empty() {
        return Err(PhenotypeV2Error::MissingCoverageDomain);
    }
    if !window.declares(domain) {
        return Err(PhenotypeV2Error::UnknownCoverageDomain(domain.to_string()));
    }
    Ok(())
}

struct PreparedFact<'a> {
    fact: &'a ClinicalFact,
    digest: [u8; 32],
}

fn evaluate_criterion(
    criterion: &RelativeCriterionV2,
    context: &PhenotypeEvaluationContextV2,
    facts: &[PreparedFact<'_>],
    output: &mut Vec<CriterionEvaluationV2>,
) -> Result<(CriterionStateV2, Vec<[u8; 32]>), PhenotypeV2Error> {
    match criterion {
        RelativeCriterionV2::FactPresent {
            criterion_id,
            concept,
            minimum_count,
            coverage_domain,
        } => {
            let matches = matching_fact_digests(facts, concept);
            let coverage = context
                .coverage_for(coverage_domain)
                .ok_or_else(|| PhenotypeV2Error::UnknownCoverageDomain(coverage_domain.clone()))?;
            let state = if matches.len() >= *minimum_count as usize {
                CriterionStateV2::Satisfied
            } else if coverage.status == CoverageStatusV2::Complete {
                CriterionStateV2::NotSatisfied
            } else {
                CriterionStateV2::Indeterminate
            };
            output.push(CriterionEvaluationV2 {
                criterion_id: criterion_id.clone(),
                state,
                evidence_fact_digests: sorted_unique(matches.clone()),
                reason: match state {
                    CriterionStateV2::Satisfied => "minimum positive evidence count observed".to_string(),
                    CriterionStateV2::NotSatisfied => "complete coverage with insufficient matching evidence".to_string(),
                    CriterionStateV2::Indeterminate => "insufficient matching evidence under incomplete/unknown coverage".to_string(),
                },
            });
            Ok((state, matches))
        }
        RelativeCriterionV2::FactAbsent {
            criterion_id,
            concept,
            coverage_domain,
        } => {
            let matches = matching_fact_digests(facts, concept);
            let coverage = context
                .coverage_for(coverage_domain)
                .ok_or_else(|| PhenotypeV2Error::UnknownCoverageDomain(coverage_domain.clone()))?;
            let state = if !matches.is_empty() {
                CriterionStateV2::NotSatisfied
            } else if coverage.status == CoverageStatusV2::Complete {
                CriterionStateV2::Satisfied
            } else {
                CriterionStateV2::Indeterminate
            };
            output.push(CriterionEvaluationV2 {
                criterion_id: criterion_id.clone(),
                state,
                evidence_fact_digests: sorted_unique(matches.clone()),
                reason: match state {
                    CriterionStateV2::Satisfied => "complete coverage with no matching fact observed".to_string(),
                    CriterionStateV2::NotSatisfied => "contradictory matching fact observed".to_string(),
                    CriterionStateV2::Indeterminate => "absence cannot be established under incomplete/unknown coverage".to_string(),
                },
            });
            Ok((state, matches))
        }
        RelativeCriterionV2::All { criterion_id, children } => {
            let mut states = Vec::with_capacity(children.len());
            let mut evidence = Vec::new();
            for child in children {
                let (state, child_evidence) = evaluate_criterion(child, context, facts, output)?;
                states.push(state);
                evidence.extend(child_evidence);
            }
            let state = if states.iter().any(|state| *state == CriterionStateV2::NotSatisfied) {
                CriterionStateV2::NotSatisfied
            } else if states.iter().all(|state| *state == CriterionStateV2::Satisfied) {
                CriterionStateV2::Satisfied
            } else {
                CriterionStateV2::Indeterminate
            };
            let evidence = sorted_unique(evidence);
            output.push(CriterionEvaluationV2 {
                criterion_id: criterion_id.clone(),
                state,
                evidence_fact_digests: evidence.clone(),
                reason: "all-child criterion aggregation".to_string(),
            });
            Ok((state, evidence))
        }
        RelativeCriterionV2::Any { criterion_id, children } => {
            let mut states = Vec::with_capacity(children.len());
            let mut evidence = Vec::new();
            for child in children {
                let (state, child_evidence) = evaluate_criterion(child, context, facts, output)?;
                states.push(state);
                evidence.extend(child_evidence);
            }
            let state = if states.iter().any(|state| *state == CriterionStateV2::Satisfied) {
                CriterionStateV2::Satisfied
            } else if states.iter().all(|state| *state == CriterionStateV2::NotSatisfied) {
                CriterionStateV2::NotSatisfied
            } else {
                CriterionStateV2::Indeterminate
            };
            let evidence = sorted_unique(evidence);
            output.push(CriterionEvaluationV2 {
                criterion_id: criterion_id.clone(),
                state,
                evidence_fact_digests: evidence.clone(),
                reason: "any-child criterion aggregation".to_string(),
            });
            Ok((state, evidence))
        }
    }
}

fn matching_fact_digests(
    facts: &[PreparedFact<'_>],
    concept: &ConceptIdentityV2,
) -> Vec<[u8; 32]> {
    facts
        .iter()
        .filter(|prepared| prepared.fact.concept.coding.iter().any(|coding| concept.matches(coding)))
        .map(|prepared| prepared.digest)
        .collect()
}

fn sorted_unique(values: Vec<[u8; 32]>) -> Vec<[u8; 32]> {
    let mut set = BTreeSet::new();
    set.extend(values);
    set.into_iter().collect()
}

fn validate_evaluation(evaluation: &RelativePhenotypeEvaluationV2) -> Result<(), PhenotypeV2Error> {
    if evaluation.schema_version != PHENOTYPE_V2_VERSION
        || evaluation.phenotype_id.trim().is_empty()
        || evaluation.phenotype_version.trim().is_empty()
        || evaluation.subject.resource_type.trim().is_empty()
        || evaluation.subject.id.trim().is_empty()
        || evaluation.definition_digest == [0; 32]
        || evaluation.evaluation_context_digest == [0; 32]
        || evaluation.window_start_micros >= evaluation.window_end_micros
        || evaluation.evidence_fact_digests.iter().any(|digest| *digest == [0; 32])
    {
        return Err(PhenotypeV2Error::InvalidEvaluation);
    }
    Ok(())
}

fn domain_digest(context: &str, tag: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&PHENOTYPE_V2_VERSION.to_be_bytes());
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
    fn bytes(&mut self, value: &[u8]) -> Result<(), PhenotypeV2Error> {
        self.u32(u32::try_from(value.len()).map_err(|_| PhenotypeV2Error::LengthOverflow)?);
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn string(&mut self, value: &str) -> Result<(), PhenotypeV2Error> { self.bytes(value.as_bytes()) }
    fn vector_len(&mut self, len: usize) -> Result<(), PhenotypeV2Error> {
        self.u32(u32::try_from(len).map_err(|_| PhenotypeV2Error::LengthOverflow)?);
        Ok(())
    }
}

fn encode_artifact(writer: &mut CanonicalWriter, value: &EvidenceArtifactIdentityV2) -> Result<(), PhenotypeV2Error> {
    writer.string(&value.namespace)?;
    writer.string(&value.artifact_id)?;
    writer.string(&value.version)?;
    writer.bytes(&value.digest)
}

fn encode_concept(writer: &mut CanonicalWriter, value: &ConceptIdentityV2) -> Result<(), PhenotypeV2Error> {
    writer.string(&value.system)?;
    writer.string(&value.code)?;
    writer.string(&value.version)
}

fn encode_definition(writer: &mut CanonicalWriter, definition: &RelativePhenotypeDefinitionV2) -> Result<(), PhenotypeV2Error> {
    writer.u16(definition.schema_version);
    writer.string(&definition.phenotype_id)?;
    writer.string(&definition.version)?;
    writer.string(&definition.subject_resource_type)?;
    writer.i64(definition.window.start_offset_micros);
    writer.i64(definition.window.end_offset_micros);
    let mut domains = definition.window.required_coverage_domains.clone();
    domains.sort();
    writer.vector_len(domains.len())?;
    for domain in &domains { writer.string(domain)?; }
    encode_criterion(writer, &definition.criterion)?;
    encode_artifact(writer, &definition.definition_evidence)
}

fn encode_criterion(writer: &mut CanonicalWriter, criterion: &RelativeCriterionV2) -> Result<(), PhenotypeV2Error> {
    match criterion {
        RelativeCriterionV2::FactPresent { criterion_id, concept, minimum_count, coverage_domain } => {
            writer.u8(0);
            writer.string(criterion_id)?;
            encode_concept(writer, concept)?;
            writer.u32(*minimum_count);
            writer.string(coverage_domain)?;
        }
        RelativeCriterionV2::FactAbsent { criterion_id, concept, coverage_domain } => {
            writer.u8(1);
            writer.string(criterion_id)?;
            encode_concept(writer, concept)?;
            writer.string(coverage_domain)?;
        }
        RelativeCriterionV2::All { criterion_id, children } => {
            writer.u8(2);
            writer.string(criterion_id)?;
            writer.vector_len(children.len())?;
            for child in children { encode_criterion(writer, child)?; }
        }
        RelativeCriterionV2::Any { criterion_id, children } => {
            writer.u8(3);
            writer.string(criterion_id)?;
            writer.vector_len(children.len())?;
            for child in children { encode_criterion(writer, child)?; }
        }
    }
    Ok(())
}

fn encode_context(writer: &mut CanonicalWriter, context: &PhenotypeEvaluationContextV2) -> Result<(), PhenotypeV2Error> {
    writer.u16(context.schema_version);
    writer.i64(context.anchor_micros);
    encode_artifact(writer, &context.anchor_evidence)?;
    let mut coverage = context.coverage.clone();
    coverage.sort_by(|left, right| left.domain.cmp(&right.domain));
    writer.vector_len(coverage.len())?;
    for item in &coverage {
        writer.string(&item.domain)?;
        writer.u8(match item.status {
            CoverageStatusV2::Complete => 0,
            CoverageStatusV2::Incomplete => 1,
            CoverageStatusV2::Unknown => 2,
        });
        encode_artifact(writer, &item.source)?;
    }
    let mut context_evidence = context.context_evidence.clone();
    context_evidence.sort();
    writer.vector_len(context_evidence.len())?;
    for item in &context_evidence { encode_artifact(writer, item)?; }
    Ok(())
}

fn encode_evaluation(writer: &mut CanonicalWriter, evaluation: &RelativePhenotypeEvaluationV2) -> Result<(), PhenotypeV2Error> {
    writer.u16(evaluation.schema_version);
    writer.string(&evaluation.phenotype_id)?;
    writer.string(&evaluation.phenotype_version)?;
    writer.bytes(&evaluation.definition_digest)?;
    writer.bytes(&evaluation.evaluation_context_digest)?;
    writer.string(&evaluation.subject.resource_type)?;
    writer.string(&evaluation.subject.id)?;
    writer.i64(evaluation.anchor_micros);
    writer.i64(evaluation.window_start_micros);
    writer.i64(evaluation.window_end_micros);
    writer.u8(match evaluation.state {
        CriterionStateV2::Satisfied => 0,
        CriterionStateV2::NotSatisfied => 1,
        CriterionStateV2::Indeterminate => 2,
    });
    writer.vector_len(evaluation.criterion_evaluations.len())?;
    for item in &evaluation.criterion_evaluations {
        writer.string(&item.criterion_id)?;
        writer.u8(match item.state {
            CriterionStateV2::Satisfied => 0,
            CriterionStateV2::NotSatisfied => 1,
            CriterionStateV2::Indeterminate => 2,
        });
        let evidence = sorted_unique(item.evidence_fact_digests.clone());
        writer.vector_len(evidence.len())?;
        for digest in &evidence { writer.bytes(digest)?; }
        writer.string(&item.reason)?;
    }
    let evidence = sorted_unique(evaluation.evidence_fact_digests.clone());
    writer.vector_len(evidence.len())?;
    for digest in &evidence { writer.bytes(digest)?; }
    Ok(())
}

#[derive(Debug, Error)]
pub enum PhenotypeV2Error {
    #[error("unsupported phenotype v2 schema version: {0}")]
    UnsupportedSchemaVersion(u16),
    #[error("evidence artifact identity is incomplete")]
    IncompleteEvidenceIdentity,
    #[error("evidence digest may not be all zeroes")]
    ZeroEvidenceDigest,
    #[error("concept identity is incomplete")]
    IncompleteConceptIdentity,
    #[error("terminology version is required")]
    MissingTerminologyVersion,
    #[error("coverage domain is missing")]
    MissingCoverageDomain,
    #[error("duplicate coverage domain: {0}")]
    DuplicateCoverageDomain(String),
    #[error("relative observation window is invalid")]
    InvalidRelativeWindow,
    #[error("definition identity is incomplete")]
    IncompleteDefinitionIdentity,
    #[error("criterion tree exceeds maximum depth")]
    CriterionDepthExceeded,
    #[error("criterion tree exceeds maximum count")]
    TooManyCriteria,
    #[error("criterion id is missing")]
    MissingCriterionId,
    #[error("duplicate criterion id: {0}")]
    DuplicateCriterionId(String),
    #[error("criterion minimum count must be positive")]
    ZeroMinimumCount,
    #[error("criterion group may not be empty")]
    EmptyCriterionGroup,
    #[error("criterion references undeclared coverage domain: {0}")]
    UnknownCoverageDomain(String),
    #[error("evaluation-context coverage domains do not exactly match the definition")]
    CoverageDomainSetMismatch,
    #[error("subject identity is invalid")]
    InvalidSubject,
    #[error("time arithmetic overflow")]
    TimeOverflow,
    #[error("derived absolute observation window is invalid")]
    InvalidDerivedWindow,
    #[error("facts from multiple subjects were supplied: expected {expected}, found {found}")]
    MixedSubjectFacts { expected: String, found: String },
    #[error("duplicate ClinicalFact snapshot identity")]
    DuplicateFactIdentity,
    #[error("relative phenotype evaluation is invalid")]
    InvalidEvaluation,
    #[error("canonical framing length exceeds v2 limit")]
    LengthOverflow,
    #[error(transparent)]
    Clinical(#[from] ClinicalSemanticsError),
}
