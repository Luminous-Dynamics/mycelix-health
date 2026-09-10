#![deny(unsafe_code)]
//! Computable medication-administration occurrence identities.
//!
//! An administration record ID is display/audit metadata. It is not the identity of
//! the intended clinical dose occurrence. This crate derives a separate occurrence
//! digest from exact order intent without using activation, dispensing, recording
//! timestamps, or caller-selected administration IDs.
//!
//! V1 intentionally refuses to infer scheduled occurrences from wall-clock rounding.
//! Scheduled doses require a bounded resolved schedule-plan segment. PRN doses require
//! an explicit intent artifact with a nonzero nonce.

use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use mycelix_fhir_medication_semantics::MedicationRequestArtifact;
use mycelix_medication_semantics::AdministrationTiming;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const SCHEDULE_PLAN_TAG: &[u8] = b"mycelix-health/administration-schedule-plan-v1";
const PRN_INTENT_TAG: &[u8] = b"mycelix-health/administration-prn-intent-v1";
const OCCURRENCE_TAG: &[u8] = b"mycelix-health/administration-occurrence-v1";
const MAX_OCCURRENCES_PER_SEGMENT: usize = 1024;
const MAX_PRN_INTENT_FUTURE_SKEW_MICROS: i64 = 300_000_000;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduledOccurrenceWindowV1 {
    /// Stable ordinal within the exact MedicationRequest + dosage instruction.
    pub occurrence_ordinal: u64,
    /// Canonical UTC microsecond boundary resolved from the source schedule semantics.
    pub intended_start_micros: i64,
    /// Optional end boundary for a duration-bearing administration occurrence.
    pub intended_end_micros: Option<i64>,
}

impl ScheduledOccurrenceWindowV1 {
    fn validate(&self) -> Result<(), OccurrenceError> {
        if self
            .intended_end_micros
            .is_some_and(|end| end < self.intended_start_micros)
        {
            return Err(OccurrenceError::OccurrenceEndsBeforeStart);
        }
        Ok(())
    }
}

/// Bounded exact schedule material produced by a schedule resolver.
///
/// The segment contains canonical UTC occurrence windows, so downstream identity does
/// not recompute timezone/DST rules. `source_schedule_evidence_digest` identifies the
/// exact resolver/ruleset evidence used to produce those windows.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdministrationSchedulePlanSegmentV1 {
    pub schema_version: u16,
    pub plan_id: String,
    pub medication_artifact_digest: StoredDigest,
    pub patient_subject_binding_evidence_digest: StoredDigest,
    pub dosage_index: u32,
    pub segment_index: u32,
    pub source_schedule_evidence_digest: StoredDigest,
    pub generated_at_micros: i64,
    pub occurrences: Vec<ScheduledOccurrenceWindowV1>,
}

