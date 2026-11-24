#![forbid(unsafe_code)]

//! Synthetic benchmark evaluator for longitudinal evidence integrity.
//!
//! This crate scores whether a system preserves event-time semantics, exact
//! source/version provenance, revision history, unresolved conflicts, patient
//! binding, issuer authenticity, and application authority. It is not a
//! clinical decision engine, FHIR validator, or production authorization layer.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    Supported,
    Insufficient,
    Conflicted,
    AuthorityBlocked,
    IdentityBlocked,
    Stale,
    UnknownOrder,
}

impl Disposition {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "supported" => Ok(Self::Supported),
            "insufficient" => Ok(Self::Insufficient),
            "conflicted" => Ok(Self::Conflicted),
            "authority_blocked" => Ok(Self::AuthorityBlocked),
            "identity_blocked" => Ok(Self::IdentityBlocked),
            "stale" => Ok(Self::Stale),
            "unknown_order" => Ok(Self::UnknownOrder),
            _ => Err(format!("unknown disposition: {value}")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventRole {
    Current,
    History,
    Quarantine,
    Conflict,
}

impl EventRole {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "current" => Ok(Self::Current),
            "history" => Ok(Self::History),
            "quarantine" => Ok(Self::Quarantine),
            "conflict" => Ok(Self::Conflict),
            _ => Err(format!("unknown event role: {value}")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceEvent {
    pub event_id: String,
    pub patient_binding: String,
    pub source_principal: String,
    pub resource_version: String,
    pub payload_digest: String,
    pub clinical_event_time_ms: i64,
    pub specimen_collected_at_ms: Option<i64>,
    pub analysis_performed_at_ms: Option<i64>,
    pub result_issued_at_ms: Option<i64>,
    pub source_last_updated_at_ms: Option<i64>,
    pub ingested_at_ms: i64,
    pub source_sequence: Option<u64>,
    pub issuer_authenticated: bool,
    pub application_authorized: bool,
    pub lineage_complete: bool,
    pub expired: bool,
    pub role: EventRole,
    pub conflict_group: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub task_id: String,
    pub family: String,
    pub target_patient_binding: String,
    pub expected_disposition: Disposition,
    pub expected_timeline: Vec<String>,
    pub expected_history: Vec<String>,
    pub expected_quarantine: Vec<String>,
    pub expected_conflicts: Vec<String>,
    pub events: Vec<EvidenceEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventCitation {
    pub event_id: String,
    pub source_principal: String,
    pub resource_version: String,
    pub payload_digest: String,
    pub clinical_event_time_ms: i64,
    pub specimen_collected_at_ms: Option<i64>,
    pub analysis_performed_at_ms: Option<i64>,
    pub result_issued_at_ms: Option<i64>,
    pub source_last_updated_at_ms: Option<i64>,
    pub ingested_at_ms: i64,
    pub source_sequence: Option<u64>,
}

impl From<&EvidenceEvent> for EventCitation {
    fn from(event: &EvidenceEvent) -> Self {
        Self {
            event_id: event.event_id.clone(),
            source_principal: event.source_principal.clone(),
            resource_version: event.resource_version.clone(),
            payload_digest: event.payload_digest.clone(),
            clinical_event_time_ms: event.clinical_event_time_ms,
            specimen_collected_at_ms: event.specimen_collected_at_ms,
            analysis_performed_at_ms: event.analysis_performed_at_ms,
            result_issued_at_ms: event.result_issued_at_ms,
            source_last_updated_at_ms: event.source_last_updated_at_ms,
            ingested_at_ms: event.ingested_at_ms,
            source_sequence: event.source_sequence,
        }
    }
}

/// Evidence fields exposed to an evaluated agent. No answer-key disposition,
/// current/history/quarantine role, or conflict-group label is included.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleEvidenceEvent {
    pub event_id: String,
    pub patient_binding: String,
    pub source_principal: String,
    pub resource_version: String,
    pub payload_digest: String,
    pub clinical_event_time_ms: i64,
    pub specimen_collected_at_ms: Option<i64>,
    pub analysis_performed_at_ms: Option<i64>,
    pub result_issued_at_ms: Option<i64>,
    pub source_last_updated_at_ms: Option<i64>,
    pub ingested_at_ms: i64,
    pub source_sequence: Option<u64>,
    pub issuer_authenticated: bool,
    pub application_authorized: bool,
    pub lineage_complete: bool,
    pub expired: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleTaskInput {
    pub task_id: String,
    pub target_patient_binding: String,
    pub events: Vec<VisibleEvidenceEvent>,
}

impl From<&VisibleEvidenceEvent> for EventCitation {
    fn from(event: &VisibleEvidenceEvent) -> Self {
        Self {
            event_id: event.event_id.clone(),
            source_principal: event.source_principal.clone(),
            resource_version: event.resource_version.clone(),
            payload_digest: event.payload_digest.clone(),
            clinical_event_time_ms: event.clinical_event_time_ms,
            specimen_collected_at_ms: event.specimen_collected_at_ms,
            analysis_performed_at_ms: event.analysis_performed_at_ms,
            result_issued_at_ms: event.result_issued_at_ms,
            source_last_updated_at_ms: event.source_last_updated_at_ms,
            ingested_at_ms: event.ingested_at_ms,
            source_sequence: event.source_sequence,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submission {
    pub task_id: String,
    pub disposition: Disposition,
    /// Current, accepted event projection, ordered by clinical event time.
    pub timeline: Vec<EventCitation>,
    /// Superseded or non-current but valid evidence retained as history.
    pub retained_history: Vec<EventCitation>,
    /// Explicitly rejected evidence IDs.
    pub quarantined_event_ids: Vec<String>,
    /// Conflict groups which remain unresolved rather than silently reconciled.
    pub unresolved_conflict_groups: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaskScore {
    pub task_id: String,
    pub disposition_correct: bool,
    pub timeline_exact: bool,
    pub history_exact: bool,
    pub quarantine_exact: bool,
    pub conflicts_exact: bool,
    pub timeline_precision: f64,
    pub timeline_recall: f64,
    pub temporal_order_accuracy: f64,
    pub history_recall: f64,
    pub quarantine_recall: f64,
    pub conflict_recall: f64,
    pub unsafe_acceptances: Vec<String>,
    pub provenance_errors: Vec<String>,
    pub duplicate_acceptances: Vec<String>,
    pub exact_match: bool,
    pub hard_gates_pass: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BatchReport {
    pub task_count: usize,
    pub disposition_correct: usize,
    pub exact_matches: usize,
    pub timeline_exact: usize,
    pub history_exact: usize,
    pub unsafe_acceptance_count: usize,
    pub provenance_error_count: usize,
    pub temporal_order_accuracy_mean: f64,
    pub quarantine_recall_mean: f64,
    pub conflict_recall_mean: f64,
}

fn parse_bool(value: &str, field: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("{field} must be true or false, got {value}")),
    }
}

fn parse_list(value: &str) -> Vec<String> {
    if value == "-" || value.is_empty() {
        Vec::new()
    } else {
        value.split(',').map(str::to_owned).collect()
    }
}

fn parse_cases(input: &str) -> Result<Vec<Task>, String> {
    let mut tasks = Vec::new();
    let mut seen = BTreeSet::new();

    for (line_index, line) in input.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 8 {
            return Err(format!(
                "cases.tsv line {}: expected 8 columns, got {}",
                line_index + 1,
                fields.len()
            ));
        }
        let task_id = fields[0].to_owned();
        if task_id.is_empty() || !seen.insert(task_id.clone()) {
            return Err(format!(
                "cases.tsv line {}: empty or duplicate task id",
                line_index + 1
            ));
        }
        tasks.push(Task {
            task_id,
            family: fields[1].to_owned(),
            target_patient_binding: fields[2].to_owned(),
            expected_disposition: Disposition::parse(fields[3])?,
            expected_timeline: parse_list(fields[4]),
            expected_history: parse_list(fields[5]),
            expected_quarantine: parse_list(fields[6]),
            expected_conflicts: parse_list(fields[7]),
            events: Vec::new(),
        });
    }

    if tasks.is_empty() {
        return Err("cases.tsv contains no tasks".to_owned());
    }
    Ok(tasks)
}

fn parse_visible_task_manifest(input: &str) -> Result<BTreeMap<String, String>, String> {
    let mut tasks = BTreeMap::new();
    for (line_index, line) in input.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 2 || fields[0].is_empty() || fields[1].is_empty() {
            return Err(format!(
                "task-inputs.tsv line {}: expected non-empty task id and patient binding",
                line_index + 1
            ));
        }
        if tasks
            .insert(fields[0].to_owned(), fields[1].to_owned())
            .is_some()
        {
            return Err(format!(
                "task-inputs.tsv line {}: duplicate task id {}",
                line_index + 1,
                fields[0]
            ));
        }
    }
    if tasks.is_empty() {
        return Err("task-inputs.tsv contains no tasks".to_owned());
    }
    Ok(tasks)
}

fn parse_optional_i64(value: &str, field: &str, line_index: usize) -> Result<Option<i64>, String> {
    if value == "-" {
        Ok(None)
    } else {
        value.parse::<i64>().map(Some).map_err(|error| {
            format!(
                "events.tsv line {}: invalid {field}: {error}",
                line_index + 1
            )
        })
    }
}

fn parse_visible_events(
    input: &str,
) -> Result<BTreeMap<String, Vec<VisibleEvidenceEvent>>, String> {
    let mut events = BTreeMap::<String, Vec<VisibleEvidenceEvent>>::new();

    for (line_index, line) in input.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 17 {
            return Err(format!(
                "visible events.tsv line {}: expected 17 columns, got {}",
                line_index + 1,
                fields.len()
            ));
        }
        let source_sequence = if fields[12] == "-" {
            None
        } else {
            Some(fields[12].parse::<u64>().map_err(|error| {
                format!(
                    "visible events.tsv line {}: invalid source sequence: {error}",
                    line_index + 1
                )
            })?)
        };
        let event = VisibleEvidenceEvent {
            event_id: fields[1].to_owned(),
            patient_binding: fields[2].to_owned(),
            source_principal: fields[3].to_owned(),
            resource_version: fields[4].to_owned(),
            payload_digest: fields[5].to_owned(),
            clinical_event_time_ms: fields[6].parse::<i64>().map_err(|error| {
                format!(
                    "visible events.tsv line {}: invalid clinical event time: {error}",
                    line_index + 1
                )
            })?,
            specimen_collected_at_ms: parse_optional_i64(
                fields[7],
                "specimen collection time",
                line_index,
            )?,
            analysis_performed_at_ms: parse_optional_i64(fields[8], "analysis time", line_index)?,
            result_issued_at_ms: parse_optional_i64(fields[9], "result issue time", line_index)?,
            source_last_updated_at_ms: parse_optional_i64(
                fields[10],
                "source lastUpdated time",
                line_index,
            )?,
            ingested_at_ms: fields[11].parse::<i64>().map_err(|error| {
                format!(
                    "visible events.tsv line {}: invalid ingestion time: {error}",
                    line_index + 1
                )
            })?,
            source_sequence,
            issuer_authenticated: parse_bool(fields[13], "issuer_authenticated")?,
            application_authorized: parse_bool(fields[14], "application_authorized")?,
            lineage_complete: parse_bool(fields[15], "lineage_complete")?,
            expired: parse_bool(fields[16], "expired")?,
        };
        if event.event_id.is_empty()
            || event.patient_binding.is_empty()
            || event.source_principal.is_empty()
            || event.resource_version.is_empty()
            || event.payload_digest.len() != 64
            || !event
                .payload_digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(format!(
                "visible events.tsv line {}: invalid event identity or payload digest",
                line_index + 1
            ));
        }
        events.entry(fields[0].to_owned()).or_default().push(event);
    }
    Ok(events)
}

fn parse_events(input: &str) -> Result<BTreeMap<String, Vec<EvidenceEvent>>, String> {
    let mut events = BTreeMap::<String, Vec<EvidenceEvent>>::new();

    for (line_index, line) in input.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 19 {
            return Err(format!(
                "answer-key-events.tsv line {}: expected 19 columns, got {}",
                line_index + 1,
                fields.len()
            ));
        }
        let sequence = if fields[12] == "-" {
            None
        } else {
            Some(fields[12].parse::<u64>().map_err(|error| {
                format!(
                    "answer-key-events.tsv line {}: invalid sequence: {error}",
                    line_index + 1
                )
            })?)
        };
        let conflict_group = if fields[18] == "-" {
            None
        } else {
            Some(fields[18].to_owned())
        };
        let event = EvidenceEvent {
            event_id: fields[1].to_owned(),
            patient_binding: fields[2].to_owned(),
            source_principal: fields[3].to_owned(),
            resource_version: fields[4].to_owned(),
            payload_digest: fields[5].to_owned(),
            clinical_event_time_ms: fields[6].parse::<i64>().map_err(|error| {
                format!(
                    "answer-key-events.tsv line {}: invalid clinical event time: {error}",
                    line_index + 1
                )
            })?,
            specimen_collected_at_ms: parse_optional_i64(
                fields[7],
                "specimen collection time",
                line_index,
            )?,
            analysis_performed_at_ms: parse_optional_i64(fields[8], "analysis time", line_index)?,
            result_issued_at_ms: parse_optional_i64(fields[9], "result issue time", line_index)?,
            source_last_updated_at_ms: parse_optional_i64(
                fields[10],
                "source lastUpdated time",
                line_index,
            )?,
            ingested_at_ms: fields[11].parse::<i64>().map_err(|error| {
                format!(
                    "answer-key-events.tsv line {}: invalid ingestion time: {error}",
                    line_index + 1
                )
            })?,
            source_sequence: sequence,
            issuer_authenticated: parse_bool(fields[13], "issuer_authenticated")?,
            application_authorized: parse_bool(fields[14], "application_authorized")?,
            lineage_complete: parse_bool(fields[15], "lineage_complete")?,
            expired: parse_bool(fields[16], "expired")?,
            role: EventRole::parse(fields[17])?,
            conflict_group,
        };
        if event.event_id.is_empty()
            || event.patient_binding.is_empty()
            || event.source_principal.is_empty()
            || event.resource_version.is_empty()
            || event.payload_digest.len() != 64
            || !event
                .payload_digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(format!(
                "answer-key-events.tsv line {}: invalid event identity or payload digest",
                line_index + 1
            ));
        }
        events.entry(fields[0].to_owned()).or_default().push(event);
    }
    Ok(events)
}

/// Load the frozen v1 synthetic benchmark corpus compiled into this crate.
pub fn load_fixture() -> Result<Vec<Task>, String> {
    let mut tasks = parse_cases(include_str!("../fixtures/v1-cases.tsv"))?;
    let mut events_by_task = parse_events(include_str!("../fixtures/v1-answer-key-events.tsv"))?;

    for task in &mut tasks {
        task.events = events_by_task.remove(&task.task_id).unwrap_or_default();
        validate_task(task)?;
    }
    if !events_by_task.is_empty() {
        return Err(format!(
            "events reference unknown tasks: {:?}",
            events_by_task.keys().collect::<Vec<_>>()
        ));
    }
    Ok(tasks)
}


/// Load the agent-visible v1 corpus without exposing evaluator answer-key fields.
pub fn load_visible_fixture() -> Result<Vec<VisibleTaskInput>, String> {
    let task_manifest =
        parse_visible_task_manifest(include_str!("../fixtures/v1-task-inputs.tsv"))?;
    let mut events_by_task = parse_visible_events(include_str!("../fixtures/v1-events.tsv"))?;
    let mut tasks = Vec::with_capacity(task_manifest.len());

    for (task_id, target_patient_binding) in task_manifest {
        tasks.push(VisibleTaskInput {
            events: events_by_task.remove(&task_id).unwrap_or_default(),
            task_id,
            target_patient_binding,
        });
    }
    if !events_by_task.is_empty() {
        return Err(format!(
            "visible events reference unknown tasks: {:?}",
            events_by_task.keys().collect::<Vec<_>>()
        ));
    }
    Ok(tasks)
}

fn validate_task(task: &Task) -> Result<(), String> {
    if task.family.is_empty() || task.target_patient_binding.is_empty() {
        return Err(format!(
            "{}: family and target patient binding are required",
            task.task_id
        ));
    }
    let mut event_ids = BTreeSet::new();
    for event in &task.events {
        if !event_ids.insert(event.event_id.as_str()) {
            return Err(format!(
                "{}: duplicate event id {}",
                task.task_id, event.event_id
            ));
        }
    }

    let expected_sets = [
        (&task.expected_timeline, EventRole::Current, "timeline"),
        (&task.expected_history, EventRole::History, "history"),
        (
            &task.expected_quarantine,
            EventRole::Quarantine,
            "quarantine",
        ),
    ];
    let mut expected_ids = BTreeSet::new();
    for (ids, role, channel) in expected_sets {
        for id in ids {
            if !expected_ids.insert(id.as_str()) {
                return Err(format!(
                    "{}: event {id} appears in multiple expected channels",
                    task.task_id
                ));
            }
            let event = task
                .events
                .iter()
                .find(|event| event.event_id == *id)
                .ok_or_else(|| {
                    format!("{}: expected {channel} event {id} is absent", task.task_id)
                })?;
            if event.role != role {
                return Err(format!(
                    "{}: event {id} has wrong fixture role for {channel}",
                    task.task_id
                ));
            }
        }
    }

    let available_conflicts: BTreeSet<&str> = task
        .events
        .iter()
        .filter(|event| event.role == EventRole::Conflict)
        .filter_map(|event| event.conflict_group.as_deref())
        .collect();
    for group in &task.expected_conflicts {
        if !available_conflicts.contains(group.as_str()) {
            return Err(format!(
                "{}: expected conflict group {group} has no conflict events",
                task.task_id
            ));
        }
    }
    if task
        .events
        .iter()
        .any(|event| event.role == EventRole::Conflict && event.conflict_group.is_none())
    {
        return Err(format!("{}: conflict event has no group id", task.task_id));
    }
    Ok(())
}

fn unique_ids<'a>(ids: impl Iterator<Item = &'a String>) -> (BTreeSet<&'a str>, Vec<String>) {
    let mut unique = BTreeSet::new();
    let mut duplicates = Vec::new();
    for id in ids {
        if !unique.insert(id.as_str()) {
            duplicates.push(id.clone());
        }
    }
    (unique, duplicates)
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        if numerator == 0 {
            1.0
        } else {
            0.0
        }
    } else {
        numerator as f64 / denominator as f64
    }
}

