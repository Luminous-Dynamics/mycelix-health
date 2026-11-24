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
}

impl EvidenceStatus {
    fn is_verified(&self) -> bool {
        matches!(self, Self::Verified)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub obligation_id: String,
    pub source_id: String,
    pub status: EvidenceStatus,
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
            return Err(EvaluationError::DuplicateSourceId(evidence.source_id.clone()));
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
            return Err(EvaluationError::DuplicateActionId(candidate.action_id.clone()));
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
pub fn evaluate_sufficiency(
    task: &DecisionTask,
) -> Result<SufficiencyReport, EvaluationError> {
    validate_task(task)?;

    let mut obligations = Vec::with_capacity(task.required_obligations.len());
    for obligation_id in &task.required_obligations {
        let records: Vec<&EvidenceRecord> = task
            .evidence
            .iter()
            .filter(|record| record.obligation_id == *obligation_id)
            .collect();

        let has_conflict = records
            .iter()
            .any(|record| record.status == EvidenceStatus::Conflicted);
        let verified_source_ids: Vec<String> = records
            .iter()
            .filter(|record| record.status.is_verified())
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
                .map(|record| record.status.clone())
                .filter(|status| !status.is_verified())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        };

        obligations.push(ObligationResult {
            obligation_id: obligation_id.clone(),
            satisfied,
            verified_source_ids,
            blockers,
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
            .filter(|record| !record.status.is_verified())
            .count(),
    })
}

/// Check that a derived claim cites exact, verified sources for its obligation.
/// This evaluator does not grade clinical truth; it grades support lineage.
pub fn assess_claim(task: &DecisionTask, claim: &EvidenceClaim) -> Result<ClaimAssessment, EvaluationError> {
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
        record.obligation_id == claim.obligation_id
            && record.status == EvidenceStatus::Conflicted
    }) {
        return Ok(ClaimAssessment::UnresolvedConflict);
    }

    for source_id in &claim.source_ids {
        let Some(record) = task.evidence.iter().find(|record| {
            record.source_id == *source_id && record.obligation_id == claim.obligation_id
        }) else {
            return Ok(ClaimAssessment::UnknownSource);
        };
        if !record.status.is_verified() {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn verified(obligation_id: &str, source_id: &str) -> EvidenceRecord {
        EvidenceRecord {
            obligation_id: obligation_id.to_string(),
            source_id: source_id.to_string(),
            status: EvidenceStatus::Verified,
        }
    }

    fn task(evidence: Vec<EvidenceRecord>, candidates: Vec<CandidateObservation>) -> DecisionTask {
        DecisionTask {
            task_id: "synthetic-task".to_string(),
            required_obligations: vec![
                "patient-binding".to_string(),
                "lab-provenance".to_string(),
            ],
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
        let input = task(vec![verified("patient-binding", "patient-proof-1")], Vec::new());
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
                EvidenceRecord {
                    obligation_id: "patient-binding".to_string(),
                    source_id: "wrong-patient-record".to_string(),
                    status: EvidenceStatus::WrongSubject,
                },
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
                EvidenceRecord {
                    obligation_id: "lab-provenance".to_string(),
                    source_id: "conflicting-lab-source".to_string(),
                    status: EvidenceStatus::Conflicted,
                },
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
                EvidenceRecord {
                    obligation_id: "lab-provenance".to_string(),
                    source_id: "expired-lab".to_string(),
                    status: EvidenceStatus::Expired,
                },
                EvidenceRecord {
                    obligation_id: "lab-provenance".to_string(),
                    source_id: "untrusted-lab".to_string(),
                    status: EvidenceStatus::UnauthenticatedIssuer,
                },
                EvidenceRecord {
                    obligation_id: "lab-provenance".to_string(),
                    source_id: "unauthorized-lab".to_string(),
                    status: EvidenceStatus::Unauthorized,
                },
            ],
            Vec::new(),
        );
        let report = evaluate_sufficiency(&input).unwrap();
        assert_eq!(report.state, SufficiencyState::Insufficient);
        assert_eq!(
            report.obligations[1].blockers,
            vec![
                EvidenceStatus::Expired,
                EvidenceStatus::UnauthenticatedIssuer,
                EvidenceStatus::Unauthorized
            ]
        );
    }

    #[test]
    fn a_verified_alternative_can_satisfy_an_obligation_but_conflicts_remain_visible() {
        let input = task(
            vec![
                verified("patient-binding", "patient-proof-1"),
                EvidenceRecord {
                    obligation_id: "lab-provenance".to_string(),
                    source_id: "rejected-old-source".to_string(),
                    status: EvidenceStatus::Expired,
                },
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
                EvidenceRecord {
                    obligation_id: "lab-provenance".to_string(),
                    source_id: "wrong-patient-lab".to_string(),
                    status: EvidenceStatus::WrongSubject,
                },
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
                EvidenceRecord {
                    obligation_id: "lab-provenance".to_string(),
                    source_id: "expired-lab".to_string(),
                    status: EvidenceStatus::Expired,
                },
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
    fn task_contract_rejects_vacuous_and_ambiguous_inputs() {
        let mut empty = task(Vec::new(), Vec::new());
        empty.required_obligations.clear();
        assert_eq!(
            evaluate_sufficiency(&empty),
            Err(EvaluationError::NoRequiredObligations)
        );

        let mut duplicate = task(Vec::new(), Vec::new());
        duplicate.required_obligations.push("lab-provenance".to_string());
        assert_eq!(
            evaluate_sufficiency(&duplicate),
            Err(EvaluationError::DuplicateObligation(
                "lab-provenance".to_string()
            ))
        );
    }
}
