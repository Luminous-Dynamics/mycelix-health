#![deny(unsafe_code)]
//! Research-only baseline covariate missingness diagnostics v1.
//!
//! This crate reports missingness in one exact verified encoded baseline matrix.
//! It does not decide whether missingness is acceptable, remove incomplete rows,
//! impute values, infer a missingness mechanism, or authorize causal estimation.

use mycelix_target_trial_covariate_encoding_v1::{
    EncodedCovariateValueV1, VerifiedEncodedBaselineCovariateMatrixV1,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

pub const TARGET_TRIAL_MISSINGNESS_DIAGNOSTICS_V1_VERSION: u16 = 1;

const DIAGNOSTIC_TAG: &[u8] = b"mycelix/target-trial-missingness-diagnostics/v1";
const DIAGNOSTIC_CONTEXT: &str = "mycelix.health.target-trial-missingness-diagnostics.v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategyMissingnessV1 {
    pub strategy_id: String,
    pub subject_count: u64,
    pub complete_subject_count: u64,
    pub incomplete_subject_count: u64,
    pub present_cell_count: u64,
    pub missing_cell_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfounderStrategyMissingnessV1 {
    pub strategy_id: String,
    pub present_cell_count: u64,
    pub missing_cell_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfounderMissingnessV1 {
    pub confounder_id: String,
    pub present_cell_count: u64,
    pub missing_cell_count: u64,
    pub by_strategy: Vec<ConfounderStrategyMissingnessV1>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MissingnessDiagnosticsV1 {
    pub schema_version: u16,
    pub encoded_matrix_digest: [u8; 32],
    pub protocol_digest: [u8; 32],
    pub emulation_plan_digest: [u8; 32],
    pub estimand_id: String,
    pub analysis_manifest_digest: [u8; 32],
    pub subject_count: u64,
    pub confounder_count: u64,
    pub total_cell_count: u64,
    pub present_cell_count: u64,
    pub missing_cell_count: u64,
    pub complete_subject_count: u64,
    pub incomplete_subject_count: u64,
    pub by_strategy: Vec<StrategyMissingnessV1>,
    pub by_confounder: Vec<ConfounderMissingnessV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MissingnessDiagnosticsDigestV1([u8; 32]);

impl MissingnessDiagnosticsDigestV1 {
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

pub struct VerifiedMissingnessDiagnosticsV1 {
    diagnostics: MissingnessDiagnosticsV1,
    digest: MissingnessDiagnosticsDigestV1,
}

impl VerifiedMissingnessDiagnosticsV1 {
    #[must_use]
    pub fn diagnostics(&self) -> &MissingnessDiagnosticsV1 {
        &self.diagnostics
    }

    #[must_use]
    pub const fn digest(&self) -> MissingnessDiagnosticsDigestV1 {
        self.digest
    }
}

#[derive(Default)]
struct StrategyAccumulator {
    subject_count: u64,
    complete_subject_count: u64,
    incomplete_subject_count: u64,
    present_cell_count: u64,
    missing_cell_count: u64,
}

#[derive(Default)]
struct ConfounderAccumulator {
    present_cell_count: u64,
    missing_cell_count: u64,
    by_strategy: BTreeMap<String, CellAccumulator>,
}

#[derive(Default)]
struct CellAccumulator {
    present_cell_count: u64,
    missing_cell_count: u64,
}

pub fn build_missingness_diagnostics_v1(
    encoded: &VerifiedEncodedBaselineCovariateMatrixV1,
) -> Result<VerifiedMissingnessDiagnosticsV1, MissingnessDiagnosticsV1Error> {
    let matrix = encoded.matrix();
    let subject_count = u64::try_from(matrix.rows.len())
        .map_err(|_| MissingnessDiagnosticsV1Error::CountOverflow)?;
    let confounder_count = u64::try_from(matrix.columns.len())
        .map_err(|_| MissingnessDiagnosticsV1Error::CountOverflow)?;
    let total_cell_count = subject_count
        .checked_mul(confounder_count)
        .ok_or(MissingnessDiagnosticsV1Error::CountOverflow)?;

    let mut present_cell_count = 0_u64;
    let mut missing_cell_count = 0_u64;
    let mut complete_subject_count = 0_u64;
    let mut incomplete_subject_count = 0_u64;
    let mut strategy_accumulators: BTreeMap<String, StrategyAccumulator> = BTreeMap::new();
    let mut confounder_accumulators: BTreeMap<String, ConfounderAccumulator> = matrix
        .columns
        .iter()
        .map(|column| (column.confounder_id.clone(), ConfounderAccumulator::default()))
        .collect();

    for row in &matrix.rows {
        let mut row_missing = 0_u64;
        let strategy = strategy_accumulators
            .entry(row.strategy_id.clone())
            .or_default();
        strategy.subject_count = checked_inc(strategy.subject_count)?;

        for (cell, column) in row.cells.iter().zip(&matrix.columns) {
            let confounder = confounder_accumulators
                .get_mut(&column.confounder_id)
                .ok_or(MissingnessDiagnosticsV1Error::InternalCoordinateMismatch)?;
            let confounder_strategy = confounder
                .by_strategy
                .entry(row.strategy_id.clone())
                .or_default();

            if matches!(&cell.value, EncodedCovariateValueV1::Missing) {
                missing_cell_count = checked_inc(missing_cell_count)?;
                row_missing = checked_inc(row_missing)?;
                strategy.missing_cell_count = checked_inc(strategy.missing_cell_count)?;
                confounder.missing_cell_count = checked_inc(confounder.missing_cell_count)?;
                confounder_strategy.missing_cell_count =
                    checked_inc(confounder_strategy.missing_cell_count)?;
            } else {
                present_cell_count = checked_inc(present_cell_count)?;
                strategy.present_cell_count = checked_inc(strategy.present_cell_count)?;
                confounder.present_cell_count = checked_inc(confounder.present_cell_count)?;
                confounder_strategy.present_cell_count =
                    checked_inc(confounder_strategy.present_cell_count)?;
            }
        }

        if row_missing == 0 {
            complete_subject_count = checked_inc(complete_subject_count)?;
            strategy.complete_subject_count = checked_inc(strategy.complete_subject_count)?;
        } else {
            incomplete_subject_count = checked_inc(incomplete_subject_count)?;
            strategy.incomplete_subject_count = checked_inc(strategy.incomplete_subject_count)?;
        }
    }

    let by_strategy = strategy_accumulators
        .into_iter()
        .map(|(strategy_id, value)| StrategyMissingnessV1 {
            strategy_id,
            subject_count: value.subject_count,
            complete_subject_count: value.complete_subject_count,
            incomplete_subject_count: value.incomplete_subject_count,
            present_cell_count: value.present_cell_count,
            missing_cell_count: value.missing_cell_count,
        })
        .collect();

    let by_confounder = confounder_accumulators
        .into_iter()
        .map(|(confounder_id, value)| ConfounderMissingnessV1 {
            confounder_id,
            present_cell_count: value.present_cell_count,
            missing_cell_count: value.missing_cell_count,
            by_strategy: value
                .by_strategy
                .into_iter()
                .map(|(strategy_id, counts)| ConfounderStrategyMissingnessV1 {
                    strategy_id,
                    present_cell_count: counts.present_cell_count,
                    missing_cell_count: counts.missing_cell_count,
                })
                .collect(),
        })
        .collect();

    let diagnostics = MissingnessDiagnosticsV1 {
        schema_version: TARGET_TRIAL_MISSINGNESS_DIAGNOSTICS_V1_VERSION,
        encoded_matrix_digest: encoded.digest().into_bytes(),
        protocol_digest: matrix.protocol_digest,
        emulation_plan_digest: matrix.emulation_plan_digest,
        estimand_id: matrix.estimand_id.clone(),
        analysis_manifest_digest: matrix.analysis_manifest_digest,
        subject_count,
        confounder_count,
        total_cell_count,
        present_cell_count,
        missing_cell_count,
        complete_subject_count,
        incomplete_subject_count,
        by_strategy,
        by_confounder,
    };
    validate_missingness_diagnostics_v1(&diagnostics)?;
    let digest = missingness_diagnostics_digest_v1(&diagnostics)?;
    Ok(VerifiedMissingnessDiagnosticsV1 {
        diagnostics,
        digest,
    })
}

pub fn verify_missingness_diagnostics_v1(
    encoded: &VerifiedEncodedBaselineCovariateMatrixV1,
    supplied: &MissingnessDiagnosticsV1,
) -> Result<VerifiedMissingnessDiagnosticsV1, MissingnessDiagnosticsV1Error> {
    validate_missingness_diagnostics_v1(supplied)?;
    let rebuilt = build_missingness_diagnostics_v1(encoded)?;
    let supplied_digest = missingness_diagnostics_digest_v1(supplied)?;
    if rebuilt.digest != supplied_digest {
        return Err(MissingnessDiagnosticsV1Error::DiagnosticsMismatch);
    }
    Ok(VerifiedMissingnessDiagnosticsV1 {
        diagnostics: supplied.clone(),
        digest: supplied_digest,
    })
}

pub fn missingness_diagnostics_digest_v1(
    diagnostics: &MissingnessDiagnosticsV1,
) -> Result<MissingnessDiagnosticsDigestV1, MissingnessDiagnosticsV1Error> {
    validate_missingness_diagnostics_v1(diagnostics)?;
    let mut writer = CanonicalWriter::default();
    encode_diagnostics(&mut writer, diagnostics)?;
    Ok(MissingnessDiagnosticsDigestV1(domain_digest(
        DIAGNOSTIC_CONTEXT,
        DIAGNOSTIC_TAG,
        &writer.finish(),
    )))
}

pub fn validate_missingness_diagnostics_v1(
    value: &MissingnessDiagnosticsV1,
) -> Result<(), MissingnessDiagnosticsV1Error> {
    if value.schema_version != TARGET_TRIAL_MISSINGNESS_DIAGNOSTICS_V1_VERSION
        || value.encoded_matrix_digest == [0; 32]
        || value.protocol_digest == [0; 32]
        || value.emulation_plan_digest == [0; 32]
        || value.analysis_manifest_digest == [0; 32]
        || value.estimand_id.trim().is_empty()
        || value.subject_count == 0
        || value.confounder_count == 0
        || value.by_strategy.is_empty()
        || value.by_confounder.is_empty()
    {
        return Err(MissingnessDiagnosticsV1Error::InvalidDiagnostics);
    }

    let expected_total = value
        .subject_count
        .checked_mul(value.confounder_count)
        .ok_or(MissingnessDiagnosticsV1Error::CountOverflow)?;
    if value.total_cell_count != expected_total
        || checked_add(value.present_cell_count, value.missing_cell_count)? != expected_total
        || checked_add(value.complete_subject_count, value.incomplete_subject_count)?
            != value.subject_count
        || u64::try_from(value.by_confounder.len())
            .map_err(|_| MissingnessDiagnosticsV1Error::CountOverflow)?
            != value.confounder_count
    {
        return Err(MissingnessDiagnosticsV1Error::InconsistentCounts);
    }

    validate_strategy_totals(value)?;
    validate_confounder_totals(value)?;
    Ok(())
}

fn validate_strategy_totals(
    value: &MissingnessDiagnosticsV1,
) -> Result<(), MissingnessDiagnosticsV1Error> {
    let mut previous: Option<&str> = None;
    let mut subject_sum = 0_u64;
    let mut complete_sum = 0_u64;
    let mut incomplete_sum = 0_u64;
    let mut present_sum = 0_u64;
    let mut missing_sum = 0_u64;

    for strategy in &value.by_strategy {
        if strategy.strategy_id.trim().is_empty() || strategy.subject_count == 0 {
            return Err(MissingnessDiagnosticsV1Error::InvalidDiagnostics);
        }
        if let Some(previous_id) = previous {
            if previous_id >= strategy.strategy_id.as_str() {
                return Err(MissingnessDiagnosticsV1Error::NonCanonicalStrategyOrder);
            }
        }
        previous = Some(strategy.strategy_id.as_str());
        if checked_add(
            strategy.complete_subject_count,
            strategy.incomplete_subject_count,
        )? != strategy.subject_count
        {
            return Err(MissingnessDiagnosticsV1Error::InconsistentCounts);
        }
        let expected_cells = strategy
            .subject_count
            .checked_mul(value.confounder_count)
            .ok_or(MissingnessDiagnosticsV1Error::CountOverflow)?;
        if checked_add(strategy.present_cell_count, strategy.missing_cell_count)?
            != expected_cells
        {
            return Err(MissingnessDiagnosticsV1Error::InconsistentCounts);
        }
        subject_sum = checked_add(subject_sum, strategy.subject_count)?;
        complete_sum = checked_add(complete_sum, strategy.complete_subject_count)?;
        incomplete_sum = checked_add(incomplete_sum, strategy.incomplete_subject_count)?;
        present_sum = checked_add(present_sum, strategy.present_cell_count)?;
        missing_sum = checked_add(missing_sum, strategy.missing_cell_count)?;
    }

    if subject_sum != value.subject_count
        || complete_sum != value.complete_subject_count
        || incomplete_sum != value.incomplete_subject_count
        || present_sum != value.present_cell_count
        || missing_sum != value.missing_cell_count
    {
        return Err(MissingnessDiagnosticsV1Error::InconsistentCounts);
    }
    Ok(())
}

fn validate_confounder_totals(
    value: &MissingnessDiagnosticsV1,
) -> Result<(), MissingnessDiagnosticsV1Error> {
    let strategy_ids: Vec<&str> = value
        .by_strategy
        .iter()
        .map(|strategy| strategy.strategy_id.as_str())
        .collect();
    let mut previous: Option<&str> = None;
    let mut present_sum = 0_u64;
    let mut missing_sum = 0_u64;

    for confounder in &value.by_confounder {
        if confounder.confounder_id.trim().is_empty()
            || checked_add(confounder.present_cell_count, confounder.missing_cell_count)?
                != value.subject_count
            || confounder.by_strategy.len() != value.by_strategy.len()
        {
            return Err(MissingnessDiagnosticsV1Error::InconsistentCounts);
        }
        if let Some(previous_id) = previous {
            if previous_id >= confounder.confounder_id.as_str() {
                return Err(MissingnessDiagnosticsV1Error::NonCanonicalConfounderOrder);
            }
        }
        previous = Some(confounder.confounder_id.as_str());

        let mut confounder_present = 0_u64;
        let mut confounder_missing = 0_u64;
        for ((nested, expected_strategy_id), strategy_total) in confounder
            .by_strategy
            .iter()
            .zip(&strategy_ids)
            .zip(&value.by_strategy)
        {
            if nested.strategy_id != *expected_strategy_id
                || checked_add(nested.present_cell_count, nested.missing_cell_count)?
                    != strategy_total.subject_count
            {
                return Err(MissingnessDiagnosticsV1Error::InconsistentCounts);
            }
            confounder_present = checked_add(confounder_present, nested.present_cell_count)?;
            confounder_missing = checked_add(confounder_missing, nested.missing_cell_count)?;
        }
        if confounder_present != confounder.present_cell_count
            || confounder_missing != confounder.missing_cell_count
        {
            return Err(MissingnessDiagnosticsV1Error::InconsistentCounts);
        }
        present_sum = checked_add(present_sum, confounder.present_cell_count)?;
        missing_sum = checked_add(missing_sum, confounder.missing_cell_count)?;
    }

    if present_sum != value.present_cell_count || missing_sum != value.missing_cell_count {
        return Err(MissingnessDiagnosticsV1Error::InconsistentCounts);
    }
    Ok(())
}

fn checked_inc(value: u64) -> Result<u64, MissingnessDiagnosticsV1Error> {
    value
        .checked_add(1)
        .ok_or(MissingnessDiagnosticsV1Error::CountOverflow)
}

fn checked_add(left: u64, right: u64) -> Result<u64, MissingnessDiagnosticsV1Error> {
    left.checked_add(right)
        .ok_or(MissingnessDiagnosticsV1Error::CountOverflow)
}

fn domain_digest(context: &str, tag: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&TARGET_TRIAL_MISSINGNESS_DIAGNOSTICS_V1_VERSION.to_be_bytes());
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
    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn bytes(&mut self, value: &[u8]) -> Result<(), MissingnessDiagnosticsV1Error> {
        self.u32(u32::try_from(value.len()).map_err(|_| MissingnessDiagnosticsV1Error::LengthOverflow)?);
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn string(&mut self, value: &str) -> Result<(), MissingnessDiagnosticsV1Error> {
        self.bytes(value.as_bytes())
    }
    fn vector_len(&mut self, len: usize) -> Result<(), MissingnessDiagnosticsV1Error> {
        self.u32(u32::try_from(len).map_err(|_| MissingnessDiagnosticsV1Error::LengthOverflow)?);
        Ok(())
    }
}

fn encode_diagnostics(
    writer: &mut CanonicalWriter,
    value: &MissingnessDiagnosticsV1,
) -> Result<(), MissingnessDiagnosticsV1Error> {
    writer.u16(value.schema_version);
    writer.bytes(&value.encoded_matrix_digest)?;
    writer.bytes(&value.protocol_digest)?;
    writer.bytes(&value.emulation_plan_digest)?;
    writer.string(&value.estimand_id)?;
    writer.bytes(&value.analysis_manifest_digest)?;
    writer.u64(value.subject_count);
    writer.u64(value.confounder_count);
    writer.u64(value.total_cell_count);
    writer.u64(value.present_cell_count);
    writer.u64(value.missing_cell_count);
    writer.u64(value.complete_subject_count);
    writer.u64(value.incomplete_subject_count);
    writer.vector_len(value.by_strategy.len())?;
    for strategy in &value.by_strategy {
        writer.string(&strategy.strategy_id)?;
        writer.u64(strategy.subject_count);
        writer.u64(strategy.complete_subject_count);
        writer.u64(strategy.incomplete_subject_count);
        writer.u64(strategy.present_cell_count);
        writer.u64(strategy.missing_cell_count);
    }
    writer.vector_len(value.by_confounder.len())?;
    for confounder in &value.by_confounder {
        writer.string(&confounder.confounder_id)?;
        writer.u64(confounder.present_cell_count);
        writer.u64(confounder.missing_cell_count);
        writer.vector_len(confounder.by_strategy.len())?;
        for nested in &confounder.by_strategy {
            writer.string(&nested.strategy_id)?;
            writer.u64(nested.present_cell_count);
            writer.u64(nested.missing_cell_count);
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum MissingnessDiagnosticsV1Error {
    #[error("missingness diagnostic is malformed")]
    InvalidDiagnostics,
    #[error("missingness counts are internally inconsistent")]
    InconsistentCounts,
    #[error("strategy rows are not in strict canonical order")]
    NonCanonicalStrategyOrder,
    #[error("confounder rows are not in strict canonical order")]
    NonCanonicalConfounderOrder,
    #[error("internal encoded-matrix coordinate mismatch")]
    InternalCoordinateMismatch,
    #[error("supplied missingness diagnostic does not reproduce from the encoded matrix")]
    DiagnosticsMismatch,
    #[error("missingness count overflow")]
    CountOverflow,
    #[error("canonical framing length overflow")]
    LengthOverflow,
}
