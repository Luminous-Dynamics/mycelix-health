// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic, offline, non-authoritative healthcare claims-integrity signals.
//!
//! This prototype flags a small number of record patterns for human review. It
//! does not infer fraud or decide coverage, payment, care, or provider status.
//! Source digests are syntax-checked only; authenticity and digest-to-source
//! verification belong in a separately trusted admission layer.

use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: u16 = 1;
pub const RULESET_VERSION: &str = "claims-integrity-rules-v1";
pub const MAX_CLAIM_LINES: usize = 20_000;
const MAX_IDENTIFIER_LEN: usize = 128;

/// Declared completeness of the caller-supplied evidence bundle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputCompleteness {
    Complete,
    Partial,
    Unknown,
}

/// Claim-line role in the supplied evidence bundle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimLineStatus {
    Submitted,
    Reversal,
}

/// A normalized claim-line record.
///
/// All references must be opaque local tokens or synthetic identifiers. This
/// type does not prove that identifiers are de-identified and must not be fed
/// real patient/provider data in the prototype.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimLineV1 {
    pub line_id: String,
    pub claim_id: String,
    pub beneficiary_ref: String,
    pub provider_ref: String,
    pub service_code: String,
    /// ISO-4217-style three-letter currency code; only syntax is checked.
    pub currency_code: String,
    /// ISO calendar date in YYYYMMDD form, interpreted under the source policy.
    pub service_date_yyyymmdd: u32,
    /// Monetary value in the smallest currency unit; currency is defined by
    /// the applicable policy/evidence manifest and is not inferred here.
    pub amount_minor_units: u64,
    pub status: ClaimLineStatus,
    /// Required on reversal records; references the line this record reverses.
    pub reverses_line_id: Option<String>,
    /// Lowercase SHA-256 digest of the source record, verified upstream.
    pub source_record_digest: String,
}

