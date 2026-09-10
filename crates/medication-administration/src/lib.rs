#![deny(unsafe_code)]
//! Evidence-bound clinician medication administration for Mycelix-Health.
//!
//! V1 is intentionally narrow: clinician-performed administration against one
//! exact active MedicationRequest and one exact finalized patient-specific dispense.
//! Self-report, caregiver report, device adherence, facility stock, and emergency
//! stock remain separate future evidence classes.
//!
//! `dispensed != administered` and `Prescribe/Dispense != Administer` are structural
//! invariants of this crate.

use mycelix_clinical_authority::{
    AuthorityPermit, AuthorityPurpose, JurisdictionCode, PrincipalBinding,
};
use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use mycelix_clinical_semantics::{CodeableConcept, Quantity, SubjectRef};
use mycelix_fhir_medication_semantics::MedicationRequestArtifact;
use mycelix_medication_activation_state::{
    ActivationLifecycle, CurrentActivation, CurrentActivationState, MedicationActivationView,
};
use mycelix_medication_dispense_state::{
    DispenseLineageHealth, DispenseSlotState, MedicationDispenseView,
};
use mycelix_medication_semantics::DoseAmount;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

const POLICY_TAG: &[u8] = b"mycelix-health/medication-administration-policy-v1";
const EVENT_TAG: &[u8] = b"mycelix-health/medication-administration-event-v1";
const RECEIPT_TAG: &[u8] = b"mycelix-health/medication-administration-receipt-v1";
const ACTIVATION_STATE_TAG: &[u8] = b"mycelix-health/medication-activation-state-v1";
const SUPPLY_TAG: &[u8] = b"mycelix-health/medication-administration-supply-v1";
const MAX_AGE_MICROS: i64 = 86_400_000_000;
const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdministrationPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub allow_emergency_override_activation: bool,
    pub allow_partial_administration: bool,
    pub max_authority_age_micros: i64,
    pub max_activation_state_age_micros: i64,
    pub max_patient_binding_age_micros: i64,
    pub max_dispense_state_age_micros: i64,
    pub max_recording_delay_micros: i64,
    pub max_future_skew_micros: i64,
}

