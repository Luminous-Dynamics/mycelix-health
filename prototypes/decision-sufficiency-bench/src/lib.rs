#![forbid(unsafe_code)]

//! Offline benchmark reference evaluator for evidence sufficiency.
//!
//! This crate is benchmark-only. Its DTOs are not production clinical records,
//! authorization tokens, or a replacement for CARE-001 / canonical Health types.
//! "Verified" means the benchmark fixture asserts that subject binding, issuer
//! authenticity, resource version, temporal validity, semantic support, complete
//! lineage, and purpose-scoped authorization have all been checked independently.

use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceStatus {
    /// All required evidence gates passed for this benchmark assertion.
    Verified,
    Missing,
    Conflicted,
    Expired,
    Unauthorized,
    WrongSubject,
    UnauthenticatedIssuer,
    MissingVersion,
    UnsupportedSemantics,
    IncompleteVerification,
}

impl EvidenceStatus {
    fn is_verified(&self) -> bool {
        matches!(self, Self::Verified)
    }
}

/// Independent evidence dimensions. A record is not verified merely because
/// a producer labels the whole object "Verified": every required dimension must
/// have exactly one passing check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceDimension {
    EvidencePresence,
    SubjectBinding,
    IssuerAuthenticity,
    ResourceVersion,
    TemporalValidity,
    SemanticSupport,
    ProvenanceLineage,
    ApplicationAuthority,
    ConflictReconciliation,
}

