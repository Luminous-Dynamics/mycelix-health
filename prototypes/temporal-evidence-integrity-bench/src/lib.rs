#![forbid(unsafe_code)]

//! Synthetic benchmark evaluator for longitudinal evidence integrity.
//!
//! This crate scores whether a system preserves event-time semantics, exact
//! source/version provenance, revision history, unresolved conflicts, patient
//! binding, issuer authenticity, and application authority. It is not a
//! clinical decision engine, FHIR validator, or production authorization layer.

use std::collections::{BTreeMap, BTreeSet};

pub use mycelix_temporal_evidence_agent_contract::{
    build_evidence_graph_v1, load_visible_fixture, Disposition, EventCitation, EvidenceGraphV1,
    EvidenceRelationKindV1, EvidenceRelationV1, Submission, VisibleEvidenceEvent, VisibleTaskInput,
};

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
    pub resource_type: String,
    pub resource_id: String,
    /// Canonical proposition + subject + effective-time identity, where supplied.
    pub assertion_key: Option<String>,
    /// Synthetic normalized value used to detect cross-source disagreement.
    pub normalized_assertion_value: Option<String>,
    pub payload_digest: String,
    pub measurement_method: Option<String>,
    pub measurement_unit: Option<String>,
    pub terminology_system: Option<String>,
    pub terminology_version: Option<String>,
    pub terminology_code: Option<String>,
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

