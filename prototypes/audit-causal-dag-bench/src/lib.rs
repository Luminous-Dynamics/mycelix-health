//! Reference verifier for per-agent clinical audit streams and cross-agent causal DAGs.
//!
//! Trust boundary: ActionHash values, claimed authors, and checkpoints must be
//! supplied by an adapter that independently verifies Holochain action
//! signatures and an external checkpoint commitment. This crate checks graph
//! structure and binding consistency; it does not perform cryptographic
//! signature verification, DHT publication, or checkpoint-signature validation.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceVersionRef {
    pub resource_type: String,
    pub logical_id: String,
    pub version_id: Option<String>,
    pub content_digest: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEvent {
    /// Holochain ActionHash, already decoded and verified by the adapter.
    pub action_hash: String,
    /// Stable identifier for the source-chain stream.
    pub stream_id: String,
    /// Author asserted by the event payload; compared with the verified author.
    pub claimed_author: String,
    /// Per-agent source-chain sequence, not a patient-global sequence.
    pub sequence: u64,
    pub previous_action_hash: Option<String>,
    pub tenant_id: String,
    pub patient_id: String,
    pub resource: Option<ResourceVersionRef>,
    pub event_kind: String,
    pub request_id: String,
    pub authorization_receipt_id: Option<String>,
    pub policy_version: Option<String>,
    pub purpose: String,
    pub recipient: String,
    pub action: String,
    pub outcome: String,
    /// Clinical/event time. It is metadata, never a source of causal ordering.
    pub event_time_ms: Option<i64>,
    /// Local observation time. It is metadata, never a source of causal ordering.
    pub observed_at_ms: Option<i64>,
    /// Explicit cross-event causal dependencies.
    pub causal_parents: Vec<String>,
}