impl AdministrationPolicyV1 {
    pub fn validate(&self) -> Result<(), AdministrationError> {
        if self.schema_version != 1 {
            return Err(AdministrationError::UnsupportedPolicyVersion(self.schema_version));
        }
        if self.policy_id.trim().is_empty() {
            return Err(AdministrationError::MissingPolicyId);
        }
        for (value, field) in [
            (self.max_authority_age_micros, "max_authority_age_micros"),
            (self.max_activation_state_age_micros, "max_activation_state_age_micros"),
            (self.max_patient_binding_age_micros, "max_patient_binding_age_micros"),
            (self.max_dispense_state_age_micros, "max_dispense_state_age_micros"),
            (self.max_recording_delay_micros, "max_recording_delay_micros"),
        ] {
            if value <= 0 || value > MAX_AGE_MICROS {
                return Err(AdministrationError::InvalidAgePolicy(field));
            }
        }
        if self.max_future_skew_micros < 0
            || self.max_future_skew_micros > MAX_FUTURE_SKEW_MICROS
        {
            return Err(AdministrationError::InvalidFutureSkewPolicy);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, AdministrationError> {
        self.validate()?;
        hash_json(DigestDomain::MedicationAdministrationPolicy, POLICY_TAG, self)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdministrationStatusV1 {
    Performed,
    PartiallyPerformed,
    NotDone,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdministrationNonPerformanceReasonV1 {
    PatientRefused,
    HeldByClinician,
    ContraindicationDiscovered,
    MedicationUnavailable,
    ProcedureCancelled,
    PatientUnavailable,
    Other,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PartialAdministrationReasonV1 {
    PatientRequestedStop,
    AdverseReactionDuringAdministration,
    DosePartiallyAvailable,
    AccessOrDeviceFailure,
    ClinicianStopped,
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdministrationTimeV1 {
    pub start_micros: i64,
    pub end_micros: Option<i64>,
}

impl AdministrationTimeV1 {
    fn validate(&self) -> Result<(), AdministrationError> {
        if self.end_micros.is_some_and(|end| end < self.start_micros) {
            return Err(AdministrationError::AdministrationEndsBeforeStart);
        }
        Ok(())
    }

    fn latest_micros(&self) -> i64 {
        self.end_micros.unwrap_or(self.start_micros)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PerformedAdministrationV1 {
    /// Index into the exact MedicationOrder dosage array.
    pub dosage_index: u32,
    pub product: CodeableConcept,
    pub dose: Quantity,
    pub route: CodeableConcept,
    pub method: Option<CodeableConcept>,
    pub site: Option<CodeableConcept>,
    pub effective: AdministrationTimeV1,
    /// Optional protected supply/lot/device evidence identity.
    pub supply_detail_evidence_digest: Option<StoredDigest>,
}

impl PerformedAdministrationV1 {
    fn validate_shape(&self) -> Result<(), AdministrationError> {
        self.product.validate_machine_actionable()?;
        self.dose.validate_machine_actionable()?;
        if self.dose.value <= 0.0 {
            return Err(AdministrationError::NonPositiveAdministeredDose);
        }
        self.route.validate_machine_actionable()?;
        if let Some(method) = &self.method {
            method.validate_machine_actionable()?;
        }
        if let Some(site) = &self.site {
            site.validate_machine_actionable()?;
        }
        self.effective.validate()?;
        if let Some(digest) = self.supply_detail_evidence_digest {
            require_domain(digest, DigestDomain::MedicationAdministrationSupplyEvidence)?;
        }
        Ok(())
    }
}

/// High-detail/private clinical event. A future DHT attestation should reference its
/// digest rather than replicate these details.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MedicationAdministrationEventV1 {
    pub schema_version: u16,
    pub administration_id: String,
    pub medication_artifact_digest: StoredDigest,
    pub activation_semantic_receipt_digest: StoredDigest,
    pub finalized_dispense_receipt_digest: StoredDigest,
    pub subject: SubjectRef,
    pub patient_subject_binding_evidence_digest: StoredDigest,
    pub administrator_principal_binding: [u8; 32],
    pub status: AdministrationStatusV1,
    pub performed: Option<PerformedAdministrationV1>,
    pub nonperformance_reason: Option<AdministrationNonPerformanceReasonV1>,
    pub partial_reason: Option<PartialAdministrationReasonV1>,
    pub rationale_commitment: Option<[u8; 32]>,
}

impl MedicationAdministrationEventV1 {
    pub fn validate_shape(&self) -> Result<(), AdministrationError> {
        if self.schema_version != 1 {
            return Err(AdministrationError::UnsupportedEventVersion(self.schema_version));
        }
        if self.administration_id.trim().is_empty() {
            return Err(AdministrationError::MissingAdministrationId);
        }
        require_domain(self.medication_artifact_digest, DigestDomain::MedicationRequestArtifact)?;
        require_activation_receipt_domain(self.activation_semantic_receipt_digest)?;
        require_domain(
            self.finalized_dispense_receipt_digest,
            DigestDomain::MedicationDispenseReceipt,
        )?;
        require_domain(
            self.patient_subject_binding_evidence_digest,
            DigestDomain::PatientSubjectBindingEvidence,
        )?;
        if self.subject.resource_type.trim().is_empty() || self.subject.id.trim().is_empty() {
            return Err(AdministrationError::InvalidSubject);
        }
        if self.administrator_principal_binding == [0u8; 32] {
            return Err(AdministrationError::ZeroAdministratorPrincipal);
        }
        if self.rationale_commitment.is_some_and(|value| value == [0u8; 32]) {
            return Err(AdministrationError::ZeroRationaleCommitment);
        }

        match self.status {
            AdministrationStatusV1::Performed => {
                self.performed
                    .as_ref()
                    .ok_or(AdministrationError::PerformedDetailsRequired)?
                    .validate_shape()?;
                if self.nonperformance_reason.is_some() || self.partial_reason.is_some() {
                    return Err(AdministrationError::InvalidStatusDetailCombination);
                }
            }
            AdministrationStatusV1::PartiallyPerformed => {
                self.performed
                    .as_ref()
                    .ok_or(AdministrationError::PerformedDetailsRequired)?
                    .validate_shape()?;
                if self.nonperformance_reason.is_some() || self.partial_reason.is_none() {
                    return Err(AdministrationError::InvalidStatusDetailCombination);
                }
            }
            AdministrationStatusV1::NotDone => {
                if self.performed.is_some()
                    || self.nonperformance_reason.is_none()
                    || self.partial_reason.is_some()
                {
                    return Err(AdministrationError::InvalidStatusDetailCombination);
                }
                if matches!(
                    self.nonperformance_reason,
                    Some(AdministrationNonPerformanceReasonV1::Other)
                ) && self.rationale_commitment.is_none()
                {
                    return Err(AdministrationError::OtherReasonRequiresCommitment);
                }
            }
            AdministrationStatusV1::Indeterminate => {
                if self.performed.is_some()
                    || self.nonperformance_reason.is_some()
                    || self.partial_reason.is_some()
                    || self.rationale_commitment.is_none()
                {
                    return Err(AdministrationError::InvalidStatusDetailCombination);
                }
            }
        }
        if matches!(self.partial_reason, Some(PartialAdministrationReasonV1::Other))
            && self.rationale_commitment.is_none()
        {
            return Err(AdministrationError::OtherReasonRequiresCommitment);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, AdministrationError> {
        self.validate_shape()?;
        hash_json(DigestDomain::MedicationAdministrationEvent, EVENT_TAG, self)
    }
}

/// Adapter-produced subject binding. It establishes which patient record the
/// clinical subject resolved to, not that the patient authored the administration.
pub struct VerifiedPatientSubjectBinding {
    subject: SubjectRef,
    patient_record_digest: StoredDigest,
    binding_evidence_digest: StoredDigest,
    resolved_at_micros: i64,
}

impl VerifiedPatientSubjectBinding {
    pub fn from_verified_evidence(
        subject: SubjectRef,
        patient_record_digest: VerifiedDigest,
        binding_evidence_digest: VerifiedDigest,
        resolved_at_micros: i64,
    ) -> Result<Self, AdministrationError> {
        if subject.resource_type.trim().is_empty() || subject.id.trim().is_empty() {
            return Err(AdministrationError::InvalidSubject);
        }
        patient_record_digest.require_domain(DigestDomain::ClinicalArtifact)?;
        binding_evidence_digest.require_domain(DigestDomain::PatientSubjectBindingEvidence)?;
        Ok(Self {
            subject,
            patient_record_digest: patient_record_digest.stored(),
            binding_evidence_digest: binding_evidence_digest.stored(),
            resolved_at_micros,
        })
    }

    pub fn subject(&self) -> &SubjectRef {
        &self.subject
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdministrationActivationProvenance {
    Qualified {
        activation_receipt_digest: StoredDigest,
    },
    EmergencyOverride {
        override_receipt_digest: StoredDigest,
        safety_assessment_digest: StoredDigest,
    },
}

impl AdministrationActivationProvenance {
    fn semantic_receipt_digest(&self) -> StoredDigest {
        match self {
            Self::Qualified {
                activation_receipt_digest,
            } => *activation_receipt_digest,
            Self::EmergencyOverride {
                override_receipt_digest,
                ..
            } => *override_receipt_digest,
        }
    }

    fn is_emergency(&self) -> bool {
        matches!(self, Self::EmergencyOverride { .. })
    }
}

pub struct VerifiedActivationForAdministration {
    medication_artifact_digest: StoredDigest,
    activation_state_digest: StoredDigest,
    provenance: AdministrationActivationProvenance,
    observed_at_micros: i64,
}

pub fn resolve_current_activation_for_administration(
    view: &MedicationActivationView,
    medication_artifact_digest: VerifiedDigest,
    observed_at_micros: i64,
) -> Result<VerifiedActivationForAdministration, AdministrationError> {
    medication_artifact_digest.require_domain(DigestDomain::MedicationRequestArtifact)?;
    let state_digest = hash_json(
        DigestDomain::MedicationActivationState,
        ACTIVATION_STATE_TAG,
        view,
    )?;
    let provenance = match &view.current {
        CurrentActivationState::None => return Err(AdministrationError::NoCurrentActivation),
        CurrentActivationState::Conflict { .. } => return Err(AdministrationError::ActivationConflict),
        CurrentActivationState::One(CurrentActivation::Qualified(lineage)) => {
            if !matches!(&lineage.lifecycle, ActivationLifecycle::Current) {
                return Err(AdministrationError::ActivationNotCurrent);
            }
            if lineage.medication_artifact_digest != medication_artifact_digest.stored() {
                return Err(AdministrationError::ActivationMedicationMismatch);
            }
            require_domain(
                lineage.activation_receipt_digest,
                DigestDomain::MedicationActivationReceipt,
            )?;
            AdministrationActivationProvenance::Qualified {
                activation_receipt_digest: lineage.activation_receipt_digest,
            }
        }
        CurrentActivationState::One(CurrentActivation::EmergencyOverride(lineage)) => {
            if !matches!(&lineage.lifecycle, ActivationLifecycle::Current) {
                return Err(AdministrationError::ActivationNotCurrent);
            }
            if lineage.medication_artifact_digest != medication_artifact_digest.stored() {
                return Err(AdministrationError::ActivationMedicationMismatch);
            }
            require_domain(
                lineage.override_receipt_digest,
                DigestDomain::EmergencyMedicationOverrideReceipt,
            )?;
            require_domain(
                lineage.safety_assessment_digest,
                DigestDomain::MedicationSafetyAssessment,
            )?;
            AdministrationActivationProvenance::EmergencyOverride {
                override_receipt_digest: lineage.override_receipt_digest,
                safety_assessment_digest: lineage.safety_assessment_digest,
            }
        }
    };
    Ok(VerifiedActivationForAdministration {
        medication_artifact_digest: medication_artifact_digest.stored(),
        activation_state_digest: state_digest.stored(),
        provenance,
        observed_at_micros,
    })
}

#[derive(Serialize)]
struct SupplyMaterialV1 {
    medication_artifact_digest: StoredDigest,
    activation_semantic_receipt_digest: StoredDigest,
    finalized_dispense_receipt_digest: StoredDigest,
    slot_index: u32,
    observed_at_micros: i64,
}

pub struct VerifiedAdministrationSupply {
    medication_artifact_digest: StoredDigest,
    activation_semantic_receipt_digest: StoredDigest,
    finalized_dispense_receipt_digest: StoredDigest,
    slot_index: u32,
    supply_evidence_digest: StoredDigest,
    observed_at_micros: i64,
}

pub fn resolve_finalized_dispense_for_administration(
    view: &MedicationDispenseView,
    medication_artifact_digest: VerifiedDigest,
    activation_semantic_receipt_digest: StoredDigest,
    finalized_dispense_receipt_digest: StoredDigest,
    observed_at_micros: i64,
) -> Result<VerifiedAdministrationSupply, AdministrationError> {
    medication_artifact_digest.require_domain(DigestDomain::MedicationRequestArtifact)?;
    require_activation_receipt_domain(activation_semantic_receipt_digest)?;
    require_domain(
        finalized_dispense_receipt_digest,
        DigestDomain::MedicationDispenseReceipt,
    )?;

    let matching: Vec<_> = view
        .lineages
        .iter()
        .filter(|lineage| {
            lineage.medication_artifact_digest == medication_artifact_digest.stored()
                && lineage.activation_semantic_receipt_digest == activation_semantic_receipt_digest
        })
        .collect();
    if matching.len() != 1 {
        return Err(if matching.is_empty() {
            AdministrationError::DispenseLineageMissing
        } else {
            AdministrationError::AmbiguousDispenseLineage
        });
    }
    let lineage = matching[0];
    if !matches!(&lineage.health, DispenseLineageHealth::Consistent) {
        return Err(AdministrationError::DispenseLineageConflict);
    }

    let mut found_slot = None;
    for slot in &lineage.slots {
        if let DispenseSlotState::Finalized(finalized) = &slot.state {
            if finalized.receipt_digest == finalized_dispense_receipt_digest {
                if found_slot.replace(slot.slot_index).is_some() {
                    return Err(AdministrationError::AmbiguousFinalizedDispense);
                }
            }
        }
    }
    let slot_index = found_slot.ok_or(AdministrationError::FinalizedDispenseMissing)?;
    match lineage.contiguous_finalized_through {
        Some(through) if through >= slot_index => {}
        _ => return Err(AdministrationError::DispenseHistoryNotContiguous),
    }

    let material = SupplyMaterialV1 {
        medication_artifact_digest: medication_artifact_digest.stored(),
        activation_semantic_receipt_digest,
        finalized_dispense_receipt_digest,
        slot_index,
        observed_at_micros,
    };
    let digest = hash_json(
        DigestDomain::MedicationAdministrationSupplyEvidence,
        SUPPLY_TAG,
        &material,
    )?;
    Ok(VerifiedAdministrationSupply {
        medication_artifact_digest: medication_artifact_digest.stored(),
        activation_semantic_receipt_digest,
        finalized_dispense_receipt_digest,
        slot_index,
        supply_evidence_digest: digest.stored(),
        observed_at_micros,
    })
}

pub struct MedicationAdministrationCapability {
    administration_id: String,
    event_digest: StoredDigest,
    medication_artifact_digest: StoredDigest,
    activation_state_digest: StoredDigest,
    activation_provenance: AdministrationActivationProvenance,
    activation_observed_at_micros: i64,
    finalized_dispense_receipt_digest: StoredDigest,
    dispense_slot_index: u32,
    supply_evidence_digest: StoredDigest,
    dispense_observed_at_micros: i64,
    patient_record_digest: StoredDigest,
    patient_subject_binding_evidence_digest: StoredDigest,
    patient_binding_resolved_at_micros: i64,
    administrator_principal: PrincipalBinding,
    authority_policy_digest: StoredDigest,
    administration_policy_digest: StoredDigest,
    authorized_at_micros: i64,
    supporting_authority_evidence: Vec<[u8; 32]>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MedicationAdministrationReceiptV1 {
    pub schema_version: u16,
    pub administration_id: String,
    pub event_digest: StoredDigest,
    pub medication_artifact_digest: StoredDigest,
    pub activation_state_digest: StoredDigest,
    pub activation_provenance: AdministrationActivationProvenance,
    pub activation_observed_at_micros: i64,
    pub finalized_dispense_receipt_digest: StoredDigest,
    pub dispense_slot_index: u32,
    pub supply_evidence_digest: StoredDigest,
    pub dispense_observed_at_micros: i64,
    pub patient_record_digest: StoredDigest,
    pub patient_subject_binding_evidence_digest: StoredDigest,
    pub patient_binding_resolved_at_micros: i64,
    pub administrator_principal: [u8; 32],
    pub authority_policy_digest: StoredDigest,
    pub administration_policy_digest: StoredDigest,
    pub authorized_at_micros: i64,
    pub supporting_authority_evidence: Vec<[u8; 32]>,
}

impl MedicationAdministrationReceiptV1 {
    pub fn validate_shape(&self) -> Result<(), AdministrationError> {
        if self.schema_version != 1 {
            return Err(AdministrationError::UnsupportedReceiptVersion(self.schema_version));
        }
        if self.administration_id.trim().is_empty() {
            return Err(AdministrationError::MissingAdministrationId);
        }
        if self.administrator_principal == [0u8; 32] {
            return Err(AdministrationError::ZeroAdministratorPrincipal);
        }
        require_domain(self.event_digest, DigestDomain::MedicationAdministrationEvent)?;
        require_domain(self.medication_artifact_digest, DigestDomain::MedicationRequestArtifact)?;
        require_domain(self.activation_state_digest, DigestDomain::MedicationActivationState)?;
        validate_activation_provenance(&self.activation_provenance)?;
        require_domain(
            self.finalized_dispense_receipt_digest,
            DigestDomain::MedicationDispenseReceipt,
        )?;
        require_domain(
            self.supply_evidence_digest,
            DigestDomain::MedicationAdministrationSupplyEvidence,
        )?;
        require_domain(self.patient_record_digest, DigestDomain::ClinicalArtifact)?;
        require_domain(
            self.patient_subject_binding_evidence_digest,
            DigestDomain::PatientSubjectBindingEvidence,
        )?;
        require_domain(self.authority_policy_digest, DigestDomain::AuthorityPolicy)?;
        require_domain(
            self.administration_policy_digest,
            DigestDomain::MedicationAdministrationPolicy,
        )?;
        if self.supporting_authority_evidence.is_empty()
            || self
                .supporting_authority_evidence
                .iter()
                .any(|value| *value == [0u8; 32])
        {
            return Err(AdministrationError::InvalidAuthorityEvidence);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, AdministrationError> {
        self.validate_shape()?;
        hash_json(
            DigestDomain::MedicationAdministrationReceipt,
            RECEIPT_TAG,
            self,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub fn authorize_clinician_administration(
    medication: &MedicationRequestArtifact,
    activation: VerifiedActivationForAdministration,
    supply: VerifiedAdministrationSupply,
    patient: VerifiedPatientSubjectBinding,
    event: &MedicationAdministrationEventV1,
    authority: AuthorityPermit,
    authority_policy_digest: VerifiedDigest,
    policy: &AdministrationPolicyV1,
    authenticated_administrator: PrincipalBinding,
    jurisdiction: &JurisdictionCode,
    execution_at_micros: i64,
) -> Result<MedicationAdministrationCapability, AdministrationError> {
    policy.validate()?;
    medication
        .validate_for_activation(execution_at_micros)
        .map_err(|error| AdministrationError::MedicationArtifact(error.to_string()))?;
    let medication_digest = medication
        .verified_digest()
        .map_err(|error| AdministrationError::MedicationArtifact(error.to_string()))?;
    medication_digest.require_domain(DigestDomain::MedicationRequestArtifact)?;

    event.validate_shape()?;
    let event_digest = event.verified_digest()?;
    if !matches!(
        event.status,
        AdministrationStatusV1::Performed | AdministrationStatusV1::PartiallyPerformed
    ) {
        return Err(AdministrationError::NonPerformedEventCannotAuthorize);
    }
    if event.status == AdministrationStatusV1::PartiallyPerformed
        && !policy.allow_partial_administration
    {
        return Err(AdministrationError::PartialAdministrationNotAllowed);
    }
    if event.medication_artifact_digest != medication_digest.stored() {
        return Err(AdministrationError::EventMedicationMismatch);
    }
    if event.administrator_principal_binding != authenticated_administrator.0 {
        return Err(AdministrationError::EventAdministratorMismatch);
    }

    if activation.medication_artifact_digest != medication_digest.stored() {
        return Err(AdministrationError::ActivationMedicationMismatch);
    }
    if activation.provenance.is_emergency() && !policy.allow_emergency_override_activation {
        return Err(AdministrationError::EmergencyActivationNotAllowed);
    }
    if event.activation_semantic_receipt_digest != activation.provenance.semantic_receipt_digest() {
        return Err(AdministrationError::EventActivationMismatch);
    }
    verify_freshness(
        activation.observed_at_micros,
        execution_at_micros,
        policy.max_activation_state_age_micros,
        policy.max_future_skew_micros,
        AdministrationError::ActivationStateFromFuture,
        AdministrationError::ActivationStateStale,
    )?;

    if &patient.subject != &medication.order().subject || &event.subject != &patient.subject {
        return Err(AdministrationError::PatientSubjectMismatch);
    }
    if event.patient_subject_binding_evidence_digest != patient.binding_evidence_digest {
        return Err(AdministrationError::PatientBindingEvidenceMismatch);
    }
    verify_freshness(
        patient.resolved_at_micros,
        execution_at_micros,
        policy.max_patient_binding_age_micros,
        policy.max_future_skew_micros,
        AdministrationError::PatientBindingFromFuture,
        AdministrationError::PatientBindingStale,
    )?;

    if supply.medication_artifact_digest != medication_digest.stored()
        || supply.activation_semantic_receipt_digest
            != activation.provenance.semantic_receipt_digest()
    {
        return Err(AdministrationError::SupplyLineageMismatch);
    }
    if event.finalized_dispense_receipt_digest != supply.finalized_dispense_receipt_digest {
        return Err(AdministrationError::EventDispenseMismatch);
    }
    verify_freshness(
        supply.observed_at_micros,
        execution_at_micros,
        policy.max_dispense_state_age_micros,
        policy.max_future_skew_micros,
        AdministrationError::DispenseStateFromFuture,
        AdministrationError::DispenseStateStale,
    )?;

    let performed = event
        .performed
        .as_ref()
        .ok_or(AdministrationError::PerformedDetailsRequired)?;
    validate_performed_against_order(performed, event.status, medication, policy)?;
    verify_event_time(
        &performed.effective,
        execution_at_micros,
        policy.max_recording_delay_micros,
        policy.max_future_skew_micros,
    )?;

    authority_policy_digest.require_domain(DigestDomain::AuthorityPolicy)?;
    if authority.purpose() != AuthorityPurpose::Administer {
        return Err(AdministrationError::WrongAuthorityPurpose);
    }
    if authority.principal() != authenticated_administrator {
        return Err(AdministrationError::AuthorityPrincipalMismatch);
    }
    if authority.target_artifact_digest().0 != event_digest.value() {
        return Err(AdministrationError::AuthorityTargetMismatch);
    }
    if authority.policy_digest().0 != authority_policy_digest.value() {
        return Err(AdministrationError::AuthorityPolicyMismatch);
    }
    if authority.jurisdiction() != jurisdiction {
        return Err(AdministrationError::AuthorityJurisdictionMismatch);
    }
    verify_freshness(
        authority.evaluated_at_micros(),
        execution_at_micros,
        policy.max_authority_age_micros,
        policy.max_future_skew_micros,
        AdministrationError::AuthorityFromFuture,
        AdministrationError::AuthorityStale,
    )?;
    if authority.supporting_evidence_digests().is_empty() {
        return Err(AdministrationError::InvalidAuthorityEvidence);
    }

    let policy_digest = policy.verified_digest()?;
    Ok(MedicationAdministrationCapability {
        administration_id: event.administration_id.clone(),
        event_digest: event_digest.stored(),
        medication_artifact_digest: medication_digest.stored(),
        activation_state_digest: activation.activation_state_digest,
        activation_provenance: activation.provenance,
        activation_observed_at_micros: activation.observed_at_micros,
        finalized_dispense_receipt_digest: supply.finalized_dispense_receipt_digest,
        dispense_slot_index: supply.slot_index,
        supply_evidence_digest: supply.supply_evidence_digest,
        dispense_observed_at_micros: supply.observed_at_micros,
        patient_record_digest: patient.patient_record_digest,
        patient_subject_binding_evidence_digest: patient.binding_evidence_digest,
        patient_binding_resolved_at_micros: patient.resolved_at_micros,
        administrator_principal: authenticated_administrator,
        authority_policy_digest: authority_policy_digest.stored(),
        administration_policy_digest: policy_digest.stored(),
        authorized_at_micros: execution_at_micros,
        supporting_authority_evidence: authority
            .supporting_evidence_digests()
            .iter()
            .map(|digest| digest.0)
            .collect(),
    })
}

pub fn consume_administration_capability(
    capability: MedicationAdministrationCapability,
) -> Result<(MedicationAdministrationReceiptV1, VerifiedDigest), AdministrationError> {
    let receipt = MedicationAdministrationReceiptV1 {
        schema_version: 1,
        administration_id: capability.administration_id,
        event_digest: capability.event_digest,
        medication_artifact_digest: capability.medication_artifact_digest,
        activation_state_digest: capability.activation_state_digest,
        activation_provenance: capability.activation_provenance,
        activation_observed_at_micros: capability.activation_observed_at_micros,
        finalized_dispense_receipt_digest: capability.finalized_dispense_receipt_digest,
        dispense_slot_index: capability.dispense_slot_index,
        supply_evidence_digest: capability.supply_evidence_digest,
        dispense_observed_at_micros: capability.dispense_observed_at_micros,
        patient_record_digest: capability.patient_record_digest,
        patient_subject_binding_evidence_digest: capability.patient_subject_binding_evidence_digest,
        patient_binding_resolved_at_micros: capability.patient_binding_resolved_at_micros,
        administrator_principal: capability.administrator_principal.0,
        authority_policy_digest: capability.authority_policy_digest,
        administration_policy_digest: capability.administration_policy_digest,
        authorized_at_micros: capability.authorized_at_micros,
        supporting_authority_evidence: capability.supporting_authority_evidence,
    };
    let digest = receipt.verified_digest()?;
    Ok((receipt, digest))
}

fn validate_performed_against_order(
    performed: &PerformedAdministrationV1,
    status: AdministrationStatusV1,
    medication: &MedicationRequestArtifact,
    policy: &AdministrationPolicyV1,
) -> Result<(), AdministrationError> {
    let dosage_index = usize::try_from(performed.dosage_index)
        .map_err(|_| AdministrationError::DosageIndexOverflow)?;
    let ordered = medication
        .order()
        .dosage
        .get(dosage_index)
        .ok_or(AdministrationError::DosageIndexOutOfBounds)?;

    if !same_coded_concept(&performed.product, &medication.order().medication) {
        return Err(AdministrationError::MedicationProductMismatch);
    }
    if !same_coded_concept(&performed.route, &ordered.route) {
        return Err(AdministrationError::RouteMismatch);
    }
    if let Some(required) = &ordered.method {
        let actual = performed
            .method
            .as_ref()
            .ok_or(AdministrationError::RequiredMethodMissing)?;
        if !same_coded_concept(actual, required) {
            return Err(AdministrationError::MethodMismatch);
        }
    }
    if let Some(required) = &ordered.site {
        let actual = performed
            .site
            .as_ref()
            .ok_or(AdministrationError::RequiredSiteMissing)?;
        if !same_coded_concept(actual, required) {
            return Err(AdministrationError::SiteMismatch);
        }
    }

    let ordered_dose = match &ordered.dose {
        DoseAmount::Quantity(value) => value,
        DoseAmount::Range(_) => return Err(AdministrationError::OrderedDoseRangeUnsupportedV1),
    };
    if ordered_dose.value <= 0.0 {
        return Err(AdministrationError::InvalidOrderedDose);
    }
    if performed.dose.system != ordered_dose.system || performed.dose.code != ordered_dose.code {
        return Err(AdministrationError::DoseUnitMismatch);
    }
    match status {
        AdministrationStatusV1::Performed => {
            if performed.dose.value.to_bits() != ordered_dose.value.to_bits() {
                return Err(AdministrationError::PerformedDoseMismatch);
            }
        }
        AdministrationStatusV1::PartiallyPerformed => {
            if !policy.allow_partial_administration {
                return Err(AdministrationError::PartialAdministrationNotAllowed);
            }
            if performed.dose.value <= 0.0 || performed.dose.value >= ordered_dose.value {
                return Err(AdministrationError::InvalidPartialDose);
            }
        }
        AdministrationStatusV1::NotDone | AdministrationStatusV1::Indeterminate => {
            return Err(AdministrationError::NonPerformedEventCannotAuthorize)
        }
    }
    Ok(())
}

fn verify_event_time(
    effective: &AdministrationTimeV1,
    execution_at_micros: i64,
    max_recording_delay_micros: i64,
    max_future_skew_micros: i64,
) -> Result<(), AdministrationError> {
    effective.validate()?;
    let delta = execution_at_micros as i128 - effective.latest_micros() as i128;
    if delta < -(max_future_skew_micros as i128) {
        return Err(AdministrationError::AdministrationTimeFromFuture);
    }
    if delta.max(0) > max_recording_delay_micros as i128 {
        return Err(AdministrationError::AdministrationRecordedTooLate);
    }
    Ok(())
}

fn validate_activation_provenance(
    provenance: &AdministrationActivationProvenance,
) -> Result<(), AdministrationError> {
    match provenance {
        AdministrationActivationProvenance::Qualified {
            activation_receipt_digest,
        } => require_domain(*activation_receipt_digest, DigestDomain::MedicationActivationReceipt),
        AdministrationActivationProvenance::EmergencyOverride {
            override_receipt_digest,
            safety_assessment_digest,
        } => {
            require_domain(
                *override_receipt_digest,
                DigestDomain::EmergencyMedicationOverrideReceipt,
            )?;
            require_domain(
                *safety_assessment_digest,
                DigestDomain::MedicationSafetyAssessment,
            )
        }
    }
}

fn same_coded_concept(left: &CodeableConcept, right: &CodeableConcept) -> bool {
    fn identities(value: &CodeableConcept) -> BTreeSet<(&str, &str, Option<&str>)> {
        value
            .coding
            .iter()
            .map(|coding| {
                (
                    coding.system.as_str(),
                    coding.code.as_str(),
                    coding.version.as_deref(),
                )
            })
            .collect()
    }
    identities(left) == identities(right)
}

fn require_activation_receipt_domain(digest: StoredDigest) -> Result<(), AdministrationError> {
    digest.validate_shape()?;
    if !matches!(
        digest.domain,
        DigestDomain::MedicationActivationReceipt
            | DigestDomain::EmergencyMedicationOverrideReceipt
    ) {
        return Err(AdministrationError::WrongActivationReceiptDomain(digest.domain));
    }
    Ok(())
}

fn require_domain(digest: StoredDigest, expected: DigestDomain) -> Result<(), AdministrationError> {
    digest.validate_shape()?;
    if digest.domain != expected {
        return Err(AdministrationError::DigestDomainMismatch {
            expected,
            actual: digest.domain,
        });
    }
    Ok(())
}

fn verify_freshness(
    evidence_at: i64,
    now: i64,
    max_age: i64,
    max_future_skew: i64,
    future_error: AdministrationError,
    stale_error: AdministrationError,
) -> Result<(), AdministrationError> {
    let delta = now as i128 - evidence_at as i128;
    if delta < -(max_future_skew as i128) {
        return Err(future_error);
    }
    if delta.max(0) > max_age as i128 {
        return Err(stale_error);
    }
    Ok(())
}

fn hash_json<T: Serialize>(
    domain: DigestDomain,
    tag: &[u8],
    value: &T,
) -> Result<VerifiedDigest, AdministrationError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| AdministrationError::Serialization(error.to_string()))?;
    let mut framed = Vec::with_capacity(tag.len() + 1 + encoded.len());
    framed.extend_from_slice(tag);
    framed.push(0);
    framed.extend_from_slice(&encoded);
    Ok(hash_canonical_bytes(domain, &framed)?)
}

#[derive(Debug, Error, PartialEq)]
pub enum AdministrationError {
    #[error("unsupported administration policy version {0}")]
    UnsupportedPolicyVersion(u16),
    #[error("administration policy id is required")]
    MissingPolicyId,
    #[error("invalid positive bounded age policy: {0}")]
    InvalidAgePolicy(&'static str),
    #[error("future-skew allowance must be between zero and five minutes")]
    InvalidFutureSkewPolicy,
    #[error("unsupported administration event version {0}")]
    UnsupportedEventVersion(u16),
    #[error("unsupported administration receipt version {0}")]
    UnsupportedReceiptVersion(u16),
    #[error("administration id is required")]
    MissingAdministrationId,
    #[error("administration subject is invalid")]
    InvalidSubject,
    #[error("administrator principal must be non-zero")]
    ZeroAdministratorPrincipal,
    #[error("rationale commitment cannot be zero")]
    ZeroRationaleCommitment,
    #[error("performed/partial administration requires typed performed details")]
    PerformedDetailsRequired,
    #[error("administration status/details combination is invalid")]
    InvalidStatusDetailCombination,
    #[error("Other reason requires protected rationale commitment")]
    OtherReasonRequiresCommitment,
    #[error("administered dose must be positive")]
    NonPositiveAdministeredDose,
    #[error("administration end precedes start")]
    AdministrationEndsBeforeStart,
    #[error("no current medication activation exists")]
    NoCurrentActivation,
    #[error("current medication activation is conflicted")]
    ActivationConflict,
    #[error("supplied activation is not current")]
    ActivationNotCurrent,
    #[error("activation targets another medication artifact")]
    ActivationMedicationMismatch,
    #[error("event targets another medication artifact")]
    EventMedicationMismatch,
    #[error("event activation lineage does not match current activation")]
    EventActivationMismatch,
    #[error("event administrator does not match authenticated administrator")]
    EventAdministratorMismatch,
    #[error("administration policy does not permit emergency-origin activation")]
    EmergencyActivationNotAllowed,
    #[error("activation-state observation is from the future")]
    ActivationStateFromFuture,
    #[error("activation-state observation is stale")]
    ActivationStateStale,
    #[error("patient subject does not match medication/event subject")]
    PatientSubjectMismatch,
    #[error("patient binding evidence does not match event")]
    PatientBindingEvidenceMismatch,
    #[error("patient binding is from the future")]
    PatientBindingFromFuture,
    #[error("patient binding is stale")]
    PatientBindingStale,
    #[error("dispense lineage for medication activation is missing")]
    DispenseLineageMissing,
    #[error("multiple dispense lineages match medication activation")]
    AmbiguousDispenseLineage,
    #[error("dispense lineage is conflicted")]
    DispenseLineageConflict,
    #[error("finalized dispense receipt is missing")]
    FinalizedDispenseMissing,
    #[error("finalized dispense receipt appears more than once")]
    AmbiguousFinalizedDispense,
    #[error("finalized dispense history is not contiguous through selected slot")]
    DispenseHistoryNotContiguous,
    #[error("supply evidence targets another medication/activation lineage")]
    SupplyLineageMismatch,
    #[error("event references another finalized dispense")]
    EventDispenseMismatch,
    #[error("dispense-state observation is from the future")]
    DispenseStateFromFuture,
    #[error("dispense-state observation is stale")]
    DispenseStateStale,
    #[error("non-performed/indeterminate event cannot create Administer capability")]
    NonPerformedEventCannotAuthorize,
    #[error("partial administration is not permitted by policy")]
    PartialAdministrationNotAllowed,
    #[error("dosage index cannot be represented on this platform")]
    DosageIndexOverflow,
    #[error("dosage index does not exist on exact medication order")]
    DosageIndexOutOfBounds,
    #[error("v1 does not infer concrete administration dose from ordered range")]
    OrderedDoseRangeUnsupportedV1,
    #[error("ordered dose must be positive")]
    InvalidOrderedDose,
    #[error("administered product does not exactly match ordered coded medication")]
    MedicationProductMismatch,
    #[error("administered route does not match ordered route")]
    RouteMismatch,
    #[error("ordered administration method is missing from event")]
    RequiredMethodMissing,
    #[error("administered method does not match ordered method")]
    MethodMismatch,
    #[error("ordered administration site is missing from event")]
    RequiredSiteMissing,
    #[error("administered site does not match ordered site")]
    SiteMismatch,
    #[error("administered dose unit does not match ordered dose unit")]
    DoseUnitMismatch,
    #[error("performed dose does not exactly match ordered dose")]
    PerformedDoseMismatch,
    #[error("partial dose must be positive and less than ordered dose")]
    InvalidPartialDose,
    #[error("administration clinical time is too far in the future")]
    AdministrationTimeFromFuture,
    #[error("administration was recorded after policy delay limit")]
    AdministrationRecordedTooLate,
    #[error("authority permit is not for administration")]
    WrongAuthorityPurpose,
    #[error("authority principal does not match authenticated administrator")]
    AuthorityPrincipalMismatch,
    #[error("authority permit targets another administration event")]
    AuthorityTargetMismatch,
    #[error("authority policy digest mismatch")]
    AuthorityPolicyMismatch,
    #[error("authority jurisdiction mismatch")]
    AuthorityJurisdictionMismatch,
    #[error("professional authority decision is from the future")]
    AuthorityFromFuture,
    #[error("professional authority decision is stale")]
    AuthorityStale,
    #[error("professional authority evidence is empty/invalid")]
    InvalidAuthorityEvidence,
    #[error("activation semantic receipt has wrong digest domain: {0:?}")]
    WrongActivationReceiptDomain(DigestDomain),
    #[error("digest domain mismatch: expected {expected:?}, got {actual:?}")]
    DigestDomainMismatch {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("medication artifact invalid for administration: {0}")]
    MedicationArtifact(String),
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error("clinical semantics invalid: {0}")]
    ClinicalSemantics(#[from] mycelix_clinical_semantics::ClinicalSemanticsError),
    #[error("clinical integrity invalid: {0}")]
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

    fn event(status: AdministrationStatusV1) -> MedicationAdministrationEventV1 {
        MedicationAdministrationEventV1 {
            schema_version: 1,
            administration_id: "admin-a".into(),
            medication_artifact_digest: stored(DigestDomain::MedicationRequestArtifact, 1),
            activation_semantic_receipt_digest: stored(DigestDomain::MedicationActivationReceipt, 2),
            finalized_dispense_receipt_digest: stored(DigestDomain::MedicationDispenseReceipt, 3),
            subject: SubjectRef {
                resource_type: "Patient".into(),
                id: "patient-a".into(),
            },
            patient_subject_binding_evidence_digest: stored(
                DigestDomain::PatientSubjectBindingEvidence,
                4,
            ),
            administrator_principal_binding: [5; 32],
            status,
            performed: None,
            nonperformance_reason: None,
            partial_reason: None,
            rationale_commitment: None,
        }
    }

    #[test]
    fn not_done_cannot_also_claim_performed_details() {
        let mut value = event(AdministrationStatusV1::NotDone);
        value.nonperformance_reason = Some(AdministrationNonPerformanceReasonV1::PatientRefused);
        value.performed = Some(PerformedAdministrationV1 {
            dosage_index: 0,
            product: CodeableConcept { coding: vec![], text: None },
            dose: Quantity::ucum(5.0, "mg"),
            route: CodeableConcept { coding: vec![], text: None },
            method: None,
            site: None,
            effective: AdministrationTimeV1 { start_micros: 10, end_micros: None },
            supply_detail_evidence_digest: None,
        });
        assert_eq!(
            value.validate_shape(),
            Err(AdministrationError::InvalidStatusDetailCombination)
        );
    }

    #[test]
    fn indeterminate_requires_rationale_commitment() {
        let value = event(AdministrationStatusV1::Indeterminate);
        assert_eq!(
            value.validate_shape(),
            Err(AdministrationError::InvalidStatusDetailCombination)
        );
    }

    #[test]
    fn administration_time_cannot_end_before_start() {
        let value = AdministrationTimeV1 { start_micros: 100, end_micros: Some(99) };
        assert_eq!(
            value.validate(),
            Err(AdministrationError::AdministrationEndsBeforeStart)
        );
    }
}
