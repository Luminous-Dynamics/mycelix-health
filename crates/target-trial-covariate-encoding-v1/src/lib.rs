#![deny(unsafe_code)]
//! Research-only explicit baseline covariate value encoding v1.
//!
//! This layer converts the exact selected ClinicalFact snapshots referenced by
//! a verified baseline identity/missingness matrix into explicitly declared
//! analysis values. It performs no imputation, unit conversion, categorization,
//! standardization, weighting, balance assessment, or causal estimation.

use mycelix_clinical_fact_snapshot::{
    clinical_fact_snapshot_digest_v1, ClinicalFactSnapshotError,
};
use mycelix_clinical_semantics::{ClinicalFact, ClinicalValue, UCUM_SYSTEM};
use mycelix_target_trial_baseline_matrix_v1::{
    baseline_covariate_matrix_digest_v1, BaselineMatrixV1Error,
    VerifiedBaselineCovariateMatrixV1,
};
use mycelix_target_trial_protocol_v2::EvidenceArtifactIdentityV2;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use thiserror::Error;

pub const TARGET_TRIAL_COVARIATE_ENCODING_V1_VERSION: u16 = 1;

const POLICY_TAG: &[u8] = b"mycelix/target-trial-covariate-encoding-policy/v1";
const MATRIX_TAG: &[u8] = b"mycelix/target-trial-encoded-baseline-matrix/v1";
const POLICY_CONTEXT: &str = "mycelix.health.target-trial-covariate-encoding-policy.v1";
const MATRIX_CONTEXT: &str = "mycelix.health.target-trial-encoded-baseline-matrix.v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CovariateEncodingRuleV1 {
    Boolean,
    Integer,
    Decimal,
    QuantityUcumExact { unit_code: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineCovariateEncodingPolicyV1 {
    pub schema_version: u16,
    pub confounder_id: String,
    pub measurement_policy_digest: [u8; 32],
    pub rule: CovariateEncodingRuleV1,
    pub policy_evidence: EvidenceArtifactIdentityV2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BaselineCovariateEncodingPolicyDigestV1([u8; 32]);

impl BaselineCovariateEncodingPolicyDigestV1 {
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EncodedCovariateValueV1 {
    Missing,
    Boolean(bool),
    Integer(i64),
    DecimalBits(u64),
    QuantityUcumExact { value_bits: u64, unit_code: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncodedCovariateColumnV1 {
    pub confounder_id: String,
    pub measurement_policy_digest: [u8; 32],
    pub encoding_policy_digest: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncodedCovariateCellV1 {
    pub confounder_id: String,
    pub measurement_receipt_digest: [u8; 32],
    pub selected_fact_snapshot_digest: Option<[u8; 32]>,
    pub value: EncodedCovariateValueV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncodedCovariateRowV1 {
    pub subject_resource_type: String,
    pub subject_id: String,
    pub strategy_id: String,
    pub contribution_digest: [u8; 32],
    pub time_zero_receipt_digest: [u8; 32],
    pub cells: Vec<EncodedCovariateCellV1>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncodedBaselineCovariateMatrixV1 {
    pub schema_version: u16,
    pub baseline_matrix_digest: [u8; 32],
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub estimand_id: String,
    pub analysis_manifest_digest: [u8; 32],
    pub columns: Vec<EncodedCovariateColumnV1>,
    pub rows: Vec<EncodedCovariateRowV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EncodedBaselineCovariateMatrixDigestV1([u8; 32]);

impl EncodedBaselineCovariateMatrixDigestV1 {
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

pub struct VerifiedEncodedBaselineCovariateMatrixV1 {
    matrix: EncodedBaselineCovariateMatrixV1,
    digest: EncodedBaselineCovariateMatrixDigestV1,
}

impl VerifiedEncodedBaselineCovariateMatrixV1 {
    #[must_use]
    pub fn matrix(&self) -> &EncodedBaselineCovariateMatrixV1 {
        &self.matrix
    }

    #[must_use]
    pub const fn digest(&self) -> EncodedBaselineCovariateMatrixDigestV1 {
        self.digest
    }

    #[must_use]
    pub fn missing_count(&self) -> usize {
        self.matrix
            .rows
            .iter()
            .flat_map(|row| row.cells.iter())
            .filter(|cell| matches!(&cell.value, EncodedCovariateValueV1::Missing))
            .count()
    }
}

pub fn baseline_covariate_encoding_policy_digest_v1(
    policy: &BaselineCovariateEncodingPolicyV1,
) -> Result<BaselineCovariateEncodingPolicyDigestV1, CovariateEncodingV1Error> {
    validate_policy(policy)?;
    let mut writer = CanonicalWriter::default();
    encode_policy(&mut writer, policy)?;
    Ok(BaselineCovariateEncodingPolicyDigestV1(domain_digest(
        POLICY_CONTEXT,
        POLICY_TAG,
        &writer.finish(),
    )))
}

pub fn build_encoded_baseline_covariate_matrix_v1(
    baseline: &VerifiedBaselineCovariateMatrixV1,
    policies: &[BaselineCovariateEncodingPolicyV1],
    source_facts: &[ClinicalFact],
) -> Result<VerifiedEncodedBaselineCovariateMatrixV1, CovariateEncodingV1Error> {
    let baseline_matrix = baseline.matrix();
    let baseline_matrix_digest = baseline_covariate_matrix_digest_v1(baseline_matrix)?.into_bytes();

    let mut policy_by_confounder = BTreeMap::new();
    for policy in policies {
        validate_policy(policy)?;
        if policy_by_confounder
            .insert(policy.confounder_id.clone(), policy)
            .is_some()
        {
            return Err(CovariateEncodingV1Error::DuplicateEncodingPolicy(
                policy.confounder_id.clone(),
            ));
        }
    }
    if policy_by_confounder.len() != baseline_matrix.columns.len() {
        return Err(CovariateEncodingV1Error::EncodingPolicySetMismatch);
    }

    let mut columns = Vec::with_capacity(baseline_matrix.columns.len());
    for column in &baseline_matrix.columns {
        let policy = policy_by_confounder
            .get(&column.confounder_id)
            .ok_or_else(|| CovariateEncodingV1Error::MissingEncodingPolicy(
                column.confounder_id.clone(),
            ))?;
        if policy.measurement_policy_digest != column.measurement_policy_digest {
            return Err(CovariateEncodingV1Error::MeasurementPolicyDigestMismatch(
                column.confounder_id.clone(),
            ));
        }
        columns.push(EncodedCovariateColumnV1 {
            confounder_id: column.confounder_id.clone(),
            measurement_policy_digest: column.measurement_policy_digest,
            encoding_policy_digest: baseline_covariate_encoding_policy_digest_v1(policy)?
                .into_bytes(),
        });
    }

    let mut fact_by_digest = BTreeMap::new();
    for fact in source_facts {
        let digest = clinical_fact_snapshot_digest_v1(fact)?.into_bytes();
        if fact_by_digest.insert(digest, fact).is_some() {
            return Err(CovariateEncodingV1Error::DuplicateSourceFact);
        }
    }

    let mut required_fact_digests = BTreeSet::new();
    for row in &baseline_matrix.rows {
        for cell in &row.cells {
            if let mycelix_target_trial_baseline_covariate_v1::BaselineMeasurementStateV1::Selected {
                fact_snapshot_digest,
                ..
            } = &cell.state
            {
                required_fact_digests.insert(*fact_snapshot_digest);
            }
        }
    }
    let supplied_fact_digests: BTreeSet<_> = fact_by_digest.keys().copied().collect();
    if required_fact_digests != supplied_fact_digests {
        return Err(CovariateEncodingV1Error::SourceFactSetMismatch);
    }

    let mut rows = Vec::with_capacity(baseline_matrix.rows.len());
    for row in &baseline_matrix.rows {
        let mut cells = Vec::with_capacity(row.cells.len());
        for (cell, column) in row.cells.iter().zip(&baseline_matrix.columns) {
            let policy = policy_by_confounder
                .get(&column.confounder_id)
                .ok_or_else(|| CovariateEncodingV1Error::MissingEncodingPolicy(
                    column.confounder_id.clone(),
                ))?;
            let (selected_fact_snapshot_digest, value) = match &cell.state {
                mycelix_target_trial_baseline_covariate_v1::BaselineMeasurementStateV1::MissingMeasurement => {
                    (None, EncodedCovariateValueV1::Missing)
                }
                mycelix_target_trial_baseline_covariate_v1::BaselineMeasurementStateV1::Selected {
                    fact_snapshot_digest,
                    ..
                } => {
                    let fact = fact_by_digest
                        .get(fact_snapshot_digest)
                        .ok_or(CovariateEncodingV1Error::SourceFactSetMismatch)?;
                    if fact.subject.resource_type != row.subject_resource_type
                        || fact.subject.id != row.subject_id
                    {
                        return Err(CovariateEncodingV1Error::SourceFactSubjectMismatch {
                            confounder_id: cell.confounder_id.clone(),
                            subject_id: row.subject_id.clone(),
                        });
                    }
                    let encoded = encode_fact_value(fact, &policy.rule, &cell.confounder_id)?;
                    (Some(*fact_snapshot_digest), encoded)
                }
            };
            cells.push(EncodedCovariateCellV1 {
                confounder_id: cell.confounder_id.clone(),
                measurement_receipt_digest: cell.measurement_receipt_digest,
                selected_fact_snapshot_digest,
                value,
            });
        }
        rows.push(EncodedCovariateRowV1 {
            subject_resource_type: row.subject_resource_type.clone(),
            subject_id: row.subject_id.clone(),
            strategy_id: row.strategy_id.clone(),
            contribution_digest: row.contribution_digest,
            time_zero_receipt_digest: row.time_zero_receipt_digest,
            cells,
        });
    }

    let encoded = EncodedBaselineCovariateMatrixV1 {
        schema_version: TARGET_TRIAL_COVARIATE_ENCODING_V1_VERSION,
        baseline_matrix_digest,
        protocol_digest: baseline_matrix.protocol_digest,
        emulation_plan_digest: baseline_matrix.emulation_plan_digest,
        estimand_id: baseline_matrix.estimand_id.clone(),
        analysis_manifest_digest: baseline_matrix.analysis_manifest_digest,
        columns,
        rows,
    };
    validate_encoded_matrix_shape(&encoded)?;
    let digest = encoded_baseline_covariate_matrix_digest_v1(&encoded)?;
    Ok(VerifiedEncodedBaselineCovariateMatrixV1 {
        matrix: encoded,
        digest,
    })
}

pub fn verify_encoded_baseline_covariate_matrix_v1(
    baseline: &VerifiedBaselineCovariateMatrixV1,
    policies: &[BaselineCovariateEncodingPolicyV1],
    source_facts: &[ClinicalFact],
    supplied: &EncodedBaselineCovariateMatrixV1,
) -> Result<VerifiedEncodedBaselineCovariateMatrixV1, CovariateEncodingV1Error> {
    validate_encoded_matrix_shape(supplied)?;
    let rebuilt = build_encoded_baseline_covariate_matrix_v1(baseline, policies, source_facts)?;
    let supplied_digest = encoded_baseline_covariate_matrix_digest_v1(supplied)?;
    if rebuilt.digest != supplied_digest {
        return Err(CovariateEncodingV1Error::EncodedMatrixMismatch);
    }
    Ok(VerifiedEncodedBaselineCovariateMatrixV1 {
        matrix: supplied.clone(),
        digest: supplied_digest,
    })
}

pub fn encoded_baseline_covariate_matrix_digest_v1(
    matrix: &EncodedBaselineCovariateMatrixV1,
) -> Result<EncodedBaselineCovariateMatrixDigestV1, CovariateEncodingV1Error> {
    validate_encoded_matrix_shape(matrix)?;
    let mut writer = CanonicalWriter::default();
    encode_matrix(&mut writer, matrix)?;
    Ok(EncodedBaselineCovariateMatrixDigestV1(domain_digest(
        MATRIX_CONTEXT,
        MATRIX_TAG,
        &writer.finish(),
    )))
}

fn encode_fact_value(
    fact: &ClinicalFact,
    rule: &CovariateEncodingRuleV1,
    confounder_id: &str,
) -> Result<EncodedCovariateValueV1, CovariateEncodingV1Error> {
    fact.validate_machine_actionable()?;
    match (rule, &fact.value) {
        (CovariateEncodingRuleV1::Boolean, ClinicalValue::Boolean(value)) => {
            Ok(EncodedCovariateValueV1::Boolean(*value))
        }
        (CovariateEncodingRuleV1::Integer, ClinicalValue::Integer(value)) => {
            Ok(EncodedCovariateValueV1::Integer(*value))
        }
        (CovariateEncodingRuleV1::Decimal, ClinicalValue::Decimal(value)) => {
            if !value.is_finite() {
                return Err(CovariateEncodingV1Error::NonFiniteAnalysisValue);
            }
            Ok(EncodedCovariateValueV1::DecimalBits(value.to_bits()))
        }
        (
            CovariateEncodingRuleV1::QuantityUcumExact { unit_code },
            ClinicalValue::Quantity(quantity),
        ) => {
            if quantity.system != UCUM_SYSTEM || quantity.code != *unit_code {
                return Err(CovariateEncodingV1Error::QuantityUnitMismatch {
                    confounder_id: confounder_id.to_string(),
                });
            }
            if !quantity.value.is_finite() {
                return Err(CovariateEncodingV1Error::NonFiniteAnalysisValue);
            }
            Ok(EncodedCovariateValueV1::QuantityUcumExact {
                value_bits: quantity.value.to_bits(),
                unit_code: unit_code.clone(),
            })
        }
        _ => Err(CovariateEncodingV1Error::ClinicalValueVariantMismatch {
            confounder_id: confounder_id.to_string(),
        }),
    }
}

fn validate_policy(
    policy: &BaselineCovariateEncodingPolicyV1,
) -> Result<(), CovariateEncodingV1Error> {
    if policy.schema_version != TARGET_TRIAL_COVARIATE_ENCODING_V1_VERSION
        || policy.confounder_id.trim().is_empty()
        || policy.measurement_policy_digest == [0; 32]
    {
        return Err(CovariateEncodingV1Error::InvalidEncodingPolicy);
    }
    if let CovariateEncodingRuleV1::QuantityUcumExact { unit_code } = &policy.rule {
        if unit_code.trim().is_empty() {
            return Err(CovariateEncodingV1Error::InvalidEncodingPolicy);
        }
    }
    validate_artifact(&policy.policy_evidence)
}

fn validate_encoded_matrix_shape(
    matrix: &EncodedBaselineCovariateMatrixV1,
) -> Result<(), CovariateEncodingV1Error> {
    if matrix.schema_version != TARGET_TRIAL_COVARIATE_ENCODING_V1_VERSION
        || matrix.baseline_matrix_digest == [0; 32]
        || matrix.protocol_digest == [0; 32]
        || matrix.emulation_plan_digest == [0; 32]
        || matrix.analysis_manifest_digest == [0; 32]
        || matrix.estimand_id.trim().is_empty()
        || matrix.columns.is_empty()
        || matrix.rows.is_empty()
    {
        return Err(CovariateEncodingV1Error::InvalidEncodedMatrix);
    }

    let mut previous_column: Option<&str> = None;
    let mut columns = HashSet::new();
    for column in &matrix.columns {
        if column.confounder_id.trim().is_empty()
            || column.measurement_policy_digest == [0; 32]
            || column.encoding_policy_digest == [0; 32]
        {
            return Err(CovariateEncodingV1Error::InvalidEncodedMatrix);
        }
        if let Some(previous) = previous_column {
            if previous >= column.confounder_id.as_str() {
                return Err(CovariateEncodingV1Error::NonCanonicalColumnOrder);
            }
        }
        previous_column = Some(column.confounder_id.as_str());
        if !columns.insert(column.confounder_id.clone()) {
            return Err(CovariateEncodingV1Error::InvalidEncodedMatrix);
        }
    }

    let mut previous_subject: Option<(&str, &str)> = None;
    for row in &matrix.rows {
        if row.subject_resource_type.trim().is_empty()
            || row.subject_id.trim().is_empty()
            || row.strategy_id.trim().is_empty()
            || row.contribution_digest == [0; 32]
            || row.time_zero_receipt_digest == [0; 32]
            || row.cells.len() != matrix.columns.len()
        {
            return Err(CovariateEncodingV1Error::InvalidEncodedMatrix);
        }
        let subject = (row.subject_resource_type.as_str(), row.subject_id.as_str());
        if let Some(previous) = previous_subject {
            if previous >= subject {
                return Err(CovariateEncodingV1Error::NonCanonicalRowOrder);
            }
        }
        previous_subject = Some(subject);
        for (cell, column) in row.cells.iter().zip(&matrix.columns) {
            if cell.confounder_id != column.confounder_id
                || cell.measurement_receipt_digest == [0; 32]
            {
                return Err(CovariateEncodingV1Error::InvalidEncodedMatrix);
            }
            match (&cell.selected_fact_snapshot_digest, &cell.value) {
                (None, EncodedCovariateValueV1::Missing) => {}
                (Some(digest), EncodedCovariateValueV1::Missing) if *digest != [0; 32] => {
                    return Err(CovariateEncodingV1Error::InvalidEncodedMatrix);
                }
                (Some(digest), _) if *digest != [0; 32] => {}
                _ => return Err(CovariateEncodingV1Error::InvalidEncodedMatrix),
            }
        }
    }
    Ok(())
}

fn validate_artifact(value: &EvidenceArtifactIdentityV2) -> Result<(), CovariateEncodingV1Error> {
    if value.namespace.trim().is_empty()
        || value.artifact_id.trim().is_empty()
        || value.version.trim().is_empty()
        || value.digest == [0; 32]
    {
        return Err(CovariateEncodingV1Error::InvalidEvidenceIdentity);
    }
    Ok(())
}

fn domain_digest(context: &str, tag: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&TARGET_TRIAL_COVARIATE_ENCODING_V1_VERSION.to_be_bytes());
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
    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn string(&mut self, value: &str) -> Result<(), CovariateEncodingV1Error> {
        self.bytes(value.as_bytes())
    }
    fn bytes(&mut self, value: &[u8]) -> Result<(), CovariateEncodingV1Error> {
        self.u32(u32::try_from(value.len()).map_err(|_| CovariateEncodingV1Error::LengthOverflow)?);
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn vector_len(&mut self, len: usize) -> Result<(), CovariateEncodingV1Error> {
        self.u32(u32::try_from(len).map_err(|_| CovariateEncodingV1Error::LengthOverflow)?);
        Ok(())
    }
}

fn encode_artifact(
    writer: &mut CanonicalWriter,
    value: &EvidenceArtifactIdentityV2,
) -> Result<(), CovariateEncodingV1Error> {
    writer.string(&value.namespace)?;
    writer.string(&value.artifact_id)?;
    writer.string(&value.version)?;
    writer.bytes(&value.digest)
}

fn encode_rule(
    writer: &mut CanonicalWriter,
    value: &CovariateEncodingRuleV1,
) -> Result<(), CovariateEncodingV1Error> {
    match value {
        CovariateEncodingRuleV1::Boolean => writer.u8(0),
        CovariateEncodingRuleV1::Integer => writer.u8(1),
        CovariateEncodingRuleV1::Decimal => writer.u8(2),
        CovariateEncodingRuleV1::QuantityUcumExact { unit_code } => {
            writer.u8(3);
            writer.string(unit_code)?;
        }
    }
    Ok(())
}

fn encode_policy(
    writer: &mut CanonicalWriter,
    value: &BaselineCovariateEncodingPolicyV1,
) -> Result<(), CovariateEncodingV1Error> {
    writer.u16(value.schema_version);
    writer.string(&value.confounder_id)?;
    writer.bytes(&value.measurement_policy_digest)?;
    encode_rule(writer, &value.rule)?;
    encode_artifact(writer, &value.policy_evidence)
}

fn encode_value(
    writer: &mut CanonicalWriter,
    value: &EncodedCovariateValueV1,
) -> Result<(), CovariateEncodingV1Error> {
    match value {
        EncodedCovariateValueV1::Missing => writer.u8(0),
        EncodedCovariateValueV1::Boolean(value) => {
            writer.u8(1);
            writer.u8(u8::from(*value));
        }
        EncodedCovariateValueV1::Integer(value) => {
            writer.u8(2);
            writer.i64(*value);
        }
        EncodedCovariateValueV1::DecimalBits(bits) => {
            writer.u8(3);
            writer.u64(*bits);
        }
        EncodedCovariateValueV1::QuantityUcumExact {
            value_bits,
            unit_code,
        } => {
            writer.u8(4);
            writer.u64(*value_bits);
            writer.string(unit_code)?;
        }
    }
    Ok(())
}

fn encode_matrix(
    writer: &mut CanonicalWriter,
    matrix: &EncodedBaselineCovariateMatrixV1,
) -> Result<(), CovariateEncodingV1Error> {
    writer.u16(matrix.schema_version);
    writer.bytes(&matrix.baseline_matrix_digest)?;
    writer.bytes(&matrix.protocol_digest)?;
    writer.bytes(&matrix.emulation_plan_digest)?;
    writer.string(&matrix.estimand_id)?;
    writer.bytes(&matrix.analysis_manifest_digest)?;
    writer.vector_len(matrix.columns.len())?;
    for column in &matrix.columns {
        writer.string(&column.confounder_id)?;
        writer.bytes(&column.measurement_policy_digest)?;
        writer.bytes(&column.encoding_policy_digest)?;
    }
    writer.vector_len(matrix.rows.len())?;
    for row in &matrix.rows {
        writer.string(&row.subject_resource_type)?;
        writer.string(&row.subject_id)?;
        writer.string(&row.strategy_id)?;
        writer.bytes(&row.contribution_digest)?;
        writer.bytes(&row.time_zero_receipt_digest)?;
        writer.vector_len(row.cells.len())?;
        for cell in &row.cells {
            writer.string(&cell.confounder_id)?;
            writer.bytes(&cell.measurement_receipt_digest)?;
            match cell.selected_fact_snapshot_digest {
                Some(digest) => {
                    writer.u8(1);
                    writer.bytes(&digest)?;
                }
                None => writer.u8(0),
            }
            encode_value(writer, &cell.value)?;
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum CovariateEncodingV1Error {
    #[error("encoding policy is invalid")]
    InvalidEncodingPolicy,
    #[error("evidence artifact identity is invalid")]
    InvalidEvidenceIdentity,
    #[error("duplicate encoding policy for confounder: {0}")]
    DuplicateEncodingPolicy(String),
    #[error("encoding-policy column set differs from the baseline matrix")]
    EncodingPolicySetMismatch,
    #[error("missing encoding policy for confounder: {0}")]
    MissingEncodingPolicy(String),
    #[error("encoding policy binds a different baseline measurement policy: {0}")]
    MeasurementPolicyDigestMismatch(String),
    #[error("duplicate source ClinicalFact snapshot supplied")]
    DuplicateSourceFact,
    #[error("source ClinicalFact set differs from the matrix selected-fact set")]
    SourceFactSetMismatch,
    #[error("source fact subject differs from matrix row for {subject_id}/{confounder_id}")]
    SourceFactSubjectMismatch {
        confounder_id: String,
        subject_id: String,
    },
    #[error("clinical value variant does not match encoding policy for {confounder_id}")]
    ClinicalValueVariantMismatch { confounder_id: String },
    #[error("quantity UCUM unit differs from encoding policy for {confounder_id}")]
    QuantityUnitMismatch { confounder_id: String },
    #[error("analysis value is non-finite")]
    NonFiniteAnalysisValue,
    #[error("encoded matrix is malformed")]
    InvalidEncodedMatrix,
    #[error("encoded matrix columns are not strictly canonical")]
    NonCanonicalColumnOrder,
    #[error("encoded matrix rows are not strictly canonical")]
    NonCanonicalRowOrder,
    #[error("encoded matrix does not reproduce from the supplied evidence")]
    EncodedMatrixMismatch,
    #[error("canonical framing length overflow")]
    LengthOverflow,
    #[error(transparent)]
    BaselineMatrix(#[from] BaselineMatrixV1Error),
    #[error(transparent)]
    Snapshot(#[from] ClinicalFactSnapshotError),
    #[error(transparent)]
    Clinical(#[from] mycelix_clinical_semantics::ClinicalSemanticsError),
}
