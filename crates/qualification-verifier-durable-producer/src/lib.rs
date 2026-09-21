#![forbid(unsafe_code)]
//! Durable-freshness composition for the isolated qualification verifier mint producer.
//!
//! This crate composes two lower proof lines without weakening either one:
//!
//! - QUAL-EVID-009C1 / #232: process-local producer, hybrid signing, provider attestation;
//! - QUAL-EVID-009C2 / #234: restart-durable challenge/time/policy state.
//!
//! The public wrapper never exposes the raw in-memory producer or the durable store.
//! A mint attempt must consume the durable challenge first; only a fresh durable
//! consumption can invoke the lower producer.

use mycelix_qualification_verifier_freshness_store as freshness;
use mycelix_qualification_verifier_mint_contract as contract;
use mycelix_qualification_verifier_mint_producer as producer;

/// A challenge returned only after the exact producer-generated nonce/window
/// has been committed to the durable freshness store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurablyIssuedVerifierChallengeV1 {
    challenge: contract::Nonce,
    issued_at_micros: i64,
    expires_at_micros: i64,
    durable_head: freshness::FreshnessStoreHeadV1,
}

impl DurablyIssuedVerifierChallengeV1 {
    pub fn challenge(&self) -> contract::Nonce {
        self.challenge
    }

    pub fn issued_at_micros(&self) -> i64 {
        self.issued_at_micros
    }

    pub fn expires_at_micros(&self) -> i64 {
        self.expires_at_micros
    }

    pub fn durable_head(&self) -> freshness::FreshnessStoreHeadV1 {
        self.durable_head
    }
}

/// Product/daemon-facing composition of the lower producer and durable store.
///
/// There is intentionally no accessor for either inner object. Selecting this
/// wrapper means durable freshness is mandatory for every issued challenge and
/// every mint attempt.
pub struct DurableQualificationVerifierMintProducerV1 {
    producer: producer::QualificationVerifierMintProducerV1,
    freshness: freshness::DurableFreshnessStoreV1,
    expected_policy_generation: u64,
    expected_key_lineage_commitment: contract::Digest,
    poisoned: bool,
}

impl DurableQualificationVerifierMintProducerV1 {
    /// Compose one already-created durable freshness store with one exact
    /// software verifier identity.
    ///
    /// The store's current policy generation and hybrid key-lineage commitment
    /// must match the signer before the lower producer is constructed.
    pub fn new(
        freshness: freshness::DurableFreshnessStoreV1,
        identity: producer::SoftwareVerifierIdentityV1,
        max_challenge_lifetime_micros: i64,
        max_receipt_lifetime_micros: i64,
    ) -> Result<Self, DurableProducerError> {
        let expected_policy_generation = identity.policy_generation();
        let expected_key_lineage_commitment = identity
            .key_lineage_commitment()
            .map_err(DurableProducerError::Producer)?;

        require_head_alignment(
            freshness.head(),
            expected_policy_generation,
            expected_key_lineage_commitment,
        )?;

        let producer = producer::QualificationVerifierMintProducerV1::new(
            identity,
            max_challenge_lifetime_micros,
            max_receipt_lifetime_micros,
        )
        .map_err(DurableProducerError::Producer)?;

        Ok(Self {
            producer,
            freshness,
            expected_policy_generation,
            expected_key_lineage_commitment,
            poisoned: false,
        })
    }

    /// Return the opaque durable-store head intended for later external
    /// monotonic witnessing. This does not itself prove external currentness.
    pub fn freshness_head(&self) -> freshness::FreshnessStoreHeadV1 {
        self.freshness.head()
    }

    /// Issue a producer-generated challenge and persist the exact nonce/window
    /// before returning it.
    ///
    /// If the lower producer issues successfully but durable persistence fails,
    /// this wrapper enters a fail-stop poisoned state. The unpersisted process-
    /// local challenge is never returned through this API.
    pub fn issue_challenge(
        &mut self,
        now_micros: i64,
        requested_lifetime_micros: i64,
    ) -> Result<DurablyIssuedVerifierChallengeV1, DurableProducerError> {
        self.require_ready_and_aligned()?;
        self.require_not_before_durable_time_floor(now_micros)?;

        let issued = self
            .producer
            .issue_challenge(now_micros, requested_lifetime_micros)
            .map_err(DurableProducerError::Producer)?;

        let durable_head = match self.freshness.issue_challenge(
            issued.challenge(),
            issued.issued_at_micros(),
            issued.expires_at_micros(),
        ) {
            Ok(head) => head,
            Err(error) => {
                // The lower producer now contains freshness state that the
                // durable store does not. Never continue from this process.
                self.poisoned = true;
                return Err(DurableProducerError::Store(error));
            }
        };

        Ok(DurablyIssuedVerifierChallengeV1 {
            challenge: issued.challenge(),
            issued_at_micros: issued.issued_at_micros(),
            expires_at_micros: issued.expires_at_micros(),
            durable_head,
        })
    }

