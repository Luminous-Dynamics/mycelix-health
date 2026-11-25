#![forbid(unsafe_code)]

//! Answer-key-free contract for candidate agents in the temporal evidence benchmark.
//!
//! This crate deliberately does not depend on the evaluator crate and never
//! includes answer-key fixtures. A candidate process may depend on this crate
//! without compiling evaluator truth, hidden roles, or expected dispositions.

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
    pub fn parse(value: &str) -> Result<Self, String> {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventCitation {
    pub event_id: String,
    pub source_principal: String,
    pub resource_version: String,
    pub resource_type: String,
    pub resource_id: String,
    pub assertion_key: Option<String>,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleEvidenceEvent {
    pub event_id: String,
    pub patient_binding: String,
    pub source_principal: String,
    pub resource_version: String,
    pub resource_type: String,
    pub resource_id: String,
    pub assertion_key: Option<String>,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleTaskInput {
    pub task_id: String,
    pub target_patient_binding: String,
    pub request: String,
    pub events: Vec<VisibleEvidenceEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submission {
    pub task_id: String,
    pub disposition: Disposition,
    /// Current accepted evidence in clinical event-time order.
    pub timeline: Vec<EventCitation>,
    /// Valid but non-current evidence retained as history.
    pub retained_history: Vec<EventCitation>,
    /// Explicitly rejected evidence IDs.
    pub quarantined_event_ids: Vec<String>,
    /// Unresolved source-resource or semantic assertion conflict groups.
    pub unresolved_conflict_groups: Vec<String>,
}

impl From<&VisibleEvidenceEvent> for EventCitation {
    fn from(event: &VisibleEvidenceEvent) -> Self {
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

fn optional_text(value: &str) -> Option<String> {
    if value == "-" || value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn parse_bool(value: &str, field: &str, line: usize) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!(
            "visible events.tsv line {}: {field} must be true or false",
            line + 1
        )),
    }
}

fn parse_optional_i64(value: &str, field: &str, line: usize) -> Result<Option<i64>, String> {
    if value == "-" {
        Ok(None)
    } else {
        value.parse::<i64>().map(Some).map_err(|error| {
            format!(
                "visible events.tsv line {}: invalid {field}: {error}",
                line + 1
            )
        })
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_task_manifest(input: &str) -> Result<BTreeMap<String, (String, String)>, String> {
    let mut tasks = BTreeMap::new();
    for (line, row) in input.lines().enumerate() {
        if row.is_empty() || row.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = row.split('\t').collect();
        if fields.len() != 3
            || fields[0].is_empty()
            || fields[1].is_empty()
            || fields[2].trim().is_empty()
        {
            return Err(format!(
                "task-inputs.tsv line {}: expected task id, subject binding, and request",
                line + 1
            ));
        }
        if tasks
            .insert(
                fields[0].to_owned(),
                (fields[1].to_owned(), fields[2].to_owned()),
            )
            .is_some()
        {
            return Err(format!(
                "task-inputs.tsv line {}: duplicate task id {}",
                line + 1,
                fields[0]
            ));
        }
    }
    if tasks.is_empty() {
        return Err("task-inputs.tsv contains no tasks".to_owned());
    }
    Ok(tasks)
}

fn parse_visible_events(
    input: &str,
) -> Result<BTreeMap<String, Vec<VisibleEvidenceEvent>>, String> {
    let mut events_by_task = BTreeMap::<String, Vec<VisibleEvidenceEvent>>::new();
    let mut seen = BTreeSet::<(String, String)>::new();

    for (line, row) in input.lines().enumerate() {
        if row.is_empty() || row.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = row.split('\t').collect();
        if fields.len() != 26 {
            return Err(format!(
                "visible events.tsv line {}: expected 26 columns, got {}",
                line + 1,
                fields.len()
            ));
        }
        let task_id = fields[0].to_owned();
        let event_id = fields[1].to_owned();
        if task_id.is_empty()
            || event_id.is_empty()
            || !seen.insert((task_id.clone(), event_id.clone()))
        {
            return Err(format!(
                "visible events.tsv line {}: empty task/event ID or duplicate event",
                line + 1
            ));
        }

        let source_sequence = if fields[17] == "-" {
            None
        } else {
            Some(fields[17].parse::<u64>().map_err(|error| {
                format!(
                    "visible events.tsv line {}: invalid source sequence: {error}",
                    line + 1
                )
            })?)
        };

        let event = VisibleEvidenceEvent {
            event_id,
            patient_binding: fields[2].to_owned(),
            source_principal: fields[3].to_owned(),
            resource_version: fields[4].to_owned(),
            resource_type: fields[22].to_owned(),
            resource_id: fields[23].to_owned(),
            assertion_key: optional_text(fields[24]),
            normalized_assertion_value: optional_text(fields[25]),
            payload_digest: fields[5].to_owned(),
            measurement_method: optional_text(fields[6]),
            measurement_unit: optional_text(fields[7]),
            terminology_system: optional_text(fields[8]),
            terminology_version: optional_text(fields[9]),
            terminology_code: optional_text(fields[10]),
            clinical_event_time_ms: fields[11].parse::<i64>().map_err(|error| {
                format!(
                    "visible events.tsv line {}: invalid clinical event time: {error}",
                    line + 1
                )
            })?,
            specimen_collected_at_ms: parse_optional_i64(
                fields[12],
                "specimen collection time",
                line,
            )?,
            analysis_performed_at_ms: parse_optional_i64(fields[13], "analysis time", line)?,
            result_issued_at_ms: parse_optional_i64(fields[14], "result issue time", line)?,
            source_last_updated_at_ms: parse_optional_i64(
                fields[15],
                "source lastUpdated time",
                line,
            )?,
            ingested_at_ms: fields[16].parse::<i64>().map_err(|error| {
                format!(
                    "visible events.tsv line {}: invalid ingestion time: {error}",
                    line + 1
                )
            })?,
            source_sequence,
            issuer_authenticated: parse_bool(fields[18], "issuer_authenticated", line)?,
            application_authorized: parse_bool(fields[19], "application_authorized", line)?,
            lineage_complete: parse_bool(fields[20], "lineage_complete", line)?,
            expired: parse_bool(fields[21], "expired", line)?,
        };

        if event.patient_binding.is_empty()
            || event.source_principal.is_empty()
            || event.resource_version.is_empty()
            || event.resource_type.is_empty()
            || event.resource_id.is_empty()
            || !valid_digest(&event.payload_digest)
            || event.assertion_key.is_some() != event.normalized_assertion_value.is_some()
        {
            return Err(format!(
                "visible events.tsv line {}: invalid identity, digest, or assertion pair",
                line + 1
            ));
        }
        events_by_task.entry(task_id).or_default().push(event);
    }
    Ok(events_by_task)
}

/// Load the frozen answer-key-free candidate corpus.
///
/// This function compiles only the visible task manifest and evidence envelope.
/// It does not reference evaluator roles, expected dispositions, expected
/// timelines, or any answer-key file.
pub fn load_visible_fixture() -> Result<Vec<VisibleTaskInput>, String> {
    let manifest = parse_task_manifest(include_str!(
        "../../temporal-evidence-integrity-bench/fixtures/v1-task-inputs.tsv"
    ))?;
    let mut events_by_task = parse_visible_events(include_str!(
        "../../temporal-evidence-integrity-bench/fixtures/v1-events.tsv"
    ))?;

    let mut tasks = Vec::with_capacity(manifest.len());
    for (task_id, (target_patient_binding, request)) in manifest {
        let events = events_by_task.remove(&task_id).unwrap_or_default();
        tasks.push(VisibleTaskInput {
            task_id,
            target_patient_binding,
            request,
            events,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_contract_loads_only_answer_free_visible_fixtures() {
        let tasks = load_visible_fixture().expect("visible fixture must validate");
        assert_eq!(tasks.len(), 20);
        assert!(tasks.iter().all(|task| !task.request.trim().is_empty()));
        assert_eq!(
            tasks.iter().map(|task| task.events.len()).sum::<usize>(),
            32
        );
    }

    #[test]
    fn visible_contract_has_no_answer_key_fields() {
        let task = load_visible_fixture().unwrap().remove(0);
        let event = &task.events[0];
        assert!(!event.event_id.is_empty());
        let _submission = Submission {
            task_id: task.task_id,
            disposition: Disposition::Insufficient,
            timeline: vec![EventCitation::from(event)],
            retained_history: Vec::new(),
            quarantined_event_ids: Vec::new(),
            unresolved_conflict_groups: Vec::new(),
        };
    }

    #[test]
    fn malformed_digest_is_rejected() {
        let source = include_str!(
            "../../temporal-evidence-integrity-bench/fixtures/v1-events.tsv"
        )
        .replacen(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "invalid",
            1,
        );
        assert!(parse_visible_events(&source).is_err());
    }

    #[test]
    fn assertion_identity_and_value_must_appear_as_a_pair() {
        let source = include_str!(
            "../../temporal-evidence-integrity-bench/fixtures/v1-events.tsv"
        )
        .replace(
            "subject:PAT-A|concept:synthetic-condition-status|effective:21000\tpresent",
            "subject:PAT-A|concept:synthetic-condition-status|effective:21000\t-",
        );
        assert!(parse_visible_events(&source).is_err());
    }
}