impl From<&EvidenceEvent> for EventCitation {
    fn from(event: &EvidenceEvent) -> Self {
        Self {
            event_id: event.event_id.clone(),
            source_principal: event.source_principal.clone(),
            resource_version: event.resource_version.clone(),
            resource_type: event.resource_type.clone(),
            resource_id: event.resource_id.clone(),
            assertion_key: event.assertion_key.clone(),
            normalized_assertion_value: event.normalized_assertion_value.clone(),
            payload_digest: event.payload_digest.clone(),
            measurement_method: event.measurement_method.clone(),
            measurement_unit: event.measurement_unit.clone(),
            terminology_system: event.terminology_system.clone(),
            terminology_version: event.terminology_version.clone(),
            terminology_code: event.terminology_code.clone(),
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

fn visible_graph_event(event: &EvidenceEvent) -> VisibleEvidenceEvent {
    VisibleEvidenceEvent {
        event_id: event.event_id.clone(),
        patient_binding: event.patient_binding.clone(),
        source_principal: event.source_principal.clone(),
        resource_version: event.resource_version.clone(),
        resource_type: event.resource_type.clone(),
        resource_id: event.resource_id.clone(),
        assertion_key: event.assertion_key.clone(),
        normalized_assertion_value: event.normalized_assertion_value.clone(),
        payload_digest: event.payload_digest.clone(),
        measurement_method: event.measurement_method.clone(),
        measurement_unit: event.measurement_unit.clone(),
        terminology_system: event.terminology_system.clone(),
        terminology_version: event.terminology_version.clone(),
        terminology_code: event.terminology_code.clone(),
        clinical_event_time_ms: event.clinical_event_time_ms,
        specimen_collected_at_ms: event.specimen_collected_at_ms,
        analysis_performed_at_ms: event.analysis_performed_at_ms,
        result_issued_at_ms: event.result_issued_at_ms,
        source_last_updated_at_ms: event.source_last_updated_at_ms,
        ingested_at_ms: event.ingested_at_ms,
        source_sequence: event.source_sequence,
        issuer_authenticated: event.issuer_authenticated,
        application_authorized: event.application_authorized,
        lineage_complete: event.lineage_complete,
        expired: event.expired,
    }
}

fn expected_evidence_graph(task: &Task) -> EvidenceGraphV1 {
    let visible_events: Vec<VisibleEvidenceEvent> =
        task.events.iter().map(visible_graph_event).collect();
    build_evidence_graph_v1(&visible_events)
}

/// Score candidate graph structure independently from timeline and clinical-safety metrics.
///
/// Relations are evidence/provenance structure, not clinical truth. This scorer checks
/// graph coverage, unsupported edges, duplicate output, and reference integrity.
pub fn evaluate_evidence_graph_v1(
    expected: &EvidenceGraphV1,
    actual: &EvidenceGraphV1,
) -> EvidenceGraphScoreV1 {
    let expected_nodes: BTreeSet<&str> = expected.nodes.iter().map(String::as_str).collect();
    let actual_nodes: BTreeSet<&str> = actual.nodes.iter().map(String::as_str).collect();
    let expected_relations: BTreeSet<&EvidenceRelationV1> =
        expected.relations.iter().collect();
    let actual_relations: BTreeSet<&EvidenceRelationV1> = actual.relations.iter().collect();

    let duplicate_node_count = actual.nodes.len().saturating_sub(actual_nodes.len());
    let duplicate_relation_count =
        actual.relations.len().saturating_sub(actual_relations.len());
    let unknown_node_count = actual_nodes.difference(&expected_nodes).count();
    let dangling_relation_count = actual
        .relations
        .iter()
        .filter(|relation| {
            relation.from_event_id == relation.to_event_id
                || !actual_nodes.contains(relation.from_event_id.as_str())
                || !actual_nodes.contains(relation.to_event_id.as_str())
                || !expected_nodes.contains(relation.from_event_id.as_str())
                || !expected_nodes.contains(relation.to_event_id.as_str())
        })
        .count();

    let node_overlap = expected_nodes.intersection(&actual_nodes).count();
    let edge_overlap = expected_relations.intersection(&actual_relations).count();
    let node_precision = ratio(node_overlap, actual_nodes.len());
    let node_recall = ratio(node_overlap, expected_nodes.len());
    let edge_precision = ratio(edge_overlap, actual_relations.len());
    let edge_recall = ratio(edge_overlap, expected_relations.len());

    let structurally_valid = duplicate_node_count == 0
        && duplicate_relation_count == 0
        && dangling_relation_count == 0
        && unknown_node_count == 0;
    let exact_match = structurally_valid
        && expected_nodes == actual_nodes
        && expected_relations == actual_relations;

    EvidenceGraphScoreV1 {
        node_precision,
        node_recall,
        edge_precision,
        edge_recall,
        structurally_valid,
        duplicate_node_count,
        duplicate_relation_count,
        dangling_relation_count,
        unknown_node_count,
        exact_match,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaskScore {
    pub task_id: String,
    pub task_identity_correct: bool,
    pub disposition_correct: bool,
    pub timeline_exact: bool,
    pub history_exact: bool,
    pub quarantine_exact: bool,
    pub conflicts_exact: bool,
    pub timeline_precision: f64,
    pub timeline_recall: f64,
    pub temporal_order_accuracy: f64,
    pub history_recall: f64,
    pub quarantine_precision: f64,
    pub quarantine_recall: f64,
    pub conflict_precision: f64,
    pub conflict_recall: f64,
    pub unsafe_acceptances: Vec<String>,
    pub provenance_errors: Vec<String>,
    pub duplicate_acceptances: Vec<String>,
    pub evidence_graph: EvidenceGraphScoreV1,
    pub exact_match: bool,
    pub hard_gates_pass: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceGraphScoreV1 {
    pub node_precision: f64,
    pub node_recall: f64,
    pub edge_precision: f64,
    pub edge_recall: f64,
    pub structurally_valid: bool,
    pub duplicate_node_count: usize,
    pub duplicate_relation_count: usize,
    pub dangling_relation_count: usize,
    pub unknown_node_count: usize,
    pub exact_match: bool,
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
    pub quarantine_precision_mean: f64,
    pub quarantine_recall_mean: f64,
    pub conflict_precision_mean: f64,
    pub conflict_recall_mean: f64,
    pub graph_exact_matches: usize,
    pub graph_structural_invalid_count: usize,
    pub graph_node_precision_mean: f64,
    pub graph_node_recall_mean: f64,
    pub graph_edge_precision_mean: f64,
    pub graph_edge_recall_mean: f64,
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

fn optional_text(value: &str) -> Option<String> {
    if value == "-" || value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn parse_events(input: &str) -> Result<BTreeMap<String, Vec<EvidenceEvent>>, String> {
    let mut events = BTreeMap::<String, Vec<EvidenceEvent>>::new();

    for (line_index, line) in input.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 28 {
            return Err(format!(
                "answer-key-events.tsv line {}: expected 28 columns, got {}",
                line_index + 1,
                fields.len()
            ));
        }
        let sequence = if fields[17] == "-" {
            None
        } else {
            Some(fields[17].parse::<u64>().map_err(|error| {
                format!(
                    "answer-key-events.tsv line {}: invalid sequence: {error}",
                    line_index + 1
                )
            })?)
        };
        let conflict_group = optional_text(fields[23]);
        let event = EvidenceEvent {
            event_id: fields[1].to_owned(),
            patient_binding: fields[2].to_owned(),
            source_principal: fields[3].to_owned(),
            resource_version: fields[4].to_owned(),
            resource_type: fields[24].to_owned(),
            resource_id: fields[25].to_owned(),
            assertion_key: optional_text(fields[26]),
            normalized_assertion_value: optional_text(fields[27]),
            payload_digest: fields[5].to_owned(),
            measurement_method: optional_text(fields[6]),
            measurement_unit: optional_text(fields[7]),
            terminology_system: optional_text(fields[8]),
            terminology_version: optional_text(fields[9]),
            terminology_code: optional_text(fields[10]),
            clinical_event_time_ms: fields[11].parse::<i64>().map_err(|error| {
                format!(
                    "answer-key-events.tsv line {}: invalid clinical event time: {error}",
                    line_index + 1
                )
            })?,
            specimen_collected_at_ms: parse_optional_i64(
                fields[12],
                "specimen collection time",
                line_index,
            )?,
            analysis_performed_at_ms: parse_optional_i64(fields[13], "analysis time", line_index)?,
            result_issued_at_ms: parse_optional_i64(fields[14], "result issue time", line_index)?,
            source_last_updated_at_ms: parse_optional_i64(
                fields[15],
                "source lastUpdated time",
                line_index,
            )?,
            ingested_at_ms: fields[16].parse::<i64>().map_err(|error| {
                format!(
                    "answer-key-events.tsv line {}: invalid ingestion time: {error}",
                    line_index + 1
                )
            })?,
            source_sequence: sequence,
            issuer_authenticated: parse_bool(fields[18], "issuer_authenticated")?,
            application_authorized: parse_bool(fields[19], "application_authorized")?,
            lineage_complete: parse_bool(fields[20], "lineage_complete")?,
            expired: parse_bool(fields[21], "expired")?,
            role: EventRole::parse(fields[22])?,
            conflict_group,
        };
        if event.assertion_key.is_some() != event.normalized_assertion_value.is_some()
            || event.event_id.is_empty()
            || event.patient_binding.is_empty()
            || event.source_principal.is_empty()
            || event.resource_version.is_empty()
            || event.resource_type.is_empty()
            || event.resource_id.is_empty()
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
        && event.resource_type == citation.resource_type
        && event.resource_id == citation.resource_id
        && event.assertion_key == citation.assertion_key
        && event.normalized_assertion_value == citation.normalized_assertion_value
        && event.payload_digest == citation.payload_digest
        && event.measurement_method == citation.measurement_method
        && event.measurement_unit == citation.measurement_unit
        && event.terminology_system == citation.terminology_system
        && event.terminology_version == citation.terminology_version
        && event.terminology_code == citation.terminology_code
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
    let (quarantine_precision, quarantine_recall) =
        set_precision_recall(&task.expected_quarantine, &submission.quarantined_event_ids);
    let (conflict_precision, conflict_recall) = set_precision_recall(
        &task.expected_conflicts,
        &submission.unresolved_conflict_groups,
    );
    let task_identity_correct = submission.task_id == task.task_id;
    let evidence_graph = evaluate_evidence_graph_v1(
        &expected_evidence_graph(task),
        &submission.evidence_graph,
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
    let exact_match = task_identity_correct
        && disposition_correct
        && timeline_exact
        && history_exact
        && quarantine_exact
        && conflicts_exact
        && unsafe_acceptances.is_empty()
        && provenance_errors.is_empty()
        && duplicates.is_empty();

    let hard_gates_pass = task_identity_correct
        && unsafe_acceptances.is_empty()
        && provenance_errors.is_empty()
        && duplicates.is_empty()
        && quarantine_recall == 1.0
        && conflict_recall == 1.0;

    TaskScore {
        task_id: task.task_id.clone(),
        task_identity_correct,
        disposition_correct,
        timeline_exact,
        history_exact,
        quarantine_exact,
        conflicts_exact,
        timeline_precision,
        timeline_recall,
        temporal_order_accuracy: pairwise_order_accuracy(expected_timeline, &actual_timeline),
        history_recall,
        quarantine_precision,
        quarantine_recall,
        conflict_precision,
        conflict_recall,
        unsafe_acceptances,
        provenance_errors,
        duplicate_acceptances: duplicates,
        evidence_graph,
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
        evidence_graph: expected_evidence_graph(task),
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
        evidence_graph: EvidenceGraphV1 {
            nodes: Vec::new(),
            relations: Vec::new(),
        },
    }
}


/// Resource-version baseline: groups only by the logical resource identity
/// (subject, source principal, resource type, resource ID), detects conflicting
/// payloads for the same version, deduplicates exact replays, and uses explicit
/// source sequence—not clocks or arrival time—to select a current revision.
/// It intentionally does not solve cross-source semantic assertion conflicts.
pub fn source_sequence_revision_baseline(input: &VisibleTaskInput) -> Submission {
    let mut groups: BTreeMap<(String, String, String, String), Vec<&VisibleEvidenceEvent>> =
        BTreeMap::new();
    let mut quarantined_event_ids = Vec::new();
    let mut identity_blocked = false;
    let mut authority_blocked = false;
    let mut stale = false;
    let mut insufficient = false;

    for event in &input.events {
        if event.patient_binding != input.target_patient_binding {
            identity_blocked = true;
            quarantined_event_ids.push(event.event_id.clone());
            continue;
        }
        if !event.application_authorized {
            authority_blocked = true;
            quarantined_event_ids.push(event.event_id.clone());
            continue;
        }
        if event.expired {
            stale = true;
            quarantined_event_ids.push(event.event_id.clone());
            continue;
        }
        if !event.issuer_authenticated || !event.lineage_complete {
            insufficient = true;
            quarantined_event_ids.push(event.event_id.clone());
            continue;
        }

        groups
            .entry((
                event.patient_binding.clone(),
                event.source_principal.clone(),
                event.resource_type.clone(),
                event.resource_id.clone(),
            ))
            .or_default()
            .push(event);
    }

    let mut timeline = Vec::<EventCitation>::new();
    let mut retained_history = Vec::<EventCitation>::new();
    let mut unresolved_conflict_groups = Vec::new();
    let mut unknown_order = false;

    for ((_, source, resource_type, resource_id), events) in groups {
        let mut by_version: BTreeMap<&str, BTreeMap<&str, Vec<&VisibleEvidenceEvent>>> =
            BTreeMap::new();
        for event in &events {
            by_version
                .entry(event.resource_version.as_str())
                .or_default()
                .entry(event.payload_digest.as_str())
                .or_default()
                .push(*event);
        }

        let conflicting_version = by_version
            .iter()
            .find(|(_, digests)| digests.len() > 1)
            .map(|(version, _)| *version);
        if let Some(version) = conflicting_version {
            unresolved_conflict_groups.push(format!(
                "resource:{source}:{resource_type}:{resource_id}:{version}"
            ));
            continue;
        }

        let mut unique_revisions = Vec::<&VisibleEvidenceEvent>::new();
        for (_, digests) in by_version {
            let mut replay_group = digests
                .into_values()
                .next()
                .expect("version group contains at least one digest");
            // Local arrival time is not source revision order or replay identity.
            // Choose a stable representative so clock/arrival perturbations cannot
            // change which identical source assertion becomes the canonical copy.
            replay_group.sort_by(|left, right| left.event_id.cmp(&right.event_id));
            if let Some(original) = replay_group.first() {
                unique_revisions.push(*original);
                quarantined_event_ids.extend(
                    replay_group
                        .iter()
                        .skip(1)
                        .map(|duplicate| duplicate.event_id.clone()),
                );
            }
        }

        let mut observed_sequences = BTreeSet::new();
        let sequence_order_is_ambiguous = unique_revisions.iter().any(|event| {
            event
                .source_sequence
                .is_none_or(|sequence| !observed_sequences.insert(sequence))
        });
        if sequence_order_is_ambiguous {
            unknown_order = true;
            retained_history.extend(unique_revisions.into_iter().map(EventCitation::from));
            continue;
        }

        unique_revisions.sort_by_key(|event| event.source_sequence);
        if let Some(current) = unique_revisions.pop() {
            timeline.push(EventCitation::from(current));
        }
        retained_history.extend(unique_revisions.into_iter().map(EventCitation::from));
    }

    // Detect disagreement across independent resources only where the visible
    // input explicitly says they address the same canonical proposition.
    let mut values_by_assertion: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for citation in &timeline {
        if let (Some(assertion_key), Some(value)) = (
            citation.assertion_key.as_deref(),
            citation.normalized_assertion_value.as_deref(),
        ) {
            values_by_assertion
                .entry(assertion_key)
                .or_default()
                .insert(value);
        }
    }
    let conflicting_assertions: BTreeSet<&str> = values_by_assertion
        .iter()
        .filter(|(_, values)| values.len() > 1)
        .map(|(assertion_key, _)| *assertion_key)
        .collect();
    if !conflicting_assertions.is_empty() {
        unresolved_conflict_groups.extend(
            conflicting_assertions
                .iter()
                .map(|assertion_key| format!("assertion:{assertion_key}")),
        );
        timeline.retain(|citation| {
            !citation
                .assertion_key
                .as_deref()
                .is_some_and(|key| conflicting_assertions.contains(key))
        });
    }

    let timeline_order = |left: &EventCitation, right: &EventCitation| {
        left.clinical_event_time_ms
            .cmp(&right.clinical_event_time_ms)
            .then_with(|| left.event_id.cmp(&right.event_id))
    };
    timeline.sort_by(timeline_order);
    retained_history.sort_by(timeline_order);
    quarantined_event_ids.sort();
    quarantined_event_ids.dedup();
    unresolved_conflict_groups.sort();

    let disposition = if !unresolved_conflict_groups.is_empty() {
        Disposition::Conflicted
    } else if timeline.is_empty() && retained_history.is_empty() && identity_blocked {
        Disposition::IdentityBlocked
    } else if timeline.is_empty() && retained_history.is_empty() && authority_blocked {
        Disposition::AuthorityBlocked
    } else if timeline.is_empty() && retained_history.is_empty() && stale {
        Disposition::Stale
    } else if timeline.is_empty() && retained_history.is_empty() && insufficient {
        Disposition::Insufficient
    } else if timeline.is_empty() && unknown_order {
        Disposition::UnknownOrder
    } else {
        Disposition::Supported
    };

    Submission {
        task_id: input.task_id.clone(),
        disposition,
        timeline,
        retained_history,
        quarantined_event_ids,
        unresolved_conflict_groups,
        evidence_graph: build_evidence_graph_v1(&input.events),
    }
}


/// Exact one-sided Clopper-Pearson upper confidence bound when zero failures
/// are observed in independent Bernoulli trials.
///
/// This function is deliberately undefined for a non-zero observed failure
/// count; use an exact beta-quantile implementation for that case. The caller
/// must define an independent trial unit. Tasks sharing one synthetic subject
/// are not independent patient-level trials.
pub fn zero_failure_upper_confidence_bound(
    independent_trials: usize,
    confidence_level: f64,
) -> Result<f64, String> {
    if independent_trials == 0 {
        return Err("independent_trials must be greater than zero".to_owned());
    }
    if !confidence_level.is_finite() || confidence_level <= 0.0 || confidence_level >= 1.0 {
        return Err("confidence_level must be finite and strictly between zero and one".to_owned());
    }
    Ok(1.0 - (1.0 - confidence_level).powf(1.0 / independent_trials as f64))
}

/// Minimum independent zero-failure trials required to put the one-sided
/// Clopper-Pearson upper bound at or below maximum_failure_rate.
pub fn minimum_zero_failure_trials(
    maximum_failure_rate: f64,
    confidence_level: f64,
) -> Result<usize, String> {
    if !maximum_failure_rate.is_finite()
        || maximum_failure_rate <= 0.0
        || maximum_failure_rate >= 1.0
    {
        return Err("maximum_failure_rate must be finite and strictly between zero and one".to_owned());
    }
    if !confidence_level.is_finite() || confidence_level <= 0.0 || confidence_level >= 1.0 {
        return Err("confidence_level must be finite and strictly between zero and one".to_owned());
    }
    let trials = ((1.0 - confidence_level).ln() / (1.0 - maximum_failure_rate).ln()).ceil();
    if !trials.is_finite() || trials > usize::MAX as f64 {
        return Err("required independent trial count is not representable".to_owned());
    }
    Ok(trials.max(1.0) as usize)
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
        quarantine_precision_mean: 0.0,
        quarantine_recall_mean: 0.0,
        conflict_precision_mean: 0.0,
        conflict_recall_mean: 0.0,
        graph_exact_matches: 0,
        graph_structural_invalid_count: 0,
        graph_node_precision_mean: 0.0,
        graph_node_recall_mean: 0.0,
        graph_edge_precision_mean: 0.0,
        graph_edge_recall_mean: 0.0,
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
        report.quarantine_precision_mean += score.quarantine_precision;
        report.quarantine_recall_mean += score.quarantine_recall;
        report.conflict_precision_mean += score.conflict_precision;
        report.conflict_recall_mean += score.conflict_recall;
        report.graph_exact_matches += usize::from(score.evidence_graph.exact_match);
        report.graph_structural_invalid_count +=
            usize::from(!score.evidence_graph.structurally_valid);
        report.graph_node_precision_mean += score.evidence_graph.node_precision;
        report.graph_node_recall_mean += score.evidence_graph.node_recall;
        report.graph_edge_precision_mean += score.evidence_graph.edge_precision;
        report.graph_edge_recall_mean += score.evidence_graph.edge_recall;
    }

    let count = report.task_count as f64;
    report.temporal_order_accuracy_mean /= count;
    report.quarantine_precision_mean /= count;
    report.quarantine_recall_mean /= count;
    report.conflict_precision_mean /= count;
    report.conflict_recall_mean /= count;
    report.graph_node_precision_mean /= count;
    report.graph_node_recall_mean /= count;
    report.graph_edge_precision_mean /= count;
    report.graph_edge_recall_mean /= count;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn zero_failure_confidence_bound_requires_independent_trials_and_valid_confidence() {
        let upper_21 = zero_failure_upper_confidence_bound(21, 0.95).unwrap();
        assert!(upper_21 > 0.13 && upper_21 < 0.14);

        let upper_299 = zero_failure_upper_confidence_bound(299, 0.95).unwrap();
        assert!(upper_299 < 0.01);

        assert_eq!(minimum_zero_failure_trials(0.01, 0.95).unwrap(), 299);
        assert!(zero_failure_upper_confidence_bound(0, 0.95).is_err());
        assert!(zero_failure_upper_confidence_bound(21, 1.0).is_err());
        assert!(minimum_zero_failure_trials(0.0, 0.95).is_err());
    }

    #[test]
    fn frozen_v1_corpus_loads_twenty_one_synthetic_tasks() {
        let tasks = load_fixture().expect("fixture must validate");
        assert_eq!(tasks.len(), 21);
        assert_eq!(
            tasks.iter().map(|task| task.events.len()).sum::<usize>(),
            34
        );
    }

    #[test]
    fn visible_input_is_separate_from_answer_key_and_negative_control_is_detected() {
        let visible = load_visible_fixture().expect("visible fixture must validate");
        let answer_key = load_fixture().expect("answer key fixture must validate");
        assert_eq!(visible.len(), 21);
        assert!(visible.iter().all(|task| !task.request.trim().is_empty()));
        assert_eq!(
            visible.iter().map(|task| task.events.len()).sum::<usize>(),
            34
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
    fn naive_baseline_generates_one_scored_output_for_every_visible_task() {
        let visible = load_visible_fixture().expect("visible fixture must validate");
        let tasks = load_fixture().expect("answer key fixture must validate");
        let truth_by_id: BTreeMap<&str, &Task> = tasks
            .iter()
            .map(|task| (task.task_id.as_str(), task))
            .collect();

        let mut submissions = Vec::with_capacity(visible.len());
        for input in &visible {
            assert!(!input.request.trim().is_empty());
            let truth = truth_by_id
                .get(input.task_id.as_str())
                .expect("every visible task has an evaluator answer");
            let submission = naive_ingestion_order_baseline(input);
            let score = evaluate(truth, &submission);
            assert_eq!(score.task_id, input.task_id);
            submissions.push(submission);
        }

        let report = evaluate_batch(&tasks, &submissions).expect("complete negative-control batch");
        assert_eq!(report.task_count, 21);
        assert!(report.exact_matches < report.task_count);
        assert!(report.unsafe_acceptance_count > 0);
    }

    #[test]
    fn semantic_assertion_identity_and_value_are_bound_into_citations() {
        let tasks = load_fixture().expect("answer key must validate");
        let task = tasks.iter().find(|task| task.task_id == "T17").unwrap();
        let event = task
            .events
            .iter()
            .find(|event| event.event_id == "E26")
            .unwrap();
        assert_eq!(
            event.assertion_key.as_deref(),
            Some("subject:PAT-A|concept:synthetic-condition-status|effective:21000")
        );
        assert_eq!(event.normalized_assertion_value.as_deref(), Some("present"));
        let mut citation = EventCitation::from(event);
        citation.normalized_assertion_value = Some("absent".to_owned());
        assert!(!citation_matches(event, &citation));
    }

    #[test]
    fn source_sequence_baseline_respects_resource_revision_and_replay_semantics() {
        let visible = load_visible_fixture().expect("visible fixture must validate");
        let answer_key = load_fixture().expect("answer key must validate");
        let truth_by_id: BTreeMap<&str, &Task> = answer_key
            .iter()
            .map(|task| (task.task_id.as_str(), task))
            .collect();
        let visible_by_id: BTreeMap<&str, &VisibleTaskInput> = visible
            .iter()
            .map(|task| (task.task_id.as_str(), task))
            .collect();

        for task_id in ["T03", "T04", "T05", "T06", "T16", "T17", "T20", "T21"] {
            let submission = source_sequence_revision_baseline(visible_by_id[task_id]);
            let score = evaluate(truth_by_id[task_id], &submission);
            assert!(score.exact_match, "{task_id} did not match: {score:?}");
            assert!(score.hard_gates_pass, "{task_id} failed a hard gate");
        }

        let corrected = source_sequence_revision_baseline(visible_by_id["T03"]);
        assert_eq!(corrected.timeline[0].event_id, "E06");
        assert_eq!(corrected.retained_history[0].event_id, "E05");

        let replay = source_sequence_revision_baseline(visible_by_id["T06"]);
        assert_eq!(replay.timeline[0].event_id, "E12");
        assert_eq!(replay.quarantined_event_ids, vec!["E13".to_owned()]);

        let unknown = source_sequence_revision_baseline(visible_by_id["T16"]);
        assert_eq!(unknown.disposition, Disposition::UnknownOrder);
        assert!(unknown.timeline.is_empty());
        assert_eq!(unknown.retained_history[0].event_id, "E25");

        let tied = source_sequence_revision_baseline(visible_by_id["T21"]);
        assert_eq!(tied.disposition, Disposition::UnknownOrder);
        assert!(tied.timeline.is_empty());
        assert_eq!(
            tied.retained_history
                .iter()
                .map(|event| event.event_id.as_str())
                .collect::<Vec<_>>(),
            vec!["E33", "E34"]
        );
    }

    #[test]
    fn source_sequence_baseline_detects_cross_source_semantic_conflict() {
        let visible = load_visible_fixture().expect("visible fixture must validate");
        let answer_key = load_fixture().expect("answer key must validate");
        let truth = answer_key
            .iter()
            .find(|task| task.task_id == "T17")
            .unwrap();
        let input = visible.iter().find(|task| task.task_id == "T17").unwrap();
        let submission = source_sequence_revision_baseline(input);
        let score = evaluate(truth, &submission);
        assert!(score.exact_match, "{score:?}");
        assert!(score.hard_gates_pass);
        assert!(submission.timeline.is_empty());
        assert_eq!(
            submission.unresolved_conflict_groups,
            vec![
                "assertion:subject:PAT-A|concept:synthetic-condition-status|effective:21000"
                    .to_owned()
            ]
        );
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
            assert!(
                result.evidence_graph.exact_match,
                "{} did not exactly match its reference evidence graph: {:?}",
                task.task_id,
                result.evidence_graph
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
    fn logical_resource_identity_is_visible_and_part_of_exact_provenance() {
        let answer_key = load_fixture().expect("answer key must validate");
        let visible = load_visible_fixture().expect("visible corpus must validate");

        let truth = answer_key
            .iter()
            .find(|task| task.task_id == "T03")
            .unwrap();
        let prior = truth
            .events
            .iter()
            .find(|event| event.event_id == "E05")
            .unwrap();
        let current = truth
            .events
            .iter()
            .find(|event| event.event_id == "E06")
            .unwrap();
        assert_eq!(prior.resource_type, "Observation");
        assert_eq!(prior.resource_id, "lab-panel-03");
        assert_eq!(current.resource_id, "lab-panel-03");
        assert_ne!(prior.resource_version, current.resource_version);

        let visible_task = visible.iter().find(|task| task.task_id == "T03").unwrap();
        let visible_prior = visible_task
            .events
            .iter()
            .find(|event| event.event_id == "E05")
            .unwrap();
        let visible_current = visible_task
            .events
            .iter()
            .find(|event| event.event_id == "E06")
            .unwrap();
        assert_eq!(visible_prior.resource_id, visible_current.resource_id);

        let mut submission = reference_submission(truth);
        submission.timeline[0].resource_id = "wrong-resource".to_owned();
        let score = evaluate(truth, &submission);
        assert_eq!(score.provenance_errors, vec!["E06".to_owned()]);
        assert!(!score.hard_gates_pass);
    }

    #[test]
    fn source_version_digest_substitution_is_a_provenance_error() {
        let tasks = load_fixture().expect("fixture must validate");
        let task = tasks.iter().find(|task| task.task_id == "T03").unwrap();
        let mut submission = reference_submission(task);
        submission.timeline[0].payload_digest = "0".repeat(64);
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
    fn measurement_method_and_terminology_versions_remain_explicit() {
        let tasks = load_fixture().expect("fixture must validate");
        let visible = load_visible_fixture().expect("visible fixture must validate");

        let method_task = tasks.iter().find(|task| task.task_id == "T18").unwrap();
        let method_a = method_task
            .events
            .iter()
            .find(|event| event.event_id == "E28")
            .unwrap();
        let method_b = method_task
            .events
            .iter()
            .find(|event| event.event_id == "E29")
            .unwrap();
        assert_eq!(method_a.measurement_method.as_deref(), Some("assay-A"));
        assert_eq!(method_b.measurement_method.as_deref(), Some("assay-B"));
        assert_eq!(method_a.measurement_unit.as_deref(), Some("mmol/L"));
        assert_eq!(method_b.measurement_unit.as_deref(), Some("mmol/L"));

        let terminology_task = tasks.iter().find(|task| task.task_id == "T19").unwrap();
        let terminology_a = terminology_task
            .events
            .iter()
            .find(|event| event.event_id == "E30")
            .unwrap();
        let terminology_b = terminology_task
            .events
            .iter()
            .find(|event| event.event_id == "E31")
            .unwrap();
        assert_eq!(
            terminology_a.terminology_system.as_deref(),
            Some("urn:mycelix:synthetic-terminology")
        );
        assert_eq!(
            terminology_a.terminology_version.as_deref(),
            Some("2026-01")
        );
        assert_eq!(
            terminology_b.terminology_version.as_deref(),
            Some("2026-06")
        );
        assert_eq!(
            terminology_a.terminology_code,
            terminology_b.terminology_code
        );

        let visible_method_task = visible.iter().find(|task| task.task_id == "T18").unwrap();
        let visible_method_a = visible_method_task
            .events
            .iter()
            .find(|event| event.event_id == "E28")
            .unwrap();
        assert_eq!(
            visible_method_a.measurement_method.as_deref(),
            Some("assay-A")
        );
        assert!(!visible_method_task.request.trim().is_empty());

        let citation = EventCitation::from(method_a);
        assert_eq!(citation.measurement_method, method_a.measurement_method);
        assert_eq!(citation.measurement_unit, method_a.measurement_unit);
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
    fn submission_must_be_bound_to_the_requested_task() {
        let tasks = load_fixture().expect("fixture must validate");
        let task = tasks.iter().find(|task| task.task_id == "T01").unwrap();
        let mut submission = reference_submission(task);
        submission.task_id = "T99".to_owned();
        let result = evaluate(task, &submission);
        assert!(!result.task_identity_correct);
        assert!(!result.exact_match);
        assert!(!result.hard_gates_pass);
    }

    #[test]
    fn false_positive_quarantine_and_conflict_labels_reduce_precision() {
        let tasks = load_fixture().expect("fixture must validate");
        let quarantine_task = tasks.iter().find(|task| task.task_id == "T07").unwrap();
        let mut quarantine_submission = reference_submission(quarantine_task);
        quarantine_submission
            .quarantined_event_ids
            .push("EXTRA".to_owned());
        let quarantine_score = evaluate(quarantine_task, &quarantine_submission);
        assert_eq!(quarantine_score.quarantine_precision, 0.5);
        assert_eq!(quarantine_score.quarantine_recall, 1.0);
        assert!(!quarantine_score.exact_match);

        let conflict_task = tasks.iter().find(|task| task.task_id == "T05").unwrap();
        let mut conflict_submission = reference_submission(conflict_task);
        conflict_submission
            .unresolved_conflict_groups
            .push("EXTRA".to_owned());
        let conflict_score = evaluate(conflict_task, &conflict_submission);
        assert_eq!(conflict_score.conflict_precision, 0.5);
        assert_eq!(conflict_score.conflict_recall, 1.0);
        assert!(!conflict_score.exact_match);
    }

    #[test]
    fn batch_requires_one_output_per_task() {
        let tasks = load_fixture().expect("fixture must validate");
        let partial: Vec<Submission> = tasks.iter().take(20).map(reference_submission).collect();
        assert!(evaluate_batch(&tasks, &partial).is_err());
        let complete: Vec<Submission> = tasks.iter().map(reference_submission).collect();
        let report = evaluate_batch(&tasks, &complete).expect("complete batch");
        assert_eq!(report.task_count, 21);
        assert_eq!(report.exact_matches, 21);
        assert_eq!(report.unsafe_acceptance_count, 0);
        assert_eq!(report.provenance_error_count, 0);
        assert_eq!(report.graph_exact_matches, 21);
        assert_eq!(report.graph_structural_invalid_count, 0);
        assert_eq!(report.graph_edge_precision_mean, 1.0);
        assert_eq!(report.graph_edge_recall_mean, 1.0);
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

    #[test]
    fn evidence_graph_score_is_exact_for_reference_outputs() {
        let tasks = load_fixture().expect("fixture must validate");
        for task in &tasks {
            let expected = expected_evidence_graph(task);
            let score = evaluate_evidence_graph_v1(
                &expected,
                &reference_submission(task).evidence_graph,
            );
            assert!(score.exact_match, "{} graph mismatch: {score:?}", task.task_id);
            assert!(score.structurally_valid);
            assert_eq!(score.node_precision, 1.0);
            assert_eq!(score.node_recall, 1.0);
            assert_eq!(score.edge_precision, 1.0);
            assert_eq!(score.edge_recall, 1.0);
        }
    }

    #[test]
    fn graph_edge_omission_and_substitution_are_scored_separately_from_timeline() {
        let tasks = load_fixture().expect("fixture must validate");
        let task = tasks.iter().find(|task| task.task_id == "T03").unwrap();
        let mut submission = reference_submission(task);
        let timeline_exact_before = evaluate(task, &submission).timeline_exact;
        assert!(submission.evidence_graph.relations.len() >= 2);

        submission.evidence_graph.relations.pop();
        let missing_edge = evaluate(task, &submission);
        assert!(missing_edge.evidence_graph.edge_recall < 1.0);
        assert_eq!(missing_edge.timeline_exact, timeline_exact_before);
        assert!(missing_edge.hard_gates_pass);

        let mut unsupported = reference_submission(task);
        unsupported.evidence_graph.relations.push(EvidenceRelationV1 {
            from_event_id: "E05".to_owned(),
            to_event_id: "E06".to_owned(),
            kind: EvidenceRelationKindV1::ContradictoryClinicalAssertion,
        });
        let extra_edge = evaluate(task, &unsupported);
        assert!(extra_edge.evidence_graph.edge_precision < 1.0);
        assert_eq!(extra_edge.evidence_graph.edge_recall, 1.0);
        assert!(extra_edge.evidence_graph.structurally_valid);
        assert_eq!(extra_edge.timeline_exact, timeline_exact_before);
    }

    #[test]
    fn graph_scorer_rejects_dangling_and_unknown_structure() {
        let tasks = load_fixture().expect("fixture must validate");
        let task = tasks.iter().find(|task| task.task_id == "T03").unwrap();
        let expected = expected_evidence_graph(task);
        let mut actual = expected.clone();
        actual.relations.push(EvidenceRelationV1 {
            from_event_id: "UNKNOWN-EVENT".to_owned(),
            to_event_id: "E06".to_owned(),
            kind: EvidenceRelationKindV1::VersionPayloadCollision,
        });
        actual.nodes.push("UNSEEN-NODE".to_owned());

        let score = evaluate_evidence_graph_v1(&expected, &actual);
        assert!(!score.structurally_valid);
        assert_eq!(score.dangling_relation_count, 1);
        assert_eq!(score.unknown_node_count, 1);
        assert!(!score.exact_match);
    }

    #[test]
    fn graph_scorer_rejects_duplicate_nodes_and_relations() {
        let tasks = load_fixture().expect("fixture must validate");
        let task = tasks.iter().find(|task| task.task_id == "T03").unwrap();
        let expected = expected_evidence_graph(task);
        let mut actual = expected.clone();
        actual.nodes.push(actual.nodes[0].clone());
        actual.relations.push(actual.relations[0].clone());

        let score = evaluate_evidence_graph_v1(&expected, &actual);
        assert!(!score.structurally_valid);
        assert_eq!(score.duplicate_node_count, 1);
        assert_eq!(score.duplicate_relation_count, 1);
        assert!(!score.exact_match);
    }

    #[test]
    fn source_revision_baseline_emits_exact_graph_while_naive_control_does_not() {
        let visible = load_visible_fixture().expect("visible fixtures must validate");
        let tasks = load_fixture().expect("answer key must validate");
        let visible_task = visible.iter().find(|task| task.task_id == "T17").unwrap();
        let task = tasks.iter().find(|task| task.task_id == "T17").unwrap();

        let revision = evaluate(task, &source_sequence_revision_baseline(visible_task));
        let naive = evaluate(task, &naive_ingestion_order_baseline(visible_task));

        assert!(revision.evidence_graph.exact_match);
        assert!(!naive.evidence_graph.exact_match);
        assert_eq!(revision.timeline_exact, naive.timeline_exact);
    }

}