    /// Durably consume the exact request challenge before invoking the real
    /// process-local producer.
    ///
    /// The evidence type remains the private-constructor #232 type. This child
    /// does not invent a new way for application code to manufacture evidence.
    pub fn mint(
        &mut self,
        request: contract::VerifierMintRequestV1,
        evidence: producer::VerifierOwnedMintEvidenceV1,
        now_micros: i64,
    ) -> Result<producer::ProviderAuthenticatedMintReceiptV1, DurableProducerError> {
        let challenge = request.verifier_challenge();
        self.with_durable_challenge(challenge, now_micros, move |inner| {
            inner.mint(request, evidence, now_micros)
        })
    }

    fn with_durable_challenge<T, F>(
        &mut self,
        challenge: contract::Nonce,
        now_micros: i64,
        delegate: F,
    ) -> Result<T, DurableProducerError>
    where
        F: FnOnce(
            &mut producer::QualificationVerifierMintProducerV1,
        ) -> Result<T, producer::ProducerError>,
    {
        self.require_ready_and_aligned()?;
        self.require_not_before_durable_time_floor(now_micros)?;

        // Security-critical ordering: #234 persists the consume event before
        // any request/evidence verification or signing can occur below.
        let status = self
            .freshness
            .consume_challenge(challenge, now_micros)
            .map_err(DurableProducerError::Store)?;

        if status != freshness::ChallengeConsumptionStatusV1::Fresh {
            return Err(DurableProducerError::DurableChallengeExpired);
        }

        delegate(&mut self.producer).map_err(DurableProducerError::Producer)
    }

    fn require_ready_and_aligned(&self) -> Result<(), DurableProducerError> {
        if self.poisoned {
            return Err(DurableProducerError::PoisonedAfterPartialIssue);
        }
        require_head_alignment(
            self.freshness.head(),
            self.expected_policy_generation,
            self.expected_key_lineage_commitment,
        )
    }

    fn require_not_before_durable_time_floor(
        &self,
        now_micros: i64,
    ) -> Result<(), DurableProducerError> {
        if now_micros < self.freshness.head().security_time_floor_micros() {
            return Err(DurableProducerError::TimeRollbackAgainstDurableStore);
        }
        Ok(())
    }
}