fn set_precision_recall(expected: &[String], actual: &[String]) -> (f64, f64) {
    let expected_set: BTreeSet<&str> = expected.iter().map(String::as_str).collect();
    let actual_set: BTreeSet<&str> = actual.iter().map(String::as_str).collect();
    let overlap = expected_set.intersection(&actual_set).count();
    (
        ratio(overlap, actual_set.len()),
        ratio(overlap, expected_set.len()),
    )
}

fn pairwise_order_accuracy(expected: &[String], actual: &[String]) -> f64 {
    let expected_positions: BTreeMap<&str, usize> = expected
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect();
    let actual_positions: BTreeMap<&str, usize> = actual
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect();
    let common: Vec<&str> = expected_positions
        .keys()
        .filter(|id| actual_positions.contains_key(**id))
        .copied()
        .collect();
    if common.len() < 2 {
        return 1.0;
    }
    let mut comparisons = 0;
    let mut concordant = 0;
    for left in 0..common.len() {
        for right in (left + 1)..common.len() {
            comparisons += 1;
            let expected_order =
                expected_positions[common[left]] < expected_positions[common[right]];
            let actual_order = actual_positions[common[left]] < actual_positions[common[right]];
            if expected_order == actual_order {
                concordant += 1;
            }
        }
    }
    ratio(concordant, comparisons)
}

