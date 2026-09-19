#![forbid(unsafe_code)]
//! Evidence-bearing security-monitoring reference semantics.
//!
//! This crate freezes one narrow theorem:
//!
//! > `Clean` is an evidence-bearing assessment, not the absence of an error.
//!
//! It deliberately does not fetch Holochain records, implement anomaly rules,
//! persist audit events, or make regulatory/security-compliance claims. Those
//! mechanisms remain independent qualification lines.

use core::fmt;

pub const MONITORING_CONTRACT_V1: u16 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubjectContext([u8; 32]);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuleSetId([u8; 32]);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EvidenceDigest([u8; 32]);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnomalyId([u8; 32]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdError {
    AllZero,
}

macro_rules! opaque32 {
    ($ty:ident) => {
        impl $ty {
            pub fn new(bytes: [u8; 32]) -> Result<Self, IdError> {
                if bytes == [0u8; 32] {
                    return Err(IdError::AllZero);
                }
                Ok(Self(bytes))
            }

            pub fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl fmt::Debug for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($ty), "([redacted])"))
            }
        }
    };
}

opaque32!(SubjectContext);
opaque32!(RuleSetId);
opaque32!(EvidenceDigest);
opaque32!(AnomalyId);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationWindow {
    pub start_micros: i64,
    pub end_micros: i64,
}