fn require_head_alignment(
    head: freshness::FreshnessStoreHeadV1,
    expected_policy_generation: u64,
    expected_key_lineage_commitment: contract::Digest,
) -> Result<(), DurableProducerError> {
    if head.policy_generation() != expected_policy_generation {
        return Err(DurableProducerError::PolicyGenerationMismatch);
    }
    if head.key_lineage_commitment() != expected_key_lineage_commitment {
        return Err(DurableProducerError::KeyLineageMismatch);
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DurableProducerError {
    #[error("durable freshness-store policy generation does not match producer identity")]
    PolicyGenerationMismatch,
    #[error("durable freshness-store key lineage does not match producer identity")]
    KeyLineageMismatch,
    #[error("observed time is older than the durable freshness-store time floor")]
    TimeRollbackAgainstDurableStore,
    #[error("durable challenge was expired and has been consumed")]
    DurableChallengeExpired,
    #[error("wrapper is fail-stop poisoned after process-local challenge issue without durable persistence")]
    PoisonedAfterPartialIssue,
    #[error("lower mint producer rejected operation: {0}")]
    Producer(#[source] producer::ProducerError),
    #[error("durable freshness store rejected operation: {0}")]
    Store(#[source] freshness::FreshnessStoreError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(1);

    fn d(value: u8) -> contract::Digest {
        contract::Digest::new([value; 32]).expect("nonzero digest")
    }

    fn identity(seed: u8, generation: u64) -> producer::SoftwareVerifierIdentityV1 {
        producer::SoftwareVerifierIdentityV1::from_seeds(
            "durable-health-verifier".to_string(),
            d(40),
            d(41),
            generation,
            [seed; 32],
            [seed.wrapping_add(1); 32],
        )
        .expect("valid identity")
    }

    fn test_path(name: &str) -> PathBuf {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "mycelix-durable-producer-{name}-{}-{id}.bin",
            std::process::id()
        ))
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_file(path);
        if let Some(parent) = path.parent() {
            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                for sequence in 1..64 {
                    let _ = fs::remove_file(parent.join(format!(".{file_name}.tmp.{sequence}")));
                }
            }
        }
    }

    fn wrapper(path: &Path) -> DurableQualificationVerifierMintProducerV1 {
        let identity = identity(11, 1);
        let lineage = identity.key_lineage_commitment().expect("lineage");
        let store = freshness::DurableFreshnessStoreV1::create(
            path,
            d(1),
            1_000,
            identity.policy_generation(),
            lineage,
        )
        .expect("store");
        DurableQualificationVerifierMintProducerV1::new(store, identity, 500, 500)
            .expect("wrapper")
    }

    #[test]
    fn returned_challenge_is_already_durable() {
        let path = test_path("issued-durable");
        cleanup(&path);
        let mut composed = wrapper(&path);
        let issued = composed.issue_challenge(1_010, 100).expect("issue");
        assert!(issued.durable_head().sequence() > 1);
        let nonce = issued.challenge();
        drop(composed);

        let loaded = freshness::DurableFreshnessStoreV1::load(&path).expect("reload");
        assert!(loaded.is_challenge_active(nonce));
        cleanup(&path);
    }

    #[test]
    fn durable_consume_occurs_before_delegate() {
        let path = test_path("consume-before-delegate");
        cleanup(&path);
        let mut composed = wrapper(&path);
        let issued = composed.issue_challenge(1_010, 100).expect("issue");
        let nonce = issued.challenge();
        let mut delegated = false;

        let value = composed
            .with_durable_challenge(nonce, 1_020, |_producer| {
                delegated = true;
                Ok::<_, producer::ProducerError>(42u64)
            })
            .expect("delegate");

        assert_eq!(value, 42);
        assert!(delegated);
        assert!(composed.freshness.is_challenge_consumed(nonce));
        cleanup(&path);
    }

    #[test]
    fn delegate_failure_does_not_restore_consumed_challenge() {
        let path = test_path("delegate-failure");
        cleanup(&path);
        let mut composed = wrapper(&path);
        let nonce = composed
            .issue_challenge(1_010, 100)
            .expect("issue")
            .challenge();

        assert!(matches!(
            composed.with_durable_challenge(nonce, 1_020, |_producer| {
                Err::<(), _>(producer::ProducerError::RequestNotCurrent)
            }),
            Err(DurableProducerError::Producer(
                producer::ProducerError::RequestNotCurrent
            ))
        ));
        assert!(composed.freshness.is_challenge_consumed(nonce));
        cleanup(&path);
    }

    #[test]
    fn expired_durable_challenge_is_burned_without_delegate() {
        let path = test_path("expired");
        cleanup(&path);
        let mut composed = wrapper(&path);
        let nonce = composed
            .issue_challenge(1_010, 10)
            .expect("issue")
            .challenge();
        let mut delegated = false;

        assert!(matches!(
            composed.with_durable_challenge(nonce, 1_021, |_producer| {
                delegated = true;
                Ok::<_, producer::ProducerError>(())
            }),
            Err(DurableProducerError::DurableChallengeExpired)
        ));
        assert!(!delegated);
        assert!(composed.freshness.is_challenge_consumed(nonce));
        cleanup(&path);
    }

    #[test]
    fn policy_generation_mismatch_denies_construction() {
        let path = test_path("policy-mismatch");
        cleanup(&path);
        let original = identity(11, 1);
        let original_lineage = original.key_lineage_commitment().expect("lineage");
        let store = freshness::DurableFreshnessStoreV1::create(
            &path,
            d(1),
            1_000,
            1,
            original_lineage,
        )
        .expect("store");
        let replacement = identity(21, 2);

        assert!(matches!(
            DurableQualificationVerifierMintProducerV1::new(store, replacement, 500, 500),
            Err(DurableProducerError::PolicyGenerationMismatch)
        ));
        cleanup(&path);
    }

    #[test]
    fn key_lineage_mismatch_denies_construction() {
        let path = test_path("lineage-mismatch");
        cleanup(&path);
        let original = identity(11, 1);
        let original_lineage = original.key_lineage_commitment().expect("lineage");
        let store = freshness::DurableFreshnessStoreV1::create(
            &path,
            d(1),
            1_000,
            1,
            original_lineage,
        )
        .expect("store");
        let replacement = identity(21, 1);

        assert!(matches!(
            DurableQualificationVerifierMintProducerV1::new(store, replacement, 500, 500),
            Err(DurableProducerError::KeyLineageMismatch)
        ));
        cleanup(&path);
    }

    #[test]
    fn durable_time_floor_is_checked_before_process_local_issue() {
        let path = test_path("time-floor");
        cleanup(&path);
        let mut composed = wrapper(&path);
        composed.freshness.advance_time(1_500).expect("advance");

        assert!(matches!(
            composed.issue_challenge(1_499, 10),
            Err(DurableProducerError::TimeRollbackAgainstDurableStore)
        ));
        cleanup(&path);
    }

    #[test]
    fn restart_does_not_reconstruct_process_local_authority() {
        let path = test_path("restart-profile");
        cleanup(&path);
        let mut first = wrapper(&path);
        let nonce = first
            .issue_challenge(1_010, 100)
            .expect("issue")
            .challenge();
        drop(first);

        let store = freshness::DurableFreshnessStoreV1::load(&path).expect("load");
        assert!(store.is_challenge_active(nonce));
        let replacement_identity = identity(11, 1);
        let second = DurableQualificationVerifierMintProducerV1::new(
            store,
            replacement_identity,
            500,
            500,
        )
        .expect("restart wrapper");

        // The durable challenge remains represented, but constructor creates a
        // fresh lower producer rather than rehydrating its private nonce map.
        // A real successful mint cannot be fabricated in this child because
        // VerifierOwnedMintEvidenceV1 has no public constructor.
        assert_eq!(second.freshness_head().policy_generation(), 1);
        cleanup(&path);
    }
}
