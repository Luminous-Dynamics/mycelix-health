#![deny(unsafe_code)]
//! Research-only baseline covariate identity/missingness matrix v1.
//!
//! This layer binds an exact fixed-horizon analysis contribution manifest to
//! exact verified baseline-confounder measurement receipts. It intentionally
//! does not turn selected clinical facts into numeric/categorical analysis
//! features and never imputes missing measurements.

use mycelix_target_trial_analysis_contribution_v1::{
    build_fixed_horizon_analysis_manifest_v1, fixed_horizon_analysis_manifest_digest_v1,
    AnalysisContributionV1Error, VerifiedFixedHorizonContributionV1,
};
use mycelix_target_trial_baseline_covariate_v1::{
    BaselineMeasurementStateV1, VerifiedBaselineCovariateMeasurementV1,
};
use mycelix_target_trial_protocol_v2::{
    target_trial_emulation_plan_digest_v2, target_trial_protocol_digest_v2,
    EvidenceArtifactIdentityV2, TargetTrialEmulationPlanV2, TargetTrialProtocolV2,
    TargetTrialV2Error,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use thiserror::Error;

pub const TARGET_TRIAL_BASELINE_MATRIX_V1_VERSION: u16 = 1;

const MATRIX_TAG: &[u8] = b"mycelix/target-trial-baseline-covariate-matrix/v1";
const MATRIX_CONTEXT: &str = "mycelix.health.target-trial-baseline-covariate-matrix.v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineCovariateColumnV1 {
    pub confounder_id: String,
    pub protocol_definition: EvidenceArtifactIdentityV2,
    pub measurement_policy_digest: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineCovariateCellV1 {
    pub confounder_id: String,
    pub measurement_receipt_digest: [u8; 32],
    pub state: BaselineMeasurementStateV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineCovariateRowV1 {
    pub subject_resource_type: String,
    pub subject_id: String,
    pub strategy_id: String,
    pub contribution_digest: [u8; 32],
    pub time_zero_receipt_digest: [u8; 32],
    pub cells: Vec<BaselineCovariateCellV1>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineCovariateMatrixV1 {
    pub schema_version: u16,
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub estimand_id: String,
    pub analysis_manifest_digest: [u8; 32],
    pub columns: Vec<BaselineCovariateColumnV1>,
    pub rows: Vec<BaselineCovariateRowV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BaselineCovariateMatrixDigestV1([u8; 32]);

impl BaselineCovariateMatrixDigestV1 {
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

pub struct VerifiedBaselineCovariateMatrixV1 {
    matrix: BaselineCovariateMatrixV1,
    digest: BaselineCovariateMatrixDigestV1,
}

impl VerifiedBaselineCovariateMatrixV1 {
    #[must_use]
    pub fn matrix(&self) -> &BaselineCovariateMatrixV1 {
        &self.matrix
    }

    #[must_use]
    pub const fn digest(&self) -> BaselineCovariateMatrixDigestV1 {
        self.digest
    }

    #[must_use]
    pub fn missing_measurement_count(&self) -> usize {
        self.matrix
            .rows
            .iter()
            .flat_map(|row| row.cells.iter())
            .filter(|cell| matches!(cell.state, BaselineMeasurementStateV1::MissingMeasurement))
            .count()
    }
}

pub fn build_baseline_covariate_matrix_v1(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    contributions: &[VerifiedFixedHorizonContributionV1],
    measurements: &[VerifiedBaselineCovariateMeasurementV1],
) -> Result<VerifiedBaselineCovariateMatrixV1, BaselineMatrixV1Error> {
    plan.validate_against(protocol)?;
    if plan.baseline_confounders.is_empty() {
        return Err(BaselineMatrixV1Error::NoBaselineConfounders);
    }

    let protocol_digest = target_trial_protocol_digest_v2(protocol)?.into_bytes();
    let emulation_plan_digest =
        target_trial_emulation_plan_digest_v2(protocol, plan)?.into_bytes();
    let analysis_manifest = build_fixed_horizon_analysis_manifest_v1(contributions)?;
    if analysis_manifest.protocol_digest != protocol_digest
        || analysis_manifest.emulation_plan_digest != emulation_plan_digest
    {
        return Err(BaselineMatrixV1Error::AnalysisCoordinatesMismatch);
    }
    let analysis_manifest_digest =
        fixed_horizon_analysis_manifest_digest_v1(&analysis_manifest)?.into_bytes();

    let mut contribution_by_subject = BTreeMap::new();
    for contribution in contributions {
        let receipt = contribution.receipt();
        let key = (
            receipt.subject_resource_type.clone(),
            receipt.subject_id.clone(),
        );
        if contribution_by_subject.insert(key.clone(), contribution).is_some() {
            return Err(BaselineMatrixV1Error::DuplicateSubjectContribution {
                resource_type: key.0,
                id: key.1,
            });
        }
    }

    let mut confounder_definitions = BTreeMap::new();
    for confounder in &plan.baseline_confounders {
        if confounder_definitions
            .insert(confounder.confounder_id.clone(), confounder.definition.clone())
            .is_some()
        {
            return Err(BaselineMatrixV1Error::DuplicateConfounder(
                confounder.confounder_id.clone(),
            ));
        }
    }

    let mut policy_by_confounder: BTreeMap<String, [u8; 32]> = BTreeMap::new();
    let mut cell_by_coordinate: BTreeMap<(String, String, String), BaselineCovariateCellV1> =
        BTreeMap::new();

    for measurement in measurements {
        let receipt = measurement.receipt();
        if receipt.protocol_digest != protocol_digest
            || receipt.emulation_plan_digest != emulation_plan_digest
        {
            return Err(BaselineMatrixV1Error::MeasurementCoordinatesMismatch);
        }
        if !confounder_definitions.contains_key(&receipt.confounder_id) {
            return Err(BaselineMatrixV1Error::UnknownConfounder(
                receipt.confounder_id.clone(),
            ));
        }

        let subject_key = (
            receipt.subject_resource_type.clone(),
            receipt.subject_id.clone(),
        );
        let contribution = contribution_by_subject
            .get(&subject_key)
            .ok_or_else(|| BaselineMatrixV1Error::UnknownMeasurementSubject {
                resource_type: subject_key.0.clone(),
                id: subject_key.1.clone(),
            })?;
        let contribution_receipt = contribution.receipt();
        if receipt.time_zero_receipt_digest != contribution_receipt.time_zero_receipt_digest
            || receipt.time_zero_micros != contribution_receipt.time_zero_micros
        {
            return Err(BaselineMatrixV1Error::MeasurementTimeZeroMismatch {
                resource_type: subject_key.0,
                id: subject_key.1,
                confounder_id: receipt.confounder_id.clone(),
            });
        }

        match policy_by_confounder.get(&receipt.confounder_id) {
            Some(existing) if *existing != receipt.measurement_policy_digest => {
                return Err(BaselineMatrixV1Error::MixedMeasurementPolicy {
                    confounder_id: receipt.confounder_id.clone(),
                });
            }
            None => {
                policy_by_confounder.insert(
                    receipt.confounder_id.clone(),
                    receipt.measurement_policy_digest,
                );
            }
            _ => {}
        }

        let coordinate = (
            receipt.subject_resource_type.clone(),
            receipt.subject_id.clone(),
            receipt.confounder_id.clone(),
        );
        let cell = BaselineCovariateCellV1 {
            confounder_id: receipt.confounder_id.clone(),
            measurement_receipt_digest: measurement.digest().into_bytes(),
            state: receipt.state.clone(),
        };
        if cell_by_coordinate.insert(coordinate.clone(), cell).is_some() {
            return Err(BaselineMatrixV1Error::DuplicateMeasurementCell {
                resource_type: coordinate.0,
                id: coordinate.1,
                confounder_id: coordinate.2,
            });
        }
    }

    let mut columns = Vec::with_capacity(confounder_definitions.len());
    for (confounder_id, definition) in &confounder_definitions {
        let measurement_policy_digest = policy_by_confounder
            .get(confounder_id)
            .copied()
            .ok_or_else(|| BaselineMatrixV1Error::MissingConfounderMeasurements(
                confounder_id.clone(),
            ))?;
        columns.push(BaselineCovariateColumnV1 {
            confounder_id: confounder_id.clone(),
            protocol_definition: definition.clone(),
            measurement_policy_digest,
        });
    }

    let mut rows = Vec::with_capacity(contribution_by_subject.len());
    for ((resource_type, subject_id), contribution) in &contribution_by_subject {
        let contribution_receipt = contribution.receipt();
        let mut cells = Vec::with_capacity(columns.len());
        for column in &columns {
            let coordinate = (
                resource_type.clone(),
                subject_id.clone(),
                column.confounder_id.clone(),
            );
            let cell = cell_by_coordinate
                .get(&coordinate)
                .cloned()
                .ok_or_else(|| BaselineMatrixV1Error::MissingMeasurementCell {
                    resource_type: resource_type.clone(),
                    id: subject_id.clone(),
                    confounder_id: column.confounder_id.clone(),
                })?;
            cells.push(cell);
        }
        rows.push(BaselineCovariateRowV1 {
            subject_resource_type: resource_type.clone(),
            subject_id: subject_id.clone(),
            strategy_id: contribution_receipt.strategy_id.clone(),
            contribution_digest: contribution.digest().into_bytes(),
            time_zero_receipt_digest: contribution_receipt.time_zero_receipt_digest,
            cells,
        });
    }

    let expected_cells = rows
        .len()
        .checked_mul(columns.len())
        .ok_or(BaselineMatrixV1Error::LengthOverflow)?;
    if cell_by_coordinate.len() != expected_cells {
        return Err(BaselineMatrixV1Error::UnexpectedMeasurementCellCount);
    }

    let matrix = BaselineCovariateMatrixV1 {
        schema_version: TARGET_TRIAL_BASELINE_MATRIX_V1_VERSION,
        protocol_digest,
        emulation_plan_digest,
        estimand_id: analysis_manifest.estimand_id,
        analysis_manifest_digest,
        columns,
        rows,
    };
    validate_matrix_shape(&matrix)?;
    let digest = baseline_covariate_matrix_digest_v1(&matrix)?;
    Ok(VerifiedBaselineCovariateMatrixV1 { matrix, digest })
}

pub fn baseline_covariate_matrix_digest_v1(
    matrix: &BaselineCovariateMatrixV1,
) -> Result<BaselineCovariateMatrixDigestV1, BaselineMatrixV1Error> {
    validate_matrix_shape(matrix)?;
    let mut writer = CanonicalWriter::default();
    encode_matrix(&mut writer, matrix)?;
    Ok(BaselineCovariateMatrixDigestV1(domain_digest(
        MATRIX_CONTEXT,
        MATRIX_TAG,
        &writer.finish(),
    )))
}

fn validate_matrix_shape(matrix: &BaselineCovariateMatrixV1) -> Result<(), BaselineMatrixV1Error> {
    if matrix.schema_version != TARGET_TRIAL_BASELINE_MATRIX_V1_VERSION
        || matrix.protocol_digest == [0; 32]
        || matrix.emulation_plan_digest == [0; 32]
        || matrix.analysis_manifest_digest == [0; 32]
        || matrix.estimand_id.trim().is_empty()
        || matrix.columns.is_empty()
        || matrix.rows.is_empty()
    {
        return Err(BaselineMatrixV1Error::InvalidMatrix);
    }

    let mut previous_column: Option<&str> = None;
    let mut column_ids = HashSet::new();
    for column in &matrix.columns {
        if column.confounder_id.trim().is_empty() || column.measurement_policy_digest == [0; 32] {
            return Err(BaselineMatrixV1Error::InvalidMatrix);
        }
        validate_artifact(&column.protocol_definition)?;
        if let Some(previous) = previous_column {
            if previous >= column.confounder_id.as_str() {
                return Err(BaselineMatrixV1Error::NonCanonicalColumnOrder);
            }
        }
        previous_column = Some(column.confounder_id.as_str());
        if !column_ids.insert(column.confounder_id.clone()) {
            return Err(BaselineMatrixV1Error::DuplicateConfounder(
                column.confounder_id.clone(),
            ));
        }
    }

    let mut previous_subject: Option<(&str, &str)> = None;
    let mut subjects = HashSet::new();
    for row in &matrix.rows {
        if row.subject_resource_type.trim().is_empty()
            || row.subject_id.trim().is_empty()
            || row.strategy_id.trim().is_empty()
            || row.contribution_digest == [0; 32]
            || row.time_zero_receipt_digest == [0; 32]
            || row.cells.len() != matrix.columns.len()
        {
            return Err(BaselineMatrixV1Error::InvalidMatrix);
        }
        let subject = (row.subject_resource_type.as_str(), row.subject_id.as_str());
        if let Some(previous) = previous_subject {
            if previous >= subject {
                return Err(BaselineMatrixV1Error::NonCanonicalRowOrder);
            }
        }
        previous_subject = Some(subject);
        if !subjects.insert((row.subject_resource_type.clone(), row.subject_id.clone())) {
            return Err(BaselineMatrixV1Error::DuplicateSubjectContribution {
                resource_type: row.subject_resource_type.clone(),
                id: row.subject_id.clone(),
            });
        }

        for (cell, column) in row.cells.iter().zip(&matrix.columns) {
            if cell.confounder_id != column.confounder_id
                || cell.measurement_receipt_digest == [0; 32]
            {
                return Err(BaselineMatrixV1Error::InvalidMatrix);
            }
            if let BaselineMeasurementStateV1::Selected {
                fact_snapshot_digest,
                ..
            } = &cell.state
            {
                if *fact_snapshot_digest == [0; 32] {
                    return Err(BaselineMatrixV1Error::InvalidMatrix);
                }
            }
        }
    }
    Ok(())
}

fn validate_artifact(value: &EvidenceArtifactIdentityV2) -> Result<(), BaselineMatrixV1Error> {
    if value.namespace.trim().is_empty()
        || value.artifact_id.trim().is_empty()
        || value.version.trim().is_empty()
        || value.digest == [0; 32]
    {
        return Err(BaselineMatrixV1Error::InvalidEvidenceIdentity);
    }
    Ok(())
}

fn domain_digest(context: &str, tag: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&TARGET_TRIAL_BASELINE_MATRIX_V1_VERSION.to_be_bytes());
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

    fn string(&mut self, value: &str) -> Result<(), BaselineMatrixV1Error> {
        self.bytes(value.as_bytes())
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), BaselineMatrixV1Error> {
        self.u32(u32::try_from(value.len()).map_err(|_| BaselineMatrixV1Error::LengthOverflow)?);
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    fn vector_len(&mut self, len: usize) -> Result<(), BaselineMatrixV1Error> {
        self.u32(u32::try_from(len).map_err(|_| BaselineMatrixV1Error::LengthOverflow)?);
        Ok(())
    }
}

fn encode_artifact(
    writer: &mut CanonicalWriter,
    value: &EvidenceArtifactIdentityV2,
) -> Result<(), BaselineMatrixV1Error> {
    writer.string(&value.namespace)?;
    writer.string(&value.artifact_id)?;
    writer.string(&value.version)?;
    writer.bytes(&value.digest)
}

fn encode_state(
    writer: &mut CanonicalWriter,
    value: &BaselineMeasurementStateV1,
) -> Result<(), BaselineMatrixV1Error> {
    match value {
        BaselineMeasurementStateV1::Selected {
            fact_snapshot_digest,
            effective_at_micros,
        } => {
            writer.u8(0);
            writer.bytes(fact_snapshot_digest)?;
            writer.i64(*effective_at_micros);
        }
        BaselineMeasurementStateV1::MissingMeasurement => writer.u8(1),
    }
    Ok(())
}

fn encode_matrix(
    writer: &mut CanonicalWriter,
    matrix: &BaselineCovariateMatrixV1,
) -> Result<(), BaselineMatrixV1Error> {
    writer.u16(matrix.schema_version);
    writer.bytes(&matrix.protocol_digest)?;
    writer.bytes(&matrix.emulation_plan_digest)?;
    writer.string(&matrix.estimand_id)?;
    writer.bytes(&matrix.analysis_manifest_digest)?;
    writer.vector_len(matrix.columns.len())?;
    for column in &matrix.columns {
        writer.string(&column.confounder_id)?;
        encode_artifact(writer, &column.protocol_definition)?;
        writer.bytes(&column.measurement_policy_digest)?;
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
            encode_state(writer, &cell.state)?;
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum BaselineMatrixV1Error {
    #[error("emulation plan declares no baseline confounders")]
    NoBaselineConfounders,
    #[error("analysis contribution manifest does not match the live protocol/emulation plan")]
    AnalysisCoordinatesMismatch,
    #[error("baseline measurement does not match the live protocol/emulation plan")]
    MeasurementCoordinatesMismatch,
    #[error("unknown baseline confounder: {0}")]
    UnknownConfounder(String),
    #[error("duplicate baseline confounder: {0}")]
    DuplicateConfounder(String),
    #[error("measurement references a subject outside the exact analysis manifest: {resource_type}/{id}")]
    UnknownMeasurementSubject { resource_type: String, id: String },
    #[error("measurement time zero differs from the analysis contribution for {resource_type}/{id}/{confounder_id}")]
    MeasurementTimeZeroMismatch {
        resource_type: String,
        id: String,
        confounder_id: String,
    },
    #[error("confounder uses different measurement policies across subjects: {confounder_id}")]
    MixedMeasurementPolicy { confounder_id: String },
    #[error("duplicate measurement cell for {resource_type}/{id}/{confounder_id}")]
    DuplicateMeasurementCell {
        resource_type: String,
        id: String,
        confounder_id: String,
    },
    #[error("no measurement receipts were supplied for declared confounder: {0}")]
    MissingConfounderMeasurements(String),
    #[error("missing matrix cell for {resource_type}/{id}/{confounder_id}")]
    MissingMeasurementCell {
        resource_type: String,
        id: String,
        confounder_id: String,
    },
    #[error("unexpected measurement-cell count")]
    UnexpectedMeasurementCellCount,
    #[error("duplicate subject contribution: {resource_type}/{id}")]
    DuplicateSubjectContribution { resource_type: String, id: String },
    #[error("matrix is malformed")]
    InvalidMatrix,
    #[error("matrix columns are not strictly canonical")]
    NonCanonicalColumnOrder,
    #[error("matrix rows are not strictly canonical")]
    NonCanonicalRowOrder,
    #[error("evidence artifact identity is invalid")]
    InvalidEvidenceIdentity,
    #[error("canonical framing length overflow")]
    LengthOverflow,
    #[error(transparent)]
    Protocol(#[from] TargetTrialV2Error),
    #[error(transparent)]
    Analysis(#[from] AnalysisContributionV1Error),
}
