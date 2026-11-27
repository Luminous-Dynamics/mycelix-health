//! Transactional clinical evidence release contract.
//!
//! A production adapter must implement authority revalidation, idempotency
//! reservation, durable audit intent, the release side effect, and outcome
//! recording in one atomic transaction. The in-memory adapter is a test oracle;
//! it does not prove that a remote FHIR server or Holochain DHT has this ability.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Exact request binding carried by a signed authorization receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseBinding {
    pub patient_id: String,
    pub tenant_id: String,
    pub actor_id: String,
    pub recipient_id: String,
    pub purpose: String,
    pub action: String,
    pub resource_id: String,
    pub resource_version: String,
    pub resource_digest: String,
    pub clinical_event_time_ms: i64,
    pub encounter_id: Option<String>,
}

/// Versioned authorization receipt. Its signature must cover every field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizationReceipt {
    pub schema_version: u16,
    pub receipt_id: String,
    pub signer_key_id: String,
    pub signature: Vec<u8>,
    pub binding: ReleaseBinding,
    pub consent_id: String,
    pub consent_version: u64,
    pub revocation_epoch: u64,
    pub policy_version: String,
    pub nonce: String,
    pub idempotency_key: String,
    pub not_before_ms: i64,
    pub expires_at_ms: i64,
}