/// Evidence bundle consumed by the offline rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimEvidenceBundleV1 {
    pub schema_version: u16,
    /// Lowercase SHA-256 digest of a canonical source manifest, verified upstream.
    pub input_manifest_digest: String,
    /// Lowercase SHA-256 digest of the applicable policy, verified upstream.
    pub policy_digest: String,
    pub code_set_version: String,
    pub completeness: InputCompleteness,
    pub lines: Vec<ClaimLineV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisStatus {
    Complete,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisScope {
    DeclaredCompleteBundle,
    ObservedRecordsOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnalysisReason {
    UnsupportedSchemaVersion,
    InvalidInputManifestDigest,
    InvalidPolicyDigest,
    InvalidCodeSetVersion,
    InvalidCurrencyCode,
    LineCountLimitExceeded,
    EmptyInput,
    DuplicateLineId,
    InvalidLineIdentifier,
    InvalidServiceDate,
    InvalidSourceRecordDigest,
    InvalidReversalReference,
    PartialInput,
    UnknownInputCompleteness,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalKind {
    PotentialDuplicateServiceLines,
    ReversalTargetNotFound,
    ReversalTargetNotSubmitted,
    ReversalTargetMismatch,
}

/// The sole authority marker available in V1; signals cannot represent a
/// substantiated finding or enforcement decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionAuthority {
    NonAuthoritativeReviewLead,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalReason {
    MatchingBeneficiaryProviderServiceDateAndAmount,
    ReversalReferenceAbsentFromBundle,
    ReversalTargetIsNotSubmittedLine,
    ReversalDoesNotMatchTargetService,
}

/// A line reference keeps the source digest attached to the exact line it
/// describes. A digest here is a locator/commitment string, not authentication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineEvidenceRefV1 {
    pub line_id: String,
    pub source_record_digest: String,
}

/// A signal is a lead for review, never a finding of fraud or wrongdoing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnomalySignalV1 {
    /// Stable within the exact evidence bundle and this ruleset. Not a signature.
    pub signal_key: String,
    pub ruleset_version: String,
    pub code_set_version: String,
    pub input_completeness: InputCompleteness,
    pub kind: SignalKind,
    pub reason: SignalReason,
    pub decision_authority: DecisionAuthority,
    pub input_manifest_digest: String,
    pub policy_digest: String,
    /// Sorted by line identifier; each line remains paired with its own digest.
    pub evidence: Vec<LineEvidenceRefV1>,
}

impl AnomalySignalV1 {
    pub fn line_ids(&self) -> Vec<&str> {
        self.evidence.iter().map(|reference| reference.line_id.as_str()).collect()
    }
}

/// Result from a single deterministic evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationReportV1 {
    pub schema_version: u16,
    pub ruleset_version: String,
    pub status: AnalysisStatus,
    pub scope: AnalysisScope,
    pub input_manifest_digest: String,
    pub policy_digest: String,
    pub code_set_version: String,
    pub lines_examined: usize,
    pub reasons: Vec<AnalysisReason>,
    pub signals: Vec<AnomalySignalV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DuplicateKey {
    beneficiary_ref: String,
    provider_ref: String,
    service_code: String,
    currency_code: String,
    service_date_yyyymmdd: u32,
    amount_minor_units: u64,
}

/// Evaluate a complete or incomplete evidence bundle with deterministic rules.
///
/// Invalid schemas, identifiers, dates, or digest syntax fail closed and return
/// no signals. Partial/unknown bundles are analyzed only within the observed
/// records, explicitly marked Indeterminate, and must not be interpreted as a
/// clean bill of health.
pub fn evaluate(bundle: &ClaimEvidenceBundleV1) -> EvaluationReportV1 {
    let mut reasons = Vec::new();

    if bundle.schema_version != SCHEMA_VERSION {
        reasons.push(AnalysisReason::UnsupportedSchemaVersion);
    }
    if !is_sha256_hex(&bundle.input_manifest_digest) {
        reasons.push(AnalysisReason::InvalidInputManifestDigest);
    }
    if !is_sha256_hex(&bundle.policy_digest) {
        reasons.push(AnalysisReason::InvalidPolicyDigest);
    }
    if !is_identifier(&bundle.code_set_version) {
        reasons.push(AnalysisReason::InvalidCodeSetVersion);
    }
    if bundle.lines.len() > MAX_CLAIM_LINES {
        reasons.push(AnalysisReason::LineCountLimitExceeded);
    }
    if bundle.lines.is_empty() {
        reasons.push(AnalysisReason::EmptyInput);
    }

    if !reasons.is_empty() {
        return report(
            bundle,
            AnalysisStatus::Indeterminate,
            AnalysisScope::ObservedRecordsOnly,
            reasons,
            Vec::new(),
            0,
        );
    }

    let mut seen_line_ids = BTreeSet::new();
    let mut has_duplicate_line_id = false;
    for line in &bundle.lines {
        if !seen_line_ids.insert(line.line_id.clone()) {
            has_duplicate_line_id = true;
        }
    }

    if has_duplicate_line_id {
        return report(
            bundle,
            AnalysisStatus::Indeterminate,
            AnalysisScope::ObservedRecordsOnly,
            vec![AnalysisReason::DuplicateLineId],
            Vec::new(),
            0,
        );
    }

    for line in &bundle.lines {
        if !is_identifier(&line.line_id)
            || !is_identifier(&line.claim_id)
            || !is_identifier(&line.beneficiary_ref)
            || !is_identifier(&line.provider_ref)
            || !is_identifier(&line.service_code)
        {
            reasons.push(AnalysisReason::InvalidLineIdentifier);
        }
        if !is_valid_currency_code(&line.currency_code) {
            reasons.push(AnalysisReason::InvalidCurrencyCode);
        }
        if !is_valid_date(line.service_date_yyyymmdd) {
            reasons.push(AnalysisReason::InvalidServiceDate);
        }
        if !is_sha256_hex(&line.source_record_digest) {
            reasons.push(AnalysisReason::InvalidSourceRecordDigest);
        }

        match line.status {
            ClaimLineStatus::Submitted if line.reverses_line_id.is_some() => {
                reasons.push(AnalysisReason::InvalidReversalReference);
            }
            ClaimLineStatus::Reversal => match line.reverses_line_id.as_deref() {
                Some(target) if is_identifier(target) && target != line.line_id => {}
                _ => reasons.push(AnalysisReason::InvalidReversalReference),
            },
            ClaimLineStatus::Submitted => {}
        }
    }

    reasons.sort_by_key(reason_sort_key);
    reasons.dedup();

    if !reasons.is_empty() {
        return report(
            bundle,
            AnalysisStatus::Indeterminate,
            AnalysisScope::ObservedRecordsOnly,
            reasons,
            Vec::new(),
            0,
        );
    }

    let by_id: BTreeMap<&str, &ClaimLineV1> = bundle
        .lines
        .iter()
        .map(|line| (line.line_id.as_str(), line))
        .collect();

    let mut signals = Vec::new();
    let mut known_reversed_targets = BTreeSet::new();

    for line in &bundle.lines {
        if line.status != ClaimLineStatus::Reversal {
            continue;
        }
        let target = line.reverses_line_id.as_deref().unwrap_or_default();
        match by_id.get(target) {
            None => signals.push(make_signal(
                bundle,
                format!("reversal_target_missing:{}", line.line_id),
                SignalKind::ReversalTargetNotFound,
                SignalReason::ReversalReferenceAbsentFromBundle,
                vec![line],
            )),
            Some(target_line)
                if target_line.status == ClaimLineStatus::Submitted
                    && reversal_matches_target(line, target_line) =>
            {
                // A reversal suppresses a submitted line from duplicate grouping
                // only when the service identity and full amount agree. A dangling
                // or mismatched reversal must not make duplicate evidence disappear.
                known_reversed_targets.insert(target.to_string());
            }
            Some(target_line) if target_line.status == ClaimLineStatus::Submitted => {
                signals.push(make_signal(
                    bundle,
                    format!("reversal_target_mismatch:{}", line.line_id),
                    SignalKind::ReversalTargetMismatch,
                    SignalReason::ReversalDoesNotMatchTargetService,
                    vec![line, *target_line],
                ));
            }
            Some(target_line) => signals.push(make_signal(
                bundle,
                format!("reversal_target_not_submitted:{}", line.line_id),
                SignalKind::ReversalTargetNotSubmitted,
                SignalReason::ReversalTargetIsNotSubmittedLine,
                vec![line, *target_line],
            )),
        }
    }

    // BTreeMap makes grouping and output independent of caller input order.
    let mut groups: BTreeMap<DuplicateKey, Vec<&ClaimLineV1>> = BTreeMap::new();
    for line in &bundle.lines {
        if line.status != ClaimLineStatus::Submitted
            || known_reversed_targets.contains(&line.line_id)
        {
            continue;
        }

        groups
            .entry(DuplicateKey {
                beneficiary_ref: line.beneficiary_ref.clone(),
                provider_ref: line.provider_ref.clone(),
                service_code: line.service_code.clone(),
                currency_code: line.currency_code.clone(),
                service_date_yyyymmdd: line.service_date_yyyymmdd,
                amount_minor_units: line.amount_minor_units,
            })
            .or_default()
            .push(line);
    }

    for mut group in groups.into_values() {
        if group.len() < 2 {
            continue;
        }
        group.sort_by(|left, right| left.line_id.cmp(&right.line_id));
        let first_line_id = group.first().map(|line| line.line_id.as_str()).unwrap_or_default();
        signals.push(make_signal(
            bundle,
            format!("potential_duplicate_group:{first_line_id}"),
            SignalKind::PotentialDuplicateServiceLines,
            SignalReason::MatchingBeneficiaryProviderServiceDateAndAmount,
            group,
        ));
    }

    signals.sort_by(|left, right| left.signal_key.cmp(&right.signal_key));

    let (status, scope) = match bundle.completeness {
        InputCompleteness::Complete => (
            AnalysisStatus::Complete,
            AnalysisScope::DeclaredCompleteBundle,
        ),
        InputCompleteness::Partial => {
            reasons.push(AnalysisReason::PartialInput);
            (
                AnalysisStatus::Indeterminate,
                AnalysisScope::ObservedRecordsOnly,
            )
        }
        InputCompleteness::Unknown => {
            reasons.push(AnalysisReason::UnknownInputCompleteness);
            (
                AnalysisStatus::Indeterminate,
                AnalysisScope::ObservedRecordsOnly,
            )
        }
    };

    report(
        bundle,
        status,
        scope,
        reasons,
        signals,
        bundle.lines.len(),
    )
}

/// Return true only when a reversal matches the submitted service identity
/// and full amount. Claim IDs may differ because a reversal can be recorded in
/// a separate adjustment claim; the service-level fields must still agree.
fn reversal_matches_target(reversal: &ClaimLineV1, target: &ClaimLineV1) -> bool {
    reversal.beneficiary_ref == target.beneficiary_ref
        && reversal.provider_ref == target.provider_ref
        && reversal.service_code == target.service_code
        && reversal.currency_code == target.currency_code
        && reversal.service_date_yyyymmdd == target.service_date_yyyymmdd
        && reversal.amount_minor_units == target.amount_minor_units
}

fn make_signal(
    bundle: &ClaimEvidenceBundleV1,
    signal_key: String,
    kind: SignalKind,
    reason: SignalReason,
    mut lines: Vec<&ClaimLineV1>,
) -> AnomalySignalV1 {
    lines.sort_by(|left, right| left.line_id.cmp(&right.line_id));
    let evidence = lines
        .into_iter()
        .map(|line| LineEvidenceRefV1 {
            line_id: line.line_id.clone(),
            source_record_digest: line.source_record_digest.clone(),
        })
        .collect();

    AnomalySignalV1 {
        signal_key,
        ruleset_version: RULESET_VERSION.to_string(),
        code_set_version: bundle.code_set_version.clone(),
        input_completeness: bundle.completeness,
        kind,
        reason,
        decision_authority: DecisionAuthority::NonAuthoritativeReviewLead,
        input_manifest_digest: bundle.input_manifest_digest.clone(),
        policy_digest: bundle.policy_digest.clone(),
        evidence,
    }
}

fn report(
    bundle: &ClaimEvidenceBundleV1,
    status: AnalysisStatus,
    scope: AnalysisScope,
    reasons: Vec<AnalysisReason>,
    signals: Vec<AnomalySignalV1>,
    lines_examined: usize,
) -> EvaluationReportV1 {
    EvaluationReportV1 {
        schema_version: SCHEMA_VERSION,
        ruleset_version: RULESET_VERSION.to_string(),
        status,
        scope,
        input_manifest_digest: bundle.input_manifest_digest.clone(),
        policy_digest: bundle.policy_digest.clone(),
        code_set_version: bundle.code_set_version.clone(),
        lines_examined,
        reasons,
        signals,
    }
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn is_valid_currency_code(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_valid_date(value: u32) -> bool {
    let year = value / 10_000;
    let month = (value / 100) % 100;
    let day = value % 100;
    if year == 0 || year > 9999 || !(1..=12).contains(&month) {
        return false;
    }

    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
}

fn reason_sort_key(reason: &AnalysisReason) -> u8 {
    match reason {
        AnalysisReason::UnsupportedSchemaVersion => 0,
        AnalysisReason::InvalidInputManifestDigest => 1,
        AnalysisReason::InvalidPolicyDigest => 2,
        AnalysisReason::InvalidCodeSetVersion => 3,
        AnalysisReason::InvalidCurrencyCode => 4,
        AnalysisReason::LineCountLimitExceeded => 5,
        AnalysisReason::EmptyInput => 6,
        AnalysisReason::DuplicateLineId => 7,
        AnalysisReason::InvalidLineIdentifier => 8,
        AnalysisReason::InvalidServiceDate => 9,
        AnalysisReason::InvalidSourceRecordDigest => 10,
        AnalysisReason::InvalidReversalReference => 11,
        AnalysisReason::PartialInput => 12,
        AnalysisReason::UnknownInputCompleteness => 13,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn line(id: &str) -> ClaimLineV1 {
        ClaimLineV1 {
            line_id: id.to_string(),
            claim_id: format!("claim-{id}"),
            beneficiary_ref: "beneficiary-demo-1".to_string(),
            provider_ref: "provider-demo-1".to_string(),
            service_code: "SVC-100".to_string(),
            currency_code: "USD".to_string(),
            service_date_yyyymmdd: 20261010,
            amount_minor_units: 12_500,
            status: ClaimLineStatus::Submitted,
            reverses_line_id: None,
            source_record_digest: DIGEST_A.to_string(),
        }
    }

    fn bundle(lines: Vec<ClaimLineV1>) -> ClaimEvidenceBundleV1 {
        ClaimEvidenceBundleV1 {
            schema_version: SCHEMA_VERSION,
            input_manifest_digest: DIGEST_A.to_string(),
            policy_digest: DIGEST_B.to_string(),
            code_set_version: "test-codes-v1".to_string(),
            completeness: InputCompleteness::Complete,
            lines,
        }
    }

    #[test]
    fn flags_matching_service_lines_as_review_lead_not_fraud() {
        let report = evaluate(&bundle(vec![line("line-a"), line("line-b")]));
        assert_eq!(report.status, AnalysisStatus::Complete);
        assert_eq!(report.signals.len(), 1);
        let signal = &report.signals[0];
        assert_eq!(signal.kind, SignalKind::PotentialDuplicateServiceLines);
        assert_eq!(signal.line_ids(), vec!["line-a", "line-b"]);
        assert_eq!(
            signal.decision_authority,
            DecisionAuthority::NonAuthoritativeReviewLead
        );
    }

    #[test]
    fn evidence_keeps_each_line_paired_with_its_own_source_digest() {
        let mut first = line("line-a");
        first.source_record_digest = DIGEST_A.to_string();
        let mut second = line("line-b");
        second.source_record_digest = DIGEST_B.to_string();

        let report = evaluate(&bundle(vec![second, first]));
        let signal = &report.signals[0];
        assert_eq!(signal.ruleset_version, RULESET_VERSION);
        assert_eq!(signal.code_set_version, "test-codes-v1");
        assert_eq!(
            signal.evidence,
            vec![
                LineEvidenceRefV1 {
                    line_id: "line-a".to_string(),
                    source_record_digest: DIGEST_A.to_string(),
                },
                LineEvidenceRefV1 {
                    line_id: "line-b".to_string(),
                    source_record_digest: DIGEST_B.to_string(),
                },
            ]
        );
    }

    #[test]
    fn different_amount_date_or_currency_does_not_group() {
        let mut second = line("line-b");
        second.amount_minor_units += 1;
        let mut third = line("line-c");
        third.service_date_yyyymmdd += 1;
        let mut fourth = line("line-d");
        fourth.currency_code = "ZAR".to_string();
        let report = evaluate(&bundle(vec![line("line-a"), second, third, fourth]));
        assert!(report.signals.is_empty());
        assert_eq!(report.status, AnalysisStatus::Complete);
    }

    #[test]
    fn reversal_and_single_replacement_do_not_auto_flag_duplicate() {
        let original = line("line-original");

        let mut reversal = line("line-reversal");
        reversal.status = ClaimLineStatus::Reversal;
        reversal.reverses_line_id = Some("line-original".to_string());
        reversal.source_record_digest = DIGEST_B.to_string();

        let replacement = line("line-replacement");
        let report = evaluate(&bundle(vec![original, reversal, replacement]));
        assert!(report.signals.is_empty());
        assert_eq!(report.status, AnalysisStatus::Complete);
    }

    #[test]
    fn mismatched_reversal_does_not_suppress_duplicate_signal() {
        let original = line("line-original");
        let duplicate = line("line-duplicate");
        let mut reversal = line("line-reversal");
        reversal.status = ClaimLineStatus::Reversal;
        reversal.reverses_line_id = Some("line-original".to_string());
        reversal.amount_minor_units -= 1;
        reversal.source_record_digest = DIGEST_B.to_string();

        let report = evaluate(&bundle(vec![original, duplicate, reversal]));
        assert_eq!(report.status, AnalysisStatus::Complete);
        assert!(report.signals.iter().any(|signal| {
            signal.kind == SignalKind::ReversalTargetMismatch
                && signal.line_ids() == vec!["line-original", "line-reversal"]
        }));
        assert!(report.signals.iter().any(|signal| {
            signal.kind == SignalKind::PotentialDuplicateServiceLines
                && signal.line_ids() == vec!["line-duplicate", "line-original"]
        }));
    }

    #[test]
    fn missing_reversal_target_creates_traceable_signal() {
        let mut reversal = line("line-reversal");
        reversal.status = ClaimLineStatus::Reversal;
        reversal.reverses_line_id = Some("absent-line".to_string());
        let report = evaluate(&bundle(vec![reversal]));
        assert_eq!(report.signals.len(), 1);
        assert_eq!(report.signals[0].kind, SignalKind::ReversalTargetNotFound);
        assert_eq!(report.signals[0].line_ids(), vec!["line-reversal"]);
        assert_eq!(report.status, AnalysisStatus::Complete);
    }

    #[test]
    fn reversal_target_that_is_not_a_submitted_line_is_flagged() {
        let mut target = line("line-target");
        target.status = ClaimLineStatus::Reversal;
        target.reverses_line_id = Some("line-old".to_string());

        let mut reversal = line("line-reversal");
        reversal.status = ClaimLineStatus::Reversal;
        reversal.reverses_line_id = Some("line-target".to_string());

        let report = evaluate(&bundle(vec![target, reversal]));
        assert!(report
            .signals
            .iter()
            .any(|signal| signal.kind == SignalKind::ReversalTargetNotSubmitted));
        assert_eq!(report.status, AnalysisStatus::Complete);
    }

    #[test]
    fn partial_input_never_reports_complete_even_if_no_signals() {
        let mut input = bundle(vec![line("line-a")]);
        input.completeness = InputCompleteness::Partial;
        let report = evaluate(&input);
        assert_eq!(report.status, AnalysisStatus::Indeterminate);
        assert_eq!(report.scope, AnalysisScope::ObservedRecordsOnly);
        assert_eq!(report.reasons, vec![AnalysisReason::PartialInput]);
    }

    #[test]
    fn unknown_completeness_never_reports_complete_even_without_signals() {
        let mut input = bundle(vec![line("line-a")]);
        input.completeness = InputCompleteness::Unknown;
        let report = evaluate(&input);

        assert_eq!(report.status, AnalysisStatus::Indeterminate);
        assert_eq!(report.scope, AnalysisScope::ObservedRecordsOnly);
        assert_eq!(
            report.reasons,
            vec![AnalysisReason::UnknownInputCompleteness]
        );
        assert!(report.signals.is_empty());
    }

    #[test]
    fn invalid_identifier_fails_closed_without_signals() {
        let mut invalid = line("line-a");
        invalid.provider_ref = "provider with spaces".to_string();
        let report = evaluate(&bundle(vec![invalid]));

        assert_eq!(report.status, AnalysisStatus::Indeterminate);
        assert_eq!(report.reasons, vec![AnalysisReason::InvalidLineIdentifier]);
        assert!(report.signals.is_empty());
    }

    #[test]
    fn malformed_digest_fails_closed_without_signals() {
        let mut input = bundle(vec![line("line-a"), line("line-b")]);
        input.policy_digest = "not-a-digest".to_string();
        let report = evaluate(&input);
        assert_eq!(report.status, AnalysisStatus::Indeterminate);
        assert_eq!(report.signals.len(), 0);
        assert!(report.reasons.contains(&AnalysisReason::InvalidPolicyDigest));
    }

    #[test]
    fn duplicate_line_identity_fails_closed() {
        let report = evaluate(&bundle(vec![line("same-id"), line("same-id")]));
        assert_eq!(report.status, AnalysisStatus::Indeterminate);
        assert_eq!(report.reasons, vec![AnalysisReason::DuplicateLineId]);
        assert!(report.signals.is_empty());
    }

    #[test]
    fn invalid_calendar_date_fails_closed() {
        let mut invalid = line("line-a");
        invalid.service_date_yyyymmdd = 20260230;
        let report = evaluate(&bundle(vec![invalid]));
        assert_eq!(report.status, AnalysisStatus::Indeterminate);
        assert!(report.reasons.contains(&AnalysisReason::InvalidServiceDate));
    }

    #[test]
    fn signal_order_is_independent_of_input_line_order() {
        let input_a = bundle(vec![line("line-b"), line("line-a"), line("line-d"), line("line-c")]);
        let input_b = bundle(vec![line("line-c"), line("line-a"), line("line-b"), line("line-d")]);
        assert_eq!(evaluate(&input_a), evaluate(&input_b));
    }

    #[test]
    fn invalid_currency_code_fails_closed() {
        let mut invalid = line("line-a");
        invalid.currency_code = "usd".to_string();
        let report = evaluate(&bundle(vec![invalid]));
        assert_eq!(report.status, AnalysisStatus::Indeterminate);
        assert!(report.reasons.contains(&AnalysisReason::InvalidCurrencyCode));
        assert!(report.signals.is_empty());
    }

    #[test]
    fn empty_input_is_indeterminate_not_a_clean_result() {
        let report = evaluate(&bundle(Vec::new()));
        assert_eq!(report.status, AnalysisStatus::Indeterminate);
        assert!(report.reasons.contains(&AnalysisReason::EmptyInput));
        assert!(report.signals.is_empty());
    }

    #[test]
    fn invalid_schema_fails_closed() {
        let mut input = bundle(vec![line("line-a")]);
        input.schema_version = 99;
        let report = evaluate(&input);
        assert_eq!(report.status, AnalysisStatus::Indeterminate);
        assert!(report.reasons.contains(&AnalysisReason::UnsupportedSchemaVersion));
        assert!(report.signals.is_empty());
    }
}
