use std::collections::BTreeSet;

use mycelix_temporal_evidence_integrity_bench::{
    load_visible_fixture, source_sequence_revision_baseline, Disposition, Submission,
    VisibleEvidenceEvent, VisibleTaskInput,
};

fn task(task_id: &str) -> VisibleTaskInput {
    load_visible_fixture()
        .expect("visible fixtures must validate")
        .into_iter()
        .find(|candidate| candidate.task_id == task_id)
        .unwrap_or_else(|| panic!("missing fixture task {task_id}"))
}

fn event_mut<'a>(
    input: &'a mut VisibleTaskInput,
    event_id: &str,
) -> &'a mut VisibleEvidenceEvent {
    input
        .events
        .iter_mut()
        .find(|event| event.event_id == event_id)
        .unwrap_or_else(|| panic!("missing event {event_id}"))
}

fn accepted_ids(submission: &Submission) -> BTreeSet<String> {
    submission
        .timeline
        .iter()
        .chain(submission.retained_history.iter())
        .map(|event| event.event_id.clone())
        .collect()
}

fn semantic_projection(
    submission: &Submission,
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    (
        submission
            .timeline
            .iter()
            .map(|event| event.event_id.clone())
            .collect(),
        submission
            .retained_history
            .iter()
            .map(|event| event.event_id.clone())
            .collect(),
        submission.quarantined_event_ids.clone(),
        submission.unresolved_conflict_groups.clone(),
    )
}

fn disable_application_authority(event: &mut VisibleEvidenceEvent) {
    event.application_authorized = false;
}

fn unauthenticate_issuer(event: &mut VisibleEvidenceEvent) {
    event.issuer_authenticated = false;
}

fn remove_lineage(event: &mut VisibleEvidenceEvent) {
    event.lineage_complete = false;
}

fn change_subject(event: &mut VisibleEvidenceEvent) {
    event.patient_binding = "PAT-B".to_owned();
}

#[test]
fn baseline_projection_is_invariant_to_visible_event_array_permutation() {
    let input = task("T04");
    let expected = semantic_projection(&source_sequence_revision_baseline(&input));

    let mut permuted = input;
    permuted.events.reverse();
    let actual = semantic_projection(&source_sequence_revision_baseline(&permuted));

    assert_eq!(actual, expected);
}

#[test]
fn exact_replay_representative_is_independent_of_local_ingestion_time() {
    let input = task("T06");
    let before = source_sequence_revision_baseline(&input);

    let mut clock_permuted = input;
    for event in &mut clock_permuted.events {
        event.ingested_at_ms = -event.ingested_at_ms;
    }
    let after = source_sequence_revision_baseline(&clock_permuted);

    assert_eq!(
        semantic_projection(&after),
        semantic_projection(&before),
        "replay identity must not depend on which copy arrived first"
    );
    assert_eq!(
        before
            .timeline
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<Vec<_>>(),
        vec!["E12"]
    );
    assert_eq!(before.quarantined_event_ids, vec!["E13"]);
}

#[test]
fn source_last_updated_and_ingestion_clocks_cannot_override_source_sequence() {
    let mut input = task("T03");
    let before = source_sequence_revision_baseline(&input);

    event_mut(&mut input, "E05").source_last_updated_at_ms = Some(9_000_000);
    event_mut(&mut input, "E06").source_last_updated_at_ms = Some(1);
    event_mut(&mut input, "E05").ingested_at_ms = 9_000_000;
    event_mut(&mut input, "E06").ingested_at_ms = 1;

    let after = source_sequence_revision_baseline(&input);
    assert_eq!(
        semantic_projection(&after),
        semantic_projection(&before),
        "explicit source sequence, not local clocks, selects the revision"
    );
    assert_eq!(
        after
            .timeline
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<Vec<_>>(),
        vec!["E06"]
    );
    assert_eq!(
        after
            .retained_history
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<Vec<_>>(),
        vec!["E05"]
    );
}

#[test]
fn trust_boundary_degradation_never_expands_accepted_evidence() {
    let mutations: [fn(&mut VisibleEvidenceEvent); 4] = [
        disable_application_authority,
        unauthenticate_issuer,
        remove_lineage,
        change_subject,
    ];

    for mutate in mutations {
        let original = task("T02");
        let before = source_sequence_revision_baseline(&original);
        let before_ids = accepted_ids(&before);

        let mut degraded = original;
        mutate(event_mut(&mut degraded, "E03"));
        let after = source_sequence_revision_baseline(&degraded);
        let after_ids = accepted_ids(&after);

        assert!(
            after_ids.is_subset(&before_ids),
            "removing trust for E03 must not add accepted evidence: before={before_ids:?}, after={after_ids:?}"
        );
        assert!(
            !after_ids.contains("E03"),
            "E03 must be excluded after its trust boundary is degraded"
        );
    }
}

#[test]
fn equal_source_sequences_remain_unknown_after_clock_perturbation() {
    let mut input = task("T21");
    let before = source_sequence_revision_baseline(&input);

    assert_eq!(before.disposition, Disposition::UnknownOrder);
    assert!(before.timeline.is_empty());
    assert_eq!(
        before
            .retained_history
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<Vec<_>>(),
        vec!["E33", "E34"]
    );

    for event in &mut input.events {
        event.ingested_at_ms = -event.ingested_at_ms;
        event.source_last_updated_at_ms =
            Some(10_000_000 - event.source_last_updated_at_ms.unwrap_or_default());
    }
    let after = source_sequence_revision_baseline(&input);

    assert_eq!(after.disposition, Disposition::UnknownOrder);
    assert!(after.timeline.is_empty());
    assert_eq!(
        after
            .retained_history
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<Vec<_>>(),
        vec!["E33", "E34"]
    );
}

#[test]
fn adding_an_exact_replay_does_not_create_a_second_accepted_assertion() {
    let mut input = task("T06");
    let mut replay = input
        .events
        .iter()
        .find(|event| event.event_id == "E12")
        .expect("E12 replay source")
        .clone();
    replay.event_id = "E99".to_owned();
    replay.ingested_at_ms = -1;
    input.events.push(replay);

    let submission = source_sequence_revision_baseline(&input);
    assert_eq!(submission.timeline.len(), 1);
    assert_eq!(submission.timeline[0].event_id, "E12");
    assert_eq!(accepted_ids(&submission).len(), 1);
    assert!(submission.quarantined_event_ids.contains(&"E13".to_owned()));
    assert!(submission.quarantined_event_ids.contains(&"E99".to_owned()));
}
