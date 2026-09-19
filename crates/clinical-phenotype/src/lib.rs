#![deny(unsafe_code)]
//! Evidence-bound longitudinal computable phenotyping for Mycelix Health.
//!
//! V1 deliberately focuses on concept presence/absence inside a bounded time
//! window. The central safety invariant is that absence becomes evidence only
//! when the relevant data-coverage domain is explicitly complete.

use mycelix_clinical_fact_snapshot::clinical_fact_snapshot_digest_v1;
use mycelix_clinical_semantics::{ClinicalFact, ClinicalSemanticsError, Coding, SubjectRef};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use thiserror::Error;

pub const PHENOTYPE_VERSION: u16 = 1;
const MAX_CRITERIA: usize = 1024;
const MAX_DEPTH: usize = 32;
const DEFINITION_TAG: &[u8] = b"mycelix/clinical-phenotype-definition/v1";
const EVALUATION_TAG: &[u8] = b"mycelix/clinical-phenotype-evaluation/v1";
const DEFINITION_CONTEXT: &str = "mycelix.health.clinical-phenotype-definition.v1";
const EVALUATION_CONTEXT: &str = "mycelix.health.clinical-phenotype-evaluation.v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceArtifactIdentityV1 {
    pub namespace: String,
    pub artifact_id: String,
    pub version: String,
    pub digest: [u8; 32],
}

