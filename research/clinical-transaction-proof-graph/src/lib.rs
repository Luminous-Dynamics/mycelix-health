//! Offline structural verifier for clinical transaction evidence graphs.
//!
//! Graph structure, cross-event binding, causal transitions, component
//! qualification freshness, and recipient-observed effect binding are verified
//! here. Cryptographic verification is injected through EvidenceAuthenticator.
//! The fixture authenticator in these tests is not production cryptography.

use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationBinding {
    pub operation_id: String,
    pub attempt_id: String,
    pub patient_ref_digest: String,
    pub tenant_ref_digest: String,
    pub authenticated_actor_ref_digest: String,
    pub recipient_ref_digest: String,
    pub purpose: String,
    pub action: String,
    pub resource_set_digest: String,
    pub source_system_ref_digest: String,
    pub source_resource_version: String,
    pub source_resource_digest: String,
    pub encounter_ref_digest: Option<String>,
    pub consent_revision_digest: String,
    pub revocation_epoch: u64,
    pub policy_digest: String,
    pub idempotency_key_digest: String,
}

impl OperationBinding {
    fn is_complete(&self) -> bool {
        [
            &self.operation_id,
            &self.attempt_id,
            &self.patient_ref_digest,
            &self.tenant_ref_digest,
            &self.authenticated_actor_ref_digest,
            &self.recipient_ref_digest,
            &self.purpose,
            &self.action,
            &self.resource_set_digest,
            &self.source_system_ref_digest,
            &self.source_resource_version,
            &self.source_resource_digest,
            &self.consent_revision_digest,
            &self.policy_digest,
            &self.idempotency_key_digest,
        ]
        .iter()
        .all(|value| !value.trim().is_empty())
            && self
                .encounter_ref_digest
                .as_ref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(true)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorRole {
    Client,
    IdentityProvider,
    PolicyEngine,
    Gateway,
    ExecutionBackend,
    Recipient,
    Reconciler,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitOutcome {
    Committed,
    Rejected,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeliveryOutcome {
    RecipientEffectObserved,
    Failed,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventKind {
    Requested,
    IdentityBound,
    AuthorityDecided,
    Prepared,
    CommitAttempted,
    CommitResult(CommitOutcome),
    CommitReconciled(CommitOutcome),
    DeliveryAttempted,
    DeliveryResult(DeliveryOutcome),
    DeliveryReconciled(DeliveryOutcome),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParentRef {
    pub event_id: String,
    pub event_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceEvent {
    pub event_id: String,
    pub event_digest: String,
    pub signature: String,
    pub binding: OperationBinding,
    pub kind: EventKind,
    pub actor_id: String,
    pub actor_ref_digest: String,
    pub actor_role: ActorRole,
    pub actor_sequence: u64,
    pub observed_at_ms: i64,
    pub component_id: String,
    pub component_build_digest: String,
    pub component_config_digest: String,
    pub parents: Vec<ParentRef>,
    pub observed_resource_set_digest: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofGraph {
    pub binding: OperationBinding,
    pub events: Vec<EvidenceEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentQualification {
    pub build_digest: String,
    pub config_digest: String,
    pub valid_until_ms: i64,
    pub independently_issued: bool,
}

pub trait EvidenceAuthenticator {
    /// Production implementations must verify canonical event digests and actor
    /// signatures against trusted, current keys.
    fn verify_event_and_signature(&self, event: &EvidenceEvent) -> bool;
}

pub trait QualificationRegistry {
    fn qualification_for(&self, component_id: &str) -> Option<ComponentQualification>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Verdict {
    Verified,
    Rejected,
    Defer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationConclusion {
    RecipientEffectObserved,
    RejectedBeforeCommit,
    DeliveryFailed,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReasonCode {
    EmptyGraph,
    IncompleteBinding,
    DuplicateEventId,
    CrossEventBindingMismatch,
    InvalidDigestOrSignature,
    MissingComponentQualification,
    SelfIssuedQualification,
    QualificationExpired,
    ComponentBuildMismatch,
    ComponentConfigMismatch,
    MissingParentEvidence,
    DuplicateParentReference,
    ParentDigestMismatch,
    MultipleOrMissingRoots,
    CausalCycle,
    MissingRequiredParent,
    InvalidStateTransition,
    DuplicateActorSequence,
    ActorSequenceGap,
    MissingCommitOutcome,
    UnknownCommitOutcome,
    MissingDeliveryOutcome,
    UnknownDeliveryOutcome,
    RecipientWitnessRequired,
    RecipientIdentityMismatch,
    ResourceSetEffectMismatch,
    DeliveryAfterRejectedCommit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReport {
    pub verdict: Verdict,
    pub conclusion: OperationConclusion,
    pub findings: Vec<ReasonCode>,
    pub verified_event_count: usize,
}

impl VerificationReport {
    fn rejected(reason: ReasonCode, event_count: usize) -> Self {
        Self {
            verdict: Verdict::Rejected,
            conclusion: OperationConclusion::Unresolved,
            findings: vec![reason],
            verified_event_count: event_count,
        }
    }

    fn defer(reason: ReasonCode, event_count: usize) -> Self {
        Self {
            verdict: Verdict::Defer,
            conclusion: OperationConclusion::Unresolved,
            findings: vec![reason],
            verified_event_count: event_count,
        }
    }

    fn verified(conclusion: OperationConclusion, event_count: usize) -> Self {
        Self {
            verdict: Verdict::Verified,
            conclusion,
            findings: Vec::new(),
            verified_event_count: event_count,
        }
    }
}

/// Verify one frozen operation attempt. The result is deterministic for a fixed
/// graph, verification time, authenticator and qualification registry.
pub fn verify_graph<A, Q>(
    graph: &ProofGraph,
    now_ms: i64,
    authenticator: &A,
    qualifications: &Q,
) -> VerificationReport
where
    A: EvidenceAuthenticator,
    Q: QualificationRegistry,
{
    let event_count = graph.events.len();
    if event_count == 0 {
        return VerificationReport::defer(ReasonCode::EmptyGraph, event_count);
    }
    if !graph.binding.is_complete() {
        return VerificationReport::defer(ReasonCode::IncompleteBinding, event_count);
    }

    let mut event_index = HashMap::with_capacity(event_count);
    for (index, event) in graph.events.iter().enumerate() {
        if event_index.insert(event.event_id.as_str(), index).is_some() {
            return VerificationReport::rejected(ReasonCode::DuplicateEventId, event_count);
        }
        if event.binding != graph.binding {
            return VerificationReport::rejected(
                ReasonCode::CrossEventBindingMismatch,
                event_count,
            );
        }
        if event.event_id.trim().is_empty()
            || event.event_digest.trim().is_empty()
            || event.signature.trim().is_empty()
            || event.actor_id.trim().is_empty()
            || event.actor_ref_digest.trim().is_empty()
            || event.component_id.trim().is_empty()
            || event.component_build_digest.trim().is_empty()
            || event.component_config_digest.trim().is_empty()
        {
            return VerificationReport::defer(ReasonCode::IncompleteBinding, event_count);
        }
        if !authenticator.verify_event_and_signature(event) {
            return VerificationReport::rejected(
                ReasonCode::InvalidDigestOrSignature,
                event_count,
            );
        }

        let Some(qualification) = qualifications.qualification_for(&event.component_id) else {
            return VerificationReport::defer(
                ReasonCode::MissingComponentQualification,
                event_count,
            );
        };
        if !qualification.independently_issued {
            return VerificationReport::defer(ReasonCode::SelfIssuedQualification, event_count);
        }
        if qualification.valid_until_ms <= now_ms {
            return VerificationReport::defer(ReasonCode::QualificationExpired, event_count);
        }
        if qualification.build_digest != event.component_build_digest {
            return VerificationReport::defer(ReasonCode::ComponentBuildMismatch, event_count);
        }
        if qualification.config_digest != event.component_config_digest {
            return VerificationReport::defer(ReasonCode::ComponentConfigMismatch, event_count);
        }
    }

    let mut roots = Vec::new();
    let mut indegree = vec![0usize; event_count];
    let mut children = vec![Vec::<usize>::new(); event_count];

    for (child_index, event) in graph.events.iter().enumerate() {
        if event.parents.is_empty() {
            if event.kind != EventKind::Requested {
                return VerificationReport::rejected(
                    ReasonCode::MultipleOrMissingRoots,
                    event_count,
                );
            }
            roots.push(child_index);
            continue;
        }

        let mut seen_parents = std::collections::HashSet::new();
        for parent in &event.parents {
            if !seen_parents.insert(parent.event_id.as_str()) {
                return VerificationReport::rejected(
                    ReasonCode::DuplicateParentReference,
                    event_count,
                );
            }
            let Some(parent_index) = event_index.get(parent.event_id.as_str()).copied() else {
                return VerificationReport::defer(ReasonCode::MissingParentEvidence, event_count);
            };
            let parent_event = &graph.events[parent_index];
            if parent_event.event_digest != parent.event_digest {
                return VerificationReport::rejected(
                    ReasonCode::ParentDigestMismatch,
                    event_count,
                );
            }
            indegree[child_index] += 1;
            children[parent_index].push(child_index);
        }
    }

    // Deterministic Kahn topological sort. Wall-clock timestamps are deliberately
    // excluded: causal parent edges, not timestamp order, establish precedence.
    let mut ready: Vec<usize> = (0..event_count)
        .filter(|index| indegree[*index] == 0)
        .collect();
    ready.sort_by(|left, right| {
        graph.events[*left]
            .event_id
            .cmp(&graph.events[*right].event_id)
    });
    let mut queue: VecDeque<usize> = ready.into();
    let mut topological_order = Vec::with_capacity(event_count);

    while let Some(parent_index) = queue.pop_front() {
        topological_order.push(parent_index);
        children[parent_index].sort_by(|left, right| {
            graph.events[*left]
                .event_id
                .cmp(&graph.events[*right].event_id)
        });
        for child_index in &children[parent_index] {
            indegree[*child_index] -= 1;
            if indegree[*child_index] == 0 {
                queue.push_back(*child_index);
            }
        }
    }

    if topological_order.len() != event_count {
        return VerificationReport::rejected(ReasonCode::CausalCycle, event_count);
    }
    if roots.len() != 1 {
        return VerificationReport::rejected(ReasonCode::MultipleOrMissingRoots, event_count);
    }

    if let Some(reason) = validate_actor_sequences(&graph.events) {
        return match reason {
            ReasonCode::ActorSequenceGap => VerificationReport::defer(reason, event_count),
            _ => VerificationReport::rejected(reason, event_count),
        };
    }

    for event in &graph.events {
        if !valid_role_for_event(event) {
            return VerificationReport::rejected(
                ReasonCode::InvalidStateTransition,
                event_count,
            );
        }
        if !valid_parent_transition(event, graph) {
            return VerificationReport::rejected(
                ReasonCode::MissingRequiredParent,
                event_count,
            );
        }
    }

    let mut latest_commit: Option<(CommitOutcome, &EvidenceEvent)> = None;
    let mut latest_delivery: Option<(DeliveryOutcome, &EvidenceEvent)> = None;
    for index in topological_order {
        let event = &graph.events[index];
        match event.kind {
            EventKind::CommitResult(outcome) | EventKind::CommitReconciled(outcome) => {
                latest_commit = Some((outcome, event));
            }
            EventKind::DeliveryResult(outcome) | EventKind::DeliveryReconciled(outcome) => {
                latest_delivery = Some((outcome, event));
            }
            _ => {}
        }
    }

    let Some((commit_outcome, _)) = latest_commit else {
        return VerificationReport::defer(ReasonCode::MissingCommitOutcome, event_count);
    };
    match commit_outcome {
        CommitOutcome::Unknown => {
            return VerificationReport::defer(ReasonCode::UnknownCommitOutcome, event_count);
        }
        CommitOutcome::Rejected => {
            if latest_delivery.is_some() {
                return VerificationReport::rejected(
                    ReasonCode::DeliveryAfterRejectedCommit,
                    event_count,
                );
            }
            return VerificationReport::verified(
                OperationConclusion::RejectedBeforeCommit,
                event_count,
            );
        }
        CommitOutcome::Committed => {}
    }

    let Some((delivery_outcome, delivery_event)) = latest_delivery else {
        return VerificationReport::defer(ReasonCode::MissingDeliveryOutcome, event_count);
    };

    match delivery_outcome {
        DeliveryOutcome::Unknown => {
            VerificationReport::defer(ReasonCode::UnknownDeliveryOutcome, event_count)
        }
        DeliveryOutcome::Failed => {
            if delivery_event.actor_role != ActorRole::Recipient {
                VerificationReport::rejected(ReasonCode::RecipientWitnessRequired, event_count)
            } else if delivery_event.actor_ref_digest != graph.binding.recipient_ref_digest {
                VerificationReport::rejected(ReasonCode::RecipientIdentityMismatch, event_count)
            } else {
                VerificationReport::verified(OperationConclusion::DeliveryFailed, event_count)
            }
        }
        DeliveryOutcome::RecipientEffectObserved => {
            if delivery_event.actor_role != ActorRole::Recipient {
                return VerificationReport::rejected(
                    ReasonCode::RecipientWitnessRequired,
                    event_count,
                );
            }
            if delivery_event.actor_ref_digest != graph.binding.recipient_ref_digest {
                return VerificationReport::rejected(
                    ReasonCode::RecipientIdentityMismatch,
                    event_count,
                );
            }
            if delivery_event.observed_resource_set_digest.as_deref()
                != Some(graph.binding.resource_set_digest.as_str())
            {
                return VerificationReport::rejected(
                    ReasonCode::ResourceSetEffectMismatch,
                    event_count,
                );
            }
            VerificationReport::verified(
                OperationConclusion::RecipientEffectObserved,
                event_count,
            )
        }
    }
}

fn validate_actor_sequences(events: &[EvidenceEvent]) -> Option<ReasonCode> {
    let mut by_actor: HashMap<&str, Vec<u64>> = HashMap::new();
    for event in events {
        by_actor
            .entry(event.actor_id.as_str())
            .or_default()
            .push(event.actor_sequence);
    }
    for sequences in by_actor.values_mut() {
        sequences.sort_unstable();
        for pair in sequences.windows(2) {
            if pair[0] == pair[1] {
                return Some(ReasonCode::DuplicateActorSequence);
            }
            if pair[1] != pair[0] + 1 {
                return Some(ReasonCode::ActorSequenceGap);
            }
        }
    }
    None
}

fn valid_role_for_event(event: &EvidenceEvent) -> bool {
    match event.kind {
        EventKind::Requested => event.actor_role == ActorRole::Client,
        EventKind::IdentityBound => event.actor_role == ActorRole::IdentityProvider,
        EventKind::AuthorityDecided => event.actor_role == ActorRole::PolicyEngine,
        EventKind::Prepared | EventKind::CommitAttempted | EventKind::DeliveryAttempted => {
            event.actor_role == ActorRole::Gateway
        }
        EventKind::CommitResult(_) => event.actor_role == ActorRole::ExecutionBackend,
        EventKind::CommitReconciled(_) => event.actor_role == ActorRole::Reconciler,
        EventKind::DeliveryResult(_) | EventKind::DeliveryReconciled(_) => {
            event.actor_role == ActorRole::Recipient
        }
    }
}

fn valid_parent_transition(event: &EvidenceEvent, graph: &ProofGraph) -> bool {
    let has_parent = |predicate: &dyn Fn(EventKind) -> bool| {
        event.parents.iter().any(|parent| {
            graph
                .events
                .iter()
                .find(|candidate| candidate.event_id == parent.event_id)
                .map(|candidate| predicate(candidate.kind))
                .unwrap_or(false)
        })
    };

    match event.kind {
        EventKind::Requested => event.parents.is_empty(),
        EventKind::IdentityBound => has_parent(&|kind| kind == EventKind::Requested),
        EventKind::AuthorityDecided => has_parent(&|kind| kind == EventKind::IdentityBound),
        EventKind::Prepared => has_parent(&|kind| kind == EventKind::AuthorityDecided),
        EventKind::CommitAttempted => has_parent(&|kind| kind == EventKind::Prepared),
        EventKind::CommitResult(_) => has_parent(&|kind| kind == EventKind::CommitAttempted),
        EventKind::CommitReconciled(outcome) => {
            outcome != CommitOutcome::Unknown
                && has_parent(&|kind| kind == EventKind::CommitResult(CommitOutcome::Unknown))
        }
        EventKind::DeliveryAttempted => {
            has_parent(&|kind| kind == EventKind::CommitResult(CommitOutcome::Committed))
                || has_parent(&|kind| {
                    kind == EventKind::CommitReconciled(CommitOutcome::Committed)
                })
        }
        EventKind::DeliveryResult(_) => has_parent(&|kind| kind == EventKind::DeliveryAttempted),
        EventKind::DeliveryReconciled(outcome) => {
            outcome != DeliveryOutcome::Unknown
                && has_parent(&|kind| kind == EventKind::DeliveryResult(DeliveryOutcome::Unknown))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixtureAuthenticator;

    impl EvidenceAuthenticator for FixtureAuthenticator {
        fn verify_event_and_signature(&self, event: &EvidenceEvent) -> bool {
            event.event_digest == format!("fixture-digest:{}", event.event_id)
                && event.signature == format!("fixture-signature:{}", event.actor_id)
        }
    }

    struct FixtureQualifications {
        entries: HashMap<String, ComponentQualification>,
    }

    impl QualificationRegistry for FixtureQualifications {
        fn qualification_for(&self, component_id: &str) -> Option<ComponentQualification> {
            self.entries.get(component_id).cloned()
        }
    }

    fn binding() -> OperationBinding {
        OperationBinding {
            operation_id: "operation-001".into(),
            attempt_id: "attempt-001".into(),
            patient_ref_digest: "sha256:patient".into(),
            tenant_ref_digest: "sha256:tenant".into(),
            authenticated_actor_ref_digest: "sha256:actor".into(),
            recipient_ref_digest: "sha256:recipient".into(),
            purpose: "treatment".into(),
            action: "read".into(),
            resource_set_digest: "sha256:resource-set-v7".into(),
            source_system_ref_digest: "sha256:source-system".into(),
            source_resource_version: "7".into(),
            source_resource_digest: "sha256:source-v7".into(),
            encounter_ref_digest: Some("sha256:encounter".into()),
            consent_revision_digest: "sha256:consent-v4".into(),
            revocation_epoch: 12,
            policy_digest: "sha256:policy-v3".into(),
            idempotency_key_digest: "sha256:idempotency-key".into(),
        }
    }

    fn qualifications() -> FixtureQualifications {
        let mut entries = HashMap::new();
        for component in [
            "client", "idp", "policy", "gateway", "backend", "recipient", "reconciler",
        ] {
            entries.insert(
                component.into(),
                ComponentQualification {
                    build_digest: format!("build:{component}"),
                    config_digest: format!("config:{component}"),
                    valid_until_ms: 10_000,
                    independently_issued: true,
                },
            );
        }
        FixtureQualifications { entries }
    }

    fn event(
        id: &str,
        actor: &str,
        role: ActorRole,
        sequence: u64,
        kind: EventKind,
        parents: &[&EvidenceEvent],
    ) -> EvidenceEvent {
        let component = match role {
            ActorRole::Client => "client",
            ActorRole::IdentityProvider => "idp",
            ActorRole::PolicyEngine => "policy",
            ActorRole::Gateway => "gateway",
            ActorRole::ExecutionBackend => "backend",
            ActorRole::Recipient => "recipient",
            ActorRole::Reconciler => "reconciler",
        };
        EvidenceEvent {
            event_id: id.into(),
            event_digest: format!("fixture-digest:{id}"),
            signature: format!("fixture-signature:{actor}"),
            binding: binding(),
            kind,
            actor_id: actor.into(),
            actor_ref_digest: if role == ActorRole::Recipient {
                "sha256:recipient".into()
            } else {
                format!("sha256:{actor}")
            },
            actor_role: role,
            actor_sequence: sequence,
            observed_at_ms: 1_000 - (sequence as i64 * 10),
            component_id: component.into(),
            component_build_digest: format!("build:{component}"),
            component_config_digest: format!("config:{component}"),
            parents: parents
                .iter()
                .map(|parent| ParentRef {
                    event_id: parent.event_id.clone(),
                    event_digest: parent.event_digest.clone(),
                })
                .collect(),
            observed_resource_set_digest: if kind
                == EventKind::DeliveryResult(DeliveryOutcome::RecipientEffectObserved)
            {
                Some("sha256:resource-set-v7".into())
            } else {
                None
            },
        }
    }

    fn successful_graph() -> ProofGraph {
        let requested = event(
            "event-01", "patient-client", ActorRole::Client, 1,
            EventKind::Requested, &[],
        );
        let identity = event(
            "event-02", "identity-service", ActorRole::IdentityProvider, 1,
            EventKind::IdentityBound, &[&requested],
        );
        let authority = event(
            "event-03", "policy-engine", ActorRole::PolicyEngine, 1,
            EventKind::AuthorityDecided, &[&identity],
        );
        let prepared = event(
            "event-04", "gateway", ActorRole::Gateway, 1,
            EventKind::Prepared, &[&authority],
        );
        let commit_attempt = event(
            "event-05", "gateway", ActorRole::Gateway, 2,
            EventKind::CommitAttempted, &[&prepared],
        );
        let committed = event(
            "event-06", "database-adapter", ActorRole::ExecutionBackend, 1,
            EventKind::CommitResult(CommitOutcome::Committed), &[&commit_attempt, &authority],
        );
        let delivery_attempt = event(
            "event-07", "gateway", ActorRole::Gateway, 3,
            EventKind::DeliveryAttempted, &[&committed],
        );
        let observed = event(
            "event-08", "recipient", ActorRole::Recipient, 1,
            EventKind::DeliveryResult(DeliveryOutcome::RecipientEffectObserved), &[&delivery_attempt],
        );
        ProofGraph {
            binding: binding(),
            events: vec![
                requested, identity, authority, prepared, commit_attempt, committed,
                delivery_attempt, observed,
            ],
        }
    }

    #[test]
    fn verifies_recipient_effect_with_exact_resource_set() {
        let graph = successful_graph();
        let report = verify_graph(&graph, 5_000, &FixtureAuthenticator, &qualifications());
        assert_eq!(report.verdict, Verdict::Verified);
        assert_eq!(report.conclusion, OperationConclusion::RecipientEffectObserved);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn causal_edges_not_wall_clock_order_define_order() {
        let mut graph = successful_graph();
        graph.events[0].observed_at_ms = 9_000;
        graph.events[1].observed_at_ms = 100;
        graph.events[2].observed_at_ms = 50;
        let report = verify_graph(&graph, 5_000, &FixtureAuthenticator, &qualifications());
        assert_eq!(report.verdict, Verdict::Verified);
    }

    #[test]
    fn parent_digest_tampering_is_rejected() {
        let mut graph = successful_graph();
        graph.events[1].parents[0].event_digest = "sha256:forged".into();
        let report = verify_graph(&graph, 5_000, &FixtureAuthenticator, &qualifications());
        assert_eq!(report.verdict, Verdict::Rejected);
        assert_eq!(report.findings, vec![ReasonCode::ParentDigestMismatch]);
    }

    #[test]
    fn missing_parent_evidence_defers() {
        let mut graph = successful_graph();
        graph.events[1].parents[0].event_id = "event-not-present".into();
        let report = verify_graph(&graph, 5_000, &FixtureAuthenticator, &qualifications());
        assert_eq!(report.verdict, Verdict::Defer);
        assert_eq!(report.findings, vec![ReasonCode::MissingParentEvidence]);
    }

    #[test]
    fn cross_patient_binding_is_rejected() {
        let mut graph = successful_graph();
        graph.events[3].binding.patient_ref_digest = "sha256:other-patient".into();
        let report = verify_graph(&graph, 5_000, &FixtureAuthenticator, &qualifications());
        assert_eq!(report.verdict, Verdict::Rejected);
        assert_eq!(report.findings, vec![ReasonCode::CrossEventBindingMismatch]);
    }

    #[test]
    fn expired_qualification_defers_instead_of_trusting_stale_evidence() {
        let graph = successful_graph();
        let mut registry = qualifications();
        registry.entries.get_mut("backend").unwrap().valid_until_ms = 5_000;
        let report = verify_graph(&graph, 5_000, &FixtureAuthenticator, &registry);
        assert_eq!(report.verdict, Verdict::Defer);
        assert_eq!(report.findings, vec![ReasonCode::QualificationExpired]);
    }

    #[test]
    fn untrusted_self_issued_qualification_defers() {
        let graph = successful_graph();
        let mut registry = qualifications();
        registry.entries.get_mut("backend").unwrap().independently_issued = false;
        let report = verify_graph(&graph, 5_000, &FixtureAuthenticator, &registry);
        assert_eq!(report.verdict, Verdict::Defer);
        assert_eq!(report.findings, vec![ReasonCode::SelfIssuedQualification]);
    }

    #[test]
    fn unknown_commit_outcome_stays_unresolved() {
        let mut graph = successful_graph();
        graph.events.truncate(6);
        graph.events[5].kind = EventKind::CommitResult(CommitOutcome::Unknown);
        let report = verify_graph(&graph, 5_000, &FixtureAuthenticator, &qualifications());
        assert_eq!(report.verdict, Verdict::Defer);
        assert_eq!(report.findings, vec![ReasonCode::UnknownCommitOutcome]);
    }

    #[test]
    fn sender_ack_cannot_substitute_for_recipient_effect_witness() {
        let mut graph = successful_graph();
        graph.events[7].actor_role = ActorRole::Gateway;
        graph.events[7].component_id = "gateway".into();
        graph.events[7].component_build_digest = "build:gateway".into();
        graph.events[7].component_config_digest = "config:gateway".into();
        graph.events[7].actor_id = "gateway".into();
        graph.events[7].actor_ref_digest = "sha256:gateway".into();
        graph.events[7].signature = "fixture-signature:gateway".into();
        let report = verify_graph(&graph, 5_000, &FixtureAuthenticator, &qualifications());
        assert_eq!(report.verdict, Verdict::Rejected);
        assert_eq!(report.findings, vec![ReasonCode::InvalidStateTransition]);
    }

    #[test]
    fn recipient_cannot_claim_a_different_resource_set() {
        let mut graph = successful_graph();
        graph.events[7].observed_resource_set_digest = Some("sha256:other-set".into());
        let report = verify_graph(&graph, 5_000, &FixtureAuthenticator, &qualifications());
        assert_eq!(report.verdict, Verdict::Rejected);
        assert_eq!(report.findings, vec![ReasonCode::ResourceSetEffectMismatch]);
    }

    #[test]
    fn cycle_is_rejected() {
        let mut graph = successful_graph();
        graph.events[0].parents.push(ParentRef {
            event_id: "event-08".into(),
            event_digest: "fixture-digest:event-08".into(),
        });
        let report = verify_graph(&graph, 5_000, &FixtureAuthenticator, &qualifications());
        assert_eq!(report.verdict, Verdict::Rejected);
        assert_eq!(report.findings, vec![ReasonCode::CausalCycle]);
    }

    #[test]
    fn actor_sequence_gap_defers_for_missing_actor_evidence() {
        let mut graph = successful_graph();
        graph.events[4].actor_sequence = 4;
        let report = verify_graph(&graph, 5_000, &FixtureAuthenticator, &qualifications());
        assert_eq!(report.verdict, Verdict::Defer);
        assert_eq!(report.findings, vec![ReasonCode::ActorSequenceGap]);
    }

    #[test]
    fn invalid_signature_is_rejected() {
        let mut graph = successful_graph();
        graph.events[2].signature = "tampered".into();
        let report = verify_graph(&graph, 5_000, &FixtureAuthenticator, &qualifications());
        assert_eq!(report.verdict, Verdict::Rejected);
        assert_eq!(report.findings, vec![ReasonCode::InvalidDigestOrSignature]);
    }

    #[test]
    fn rejected_commit_is_a_verified_non_release() {
        let mut graph = successful_graph();
        graph.events.truncate(6);
        graph.events[5].kind = EventKind::CommitResult(CommitOutcome::Rejected);
        let report = verify_graph(&graph, 5_000, &FixtureAuthenticator, &qualifications());
        assert_eq!(report.verdict, Verdict::Verified);
        assert_eq!(report.conclusion, OperationConclusion::RejectedBeforeCommit);
    }
}