impl AdministrationSchedulePlanSegmentV1 {
    pub fn validate_shape(&self) -> Result<(), OccurrenceError> {
        if self.schema_version != 1 {
            return Err(OccurrenceError::UnsupportedSchedulePlanVersion(
                self.schema_version,
            ));
        }
        if self.plan_id.trim().is_empty() {
            return Err(OccurrenceError::MissingPlanId);
        }
        require_domain(
            self.medication_artifact_digest,
            DigestDomain::MedicationRequestArtifact,
        )?;
        require_domain(
            self.patient_subject_binding_evidence_digest,
            DigestDomain::PatientSubjectBindingEvidence,
        )?;
        require_domain(
            self.source_schedule_evidence_digest,
            DigestDomain::EvidenceCapsule,
        )?;
        if self.occurrences.is_empty() {
            return Err(OccurrenceError::EmptySchedulePlan);
        }
        if self.occurrences.len() > MAX_OCCURRENCES_PER_SEGMENT {
            return Err(OccurrenceError::SchedulePlanTooLarge);
        }

        let mut previous_ordinal = None;
        let mut previous_start = None;
        for occurrence in &self.occurrences {
            occurrence.validate()?;
            if previous_ordinal.is_some_and(|value| occurrence.occurrence_ordinal <= value) {
                return Err(OccurrenceError::ScheduleOrdinalsNotStrictlyIncreasing);
            }
            if previous_start.is_some_and(|value| occurrence.intended_start_micros <= value) {
                return Err(OccurrenceError::ScheduleStartsNotStrictlyIncreasing);
            }
            previous_ordinal = Some(occurrence.occurrence_ordinal);
            previous_start = Some(occurrence.intended_start_micros);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, OccurrenceError> {
        self.validate_shape()?;
        hash_json(
            DigestDomain::MedicationAdministrationSchedulePlan,
            SCHEDULE_PLAN_TAG,
            self,
        )
    }
}

/// Explicit PRN intent. A new clinical intent gets a new nonzero nonce; the wall clock
/// alone is never used as PRN occurrence identity.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PrnAdministrationIntentV1 {
    pub schema_version: u16,
    pub medication_artifact_digest: StoredDigest,
    pub patient_subject_binding_evidence_digest: StoredDigest,
    pub dosage_index: u32,
    pub intent_nonce: [u8; 32],
    pub intent_created_at_micros: i64,
    /// Optional evidence for the clinical reason/trigger that caused this PRN intent.
    pub reason_evidence_digest: Option<StoredDigest>,
}

impl PrnAdministrationIntentV1 {
    pub fn validate_shape(&self) -> Result<(), OccurrenceError> {
        if self.schema_version != 1 {
            return Err(OccurrenceError::UnsupportedPrnIntentVersion(
                self.schema_version,
            ));
        }
        require_domain(
            self.medication_artifact_digest,
            DigestDomain::MedicationRequestArtifact,
        )?;
        require_domain(
            self.patient_subject_binding_evidence_digest,
            DigestDomain::PatientSubjectBindingEvidence,
        )?;
        if self.intent_nonce == [0u8; 32] {
            return Err(OccurrenceError::ZeroPrnIntentNonce);
        }
        if let Some(reason) = self.reason_evidence_digest {
            require_domain(reason, DigestDomain::EvidenceCapsule)?;
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, OccurrenceError> {
        self.validate_shape()?;
        hash_json(
            DigestDomain::MedicationAdministrationPrnIntent,
            PRN_INTENT_TAG,
            self,
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdministrationOccurrenceKindV1 {
    /// Exact MedicationRequest + dosage index + subject binding defines one occurrence.
    OneTime,
    /// Exact bounded schedule plan + ordinal/window defines the occurrence.
    Scheduled {
        schedule_plan_digest: StoredDigest,
        occurrence_ordinal: u64,
        intended_start_micros: i64,
        intended_end_micros: Option<i64>,
    },
    /// Exact PRN intent artifact defines the occurrence.
    AsNeeded {
        prn_intent_digest: StoredDigest,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MedicationAdministrationOccurrenceV1 {
    pub schema_version: u16,
    pub medication_artifact_digest: StoredDigest,
    pub patient_subject_binding_evidence_digest: StoredDigest,
    pub dosage_index: u32,
    pub kind: AdministrationOccurrenceKindV1,
}

impl MedicationAdministrationOccurrenceV1 {
    pub fn validate_shape(&self) -> Result<(), OccurrenceError> {
        if self.schema_version != 1 {
            return Err(OccurrenceError::UnsupportedOccurrenceVersion(
                self.schema_version,
            ));
        }
        require_domain(
            self.medication_artifact_digest,
            DigestDomain::MedicationRequestArtifact,
        )?;
        require_domain(
            self.patient_subject_binding_evidence_digest,
            DigestDomain::PatientSubjectBindingEvidence,
        )?;
        match self.kind {
            AdministrationOccurrenceKindV1::OneTime => {}
            AdministrationOccurrenceKindV1::Scheduled {
                schedule_plan_digest,
                intended_start_micros,
                intended_end_micros,
                ..
            } => {
                require_domain(
                    schedule_plan_digest,
                    DigestDomain::MedicationAdministrationSchedulePlan,
                )?;
                if intended_end_micros.is_some_and(|end| end < intended_start_micros) {
                    return Err(OccurrenceError::OccurrenceEndsBeforeStart);
                }
            }
            AdministrationOccurrenceKindV1::AsNeeded { prn_intent_digest } => {
                require_domain(
                    prn_intent_digest,
                    DigestDomain::MedicationAdministrationPrnIntent,
                )?;
            }
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, OccurrenceError> {
        self.validate_shape()?;
        hash_json(
            DigestDomain::MedicationAdministrationOccurrence,
            OCCURRENCE_TAG,
            self,
        )
    }
}

/// Derive the unique v1 occurrence for a one-time dosage instruction.
pub fn derive_one_time_occurrence(
    medication: &MedicationRequestArtifact,
    patient_subject_binding_evidence_digest: VerifiedDigest,
    dosage_index: u32,
) -> Result<MedicationAdministrationOccurrenceV1, OccurrenceError> {
    let medication_digest = medication_digest(medication)?;
    patient_subject_binding_evidence_digest
        .require_domain(DigestDomain::PatientSubjectBindingEvidence)?;
    let dosage = dosage(medication, dosage_index)?;
    if !matches!(dosage.timing, AdministrationTiming::OneTime) {
        return Err(OccurrenceError::TimingKindMismatch);
    }
    let occurrence = MedicationAdministrationOccurrenceV1 {
        schema_version: 1,
        medication_artifact_digest: medication_digest.stored(),
        patient_subject_binding_evidence_digest: patient_subject_binding_evidence_digest.stored(),
        dosage_index,
        kind: AdministrationOccurrenceKindV1::OneTime,
    };
    occurrence.validate_shape()?;
    Ok(occurrence)
}

/// Validate a bounded schedule plan against the exact medication/dosage/subject and
/// derive one occurrence by stable ordinal. The schedule plan itself, rather than a
/// local clock calculation, commits to the intended UTC window.
pub fn derive_scheduled_occurrence(
    medication: &MedicationRequestArtifact,
    patient_subject_binding_evidence_digest: VerifiedDigest,
    dosage_index: u32,
    plan: &AdministrationSchedulePlanSegmentV1,
    occurrence_ordinal: u64,
) -> Result<MedicationAdministrationOccurrenceV1, OccurrenceError> {
    let medication_digest = medication_digest(medication)?;
    patient_subject_binding_evidence_digest
        .require_domain(DigestDomain::PatientSubjectBindingEvidence)?;
    let dosage = dosage(medication, dosage_index)?;
    if !matches!(dosage.timing, AdministrationTiming::Scheduled { .. }) {
        return Err(OccurrenceError::TimingKindMismatch);
    }

    plan.validate_shape()?;
    if plan.medication_artifact_digest != medication_digest.stored()
        || plan.patient_subject_binding_evidence_digest
            != patient_subject_binding_evidence_digest.stored()
        || plan.dosage_index != dosage_index
    {
        return Err(OccurrenceError::SchedulePlanLineageMismatch);
    }
    let plan_digest = plan.verified_digest()?;
    let window = plan
        .occurrences
        .iter()
        .find(|value| value.occurrence_ordinal == occurrence_ordinal)
        .ok_or(OccurrenceError::ScheduledOccurrenceMissing)?;

    let occurrence = MedicationAdministrationOccurrenceV1 {
        schema_version: 1,
        medication_artifact_digest: medication_digest.stored(),
        patient_subject_binding_evidence_digest: patient_subject_binding_evidence_digest.stored(),
        dosage_index,
        kind: AdministrationOccurrenceKindV1::Scheduled {
            schedule_plan_digest: plan_digest.stored(),
            occurrence_ordinal,
            intended_start_micros: window.intended_start_micros,
            intended_end_micros: window.intended_end_micros,
        },
    };
    occurrence.validate_shape()?;
    Ok(occurrence)
}

/// Derive a PRN occurrence from an explicit intent artifact. `execution_at_micros` is
/// used only for a bounded future-clock check; it is not part of occurrence identity.
pub fn derive_prn_occurrence(
    medication: &MedicationRequestArtifact,
    patient_subject_binding_evidence_digest: VerifiedDigest,
    dosage_index: u32,
    intent: &PrnAdministrationIntentV1,
    execution_at_micros: i64,
) -> Result<MedicationAdministrationOccurrenceV1, OccurrenceError> {
    let medication_digest = medication_digest(medication)?;
    patient_subject_binding_evidence_digest
        .require_domain(DigestDomain::PatientSubjectBindingEvidence)?;
    let dosage = dosage(medication, dosage_index)?;
    if !matches!(dosage.timing, AdministrationTiming::AsNeeded { .. }) {
        return Err(OccurrenceError::TimingKindMismatch);
    }

    intent.validate_shape()?;
    if intent.medication_artifact_digest != medication_digest.stored()
        || intent.patient_subject_binding_evidence_digest
            != patient_subject_binding_evidence_digest.stored()
        || intent.dosage_index != dosage_index
    {
        return Err(OccurrenceError::PrnIntentLineageMismatch);
    }
    if intent.intent_created_at_micros
        > execution_at_micros.saturating_add(MAX_PRN_INTENT_FUTURE_SKEW_MICROS)
    {
        return Err(OccurrenceError::PrnIntentFromFuture);
    }
    let intent_digest = intent.verified_digest()?;
    let occurrence = MedicationAdministrationOccurrenceV1 {
        schema_version: 1,
        medication_artifact_digest: medication_digest.stored(),
        patient_subject_binding_evidence_digest: patient_subject_binding_evidence_digest.stored(),
        dosage_index,
        kind: AdministrationOccurrenceKindV1::AsNeeded {
            prn_intent_digest: intent_digest.stored(),
        },
    };
    occurrence.validate_shape()?;
    Ok(occurrence)
}

fn medication_digest(
    medication: &MedicationRequestArtifact,
) -> Result<VerifiedDigest, OccurrenceError> {
    medication
        .verified_digest()
        .map_err(|error| OccurrenceError::MedicationArtifact(error.to_string()))?
        .require_domain(DigestDomain::MedicationRequestArtifact)
        .map_err(OccurrenceError::from)
}

fn dosage(
    medication: &MedicationRequestArtifact,
    dosage_index: u32,
) -> Result<&mycelix_medication_semantics::DosageInstruction, OccurrenceError> {
    let index = usize::try_from(dosage_index).map_err(|_| OccurrenceError::DosageIndexOverflow)?;
    medication
        .order()
        .dosage
        .get(index)
        .ok_or(OccurrenceError::DosageIndexOutOfBounds)
}

fn require_domain(digest: StoredDigest, expected: DigestDomain) -> Result<(), OccurrenceError> {
    digest.validate_shape()?;
    if digest.domain != expected {
        return Err(OccurrenceError::WrongDigestDomain {
            expected,
            actual: digest.domain,
        });
    }
    Ok(())
}

fn hash_json<T: Serialize>(
    domain: DigestDomain,
    tag: &[u8],
    value: &T,
) -> Result<VerifiedDigest, OccurrenceError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| OccurrenceError::Serialization(error.to_string()))?;
    let mut framed = Vec::with_capacity(tag.len() + 1 + encoded.len());
    framed.extend_from_slice(tag);
    framed.push(0);
    framed.extend_from_slice(&encoded);
    Ok(hash_canonical_bytes(domain, &framed)?)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OccurrenceError {
    #[error("unsupported administration schedule-plan version {0}")]
    UnsupportedSchedulePlanVersion(u16),
    #[error("unsupported PRN administration intent version {0}")]
    UnsupportedPrnIntentVersion(u16),
    #[error("unsupported administration occurrence version {0}")]
    UnsupportedOccurrenceVersion(u16),
    #[error("administration schedule plan ID is required")]
    MissingPlanId,
    #[error("administration schedule plan cannot be empty")]
    EmptySchedulePlan,
    #[error("administration schedule plan exceeds bounded segment size")]
    SchedulePlanTooLarge,
    #[error("schedule occurrence ordinals must be strictly increasing")]
    ScheduleOrdinalsNotStrictlyIncreasing,
    #[error("schedule occurrence start times must be strictly increasing")]
    ScheduleStartsNotStrictlyIncreasing,
    #[error("administration occurrence ends before it starts")]
    OccurrenceEndsBeforeStart,
    #[error("PRN administration intent nonce cannot be zero")]
    ZeroPrnIntentNonce,
    #[error("PRN administration intent appears too far in the future")]
    PrnIntentFromFuture,
    #[error("medication dosage index overflow")]
    DosageIndexOverflow,
    #[error("medication dosage index is out of bounds")]
    DosageIndexOutOfBounds,
    #[error("requested occurrence kind does not match the exact dosage timing kind")]
    TimingKindMismatch,
    #[error("resolved schedule plan does not match medication/dosage/subject lineage")]
    SchedulePlanLineageMismatch,
    #[error("requested scheduled occurrence ordinal is absent from the exact plan segment")]
    ScheduledOccurrenceMissing,
    #[error("PRN intent does not match medication/dosage/subject lineage")]
    PrnIntentLineageMismatch,
    #[error("medication artifact error: {0}")]
    MedicationArtifact(String),
    #[error("occurrence serialization failed: {0}")]
    Serialization(String),
    #[error("digest domain mismatch: expected {expected:?}, got {actual:?}")]
    WrongDigestDomain {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_integrity::DigestAlgorithm;

    fn stored(domain: DigestDomain, seed: u8) -> StoredDigest {
        StoredDigest {
            algorithm: DigestAlgorithm::Blake3_256,
            domain,
            value: [seed; 32],
        }
    }

    #[test]
    fn schedule_plan_rejects_duplicate_or_backwards_ordinals() {
        let plan = AdministrationSchedulePlanSegmentV1 {
            schema_version: 1,
            plan_id: "plan-a".into(),
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            patient_subject_binding_evidence_digest: stored(
                DigestDomain::PatientSubjectBindingEvidence,
                2,
            ),
            dosage_index: 0,
            segment_index: 0,
            source_schedule_evidence_digest: stored(DigestDomain::EvidenceCapsule, 3),
            generated_at_micros: 10,
            occurrences: vec![
                ScheduledOccurrenceWindowV1 {
                    occurrence_ordinal: 1,
                    intended_start_micros: 100,
                    intended_end_micros: None,
                },
                ScheduledOccurrenceWindowV1 {
                    occurrence_ordinal: 1,
                    intended_start_micros: 200,
                    intended_end_micros: None,
                },
            ],
        };
        assert_eq!(
            plan.validate_shape(),
            Err(OccurrenceError::ScheduleOrdinalsNotStrictlyIncreasing)
        );
    }

    #[test]
    fn occurrence_identity_does_not_include_administration_id_or_activation() {
        let occurrence = MedicationAdministrationOccurrenceV1 {
            schema_version: 1,
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            patient_subject_binding_evidence_digest: stored(
                DigestDomain::PatientSubjectBindingEvidence,
                2,
            ),
            dosage_index: 0,
            kind: AdministrationOccurrenceKindV1::OneTime,
        };
        let first = occurrence.verified_digest().unwrap();
        let second = occurrence.verified_digest().unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn prn_nonce_changes_occurrence_identity() {
        let first = PrnAdministrationIntentV1 {
            schema_version: 1,
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            patient_subject_binding_evidence_digest: stored(
                DigestDomain::PatientSubjectBindingEvidence,
                2,
            ),
            dosage_index: 0,
            intent_nonce: [3; 32],
            intent_created_at_micros: 10,
            reason_evidence_digest: None,
        };
        let mut second = first.clone();
        second.intent_nonce = [4; 32];
        assert_ne!(
            first.verified_digest().unwrap(),
            second.verified_digest().unwrap()
        );
    }

    #[test]
    fn schedule_plan_digest_commits_to_resolved_window() {
        let first = AdministrationSchedulePlanSegmentV1 {
            schema_version: 1,
            plan_id: "plan-a".into(),
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            patient_subject_binding_evidence_digest: stored(
                DigestDomain::PatientSubjectBindingEvidence,
                2,
            ),
            dosage_index: 0,
            segment_index: 0,
            source_schedule_evidence_digest: stored(DigestDomain::EvidenceCapsule, 3),
            generated_at_micros: 10,
            occurrences: vec![ScheduledOccurrenceWindowV1 {
                occurrence_ordinal: 0,
                intended_start_micros: 100,
                intended_end_micros: None,
            }],
        };
        let mut second = first.clone();
        second.occurrences[0].intended_start_micros = 101;
        assert_ne!(
            first.verified_digest().unwrap(),
            second.verified_digest().unwrap()
        );
    }
}