impl EvidenceArtifactIdentityV1 {
    fn validate(&self) -> Result<(), PhenotypeError> {
        if self.namespace.trim().is_empty()
            || self.artifact_id.trim().is_empty()
            || self.version.trim().is_empty()
        {
            return Err(PhenotypeError::IncompleteEvidenceIdentity);
        }
        if self.digest == [0; 32] {
            return Err(PhenotypeError::ZeroEvidenceDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConceptIdentityV1 {
    pub system: String,
    pub code: String,
    pub version: String,
}

impl ConceptIdentityV1 {
    fn validate(&self) -> Result<(), PhenotypeError> {
        if self.system.trim().is_empty() || self.code.trim().is_empty() {
            return Err(PhenotypeError::IncompleteConceptIdentity);
        }
        if self.version.trim().is_empty() {
            return Err(PhenotypeError::MissingTerminologyVersion);
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
pub enum CoverageStatusV1 {
    Complete,
    Incomplete,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverageEvidenceV1 {
    pub domain: String,
    pub status: CoverageStatusV1,
    pub source: EvidenceArtifactIdentityV1,
}

impl CoverageEvidenceV1 {
    fn validate(&self) -> Result<(), PhenotypeError> {
        if self.domain.trim().is_empty() {
            return Err(PhenotypeError::MissingCoverageDomain);
        }
        self.source.validate()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObservationWindowV1 {
    pub start_micros: i64,
    pub end_micros: i64,
    pub coverage: Vec<CoverageEvidenceV1>,
}

impl ObservationWindowV1 {
    fn validate(&self) -> Result<(), PhenotypeError> {
        if self.start_micros >= self.end_micros {
            return Err(PhenotypeError::InvalidObservationWindow);
        }
        let mut domains = HashSet::new();
        for item in &self.coverage {
            item.validate()?;
            if !domains.insert(item.domain.clone()) {
                return Err(PhenotypeError::DuplicateCoverageDomain(item.domain.clone()));
            }
        }
        Ok(())
    }

    fn coverage_for(&self, domain: &str) -> Option<&CoverageEvidenceV1> {
        self.coverage.iter().find(|item| item.domain == domain)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CriterionV1 {
    FactPresent {
        criterion_id: String,
        concept: ConceptIdentityV1,
        minimum_count: u32,
        coverage_domain: String,
    },
    FactAbsent {
        criterion_id: String,
        concept: ConceptIdentityV1,
        coverage_domain: String,
    },
    All {
        criterion_id: String,
        children: Vec<CriterionV1>,
    },
    Any {
        criterion_id: String,
        children: Vec<CriterionV1>,
    },
}

impl CriterionV1 {
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
pub struct PhenotypeDefinitionV1 {
    pub schema_version: u16,
    pub phenotype_id: String,
    pub version: String,
    pub subject_resource_type: String,
    pub window: ObservationWindowV1,
    pub criterion: CriterionV1,
    pub definition_evidence: EvidenceArtifactIdentityV1,
}

impl PhenotypeDefinitionV1 {
    pub fn validate(&self) -> Result<(), PhenotypeError> {
        if self.schema_version != PHENOTYPE_VERSION {
            return Err(PhenotypeError::UnsupportedSchemaVersion(self.schema_version));
        }
        if self.phenotype_id.trim().is_empty()
            || self.version.trim().is_empty()
            || self.subject_resource_type.trim().is_empty()
        {
            return Err(PhenotypeError::IncompleteDefinitionIdentity);
        }
        self.window.validate()?;
        self.definition_evidence.validate()?;

        let mut ids = HashSet::new();
        let mut count = 0usize;
        validate_criterion(&self.criterion, &self.window, 0, &mut count, &mut ids)?;
        Ok(())
    }
}

fn validate_criterion(
    criterion: &CriterionV1,
    window: &ObservationWindowV1,
    depth: usize,
    count: &mut usize,
    ids: &mut HashSet<String>,
) -> Result<(), PhenotypeError> {
    if depth > MAX_DEPTH {
        return Err(PhenotypeError::CriterionDepthExceeded);
    }
    *count += 1;
    if *count > MAX_CRITERIA {
        return Err(PhenotypeError::TooManyCriteria);
    }
    if criterion.id().trim().is_empty() {
        return Err(PhenotypeError::MissingCriterionId);
    }
    if !ids.insert(criterion.id().to_string()) {
        return Err(PhenotypeError::DuplicateCriterionId(criterion.id().to_string()));
    }

    match criterion {
        CriterionV1::FactPresent {
            concept,
            minimum_count,
            coverage_domain,
            ..
        } => {
            concept.validate()?;
            if *minimum_count == 0 {
                return Err(PhenotypeError::ZeroMinimumCount);
            }
            require_coverage_domain(window, coverage_domain)?;
        }
        CriterionV1::FactAbsent {
            concept,
            coverage_domain,
            ..
        } => {
            concept.validate()?;
            require_coverage_domain(window, coverage_domain)?;
        }
        CriterionV1::All { children, .. } | CriterionV1::Any { children, .. } => {
            if children.is_empty() {
                return Err(PhenotypeError::EmptyCriterionGroup);
            }
            for child in children {
                validate_criterion(child, window, depth + 1, count, ids)?;
            }
        }
    }
    Ok(())
}

fn require_coverage_domain(
    window: &ObservationWindowV1,
    domain: &str,
) -> Result<(), PhenotypeError> {
    if domain.trim().is_empty() {
        return Err(PhenotypeError::MissingCoverageDomain);
    }
    if window.coverage_for(domain).is_none() {
        return Err(PhenotypeError::UnknownCoverageDomain(domain.to_string()));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CriterionStateV1 {
    Satisfied,
    NotSatisfied,
    Indeterminate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CriterionEvaluationV1 {
    pub criterion_id: String,
    pub state: CriterionStateV1,
    pub evidence_fact_digests: Vec<[u8; 32]>,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhenotypeEvaluationV1 {
    pub schema_version: u16,
    pub phenotype_id: String,
    pub phenotype_version: String,
    pub subject: SubjectRef,
    pub state: CriterionStateV1,
    pub definition_digest: [u8; 32],
    pub criterion_evaluations: Vec<CriterionEvaluationV1>,
    pub evidence_fact_digests: Vec<[u8; 32]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PhenotypeDefinitionDigestV1([u8; 32]);

impl PhenotypeDefinitionDigestV1 {
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PhenotypeEvaluationDigestV1([u8; 32]);

impl PhenotypeEvaluationDigestV1 {
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

pub fn phenotype_definition_digest_v1(
    definition: &PhenotypeDefinitionV1,
) -> Result<PhenotypeDefinitionDigestV1, PhenotypeError> {
    definition.validate()?;
    let mut writer = CanonicalWriter::default();
    encode_definition(&mut writer, definition)?;
    let bytes = writer.finish();
    Ok(PhenotypeDefinitionDigestV1(domain_digest(
        DEFINITION_CONTEXT,
        DEFINITION_TAG,
        &bytes,
    )))
}

pub fn evaluate_phenotype_v1(
    definition: &PhenotypeDefinitionV1,
    subject_id: &str,
    facts: &[ClinicalFact],
) -> Result<PhenotypeEvaluationV1, PhenotypeError> {
    definition.validate()?;
    if subject_id.trim().is_empty() {
        return Err(PhenotypeError::InvalidSubject);
    }

    let mut prepared = Vec::new();
    let mut seen_digests = HashSet::new();
    for fact in facts {
        fact.validate_machine_actionable()?;
        if fact.subject.resource_type != definition.subject_resource_type || fact.subject.id != subject_id {
            return Err(PhenotypeError::MixedSubjectFacts {
                expected: format!("{}/{}", definition.subject_resource_type, subject_id),
                found: format!("{}/{}", fact.subject.resource_type, fact.subject.id),
            });
        }
        let digest = clinical_fact_snapshot_digest_v1(fact)?.into_bytes();
        if !seen_digests.insert(digest) {
            return Err(PhenotypeError::DuplicateFactIdentity);
        }
        if fact.effective_at_micros >= definition.window.start_micros
            && fact.effective_at_micros < definition.window.end_micros
        {
            prepared.push(PreparedFact { fact, digest });
        }
    }

    let mut evaluations = Vec::new();
    let (state, evidence) = evaluate_criterion(
        &definition.criterion,
        &definition.window,
        &prepared,
        &mut evaluations,
    )?;
    let definition_digest = phenotype_definition_digest_v1(definition)?.into_bytes();
    let evidence_fact_digests = sorted_unique(evidence);

    Ok(PhenotypeEvaluationV1 {
        schema_version: PHENOTYPE_VERSION,
        phenotype_id: definition.phenotype_id.clone(),
        phenotype_version: definition.version.clone(),
        subject: SubjectRef {
            resource_type: definition.subject_resource_type.clone(),
            id: subject_id.to_string(),
        },
        state,
        definition_digest,
        criterion_evaluations: evaluations,
        evidence_fact_digests,
    })
}

pub fn phenotype_evaluation_digest_v1(
    evaluation: &PhenotypeEvaluationV1,
) -> Result<PhenotypeEvaluationDigestV1, PhenotypeError> {
    if evaluation.schema_version != PHENOTYPE_VERSION
        || evaluation.phenotype_id.trim().is_empty()
        || evaluation.phenotype_version.trim().is_empty()
        || evaluation.subject.resource_type.trim().is_empty()
        || evaluation.subject.id.trim().is_empty()
        || evaluation.definition_digest == [0; 32]
    {
        return Err(PhenotypeError::InvalidEvaluation);
    }
    let mut writer = CanonicalWriter::default();
    encode_evaluation(&mut writer, evaluation)?;
    let bytes = writer.finish();
    Ok(PhenotypeEvaluationDigestV1(domain_digest(
        EVALUATION_CONTEXT,
        EVALUATION_TAG,
        &bytes,
    )))
}

struct PreparedFact<'a> {
    fact: &'a ClinicalFact,
    digest: [u8; 32],
}

fn evaluate_criterion(
    criterion: &CriterionV1,
    window: &ObservationWindowV1,
    facts: &[PreparedFact<'_>],
    output: &mut Vec<CriterionEvaluationV1>,
) -> Result<(CriterionStateV1, Vec<[u8; 32]>), PhenotypeError> {
    match criterion {
        CriterionV1::FactPresent {
            criterion_id,
            concept,
            minimum_count,
            coverage_domain,
        } => {
            let matches = matching_fact_digests(facts, concept);
            let coverage = window
                .coverage_for(coverage_domain)
                .ok_or_else(|| PhenotypeError::UnknownCoverageDomain(coverage_domain.clone()))?;
            let state = if matches.len() >= *minimum_count as usize {
                CriterionStateV1::Satisfied
            } else if coverage.status == CoverageStatusV1::Complete {
                CriterionStateV1::NotSatisfied
            } else {
                CriterionStateV1::Indeterminate
            };
            let reason = match state {
                CriterionStateV1::Satisfied => format!(
                    "observed {} matching fact(s), meeting minimum {}",
                    matches.len(), minimum_count
                ),
                CriterionStateV1::NotSatisfied => format!(
                    "complete coverage observed {} matching fact(s), below minimum {}",
                    matches.len(), minimum_count
                ),
                CriterionStateV1::Indeterminate => format!(
                    "only {} matching fact(s) observed and coverage domain '{}' is not complete",
                    matches.len(), coverage_domain
                ),
            };
            output.push(CriterionEvaluationV1 {
                criterion_id: criterion_id.clone(),
                state,
                evidence_fact_digests: matches.clone(),
                reason,
            });
            Ok((state, matches))
        }
        CriterionV1::FactAbsent {
            criterion_id,
            concept,
            coverage_domain,
        } => {
            let matches = matching_fact_digests(facts, concept);
            let coverage = window
                .coverage_for(coverage_domain)
                .ok_or_else(|| PhenotypeError::UnknownCoverageDomain(coverage_domain.clone()))?;
            let state = if !matches.is_empty() {
                CriterionStateV1::NotSatisfied
            } else if coverage.status == CoverageStatusV1::Complete {
                CriterionStateV1::Satisfied
            } else {
                CriterionStateV1::Indeterminate
            };
            let reason = match state {
                CriterionStateV1::NotSatisfied => {
                    format!("observed {} matching fact(s) that refute absence", matches.len())
                }
                CriterionStateV1::Satisfied => {
                    "no matching facts observed under complete coverage".to_string()
                }
                CriterionStateV1::Indeterminate => format!(
                    "no matching facts observed but coverage domain '{}' is not complete",
                    coverage_domain
                ),
            };
            output.push(CriterionEvaluationV1 {
                criterion_id: criterion_id.clone(),
                state,
                evidence_fact_digests: matches.clone(),
                reason,
            });
            Ok((state, matches))
        }
        CriterionV1::All {
            criterion_id,
            children,
        } => {
            let mut child_states = Vec::with_capacity(children.len());
            let mut evidence = Vec::new();
            for child in children {
                let (state, child_evidence) = evaluate_criterion(child, window, facts, output)?;
                child_states.push(state);
                evidence.extend(child_evidence);
            }
            let state = if child_states.contains(&CriterionStateV1::NotSatisfied) {
                CriterionStateV1::NotSatisfied
            } else if child_states.contains(&CriterionStateV1::Indeterminate) {
                CriterionStateV1::Indeterminate
            } else {
                CriterionStateV1::Satisfied
            };
            let evidence = sorted_unique(evidence);
            output.push(CriterionEvaluationV1 {
                criterion_id: criterion_id.clone(),
                state,
                evidence_fact_digests: evidence.clone(),
                reason: "all-child composition".to_string(),
            });
            Ok((state, evidence))
        }
        CriterionV1::Any {
            criterion_id,
            children,
        } => {
            let mut child_states = Vec::with_capacity(children.len());
            let mut evidence = Vec::new();
            for child in children {
                let (state, child_evidence) = evaluate_criterion(child, window, facts, output)?;
                child_states.push(state);
                evidence.extend(child_evidence);
            }
            let state = if child_states.contains(&CriterionStateV1::Satisfied) {
                CriterionStateV1::Satisfied
            } else if child_states.contains(&CriterionStateV1::Indeterminate) {
                CriterionStateV1::Indeterminate
            } else {
                CriterionStateV1::NotSatisfied
            };
            let evidence = sorted_unique(evidence);
            output.push(CriterionEvaluationV1 {
                criterion_id: criterion_id.clone(),
                state,
                evidence_fact_digests: evidence.clone(),
                reason: "any-child composition".to_string(),
            });
            Ok((state, evidence))
        }
    }
}

fn matching_fact_digests(
    facts: &[PreparedFact<'_>],
    concept: &ConceptIdentityV1,
) -> Vec<[u8; 32]> {
    let mut matches: Vec<_> = facts
        .iter()
        .filter(|prepared| prepared.fact.concept.coding.iter().any(|coding| concept.matches(coding)))
        .map(|prepared| prepared.digest)
        .collect();
    matches.sort_unstable();
    matches
}

fn sorted_unique(values: Vec<[u8; 32]>) -> Vec<[u8; 32]> {
    let set: BTreeSet<_> = values.into_iter().collect();
    set.into_iter().collect()
}

fn domain_digest(context: &str, tag: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&PHENOTYPE_VERSION.to_be_bytes());
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
    fn finish(self) -> Vec<u8> {
        self.bytes
    }
    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }
    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn bytes(&mut self, value: &[u8]) -> Result<(), PhenotypeError> {
        let len = u32::try_from(value.len()).map_err(|_| PhenotypeError::LengthOverflow)?;
        self.u32(len);
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn string(&mut self, value: &str) -> Result<(), PhenotypeError> {
        self.bytes(value.as_bytes())
    }
    fn vector_len(&mut self, len: usize) -> Result<(), PhenotypeError> {
        self.u32(u32::try_from(len).map_err(|_| PhenotypeError::LengthOverflow)?);
        Ok(())
    }
}

fn encode_artifact(writer: &mut CanonicalWriter, value: &EvidenceArtifactIdentityV1) -> Result<(), PhenotypeError> {
    writer.string(&value.namespace)?;
    writer.string(&value.artifact_id)?;
    writer.string(&value.version)?;
    writer.bytes(&value.digest)?;
    Ok(())
}

fn encode_concept(writer: &mut CanonicalWriter, value: &ConceptIdentityV1) -> Result<(), PhenotypeError> {
    writer.string(&value.system)?;
    writer.string(&value.code)?;
    writer.string(&value.version)?;
    Ok(())
}

fn encode_definition(writer: &mut CanonicalWriter, definition: &PhenotypeDefinitionV1) -> Result<(), PhenotypeError> {
    writer.u16(definition.schema_version);
    writer.string(&definition.phenotype_id)?;
    writer.string(&definition.version)?;
    writer.string(&definition.subject_resource_type)?;
    writer.i64(definition.window.start_micros);
    writer.i64(definition.window.end_micros);
    writer.vector_len(definition.window.coverage.len())?;
    for coverage in &definition.window.coverage {
        writer.string(&coverage.domain)?;
        writer.u8(match coverage.status {
            CoverageStatusV1::Complete => 0,
            CoverageStatusV1::Incomplete => 1,
            CoverageStatusV1::Unknown => 2,
        });
        encode_artifact(writer, &coverage.source)?;
    }
    encode_criterion(writer, &definition.criterion)?;
    encode_artifact(writer, &definition.definition_evidence)?;
    Ok(())
}

fn encode_criterion(writer: &mut CanonicalWriter, criterion: &CriterionV1) -> Result<(), PhenotypeError> {
    match criterion {
        CriterionV1::FactPresent {
            criterion_id,
            concept,
            minimum_count,
            coverage_domain,
        } => {
            writer.u8(0);
            writer.string(criterion_id)?;
            encode_concept(writer, concept)?;
            writer.u32(*minimum_count);
            writer.string(coverage_domain)?;
        }
        CriterionV1::FactAbsent {
            criterion_id,
            concept,
            coverage_domain,
        } => {
            writer.u8(1);
            writer.string(criterion_id)?;
            encode_concept(writer, concept)?;
            writer.string(coverage_domain)?;
        }
        CriterionV1::All {
            criterion_id,
            children,
        } => {
            writer.u8(2);
            writer.string(criterion_id)?;
            writer.vector_len(children.len())?;
            for child in children {
                encode_criterion(writer, child)?;
            }
        }
        CriterionV1::Any {
            criterion_id,
            children,
        } => {
            writer.u8(3);
            writer.string(criterion_id)?;
            writer.vector_len(children.len())?;
            for child in children {
                encode_criterion(writer, child)?;
            }
        }
    }
    Ok(())
}

fn encode_evaluation(writer: &mut CanonicalWriter, evaluation: &PhenotypeEvaluationV1) -> Result<(), PhenotypeError> {
    writer.u16(evaluation.schema_version);
    writer.string(&evaluation.phenotype_id)?;
    writer.string(&evaluation.phenotype_version)?;
    writer.string(&evaluation.subject.resource_type)?;
    writer.string(&evaluation.subject.id)?;
    writer.u8(match evaluation.state {
        CriterionStateV1::Satisfied => 0,
        CriterionStateV1::NotSatisfied => 1,
        CriterionStateV1::Indeterminate => 2,
    });
    writer.bytes(&evaluation.definition_digest)?;
    writer.vector_len(evaluation.criterion_evaluations.len())?;
    for item in &evaluation.criterion_evaluations {
        writer.string(&item.criterion_id)?;
        writer.u8(match item.state {
            CriterionStateV1::Satisfied => 0,
            CriterionStateV1::NotSatisfied => 1,
            CriterionStateV1::Indeterminate => 2,
        });
        writer.vector_len(item.evidence_fact_digests.len())?;
        for digest in &item.evidence_fact_digests {
            writer.bytes(digest)?;
        }
        writer.string(&item.reason)?;
    }
    writer.vector_len(evaluation.evidence_fact_digests.len())?;
    for digest in &evaluation.evidence_fact_digests {
        writer.bytes(digest)?;
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum PhenotypeError {
    #[error("unsupported phenotype schema version: {0}")]
    UnsupportedSchemaVersion(u16),
    #[error("phenotype definition identity is incomplete")]
    IncompleteDefinitionIdentity,
    #[error("evidence artifact identity is incomplete")]
    IncompleteEvidenceIdentity,
    #[error("evidence artifact digest may not be all zeroes")]
    ZeroEvidenceDigest,
    #[error("concept identity is incomplete")]
    IncompleteConceptIdentity,
    #[error("criterion terminology version is required")]
    MissingTerminologyVersion,
    #[error("observation window must satisfy start < end")]
    InvalidObservationWindow,
    #[error("coverage domain is missing")]
    MissingCoverageDomain,
    #[error("duplicate coverage domain: {0}")]
    DuplicateCoverageDomain(String),
    #[error("criterion references unknown coverage domain: {0}")]
    UnknownCoverageDomain(String),
    #[error("criterion id is missing")]
    MissingCriterionId,
    #[error("duplicate criterion id: {0}")]
    DuplicateCriterionId(String),
    #[error("FactPresent minimum_count must be greater than zero")]
    ZeroMinimumCount,
    #[error("All/Any criterion group may not be empty")]
    EmptyCriterionGroup,
    #[error("criterion nesting exceeds v1 depth limit")]
    CriterionDepthExceeded,
    #[error("phenotype definition exceeds v1 criterion-count limit")]
    TooManyCriteria,
    #[error("subject id is invalid")]
    InvalidSubject,
    #[error("fact set mixes subjects: expected {expected}, found {found}")]
    MixedSubjectFacts { expected: String, found: String },
    #[error("duplicate ClinicalFact snapshot identity in input")]
    DuplicateFactIdentity,
    #[error("phenotype evaluation is malformed")]
    InvalidEvaluation,
    #[error("canonical framing length exceeds v1 limit")]
    LengthOverflow,
    #[error(transparent)]
    ClinicalSemantics(#[from] ClinicalSemanticsError),
    #[error(transparent)]
    ClinicalFactSnapshot(#[from] mycelix_clinical_fact_snapshot::ClinicalFactSnapshotError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_semantics::{
        ClinicalValue, CodeableConcept, Coding, FactProvenance, Quantity, Uncertainty,
    };

    fn artifact(id: &str, byte: u8) -> EvidenceArtifactIdentityV1 {
        EvidenceArtifactIdentityV1 {
            namespace: "research/evidence/v1".to_string(),
            artifact_id: id.to_string(),
            version: "1".to_string(),
            digest: [byte; 32],
        }
    }

    fn concept(code: &str) -> ConceptIdentityV1 {
        ConceptIdentityV1 {
            system: "http://loinc.org".to_string(),
            code: code.to_string(),
            version: "2.83".to_string(),
        }
    }

    fn definition(status: CoverageStatusV1, criterion: CriterionV1) -> PhenotypeDefinitionV1 {
        PhenotypeDefinitionV1 {
            schema_version: PHENOTYPE_VERSION,
            phenotype_id: "phenotype:test".to_string(),
            version: "1".to_string(),
            subject_resource_type: "Patient".to_string(),
            window: ObservationWindowV1 {
                start_micros: 100,
                end_micros: 200,
                coverage: vec![CoverageEvidenceV1 {
                    domain: "labs".to_string(),
                    status,
                    source: artifact("coverage", 1),
                }],
            },
            criterion,
            definition_evidence: artifact("definition", 2),
        }
    }

    fn fact(id: &str, patient: &str, code: &str, time: i64) -> ClinicalFact {
        ClinicalFact {
            fact_id: id.to_string(),
            subject: SubjectRef {
                resource_type: "Patient".to_string(),
                id: patient.to_string(),
            },
            concept: CodeableConcept {
                coding: vec![Coding {
                    system: "http://loinc.org".to_string(),
                    code: code.to_string(),
                    display: None,
                    version: Some("2.83".to_string()),
                }],
                text: None,
            },
            value: ClinicalValue::Quantity(Quantity::ucum(5.5, "mmol/L")),
            effective_at_micros: time,
            provenance: FactProvenance {
                source_system: "test".to_string(),
                source_resource_type: "Observation".to_string(),
                source_resource_id: id.to_string(),
                source_version: Some("1".to_string()),
                recorded_at_micros: time,
                asserted_by: None,
                transformation: None,
            },
            uncertainty: Some(Uncertainty {
                confidence: None,
                interpretation: None,
            }),
        }
    }

    #[test]
    fn absent_fact_under_incomplete_coverage_is_indeterminate() {
        let definition = definition(
            CoverageStatusV1::Incomplete,
            CriterionV1::FactAbsent {
                criterion_id: "no-a".to_string(),
                concept: concept("A"),
                coverage_domain: "labs".to_string(),
            },
        );
        let evaluation = evaluate_phenotype_v1(&definition, "patient-a", &[]).unwrap();
        assert_eq!(evaluation.state, CriterionStateV1::Indeterminate);
    }

    #[test]
    fn absent_fact_under_complete_coverage_is_satisfied() {
        let definition = definition(
            CoverageStatusV1::Complete,
            CriterionV1::FactAbsent {
                criterion_id: "no-a".to_string(),
                concept: concept("A"),
                coverage_domain: "labs".to_string(),
            },
        );
        let evaluation = evaluate_phenotype_v1(&definition, "patient-a", &[]).unwrap();
        assert_eq!(evaluation.state, CriterionStateV1::Satisfied);
    }

    #[test]
    fn observed_fact_refutes_absence_even_when_coverage_is_incomplete() {
        let definition = definition(
            CoverageStatusV1::Incomplete,
            CriterionV1::FactAbsent {
                criterion_id: "no-a".to_string(),
                concept: concept("A"),
                coverage_domain: "labs".to_string(),
            },
        );
        let evaluation = evaluate_phenotype_v1(
            &definition,
            "patient-a",
            &[fact("fact-a", "patient-a", "A", 150)],
        )
        .unwrap();
        assert_eq!(evaluation.state, CriterionStateV1::NotSatisfied);
        assert_eq!(evaluation.evidence_fact_digests.len(), 1);
    }

    #[test]
    fn positive_fact_satisfies_presence_even_when_coverage_is_incomplete() {
        let definition = definition(
            CoverageStatusV1::Incomplete,
            CriterionV1::FactPresent {
                criterion_id: "has-a".to_string(),
                concept: concept("A"),
                minimum_count: 1,
                coverage_domain: "labs".to_string(),
            },
        );
        let evaluation = evaluate_phenotype_v1(
            &definition,
            "patient-a",
            &[fact("fact-a", "patient-a", "A", 150)],
        )
        .unwrap();
        assert_eq!(evaluation.state, CriterionStateV1::Satisfied);
    }

    #[test]
    fn fact_outside_window_cannot_satisfy_presence() {
        let definition = definition(
            CoverageStatusV1::Complete,
            CriterionV1::FactPresent {
                criterion_id: "has-a".to_string(),
                concept: concept("A"),
                minimum_count: 1,
                coverage_domain: "labs".to_string(),
            },
        );
        let evaluation = evaluate_phenotype_v1(
            &definition,
            "patient-a",
            &[fact("fact-a", "patient-a", "A", 200)],
        )
        .unwrap();
        assert_eq!(evaluation.state, CriterionStateV1::NotSatisfied);
        assert!(evaluation.evidence_fact_digests.is_empty());
    }

    #[test]
    fn patient_mixing_is_rejected() {
        let definition = definition(
            CoverageStatusV1::Complete,
            CriterionV1::FactPresent {
                criterion_id: "has-a".to_string(),
                concept: concept("A"),
                minimum_count: 1,
                coverage_domain: "labs".to_string(),
            },
        );
        assert!(matches!(
            evaluate_phenotype_v1(
                &definition,
                "patient-a",
                &[fact("fact-b", "patient-b", "A", 150)]
            ),
            Err(PhenotypeError::MixedSubjectFacts { .. })
        ));
    }

    #[test]
    fn terminology_version_is_part_of_matching_semantics() {
        let definition = definition(
            CoverageStatusV1::Complete,
            CriterionV1::FactPresent {
                criterion_id: "has-a".to_string(),
                concept: concept("A"),
                minimum_count: 1,
                coverage_domain: "labs".to_string(),
            },
        );
        let mut input = fact("fact-a", "patient-a", "A", 150);
        input.concept.coding[0].version = Some("2.82".to_string());
        let evaluation = evaluate_phenotype_v1(&definition, "patient-a", &[input]).unwrap();
        assert_eq!(evaluation.state, CriterionStateV1::NotSatisfied);
    }

    #[test]
    fn all_propagates_indeterminate_without_erasing_definite_failure() {
        let definition = definition(
            CoverageStatusV1::Incomplete,
            CriterionV1::All {
                criterion_id: "all".to_string(),
                children: vec![
                    CriterionV1::FactPresent {
                        criterion_id: "has-a".to_string(),
                        concept: concept("A"),
                        minimum_count: 1,
                        coverage_domain: "labs".to_string(),
                    },
                    CriterionV1::FactAbsent {
                        criterion_id: "no-b".to_string(),
                        concept: concept("B"),
                        coverage_domain: "labs".to_string(),
                    },
                ],
            },
        );
        let evaluation = evaluate_phenotype_v1(
            &definition,
            "patient-a",
            &[fact("fact-a", "patient-a", "A", 150)],
        )
        .unwrap();
        assert_eq!(evaluation.state, CriterionStateV1::Indeterminate);

        let failed = evaluate_phenotype_v1(
            &definition,
            "patient-a",
            &[
                fact("fact-a", "patient-a", "A", 150),
                fact("fact-b", "patient-a", "B", 151),
            ],
        )
        .unwrap();
        assert_eq!(failed.state, CriterionStateV1::NotSatisfied);
    }

    #[test]
    fn definition_and_evaluation_digests_are_deterministic() {
        let definition = definition(
            CoverageStatusV1::Complete,
            CriterionV1::FactPresent {
                criterion_id: "has-a".to_string(),
                concept: concept("A"),
                minimum_count: 1,
                coverage_domain: "labs".to_string(),
            },
        );
        let definition_digest = phenotype_definition_digest_v1(&definition).unwrap();
        assert_eq!(definition_digest, phenotype_definition_digest_v1(&definition).unwrap());

        let evaluation = evaluate_phenotype_v1(
            &definition,
            "patient-a",
            &[fact("fact-a", "patient-a", "A", 150)],
        )
        .unwrap();
        let digest = phenotype_evaluation_digest_v1(&evaluation).unwrap();
        assert_eq!(digest, phenotype_evaluation_digest_v1(&evaluation).unwrap());
    }
}