impl ObservationWindow {
    pub fn new(start_micros: i64, end_micros: i64) -> Result<Self, WindowError> {
        if end_micros < start_micros {
            return Err(WindowError::EndBeforeStart);
        }
        Ok(Self {
            start_micros,
            end_micros,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowError {
    EndBeforeStart,
}

/// Complete evidence retrieved for one monitoring evaluation.
///
/// Zero source records is valid: an explicitly queried empty evidence set is
/// different from an unavailable source. The digest must still identify that
/// exact empty set canonically.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompleteEvidence {
    pub subject: SubjectContext,
    pub window: ObservationWindow,
    pub source_record_count: u64,
    pub source_set_digest: EvidenceDigest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceInput {
    Complete(CompleteEvidence),
    Unavailable(MonitoringUnavailable),
    Invalid(MonitoringEvidenceError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitoringUnavailable {
    pub reason: UnavailableReason,
    pub detail_code: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnavailableReason {
    AuthorizationDependencyUnavailable,
    Network,
    Authentication,
    Unauthorized,
    Countersigning,
    SourceNotConfigured,
    PartialEvidence,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitoringEvidenceError {
    pub reason: EvidenceErrorReason,
    pub source_record_index: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceErrorReason {
    Decode,
    UnsupportedSchema,
    MalformedRecord,
    AmbiguousOrdering,
    DigestMismatch,
    RuleInputInvalid,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnomalyKind {
    RapidAccess,
    BulkExport,
    OffHours,
    UnrelatedPatient,
    DecryptionFailure,
    AuthorizationFailureBurst,
    Other(u16),
}

/// Rule output only. It intentionally carries no clear patient identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessAnomaly {
    pub id: AnomalyId,
    pub kind: AnomalyKind,
    /// Basis points, 0..=10_000. Avoids floating-point/NaN ambiguity in the
    /// reference contract.
    pub severity_bps: u16,
    pub source_record_count: u64,
}

impl AccessAnomaly {
    pub fn new(
        id: AnomalyId,
        kind: AnomalyKind,
        severity_bps: u16,
        source_record_count: u64,
    ) -> Result<Self, AnomalyError> {
        if severity_bps > 10_000 {
            return Err(AnomalyError::SeverityOutOfRange);
        }
        if source_record_count == 0 {
            return Err(AnomalyError::NoSupportingRecords);
        }
        Ok(Self {
            id,
            kind,
            severity_bps,
            source_record_count,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnomalyError {
    SeverityOutOfRange,
    NoSupportingRecords,
}

/// Positive receipt that monitoring actually executed over a complete evidence
/// set. A `Clean` result is invalid without this receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitoringEvidenceReceipt {
    pub contract_version: u16,
    pub subject: SubjectContext,
    pub window: ObservationWindow,
    pub source_record_count: u64,
    pub source_set_digest: EvidenceDigest,
    pub rule_set: RuleSetId,
    pub evaluated_at_micros: i64,
}

impl MonitoringEvidenceReceipt {
    fn new(
        evidence: &CompleteEvidence,
        rule_set: RuleSetId,
        evaluated_at_micros: i64,
    ) -> Result<Self, AssessmentError> {
        if evaluated_at_micros < evidence.window.end_micros {
            return Err(AssessmentError::EvaluationPredatesWindowEnd);
        }
        Ok(Self {
            contract_version: MONITORING_CONTRACT_V1,
            subject: evidence.subject,
            window: evidence.window,
            source_record_count: evidence.source_record_count,
            source_set_digest: evidence.source_set_digest,
            rule_set,
            evaluated_at_micros,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MonitoringAssessment {
    /// Complete evidence was retrieved and evaluated; no rules fired.
    Clean(MonitoringEvidenceReceipt),
    /// Complete evidence was retrieved and evaluated; one or more rules fired.
    Anomalies {
        anomalies: Vec<AccessAnomaly>,
        evidence: MonitoringEvidenceReceipt,
    },
    /// Monitoring did not have the evidence needed to decide.
    Unavailable(MonitoringUnavailable),
    /// Evidence was present but could not be trusted/evaluated.
    InvalidEvidence(MonitoringEvidenceError),
}

impl MonitoringAssessment {
    pub fn is_clean(&self) -> bool {
        matches!(self, Self::Clean(_))
    }

    pub fn has_positive_evidence_receipt(&self) -> bool {
        matches!(self, Self::Clean(_) | Self::Anomalies { .. })
    }

    pub fn evidence_receipt(&self) -> Option<&MonitoringEvidenceReceipt> {
        match self {
            Self::Clean(receipt) => Some(receipt),
            Self::Anomalies { evidence, .. } => Some(evidence),
            Self::Unavailable(_) | Self::InvalidEvidence(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssessmentError {
    EvaluationPredatesWindowEnd,
    DuplicateAnomalyId,
    AnomalyClaimsMoreRecordsThanEvidence,
}

/// Converts evidence retrieval + deterministic rule output into the typed
/// monitoring state. Retrieval failures cannot become `Clean` through this API.
pub fn assess(
    input: EvidenceInput,
    rule_set: RuleSetId,
    evaluated_at_micros: i64,
    anomalies: Vec<AccessAnomaly>,
) -> Result<MonitoringAssessment, AssessmentError> {
    match input {
        EvidenceInput::Unavailable(reason) => Ok(MonitoringAssessment::Unavailable(reason)),
        EvidenceInput::Invalid(reason) => Ok(MonitoringAssessment::InvalidEvidence(reason)),
        EvidenceInput::Complete(evidence) => {
            let receipt =
                MonitoringEvidenceReceipt::new(&evidence, rule_set, evaluated_at_micros)?;

            for (index, anomaly) in anomalies.iter().enumerate() {
                if anomaly.source_record_count > evidence.source_record_count {
                    return Err(AssessmentError::AnomalyClaimsMoreRecordsThanEvidence);
                }
                if anomalies[..index]
                    .iter()
                    .any(|prior| prior.id == anomaly.id)
                {
                    return Err(AssessmentError::DuplicateAnomalyId);
                }
            }

            if anomalies.is_empty() {
                Ok(MonitoringAssessment::Clean(receipt))
            } else {
                Ok(MonitoringAssessment::Anomalies {
                    anomalies,
                    evidence: receipt,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject() -> SubjectContext {
        SubjectContext::new([1; 32]).unwrap()
    }

    fn rules() -> RuleSetId {
        RuleSetId::new([2; 32]).unwrap()
    }

    fn digest() -> EvidenceDigest {
        EvidenceDigest::new([3; 32]).unwrap()
    }

    fn anomaly_id(byte: u8) -> AnomalyId {
        AnomalyId::new([byte; 32]).unwrap()
    }

    fn window() -> ObservationWindow {
        ObservationWindow::new(100, 200).unwrap()
    }

    fn evidence(count: u64) -> CompleteEvidence {
        CompleteEvidence {
            subject: subject(),
            window: window(),
            source_record_count: count,
            source_set_digest: digest(),
        }
    }

    #[test]
    fn clean_requires_complete_evidence_and_returns_receipt() {
        let result = assess(EvidenceInput::Complete(evidence(4)), rules(), 250, vec![]).unwrap();
        let MonitoringAssessment::Clean(receipt) = result else {
            panic!("expected clean assessment");
        };
        assert_eq!(receipt.source_record_count, 4);
        assert_eq!(receipt.rule_set, rules());
        assert_eq!(receipt.contract_version, MONITORING_CONTRACT_V1);
    }

    #[test]
    fn zero_records_can_be_clean_when_empty_set_was_retrieved_and_digested() {
        let result = assess(EvidenceInput::Complete(evidence(0)), rules(), 250, vec![]).unwrap();
        assert!(result.is_clean());
        assert_eq!(result.evidence_receipt().unwrap().source_record_count, 0);
    }

    #[test]
    fn unavailable_never_becomes_clean_even_with_empty_anomaly_vector() {
        let result = assess(
            EvidenceInput::Unavailable(MonitoringUnavailable {
                reason: UnavailableReason::Network,
                detail_code: Some(503),
            }),
            rules(),
            250,
            vec![],
        )
        .unwrap();
        assert!(matches!(result, MonitoringAssessment::Unavailable(_)));
        assert!(!result.is_clean());
        assert!(!result.has_positive_evidence_receipt());
    }

    #[test]
    fn invalid_evidence_never_becomes_clean() {
        let result = assess(
            EvidenceInput::Invalid(MonitoringEvidenceError {
                reason: EvidenceErrorReason::Decode,
                source_record_index: None,
            }),
            rules(),
            250,
            vec![],
        )
        .unwrap();
        assert!(matches!(result, MonitoringAssessment::InvalidEvidence(_)));
        assert!(!result.is_clean());
    }

    #[test]
    fn anomalies_require_complete_evidence_and_preserve_receipt() {
        let anomaly = AccessAnomaly::new(
            anomaly_id(4),
            AnomalyKind::RapidAccess,
            8_500,
            2,
        )
        .unwrap();
        let result = assess(
            EvidenceInput::Complete(evidence(4)),
            rules(),
            250,
            vec![anomaly],
        )
        .unwrap();
        let MonitoringAssessment::Anomalies {
            anomalies,
            evidence,
        } = result
        else {
            panic!("expected anomaly assessment");
        };
        assert_eq!(anomalies.len(), 1);
        assert_eq!(evidence.source_record_count, 4);
    }

    #[test]
    fn evaluation_cannot_precede_end_of_observation_window() {
        let err = assess(EvidenceInput::Complete(evidence(1)), rules(), 199, vec![]).unwrap_err();
        assert_eq!(err, AssessmentError::EvaluationPredatesWindowEnd);
    }

    #[test]
    fn invalid_window_is_rejected() {
        assert_eq!(
            ObservationWindow::new(200, 199),
            Err(WindowError::EndBeforeStart)
        );
    }

    #[test]
    fn all_zero_identifiers_are_rejected() {
        assert_eq!(SubjectContext::new([0; 32]), Err(IdError::AllZero));
        assert_eq!(RuleSetId::new([0; 32]), Err(IdError::AllZero));
        assert_eq!(EvidenceDigest::new([0; 32]), Err(IdError::AllZero));
        assert_eq!(AnomalyId::new([0; 32]), Err(IdError::AllZero));
    }

    #[test]
    fn anomaly_severity_is_bounded() {
        assert_eq!(
            AccessAnomaly::new(anomaly_id(5), AnomalyKind::OffHours, 10_001, 1),
            Err(AnomalyError::SeverityOutOfRange)
        );
    }

    #[test]
    fn anomaly_requires_supporting_records() {
        assert_eq!(
            AccessAnomaly::new(anomaly_id(5), AnomalyKind::OffHours, 5_000, 0),
            Err(AnomalyError::NoSupportingRecords)
        );
    }

    #[test]
    fn duplicate_anomaly_ids_are_rejected() {
        let a = AccessAnomaly::new(anomaly_id(6), AnomalyKind::RapidAccess, 7_000, 1).unwrap();
        let b = AccessAnomaly::new(anomaly_id(6), AnomalyKind::BulkExport, 8_000, 1).unwrap();
        let err = assess(
            EvidenceInput::Complete(evidence(2)),
            rules(),
            250,
            vec![a, b],
        )
        .unwrap_err();
        assert_eq!(err, AssessmentError::DuplicateAnomalyId);
    }

    #[test]
    fn anomaly_cannot_claim_more_source_records_than_were_evaluated() {
        let anomaly =
            AccessAnomaly::new(anomaly_id(7), AnomalyKind::BulkExport, 9_000, 3).unwrap();
        let err = assess(
            EvidenceInput::Complete(evidence(2)),
            rules(),
            250,
            vec![anomaly],
        )
        .unwrap_err();
        assert_eq!(err, AssessmentError::AnomalyClaimsMoreRecordsThanEvidence);
    }

    #[test]
    fn unavailable_and_invalid_states_have_no_evidence_receipt() {
        let unavailable = MonitoringAssessment::Unavailable(MonitoringUnavailable {
            reason: UnavailableReason::Authentication,
            detail_code: None,
        });
        let invalid = MonitoringAssessment::InvalidEvidence(MonitoringEvidenceError {
            reason: EvidenceErrorReason::MalformedRecord,
            source_record_index: Some(3),
        });
        assert!(unavailable.evidence_receipt().is_none());
        assert!(invalid.evidence_receipt().is_none());
    }

    #[test]
    fn opaque_identifiers_redact_debug_output() {
        assert_eq!(format!("{:?}", subject()), "SubjectContext([redacted])");
        assert_eq!(format!("{:?}", rules()), "RuleSetId([redacted])");
        assert_eq!(format!("{:?}", digest()), "EvidenceDigest([redacted])");
    }
}
