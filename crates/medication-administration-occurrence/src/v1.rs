use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use mycelix_fhir_medication_semantics::MedicationRequestArtifact;
use mycelix_medication_semantics::AdministrationTiming;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const SCHEDULE_PLAN_TAG: &[u8] = b"mycelix-health/administration-schedule-plan-v1";
const SCHEDULE_RESOLUTION_TAG: &[u8] = b"mycelix-health/administration-schedule-resolution-v1";
const PRN_INTENT_TAG: &[u8] = b"mycelix-health/administration-prn-intent-v1";
const OCCURRENCE_TAG: &[u8] = b"mycelix-health/administration-occurrence-v1";
const MAX_OCCURRENCES_PER_SEGMENT: usize = 1024;
const MAX_PRN_INTENT_FUTURE_SKEW_MICROS: i64 = 300_000_000;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduledOccurrenceWindowV1 {
    pub occurrence_ordinal: u64,
    pub intended_start_micros: i64,
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

/// Semantic bounded schedule material. Resolver identity/version/time are not
/// included, so recomputing the same exact windows produces the same semantic digest.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdministrationSchedulePlanSegmentV1 {
    pub schema_version: u16,
    pub medication_artifact_digest: StoredDigest,
    pub patient_subject_binding_evidence_digest: StoredDigest,
    pub dosage_index: u32,
    pub occurrences: Vec<ScheduledOccurrenceWindowV1>,
}

