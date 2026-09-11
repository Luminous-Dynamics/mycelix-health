#![deny(unsafe_code)]
//! Evidence binding between one qualified administration receipt and one computable
//! clinical administration occurrence.
//!
//! This is an additive migration layer: existing administration event/receipt schemas
//! remain stable, while a separate binding proves which exact occurrence they describe.
//! A serialized binding is still only evidence; DHT/runtime admission remains separate.

use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use mycelix_medication_administration::{
    AdministrationStatusV1, MedicationAdministrationEventV1, MedicationAdministrationReceiptV1,
};
use mycelix_medication_administration_occurrence::{
    AdministrationOccurrenceKindV1, AdministrationSchedulePlanSegmentV1,
    AdministrationScheduleResolutionProvenanceV1, MedicationAdministrationOccurrenceV1,
    OccurrenceError, PrnAdministrationIntentV1,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const BINDING_TAG: &[u8] = b"mycelix-health/medication-administration-occurrence-binding-v1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OccurrenceEvidenceV1 {
    OneTime,
    Scheduled {
        schedule_plan_digest: StoredDigest,
        schedule_resolution_provenance_digest: StoredDigest,
    },
    AsNeeded {
        prn_intent_digest: StoredDigest,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MedicationAdministrationOccurrenceBindingV1 {
    pub schema_version: u16,
    pub administration_receipt_digest: StoredDigest,
    pub event_digest: StoredDigest,
    pub occurrence_digest: StoredDigest,
    pub medication_artifact_digest: StoredDigest,
    pub patient_subject_binding_evidence_digest: StoredDigest,
    pub dosage_index: u32,
    pub occurrence_evidence: OccurrenceEvidenceV1,
}

impl MedicationAdministrationOccurrenceBindingV1 {
    pub fn validate_shape(&self) -> Result<(), BindingError> {
        if self.schema_version != 1 {
            return Err(BindingError::UnsupportedBindingVersion(self.schema_version));
        }
        require_domain(
            self.administration_receipt_digest,
            DigestDomain::MedicationAdministrationReceipt,
        )?;
        require_domain(self.event_digest, DigestDomain::MedicationAdministrationEvent)?;
        require_domain(
            self.occurrence_digest,
            DigestDomain::MedicationAdministrationOccurrence,
        )?;
        require_domain(
            self.medication_artifact_digest,
            DigestDomain::MedicationRequestArtifact,
        )?;
        require_domain(
            self.patient_subject_binding_evidence_digest,
            DigestDomain::PatientSubjectBindingEvidence,
        )?;
        match self.occurrence_evidence {
            OccurrenceEvidenceV1::OneTime => {}
            OccurrenceEvidenceV1::Scheduled {
                schedule_plan_digest,
                schedule_resolution_provenance_digest,
            } => {
                require_domain(
                    schedule_plan_digest,
                    DigestDomain::MedicationAdministrationSchedulePlan,
                )?;
                require_domain(
                    schedule_resolution_provenance_digest,
                    DigestDomain::MedicationAdministrationScheduleResolution,
                )?;
            }
            OccurrenceEvidenceV1::AsNeeded { prn_intent_digest } => {
                require_domain(
                    prn_intent_digest,
                    DigestDomain::MedicationAdministrationPrnIntent,
                )?;
            }
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, BindingError> {
        self.validate_shape()?;
        hash_json(
            DigestDomain::MedicationAdministrationOccurrenceBinding,
            BINDING_TAG,
            self,
        )
    }
}

pub fn bind_one_time_occurrence(
    event: &MedicationAdministrationEventV1,
    receipt: &MedicationAdministrationReceiptV1,
    occurrence: &MedicationAdministrationOccurrenceV1,
) -> Result<MedicationAdministrationOccurrenceBindingV1, BindingError> {
    if !matches!(occurrence.kind, AdministrationOccurrenceKindV1::OneTime) {
        return Err(BindingError::OccurrenceEvidenceKindMismatch);
    }
    bind_common(event, receipt, occurrence, OccurrenceEvidenceV1::OneTime)
}

pub fn bind_scheduled_occurrence(
    event: &MedicationAdministrationEventV1,
    receipt: &MedicationAdministrationReceiptV1,
    occurrence: &MedicationAdministrationOccurrenceV1,
    plan: &AdministrationSchedulePlanSegmentV1,
    provenance: &AdministrationScheduleResolutionProvenanceV1,
) -> Result<MedicationAdministrationOccurrenceBindingV1, BindingError> {
    let (ordinal, intended_start_micros, intended_end_micros) = match occurrence.kind {
        AdministrationOccurrenceKindV1::Scheduled {
            occurrence_ordinal,
            intended_start_micros,
            intended_end_micros,
        } => (occurrence_ordinal, intended_start_micros, intended_end_micros),
        _ => return Err(BindingError::OccurrenceEvidenceKindMismatch),
    };

    plan.validate_shape()?;
    let plan_digest = plan.verified_digest()?;
    if plan.medication_artifact_digest != occurrence.medication_artifact_digest
        || plan.patient_subject_binding_evidence_digest
            != occurrence.patient_subject_binding_evidence_digest
        || plan.dosage_index != occurrence.dosage_index
    {
        return Err(BindingError::SchedulePlanLineageMismatch);
    }
    let window = plan
        .occurrences
        .iter()
        .find(|candidate| candidate.occurrence_ordinal == ordinal)
        .ok_or(BindingError::ScheduledOccurrenceMissing)?;
    if window.intended_start_micros != intended_start_micros
        || window.intended_end_micros != intended_end_micros
    {
        return Err(BindingError::ScheduledOccurrenceWindowMismatch);
    }

    provenance.validate_shape()?;
    if provenance.schedule_plan_digest != plan_digest.stored() {
        return Err(BindingError::ScheduleResolutionPlanMismatch);
    }
    let provenance_digest = provenance.verified_digest()?;

    bind_common(
        event,
        receipt,
        occurrence,
        OccurrenceEvidenceV1::Scheduled {
            schedule_plan_digest: plan_digest.stored(),
            schedule_resolution_provenance_digest: provenance_digest.stored(),
        },
    )
}

pub fn bind_prn_occurrence(
    event: &MedicationAdministrationEventV1,
    receipt: &MedicationAdministrationReceiptV1,
    occurrence: &MedicationAdministrationOccurrenceV1,
    intent: &PrnAdministrationIntentV1,
) -> Result<MedicationAdministrationOccurrenceBindingV1, BindingError> {
    let nonce = match occurrence.kind {
        AdministrationOccurrenceKindV1::AsNeeded { intent_nonce } => intent_nonce,
        _ => return Err(BindingError::OccurrenceEvidenceKindMismatch),
    };
    intent.validate_shape()?;
    if intent.medication_artifact_digest != occurrence.medication_artifact_digest
        || intent.patient_subject_binding_evidence_digest
            != occurrence.patient_subject_binding_evidence_digest
        || intent.dosage_index != occurrence.dosage_index
        || intent.intent_nonce != nonce
    {
        return Err(BindingError::PrnIntentLineageMismatch);
    }
    let intent_digest = intent.verified_digest()?;
    bind_common(
        event,
        receipt,
        occurrence,
        OccurrenceEvidenceV1::AsNeeded {
            prn_intent_digest: intent_digest.stored(),
        },
    )
}

fn bind_common(
    event: &MedicationAdministrationEventV1,
    receipt: &MedicationAdministrationReceiptV1,
    occurrence: &MedicationAdministrationOccurrenceV1,
    occurrence_evidence: OccurrenceEvidenceV1,
) -> Result<MedicationAdministrationOccurrenceBindingV1, BindingError> {
    occurrence.validate_shape()?;
    event
        .validate_shape()
        .map_err(|error| BindingError::AdministrationEvent(error.to_string()))?;
    receipt
        .validate_shape()
        .map_err(|error| BindingError::AdministrationReceipt(error.to_string()))?;

    if !matches!(
        event.status,
        AdministrationStatusV1::Performed | AdministrationStatusV1::PartiallyPerformed
    ) {
        return Err(BindingError::NonPerformedAdministration);
    }
    let performed = event
        .performed
        .as_ref()
        .ok_or(BindingError::MissingPerformedDetails)?;

    let event_digest = event
        .verified_digest()
        .map_err(|error| BindingError::AdministrationEvent(error.to_string()))?;
    let receipt_digest = receipt
        .verified_digest()
        .map_err(|error| BindingError::AdministrationReceipt(error.to_string()))?;
    let occurrence_digest = occurrence.verified_digest()?;

    if receipt.event_digest != event_digest.stored() {
        return Err(BindingError::ReceiptEventMismatch);
    }
    if receipt.administration_id != event.administration_id {
        return Err(BindingError::AdministrationIdMismatch);
    }
    if receipt.medication_artifact_digest != event.medication_artifact_digest
        || receipt.medication_artifact_digest != occurrence.medication_artifact_digest
    {
        return Err(BindingError::MedicationLineageMismatch);
    }
    if receipt.patient_subject_binding_evidence_digest
        != event.patient_subject_binding_evidence_digest
        || receipt.patient_subject_binding_evidence_digest
            != occurrence.patient_subject_binding_evidence_digest
    {
        return Err(BindingError::PatientBindingLineageMismatch);
    }
    if performed.dosage_index != occurrence.dosage_index {
        return Err(BindingError::DosageIndexMismatch);
    }

    let binding = MedicationAdministrationOccurrenceBindingV1 {
        schema_version: 1,
        administration_receipt_digest: receipt_digest.stored(),
        event_digest: event_digest.stored(),
        occurrence_digest: occurrence_digest.stored(),
        medication_artifact_digest: occurrence.medication_artifact_digest,
        patient_subject_binding_evidence_digest: occurrence.patient_subject_binding_evidence_digest,
        dosage_index: occurrence.dosage_index,
        occurrence_evidence,
    };
    binding.validate_shape()?;
    Ok(binding)
}

fn require_domain(digest: StoredDigest, expected: DigestDomain) -> Result<(), BindingError> {
    digest.validate_shape()?;
    if digest.domain != expected {
        return Err(BindingError::WrongDigestDomain {
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
) -> Result<VerifiedDigest, BindingError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| BindingError::Serialization(error.to_string()))?;
    let mut framed = Vec::with_capacity(tag.len() + 1 + encoded.len());
    framed.extend_from_slice(tag);
    framed.push(0);
    framed.extend_from_slice(&encoded);
    Ok(hash_canonical_bytes(domain, &framed)?)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BindingError {
    #[error("unsupported administration occurrence binding version {0}")]
    UnsupportedBindingVersion(u16),
    #[error("stored digest domain mismatch: expected {expected:?}, got {actual:?}")]
    WrongDigestDomain {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("occurrence evidence kind does not match occurrence")]
    OccurrenceEvidenceKindMismatch,
    #[error("administration event is not a performed/partially-performed action")]
    NonPerformedAdministration,
    #[error("performed administration details are missing")]
    MissingPerformedDetails,
    #[error("administration receipt does not bind the supplied event digest")]
    ReceiptEventMismatch,
    #[error("administration receipt and event IDs differ")]
    AdministrationIdMismatch,
    #[error("medication lineage differs between event/receipt/occurrence")]
    MedicationLineageMismatch,
    #[error("patient subject-binding lineage differs between event/receipt/occurrence")]
    PatientBindingLineageMismatch,
    #[error("administration dosage index differs from occurrence dosage index")]
    DosageIndexMismatch,
    #[error("schedule plan does not match occurrence lineage")]
    SchedulePlanLineageMismatch,
    #[error("scheduled occurrence ordinal is absent from the supplied plan")]
    ScheduledOccurrenceMissing,
    #[error("scheduled occurrence window differs from supplied plan")]
    ScheduledOccurrenceWindowMismatch,
    #[error("schedule-resolution provenance targets a different semantic plan")]
    ScheduleResolutionPlanMismatch,
    #[error("PRN intent does not match occurrence lineage/nonce")]
    PrnIntentLineageMismatch,
    #[error("administration event validation failed: {0}")]
    AdministrationEvent(String),
    #[error("administration receipt validation failed: {0}")]
    AdministrationReceipt(String),
    #[error("binding serialization failed: {0}")]
    Serialization(String),
    #[error(transparent)]
    Occurrence(#[from] OccurrenceError),
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
    fn scheduled_binding_shape_requires_resolution_domain() {
        let value = MedicationAdministrationOccurrenceBindingV1 {
            schema_version: 1,
            administration_receipt_digest: stored(
                DigestDomain::MedicationAdministrationReceipt,
                1,
            ),
            event_digest: stored(DigestDomain::MedicationAdministrationEvent, 2),
            occurrence_digest: stored(DigestDomain::MedicationAdministrationOccurrence, 3),
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 4),
            patient_subject_binding_evidence_digest: stored(
                DigestDomain::PatientSubjectBindingEvidence,
                5,
            ),
            dosage_index: 0,
            occurrence_evidence: OccurrenceEvidenceV1::Scheduled {
                schedule_plan_digest: stored(
                    DigestDomain::MedicationAdministrationSchedulePlan,
                    6,
                ),
                schedule_resolution_provenance_digest: stored(DigestDomain::EvidenceCapsule, 7),
            },
        };
        assert!(matches!(
            value.validate_shape(),
            Err(BindingError::WrongDigestDomain { .. })
        ));
    }

    #[test]
    fn binding_digest_is_domain_separated() {
        let value = MedicationAdministrationOccurrenceBindingV1 {
            schema_version: 1,
            administration_receipt_digest: stored(
                DigestDomain::MedicationAdministrationReceipt,
                1,
            ),
            event_digest: stored(DigestDomain::MedicationAdministrationEvent, 2),
            occurrence_digest: stored(DigestDomain::MedicationAdministrationOccurrence, 3),
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 4),
            patient_subject_binding_evidence_digest: stored(
                DigestDomain::PatientSubjectBindingEvidence,
                5,
            ),
            dosage_index: 0,
            occurrence_evidence: OccurrenceEvidenceV1::OneTime,
        };
        assert_eq!(
            value.verified_digest().unwrap().domain(),
            DigestDomain::MedicationAdministrationOccurrenceBinding
        );
    }
}