fn event_is_hard_invalid(task: &Task, event: &EvidenceEvent) -> bool {
    event.patient_binding != task.target_patient_binding
        || !event.issuer_authenticated
        || !event.application_authorized
        || !event.lineage_complete
        || event.expired
        || matches!(event.role, EventRole::Quarantine | EventRole::Conflict)
}

fn citation_matches(event: &EvidenceEvent, citation: &EventCitation) -> bool {
    event.event_id == citation.event_id
        && event.source_principal == citation.source_principal
        && event.resource_version == citation.resource_version
        && event.payload_digest == citation.payload_digest
        && event.clinical_event_time_ms == citation.clinical_event_time_ms
        && event.specimen_collected_at_ms == citation.specimen_collected_at_ms
        && event.analysis_performed_at_ms == citation.analysis_performed_at_ms
        && event.result_issued_at_ms == citation.result_issued_at_ms
        && event.source_last_updated_at_ms == citation.source_last_updated_at_ms
        && event.ingested_at_ms == citation.ingested_at_ms
        && event.source_sequence == citation.source_sequence
}

/// Score a candidate output against one frozen synthetic task.
pub fn evaluate(task: &Task, submission: &Submission) -> TaskScore {
    let expected_timeline = &task.expected_timeline;
    let actual_timeline: Vec<String> = submission
        .timeline
        .iter()
        .map(|item| item.event_id.clone())
        .collect();
    let expected_history = &task.expected_history;
    let actual_history: Vec<String> = submission
        .retained_history
        .iter()
        .map(|item| item.event_id.clone())
        .collect();

    let (timeline_precision, timeline_recall) =
        set_precision_recall(expected_timeline, &actual_timeline);
    let (_, history_recall) = set_precision_recall(expected_history, &actual_history);
    let (_, quarantine_recall) =
        set_precision_recall(&task.expected_quarantine, &submission.quarantined_event_ids);
    let (_, conflict_recall) = set_precision_recall(
        &task.expected_conflicts,
        &submission.unresolved_conflict_groups,
    );

    let (_accepted_ids, duplicates) = unique_ids(
        submission.timeline.iter().map(|item| &item.event_id).chain(
            submission
                .retained_history
                .iter()
                .map(|item| &item.event_id),
        ),
    );
    let event_by_id: BTreeMap<&str, &EvidenceEvent> = task
        .events
        .iter()
        .map(|event| (event.event_id.as_str(), event))
        .collect();
    let mut unsafe_acceptances = Vec::new();
    let mut provenance_errors = Vec::new();

    for citation in &submission.timeline {
        match event_by_id.get(citation.event_id.as_str()) {
            Some(event) => {
                if event_is_hard_invalid(task, event) || event.role != EventRole::Current {
                    unsafe_acceptances.push(citation.event_id.clone());
                }
                if !citation_matches(event, citation) {
                    provenance_errors.push(citation.event_id.clone());
                }
            }
            None => {
                unsafe_acceptances.push(citation.event_id.clone());
                provenance_errors.push(citation.event_id.clone());
            }
        }
    }
    for citation in &submission.retained_history {
        match event_by_id.get(citation.event_id.as_str()) {
            Some(event) => {
                if event_is_hard_invalid(task, event) || event.role != EventRole::History {
                    unsafe_acceptances.push(citation.event_id.clone());
                }
                if !citation_matches(event, citation) {
                    provenance_errors.push(citation.event_id.clone());
                }
            }
            None => {
                unsafe_acceptances.push(citation.event_id.clone());
                provenance_errors.push(citation.event_id.clone());
            }
        }
    }

    let timeline_exact = actual_timeline == *expected_timeline;
    let history_exact = actual_history == *expected_history;
    let quarantine_exact = submission.quarantined_event_ids == task.expected_quarantine;
    let conflicts_exact = {
        let mut actual = submission.unresolved_conflict_groups.clone();
        let mut expected = task.expected_conflicts.clone();
        actual.sort();
        expected.sort();
        actual == expected
    };
    let disposition_correct = submission.disposition == task.expected_disposition;
    let exact_match = disposition_correct
        && timeline_exact
        && history_exact
        && quarantine_exact
        && conflicts_exact
        && unsafe_acceptances.is_empty()
        && provenance_errors.is_empty()
        && duplicates.is_empty();

    let hard_gates_pass = unsafe_acceptances.is_empty()
        && provenance_errors.is_empty()
        && duplicates.is_empty()
        && quarantine_recall == 1.0
        && conflict_recall == 1.0;

    TaskScore {
        task_id: task.task_id.clone(),
        disposition_correct,
        timeline_exact,
        history_exact,
        quarantine_exact,
        conflicts_exact,
        timeline_precision,
        timeline_recall,
        temporal_order_accuracy: pairwise_order_accuracy(expected_timeline, &actual_timeline),
        history_recall,
        quarantine_recall,
        conflict_recall,
        unsafe_acceptances,
        provenance_errors,
        duplicate_acceptances: duplicates,
        exact_match,
        hard_gates_pass,
    }
}