pub const REQUIRED_EVIDENCE_DIMENSIONS: [EvidenceDimension; 9] = [
    EvidenceDimension::EvidencePresence,
    EvidenceDimension::SubjectBinding,
    EvidenceDimension::IssuerAuthenticity,
    EvidenceDimension::ResourceVersion,
    EvidenceDimension::TemporalValidity,
    EvidenceDimension::SemanticSupport,
    EvidenceDimension::ProvenanceLineage,
    EvidenceDimension::ApplicationAuthority,
    EvidenceDimension::ConflictReconciliation,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceGateState {
    Verified,
    Missing,
    Failed,
    Conflicted,
    Duplicate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceGate {
    pub dimension: EvidenceDimension,
    pub state: EvidenceGateState,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvidenceGateBlocker {
    pub source_id: String,
    pub dimension: EvidenceDimension,
    pub state: EvidenceGateState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub obligation_id: String,
    pub source_id: String,
    /// Coarse benchmark disposition. This value alone never grants sufficiency.
    pub status: EvidenceStatus,
    /// Independent checks required for acceptance of this source assertion.
    pub gates: Vec<EvidenceGate>,
}

impl EvidenceRecord {
    pub fn gate_blockers(&self) -> Vec<EvidenceGateBlocker> {
        let mut blockers = Vec::new();
        for dimension in REQUIRED_EVIDENCE_DIMENSIONS {
            let matching: Vec<&EvidenceGate> = self
                .gates
                .iter()
                .filter(|gate| gate.dimension == dimension)
                .collect();
            let state = match matching.as_slice() {
                [] => Some(EvidenceGateState::Missing),
                [gate] if gate.state == EvidenceGateState::Verified => None,
                [gate] => Some(gate.state),
                _ => Some(EvidenceGateState::Duplicate),
            };
            if let Some(state) = state {
                blockers.push(EvidenceGateBlocker {
                    source_id: self.source_id.clone(),
                    dimension,
                    state,
                });
            }
        }
        blockers
    }

    pub fn is_verified(&self) -> bool {
        self.status.is_verified() && self.gate_blockers().is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateObservation {
    pub action_id: String,
    pub target_obligation: String,
    pub authorized: bool,
    /// Expected decision-loss reduction in basis points (0..=10,000).
    pub expected_loss_reduction_bps: u16,
    /// Relative execution-cost units defined by the frozen benchmark task.
    pub cost_units: u16,
    /// Relative privacy-exposure units defined by the frozen benchmark task.
    pub privacy_units: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceBudget {
    pub max_cost_units: u16,
    pub max_privacy_units: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionTask {
    pub task_id: String,
    pub required_obligations: Vec<String>,
    pub evidence: Vec<EvidenceRecord>,
    pub candidates: Vec<CandidateObservation>,
    pub budget: ResourceBudget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvaluationError {
    EmptyTaskId,
    NoRequiredObligations,
    EmptyObligationId,
    DuplicateObligation(String),
    EmptySourceId,
    DuplicateSourceId(String),
    EvidenceForUnknownObligation(String),
    EmptyActionId,
    DuplicateActionId(String),
    InvalidLossReduction(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SufficiencyState {
    Sufficient,
    Insufficient,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObligationResult {
    pub obligation_id: String,
    pub satisfied: bool,
    pub verified_source_ids: Vec<String>,
    pub blockers: Vec<EvidenceStatus>,
    pub gate_blockers: Vec<EvidenceGateBlocker>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SufficiencyReport {
    pub state: SufficiencyState,
    pub obligations: Vec<ObligationResult>,
    pub rejected_evidence_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceClaim {
    pub obligation_id: String,
    pub source_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClaimAssessment {
    Supported,
    UnknownObligation,
    NoEvidenceCited,
    UnknownSource,
    SourceNotVerified,
    UnresolvedConflict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CandidateAssessment {
    ParetoOptimal,
    Dominated,
    Unauthorized,
    Irrelevant,
    NoExpectedDecisionValue,
    OverBudget,
}

fn validate_task(task: &DecisionTask) -> Result<(), EvaluationError> {
    if task.task_id.trim().is_empty() {
        return Err(EvaluationError::EmptyTaskId);
    }
    if task.required_obligations.is_empty() {
        return Err(EvaluationError::NoRequiredObligations);
    }

    let mut obligations = BTreeSet::new();
    for obligation in &task.required_obligations {
        if obligation.trim().is_empty() {
            return Err(EvaluationError::EmptyObligationId);
        }
        if !obligations.insert(obligation.as_str()) {
            return Err(EvaluationError::DuplicateObligation(obligation.clone()));
        }
    }

    let mut sources = BTreeSet::new();
    for evidence in &task.evidence {
        if evidence.source_id.trim().is_empty() {
            return Err(EvaluationError::EmptySourceId);
        }
        if !sources.insert(evidence.source_id.as_str()) {
            return Err(EvaluationError::DuplicateSourceId(
                evidence.source_id.clone(),
            ));
        }
        if !obligations.contains(evidence.obligation_id.as_str()) {
            return Err(EvaluationError::EvidenceForUnknownObligation(
                evidence.obligation_id.clone(),
            ));
        }
    }

    let mut actions = BTreeSet::new();
    for candidate in &task.candidates {
        if candidate.action_id.trim().is_empty() {
            return Err(EvaluationError::EmptyActionId);
        }
        if !actions.insert(candidate.action_id.as_str()) {
            return Err(EvaluationError::DuplicateActionId(
                candidate.action_id.clone(),
            ));
        }
        if candidate.expected_loss_reduction_bps > 10_000 {
            return Err(EvaluationError::InvalidLossReduction(
                candidate.action_id.clone(),
            ));
        }
    }

    Ok(())
}

/// Evaluate whether every required evidence obligation is closed.
///
/// A verified record satisfies an obligation only when no unresolved conflict
/// exists for that obligation. Invalid, expired, unauthorized, wrong-subject or
/// untrusted records never satisfy an obligation. A valid alternative source
/// may satisfy an obligation despite unrelated rejected records, but an explicit
/// unresolved contradiction always blocks it.
pub fn evaluate_sufficiency(task: &DecisionTask) -> Result<SufficiencyReport, EvaluationError> {
    validate_task(task)?;

    let mut obligations = Vec::with_capacity(task.required_obligations.len());
    for obligation_id in &task.required_obligations {
        let records: Vec<&EvidenceRecord> = task
            .evidence
            .iter()
            .filter(|record| record.obligation_id == *obligation_id)
            .collect();

        let gate_blockers: Vec<EvidenceGateBlocker> = records
            .iter()
            .flat_map(|record| record.gate_blockers())
            .collect();
        let has_conflict = records.iter().any(|record| {
            record.status == EvidenceStatus::Conflicted
                || record
                    .gate_blockers()
                    .iter()
                    .any(|blocker| blocker.state == EvidenceGateState::Conflicted)
        });
        let verified_source_ids: Vec<String> = records
            .iter()
            .filter(|record| record.is_verified())
            .map(|record| record.source_id.clone())
            .collect();

        let satisfied = !has_conflict && !verified_source_ids.is_empty();
        let blockers = if has_conflict {
            vec![EvidenceStatus::Conflicted]
        } else if satisfied {
            Vec::new()
        } else if records.is_empty() {
            vec![EvidenceStatus::Missing]
        } else {
            records
                .iter()
                .filter_map(|record| {
                    if record.is_verified() {
                        None
                    } else if record.status.is_verified() {
                        Some(EvidenceStatus::IncompleteVerification)
                    } else {
                        Some(record.status.clone())
                    }
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        };

        obligations.push(ObligationResult {
            obligation_id: obligation_id.clone(),
            satisfied,
            verified_source_ids,
            blockers,
            gate_blockers,
        });
    }

    let state = if obligations.iter().all(|result| result.satisfied) {
        SufficiencyState::Sufficient
    } else {
        SufficiencyState::Insufficient
    };

    Ok(SufficiencyReport {
        state,
        obligations,
        rejected_evidence_count: task
            .evidence
            .iter()
            .filter(|record| !record.is_verified())
            .count(),
    })
}

/// Check that a derived claim cites exact, verified sources for its obligation.
/// This evaluator does not grade clinical truth; it grades support lineage.
pub fn assess_claim(
    task: &DecisionTask,
    claim: &EvidenceClaim,
) -> Result<ClaimAssessment, EvaluationError> {
    validate_task(task)?;

    if !task
        .required_obligations
        .iter()
        .any(|obligation| obligation == &claim.obligation_id)
    {
        return Ok(ClaimAssessment::UnknownObligation);
    }
    if claim.source_ids.is_empty() {
        return Ok(ClaimAssessment::NoEvidenceCited);
    }
    if task.evidence.iter().any(|record| {
        record.obligation_id == claim.obligation_id && record.status == EvidenceStatus::Conflicted
    }) {
        return Ok(ClaimAssessment::UnresolvedConflict);
    }

    for source_id in &claim.source_ids {
        let Some(record) = task.evidence.iter().find(|record| {
            record.source_id == *source_id && record.obligation_id == claim.obligation_id
        }) else {
            return Ok(ClaimAssessment::UnknownSource);
        };
        if !record.is_verified() {
            return Ok(ClaimAssessment::SourceNotVerified);
        }
    }

    Ok(ClaimAssessment::Supported)
}

fn is_unresolved(report: &SufficiencyReport, obligation_id: &str) -> bool {
    report
        .obligations
        .iter()
        .find(|result| result.obligation_id == obligation_id)
        .map(|result| !result.satisfied)
        .unwrap_or(false)
}

fn dominates(left: &CandidateObservation, right: &CandidateObservation) -> bool {
    let no_worse = left.expected_loss_reduction_bps >= right.expected_loss_reduction_bps
        && left.cost_units <= right.cost_units
        && left.privacy_units <= right.privacy_units;
    let strictly_better = left.expected_loss_reduction_bps > right.expected_loss_reduction_bps
        || left.cost_units < right.cost_units
        || left.privacy_units < right.privacy_units;
    no_worse && strictly_better
}

/// Return the deterministic Pareto frontier of authorized, relevant, affordable
/// observations. Benefit, execution cost and privacy exposure remain separate
/// dimensions; the evaluator intentionally does not hide a weighted score or
/// choose a universal trade-off on behalf of the policy owner.
pub fn pareto_frontier(task: &DecisionTask) -> Result<Vec<String>, EvaluationError> {
    let report = evaluate_sufficiency(task)?;
    let eligible: Vec<&CandidateObservation> = task
        .candidates
        .iter()
        .filter(|candidate| {
            candidate.authorized
                && candidate.expected_loss_reduction_bps > 0
                && candidate.cost_units <= task.budget.max_cost_units
                && candidate.privacy_units <= task.budget.max_privacy_units
                && is_unresolved(&report, &candidate.target_obligation)
        })
        .collect();

    let mut frontier: Vec<String> = eligible
        .iter()
        .filter(|candidate| {
            !eligible
                .iter()
                .any(|other| other.action_id != candidate.action_id && dominates(other, candidate))
        })
        .map(|candidate| candidate.action_id.clone())
        .collect();
    frontier.sort();
    Ok(frontier)
}

pub fn assess_candidate(
    task: &DecisionTask,
    action_id: &str,
) -> Result<Option<CandidateAssessment>, EvaluationError> {
    validate_task(task)?;
    let Some(candidate) = task
        .candidates
        .iter()
        .find(|candidate| candidate.action_id == action_id)
    else {
        return Ok(None);
    };

    if !candidate.authorized {
        return Ok(Some(CandidateAssessment::Unauthorized));
    }
    let report = evaluate_sufficiency(task)?;
    if !is_unresolved(&report, &candidate.target_obligation) {
        return Ok(Some(CandidateAssessment::Irrelevant));
    }
    if candidate.expected_loss_reduction_bps == 0 {
        return Ok(Some(CandidateAssessment::NoExpectedDecisionValue));
    }
    if candidate.cost_units > task.budget.max_cost_units
        || candidate.privacy_units > task.budget.max_privacy_units
    {
        return Ok(Some(CandidateAssessment::OverBudget));
    }

    let frontier = pareto_frontier(task)?;
    if frontier.iter().any(|id| id == action_id) {
        Ok(Some(CandidateAssessment::ParetoOptimal))
    } else {
        Ok(Some(CandidateAssessment::Dominated))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceSufficiencyLabel {
    Sufficient,
    Insufficient,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentSufficiencyDecision {
    Sufficient,
    Insufficient,
    Abstain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SufficiencyPrediction {
    pub reference: ReferenceSufficiencyLabel,
    /// Model probability that the evidence is sufficient, in basis points.
    pub probability_sufficient_bps: u16,
    pub decision: AgentSufficiencyDecision,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetricError {
    EmptyPredictionSet,
    ProbabilityOutOfRange,
    InvalidThreshold,
    EmptyOutcomeSet,
    DuplicateOutcomeId(String),
    OutcomeProbabilitiesDoNotSumToTenThousand(u32),
    LossOutOfRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectiveMetrics {
    pub total: usize,
    pub covered: usize,
    pub abstained: usize,
    pub errors_on_covered: usize,
    pub true_positive: usize,
    pub false_positive: usize,
    pub false_negative: usize,
    pub true_negative: usize,
    /// Coverage as basis points of all tasks.
    pub coverage_bps: u16,
    /// Error rate among non-abstained tasks; None when coverage is zero.
    pub selective_risk_bps: Option<u16>,
    /// Brier score multiplied by 10,000; lower is better.
    pub brier_score_bps: u16,
    /// Positive predictive value among covered tasks.
    pub precision_sufficient_bps: Option<u16>,
    /// Sensitivity among covered tasks.
    pub recall_sufficient_bps: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiskCoveragePoint {
    pub confidence_threshold_bps: u16,
    pub metrics: SelectiveMetrics,
}

fn ratio_bps(numerator: usize, denominator: usize) -> Option<u16> {
    if denominator == 0 {
        return None;
    }
    Some(((numerator as u128 * 10_000) / denominator as u128) as u16)
}

/// Evaluate coverage, selective error risk, discrimination counts and Brier
/// score. Probabilities are scored for every task, including abstentions; an
/// agent cannot improve calibration metrics by hiding uncertain predictions.
pub fn evaluate_selective_predictions(
    predictions: &[SufficiencyPrediction],
) -> Result<SelectiveMetrics, MetricError> {
    if predictions.is_empty() {
        return Err(MetricError::EmptyPredictionSet);
    }

    let mut covered = 0usize;
    let mut abstained = 0usize;
    let mut errors = 0usize;
    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut fn_count = 0usize;
    let mut tn = 0usize;
    let mut squared_error_sum = 0u128;

    for prediction in predictions {
        let p = prediction.probability_sufficient_bps;
        if p > 10_000 {
            return Err(MetricError::ProbabilityOutOfRange);
        }
        let target = match prediction.reference {
            ReferenceSufficiencyLabel::Sufficient => 10_000i64,
            ReferenceSufficiencyLabel::Insufficient => 0i64,
        };
        let delta = p as i64 - target;
        squared_error_sum += (delta * delta) as u128;

        match prediction.decision {
            AgentSufficiencyDecision::Abstain => abstained += 1,
            AgentSufficiencyDecision::Sufficient => {
                covered += 1;
                if prediction.reference == ReferenceSufficiencyLabel::Sufficient {
                    tp += 1;
                } else {
                    fp += 1;
                    errors += 1;
                }
            }
            AgentSufficiencyDecision::Insufficient => {
                covered += 1;
                if prediction.reference == ReferenceSufficiencyLabel::Insufficient {
                    tn += 1;
                } else {
                    fn_count += 1;
                    errors += 1;
                }
            }
        }
    }

    let total = predictions.len();
    let brier_score_bps = (squared_error_sum / (total as u128 * 10_000)).min(10_000) as u16;

    Ok(SelectiveMetrics {
        total,
        covered,
        abstained,
        errors_on_covered: errors,
        true_positive: tp,
        false_positive: fp,
        false_negative: fn_count,
        true_negative: tn,
        coverage_bps: ratio_bps(covered, total).expect("non-empty predictions"),
        selective_risk_bps: ratio_bps(errors, covered),
        brier_score_bps,
        precision_sufficient_bps: ratio_bps(tp, tp + fp),
        recall_sufficient_bps: ratio_bps(tp, tp + fn_count),
    })
}

/// Construct a risk/coverage curve from model confidence thresholds. A prediction
/// is covered only when it is not already an abstention and its confidence in
/// the selected class reaches the threshold.
pub fn evaluate_risk_coverage_curve(
    predictions: &[SufficiencyPrediction],
    confidence_thresholds_bps: &[u16],
) -> Result<Vec<RiskCoveragePoint>, MetricError> {
    if predictions.is_empty() {
        return Err(MetricError::EmptyPredictionSet);
    }
    if predictions
        .iter()
        .any(|prediction| prediction.probability_sufficient_bps > 10_000)
    {
        return Err(MetricError::ProbabilityOutOfRange);
    }

    let mut curve = Vec::with_capacity(confidence_thresholds_bps.len());
    let mut previous_threshold = None;
    for threshold in confidence_thresholds_bps {
        if *threshold > 10_000 || previous_threshold.map(|p| *threshold < p).unwrap_or(false) {
            return Err(MetricError::InvalidThreshold);
        }
        previous_threshold = Some(*threshold);

        let filtered: Vec<SufficiencyPrediction> = predictions
            .iter()
            .copied()
            .map(|mut prediction| {
                let confidence = match prediction.decision {
                    AgentSufficiencyDecision::Sufficient => prediction.probability_sufficient_bps,
                    AgentSufficiencyDecision::Insufficient => {
                        10_000 - prediction.probability_sufficient_bps
                    }
                    AgentSufficiencyDecision::Abstain => 0,
                };
                if confidence < *threshold {
                    prediction.decision = AgentSufficiencyDecision::Abstain;
                }
                prediction
            })
            .collect();

        curve.push(RiskCoveragePoint {
            confidence_threshold_bps: *threshold,
            metrics: evaluate_selective_predictions(&filtered)?,
        });
    }
    Ok(curve)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetBenefitFraction {
    /// Exact numerator; divide by denominator to obtain net benefit.
    pub numerator: i64,
    pub denominator: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionCurvePoint {
    pub threshold_bps: u16,
    pub model: NetBenefitFraction,
    pub act_all: NetBenefitFraction,
    pub act_none: NetBenefitFraction,
}

fn net_benefit_fraction(
    tp: usize,
    fp: usize,
    total: usize,
    threshold_bps: u16,
) -> NetBenefitFraction {
    let threshold = threshold_bps as i64;
    let numerator = tp as i64 * (10_000 - threshold) - fp as i64 * threshold;
    let denominator = total as u64 * (10_000 - threshold_bps as u64);
    NetBenefitFraction {
        numerator,
        denominator,
    }
}

/// Compute net benefit for accepting an assertion at a predeclared sufficiency
/// threshold. False acceptance (claiming sufficient when insufficient) is
/// weighted by the threshold odds. Returns exact rational values and the
/// act-all / act-none references; it does not select the clinical threshold.
pub fn evaluate_net_benefit(
    predictions: &[SufficiencyPrediction],
    threshold_bps: u16,
) -> Result<DecisionCurvePoint, MetricError> {
    if predictions.is_empty() {
        return Err(MetricError::EmptyPredictionSet);
    }
    if threshold_bps == 0 || threshold_bps >= 10_000 {
        return Err(MetricError::InvalidThreshold);
    }
    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut positive = 0usize;
    let mut negative = 0usize;

    for prediction in predictions {
        if prediction.probability_sufficient_bps > 10_000 {
            return Err(MetricError::ProbabilityOutOfRange);
        }
        match prediction.reference {
            ReferenceSufficiencyLabel::Sufficient => positive += 1,
            ReferenceSufficiencyLabel::Insufficient => negative += 1,
        }
        if prediction.probability_sufficient_bps >= threshold_bps {
            match prediction.reference {
                ReferenceSufficiencyLabel::Sufficient => tp += 1,
                ReferenceSufficiencyLabel::Insufficient => fp += 1,
            }
        }
    }

    let total = predictions.len();
    Ok(DecisionCurvePoint {
        threshold_bps,
        model: net_benefit_fraction(tp, fp, total, threshold_bps),
        act_all: net_benefit_fraction(positive, negative, total, threshold_bps),
        act_none: NetBenefitFraction {
            numerator: 0,
            denominator: total as u64,
        },
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InformationOutcome {
    pub outcome_id: String,
    pub probability_bps: u16,
    /// Optimal expected decision loss after observing this outcome.
    pub best_expected_loss_bps: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvsiModel {
    pub action_id: String,
    pub authorized: bool,
    pub current_best_expected_loss_bps: u16,
    pub outcomes: Vec<InformationOutcome>,
    /// Kept separate from EVSI; no universal conversion is imposed.
    pub cost_units: u16,
    /// Kept separate from EVSI; no universal conversion is imposed.
    pub privacy_units: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvsiReport {
    pub action_id: String,
    pub authorized: bool,
    pub expected_post_observation_loss_bps: u16,
    /// Current optimal expected loss minus expected optimal loss after observation.
    /// Negative values indicate modelled value destruction or an inconsistent oracle.
    pub gross_evsi_bps: i32,
    pub cost_units: u16,
    pub privacy_units: u16,
}

/// Calculate EVSI from a frozen synthetic outcome distribution. Outcome
/// probabilities must sum to 10,000 basis points. EVSI is reported separately
/// from cost and privacy; authorization is surfaced but never overridden by
/// positive informational value.
pub fn evaluate_evsi(model: &EvsiModel) -> Result<EvsiReport, MetricError> {
    if model.outcomes.is_empty() {
        return Err(MetricError::EmptyOutcomeSet);
    }
    if model.current_best_expected_loss_bps > 10_000
        || model
            .outcomes
            .iter()
            .any(|outcome| outcome.best_expected_loss_bps > 10_000)
    {
        return Err(MetricError::LossOutOfRange);
    }

    let mut ids = BTreeSet::new();
    let mut probability_sum = 0u32;
    let mut weighted_loss_sum = 0u128;
    for outcome in &model.outcomes {
        if outcome.outcome_id.trim().is_empty() || !ids.insert(outcome.outcome_id.as_str()) {
            return Err(MetricError::DuplicateOutcomeId(outcome.outcome_id.clone()));
        }
        if outcome.probability_bps > 10_000 {
            return Err(MetricError::ProbabilityOutOfRange);
        }
        probability_sum += outcome.probability_bps as u32;
        weighted_loss_sum +=
            outcome.probability_bps as u128 * outcome.best_expected_loss_bps as u128;
    }
    if probability_sum != 10_000 {
        return Err(MetricError::OutcomeProbabilitiesDoNotSumToTenThousand(
            probability_sum,
        ));
    }

    let expected_post_observation_loss_bps =
        ((weighted_loss_sum + 5_000) / 10_000).min(10_000) as u16;
    Ok(EvsiReport {
        action_id: model.action_id.clone(),
        authorized: model.authorized,
        expected_post_observation_loss_bps,
        gross_evsi_bps: model.current_best_expected_loss_bps as i32
            - expected_post_observation_loss_bps as i32,
        cost_units: model.cost_units,
        privacy_units: model.privacy_units,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gates_for_status(status: &EvidenceStatus) -> Vec<EvidenceGate> {
        let mut gates: Vec<EvidenceGate> = REQUIRED_EVIDENCE_DIMENSIONS
            .iter()
            .copied()
            .map(|dimension| EvidenceGate {
                dimension,
                state: EvidenceGateState::Verified,
            })
            .collect();

        let failure = match status {
            EvidenceStatus::Verified => None,
            EvidenceStatus::Missing => Some((
                EvidenceDimension::EvidencePresence,
                EvidenceGateState::Missing,
            )),
            EvidenceStatus::Conflicted => Some((
                EvidenceDimension::ConflictReconciliation,
                EvidenceGateState::Conflicted,
            )),
            EvidenceStatus::Expired => Some((
                EvidenceDimension::TemporalValidity,
                EvidenceGateState::Failed,
            )),
            EvidenceStatus::Unauthorized => Some((
                EvidenceDimension::ApplicationAuthority,
                EvidenceGateState::Failed,
            )),
            EvidenceStatus::WrongSubject => Some((
                EvidenceDimension::SubjectBinding,
                EvidenceGateState::Failed,
            )),
            EvidenceStatus::UnauthenticatedIssuer => Some((
                EvidenceDimension::IssuerAuthenticity,
                EvidenceGateState::Failed,
            )),
            EvidenceStatus::MissingVersion => Some((
                EvidenceDimension::ResourceVersion,
                EvidenceGateState::Missing,
            )),
            EvidenceStatus::UnsupportedSemantics => Some((
                EvidenceDimension::SemanticSupport,
                EvidenceGateState::Failed,
            )),
            EvidenceStatus::IncompleteVerification => Some((
                EvidenceDimension::ProvenanceLineage,
                EvidenceGateState::Missing,
            )),
        };

        if let Some((dimension, state)) = failure {
            let gate = gates
                .iter_mut()
                .find(|gate| gate.dimension == dimension)
                .expect("every evidence failure maps to a required dimension");
            gate.state = state;
        }
        gates
    }

    fn evidence_record(
        obligation_id: &str,
        source_id: &str,
        status: EvidenceStatus,
    ) -> EvidenceRecord {
        EvidenceRecord {
            obligation_id: obligation_id.to_string(),
            source_id: source_id.to_string(),
            gates: gates_for_status(&status),
            status,
        }
    }

    fn verified(obligation_id: &str, source_id: &str) -> EvidenceRecord {
        evidence_record(obligation_id, source_id, EvidenceStatus::Verified)
    }

    fn task(evidence: Vec<EvidenceRecord>, candidates: Vec<CandidateObservation>) -> DecisionTask {
        DecisionTask {
            task_id: "synthetic-task".to_string(),
            required_obligations: vec!["patient-binding".to_string(), "lab-provenance".to_string()],
            evidence,
            candidates,
            budget: ResourceBudget {
                max_cost_units: 10,
                max_privacy_units: 8,
            },
        }
    }

    fn action(
        action_id: &str,
        target_obligation: &str,
        authorized: bool,
        benefit: u16,
        cost: u16,
        privacy: u16,
    ) -> CandidateObservation {
        CandidateObservation {
            action_id: action_id.to_string(),
            target_obligation: target_obligation.to_string(),
            authorized,
            expected_loss_reduction_bps: benefit,
            cost_units: cost,
            privacy_units: privacy,
        }
    }

    #[test]
    fn sufficient_requires_every_obligation_to_be_verified() {
        let input = task(
            vec![
                verified("patient-binding", "patient-proof-1"),
                verified("lab-provenance", "lab-source-1"),
            ],
            Vec::new(),
        );
        assert_eq!(
            evaluate_sufficiency(&input).unwrap().state,
            SufficiencyState::Sufficient
        );
    }

    #[test]
    fn missing_evidence_is_insufficient_not_assumed_from_context() {
        let input = task(
            vec![verified("patient-binding", "patient-proof-1")],
            Vec::new(),
        );
        let report = evaluate_sufficiency(&input).unwrap();
        assert_eq!(report.state, SufficiencyState::Insufficient);
        assert_eq!(
            report.obligations[1].blockers,
            vec![EvidenceStatus::Missing]
        );
    }

    #[test]
    fn wrong_subject_never_satisfies_patient_binding() {
        let input = task(
            vec![
                evidence_record("patient-binding", "wrong-patient-record", EvidenceStatus::WrongSubject),
                verified("lab-provenance", "lab-source-1"),
            ],
            Vec::new(),
        );
        let report = evaluate_sufficiency(&input).unwrap();
        assert_eq!(report.state, SufficiencyState::Insufficient);
        assert_eq!(
            report.obligations[0].blockers,
            vec![EvidenceStatus::WrongSubject]
        );
    }

    #[test]
    fn unresolved_conflict_blocks_even_when_a_verified_record_exists() {
        let input = task(
            vec![
                verified("patient-binding", "patient-proof-1"),
                verified("lab-provenance", "lab-source-1"),
                evidence_record("lab-provenance", "conflicting-lab-source", EvidenceStatus::Conflicted),
            ],
            Vec::new(),
        );
        let report = evaluate_sufficiency(&input).unwrap();
        assert_eq!(report.state, SufficiencyState::Insufficient);
        assert_eq!(
            report.obligations[1].blockers,
            vec![EvidenceStatus::Conflicted]
        );
    }

    #[test]
    fn expired_untrusted_and_unauthorized_records_do_not_close_obligations() {
        let input = task(
            vec![
                verified("patient-binding", "patient-proof-1"),
                evidence_record("lab-provenance", "expired-lab", EvidenceStatus::Expired),
                evidence_record("lab-provenance", "untrusted-lab", EvidenceStatus::UnauthenticatedIssuer),
                evidence_record("lab-provenance", "unauthorized-lab", EvidenceStatus::Unauthorized),
            ],
            Vec::new(),
        );
        let report = evaluate_sufficiency(&input).unwrap();
        assert_eq!(report.state, SufficiencyState::Insufficient);
        assert_eq!(
            report.obligations[1].blockers,
            vec![
                EvidenceStatus::Expired,
                EvidenceStatus::Unauthorized,
                EvidenceStatus::UnauthenticatedIssuer
            ]
        );
    }

    #[test]
    fn a_verified_alternative_can_satisfy_an_obligation_but_conflicts_remain_visible() {
        let input = task(
            vec![
                verified("patient-binding", "patient-proof-1"),
                evidence_record("lab-provenance", "rejected-old-source", EvidenceStatus::Expired),
                verified("lab-provenance", "current-lab-source"),
            ],
            Vec::new(),
        );
        let report = evaluate_sufficiency(&input).unwrap();
        assert_eq!(report.state, SufficiencyState::Sufficient);
        assert_eq!(report.rejected_evidence_count, 1);
    }

    #[test]
    fn claims_must_cite_exact_verified_source_ids() {
        let input = task(
            vec![
                verified("patient-binding", "patient-proof-1"),
                verified("lab-provenance", "lab-source-1"),
            ],
            Vec::new(),
        );
        assert_eq!(
            assess_claim(
                &input,
                &EvidenceClaim {
                    obligation_id: "lab-provenance".to_string(),
                    source_ids: vec!["lab-source-1".to_string()],
                }
            )
            .unwrap(),
            ClaimAssessment::Supported
        );
        assert_eq!(
            assess_claim(
                &input,
                &EvidenceClaim {
                    obligation_id: "lab-provenance".to_string(),
                    source_ids: vec!["invented-source".to_string()],
                }
            )
            .unwrap(),
            ClaimAssessment::UnknownSource
        );
    }

    #[test]
    fn claims_cannot_be_supported_by_wrong_patient_or_conflicted_sources() {
        let input = task(
            vec![
                verified("patient-binding", "patient-proof-1"),
                evidence_record("lab-provenance", "wrong-patient-lab", EvidenceStatus::WrongSubject),
            ],
            Vec::new(),
        );
        assert_eq!(
            assess_claim(
                &input,
                &EvidenceClaim {
                    obligation_id: "lab-provenance".to_string(),
                    source_ids: vec!["wrong-patient-lab".to_string()],
                }
            )
            .unwrap(),
            ClaimAssessment::SourceNotVerified
        );
    }

    #[test]
    fn retrieval_frontier_excludes_unauthorized_irrelevant_zero_value_and_over_budget_actions() {
        let input = task(
            vec![
                verified("patient-binding", "patient-proof-1"),
                evidence_record("lab-provenance", "expired-lab", EvidenceStatus::Expired),
            ],
            vec![
                action("high-value", "lab-provenance", true, 8_000, 4, 5),
                action("low-cost", "lab-provenance", true, 5_000, 2, 1),
                action("dominated", "lab-provenance", true, 4_000, 5, 5),
                action("unauthorized", "lab-provenance", false, 10_000, 1, 1),
                action("irrelevant", "not-required", true, 9_000, 1, 1),
                action("zero-value", "lab-provenance", true, 0, 1, 1),
                action("over-budget", "lab-provenance", true, 9_000, 11, 1),
            ],
        );

        assert_eq!(
            pareto_frontier(&input).unwrap(),
            vec!["high-value".to_string(), "low-cost".to_string()]
        );
        assert_eq!(
            assess_candidate(&input, "unauthorized").unwrap(),
            Some(CandidateAssessment::Unauthorized)
        );
        assert_eq!(
            assess_candidate(&input, "irrelevant").unwrap(),
            Some(CandidateAssessment::Irrelevant)
        );
        assert_eq!(
            assess_candidate(&input, "zero-value").unwrap(),
            Some(CandidateAssessment::NoExpectedDecisionValue)
        );
        assert_eq!(
            assess_candidate(&input, "over-budget").unwrap(),
            Some(CandidateAssessment::OverBudget)
        );
        assert_eq!(
            assess_candidate(&input, "dominated").unwrap(),
            Some(CandidateAssessment::Dominated)
        );
        assert_eq!(
            assess_candidate(&input, "high-value").unwrap(),
            Some(CandidateAssessment::ParetoOptimal)
        );
        assert_eq!(assess_candidate(&input, "missing-action").unwrap(), None);
    }

    #[test]
    fn sufficient_evidence_means_no_retrieval_is_needed_for_that_task() {
        let input = task(
            vec![
                verified("patient-binding", "patient-proof-1"),
                verified("lab-provenance", "lab-source-1"),
            ],
            vec![action("unneeded-read", "lab-provenance", true, 1_000, 1, 1)],
        );
        assert!(pareto_frontier(&input).unwrap().is_empty());
        assert_eq!(
            assess_candidate(&input, "unneeded-read").unwrap(),
            Some(CandidateAssessment::Irrelevant)
        );
    }

    #[test]
    fn missing_version_and_unsupported_semantics_are_distinct_blockers() {
        let input = task(
            vec![
                verified("patient-binding", "patient-proof-1"),
                evidence_record("lab-provenance", "unversioned-lab", EvidenceStatus::MissingVersion),
                evidence_record("lab-provenance", "unsupported-unit-lab", EvidenceStatus::UnsupportedSemantics),
            ],
            Vec::new(),
        );
        let report = evaluate_sufficiency(&input).unwrap();
        assert_eq!(report.state, SufficiencyState::Insufficient);
        assert_eq!(
            report.obligations[1].blockers,
            vec![
                EvidenceStatus::MissingVersion,
                EvidenceStatus::UnsupportedSemantics
            ]
        );
    }

    #[test]
    fn selective_metrics_score_abstentions_as_uncalibrated_predictions_not_free_passes() {
        let predictions = vec![
            SufficiencyPrediction {
                reference: ReferenceSufficiencyLabel::Sufficient,
                probability_sufficient_bps: 9_000,
                decision: AgentSufficiencyDecision::Sufficient,
            },
            SufficiencyPrediction {
                reference: ReferenceSufficiencyLabel::Insufficient,
                probability_sufficient_bps: 8_000,
                decision: AgentSufficiencyDecision::Abstain,
            },
            SufficiencyPrediction {
                reference: ReferenceSufficiencyLabel::Insufficient,
                probability_sufficient_bps: 1_000,
                decision: AgentSufficiencyDecision::Insufficient,
            },
        ];
        let metrics = evaluate_selective_predictions(&predictions).unwrap();
        assert_eq!(metrics.covered, 2);
        assert_eq!(metrics.abstained, 1);
        assert_eq!(metrics.coverage_bps, 6_666);
        assert_eq!(metrics.errors_on_covered, 0);
        assert_eq!(metrics.selective_risk_bps, Some(0));
        assert_eq!(metrics.true_positive, 1);
        assert_eq!(metrics.true_negative, 1);
        assert_eq!(metrics.brier_score_bps, 2_200);
    }

    #[test]
    fn risk_coverage_curve_reduces_coverage_as_confidence_threshold_rises() {
        let predictions = vec![
            SufficiencyPrediction {
                reference: ReferenceSufficiencyLabel::Sufficient,
                probability_sufficient_bps: 9_000,
                decision: AgentSufficiencyDecision::Sufficient,
            },
            SufficiencyPrediction {
                reference: ReferenceSufficiencyLabel::Insufficient,
                probability_sufficient_bps: 6_000,
                decision: AgentSufficiencyDecision::Sufficient,
            },
            SufficiencyPrediction {
                reference: ReferenceSufficiencyLabel::Insufficient,
                probability_sufficient_bps: 1_000,
                decision: AgentSufficiencyDecision::Insufficient,
            },
        ];
        let curve = evaluate_risk_coverage_curve(&predictions, &[0, 5_000, 8_000]).unwrap();
        assert_eq!(curve.len(), 3);
        assert_eq!(curve[0].metrics.coverage_bps, 10_000);
        assert_eq!(curve[1].metrics.coverage_bps, 10_000);
        assert_eq!(curve[2].metrics.coverage_bps, 6_666);
        assert_eq!(curve[0].metrics.selective_risk_bps, Some(3_333));
        assert_eq!(curve[2].metrics.selective_risk_bps, Some(0));
    }

    #[test]
    fn risk_coverage_curve_rejects_out_of_range_probabilities_before_confidence_math() {
        let predictions = vec![SufficiencyPrediction {
            reference: ReferenceSufficiencyLabel::Insufficient,
            probability_sufficient_bps: 10_001,
            decision: AgentSufficiencyDecision::Insufficient,
        }];
        assert_eq!(
            evaluate_risk_coverage_curve(&predictions, &[8_000]),
            Err(MetricError::ProbabilityOutOfRange)
        );
    }

    #[test]
    fn decision_curve_reports_model_and_default_policy_net_benefits() {
        let predictions = vec![
            SufficiencyPrediction {
                reference: ReferenceSufficiencyLabel::Sufficient,
                probability_sufficient_bps: 9_000,
                decision: AgentSufficiencyDecision::Sufficient,
            },
            SufficiencyPrediction {
                reference: ReferenceSufficiencyLabel::Insufficient,
                probability_sufficient_bps: 8_000,
                decision: AgentSufficiencyDecision::Sufficient,
            },
            SufficiencyPrediction {
                reference: ReferenceSufficiencyLabel::Insufficient,
                probability_sufficient_bps: 1_000,
                decision: AgentSufficiencyDecision::Insufficient,
            },
        ];
        let curve = evaluate_net_benefit(&predictions, 7_500).unwrap();
        assert_eq!(curve.model.numerator, -5_000);
        assert_eq!(curve.model.denominator, 3 * 2_500);
        assert_eq!(curve.act_all.numerator, -12_500);
        assert_eq!(curve.act_none.numerator, 0);
    }

    #[test]
    fn evsi_uses_probability_weighted_post_observation_loss_and_keeps_cost_privacy_separate() {
        let model = EvsiModel {
            action_id: "obtain-current-lab".to_string(),
            authorized: true,
            current_best_expected_loss_bps: 6_000,
            outcomes: vec![
                InformationOutcome {
                    outcome_id: "resolves-conflict".to_string(),
                    probability_bps: 7_000,
                    best_expected_loss_bps: 2_000,
                },
                InformationOutcome {
                    outcome_id: "remains-ambiguous".to_string(),
                    probability_bps: 3_000,
                    best_expected_loss_bps: 5_000,
                },
            ],
            cost_units: 4,
            privacy_units: 2,
        };
        let report = evaluate_evsi(&model).unwrap();
        assert_eq!(report.expected_post_observation_loss_bps, 2_900);
        assert_eq!(report.gross_evsi_bps, 3_100);
        assert_eq!(report.cost_units, 4);
        assert_eq!(report.privacy_units, 2);
        assert!(report.authorized);
    }

    #[test]
    fn positive_evsi_never_turns_an_unauthorized_acquisition_into_an_authorized_action() {
        let model = EvsiModel {
            action_id: "unauthorized-read".to_string(),
            authorized: false,
            current_best_expected_loss_bps: 8_000,
            outcomes: vec![InformationOutcome {
                outcome_id: "revealing".to_string(),
                probability_bps: 10_000,
                best_expected_loss_bps: 1_000,
            }],
            cost_units: 1,
            privacy_units: 1,
        };
        let report = evaluate_evsi(&model).unwrap();
        assert_eq!(report.gross_evsi_bps, 7_000);
        assert!(!report.authorized);
    }

    #[test]
    fn evsi_rejects_incomplete_outcome_probability_mass() {
        let model = EvsiModel {
            action_id: "partial-model".to_string(),
            authorized: true,
            current_best_expected_loss_bps: 5_000,
            outcomes: vec![InformationOutcome {
                outcome_id: "only-half-the-world".to_string(),
                probability_bps: 5_000,
                best_expected_loss_bps: 1_000,
            }],
            cost_units: 0,
            privacy_units: 0,
        };
        assert_eq!(
            evaluate_evsi(&model),
            Err(MetricError::OutcomeProbabilitiesDoNotSumToTenThousand(
                5_000
            ))
        );
    }

    #[test]
    fn frozen_fixture_catalogue_drives_reference_evaluation() {
        let mut ids = BTreeSet::new();
        let mut rows = 0usize;

        for line in include_str!("../fixtures/v1-cases.tsv").lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('|').collect();
            assert_eq!(fields.len(), 4, "malformed fixture catalogue row");
            let case_id = fields[0];
            let profile = fields[1];
            let expected = fields[2];
            assert!(!case_id.trim().is_empty(), "fixture ID must not be empty");
            assert!(ids.insert(case_id), "duplicate fixture ID: {}", case_id);
            assert!(
                ["sufficient", "insufficient", "invalid"].contains(&expected),
                "unknown expected sufficiency in fixture {}",
                case_id
            );
            assert!(
                !fields[3].trim().is_empty(),
                "fixture purpose must not be empty"
            );

            let (input, expected_frontier) = match profile {
                "complete" | "already_sufficient" => (
                    task(
                        vec![
                            verified("patient-binding", "patient-proof-1"),
                            verified("lab-provenance", "lab-source-1"),
                        ],
                        if profile == "already_sufficient" {
                            vec![action("unneeded-read", "lab-provenance", true, 1_000, 1, 1)]
                        } else {
                            Vec::new()
                        },
                    ),
                    None,
                ),
                "missing" => (
                    task(
                        vec![verified("patient-binding", "patient-proof-1")],
                        Vec::new(),
                    ),
                    None,
                ),
                "wrong_subject" => (
                    task(
                        vec![
                            evidence_record("patient-binding", "wrong-patient-proof", EvidenceStatus::WrongSubject),
                            verified("lab-provenance", "lab-source-1"),
                        ],
                        Vec::new(),
                    ),
                    None,
                ),
                "conflicted" => (
                    task(
                        vec![
                            verified("patient-binding", "patient-proof-1"),
                            verified("lab-provenance", "lab-source-1"),
                            evidence_record("lab-provenance", "conflicting-lab", EvidenceStatus::Conflicted),
                        ],
                        Vec::new(),
                    ),
                    None,
                ),
                "expired"
                | "unauthenticated"
                | "unauthorized"
                | "missing_version"
                | "unsupported_semantics" => {
                    let status = match profile {
                        "expired" => EvidenceStatus::Expired,
                        "unauthenticated" => EvidenceStatus::UnauthenticatedIssuer,
                        "unauthorized" => EvidenceStatus::Unauthorized,
                        "missing_version" => EvidenceStatus::MissingVersion,
                        _ => EvidenceStatus::UnsupportedSemantics,
                    };
                    (
                        task(
                            vec![
                                verified("patient-binding", "patient-proof-1"),
                                evidence_record("lab-provenance", "blocked-lab", status.clone()),
                            ],
                            Vec::new(),
                        ),
                        None,
                    )
                }
                "alternative" => (
                    task(
                        vec![
                            verified("patient-binding", "patient-proof-1"),
                            evidence_record("lab-provenance", "expired-lab", EvidenceStatus::Expired),
                            verified("lab-provenance", "current-lab"),
                        ],
                        Vec::new(),
                    ),
                    None,
                ),
                "pareto" => (
                    task(
                        vec![
                            verified("patient-binding", "patient-proof-1"),
                            evidence_record("lab-provenance", "expired-lab", EvidenceStatus::Expired),
                        ],
                        vec![
                            action("high-value", "lab-provenance", true, 8_000, 4, 5),
                            action("low-cost", "lab-provenance", true, 5_000, 2, 1),
                            action("dominated", "lab-provenance", true, 4_000, 5, 5),
                        ],
                    ),
                    Some(vec!["high-value".to_string(), "low-cost".to_string()]),
                ),
                "unauthorized_action" => (
                    task(
                        vec![
                            verified("patient-binding", "patient-proof-1"),
                            evidence_record("lab-provenance", "expired-lab", EvidenceStatus::Expired),
                        ],
                        vec![action(
                            "unauthorized-high-value",
                            "lab-provenance",
                            false,
                            10_000,
                            1,
                            1,
                        )],
                    ),
                    Some(Vec::new()),
                ),
                "vacuous" => {
                    let mut input = task(Vec::new(), Vec::new());
                    input.required_obligations.clear();
                    (input, None)
                }
                "ambiguous" => {
                    let mut input = task(Vec::new(), Vec::new());
                    input
                        .required_obligations
                        .push("lab-provenance".to_string());
                    (input, None)
                }
                _ => panic!("unknown frozen fixture profile {} for {}", profile, case_id),
            };

            let evaluation = evaluate_sufficiency(&input);
            match expected {
                "sufficient" => assert_eq!(
                    evaluation.unwrap().state,
                    SufficiencyState::Sufficient,
                    "fixture {}",
                    case_id
                ),
                "insufficient" => assert_eq!(
                    evaluation.unwrap().state,
                    SufficiencyState::Insufficient,
                    "fixture {}",
                    case_id
                ),
                "invalid" => assert!(
                    evaluation.is_err(),
                    "invalid fixture {} must be rejected",
                    case_id
                ),
                _ => unreachable!(),
            }

            if let Some(expected_frontier) = expected_frontier {
                assert_eq!(
                    pareto_frontier(&input).unwrap(),
                    expected_frontier,
                    "frontier for fixture {}",
                    case_id
                );
                if profile == "unauthorized_action" {
                    assert_eq!(
                        assess_candidate(&input, "unauthorized-high-value").unwrap(),
                        Some(CandidateAssessment::Unauthorized),
                        "unauthorized retrieval for fixture {}",
                        case_id
                    );
                }
            }
            rows += 1;
        }

        assert_eq!(rows, 15, "v1 fixture catalogue size changed");
    }

    #[test]
    fn task_contract_rejects_vacuous_and_ambiguous_inputs() {
        let mut empty = task(Vec::new(), Vec::new());
        empty.required_obligations.clear();
        assert_eq!(
            evaluate_sufficiency(&empty),
            Err(EvaluationError::NoRequiredObligations)
        );

        let mut duplicate = task(Vec::new(), Vec::new());
        duplicate
            .required_obligations
            .push("lab-provenance".to_string());
        assert_eq!(
            evaluate_sufficiency(&duplicate),
            Err(EvaluationError::DuplicateObligation(
                "lab-provenance".to_string()
            ))
        );
    }
}
