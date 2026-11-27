//! Versioned runtime qualification receipt contract.
//!
//! This module checks evidence structure, required probe coverage, freshness,
//! and observer privilege boundaries. Signature verification is injected.
//! It does not canonicalize/serialize the payload or implement cryptography.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RequiredProbe {
    CapabilityStatement,
    TransactionAtomicity,
    ConditionalReplay,
    StaleIfMatch,
    ConcurrentVersionConflict,
    TransactionRollback,
    DirectPostgresObserver,
}

const REQUIRED_PROBES: [RequiredProbe; 7] = [
    RequiredProbe::CapabilityStatement,
    RequiredProbe::TransactionAtomicity,
    RequiredProbe::ConditionalReplay,
    RequiredProbe::StaleIfMatch,
    RequiredProbe::ConcurrentVersionConflict,
    RequiredProbe::TransactionRollback,
    RequiredProbe::DirectPostgresObserver,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbeOutcome {
    Passed,
    Failed,
    Defer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbeEvidence {
    pub probe: RequiredProbe,
    pub outcome: ProbeOutcome,
    pub artifact_digest: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObserverBoundary {
    SameDatabaseIndependentSqlRole,
    IndependentReplica,
    ThirdPartyAuditor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentRuntimeIdentity {
    pub component_id: String,
    pub image_index_digest: String,
    pub runtime_image_digest: String,
    pub build_digest: String,
    pub config_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadOnlyObserverEvidence {
    pub observer_id: String,
    pub database_component_id: String,
    pub role_name: String,
    pub boundary: ObserverBoundary,
    pub session_read_only: bool,
    pub is_superuser: bool,
    pub can_create_roles: bool,
    pub can_insert: bool,
    pub can_update: bool,
    pub can_delete: bool,
    pub schema_digest: String,
    pub observation_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeQualificationReceiptV1 {
    pub schema_version: u32,
    pub receipt_id: String,
    pub operation_profile: String,
    pub fhir_version: String,
    pub issued_at_ms: i64,
    pub expires_at_ms: i64,
    pub server: ComponentRuntimeIdentity,
    pub database: ComponentRuntimeIdentity,
    pub adapter_build_digest: String,
    pub adapter_config_digest: String,
    pub capability_statement_digest: String,
    pub probe_corpus_digest: String,
    pub evidence_artifact_digest: String,
    pub observer: ReadOnlyObserverEvidence,
    pub probes: Vec<ProbeEvidence>,
    pub phi_used: bool,
    pub issuer_id: String,
    pub issuer_key_id: String,
    pub canonical_payload_digest: String,
    /// Detached signature. None means the receipt is not independently issued.
    pub signature: Option<String>,
}

pub trait QualificationReceiptAuthenticator {
    /// Production implementations must serialize the complete v1 receipt using
    /// a reviewed canonical format and verify its detached signature against a
    /// trusted, non-self-issued issuer key with current revocation information.
    fn verify_receipt_signature(&self, receipt: &RuntimeQualificationReceiptV1) -> bool;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QualificationVerdict {
    Qualified,
    NotQualified,
    Defer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QualificationFinding {
    UnsupportedSchemaVersion,
    IncompleteReceipt,
    InvalidDigest,
    InvalidTimeWindow,
    EvidenceNotYetIssued,
    EvidenceExpired,
    WrongFhirVersion,
    PhiPresent,
    ObserverIdentityMismatch,
    ObserverCanMutate,
    DuplicateProbe,
    MissingRequiredProbe,
    ProbeFailed,
    ProbeDeferred,
    MissingIssuerSignature,
    InvalidIssuerSignature,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualificationReport {
    pub verdict: QualificationVerdict,
    pub findings: Vec<QualificationFinding>,
    pub verified_probe_count: usize,
}

impl QualificationReport {
    fn qualified(probe_count: usize) -> Self {
        Self {
            verdict: QualificationVerdict::Qualified,
            findings: Vec::new(),
            verified_probe_count: probe_count,
        }
    }

    fn rejected(finding: QualificationFinding, probe_count: usize) -> Self {
        Self {
            verdict: QualificationVerdict::NotQualified,
            findings: vec![finding],
            verified_probe_count: probe_count,
        }
    }

    fn deferred(finding: QualificationFinding, probe_count: usize) -> Self {
        Self {
            verdict: QualificationVerdict::Defer,
            findings: vec![finding],
            verified_probe_count: probe_count,
        }
    }
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn complete_runtime_identity(identity: &ComponentRuntimeIdentity) -> bool {
    !identity.component_id.trim().is_empty()
        && valid_sha256(&identity.image_index_digest)
        && valid_sha256(&identity.runtime_image_digest)
        && valid_sha256(&identity.build_digest)
        && valid_sha256(&identity.config_digest)
}

/// Verify one runtime qualification receipt. Structural failures reject;
/// missing/stale/unknown evidence defers; a receipt is Qualified only when
/// every required probe passed and a trusted issuer signature verifies.
pub fn verify_runtime_qualification<A: QualificationReceiptAuthenticator>(
    receipt: &RuntimeQualificationReceiptV1,
    now_ms: i64,
    expected_server_id: &str,
    expected_database_id: &str,
    authenticator: &A,
) -> QualificationReport {
    let probe_count = receipt.probes.len();
    if receipt.schema_version != 1 {
        return QualificationReport::rejected(
            QualificationFinding::UnsupportedSchemaVersion,
            probe_count,
        );
    }
    if receipt.receipt_id.trim().is_empty()
        || receipt.operation_profile.trim().is_empty()
        || receipt.issuer_id.trim().is_empty()
        || receipt.issuer_key_id.trim().is_empty()
        || receipt.adapter_build_digest.trim().is_empty()
        || !valid_sha256(&receipt.adapter_build_digest)
        || !valid_sha256(&receipt.adapter_config_digest)
        || !valid_sha256(&receipt.capability_statement_digest)
        || !valid_sha256(&receipt.probe_corpus_digest)
        || !valid_sha256(&receipt.evidence_artifact_digest)
        || !valid_sha256(&receipt.canonical_payload_digest)
        || !complete_runtime_identity(&receipt.server)
        || !complete_runtime_identity(&receipt.database)
        || receipt.observer.observer_id.trim().is_empty()
        || receipt.observer.role_name.trim().is_empty()
        || !valid_sha256(&receipt.observer.schema_digest)
        || !valid_sha256(&receipt.observer.observation_digest)
    {
        return QualificationReport::rejected(
            QualificationFinding::IncompleteReceipt,
            probe_count,
        );
    }
    if receipt.issued_at_ms <= 0 || receipt.expires_at_ms <= receipt.issued_at_ms {
        return QualificationReport::rejected(
            QualificationFinding::InvalidTimeWindow,
            probe_count,
        );
    }
    if now_ms < receipt.issued_at_ms {
        return QualificationReport::rejected(
            QualificationFinding::EvidenceNotYetIssued,
            probe_count,
        );
    }
    if now_ms >= receipt.expires_at_ms {
        return QualificationReport::deferred(
            QualificationFinding::EvidenceExpired,
            probe_count,
        );
    }
    if receipt.fhir_version != "4.0.1" {
        return QualificationReport::rejected(
            QualificationFinding::WrongFhirVersion,
            probe_count,
        );
    }
    if receipt.phi_used {
        return QualificationReport::rejected(QualificationFinding::PhiPresent, probe_count);
    }
    if receipt.server.component_id != expected_server_id
        || receipt.database.component_id != expected_database_id
        || receipt.observer.database_component_id != expected_database_id
    {
        return QualificationReport::rejected(
            QualificationFinding::ObserverIdentityMismatch,
            probe_count,
        );
    }
    if !receipt.observer.session_read_only
        || receipt.observer.is_superuser
        || receipt.observer.can_create_roles
        || receipt.observer.can_insert
        || receipt.observer.can_update
        || receipt.observer.can_delete
    {
        return QualificationReport::rejected(
            QualificationFinding::ObserverCanMutate,
            probe_count,
        );
    }

    let mut seen = BTreeSet::new();
    for probe in &receipt.probes {
        if !seen.insert(probe.probe) {
            return QualificationReport::rejected(
                QualificationFinding::DuplicateProbe,
                probe_count,
            );
        }
        if !valid_sha256(&probe.artifact_digest) {
            return QualificationReport::rejected(
                QualificationFinding::InvalidDigest,
                probe_count,
            );
        }
        match probe.outcome {
            ProbeOutcome::Passed => {}
            ProbeOutcome::Failed => {
                return QualificationReport::rejected(
                    QualificationFinding::ProbeFailed,
                    probe_count,
                );
            }
            ProbeOutcome::Defer => {
                return QualificationReport::deferred(
                    QualificationFinding::ProbeDeferred,
                    probe_count,
                );
            }
        }
    }
    if REQUIRED_PROBES.iter().any(|probe| !seen.contains(probe)) {
        return QualificationReport::deferred(
            QualificationFinding::MissingRequiredProbe,
            probe_count,
        );
    }
    if receipt.signature.as_deref().map(str::trim).unwrap_or("").is_empty() {
        return QualificationReport::deferred(
            QualificationFinding::MissingIssuerSignature,
            probe_count,
        );
    }
    if !authenticator.verify_receipt_signature(receipt) {
        return QualificationReport::rejected(
            QualificationFinding::InvalidIssuerSignature,
            probe_count,
        );
    }

    QualificationReport::qualified(probe_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixtureAuthenticator {
        valid: bool,
    }

    impl QualificationReceiptAuthenticator for FixtureAuthenticator {
        fn verify_receipt_signature(&self, receipt: &RuntimeQualificationReceiptV1) -> bool {
            self.valid
                && receipt.signature.as_deref() == Some("fixture-signature")
        }
    }

    fn digest(fill: char) -> String {
        format!("sha256:{}", fill.to_string().repeat(64))
    }

    fn identity(id: &str, image: char, runtime: char, build: char, config: char) -> ComponentRuntimeIdentity {
        ComponentRuntimeIdentity {
            component_id: id.into(),
            image_index_digest: digest(image),
            runtime_image_digest: digest(runtime),
            build_digest: digest(build),
            config_digest: digest(config),
        }
    }

    fn valid_receipt() -> RuntimeQualificationReceiptV1 {
        RuntimeQualificationReceiptV1 {
            schema_version: 1,
            receipt_id: "qualification-hapi-postgres-run-47".into(),
            operation_profile: "mycelix-clinical-transaction-r4".into(),
            fhir_version: "4.0.1".into(),
            issued_at_ms: 10_000,
            expires_at_ms: 20_000,
            server: identity("hapi-r4", 'a', 'b', 'c', 'd'),
            database: identity("postgres-18", 'e', 'f', '1', '2'),
            adapter_build_digest: digest('3'),
            adapter_config_digest: digest('4'),
            capability_statement_digest: digest('5'),
            probe_corpus_digest: digest('6'),
            evidence_artifact_digest: digest('7'),
            observer: ReadOnlyObserverEvidence {
                observer_id: "hapi-observer-run-47".into(),
                database_component_id: "postgres-18".into(),
                role_name: "hapi_observer".into(),
                boundary: ObserverBoundary::SameDatabaseIndependentSqlRole,
                session_read_only: true,
                is_superuser: false,
                can_create_roles: false,
                can_insert: false,
                can_update: false,
                can_delete: false,
                schema_digest: digest('8'),
                observation_digest: digest('9'),
            },
            probes: REQUIRED_PROBES
                .iter()
                .map(|probe| ProbeEvidence {
                    probe: *probe,
                    outcome: ProbeOutcome::Passed,
                    artifact_digest: digest('a'),
                })
                .collect(),
            phi_used: false,
            issuer_id: "issuer:runtime-qualification-authority".into(),
            issuer_key_id: "runtime-qualification-key-1".into(),
            canonical_payload_digest: digest('b'),
            signature: Some("fixture-signature".into()),
        }
    }

    #[test]
    fn accepts_fresh_complete_signed_runtime_qualification() {
        let report = verify_runtime_qualification(
            &valid_receipt(),
            15_000,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: true },
        );
        assert_eq!(report.verdict, QualificationVerdict::Qualified);
        assert_eq!(report.verified_probe_count, 7);
    }

    #[test]
    fn missing_issuer_signature_defers_instead_of_qualifying() {
        let mut receipt = valid_receipt();
        receipt.signature = None;
        let report = verify_runtime_qualification(
            &receipt,
            15_000,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: true },
        );
        assert_eq!(report.verdict, QualificationVerdict::Defer);
        assert_eq!(report.findings, vec![QualificationFinding::MissingIssuerSignature]);
    }

    #[test]
    fn expired_runtime_qualification_defers() {
        let report = verify_runtime_qualification(
            &valid_receipt(),
            20_000,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: true },
        );
        assert_eq!(report.verdict, QualificationVerdict::Defer);
        assert_eq!(report.findings, vec![QualificationFinding::EvidenceExpired]);
    }

    #[test]
    fn missing_required_probe_defers() {
        let mut receipt = valid_receipt();
        receipt.probes.pop();
        let report = verify_runtime_qualification(
            &receipt,
            15_000,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: true },
        );
        assert_eq!(report.verdict, QualificationVerdict::Defer);
        assert_eq!(report.findings, vec![QualificationFinding::MissingRequiredProbe]);
    }

    #[test]
    fn failed_probe_rejects_qualification() {
        let mut receipt = valid_receipt();
        receipt.probes[0].outcome = ProbeOutcome::Failed;
        let report = verify_runtime_qualification(
            &receipt,
            15_000,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: true },
        );
        assert_eq!(report.verdict, QualificationVerdict::NotQualified);
        assert_eq!(report.findings, vec![QualificationFinding::ProbeFailed]);
    }

    #[test]
    fn write_capable_observer_rejects_qualification() {
        let mut receipt = valid_receipt();
        receipt.observer.can_update = true;
        let report = verify_runtime_qualification(
            &receipt,
            15_000,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: true },
        );
        assert_eq!(report.verdict, QualificationVerdict::NotQualified);
        assert_eq!(report.findings, vec![QualificationFinding::ObserverCanMutate]);
    }

    #[test]
    fn mismatched_observer_database_identity_rejects_qualification() {
        let mut receipt = valid_receipt();
        receipt.observer.database_component_id = "other-postgres".into();
        let report = verify_runtime_qualification(
            &receipt,
            15_000,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: true },
        );
        assert_eq!(report.verdict, QualificationVerdict::NotQualified);
        assert_eq!(report.findings, vec![QualificationFinding::ObserverIdentityMismatch]);
    }

    #[test]
    fn duplicate_probe_is_rejected() {
        let mut receipt = valid_receipt();
        receipt.probes.push(receipt.probes[0].clone());
        let report = verify_runtime_qualification(
            &receipt,
            15_000,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: true },
        );
        assert_eq!(report.verdict, QualificationVerdict::NotQualified);
        assert_eq!(report.findings, vec![QualificationFinding::DuplicateProbe]);
    }

    #[test]
    fn phi_presence_rejects_qualification() {
        let mut receipt = valid_receipt();
        receipt.phi_used = true;
        let report = verify_runtime_qualification(
            &receipt,
            15_000,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: true },
        );
        assert_eq!(report.verdict, QualificationVerdict::NotQualified);
        assert_eq!(report.findings, vec![QualificationFinding::PhiPresent]);
    }

    #[test]
    fn deferred_required_probe_defers_qualification() {
        let mut receipt = valid_receipt();
        receipt.probes[0].outcome = ProbeOutcome::Defer;
        let report = verify_runtime_qualification(
            &receipt,
            15_000,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: true },
        );
        assert_eq!(report.verdict, QualificationVerdict::Defer);
        assert_eq!(report.findings, vec![QualificationFinding::ProbeDeferred]);
    }

    #[test]
    fn future_issued_receipt_is_rejected() {
        let report = verify_runtime_qualification(
            &valid_receipt(),
            9_999,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: true },
        );
        assert_eq!(report.verdict, QualificationVerdict::NotQualified);
        assert_eq!(report.findings, vec![QualificationFinding::EvidenceNotYetIssued]);
    }

    #[test]
    fn invalid_validity_window_is_rejected() {
        let mut receipt = valid_receipt();
        receipt.expires_at_ms = receipt.issued_at_ms;
        let report = verify_runtime_qualification(
            &receipt,
            15_000,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: true },
        );
        assert_eq!(report.verdict, QualificationVerdict::NotQualified);
        assert_eq!(report.findings, vec![QualificationFinding::InvalidTimeWindow]);
    }

    #[test]
    fn invalid_issuer_signature_rejects_qualification() {
        let report = verify_runtime_qualification(
            &valid_receipt(),
            15_000,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: false },
        );
        assert_eq!(report.verdict, QualificationVerdict::NotQualified);
        assert_eq!(report.findings, vec![QualificationFinding::InvalidIssuerSignature]);
    }

    #[test]
    fn wrong_fhir_version_rejects_qualification() {
        let mut receipt = valid_receipt();
        receipt.fhir_version = "5.0.0".into();
        let report = verify_runtime_qualification(
            &receipt,
            15_000,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: true },
        );
        assert_eq!(report.verdict, QualificationVerdict::NotQualified);
        assert_eq!(report.findings, vec![QualificationFinding::WrongFhirVersion]);
    }

    #[test]
    fn malformed_runtime_digest_rejects_qualification() {
        let mut receipt = valid_receipt();
        receipt.database.image_index_digest = "postgres:latest".into();
        let report = verify_runtime_qualification(
            &receipt,
            15_000,
            "hapi-r4",
            "postgres-18",
            &FixtureAuthenticator { valid: true },
        );
        assert_eq!(report.verdict, QualificationVerdict::NotQualified);
        assert_eq!(report.findings, vec![QualificationFinding::IncompleteReceipt]);
    }
}