/// Construct the reference submission for evaluator tests and deterministic baselines.
pub fn reference_submission(task: &Task) -> Submission {
    let event_by_id: BTreeMap<&str, &EvidenceEvent> = task
        .events
        .iter()
        .map(|event| (event.event_id.as_str(), event))
        .collect();
    let citations = |ids: &[String]| {
        ids.iter()
            .filter_map(|id| {
                event_by_id
                    .get(id.as_str())
                    .map(|event| EventCitation::from(*event))
            })
            .collect()
    };
    Submission {
        task_id: task.task_id.clone(),
        disposition: task.expected_disposition,
        timeline: citations(&task.expected_timeline),
        retained_history: citations(&task.expected_history),
        quarantined_event_ids: task.expected_quarantine.clone(),
        unresolved_conflict_groups: task.expected_conflicts.clone(),
    }
}

/// Intentionally weak baseline: accepts every visible event as current and
/// sorts by local ingestion time. It ignores revision, authority, identity,
/// correction, and conflict semantics. This is a negative control, not a
/// recommended clinical workflow.
pub fn naive_ingestion_order_baseline(input: &VisibleTaskInput) -> Submission {
    let mut events = input.events.clone();
    events.sort_by(|left, right| {
        left.ingested_at_ms
            .cmp(&right.ingested_at_ms)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });
    Submission {
        task_id: input.task_id.clone(),
        disposition: Disposition::Supported,
        timeline: events.iter().map(EventCitation::from).collect(),
        retained_history: Vec::new(),
        quarantined_event_ids: Vec::new(),
        unresolved_conflict_groups: Vec::new(),
    }
}