/// Input to the atomic backend. now_ms must come from a trusted server clock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtomicReleaseIntent {
    pub schema_version: u16,
    pub receipt_id: String,
    pub binding: ReleaseBinding,
    pub consent_id: String,
    pub expected_consent_version: u64,
    pub expected_revocation_epoch: u64,
    pub policy_version: String,
    pub nonce: String,
    pub idempotency_key: String,
    pub not_before_ms: i64,
    pub expires_at_ms: i64,
    pub now_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtomicCommitResult {
    Committed { release_receipt_id: String },
    AlreadyCommitted { release_receipt_id: String },
    Denied(&'static str),
    Deferred(&'static str),
    OutcomeUnknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoredExecution {
    Completed {
        intent: AtomicReleaseIntent,
        release_receipt_id: String,
    },
    Pending {
        intent: AtomicReleaseIntent,
    },
    NotFound,
    Unavailable,
}

/// Contract: commit_once atomically revalidates authority and resource versions,
/// consumes nonce/idempotency, appends durable audit intent, performs the
/// side effect once, and records the final outcome. It must not report Committed
/// before every step is durably committed.
pub trait TransactionalReleaseBackend: Send + Sync {
    fn commit_once(&self, intent: &AtomicReleaseIntent) -> AtomicCommitResult;
    fn lookup(&self, idempotency_key: &str) -> StoredExecution;
}

/// The application supplies a real cryptographic receipt verifier.
pub trait ReceiptVerifier: Send + Sync {
    fn verify(&self, receipt: &AuthorizationReceipt) -> bool;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReleaseOutcome {
    Released { release_receipt_id: String },
    AlreadyReleased { release_receipt_id: String },
    Denied(&'static str),
    Deferred(&'static str),
}

pub struct ReleaseService<B, V> {
    backend: Arc<B>,
    verifier: Arc<V>,
}

impl<B, V> Clone for ReleaseService<B, V> {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
            verifier: Arc::clone(&self.verifier),
        }
    }
}

impl<B, V> ReleaseService<B, V>
where
    B: TransactionalReleaseBackend,
    V: ReceiptVerifier,
{
    pub fn new(backend: Arc<B>, verifier: Arc<V>) -> Self {
        Self { backend, verifier }
    }

    /// Execute or reconcile one release. request_binding and now_ms must be
    /// supplied by trusted gateway/server code, not the untrusted caller.
    pub fn execute(
        &self,
        receipt: &AuthorizationReceipt,
        request_binding: &ReleaseBinding,
        now_ms: i64,
    ) -> ReleaseOutcome {
        if receipt.schema_version != 1 {
            return ReleaseOutcome::Denied("receipt_version_unsupported");
        }
        if !self.verifier.verify(receipt) {
            return ReleaseOutcome::Denied("receipt_signature_invalid");
        }
        if &receipt.binding != request_binding {
            return ReleaseOutcome::Denied("request_binding_mismatch");
        }
        if receipt.idempotency_key.is_empty() || receipt.nonce.is_empty() {
            return ReleaseOutcome::Denied("idempotency_or_nonce_missing");
        }
        if receipt.receipt_id.is_empty() || receipt.consent_id.is_empty() {
            return ReleaseOutcome::Denied("receipt_or_consent_identity_missing");
        }

        let intent = AtomicReleaseIntent {
            schema_version: receipt.schema_version,
            receipt_id: receipt.receipt_id.clone(),
            binding: receipt.binding.clone(),
            consent_id: receipt.consent_id.clone(),
            expected_consent_version: receipt.consent_version,
            expected_revocation_epoch: receipt.revocation_epoch,
            policy_version: receipt.policy_version.clone(),
            nonce: receipt.nonce.clone(),
            idempotency_key: receipt.idempotency_key.clone(),
            not_before_ms: receipt.not_before_ms,
            expires_at_ms: receipt.expires_at_ms,
            now_ms,
        };

        // A completed historical request is reconciled before checking receipt
        // expiry. This path never performs another side effect.
        match self.backend.lookup(&intent.idempotency_key) {
            StoredExecution::Completed {
                intent: stored,
                release_receipt_id,
            } => {
                return if same_transaction_except_clock(&stored, &intent) {
                    ReleaseOutcome::AlreadyReleased { release_receipt_id }
                } else {
                    ReleaseOutcome::Denied("idempotency_key_reused_for_different_request")
                };
            }
            StoredExecution::Pending { intent: stored } => {
                return if same_transaction_except_clock(&stored, &intent) {
                    ReleaseOutcome::Deferred("release_outcome_pending_reconciliation")
                } else {
                    ReleaseOutcome::Denied("idempotency_key_reused_for_different_request")
                };
            }
            StoredExecution::Unavailable => {
                return ReleaseOutcome::Deferred("idempotency_store_unavailable");
            }
            StoredExecution::NotFound => {}
        }

        if now_ms < receipt.not_before_ms {
            return ReleaseOutcome::Denied("receipt_not_yet_valid");
        }
        if now_ms > receipt.expires_at_ms {
            return ReleaseOutcome::Denied("receipt_expired");
        }
        if receipt.expires_at_ms <= receipt.not_before_ms {
            return ReleaseOutcome::Denied("receipt_validity_window_invalid");
        }

        match self.backend.commit_once(&intent) {
            AtomicCommitResult::Committed { release_receipt_id } => {
                ReleaseOutcome::Released { release_receipt_id }
            }
            AtomicCommitResult::AlreadyCommitted { release_receipt_id } => {
                ReleaseOutcome::AlreadyReleased { release_receipt_id }
            }
            AtomicCommitResult::Denied(reason) => ReleaseOutcome::Denied(reason),
            AtomicCommitResult::Deferred(reason) => ReleaseOutcome::Deferred(reason),
            AtomicCommitResult::OutcomeUnknown => {
                ReleaseOutcome::Deferred("commit_outcome_unknown_reconcile_same_key")
            }
        }
    }

    /// Query the same idempotency key after an unknown/lost acknowledgement.
    /// Reconciliation never creates a new release.
    pub fn reconcile(&self, receipt: &AuthorizationReceipt) -> ReleaseOutcome {
        if receipt.schema_version != 1 {
            return ReleaseOutcome::Denied("receipt_version_unsupported");
        }
        if !self.verifier.verify(receipt) {
            return ReleaseOutcome::Denied("receipt_signature_invalid");
        }
        match self.backend.lookup(&receipt.idempotency_key) {
            StoredExecution::Completed {
                intent,
                release_receipt_id,
            } => {
                let expected = AtomicReleaseIntent {
                    schema_version: receipt.schema_version,
                    receipt_id: receipt.receipt_id.clone(),
                    binding: receipt.binding.clone(),
                    consent_id: receipt.consent_id.clone(),
                    expected_consent_version: receipt.consent_version,
                    expected_revocation_epoch: receipt.revocation_epoch,
                    policy_version: receipt.policy_version.clone(),
                    nonce: receipt.nonce.clone(),
                    idempotency_key: receipt.idempotency_key.clone(),
                    not_before_ms: receipt.not_before_ms,
                    expires_at_ms: receipt.expires_at_ms,
                    now_ms: intent.now_ms,
                };
                if intent == expected {
                    ReleaseOutcome::AlreadyReleased { release_receipt_id }
                } else {
                    ReleaseOutcome::Denied("idempotency_key_reused_for_different_request")
                }
            }
            StoredExecution::Pending { .. } => {
                ReleaseOutcome::Deferred("release_outcome_pending_reconciliation")
            }
            StoredExecution::NotFound => {
                ReleaseOutcome::Deferred("no_committed_result_retry_only_with_same_key")
            }
            StoredExecution::Unavailable => {
                ReleaseOutcome::Deferred("idempotency_store_unavailable")
            }
        }
    }
}

fn same_transaction_except_clock(left: &AtomicReleaseIntent, right: &AtomicReleaseIntent) -> bool {
    let mut normalized_left = left.clone();
    let mut normalized_right = right.clone();
    normalized_left.now_ms = 0;
    normalized_right.now_ms = 0;
    normalized_left == normalized_right
}

/// Current authority state held by the atomic backend and compared inside the
/// same transaction as idempotency, audit-intent and release writes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentAuthority {
    pub consent_id: String,
    pub consent_version: u64,
    pub revocation_epoch: u64,
    pub revoked: bool,
    pub revocation_state_fresh: bool,
    pub policy_version: String,
    pub resource_version: String,
    pub resource_digest: String,
    pub audit_available: bool,
    pub idempotency_store_available: bool,
    pub atomic_commit_supported: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum IdempotencyStatus {
    Pending,
    Completed { release_receipt_id: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IdempotencyRecord {
    intent: AtomicReleaseIntent,
    status: IdempotencyStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditRecord {
    pub event: &'static str,
    pub receipt_id: String,
    pub idempotency_key: String,
    pub patient_id: String,
    pub tenant_id: String,
    pub actor_id: String,
    pub recipient_id: String,
    pub purpose: String,
    pub action: String,
    pub consent_id: String,
    pub consent_version: u64,
    pub clinical_event_time_ms: i64,
    pub encounter_id: Option<String>,
    pub resource_id: String,
    pub resource_version: String,
    pub resource_digest: String,
}

/// Mutex-backed reference backend for deterministic tests only.
pub struct InMemoryAtomicBackend {
    state: Mutex<BackendState>,
}

struct BackendState {
    current: CurrentAuthority,
    idempotency: HashMap<String, IdempotencyRecord>,
    consumed_nonces: HashMap<String, String>,
    audit: Vec<AuditRecord>,
    side_effect_count: usize,
    next_release_id: u64,
    lose_next_acknowledgement: bool,
}

impl InMemoryAtomicBackend {
    pub fn new(current: CurrentAuthority) -> Self {
        Self {
            state: Mutex::new(BackendState {
                current,
                idempotency: HashMap::new(),
                consumed_nonces: HashMap::new(),
                audit: Vec::new(),
                side_effect_count: 0,
                next_release_id: 1,
                lose_next_acknowledgement: false,
            }),
        }
    }

    pub fn set_current_authority(&self, current: CurrentAuthority) {
        let mut state = self.state.lock().expect("reference backend mutex poisoned");
        state.current = current;
    }

    pub fn lose_next_acknowledgement(&self) {
        let mut state = self.state.lock().expect("reference backend mutex poisoned");
        state.lose_next_acknowledgement = true;
    }

    pub fn side_effect_count(&self) -> usize {
        self.state
            .lock()
            .expect("reference backend mutex poisoned")
            .side_effect_count
    }

    pub fn audit_records(&self) -> Vec<AuditRecord> {
        self.state
            .lock()
            .expect("reference backend mutex poisoned")
            .audit
            .clone()
    }

    pub fn consumed_nonce_count(&self) -> usize {
        self.state
            .lock()
            .expect("reference backend mutex poisoned")
            .consumed_nonces
            .len()
    }
}

impl TransactionalReleaseBackend for InMemoryAtomicBackend {
    fn commit_once(&self, intent: &AtomicReleaseIntent) -> AtomicCommitResult {
        let mut state = self.state.lock().expect("reference backend mutex poisoned");

        if intent.schema_version != 1 {
            return AtomicCommitResult::Denied("receipt_version_unsupported");
        }
        if intent.receipt_id.is_empty()
            || intent.consent_id.is_empty()
            || intent.nonce.is_empty()
            || intent.idempotency_key.is_empty()
            || intent.expires_at_ms <= intent.not_before_ms
        {
            return AtomicCommitResult::Denied("release_intent_invalid");
        }
        if !state.current.idempotency_store_available {
            return AtomicCommitResult::Deferred("idempotency_store_unavailable");
        }
        if let Some(existing) = state.idempotency.get(&intent.idempotency_key) {
            if !same_transaction_except_clock(&existing.intent, intent) {
                return AtomicCommitResult::Denied("idempotency_key_reused_for_different_request");
            }
            return match &existing.status {
                IdempotencyStatus::Pending => {
                    AtomicCommitResult::Deferred("release_outcome_pending_reconciliation")
                }
                IdempotencyStatus::Completed { release_receipt_id } => {
                    AtomicCommitResult::AlreadyCommitted {
                        release_receipt_id: release_receipt_id.clone(),
                    }
                }
            };
        }

        if !state.current.atomic_commit_supported {
            return AtomicCommitResult::Deferred("atomic_commit_not_supported");
        }
        if !state.current.revocation_state_fresh {
            return AtomicCommitResult::Deferred("revocation_state_stale");
        }
        if state.current.revoked {
            return AtomicCommitResult::Denied("consent_revoked_before_commit");
        }
        if state.current.consent_id != intent.consent_id
            || state.current.consent_version != intent.expected_consent_version
            || state.current.revocation_epoch != intent.expected_revocation_epoch
            || state.current.policy_version != intent.policy_version
        {
            return AtomicCommitResult::Denied("authority_version_changed_before_commit");
        }
        if state.current.resource_version != intent.binding.resource_version
            || state.current.resource_digest != intent.binding.resource_digest
        {
            return AtomicCommitResult::Denied("resource_version_or_digest_changed");
        }
        if !state.current.audit_available {
            return AtomicCommitResult::Deferred("audit_intent_unavailable");
        }
        if intent.now_ms < intent.not_before_ms || intent.now_ms > intent.expires_at_ms {
            return AtomicCommitResult::Denied("receipt_outside_validity_window");
        }
        if state.consumed_nonces.contains_key(&intent.nonce) {
            return AtomicCommitResult::Denied("nonce_already_consumed");
        }

        // All mutations are atomic in this reference backend implementation.
        state.idempotency.insert(
            intent.idempotency_key.clone(),
            IdempotencyRecord {
                intent: intent.clone(),
                status: IdempotencyStatus::Pending,
            },
        );
        state
            .consumed_nonces
            .insert(intent.nonce.clone(), intent.idempotency_key.clone());
        state.audit.push(AuditRecord {
            event: "release_intent",
            receipt_id: intent.receipt_id.clone(),
            idempotency_key: intent.idempotency_key.clone(),
            resource_id: intent.binding.resource_id.clone(),
            resource_version: intent.binding.resource_version.clone(),
            patient_id: intent.binding.patient_id.clone(),
            tenant_id: intent.binding.tenant_id.clone(),
            actor_id: intent.binding.actor_id.clone(),
            recipient_id: intent.binding.recipient_id.clone(),
            purpose: intent.binding.purpose.clone(),
            action: intent.binding.action.clone(),
            consent_id: intent.consent_id.clone(),
            consent_version: intent.expected_consent_version,
            clinical_event_time_ms: intent.binding.clinical_event_time_ms,
            encounter_id: intent.binding.encounter_id.clone(),
            resource_digest: intent.binding.resource_digest.clone(),
        });

        state.side_effect_count += 1;
        let release_receipt_id = format!("release-{}", state.next_release_id);
        state.next_release_id += 1;
        state.audit.push(AuditRecord {
            event: "release_completed",
            receipt_id: intent.receipt_id.clone(),
            idempotency_key: intent.idempotency_key.clone(),
            resource_id: intent.binding.resource_id.clone(),
            resource_version: intent.binding.resource_version.clone(),
            patient_id: intent.binding.patient_id.clone(),
            tenant_id: intent.binding.tenant_id.clone(),
            actor_id: intent.binding.actor_id.clone(),
            recipient_id: intent.binding.recipient_id.clone(),
            purpose: intent.binding.purpose.clone(),
            action: intent.binding.action.clone(),
            consent_id: intent.consent_id.clone(),
            consent_version: intent.expected_consent_version,
            clinical_event_time_ms: intent.binding.clinical_event_time_ms,
            encounter_id: intent.binding.encounter_id.clone(),
            resource_digest: intent.binding.resource_digest.clone(),
        });
        if let Some(record) = state.idempotency.get_mut(&intent.idempotency_key) {
            record.status = IdempotencyStatus::Completed {
                release_receipt_id: release_receipt_id.clone(),
            };
        }

        if state.lose_next_acknowledgement {
            state.lose_next_acknowledgement = false;
            AtomicCommitResult::OutcomeUnknown
        } else {
            AtomicCommitResult::Committed { release_receipt_id }
        }
    }

    fn lookup(&self, idempotency_key: &str) -> StoredExecution {
        let state = self.state.lock().expect("reference backend mutex poisoned");
        if !state.current.idempotency_store_available {
            return StoredExecution::Unavailable;
        }
        match state.idempotency.get(idempotency_key) {
            None => StoredExecution::NotFound,
            Some(record) => match &record.status {
                IdempotencyStatus::Pending => StoredExecution::Pending {
                    intent: record.intent.clone(),
                },
                IdempotencyStatus::Completed { release_receipt_id } => StoredExecution::Completed {
                    intent: record.intent.clone(),
                    release_receipt_id: release_receipt_id.clone(),
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    struct TestVerifier {
        valid: bool,
    }

    impl ReceiptVerifier for TestVerifier {
        fn verify(&self, receipt: &AuthorizationReceipt) -> bool {
            self.valid && !receipt.signature.is_empty() && !receipt.signer_key_id.is_empty()
        }
    }

    fn binding() -> ReleaseBinding {
        ReleaseBinding {
            patient_id: "patient-1".into(),
            tenant_id: "tenant-1".into(),
            actor_id: "provider-1".into(),
            recipient_id: "clinic-1".into(),
            purpose: "treatment".into(),
            action: "read".into(),
            resource_id: "observation-1".into(),
            resource_version: "7".into(),
            resource_digest: "sha256:resource-v7".into(),
            clinical_event_time_ms: 1_795_000_000_000,
            encounter_id: Some("encounter-1".into()),
        }
    }

    fn receipt() -> AuthorizationReceipt {
        AuthorizationReceipt {
            schema_version: 1,
            receipt_id: "receipt-1".into(),
            signer_key_id: "key-1".into(),
            signature: vec![1, 2, 3],
            binding: binding(),
            consent_id: "consent-1".into(),
            consent_version: 4,
            revocation_epoch: 9,
            policy_version: "policy-v3".into(),
            nonce: "nonce-1".into(),
            idempotency_key: "request-1".into(),
            not_before_ms: 1_795_000_000_000,
            expires_at_ms: 1_795_000_060_000,
        }
    }

    fn current() -> CurrentAuthority {
        CurrentAuthority {
            consent_id: "consent-1".into(),
            consent_version: 4,
            revocation_epoch: 9,
            revoked: false,
            revocation_state_fresh: true,
            policy_version: "policy-v3".into(),
            resource_version: "7".into(),
            resource_digest: "sha256:resource-v7".into(),
            audit_available: true,
            idempotency_store_available: true,
            atomic_commit_supported: true,
        }
    }

    fn service(
        backend: Arc<InMemoryAtomicBackend>,
    ) -> ReleaseService<InMemoryAtomicBackend, TestVerifier> {
        ReleaseService::new(backend, Arc::new(TestVerifier { valid: true }))
    }

    #[test]
    fn valid_release_commits_once_with_audit_intent_and_outcome() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        let result =
            service(Arc::clone(&backend)).execute(&receipt(), &binding(), 1_795_000_010_000);
        assert!(matches!(result, ReleaseOutcome::Released { .. }));
        assert_eq!(backend.side_effect_count(), 1);
        assert_eq!(backend.consumed_nonce_count(), 1);
        let audit = backend.audit_records();
        assert_eq!(audit.len(), 2);
        assert_eq!(audit[0].event, "release_intent");
        assert_eq!(audit[1].event, "release_completed");
        assert_eq!(audit[0].resource_digest, "sha256:resource-v7");
        assert_eq!(audit[0].patient_id, "patient-1");
        assert_eq!(audit[0].tenant_id, "tenant-1");
        assert_eq!(audit[0].actor_id, "provider-1");
        assert_eq!(audit[0].recipient_id, "clinic-1");
        assert_eq!(audit[0].purpose, "treatment");
        assert_eq!(audit[0].action, "read");
        assert_eq!(audit[0].consent_version, 4);
        assert_eq!(audit[0].clinical_event_time_ms, 1_795_000_000_000);
        assert_eq!(audit[0].encounter_id.as_deref(), Some("encounter-1"));
    }

    #[test]
    fn invalid_signature_never_reaches_the_backend() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        let svc = ReleaseService::new(
            Arc::clone(&backend),
            Arc::new(TestVerifier { valid: false }),
        );
        assert_eq!(
            svc.execute(&receipt(), &binding(), 1_795_000_010_000),
            ReleaseOutcome::Denied("receipt_signature_invalid")
        );
        assert_eq!(backend.side_effect_count(), 0);
    }

    #[test]
    fn unsupported_receipt_version_is_denied() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        let mut altered = receipt();
        altered.schema_version = 2;
        assert_eq!(
            service(Arc::clone(&backend)).execute(&altered, &binding(), 1_795_000_010_000),
            ReleaseOutcome::Denied("receipt_version_unsupported")
        );
        assert_eq!(backend.side_effect_count(), 0);
    }

    #[test]
    fn every_request_binding_dimension_is_immutable() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        let svc = service(Arc::clone(&backend));
        let mutations: Vec<Box<dyn Fn(&mut ReleaseBinding)>> = vec![
            Box::new(|b| b.patient_id.push_str("-other")),
            Box::new(|b| b.tenant_id.push_str("-other")),
            Box::new(|b| b.actor_id.push_str("-other")),
            Box::new(|b| b.recipient_id.push_str("-other")),
            Box::new(|b| b.purpose.push_str("-other")),
            Box::new(|b| b.action.push_str("-other")),
            Box::new(|b| b.resource_id.push_str("-other")),
            Box::new(|b| b.resource_version.push_str("-other")),
            Box::new(|b| b.resource_digest.push_str("-other")),
            Box::new(|b| b.clinical_event_time_ms += 1),
            Box::new(|b| b.encounter_id = None),
        ];
        for mutate in mutations {
            let mut changed = binding();
            mutate(&mut changed);
            assert_eq!(
                svc.execute(&receipt(), &changed, 1_795_000_010_000),
                ReleaseOutcome::Denied("request_binding_mismatch")
            );
        }
        assert_eq!(backend.side_effect_count(), 0);
    }

    #[test]
    fn expired_and_not_yet_valid_receipts_are_denied() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        let svc = service(Arc::clone(&backend));
        assert_eq!(
            svc.execute(&receipt(), &binding(), 1_795_000_000_000 - 1),
            ReleaseOutcome::Denied("receipt_not_yet_valid")
        );
        assert_eq!(
            svc.execute(&receipt(), &binding(), 1_795_000_060_001),
            ReleaseOutcome::Denied("receipt_expired")
        );
        assert_eq!(backend.side_effect_count(), 0);
    }

    #[test]
    fn consent_revoked_after_prepare_is_denied_at_atomic_commit() {
        let mut revoked = current();
        revoked.revoked = true;
        let backend = Arc::new(InMemoryAtomicBackend::new(revoked));
        assert_eq!(
            service(Arc::clone(&backend)).execute(&receipt(), &binding(), 1_795_000_010_000),
            ReleaseOutcome::Denied("consent_revoked_before_commit")
        );
        assert_eq!(backend.side_effect_count(), 0);
        assert!(backend.audit_records().is_empty());
    }

    #[test]
    fn stale_revocation_state_defers_without_release() {
        let mut stale = current();
        stale.revocation_state_fresh = false;
        let backend = Arc::new(InMemoryAtomicBackend::new(stale));
        assert_eq!(
            service(Arc::clone(&backend)).execute(&receipt(), &binding(), 1_795_000_010_000),
            ReleaseOutcome::Deferred("revocation_state_stale")
        );
        assert_eq!(backend.side_effect_count(), 0);
    }

    #[test]
    fn consent_or_policy_version_change_is_denied() {
        let mut changed = current();
        changed.consent_version += 1;
        changed.policy_version = "policy-v4".into();
        let backend = Arc::new(InMemoryAtomicBackend::new(changed));
        assert_eq!(
            service(Arc::clone(&backend)).execute(&receipt(), &binding(), 1_795_000_010_000),
            ReleaseOutcome::Denied("authority_version_changed_before_commit")
        );
        assert_eq!(backend.side_effect_count(), 0);
    }

    #[test]
    fn resource_version_or_digest_change_is_denied() {
        let mut changed = current();
        changed.resource_version = "8".into();
        let backend = Arc::new(InMemoryAtomicBackend::new(changed));
        assert_eq!(
            service(Arc::clone(&backend)).execute(&receipt(), &binding(), 1_795_000_010_000),
            ReleaseOutcome::Denied("resource_version_or_digest_changed")
        );
        assert_eq!(backend.side_effect_count(), 0);
    }

    #[test]
    fn audit_and_idempotency_store_outages_defer_without_release() {
        let mut audit_down = current();
        audit_down.audit_available = false;
        let backend = Arc::new(InMemoryAtomicBackend::new(audit_down));
        assert_eq!(
            service(Arc::clone(&backend)).execute(&receipt(), &binding(), 1_795_000_010_000),
            ReleaseOutcome::Deferred("audit_intent_unavailable")
        );
        assert_eq!(backend.side_effect_count(), 0);

        let mut replay_down = current();
        replay_down.idempotency_store_available = false;
        let backend = Arc::new(InMemoryAtomicBackend::new(replay_down));
        assert_eq!(
            service(Arc::clone(&backend)).execute(&receipt(), &binding(), 1_795_000_010_000),
            ReleaseOutcome::Deferred("idempotency_store_unavailable")
        );
        assert_eq!(backend.side_effect_count(), 0);
    }

    #[test]
    fn backend_without_atomic_commit_capability_defers() {
        let mut current = current();
        current.atomic_commit_supported = false;
        let backend = Arc::new(InMemoryAtomicBackend::new(current));
        assert_eq!(
            service(Arc::clone(&backend)).execute(&receipt(), &binding(), 1_795_000_010_000),
            ReleaseOutcome::Deferred("atomic_commit_not_supported")
        );
        assert_eq!(backend.side_effect_count(), 0);
    }

    #[test]
    fn exact_replay_returns_original_receipt_without_second_side_effect() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        let svc = service(Arc::clone(&backend));
        let original = svc.execute(&receipt(), &binding(), 1_795_000_010_000);
        let replay = svc.execute(&receipt(), &binding(), 1_795_000_020_000);
        assert!(matches!(original, ReleaseOutcome::Released { .. }));
        assert!(matches!(replay, ReleaseOutcome::AlreadyReleased { .. }));
        assert_eq!(backend.side_effect_count(), 1);
    }

    #[test]
    fn completed_release_reconciles_even_after_original_receipt_expires() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        let svc = service(Arc::clone(&backend));
        assert!(matches!(
            svc.execute(&receipt(), &binding(), 1_795_000_010_000),
            ReleaseOutcome::Released { .. }
        ));
        assert!(matches!(
            svc.execute(&receipt(), &binding(), 1_795_000_060_001),
            ReleaseOutcome::AlreadyReleased { .. }
        ));
        assert_eq!(backend.side_effect_count(), 1);
    }

    #[test]
    fn idempotency_key_cannot_be_reused_for_another_receipt_or_request() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        let svc = service(Arc::clone(&backend));
        assert!(matches!(
            svc.execute(&receipt(), &binding(), 1_795_000_010_000),
            ReleaseOutcome::Released { .. }
        ));
        let mut altered = receipt();
        altered.receipt_id = "receipt-other".into();
        assert_eq!(
            svc.execute(&altered, &binding(), 1_795_000_020_000),
            ReleaseOutcome::Denied("idempotency_key_reused_for_different_request")
        );
        assert_eq!(backend.side_effect_count(), 1);
    }

    #[test]
    fn revocation_epoch_change_blocks_a_previously_issued_receipt() {
        let mut changed = current();
        changed.revocation_epoch += 1;
        let backend = Arc::new(InMemoryAtomicBackend::new(changed));
        assert_eq!(
            service(Arc::clone(&backend)).execute(&receipt(), &binding(), 1_795_000_010_000),
            ReleaseOutcome::Denied("authority_version_changed_before_commit")
        );
        assert_eq!(backend.side_effect_count(), 0);
    }

    #[test]
    fn same_nonce_under_a_different_request_cannot_release() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        let svc = service(Arc::clone(&backend));
        assert!(matches!(
            svc.execute(&receipt(), &binding(), 1_795_000_010_000),
            ReleaseOutcome::Released { .. }
        ));
        let mut second = receipt();
        second.idempotency_key = "request-2".into();
        second.receipt_id = "receipt-2".into();
        assert_eq!(
            svc.execute(&second, &binding(), 1_795_000_020_000),
            ReleaseOutcome::Denied("nonce_already_consumed")
        );
        assert_eq!(backend.side_effect_count(), 1);
    }

    #[test]
    fn lost_acknowledgement_reconciles_without_releasing_again() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        backend.lose_next_acknowledgement();
        let svc = service(Arc::clone(&backend));
        assert_eq!(
            svc.execute(&receipt(), &binding(), 1_795_000_010_000),
            ReleaseOutcome::Deferred("commit_outcome_unknown_reconcile_same_key")
        );
        assert_eq!(backend.side_effect_count(), 1);
        assert!(matches!(
            svc.reconcile(&receipt()),
            ReleaseOutcome::AlreadyReleased { .. }
        ));
        assert_eq!(backend.side_effect_count(), 1);
        assert!(matches!(
            svc.execute(&receipt(), &binding(), 1_795_000_020_000),
            ReleaseOutcome::AlreadyReleased { .. }
        ));
        assert_eq!(backend.side_effect_count(), 1);
    }

    #[test]
    fn unavailable_reconciliation_never_creates_a_new_release() {
        let mut current = current();
        current.idempotency_store_available = false;
        let backend = Arc::new(InMemoryAtomicBackend::new(current));
        let svc = service(Arc::clone(&backend));
        assert_eq!(
            svc.reconcile(&receipt()),
            ReleaseOutcome::Deferred("idempotency_store_unavailable")
        );
        assert_eq!(backend.side_effect_count(), 0);
    }

    #[test]
    fn concurrent_duplicate_attempts_commit_at_most_once() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        let svc = Arc::new(service(Arc::clone(&backend)));
        let auth = receipt();
        let request = binding();
        let handles: Vec<_> = (0..32)
            .map(|_| {
                let svc = Arc::clone(&svc);
                let auth = auth.clone();
                let request = request.clone();
                thread::spawn(move || svc.execute(&auth, &request, 1_795_000_010_000))
            })
            .collect();

        let outcomes: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().expect("worker succeeds"))
            .collect();
        let released = outcomes
            .iter()
            .filter(|o| matches!(o, ReleaseOutcome::Released { .. }))
            .count();
        let already = outcomes
            .iter()
            .filter(|o| matches!(o, ReleaseOutcome::AlreadyReleased { .. }))
            .count();
        assert_eq!(released, 1);
        assert_eq!(already, 31);
        assert_eq!(backend.side_effect_count(), 1);
        assert_eq!(backend.audit_records().len(), 2);
    }

    #[test]
    fn reconciliation_of_missing_result_defers_instead_of_inventing_completion() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        assert_eq!(
            service(Arc::clone(&backend)).reconcile(&receipt()),
            ReleaseOutcome::Deferred("no_committed_result_retry_only_with_same_key")
        );
        assert_eq!(backend.side_effect_count(), 0);
    }

    #[test]
    fn audit_records_bind_the_exact_resource_version_and_digest() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        let _ = service(Arc::clone(&backend)).execute(&receipt(), &binding(), 1_795_000_010_000);
        let audit = backend.audit_records();
        assert!(audit.iter().all(|entry| entry.resource_version == "7"));
        assert!(audit
            .iter()
            .all(|entry| entry.resource_digest == "sha256:resource-v7"));
    }

    #[test]
    fn concurrent_distinct_idempotency_keys_cannot_consume_same_nonce_twice() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        let svc = Arc::new(service(Arc::clone(&backend)));
        let barrier = Arc::new(std::sync::Barrier::new(3));

        let mut second = receipt();
        second.receipt_id = "receipt-2".into();
        second.idempotency_key = "request-2".into();
        second.signature = vec![4, 5, 6];

        let attempts = [receipt(), second]
            .into_iter()
            .map(|authorization| {
                let svc = Arc::clone(&svc);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    svc.execute(&authorization, &binding(), 1_795_000_010_000)
                })
            })
            .collect::<Vec<_>>();

        barrier.wait();
        let outcomes = attempts
            .into_iter()
            .map(|attempt| attempt.join().expect("release worker succeeds"))
            .collect::<Vec<_>>();

        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(outcome, ReleaseOutcome::Released { .. }))
                .count(),
            1
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| {
                    **outcome == ReleaseOutcome::Denied("nonce_already_consumed")
                })
                .count(),
            1
        );
        assert_eq!(backend.side_effect_count(), 1);
        assert_eq!(backend.consumed_nonce_count(), 1);
        assert_eq!(backend.audit_records().len(), 2);
    }

    #[test]
    fn consent_revocation_race_is_linearizable_at_atomic_commit() {
        let backend = Arc::new(InMemoryAtomicBackend::new(current()));
        let svc = service(Arc::clone(&backend));
        let release_service = svc.clone();
        let release_backend = Arc::clone(&backend);
        let barrier = Arc::new(std::sync::Barrier::new(3));

        let release_barrier = Arc::clone(&barrier);
        let release = thread::spawn(move || {
            release_barrier.wait();
            release_service.execute(&receipt(), &binding(), 1_795_000_010_000)
        });

        let revoke_barrier = Arc::clone(&barrier);
        let revoke = thread::spawn(move || {
            let mut revoked = current();
            revoked.revoked = true;
            revoke_barrier.wait();
            release_backend.set_current_authority(revoked);
        });

        barrier.wait();
        let outcome = release.join().expect("release worker succeeds");
        revoke.join().expect("revocation worker succeeds");

        let committed_before_revocation =
            matches!(outcome, ReleaseOutcome::Released { .. });
        assert!(
            committed_before_revocation
                || outcome == ReleaseOutcome::Denied("consent_revoked_before_commit")
        );
        assert_eq!(
            backend.side_effect_count(),
            usize::from(committed_before_revocation)
        );
        assert_eq!(
            backend.audit_records().len(),
            2 * usize::from(committed_before_revocation)
        );
        assert_eq!(
            backend.consumed_nonce_count(),
            usize::from(committed_before_revocation)
        );

        // Once revocation has linearized, a distinct later request must not
        // reuse the old authority snapshot or create another side effect.
        let mut after_revocation = receipt();
        after_revocation.receipt_id = "receipt-after-revocation".into();
        after_revocation.idempotency_key = "request-after-revocation".into();
        after_revocation.nonce = "nonce-after-revocation".into();
        after_revocation.signature = vec![7, 8, 9];
        assert_eq!(
            svc.execute(&after_revocation, &binding(), 1_795_000_020_000),
            ReleaseOutcome::Denied("consent_revoked_before_commit")
        );
        assert_eq!(
            backend.side_effect_count(),
            usize::from(committed_before_revocation)
        );
    }
}
