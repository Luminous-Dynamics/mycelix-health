use std::collections::BTreeMap;
use std::process::ExitCode;

use mycelix_temporal_evidence_integrity_bench::{
    evaluate, evaluate_batch, load_fixture, load_visible_fixture,
    source_sequence_revision_baseline, TaskScore,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("revision baseline error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let visible = load_visible_fixture()?;
    let answer_key = load_fixture()?;
    let truth_by_id: BTreeMap<&str, _> = answer_key
        .iter()
        .map(|task| (task.task_id.as_str(), task))
        .collect();

    let mut submissions = Vec::with_capacity(visible.len());
    let mut scores: Vec<TaskScore> = Vec::with_capacity(visible.len());

    for input in &visible {
        let truth = truth_by_id
            .get(input.task_id.as_str())
            .ok_or_else(|| format!("no evaluator answer for task {}", input.task_id))?;
        let submission = source_sequence_revision_baseline(input);
        scores.push(evaluate(truth, &submission));
        submissions.push(submission);
    }

    let report = evaluate_batch(&answer_key, &submissions)?;
    println!(
        "task_id\ttask_identity_correct\tdisposition_correct\ttimeline_exact\ttimeline_precision\ttimeline_recall\ttemporal_order_accuracy\thistory_exact\thistory_recall\tquarantine_precision\tquarantine_recall\tconflict_precision\tconflict_recall\tunsafe_acceptances\tprovenance_errors\tduplicate_acceptances\thard_gates_pass\texact_match\tgraph_node_precision\tgraph_node_recall\tgraph_edge_precision\tgraph_edge_recall\tgraph_structurally_valid\tgraph_exact_match"
    );
    for score in &scores {
        print_score(score);
    }

    eprintln!(
        "summary\t{}\tdisposition_correct={}/{}\texact_matches={}/{}\ttimeline_exact={}/{}\thistory_exact={}/{}\tunsafe_acceptances={}\tprovenance_errors={}\tmean_temporal_order_accuracy={:.6}\tmean_quarantine_precision={:.6}\tmean_quarantine_recall={:.6}\tmean_conflict_precision={:.6}\tmean_conflict_recall={:.6}\tgraph_exact_matches={}\tgraph_structural_invalid={}\tmean_graph_node_precision={:.6}\tmean_graph_node_recall={:.6}\tmean_graph_edge_precision={:.6}\tmean_graph_edge_recall={:.6}",
        report.task_count,
        report.disposition_correct,
        report.task_count,
        report.exact_matches,
        report.task_count,
        report.timeline_exact,
        report.task_count,
        report.history_exact,
        report.task_count,
        report.unsafe_acceptance_count,
        report.provenance_error_count,
        report.temporal_order_accuracy_mean,
        report.quarantine_precision_mean,
        report.quarantine_recall_mean,
        report.conflict_precision_mean,
        report.conflict_recall_mean,
        report.graph_exact_matches,
        report.graph_structural_invalid_count,
        report.graph_node_precision_mean,
        report.graph_node_recall_mean,
        report.graph_edge_precision_mean,
        report.graph_edge_recall_mean
    );

    Ok(())
}

fn print_score(score: &TaskScore) {
    println!(
        "{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}",
        score.task_id,
        score.task_identity_correct,
        score.disposition_correct,
        score.timeline_exact,
        score.timeline_precision,
        score.timeline_recall,
        score.temporal_order_accuracy,
        score.history_exact,
        score.history_recall,
        score.quarantine_precision,
        score.quarantine_recall,
        score.conflict_precision,
        score.conflict_recall,
        score.unsafe_acceptances.len(),
        score.provenance_errors.len(),
        score.duplicate_acceptances.len(),
        score.hard_gates_pass,
        score.exact_match,
        score.evidence_graph.node_precision,
        score.evidence_graph.node_recall,
        score.evidence_graph.edge_precision,
        score.evidence_graph.edge_recall,
        score.evidence_graph.structurally_valid,
        score.evidence_graph.exact_match
    );
}