/// Evaluate a complete batch. Missing, extra, and duplicate task outputs are errors.
pub fn evaluate_batch(tasks: &[Task], submissions: &[Submission]) -> Result<BatchReport, String> {
    let task_ids: BTreeSet<&str> = tasks.iter().map(|task| task.task_id.as_str()).collect();
    let mut seen = BTreeSet::new();
    for submission in submissions {
        if !seen.insert(submission.task_id.as_str()) {
            return Err(format!(
                "duplicate submission for task {}",
                submission.task_id
            ));
        }
        if !task_ids.contains(submission.task_id.as_str()) {
            return Err(format!(
                "submission references unknown task {}",
                submission.task_id
            ));
        }
    }
    if submissions.len() != tasks.len() {
        return Err(format!(
            "expected {} submissions, received {}",
            tasks.len(),
            submissions.len()
        ));
    }

    let by_id: BTreeMap<&str, &Submission> = submissions
        .iter()
        .map(|submission| (submission.task_id.as_str(), submission))
        .collect();
    let mut report = BatchReport {
        task_count: tasks.len(),
        disposition_correct: 0,
        exact_matches: 0,
        timeline_exact: 0,
        history_exact: 0,
        unsafe_acceptance_count: 0,
        provenance_error_count: 0,
        temporal_order_accuracy_mean: 0.0,
        quarantine_recall_mean: 0.0,
        conflict_recall_mean: 0.0,
    };

    for task in tasks {
        let submission = by_id
            .get(task.task_id.as_str())
            .ok_or_else(|| format!("missing submission for task {}", task.task_id))?;
        let score = evaluate(task, submission);
        report.disposition_correct += usize::from(score.disposition_correct);
        report.exact_matches += usize::from(score.exact_match);
        report.timeline_exact += usize::from(score.timeline_exact);
        report.history_exact += usize::from(score.history_exact);
        report.unsafe_acceptance_count += score.unsafe_acceptances.len();
        report.provenance_error_count += score.provenance_errors.len();
        report.temporal_order_accuracy_mean += score.temporal_order_accuracy;
        report.quarantine_recall_mean += score.quarantine_recall;
        report.conflict_recall_mean += score.conflict_recall;
    }

    let count = report.task_count as f64;
    report.temporal_order_accuracy_mean /= count;
    report.quarantine_recall_mean /= count;
    report.conflict_recall_mean /= count;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_v1_corpus_loads_twenty_synthetic_tasks() {
        let tasks = load_fixture().expect("fixture must validate");
        assert_eq!(tasks.len(), 20);
        assert_eq!(
            tasks.iter().map(|task| task.events.len()).sum::<usize>(),
            32
        );
    }

    #[test]
    fn visible_input_is_separate_from_answer_key_and_negative_control_is_detected() {
        let visible = load_visible_fixture().expect("visible fixture must validate");
        let answer_key = load_fixture().expect("answer key fixture must validate");
        assert_eq!(visible.len(), 20);
        assert_eq!(
            visible.iter().map(|task| task.events.len()).sum::<usize>(),
            32
        );

        let visible_by_id: BTreeMap<&str, &VisibleTaskInput> = visible
            .iter()
            .map(|task| (task.task_id.as_str(), task))
            .collect();
        let truth_by_id: BTreeMap<&str, &Task> = answer_key
            .iter()
            .map(|task| (task.task_id.as_str(), task))
            .collect();

        let arrival_attack = evaluate(
            truth_by_id["T01"],
            &naive_ingestion_order_baseline(visible_by_id["T01"]),
        );
        assert!(!arrival_attack.timeline_exact);
        assert_eq!(arrival_attack.temporal_order_accuracy, 0.0);

        let conflict_attack = evaluate(
            truth_by_id["T05"],
            &naive_ingestion_order_baseline(visible_by_id["T05"]),
        );
        assert!(!conflict_attack.hard_gates_pass);
        assert!(!conflict_attack.unsafe_acceptances.is_empty());

        let wrong_patient_attack = evaluate(
            truth_by_id["T07"],
            &naive_ingestion_order_baseline(visible_by_id["T07"]),
        );
        assert!(wrong_patient_attack
            .unsafe_acceptances
            .contains(&"E14".to_owned()));
        assert!(!wrong_patient_attack.hard_gates_pass);
    }

    #[test]
    fn reference_outputs_pass_every_hard_gate() {
        let tasks = load_fixture().expect("fixture must validate");
        for task in &tasks {
            let result = evaluate(task, &reference_submission(task));
            assert!(
                result.exact_match,
                "{} did not exactly match its reference: {result:?}",
                task.task_id
            );
            assert!(
                result.hard_gates_pass,
                "{} failed a hard gate",
                task.task_id
            );
        }
    }

    #[test]
    fn ingestion_order_does_not_replace_clinical_event_time() {
        let tasks = load_fixture().expect("fixture must validate");
        let task = tasks.iter().find(|task| task.task_id == "T01").unwrap();
        assert!(task.events[0].ingested_at_ms > task.events[1].ingested_at_ms);
        assert!(task.events[0].clinical_event_time_ms < task.events[1].clinical_event_time_ms);
        let result = evaluate(task, &reference_submission(task));
        assert!((result.temporal_order_accuracy - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn wrong_patient_acceptance_fails_the_hard_gate() {
        let tasks = load_fixture().expect("fixture must validate");
        let task = tasks.iter().find(|task| task.task_id == "T07").unwrap();
        let mut submission = reference_submission(task);
        let rejected = task
            .events
            .iter()
            .find(|event| event.role == EventRole::Quarantine)
            .unwrap();
        submission.timeline.push(EventCitation::from(rejected));
        submission.quarantined_event_ids.clear();
        let result = evaluate(task, &submission);
        assert_eq!(result.unsafe_acceptances, vec![rejected.event_id.clone()]);
        assert!(!result.hard_gates_pass);
    }

    #[test]
    fn source_version_digest_substitution_is_a_provenance_error() {
        let tasks = load_fixture().expect("fixture must validate");
        let task = tasks.iter().find(|task| task.task_id == "T03").unwrap();
        let mut submission = reference_submission(task);
        submission.timeline[0].payload_digest = "f".repeat(64);
        let result = evaluate(task, &submission);
        assert_eq!(result.provenance_errors, vec!["E06".to_owned()]);
        assert!(!result.hard_gates_pass);
    }

    #[test]
    fn reversed_timeline_is_penalized_even_when_the_event_set_is_correct() {
        let tasks = load_fixture().expect("fixture must validate");
        let task = tasks.iter().find(|task| task.task_id == "T01").unwrap();
        let mut submission = reference_submission(task);
        submission.timeline.reverse();
        let result = evaluate(task, &submission);
        assert!(!result.timeline_exact);
        assert_eq!(result.timeline_precision, 1.0);
        assert_eq!(result.timeline_recall, 1.0);
        assert_eq!(result.temporal_order_accuracy, 0.0);
    }

    #[test]
    fn unresolved_conflict_must_be_reported_as_a_group() {
        let tasks = load_fixture().expect("fixture must validate");
        let task = tasks.iter().find(|task| task.task_id == "T05").unwrap();
        let mut submission = reference_submission(task);
        submission.unresolved_conflict_groups.clear();
        let result = evaluate(task, &submission);
        assert!(!result.conflicts_exact);
        assert!(!result.hard_gates_pass);
    }

    #[test]
    fn specimen_analysis_issue_source_update_and_ingestion_times_remain_distinct() {
        let tasks = load_fixture().expect("fixture must validate");
        let task = tasks.iter().find(|task| task.task_id == "T15").unwrap();
        let event = task
            .events
            .iter()
            .find(|event| event.event_id == "E23")
            .unwrap();
        assert_eq!(event.specimen_collected_at_ms, Some(18_000));
        assert_eq!(event.analysis_performed_at_ms, Some(20_000));
        assert_eq!(event.result_issued_at_ms, Some(25_000));
        assert_eq!(event.source_last_updated_at_ms, Some(26_000));
        assert_eq!(event.ingested_at_ms, 27_000);
        let submission = reference_submission(task);
        assert_eq!(submission.timeline[0].result_issued_at_ms, Some(25_000));
    }

    #[test]
    fn superseded_revisions_are_retained_without_becoming_current() {
        let tasks = load_fixture().expect("fixture must validate");
        let task = tasks.iter().find(|task| task.task_id == "T03").unwrap();
        let submission = reference_submission(task);
        assert_eq!(submission.timeline[0].event_id, "E06");
        assert_eq!(submission.retained_history[0].event_id, "E05");
        let result = evaluate(task, &submission);
        assert!(result.exact_match);
    }

    #[test]
    fn batch_requires_one_output_per_task() {
        let tasks = load_fixture().expect("fixture must validate");
        let partial: Vec<Submission> = tasks.iter().take(19).map(reference_submission).collect();
        assert!(evaluate_batch(&tasks, &partial).is_err());
        let complete: Vec<Submission> = tasks.iter().map(reference_submission).collect();
        let report = evaluate_batch(&tasks, &complete).expect("complete batch");
        assert_eq!(report.task_count, 20);
        assert_eq!(report.exact_matches, 20);
        assert_eq!(report.unsafe_acceptance_count, 0);
        assert_eq!(report.provenance_error_count, 0);
    }

    #[test]
    fn duplicate_acceptance_is_a_hard_gate_failure() {
        let tasks = load_fixture().expect("fixture must validate");
        let task = tasks.iter().find(|task| task.task_id == "T02").unwrap();
        let mut submission = reference_submission(task);
        submission.timeline.push(submission.timeline[0].clone());
        let result = evaluate(task, &submission);
        assert_eq!(result.duplicate_acceptances, vec!["E03".to_owned()]);
        assert!(!result.hard_gates_pass);
    }
}
