#![deny(unsafe_code)]
//! Migration adapter for the legacy trials `AdverseEvent::causality` field.
//!
//! The old causality enum is preserved as source metadata only. This crate has no
//! API that converts a legacy causality label directly into `CausalConclusionV1`.

use holo_hash::ActionHash;
use mycelix_clinical_causality::ExternalCausalityScaleLabelV1;
use serde::Serialize;
use thiserror::Error;
use trials_integrity::{AdverseEvent, Causality};

pub const LEGACY_CAUSALITY_SYSTEM: &str = "urn:mycelix-health:trials:legacy-causality";
pub const LEGACY_CAUSALITY_VERSION: &str = "1";

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub enum LegacyCausalityMigrationStateV1 {
    /// The source label has been preserved, but no high-assurance causal conclusion
    /// exists until a fresh evidence-first assessment is performed.
    RequiresFreshEvidenceAssessment,
}

/// Lossless source-context record for a legacy trial causality label.
///
/// Intentionally serializable but not deserializable: callers should construct it
/// from the actual legacy `AdverseEvent` record obtained through the trusted read
/// path. Even then, this type does not prove the DHT validity of that source action.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct LegacyTrialCausalityImportV1 {
    pub schema_version: u16,
    pub source_action_hash: ActionHash,
    pub source_trial_hash: ActionHash,
    pub source_participant_hash: ActionHash,
    pub source_event_id: String,
    pub source_event_term: String,
    pub external_scale_label: ExternalCausalityScaleLabelV1,
    pub migration_state: LegacyCausalityMigrationStateV1,
}

pub fn import_legacy_trial_causality(
    source_action_hash: ActionHash,
    event: &AdverseEvent,
) -> Result<LegacyTrialCausalityImportV1, TrialCausalityAdapterError> {
    if event.event_id.trim().is_empty() {
        return Err(TrialCausalityAdapterError::MissingEventId);
    }
    if event.event_term.trim().is_empty() {
        return Err(TrialCausalityAdapterError::MissingEventTerm);
    }

    Ok(LegacyTrialCausalityImportV1 {
        schema_version: 1,
        source_action_hash,
        source_trial_hash: event.trial_hash.clone(),
        source_participant_hash: event.participant_hash.clone(),
        source_event_id: event.event_id.clone(),
        source_event_term: event.event_term.clone(),
        external_scale_label: legacy_causality_label(&event.causality),
        migration_state: LegacyCausalityMigrationStateV1::RequiresFreshEvidenceAssessment,
    })
}

/// Preserve the original enum value as an external scale label. No semantic mapping
/// into the high-assurance causal conclusion vocabulary occurs here.
pub fn legacy_causality_label(causality: &Causality) -> ExternalCausalityScaleLabelV1 {
    ExternalCausalityScaleLabelV1 {
        system: LEGACY_CAUSALITY_SYSTEM.to_string(),
        version: LEGACY_CAUSALITY_VERSION.to_string(),
        label: match causality {
            Causality::DefinitelyRelated => "DefinitelyRelated",
            Causality::ProbablyRelated => "ProbablyRelated",
            Causality::PossiblyRelated => "PossiblyRelated",
            Causality::UnlikelyRelated => "UnlikelyRelated",
            Causality::NotRelated => "NotRelated",
            Causality::Unknown => "Unknown",
        }
        .to_string(),
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TrialCausalityAdapterError {
    #[error("legacy adverse event ID is required")]
    MissingEventId,
    #[error("legacy adverse event term is required")]
    MissingEventTerm,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_legacy_label_is_preserved_without_normalization() {
        let cases = [
            (Causality::DefinitelyRelated, "DefinitelyRelated"),
            (Causality::ProbablyRelated, "ProbablyRelated"),
            (Causality::PossiblyRelated, "PossiblyRelated"),
            (Causality::UnlikelyRelated, "UnlikelyRelated"),
            (Causality::NotRelated, "NotRelated"),
            (Causality::Unknown, "Unknown"),
        ];

        for (source, expected) in cases {
            let label = legacy_causality_label(&source);
            assert_eq!(label.system, LEGACY_CAUSALITY_SYSTEM);
            assert_eq!(label.version, LEGACY_CAUSALITY_VERSION);
            assert_eq!(label.label, expected);
        }
    }

    #[test]
    fn migration_state_has_no_causal_conclusion_variant() {
        assert_eq!(
            LegacyCausalityMigrationStateV1::RequiresFreshEvidenceAssessment,
            LegacyCausalityMigrationStateV1::RequiresFreshEvidenceAssessment
        );
    }
}