/// Must be constructed by an adapter after verifying an independent checkpoint
/// or signed receipt. The candidate event set must not nominate its own trust
/// anchor. This prototype validates consistency with the supplied commitment;
/// it does not validate the checkpoint's cryptographic signature.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedStreamCheckpoint {
    pub stream_id: String,
    pub authenticated_author: String,
    pub head_sequence: u64,
    pub head_action_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Finding {
    EmptyEventField { action_hash: String, field: &'static str },
    InvalidResourceReference { action_hash: String },
    InvalidCheckpoint { stream_id: String },
    DuplicateCheckpoint { stream_id: String },
    MissingCheckpoint { stream_id: String },
    AuthorMismatch { action_hash: String },
    UncheckpointedSuffix { action_hash: String },
    DuplicateStreamSequence { stream_id: String, sequence: u64 },
    MissingStreamAction { stream_id: String, sequence: u64 },
    InvalidStreamGenesis { action_hash: String },
    MissingPreviousAction { action_hash: String },
    PreviousActionMismatch { action_hash: String },
    CheckpointHeadMismatch { stream_id: String },
    ConflictingDuplicateAction { action_hash: String },
    DuplicateCausalParent { action_hash: String },
    EmptyCausalParent { action_hash: String },
    SelfCausalParent { action_hash: String },
    MissingCausalParent { action_hash: String, parent_hash: String },
    CrossPatientParent { action_hash: String, parent_hash: String },
    CrossTenantParent { action_hash: String, parent_hash: String },
    CausalCycle,
}

impl Finding {
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyEventField { .. } => "empty_event_field",
            Self::InvalidResourceReference { .. } => "invalid_resource_reference",
            Self::InvalidCheckpoint { .. } => "invalid_checkpoint",
            Self::DuplicateCheckpoint { .. } => "duplicate_checkpoint",
            Self::MissingCheckpoint { .. } => "missing_checkpoint",
            Self::AuthorMismatch { .. } => "author_mismatch",
            Self::UncheckpointedSuffix { .. } => "uncheckpointed_suffix",
            Self::DuplicateStreamSequence { .. } => "duplicate_stream_sequence",
            Self::MissingStreamAction { .. } => "missing_stream_action",
            Self::InvalidStreamGenesis { .. } => "invalid_stream_genesis",
            Self::MissingPreviousAction { .. } => "missing_previous_action",
            Self::PreviousActionMismatch { .. } => "previous_action_mismatch",
            Self::CheckpointHeadMismatch { .. } => "checkpoint_head_mismatch",
            Self::ConflictingDuplicateAction { .. } => "conflicting_duplicate_action",
            Self::DuplicateCausalParent { .. } => "duplicate_causal_parent",
            Self::EmptyCausalParent { .. } => "empty_causal_parent",
            Self::SelfCausalParent { .. } => "self_parent",
            Self::MissingCausalParent { .. } => "missing_causal_parent",
            Self::CrossPatientParent { .. } => "cross_patient_parent",
            Self::CrossTenantParent { .. } => "cross_tenant_parent",
            Self::CausalCycle => "causal_cycle",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewStatus {
    Verified,
    Incomplete,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReport {
    pub status: ViewStatus,
    pub findings: Vec<Finding>,
    pub event_count: usize,
    pub stream_count: usize,
    pub stream_edge_count: usize,
    pub causal_edge_count: usize,
    pub incomparable_pairs: usize,
    pub duplicate_observations: usize,
    /// Deterministic topological display order, populated only for a complete,
    /// verified view. It is a presentation linearization, not a causal total order.
    pub deterministic_display_order: Vec<String>,
}

fn reject(
    findings: &mut BTreeSet<Finding>,
    rejected: &mut bool,
    finding: Finding,
) {
    *rejected = true;
    findings.insert(finding);
}

fn defer(
    findings: &mut BTreeSet<Finding>,
    incomplete: &mut bool,
    finding: Finding,
) {
    *incomplete = true;
    findings.insert(finding);
}

fn add_edge(
    adjacency: &mut BTreeMap<String, BTreeSet<String>>,
    indegree: &mut BTreeMap<String, usize>,
    from: &str,
    to: &str,
) -> bool {
    let inserted = adjacency
        .get_mut(from)
        .expect("every graph node is initialized")
        .insert(to.to_string());
    if inserted {
        *indegree
            .get_mut(to)
            .expect("every graph node has an indegree") += 1;
    }
    inserted
}

fn reachable_from(
    start: &str,
    adjacency: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut pending = vec![start.to_string()];
    while let Some(node) = pending.pop() {
        if let Some(children) = adjacency.get(&node) {
            for child in children {
                if seen.insert(child.clone()) {
                    pending.push(child.clone());
                }
            }
        }
    }
    seen.remove(start);
    seen
}

fn has_required_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_event_shape(event: &AuditEvent, findings: &mut BTreeSet<Finding>, rejected: &mut bool) -> bool {
    let required = [
        ("action_hash", event.action_hash.as_str()),
        ("stream_id", event.stream_id.as_str()),
        ("claimed_author", event.claimed_author.as_str()),
        ("tenant_id", event.tenant_id.as_str()),
        ("patient_id", event.patient_id.as_str()),
        ("event_kind", event.event_kind.as_str()),
        ("request_id", event.request_id.as_str()),
        ("purpose", event.purpose.as_str()),
        ("recipient", event.recipient.as_str()),
        ("action", event.action.as_str()),
        ("outcome", event.outcome.as_str()),
    ];
    let mut valid = true;
    for (field, value) in required {
        if !has_required_text(value) {
            reject(
                findings,
                rejected,
                Finding::EmptyEventField {
                    action_hash: event.action_hash.clone(),
                    field,
                },
            );
            valid = false;
        }
    }
    if let Some(resource) = &event.resource {
        if !has_required_text(&resource.resource_type)
            || !has_required_text(&resource.logical_id)
            || resource
                .version_id
                .as_ref()
                .is_some_and(|value| !has_required_text(value))
            || resource
                .content_digest
                .as_ref()
                .is_some_and(|value| !has_required_text(value))
        {
            reject(
                findings,
                rejected,
                Finding::InvalidResourceReference {
                    action_hash: event.action_hash.clone(),
                },
            );
            valid = false;
        }
    }
    valid
}

/// Verify per-agent source-chain continuity, independent checkpoint binding,
/// and explicit cross-agent causal edges.
///
/// Important semantics:
/// - Source-chain sequence orders actions only within one agent stream.
/// - Cross-agent order exists only when an explicit causal parent is present.
/// - Missing peer data is Incomplete, not Verified and not automatically Rejected.
/// - Timestamps never add causal edges.
/// - A deterministic display order is returned only for complete verified views.
pub fn verify_audit_view(
    input_events: &[AuditEvent],
    input_checkpoints: &[TrustedStreamCheckpoint],
) -> VerificationReport {
    let mut findings = BTreeSet::new();
    let mut rejected = false;
    let mut incomplete = false;
    let mut duplicate_observations = 0usize;

    let mut checkpoints = BTreeMap::<String, TrustedStreamCheckpoint>::new();
    let mut conflicting_checkpoints = BTreeSet::<String>::new();
    for checkpoint in input_checkpoints {
        if !has_required_text(&checkpoint.stream_id)
            || !has_required_text(&checkpoint.authenticated_author)
            || !has_required_text(&checkpoint.head_action_hash)
        {
            reject(
                &mut findings,
                &mut rejected,
                Finding::InvalidCheckpoint {
                    stream_id: checkpoint.stream_id.clone(),
                },
            );
            continue;
        }
        match checkpoints.get(&checkpoint.stream_id) {
            None => {
                checkpoints.insert(checkpoint.stream_id.clone(), checkpoint.clone());
            }
            Some(existing) if existing == checkpoint => {
                // Identical checkpoint observations from different peers coalesce.
            }
            Some(_) => {
                conflicting_checkpoints.insert(checkpoint.stream_id.clone());
            }
        }
    }
    for stream_id in conflicting_checkpoints {
        checkpoints.remove(&stream_id);
        reject(
            &mut findings,
            &mut rejected,
            Finding::DuplicateCheckpoint { stream_id },
        );
    }

    // Normalize parent order before deduplicating repeated DHT observations.
    let mut events_by_hash = BTreeMap::<String, AuditEvent>::new();
    let mut conflicting_hashes = BTreeSet::<String>::new();
    for event in input_events {
        if !valid_event_shape(event, &mut findings, &mut rejected) {
            continue;
        }
        let mut canonical = event.clone();
        canonical.causal_parents.sort();
        match events_by_hash.get(&canonical.action_hash) {
            None => {
                events_by_hash.insert(canonical.action_hash.clone(), canonical);
            }
            Some(existing) if existing == &canonical => {
                duplicate_observations += 1;
            }
            Some(_) => {
                conflicting_hashes.insert(canonical.action_hash.clone());
            }
        }
    }
    for action_hash in conflicting_hashes {
        events_by_hash.remove(&action_hash);
        reject(
            &mut findings,
            &mut rejected,
            Finding::ConflictingDuplicateAction { action_hash },
        );
    }

    let mut grouped = BTreeMap::<String, Vec<AuditEvent>>::new();
    for event in events_by_hash.values() {
        match checkpoints.get(&event.stream_id) {
            None => defer(
                &mut findings,
                &mut incomplete,
                Finding::MissingCheckpoint {
                    stream_id: event.stream_id.clone(),
                },
            ),
            Some(checkpoint) => {
                if event.claimed_author != checkpoint.authenticated_author {
                    reject(
                        &mut findings,
                        &mut rejected,
                        Finding::AuthorMismatch {
                            action_hash: event.action_hash.clone(),
                        },
                    );
                }
                if event.sequence > checkpoint.head_sequence {
                    reject(
                        &mut findings,
                        &mut rejected,
                        Finding::UncheckpointedSuffix {
                            action_hash: event.action_hash.clone(),
                        },
                    );
                }
            }
        }
        grouped
            .entry(event.stream_id.clone())
            .or_default()
            .push(event.clone());
    }

    for checkpoint in checkpoints.values() {
        if !grouped.contains_key(&checkpoint.stream_id) {
            defer(
                &mut findings,
                &mut incomplete,
                Finding::MissingStreamAction {
                    stream_id: checkpoint.stream_id.clone(),
                    sequence: 0,
                },
            );
        }
    }

    let mut adjacency = BTreeMap::<String, BTreeSet<String>>::new();
    let mut indegree = BTreeMap::<String, usize>::new();
    for action_hash in events_by_hash.keys() {
        adjacency.insert(action_hash.clone(), BTreeSet::new());
        indegree.insert(action_hash.clone(), 0);
    }

    let mut stream_edge_count = 0usize;
    for (stream_id, stream_events) in grouped.iter_mut() {
        stream_events.sort_by_key(|event| event.sequence);
        let mut by_sequence = BTreeMap::<u64, &AuditEvent>::new();
        let mut conflicting_sequences = BTreeSet::<u64>::new();
        for event in stream_events.iter() {
            if by_sequence.contains_key(&event.sequence) {
                conflicting_sequences.insert(event.sequence);
            } else {
                by_sequence.insert(event.sequence, event);
            }
        }
        for sequence in &conflicting_sequences {
            by_sequence.remove(sequence);
            reject(
                &mut findings,
                &mut rejected,
                Finding::DuplicateStreamSequence {
                    stream_id: stream_id.clone(),
                    sequence: *sequence,
                },
            );
        }

        if !by_sequence.contains_key(&0) {
            defer(
                &mut findings,
                &mut incomplete,
                Finding::MissingStreamAction {
                    stream_id: stream_id.clone(),
                    sequence: 0,
                },
            );
        }

        for event in stream_events.iter() {
            if conflicting_sequences.contains(&event.sequence) {
                continue;
            }
            if event.sequence == 0 {
                if event.previous_action_hash.is_some() {
                    reject(
                        &mut findings,
                        &mut rejected,
                        Finding::InvalidStreamGenesis {
                            action_hash: event.action_hash.clone(),
                        },
                    );
                }
                continue;
            }

            if event.previous_action_hash.is_none() {
                reject(
                    &mut findings,
                    &mut rejected,
                    Finding::MissingPreviousAction {
                        action_hash: event.action_hash.clone(),
                    },
                );
                continue;
            }

            match by_sequence.get(&(event.sequence - 1)) {
                Some(previous) => {
                    if event.previous_action_hash.as_deref() != Some(previous.action_hash.as_str()) {
                        reject(
                            &mut findings,
                            &mut rejected,
                            Finding::PreviousActionMismatch {
                                action_hash: event.action_hash.clone(),
                            },
                        );
                    } else if add_edge(
                        &mut adjacency,
                        &mut indegree,
                        &previous.action_hash,
                        &event.action_hash,
                    ) {
                        stream_edge_count += 1;
                    }
                }
                None => {
                    let claimed_previous = event.previous_action_hash.as_ref().unwrap();
                    if events_by_hash.contains_key(claimed_previous) {
                        reject(
                            &mut findings,
                            &mut rejected,
                            Finding::PreviousActionMismatch {
                                action_hash: event.action_hash.clone(),
                            },
                        );
                    } else {
                        defer(
                            &mut findings,
                            &mut incomplete,
                            Finding::MissingStreamAction {
                                stream_id: stream_id.clone(),
                                sequence: event.sequence - 1,
                            },
                        );
                    }
                }
            }
        }

        if let Some(checkpoint) = checkpoints.get(stream_id) {
            match by_sequence.get(&checkpoint.head_sequence) {
                None => defer(
                    &mut findings,
                    &mut incomplete,
                    Finding::MissingStreamAction {
                        stream_id: stream_id.clone(),
                        sequence: checkpoint.head_sequence,
                    },
                ),
                Some(head) if head.action_hash != checkpoint.head_action_hash => reject(
                    &mut findings,
                    &mut rejected,
                    Finding::CheckpointHeadMismatch {
                        stream_id: stream_id.clone(),
                    },
                ),
                Some(_) => {}
            }
        }
    }

    let mut causal_edge_count = 0usize;
    for event in events_by_hash.values() {
        let mut seen_parents = BTreeSet::new();
        for parent_hash in &event.causal_parents {
            if !has_required_text(parent_hash) {
                reject(
                    &mut findings,
                    &mut rejected,
                    Finding::EmptyCausalParent {
                        action_hash: event.action_hash.clone(),
                    },
                );
                continue;
            }
            if !seen_parents.insert(parent_hash.clone()) {
                reject(
                    &mut findings,
                    &mut rejected,
                    Finding::DuplicateCausalParent {
                        action_hash: event.action_hash.clone(),
                    },
                );
                continue;
            }
            if parent_hash == &event.action_hash {
                reject(
                    &mut findings,
                    &mut rejected,
                    Finding::SelfCausalParent {
                        action_hash: event.action_hash.clone(),
                    },
                );
                continue;
            }
            let Some(parent) = events_by_hash.get(parent_hash) else {
                defer(
                    &mut findings,
                    &mut incomplete,
                    Finding::MissingCausalParent {
                        action_hash: event.action_hash.clone(),
                        parent_hash: parent_hash.clone(),
                    },
                );
                continue;
            };
            if parent.tenant_id != event.tenant_id {
                reject(
                    &mut findings,
                    &mut rejected,
                    Finding::CrossTenantParent {
                        action_hash: event.action_hash.clone(),
                        parent_hash: parent_hash.clone(),
                    },
                );
                continue;
            }
            if parent.patient_id != event.patient_id {
                reject(
                    &mut findings,
                    &mut rejected,
                    Finding::CrossPatientParent {
                        action_hash: event.action_hash.clone(),
                        parent_hash: parent_hash.clone(),
                    },
                );
                continue;
            }
            add_edge(
                &mut adjacency,
                &mut indegree,
                parent_hash,
                &event.action_hash,
            );
            // Count the explicit assertion even when the same ordering is
            // already implied by this event's per-agent predecessor link.
            causal_edge_count += 1;
        }
    }

    // Stable Kahn topological sort is only a deterministic display linearization.
    // It does not add order between nodes that have no path between them.
    let mut remaining_indegree = indegree.clone();
    let mut ready = BTreeSet::<String>::new();
    for (node, degree) in &remaining_indegree {
        if *degree == 0 {
            ready.insert(node.clone());
        }
    }
    let mut display_order = Vec::with_capacity(events_by_hash.len());
    while let Some(node) = ready.iter().next().cloned() {
        ready.remove(&node);
        display_order.push(node.clone());
        if let Some(children) = adjacency.get(&node) {
            for child in children {
                let degree = remaining_indegree
                    .get_mut(child)
                    .expect("child node exists in indegree map");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(child.clone());
                }
            }
        }
    }
    if display_order.len() != events_by_hash.len() {
        reject(
            &mut findings,
            &mut rejected,
            Finding::CausalCycle,
        );
        display_order.clear();
    }

    let mut incomparable_pairs = 0usize;
    if !rejected && !incomplete {
        let nodes: Vec<String> = events_by_hash.keys().cloned().collect();
        let reachability: BTreeMap<String, BTreeSet<String>> = nodes
            .iter()
            .map(|node| (node.clone(), reachable_from(node, &adjacency)))
            .collect();
        for left_index in 0..nodes.len() {
            for right_index in (left_index + 1)..nodes.len() {
                let left = &nodes[left_index];
                let right = &nodes[right_index];
                let left_before_right = reachability[left].contains(right);
                let right_before_left = reachability[right].contains(left);
                if !left_before_right && !right_before_left {
                    incomparable_pairs += 1;
                }
            }
        }
    } else {
        display_order.clear();
    }

    let status = if rejected {
        ViewStatus::Rejected
    } else if incomplete {
        ViewStatus::Incomplete
    } else {
        ViewStatus::Verified
    };

    VerificationReport {
        status,
        findings: findings.into_iter().collect(),
        event_count: events_by_hash.len(),
        stream_count: grouped.len(),
        stream_edge_count,
        causal_edge_count,
        incomparable_pairs,
        duplicate_observations,
        deterministic_display_order: display_order,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(
        action_hash: &str,
        stream_id: &str,
        author: &str,
        sequence: u64,
        previous_action_hash: Option<&str>,
        patient_id: &str,
        tenant_id: &str,
        causal_parents: &[&str],
        event_time_ms: Option<i64>,
    ) -> AuditEvent {
        AuditEvent {
            action_hash: action_hash.to_string(),
            stream_id: stream_id.to_string(),
            claimed_author: author.to_string(),
            sequence,
            previous_action_hash: previous_action_hash.map(str::to_string),
            tenant_id: tenant_id.to_string(),
            patient_id: patient_id.to_string(),
            resource: Some(ResourceVersionRef {
                resource_type: "Observation".to_string(),
                logical_id: format!("resource-{action_hash}"),
                version_id: Some("v1".to_string()),
                content_digest: Some(format!("sha256:{action_hash}")),
            }),
            event_kind: "clinical_read".to_string(),
            request_id: format!("request-{action_hash}"),
            authorization_receipt_id: Some("receipt-1".to_string()),
            policy_version: Some("policy-v1".to_string()),
            purpose: "treatment".to_string(),
            recipient: "clinic-a".to_string(),
            action: "read".to_string(),
            outcome: "committed".to_string(),
            event_time_ms,
            observed_at_ms: None,
            causal_parents: causal_parents.iter().map(|value| value.to_string()).collect(),
        }
    }

    fn checkpoint(
        stream_id: &str,
        author: &str,
        head_sequence: u64,
        head_action_hash: &str,
    ) -> TrustedStreamCheckpoint {
        TrustedStreamCheckpoint {
            stream_id: stream_id.to_string(),
            authenticated_author: author.to_string(),
            head_sequence,
            head_action_hash: head_action_hash.to_string(),
        }
    }

    fn code_present(report: &VerificationReport, code: &str) -> bool {
        report.findings.iter().any(|finding| finding.code() == code)
    }

    fn scenario(case_id: &str) -> (Vec<AuditEvent>, Vec<TrustedStreamCheckpoint>) {
        let p0 = event("a0", "agent-a", "agent-a", 0, None, "patient-1", "tenant-1", &[], Some(100));
        let p1 = event("a1", "agent-a", "agent-a", 1, Some("a0"), "patient-1", "tenant-1", &[], Some(200));
        let q0 = event("b0", "agent-b", "agent-b", 0, None, "patient-1", "tenant-1", &[], Some(50));
        let q1 = event("b1", "agent-b", "agent-b", 1, Some("b0"), "patient-1", "tenant-1", &[], Some(300));
        match case_id {
            "valid_two_agent_unrelated" => (
                vec![p0, p1, q0, q1],
                vec![
                    checkpoint("agent-a", "agent-a", 1, "a1"),
                    checkpoint("agent-b", "agent-b", 1, "b1"),
                ],
            ),
            "explicit_cross_agent_parent" => {
                let child = event("b0", "agent-b", "agent-b", 0, None, "patient-1", "tenant-1", &["a0"], Some(10));
                (
                    vec![p0, child],
                    vec![
                        checkpoint("agent-a", "agent-a", 0, "a0"),
                        checkpoint("agent-b", "agent-b", 0, "b0"),
                    ],
                )
            }
            "out_of_order_publication" => (
                vec![p1, p0],
                vec![checkpoint("agent-a", "agent-a", 1, "a1")],
            ),
            "clock_skew_unrelated" => (
                vec![
                    event("a0", "agent-a", "agent-a", 0, None, "patient-1", "tenant-1", &[], Some(9_000)),
                    event("b0", "agent-b", "agent-b", 0, None, "patient-1", "tenant-1", &[], Some(-9_000)),
                ],
                vec![
                    checkpoint("agent-a", "agent-a", 0, "a0"),
                    checkpoint("agent-b", "agent-b", 0, "b0"),
                ],
            ),
            "same_timestamp_unrelated" => (
                vec![
                    event("a0", "agent-a", "agent-a", 0, None, "patient-1", "tenant-1", &[], Some(10)),
                    event("b0", "agent-b", "agent-b", 0, None, "patient-1", "tenant-1", &[], Some(10)),
                ],
                vec![
                    checkpoint("agent-a", "agent-a", 0, "a0"),
                    checkpoint("agent-b", "agent-b", 0, "b0"),
                ],
            ),
            "missing_checkpoint" => (vec![p0], vec![]),
            "missing_stream_action" => {
                let a2 = event("a2", "agent-a", "agent-a", 2, Some("a1"), "patient-1", "tenant-1", &[], Some(300));
                (
                    vec![p0, a2],
                    vec![checkpoint("agent-a", "agent-a", 2, "a2")],
                )
            }
            "missing_causal_parent" => {
                let child = event("b0", "agent-b", "agent-b", 0, None, "patient-1", "tenant-1", &["not-yet-visible"], Some(10));
                (
                    vec![child],
                    vec![checkpoint("agent-b", "agent-b", 0, "b0")],
                )
            }
            "author_mismatch" => (
                vec![event("a0", "agent-a", "attacker", 0, None, "patient-1", "tenant-1", &[], Some(10))],
                vec![checkpoint("agent-a", "agent-a", 0, "a0")],
            ),
            "cross_patient_parent" => {
                let parent = event("a0", "agent-a", "agent-a", 0, None, "patient-1", "tenant-1", &[], Some(10));
                let child = event("b0", "agent-b", "agent-b", 0, None, "patient-2", "tenant-1", &["a0"], Some(20));
                (
                    vec![parent, child],
                    vec![
                        checkpoint("agent-a", "agent-a", 0, "a0"),
                        checkpoint("agent-b", "agent-b", 0, "b0"),
                    ],
                )
            }
            "cross_tenant_parent" => {
                let parent = event("a0", "agent-a", "agent-a", 0, None, "patient-1", "tenant-1", &[], Some(10));
                let child = event("b0", "agent-b", "agent-b", 0, None, "patient-1", "tenant-2", &["a0"], Some(20));
                (
                    vec![parent, child],
                    vec![
                        checkpoint("agent-a", "agent-a", 0, "a0"),
                        checkpoint("agent-b", "agent-b", 0, "b0"),
                    ],
                )
            }
            "previous_action_mismatch" => {
                let child = event("a1", "agent-a", "agent-a", 1, Some("wrong-parent"), "patient-1", "tenant-1", &[], Some(20));
                (
                    vec![p0, child],
                    vec![checkpoint("agent-a", "agent-a", 1, "a1")],
                )
            }
            "conflicting_duplicate_action" => {
                let mut conflicting = p0.clone();
                conflicting.patient_id = "patient-other".to_string();
                (
                    vec![p0, conflicting],
                    vec![checkpoint("agent-a", "agent-a", 0, "a0")],
                )
            }
            "identical_duplicate_action" => (
                vec![p0.clone(), p0],
                vec![checkpoint("agent-a", "agent-a", 0, "a0")],
            ),
            "checkpoint_head_mismatch" => (
                vec![p0],
                vec![checkpoint("agent-a", "agent-a", 0, "not-a0")],
            ),
            "causal_cycle" => {
                let a = event("a0", "agent-a", "agent-a", 0, None, "patient-1", "tenant-1", &["b0"], Some(10));
                let b = event("b0", "agent-b", "agent-b", 0, None, "patient-1", "tenant-1", &["a0"], Some(20));
                (
                    vec![a, b],
                    vec![
                        checkpoint("agent-a", "agent-a", 0, "a0"),
                        checkpoint("agent-b", "agent-b", 0, "b0"),
                    ],
                )
            }
            "self_parent" => {
                let a = event("a0", "agent-a", "agent-a", 0, None, "patient-1", "tenant-1", &["a0"], Some(10));
                (vec![a], vec![checkpoint("agent-a", "agent-a", 0, "a0")])
            }
            "duplicate_causal_parent" => {
                let child = event("b0", "agent-b", "agent-b", 0, None, "patient-1", "tenant-1", &["a0", "a0"], Some(20));
                (
                    vec![p0, child],
                    vec![
                        checkpoint("agent-a", "agent-a", 0, "a0"),
                        checkpoint("agent-b", "agent-b", 0, "b0"),
                    ],
                )
            }
            "uncheckpointed_suffix" => (
                vec![
                    p0,
                    event("a1", "agent-a", "agent-a", 1, Some("a0"), "patient-1", "tenant-1", &[], Some(20)),
                ],
                vec![checkpoint("agent-a", "agent-a", 0, "a0")],
            ),
            _ => panic!("no executable scenario for fixture {case_id}"),
        }
    }

    #[test]
    fn frozen_fixture_catalogue_is_executable() {
        let mut ids = BTreeSet::new();
        let mut rows = 0usize;
        for line in include_str!("../fixtures/v1-cases.tsv").lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('|').collect();
            assert_eq!(fields.len(), 4, "fixture row must have four fields");
            assert!(!fields[0].trim().is_empty(), "fixture ID cannot be empty");
            assert!(ids.insert(fields[0]), "duplicate fixture ID: {}", fields[0]);

            let (events, checkpoints) = scenario(fields[0]);
            let report = verify_audit_view(&events, &checkpoints);
            assert_eq!(format!("{:?}", report.status), fields[1], "status for {}", fields[0]);
            if fields[2] == "none" {
                assert!(report.findings.is_empty(), "unexpected findings for {}", fields[0]);
            } else {
                assert!(code_present(&report, fields[2]), "missing {} for {}", fields[2], fields[0]);
            }
            assert!(!fields[3].trim().is_empty(), "fixture description cannot be empty");
            rows += 1;
        }
        assert_eq!(rows, 19, "frozen v1 fixture count changed");
    }

    #[test]
    fn unrelated_agents_remain_incomparable_even_with_skewed_clocks() {
        let (events, checkpoints) = scenario("clock_skew_unrelated");
        let report = verify_audit_view(&events, &checkpoints);
        assert_eq!(report.status, ViewStatus::Verified);
        assert_eq!(report.causal_edge_count, 0);
        assert_eq!(report.stream_edge_count, 0);
        assert_eq!(report.incomparable_pairs, 1);
    }

    #[test]
    fn explicit_cross_agent_parent_creates_only_the_declared_causal_edge() {
        let (events, checkpoints) = scenario("explicit_cross_agent_parent");
        let report = verify_audit_view(&events, &checkpoints);
        assert_eq!(report.status, ViewStatus::Verified);
        assert_eq!(report.causal_edge_count, 1);
        assert_eq!(report.incomparable_pairs, 0);
    }

    #[test]
    fn publication_order_does_not_change_verification_or_display_order() {
        let (events, checkpoints) = scenario("valid_two_agent_unrelated");
        let forward = verify_audit_view(&events, &checkpoints);
        let mut reversed = events.clone();
        reversed.reverse();
        let backward = verify_audit_view(&reversed, &checkpoints);
        assert_eq!(forward, backward);
        assert_eq!(forward.status, ViewStatus::Verified);
        assert_eq!(forward.incomparable_pairs, 4);
    }

    #[test]
    fn same_timestamp_does_not_create_cross_agent_causality() {
        let (events, checkpoints) = scenario("same_timestamp_unrelated");
        let report = verify_audit_view(&events, &checkpoints);
        assert_eq!(report.status, ViewStatus::Verified);
        assert_eq!(report.incomparable_pairs, 1);
        assert_eq!(report.causal_edge_count, 0);
    }

    #[test]
    fn a_complete_peer_union_converges_after_duplicate_observations() {
        let (events, checkpoints) = scenario("valid_two_agent_unrelated");
        let mut union = events.clone();
        union.extend(events.iter().cloned());
        let report = verify_audit_view(&union, &checkpoints);
        assert_eq!(report.status, ViewStatus::Verified);
        assert_eq!(report.event_count, 4);
        assert_eq!(report.duplicate_observations, 4);
    }

    #[test]
    fn incomplete_peer_visibility_is_not_misreported_as_verified() {
        let (events, checkpoints) = scenario("missing_causal_parent");
        let report = verify_audit_view(&events, &checkpoints);
        assert_eq!(report.status, ViewStatus::Incomplete);
        assert!(code_present(&report, "missing_causal_parent"));
        assert!(report.deterministic_display_order.is_empty());
    }

    #[test]
    fn trusted_checkpoint_detects_a_truncated_stream_suffix() {
        let head = event("a1", "agent-a", "agent-a", 1, Some("a0"), "patient-1", "tenant-1", &[], Some(20));
        let genesis = event("a0", "agent-a", "agent-a", 0, None, "patient-1", "tenant-1", &[], Some(10));
        let report = verify_audit_view(
            &[genesis],
            &[checkpoint("agent-a", "agent-a", 1, &head.action_hash)],
        );
        assert_eq!(report.status, ViewStatus::Incomplete);
        assert!(code_present(&report, "missing_stream_action"));
    }

    #[test]
    fn deterministic_display_linearization_never_claims_total_causality() {
        let (events, checkpoints) = scenario("valid_two_agent_unrelated");
        let report = verify_audit_view(&events, &checkpoints);
        assert_eq!(report.deterministic_display_order.len(), 4);
        assert_eq!(report.incomparable_pairs, 4);
        // A display order exists, but it does not change the independently
        // measured incomparable-pair count.
    }
}
