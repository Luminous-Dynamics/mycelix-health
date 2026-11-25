#![forbid(unsafe_code)]

//! Versioned, answer-free contract between Mycelix Health evidence custody
//! and Symthaea's candidate-visible finite-world episode API.
//!
//! This crate does not implement cryptography, SMART/OIDC, consent registries,
//! or production identity verification. The injected verifier must verify
//! signed receipts against configured trust roots and atomically consume
//! replay nonces. Tests use a fake verifier only to qualify the policy seam.
//!
//! The operator-side evidence-reference manifest is deliberately separate
//! from the candidate episode. Never send that manifest, source payloads,
//! hidden evaluator state, priors, likelihoods, or loss tables to the candidate.

use std::collections::{BTreeMap, BTreeSet};

pub const MAX_RECEIPT_TTL_MS: i64 = 300_000;
pub const REQUIRED_GATE_COUNT: usize = 9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureAlgorithmV1 {
    Ed25519,
    Es256,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceDimensionV1 {
    EvidencePresence,
    SubjectBinding,
    IssuerAuthenticity,
    ResourceVersion,
    TemporalValidity,
    SemanticSupport,
    ProvenanceLineage,
    ApplicationAuthority,
    ConflictReconciliation,
}

pub const REQUIRED_EVIDENCE_DIMENSIONS_V1: [EvidenceDimensionV1; REQUIRED_GATE_COUNT] = [
    EvidenceDimensionV1::EvidencePresence,
    EvidenceDimensionV1::SubjectBinding,
    EvidenceDimensionV1::IssuerAuthenticity,
    EvidenceDimensionV1::ResourceVersion,
    EvidenceDimensionV1::TemporalValidity,
    EvidenceDimensionV1::SemanticSupport,
    EvidenceDimensionV1::ProvenanceLineage,
    EvidenceDimensionV1::ApplicationAuthority,
    EvidenceDimensionV1::ConflictReconciliation,
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceGateAttestationV1 {
    pub dimension: EvidenceDimensionV1,
    /// Digest of the independently produced gate attestation, not a caller flag.
    pub attestation_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceReleaseReceiptV1 {
    pub receipt_id: String,
    pub nonce: String,
    pub episode_id: String,
    pub tenant_id: String,
    /// Opaque registered binding. Never put a raw patient identifier here.
    pub subject_binding: String,
    pub purpose: String,
    pub recipient_id: String,
    pub source_connection_id: String,
    pub source_principal: String,
    pub event_id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub resource_version: String,
    pub payload_digest: String,
    pub source_receipt_digest: String,
    pub channel_id: String,
    pub outcome_id: String,
    /// Source-authenticated clinical event time; not inferred from arrival.
    pub clinical_event_time_ms: i64,
    pub specimen_collected_at_ms: Option<i64>,
    pub analysis_performed_at_ms: Option<i64>,
    pub result_issued_at_ms: Option<i64>,
    pub source_last_updated_at_ms: Option<i64>,
    pub source_sequence: Option<u64>,
    pub ingested_at_ms: i64,
    pub issued_at_ms: i64,
    pub expires_at_ms: i64,
    pub gates: Vec<EvidenceGateAttestationV1>,
    pub signing_key_id: String,
    pub signature_algorithm: SignatureAlgorithmV1,
    /// Unpadded base64url detached signature over the canonical unsigned receipt.
    pub signature: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProbeOfferReceiptV1 {
    pub receipt_id: String,
    pub nonce: String,
    pub episode_id: String,
    pub tenant_id: String,
    pub subject_binding: String,
    pub purpose: String,
    pub recipient_id: String,
    pub probe_id: String,
    pub action_id: String,
    pub allowed_resource_types: Vec<String>,
    pub acquisition_cost: f64,
    pub result_delay_steps: u64,
    pub issued_at_ms: i64,
    pub expires_at_ms: i64,
    pub authority_receipt_digest: String,
    pub signing_key_id: String,
    pub signature_algorithm: SignatureAlgorithmV1,
    /// Unpadded base64url detached signature over the canonical unsigned receipt.
    pub signature: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifierFailureV1 {
    Rejected,
    ReplayDetected,
    Unavailable,
}

pub trait EvidenceReceiptVerifierV1 {
    /// Verify the detached signature over the RFC 8785 canonical JSON payload
    /// (all receipt fields except signature; signing_key_id and algorithm are
    /// included), bind all nine gate attestations to the same evidence/subject/
    /// scope, check revocation, and atomically consume the nonce. A digest alone
    /// is never proof.
    fn verify_and_consume_evidence(
        &self,
        receipt: &EvidenceReleaseReceiptV1,
    ) -> Result<(), VerifierFailureV1>;

    /// Verify the detached signature over the canonical receipt, then verify
    /// current source/consent/recipient/action/resource authority. This only
    /// permits the option to be shown to the candidate. Execution requires a fresh
    /// independent authorization check after candidate return.
    fn verify_probe_offer(
        &self,
        receipt: &ProbeOfferReceiptV1,
    ) -> Result<(), VerifierFailureV1>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeErrorV1 {
    EmptyField(&'static str),
    InvalidNonce,
    InvalidDigest(&'static str),
    InvalidSignature,
    InvalidReceiptWindow,
    InvalidAsOfTime,
    ReceiptExpired,
    ReceiptNotYetValid,
    ReceiptTtlExceeded,
    MissingEvidenceGate(EvidenceDimensionV1),
    DuplicateEvidenceGate(EvidenceDimensionV1),
    VerifierRejected,
    ReplayDetected,
    ReceiptIdentityCollision,
    VerifierUnavailable,
    EpisodeMismatch,
    TenantMismatch,
    SubjectMismatch,
    PurposeMismatch,
    RecipientMismatch,
    InvalidEpisodeWindow,
    InvalidBudget,
    InvalidRiskLimit,
    EmptyDecisionSet,
    DuplicateDecision(String),
    DuplicateProbe(String),
    DuplicateProbeResourceType(String),
    InvalidProbeCost,
    EmptyProbeResourceScope,
    CandidateRequestedUnpermittedProbe(String),
    CandidateSelectedUnpermittedDecision(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedEvidenceSignalV1 {
    receipt: EvidenceReleaseReceiptV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedProbeOfferV1 {
    receipt: ProbeOfferReceiptV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedSignalV1 {
    pub channel_id: String,
    pub outcome_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PublicProbeOptionV1 {
    pub probe_id: String,
    pub acquisition_cost: f64,
    pub result_delay_steps: u64,
}

/// Field-for-field transport analogue of Symthaea's CandidateEpisode.
/// It contains no subject identifier, source identity, receipt, PHI or gold label.
#[derive(Clone, Debug, PartialEq)]
pub struct HealthDecisionEpisodeV1 {
    pub version: u16,
    pub episode_id: String,
    pub current_step: u64,
    pub decision_deadline_step: u64,
    pub remaining_probe_budget: f64,
    pub maximum_worst_case_loss: f64,
    pub allowed_decision_ids: Vec<String>,
    pub evidence: Vec<ObservedSignalV1>,
    pub permitted_probes: Vec<PublicProbeOptionV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateSubmissionV1 {
    pub decision_id: String,
    pub requested_probe_id: Option<String>,
}

/// Operator/evaluator-side mapping. Do not send this object to the candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceReferenceV1 {
    pub receipt_id: String,
    pub event_id: String,
    pub source_principal: String,
    pub source_connection_id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub resource_version: String,
    pub payload_digest: String,
    pub source_receipt_digest: String,
    pub clinical_event_time_ms: i64,
    pub specimen_collected_at_ms: Option<i64>,
    pub analysis_performed_at_ms: Option<i64>,
    pub result_issued_at_ms: Option<i64>,
    pub source_last_updated_at_ms: Option<i64>,
    pub source_sequence: Option<u64>,
    pub ingested_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalEvidenceReferenceV1 {
    pub channel_id: String,
    pub outcome_id: String,
    pub evidence: Vec<EvidenceReferenceV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeAuthorityReferenceV1 {
    pub probe_id: String,
    pub receipt_id: String,
    pub action_id: String,
    pub allowed_resource_types: Vec<String>,
    pub authority_receipt_digest: String,
    pub expires_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceReferenceManifestV1 {
    pub version: u16,
    pub episode_id: String,
    pub tenant_id: String,
    pub subject_binding: String,
    pub purpose: String,
    pub recipient_id: String,
    pub as_of_time_ms: i64,
    pub signals: Vec<SignalEvidenceReferenceV1>,
    pub probes: Vec<ProbeAuthorityReferenceV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EpisodeProjectionV1 {
    pub candidate_episode: HealthDecisionEpisodeV1,
    pub operator_manifest: EvidenceReferenceManifestV1,
}

fn non_empty(value: &str, field: &'static str) -> Result<(), BridgeErrorV1> {
    if value.trim().is_empty() {
        Err(BridgeErrorV1::EmptyField(field))
    } else {
        Ok(())
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_base64url_signature(value: &str) -> bool {
    value.len() == 86
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn validate_receipt_window(
    issued_at_ms: i64,
    expires_at_ms: i64,
    now_ms: i64,
) -> Result<(), BridgeErrorV1> {
    if expires_at_ms <= issued_at_ms {
        return Err(BridgeErrorV1::InvalidReceiptWindow);
    }
    if issued_at_ms > now_ms {
        return Err(BridgeErrorV1::ReceiptNotYetValid);
    }
    if expires_at_ms <= now_ms {
        return Err(BridgeErrorV1::ReceiptExpired);
    }
    if expires_at_ms.saturating_sub(issued_at_ms) > MAX_RECEIPT_TTL_MS {
        return Err(BridgeErrorV1::ReceiptTtlExceeded);
    }
    Ok(())
}

fn map_verifier_failure(error: VerifierFailureV1) -> BridgeErrorV1 {
    match error {
        VerifierFailureV1::Rejected => BridgeErrorV1::VerifierRejected,
        VerifierFailureV1::ReplayDetected => BridgeErrorV1::ReplayDetected,
        VerifierFailureV1::Unavailable => BridgeErrorV1::VerifierUnavailable,
    }
}

pub fn verify_evidence_release_v1<V: EvidenceReceiptVerifierV1>(
    receipt: EvidenceReleaseReceiptV1,
    now_ms: i64,
    verifier: &V,
) -> Result<VerifiedEvidenceSignalV1, BridgeErrorV1> {
    if receipt.nonce.len() < 16 {
        return Err(BridgeErrorV1::InvalidNonce);
    }
    for (value, field) in [
        (receipt.receipt_id.as_str(), "receipt_id"),
        (receipt.nonce.as_str(), "nonce"),
        (receipt.episode_id.as_str(), "episode_id"),
        (receipt.tenant_id.as_str(), "tenant_id"),
        (receipt.subject_binding.as_str(), "subject_binding"),
        (receipt.purpose.as_str(), "purpose"),
        (receipt.recipient_id.as_str(), "recipient_id"),
        (receipt.source_connection_id.as_str(), "source_connection_id"),
        (receipt.source_principal.as_str(), "source_principal"),
        (receipt.event_id.as_str(), "event_id"),
        (receipt.resource_type.as_str(), "resource_type"),
        (receipt.resource_id.as_str(), "resource_id"),
        (receipt.resource_version.as_str(), "resource_version"),
        (receipt.channel_id.as_str(), "channel_id"),
        (receipt.outcome_id.as_str(), "outcome_id"),
        (receipt.signing_key_id.as_str(), "signing_key_id"),
    ] {
        non_empty(value, field)?;
    }

    if !valid_base64url_signature(&receipt.signature) {
        return Err(BridgeErrorV1::InvalidSignature);
    }
    if !valid_digest(&receipt.payload_digest) {
        return Err(BridgeErrorV1::InvalidDigest("payload_digest"));
    }
    if !valid_digest(&receipt.source_receipt_digest) {
        return Err(BridgeErrorV1::InvalidDigest("source_receipt_digest"));
    }
    validate_receipt_window(receipt.issued_at_ms, receipt.expires_at_ms, now_ms)?;

    let mut dimensions = BTreeSet::new();
    for gate in &receipt.gates {
        if !valid_digest(&gate.attestation_digest) {
            return Err(BridgeErrorV1::InvalidDigest("gate_attestation_digest"));
        }
        if !dimensions.insert(gate.dimension) {
            return Err(BridgeErrorV1::DuplicateEvidenceGate(gate.dimension));
        }
    }
    for dimension in REQUIRED_EVIDENCE_DIMENSIONS_V1 {
        if !dimensions.contains(&dimension) {
            return Err(BridgeErrorV1::MissingEvidenceGate(dimension));
        }
    }
    if receipt.gates.len() != REQUIRED_GATE_COUNT {
        return Err(BridgeErrorV1::DuplicateEvidenceGate(
            EvidenceDimensionV1::EvidencePresence,
        ));
    }

    verifier
        .verify_and_consume_evidence(&receipt)
        .map_err(map_verifier_failure)?;

    Ok(VerifiedEvidenceSignalV1 { receipt })
}

pub fn verify_probe_offer_v1<V: EvidenceReceiptVerifierV1>(
    receipt: ProbeOfferReceiptV1,
    now_ms: i64,
    verifier: &V,
) -> Result<VerifiedProbeOfferV1, BridgeErrorV1> {
    if receipt.nonce.len() < 16 {
        return Err(BridgeErrorV1::InvalidNonce);
    }
    for (value, field) in [
        (receipt.receipt_id.as_str(), "probe_receipt_id"),
        (receipt.nonce.as_str(), "probe_nonce"),
        (receipt.episode_id.as_str(), "probe_episode_id"),
        (receipt.tenant_id.as_str(), "probe_tenant_id"),
        (receipt.subject_binding.as_str(), "probe_subject_binding"),
        (receipt.purpose.as_str(), "probe_purpose"),
        (receipt.recipient_id.as_str(), "probe_recipient_id"),
        (receipt.probe_id.as_str(), "probe_id"),
        (receipt.action_id.as_str(), "probe_action_id"),
        (receipt.signing_key_id.as_str(), "probe_signing_key_id"),
    ] {
        non_empty(value, field)?;
    }
    if !valid_base64url_signature(&receipt.signature) {
        return Err(BridgeErrorV1::InvalidSignature);
    }
    if !valid_digest(&receipt.authority_receipt_digest) {
        return Err(BridgeErrorV1::InvalidDigest("authority_receipt_digest"));
    }
    if !receipt.acquisition_cost.is_finite() || receipt.acquisition_cost < 0.0 {
        return Err(BridgeErrorV1::InvalidProbeCost);
    }
    if receipt.allowed_resource_types.is_empty()
        || receipt.allowed_resource_types.iter().any(|item| item.trim().is_empty())
    {
        return Err(BridgeErrorV1::EmptyProbeResourceScope);
    }
    let mut resource_types = BTreeSet::new();
    for resource_type in &receipt.allowed_resource_types {
        if !resource_types.insert(resource_type.as_str()) {
            return Err(BridgeErrorV1::DuplicateProbeResourceType(
                resource_type.clone(),
            ));
        }
    }
    validate_receipt_window(receipt.issued_at_ms, receipt.expires_at_ms, now_ms)?;
    verifier
        .verify_probe_offer(&receipt)
        .map_err(map_verifier_failure)?;
    Ok(VerifiedProbeOfferV1 { receipt })
}

#[derive(Clone, Debug, PartialEq)]
pub struct EpisodeContextV1 {
    pub episode_id: String,
    pub tenant_id: String,
    pub subject_binding: String,
    pub purpose: String,
    pub recipient_id: String,
    pub as_of_time_ms: i64,
    pub current_step: u64,
    pub decision_deadline_step: u64,
    pub remaining_probe_budget: f64,
    pub maximum_worst_case_loss: f64,
    pub allowed_decision_ids: Vec<String>,
}

pub fn project_episode_v1(
    context: EpisodeContextV1,
    signals: &[VerifiedEvidenceSignalV1],
    probes: &[VerifiedProbeOfferV1],
    now_ms: i64,
) -> Result<EpisodeProjectionV1, BridgeErrorV1> {
    for (value, field) in [
        (context.episode_id.as_str(), "episode_id"),
        (context.tenant_id.as_str(), "tenant_id"),
        (context.subject_binding.as_str(), "subject_binding"),
        (context.purpose.as_str(), "purpose"),
        (context.recipient_id.as_str(), "recipient_id"),
    ] {
        non_empty(value, field)?;
    }
    if context.decision_deadline_step < context.current_step {
        return Err(BridgeErrorV1::InvalidEpisodeWindow);
    }
    if context.as_of_time_ms < 0 || context.as_of_time_ms > now_ms {
        return Err(BridgeErrorV1::InvalidAsOfTime);
    }
    if !context.remaining_probe_budget.is_finite() || context.remaining_probe_budget < 0.0 {
        return Err(BridgeErrorV1::InvalidBudget);
    }
    if !context.maximum_worst_case_loss.is_finite()
        || !(0.0..=1.0).contains(&context.maximum_worst_case_loss)
    {
        return Err(BridgeErrorV1::InvalidRiskLimit);
    }
    if context.allowed_decision_ids.is_empty() {
        return Err(BridgeErrorV1::EmptyDecisionSet);
    }
    let mut decisions = BTreeSet::new();
    for decision in &context.allowed_decision_ids {
        non_empty(decision, "decision_id")?;
        if !decisions.insert(decision.as_str()) {
            return Err(BridgeErrorV1::DuplicateDecision(decision.clone()));
        }
    }

    let mut grouped: BTreeMap<(String, String), Vec<EvidenceReferenceV1>> = BTreeMap::new();
    let mut receipt_identity: BTreeMap<String, (String, String, EvidenceReferenceV1)> =
        BTreeMap::new();
    for signal in signals {
        let receipt = &signal.receipt;
        if receipt.episode_id != context.episode_id {
            return Err(BridgeErrorV1::EpisodeMismatch);
        }
        if receipt.tenant_id != context.tenant_id {
            return Err(BridgeErrorV1::TenantMismatch);
        }
        if receipt.subject_binding != context.subject_binding {
            return Err(BridgeErrorV1::SubjectMismatch);
        }
        if receipt.purpose != context.purpose {
            return Err(BridgeErrorV1::PurposeMismatch);
        }
        if receipt.recipient_id != context.recipient_id {
            return Err(BridgeErrorV1::RecipientMismatch);
        }
        validate_receipt_window(receipt.issued_at_ms, receipt.expires_at_ms, now_ms)?;
        if receipt.issued_at_ms > context.as_of_time_ms {
            return Err(BridgeErrorV1::ReceiptNotYetValid);
        }
        let reference = EvidenceReferenceV1 {
            receipt_id: receipt.receipt_id.clone(),
            event_id: receipt.event_id.clone(),
            source_principal: receipt.source_principal.clone(),
            source_connection_id: receipt.source_connection_id.clone(),
            resource_type: receipt.resource_type.clone(),
            resource_id: receipt.resource_id.clone(),
            resource_version: receipt.resource_version.clone(),
            payload_digest: receipt.payload_digest.clone(),
            source_receipt_digest: receipt.source_receipt_digest.clone(),
            clinical_event_time_ms: receipt.clinical_event_time_ms,
            specimen_collected_at_ms: receipt.specimen_collected_at_ms,
            analysis_performed_at_ms: receipt.analysis_performed_at_ms,
            result_issued_at_ms: receipt.result_issued_at_ms,
            source_last_updated_at_ms: receipt.source_last_updated_at_ms,
            source_sequence: receipt.source_sequence,
            ingested_at_ms: receipt.ingested_at_ms,
        };
        let identity = (
            receipt.channel_id.clone(),
            receipt.outcome_id.clone(),
            reference.clone(),
        );
        if let Some(existing) = receipt_identity.get(&receipt.receipt_id) {
            if existing != &identity {
                return Err(BridgeErrorV1::ReceiptIdentityCollision);
            }
        } else {
            receipt_identity.insert(receipt.receipt_id.clone(), identity);
        }
        grouped
            .entry((receipt.channel_id.clone(), receipt.outcome_id.clone()))
            .or_default()
            .push(reference);
    }

    let mut permitted_probe_options = Vec::with_capacity(probes.len());
    let mut probe_references = Vec::with_capacity(probes.len());
    let mut seen_probes = BTreeSet::new();
    for probe in probes {
        let receipt = &probe.receipt;
        if receipt.episode_id != context.episode_id {
            return Err(BridgeErrorV1::EpisodeMismatch);
        }
        if receipt.tenant_id != context.tenant_id {
            return Err(BridgeErrorV1::TenantMismatch);
        }
        if receipt.subject_binding != context.subject_binding {
            return Err(BridgeErrorV1::SubjectMismatch);
        }
        if receipt.purpose != context.purpose {
            return Err(BridgeErrorV1::PurposeMismatch);
        }
        if receipt.recipient_id != context.recipient_id {
            return Err(BridgeErrorV1::RecipientMismatch);
        }
        validate_receipt_window(receipt.issued_at_ms, receipt.expires_at_ms, now_ms)?;
        if receipt.issued_at_ms > context.as_of_time_ms {
            return Err(BridgeErrorV1::ReceiptNotYetValid);
        }
        if !seen_probes.insert(receipt.probe_id.as_str()) {
            return Err(BridgeErrorV1::DuplicateProbe(receipt.probe_id.clone()));
        }
        permitted_probe_options.push(PublicProbeOptionV1 {
            probe_id: receipt.probe_id.clone(),
            acquisition_cost: receipt.acquisition_cost,
            result_delay_steps: receipt.result_delay_steps,
        });
        let mut allowed_resource_types = receipt.allowed_resource_types.clone();
        allowed_resource_types.sort();
        probe_references.push(ProbeAuthorityReferenceV1 {
            probe_id: receipt.probe_id.clone(),
            receipt_id: receipt.receipt_id.clone(),
            action_id: receipt.action_id.clone(),
            allowed_resource_types,
            authority_receipt_digest: receipt.authority_receipt_digest.clone(),
            expires_at_ms: receipt.expires_at_ms,
        });
    }
    permitted_probe_options.sort_by(|left, right| left.probe_id.cmp(&right.probe_id));
    probe_references.sort_by(|left, right| left.probe_id.cmp(&right.probe_id));

    let mut evidence = Vec::with_capacity(grouped.len());
    let mut reference_signals = Vec::with_capacity(grouped.len());
    for ((channel_id, outcome_id), mut references) in grouped {
        references.sort_by(|left, right| left.receipt_id.cmp(&right.receipt_id));
        references.dedup_by(|left, right| left.receipt_id == right.receipt_id);
        evidence.push(ObservedSignalV1 {
            channel_id: channel_id.clone(),
            outcome_id: outcome_id.clone(),
        });
        reference_signals.push(SignalEvidenceReferenceV1 {
            channel_id,
            outcome_id,
            evidence: references,
        });
    }

    let candidate_episode = HealthDecisionEpisodeV1 {
        version: 1,
        episode_id: context.episode_id.clone(),
        current_step: context.current_step,
        decision_deadline_step: context.decision_deadline_step,
        remaining_probe_budget: context.remaining_probe_budget,
        maximum_worst_case_loss: context.maximum_worst_case_loss,
        allowed_decision_ids: context.allowed_decision_ids,
        evidence,
        permitted_probes: permitted_probe_options,
    };
    let operator_manifest = EvidenceReferenceManifestV1 {
        version: 1,
        episode_id: context.episode_id,
        tenant_id: context.tenant_id,
        subject_binding: context.subject_binding,
        purpose: context.purpose,
        recipient_id: context.recipient_id,
        as_of_time_ms: context.as_of_time_ms,
        signals: reference_signals,
        probes: probe_references,
    };

    Ok(EpisodeProjectionV1 {
        candidate_episode,
        operator_manifest,
    })
}

/// Validates only that a proposal references an allow-listed decision and probe.
/// This is not authorization to execute. Mycelix must re-check current policy,
/// consent, scope and revocation using the actual principal before acquisition.
pub fn validate_candidate_submission_v1(
    episode: &HealthDecisionEpisodeV1,
    submission: &CandidateSubmissionV1,
) -> Result<(), BridgeErrorV1> {
    if !episode
        .allowed_decision_ids
        .iter()
        .any(|decision| decision == &submission.decision_id)
    {
        return Err(BridgeErrorV1::CandidateSelectedUnpermittedDecision(
            submission.decision_id.clone(),
        ));
    }
    if let Some(probe_id) = &submission.requested_probe_id {
        if !episode
            .permitted_probes
            .iter()
            .any(|probe| &probe.probe_id == probe_id)
        {
            return Err(BridgeErrorV1::CandidateRequestedUnpermittedProbe(
                probe_id.clone(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AcceptingVerifier;

    impl EvidenceReceiptVerifierV1 for AcceptingVerifier {
        fn verify_and_consume_evidence(
            &self,
            _receipt: &EvidenceReleaseReceiptV1,
        ) -> Result<(), VerifierFailureV1> {
            Ok(())
        }

        fn verify_probe_offer(
            &self,
            _receipt: &ProbeOfferReceiptV1,
        ) -> Result<(), VerifierFailureV1> {
            Ok(())
        }
    }

    struct RejectingVerifier;

    impl EvidenceReceiptVerifierV1 for RejectingVerifier {
        fn verify_and_consume_evidence(
            &self,
            _receipt: &EvidenceReleaseReceiptV1,
        ) -> Result<(), VerifierFailureV1> {
            Err(VerifierFailureV1::Rejected)
        }

        fn verify_probe_offer(
            &self,
            _receipt: &ProbeOfferReceiptV1,
        ) -> Result<(), VerifierFailureV1> {
            Err(VerifierFailureV1::Unavailable)
        }
    }

    fn digest() -> String {
        "a".repeat(64)
    }

    fn evidence_receipt() -> EvidenceReleaseReceiptV1 {
        EvidenceReleaseReceiptV1 {
            receipt_id: "release-1".into(),
            nonce: "nonce-evidence-0001".into(),
            episode_id: "episode-1".into(),
            tenant_id: "tenant-1".into(),
            subject_binding: "subject-opaque-1".into(),
            purpose: "treatment".into(),
            recipient_id: "recipient-1".into(),
            source_connection_id: "connection-1".into(),
            source_principal: "issuer/clinic-A".into(),
            event_id: "event-1".into(),
            resource_type: "Observation".into(),
            resource_id: "resource-1".into(),
            resource_version: "v7".into(),
            payload_digest: digest(),
            source_receipt_digest: digest(),
            channel_id: "lab-potassium".into(),
            outcome_id: "elevated".into(),
            clinical_event_time_ms: 900,
            specimen_collected_at_ms: Some(800),
            analysis_performed_at_ms: Some(850),
            result_issued_at_ms: Some(900),
            source_last_updated_at_ms: Some(910),
            source_sequence: Some(7),
            ingested_at_ms: 1_500,
            issued_at_ms: 1_000,
            expires_at_ms: 250_000,
            gates: REQUIRED_EVIDENCE_DIMENSIONS_V1
                .into_iter()
                .map(|dimension| EvidenceGateAttestationV1 {
                    dimension,
                    attestation_digest: digest(),
                })
                .collect(),
            signing_key_id: "health-release-kid-01".into(),
            signature_algorithm: SignatureAlgorithmV1::Ed25519,
            signature: "A".repeat(86),
        }
    }

    fn probe_receipt() -> ProbeOfferReceiptV1 {
        ProbeOfferReceiptV1 {
            receipt_id: "probe-offer-1".into(),
            nonce: "nonce-probe-offer-0001".into(),
            episode_id: "episode-1".into(),
            tenant_id: "tenant-1".into(),
            subject_binding: "subject-opaque-1".into(),
            purpose: "treatment".into(),
            recipient_id: "recipient-1".into(),
            probe_id: "obtain-confirmatory-lab".into(),
            action_id: "fhir_read_observation".into(),
            allowed_resource_types: vec!["Observation".into()],
            acquisition_cost: 1.0,
            result_delay_steps: 2,
            issued_at_ms: 1_000,
            expires_at_ms: 250_000,
            authority_receipt_digest: digest(),
            signing_key_id: "health-probe-kid-01".into(),
            signature_algorithm: SignatureAlgorithmV1::Ed25519,
            signature: "A".repeat(86),
        }
    }

    fn context() -> EpisodeContextV1 {
        EpisodeContextV1 {
            episode_id: "episode-1".into(),
            tenant_id: "tenant-1".into(),
            subject_binding: "subject-opaque-1".into(),
            purpose: "treatment".into(),
            recipient_id: "recipient-1".into(),
            as_of_time_ms: 2_000,
            current_step: 2,
            decision_deadline_step: 10,
            remaining_probe_budget: 2.0,
            maximum_worst_case_loss: 0.25,
            allowed_decision_ids: vec!["abstain".into(), "review".into()],
        }
    }

    #[test]
    fn valid_evidence_maps_to_candidate_signal_and_operator_source_manifest() {
        let signal = verify_evidence_release_v1(evidence_receipt(), 2_000, &AcceptingVerifier)
            .expect("verified release");
        let probe = verify_probe_offer_v1(probe_receipt(), 2_000, &AcceptingVerifier)
            .expect("verified probe");
        let projection = project_episode_v1(context(), &[signal], &[probe], 2_000)
            .expect("projection");

        assert_eq!(projection.candidate_episode.version, 1);
        assert_eq!(projection.candidate_episode.evidence.len(), 1);
        assert_eq!(projection.candidate_episode.evidence[0].channel_id, "lab-potassium");
        assert_eq!(projection.candidate_episode.evidence[0].outcome_id, "elevated");
        assert_eq!(projection.operator_manifest.signals.len(), 1);
        assert_eq!(
            projection.operator_manifest.signals[0].evidence[0].receipt_id,
            "release-1"
        );
        assert_eq!(projection.operator_manifest.subject_binding, "subject-opaque-1");
        assert_eq!(projection.operator_manifest.as_of_time_ms, 2_000);
        assert_eq!(
            projection.operator_manifest.signals[0].evidence[0].clinical_event_time_ms,
            900
        );
        assert_eq!(
            projection.operator_manifest.signals[0].evidence[0].source_sequence,
            Some(7)
        );
        assert_eq!(projection.operator_manifest.probes.len(), 1);
        assert_eq!(
            projection.operator_manifest.probes[0].action_id,
            "fhir_read_observation"
        );
        assert_eq!(projection.candidate_episode.permitted_probes.len(), 1);
        assert_eq!(
            projection.candidate_episode.permitted_probes[0].probe_id,
            "obtain-confirmatory-lab"
        );
    }

    #[test]
    fn malformed_detached_signatures_fail_before_external_verifier() {
        let mut evidence = evidence_receipt();
        evidence.signature = "not-a-signature".into();
        assert_eq!(
            verify_evidence_release_v1(evidence, 2_000, &AcceptingVerifier),
            Err(BridgeErrorV1::InvalidSignature)
        );

        let mut probe = probe_receipt();
        probe.signature = "not-a-signature".into();
        assert_eq!(
            verify_probe_offer_v1(probe, 2_000, &AcceptingVerifier),
            Err(BridgeErrorV1::InvalidSignature)
        );
    }

    #[test]
    fn missing_or_duplicate_gate_receipt_fails_before_external_verifier() {
        let mut missing = evidence_receipt();
        missing.gates.pop();
        assert_eq!(
            verify_evidence_release_v1(missing, 2_000, &AcceptingVerifier),
            Err(BridgeErrorV1::MissingEvidenceGate(
                EvidenceDimensionV1::ConflictReconciliation
            ))
        );

        let mut duplicate = evidence_receipt();
        duplicate.gates.push(duplicate.gates[0].clone());
        assert!(matches!(
            verify_evidence_release_v1(duplicate, 2_000, &AcceptingVerifier),
            Err(BridgeErrorV1::DuplicateEvidenceGate(_))
        ));
    }

    #[test]
    fn expired_future_and_overlong_receipts_fail_closed() {
        let mut expired = evidence_receipt();
        expired.expires_at_ms = 1_500;
        assert_eq!(
            verify_evidence_release_v1(expired, 2_000, &AcceptingVerifier),
            Err(BridgeErrorV1::ReceiptExpired)
        );

        let mut future = evidence_receipt();
        future.issued_at_ms = 3_000;
        assert_eq!(
            verify_evidence_release_v1(future, 2_000, &AcceptingVerifier),
            Err(BridgeErrorV1::ReceiptNotYetValid)
        );

        let mut overlong = evidence_receipt();
        overlong.expires_at_ms = 400_000;
        assert_eq!(
            verify_evidence_release_v1(overlong, 2_000, &AcceptingVerifier),
            Err(BridgeErrorV1::ReceiptTtlExceeded)
        );
    }

    #[test]
    fn verifier_rejection_replay_and_outage_are_not_success() {
        assert_eq!(
            verify_evidence_release_v1(evidence_receipt(), 2_000, &RejectingVerifier),
            Err(BridgeErrorV1::VerifierRejected)
        );

        struct ReplayVerifier;
        impl EvidenceReceiptVerifierV1 for ReplayVerifier {
            fn verify_and_consume_evidence(
                &self,
                _receipt: &EvidenceReleaseReceiptV1,
            ) -> Result<(), VerifierFailureV1> {
                Err(VerifierFailureV1::ReplayDetected)
            }
            fn verify_probe_offer(
                &self,
                _receipt: &ProbeOfferReceiptV1,
            ) -> Result<(), VerifierFailureV1> {
                Err(VerifierFailureV1::ReplayDetected)
            }
        }
        assert_eq!(
            verify_evidence_release_v1(evidence_receipt(), 2_000, &ReplayVerifier),
            Err(BridgeErrorV1::ReplayDetected)
        );
        assert_eq!(
            verify_probe_offer_v1(probe_receipt(), 2_000, &RejectingVerifier),
            Err(BridgeErrorV1::VerifierUnavailable)
        );
    }

    #[test]
    fn cross_subject_purpose_recipient_or_tenant_signals_cannot_enter_episode() {
        let signal = verify_evidence_release_v1(evidence_receipt(), 2_000, &AcceptingVerifier)
            .expect("verified release");

        let mut wrong_subject = context();
        wrong_subject.subject_binding = "subject-opaque-B".into();
        assert_eq!(
            project_episode_v1(wrong_subject, &[signal.clone()], &[], 2_000),
            Err(BridgeErrorV1::SubjectMismatch)
        );

        let mut wrong_tenant = context();
        wrong_tenant.tenant_id = "tenant-B".into();
        assert_eq!(
            project_episode_v1(wrong_tenant, &[signal.clone()], &[], 2_000),
            Err(BridgeErrorV1::TenantMismatch)
        );

        let mut wrong_purpose = context();
        wrong_purpose.purpose = "research".into();
        assert_eq!(
            project_episode_v1(wrong_purpose, &[signal.clone()], &[], 2_000),
            Err(BridgeErrorV1::PurposeMismatch)
        );

        let mut wrong_recipient = context();
        wrong_recipient.recipient_id = "recipient-B".into();
        assert_eq!(
            project_episode_v1(wrong_recipient, &[signal], &[], 2_000),
            Err(BridgeErrorV1::RecipientMismatch)
        );
    }

    #[test]
    fn identical_channel_outcome_signals_aggregate_with_exact_lineage() {
        let first = verify_evidence_release_v1(evidence_receipt(), 2_000, &AcceptingVerifier)
            .expect("first release");
        let mut second_receipt = evidence_receipt();
        second_receipt.receipt_id = "release-2".into();
        second_receipt.event_id = "event-2".into();
        second_receipt.resource_id = "resource-2".into();
        second_receipt.nonce = "nonce-evidence-0002".into();
        let second =
            verify_evidence_release_v1(second_receipt, 2_000, &AcceptingVerifier)
                .expect("second release");

        let projection =
            project_episode_v1(context(), &[second, first], &[], 2_000).expect("projection");
        assert_eq!(projection.candidate_episode.evidence.len(), 1);
        assert_eq!(projection.operator_manifest.signals.len(), 1);
        assert_eq!(
            projection.operator_manifest.signals[0]
                .evidence
                .iter()
                .map(|item| item.receipt_id.as_str())
                .collect::<Vec<_>>(),
            vec!["release-1", "release-2"]
        );
    }

    #[test]
    fn reused_receipt_id_with_different_source_identity_is_rejected() {
        let first = verify_evidence_release_v1(evidence_receipt(), 2_000, &AcceptingVerifier)
            .expect("first release");
        let mut changed = evidence_receipt();
        changed.event_id = "different-event".into();
        changed.resource_id = "different-resource".into();
        changed.nonce = "nonce-changed-0002".into();
        let second = verify_evidence_release_v1(changed, 2_000, &AcceptingVerifier)
            .expect("second release");

        assert_eq!(
            project_episode_v1(context(), &[first, second], &[], 2_000),
            Err(BridgeErrorV1::ReceiptIdentityCollision)
        );
    }

    #[test]
    fn probe_offer_is_scope_bound_and_candidate_proposal_does_not_authorize_execution() {
        let probe = verify_probe_offer_v1(probe_receipt(), 2_000, &AcceptingVerifier)
            .expect("verified probe");
        let projection = project_episode_v1(context(), &[], &[probe], 2_000).unwrap();
        let proposal = CandidateSubmissionV1 {
            decision_id: "review".into(),
            requested_probe_id: Some("obtain-confirmatory-lab".into()),
        };
        assert!(validate_candidate_submission_v1(&projection.candidate_episode, &proposal).is_ok());

        let unauthorized = CandidateSubmissionV1 {
            decision_id: "review".into(),
            requested_probe_id: Some("read-all-patients".into()),
        };
        assert_eq!(
            validate_candidate_submission_v1(&projection.candidate_episode, &unauthorized),
            Err(BridgeErrorV1::CandidateRequestedUnpermittedProbe(
                "read-all-patients".into()
            ))
        );
        // Passing this contract check is not permission to execute. Production
        // must ask Mycelix's authority provider again immediately before access.
    }
}