impl AdministrationSchedulePlanSegmentV1 {
    pub fn validate_shape(&self) -> Result<(), OccurrenceError> {
        if self.schema_version != 1 {
            return Err(OccurrenceError::UnsupportedSchedulePlanVersion(
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

/// Evidence about a schedule-resolution run. This is intentionally not occurrence
/// identity material; deployments may separately require trusted/fresh provenance.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdministrationScheduleResolutionProvenanceV1 {
    pub schema_version: u16,
    pub schedule_plan_digest: StoredDigest,
    pub resolver_id: String,
    pub resolver_version: String,
    pub generated_at_micros: i64,
    pub source_schedule_evidence_digest: StoredDigest,
    pub timezone_rules_evidence_digest: Option<StoredDigest>,
}

impl AdministrationScheduleResolutionProvenanceV1 {
    pub fn validate_shape(&self) -> Result<(), OccurrenceError> {
        if self.schema_version != 1 {
            return Err(OccurrenceError::UnsupportedScheduleResolutionVersion(
                self.schema_version,
            ));
        }
        require_domain(
            self.schedule_plan_digest,
            DigestDomain::MedicationAdministrationSchedulePlan,
        )?;
        if self.resolver_id.trim().is_empty() {
            return Err(OccurrenceError::MissingResolverId);
        }
        if self.resolver_version.trim().is_empty() {
            return Err(OccurrenceError::MissingResolverVersion);
        }
        require_domain(
            self.source_schedule_evidence_digest,
            DigestDomain::EvidenceCapsule,
        )?;
        if let Some(timezone) = self.timezone_rules_evidence_digest {
            require_domain(timezone, DigestDomain::EvidenceCapsule)?;
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, OccurrenceError> {
        self.validate_shape()?;
        hash_json(
            DigestDomain::MedicationAdministrationScheduleResolution,
            SCHEDULE_RESOLUTION_TAG,
            self,
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PrnAdministrationIntentV1 {
    pub schema_version: u16,
    pub medication_artifact_digest: StoredDigest,
    pub patient_subject_binding_evidence_digest: StoredDigest,
    pub dosage_index: u32,
    pub intent_nonce: [u8; 32],
    pub intent_created_at_micros: i64,
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdministrationOccurrenceKindV1 {
    OneTime,
    Scheduled {
        occurrence_ordinal: u64,
        intended_start_micros: i64,
        intended_end_micros: Option<i64>,
    },
    AsNeeded {
        intent_nonce: [u8; 32],
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
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
                intended_start_micros,
                intended_end_micros,
                ..
            } => {
                if intended_end_micros.is_some_and(|end| end < intended_start_micros) {
                    return Err(OccurrenceError::OccurrenceEndsBeforeStart);
                }
            }
            AdministrationOccurrenceKindV1::AsNeeded { intent_nonce } => {
                if intent_nonce == [0u8; 32] {
                    return Err(OccurrenceError::ZeroPrnIntentNonce);
                }
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

pub fn derive_one_time_occurrence(
    medication: &MedicationRequestArtifact,
    patient_subject_binding_evidence_digest: VerifiedDigest,
    dosage_index: u32,
) -> Result<MedicationAdministrationOccurrenceV1, OccurrenceError> {
    let medication_digest = medication_digest(medication)?;
    patient_subject_binding_evidence_digest
        .require_domain(DigestDomain::PatientSubjectBindingEvidence)?;
    let dosage = dosage(medication, dosage_index)?;
    if !matches!(&dosage.timing, AdministrationTiming::OneTime) {
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
    if !matches!(&dosage.timing, AdministrationTiming::Scheduled { .. }) {
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
            occurrence_ordinal,
            intended_start_micros: window.intended_start_micros,
            intended_end_micros: window.intended_end_micros,
        },
    };
    occurrence.validate_shape()?;
    Ok(occurrence)
}

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
    if !matches!(&dosage.timing, AdministrationTiming::AsNeeded { .. }) {
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

    let occurrence = MedicationAdministrationOccurrenceV1 {
        schema_version: 1,
        medication_artifact_digest: medication_digest.stored(),
        patient_subject_binding_evidence_digest: patient_subject_binding_evidence_digest.stored(),
        dosage_index,
        kind: AdministrationOccurrenceKindV1::AsNeeded {
            intent_nonce: intent.intent_nonce,
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
    #[error("unsupported administration schedule-resolution version {0}")]
    UnsupportedScheduleResolutionVersion(u16),
    #[error("unsupported PRN administration intent version {0}")]
    UnsupportedPrnIntentVersion(u16),
    #[error("unsupported administration occurrence version {0}")]
    UnsupportedOccurrenceVersion(u16),
    #[error("administration schedule plan cannot be empty")]
    EmptySchedulePlan,
    #[error("administration schedule plan exceeds bounded v1 size")]
    SchedulePlanTooLarge,
    #[error("schedule occurrence ordinals must be strictly increasing")]
    ScheduleOrdinalsNotStrictlyIncreasing,
    #[error("schedule occurrence start times must be strictly increasing")]
    ScheduleStartsNotStrictlyIncreasing,
    #[error("administration occurrence ends before it starts")]
    OccurrenceEndsBeforeStart,
    #[error("schedule resolver ID is required")]
    MissingResolverId,
    #[error("schedule resolver version is required")]
    MissingResolverVersion,
    #[error("PRN administration intent nonce cannot be zero")]
    ZeroPrnIntentNonce,
    #[error("dosage timing does not match requested occurrence kind")]
    TimingKindMismatch,
    #[error("schedule plan does not match medication/dosage/patient lineage")]
    SchedulePlanLineageMismatch,
    #[error("requested scheduled occurrence ordinal is absent from the plan")]
    ScheduledOccurrenceMissing,
    #[error("PRN intent does not match medication/dosage/patient lineage")]
    PrnIntentLineageMismatch,
    #[error("PRN intent timestamp is implausibly in the future")]
    PrnIntentFromFuture,
    #[error("dosage index cannot be represented on this platform")]
    DosageIndexOverflow,
    #[error("dosage index is outside the medication order")]
    DosageIndexOutOfBounds,
    #[error("medication artifact validation failed: {0}")]
    MedicationArtifact(String),
    #[error("stored digest is in wrong domain: expected {expected:?}, got {actual:?}")]
    WrongDigestDomain {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("occurrence artifact serialization failed: {0}")]
    Serialization(String),
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

    fn plan() -> AdministrationSchedulePlanSegmentV1 {
        AdministrationSchedulePlanSegmentV1 {
            schema_version: 1,
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            patient_subject_binding_evidence_digest: stored(
                DigestDomain::PatientSubjectBindingEvidence,
                2,
            ),
            dosage_index: 0,
            occurrences: vec![
                ScheduledOccurrenceWindowV1 {
                    occurrence_ordinal: 10,
                    intended_start_micros: 1_000,
                    intended_end_micros: None,
                },
                ScheduledOccurrenceWindowV1 {
                    occurrence_ordinal: 11,
                    intended_start_micros: 2_000,
                    intended_end_micros: Some(2_100),
                },
            ],
        }
    }

    #[test]
    fn semantic_schedule_digest_is_stable_across_resolution_provenance() {
        let plan = plan();
        let plan_digest = plan.verified_digest().unwrap();
        let first = AdministrationScheduleResolutionProvenanceV1 {
            schema_version: 1,
            schedule_plan_digest: plan_digest.stored(),
            resolver_id: "resolver-a".into(),
            resolver_version: "1.0".into(),
            generated_at_micros: 10,
            source_schedule_evidence_digest: stored(DigestDomain::EvidenceCapsule, 3),
            timezone_rules_evidence_digest: None,
        };
        let second = AdministrationScheduleResolutionProvenanceV1 {
            schema_version: 1,
            schedule_plan_digest: plan_digest.stored(),
            resolver_id: "resolver-b".into(),
            resolver_version: "2.0".into(),
            generated_at_micros: 20,
            source_schedule_evidence_digest: stored(DigestDomain::EvidenceCapsule, 4),
            timezone_rules_evidence_digest: Some(stored(DigestDomain::EvidenceCapsule, 5)),
        };
        assert_eq!(plan.verified_digest().unwrap(), plan_digest);
        assert_ne!(first.verified_digest().unwrap(), second.verified_digest().unwrap());
    }

    #[test]
    fn scheduled_window_change_changes_occurrence_identity() {
        let first = MedicationAdministrationOccurrenceV1 {
            schema_version: 1,
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            patient_subject_binding_evidence_digest: stored(
                DigestDomain::PatientSubjectBindingEvidence,
                2,
            ),
            dosage_index: 0,
            kind: AdministrationOccurrenceKindV1::Scheduled {
                occurrence_ordinal: 10,
                intended_start_micros: 1_000,
                intended_end_micros: None,
            },
        };
        let mut second = first;
        second.kind = AdministrationOccurrenceKindV1::Scheduled {
            occurrence_ordinal: 10,
            intended_start_micros: 1_001,
            intended_end_micros: None,
        };
        assert_ne!(first.verified_digest().unwrap(), second.verified_digest().unwrap());
    }

    #[test]
    fn same_prn_nonce_has_stable_occurrence_identity() {
        let first = MedicationAdministrationOccurrenceV1 {
            schema_version: 1,
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            patient_subject_binding_evidence_digest: stored(
                DigestDomain::PatientSubjectBindingEvidence,
                2,
            ),
            dosage_index: 0,
            kind: AdministrationOccurrenceKindV1::AsNeeded {
                intent_nonce: [9; 32],
            },
        };
        let second = first;
        assert_eq!(first.verified_digest().unwrap(), second.verified_digest().unwrap());
    }

    #[test]
    fn zero_prn_nonce_fails_closed() {
        let value = MedicationAdministrationOccurrenceV1 {
            schema_version: 1,
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            patient_subject_binding_evidence_digest: stored(
                DigestDomain::PatientSubjectBindingEvidence,
                2,
            ),
            dosage_index: 0,
            kind: AdministrationOccurrenceKindV1::AsNeeded {
                intent_nonce: [0; 32],
            },
        };
        assert_eq!(value.validate_shape(), Err(OccurrenceError::ZeroPrnIntentNonce));
    }

    #[test]
    fn schedule_ordinals_must_be_strictly_increasing() {
        let mut value = plan();
        value.occurrences[1].occurrence_ordinal = 10;
        assert_eq!(
            value.validate_shape(),
            Err(OccurrenceError::ScheduleOrdinalsNotStrictlyIncreasing)
        );
    }
}
