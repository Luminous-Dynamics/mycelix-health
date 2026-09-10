use mycelix_clinical_authority::{
    AuthorityPermit, AuthorityPurpose, JurisdictionCode, PrincipalBinding,
};
use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use mycelix_clinical_semantics::{CodeableConcept, Quantity};
use mycelix_fhir_medication_semantics::MedicationRequestArtifact;
use mycelix_medication_activation_state::{
    ActivationLifecycle, CurrentActivation, CurrentActivationState, MedicationActivationView,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use thiserror::Error;

const POLICY_SCHEMA_TAG: &[u8] = b"mycelix-health/medication-dispense-policy-v1";
const REQUEST_SCHEMA_TAG: &[u8] = b"mycelix-health/medication-dispense-request-v1";
const STATE_SCHEMA_TAG: &[u8] = b"mycelix-health/medication-activation-state-v1";
const LEDGER_SCHEMA_TAG: &[u8] = b"mycelix-health/medication-dispense-ledger-v1";
const PREFLIGHT_SCHEMA_TAG: &[u8] = b"mycelix-health/medication-dispense-preflight-v1";
const MAX_EVIDENCE_AGE_MICROS: i64 = 86_400_000_000;
const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DispensePolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    /// Emergency-origin medication may be dispensed only when deployment policy opts in.
    pub allow_emergency_override_activation: bool,
    pub max_authority_age_micros: i64,
    pub max_activation_state_age_micros: i64,
    pub max_pharmacy_context_age_micros: i64,
    pub max_ledger_snapshot_age_micros: i64,
    pub max_future_skew_micros: i64,
}

impl DispensePolicyV1 {
    pub fn validate(&self) -> Result<(), DispenseError> {
        if self.schema_version != 1 {
            return Err(DispenseError::UnsupportedPolicyVersion(self.schema_version));
        }
        if self.policy_id.trim().is_empty() {
            return Err(DispenseError::MissingPolicyId);
        }
        for (value, field) in [
            (self.max_authority_age_micros, "max_authority_age_micros"),
            (
                self.max_activation_state_age_micros,
                "max_activation_state_age_micros",
            ),
            (
                self.max_pharmacy_context_age_micros,
                "max_pharmacy_context_age_micros",
            ),
            (
                self.max_ledger_snapshot_age_micros,
                "max_ledger_snapshot_age_micros",
            ),
        ] {
            if value <= 0 || value > MAX_EVIDENCE_AGE_MICROS {
                return Err(DispenseError::InvalidAgePolicy(field));
            }
        }
        if self.max_future_skew_micros < 0
            || self.max_future_skew_micros > MAX_FUTURE_SKEW_MICROS
        {
            return Err(DispenseError::InvalidFutureSkewPolicy);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, DispenseError> {
        self.validate()?;
        hash_json(
            DigestDomain::MedicationDispensePolicy,
            POLICY_SCHEMA_TAG,
            self,
        )
    }
}

/// Activation provenance is carried through preflight instead of flattened to `active=true`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DispenseActivationProvenance {
    Qualified {
        activation_receipt_digest: StoredDigest,
    },
    EmergencyOverride {
        override_receipt_digest: StoredDigest,
        safety_assessment_digest: StoredDigest,
    },
}

impl DispenseActivationProvenance {
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

/// Non-serializable evidence that the canonical activation view contained exactly one
/// current lineage for this medication when observed.
pub struct VerifiedActivationForDispense {
    medication_artifact_digest: StoredDigest,
    activation_state_digest: StoredDigest,
    provenance: DispenseActivationProvenance,
    observed_at_micros: i64,
}

impl VerifiedActivationForDispense {
    pub fn medication_artifact_digest(&self) -> StoredDigest {
        self.medication_artifact_digest
    }

    pub fn activation_state_digest(&self) -> StoredDigest {
        self.activation_state_digest
    }

    pub fn provenance(&self) -> &DispenseActivationProvenance {
        &self.provenance
    }

    pub fn observed_at_micros(&self) -> i64 {
        self.observed_at_micros
    }
}

pub fn resolve_current_activation_for_dispense(
    view: &MedicationActivationView,
    medication_artifact_digest: VerifiedDigest,
    observed_at_micros: i64,
) -> Result<VerifiedActivationForDispense, DispenseError> {
    medication_artifact_digest.require_domain(DigestDomain::MedicationRequestArtifact)?;
    let state_digest = hash_json(DigestDomain::MedicationActivationState, STATE_SCHEMA_TAG, view)?;

    let provenance = match &view.current {
        CurrentActivationState::None => return Err(DispenseError::NoCurrentActivation),
        CurrentActivationState::Conflict { .. } => return Err(DispenseError::ActivationConflict),
        CurrentActivationState::One(CurrentActivation::Qualified(lineage)) => {
            if !matches!(&lineage.lifecycle, ActivationLifecycle::Current) {
                return Err(DispenseError::ActivationNotCurrent);
            }
            if lineage.medication_artifact_digest != medication_artifact_digest.stored() {
                return Err(DispenseError::ActivationMedicationMismatch);
            }
            require_domain(
                lineage.activation_receipt_digest,
                DigestDomain::MedicationActivationReceipt,
            )?;
            DispenseActivationProvenance::Qualified {
                activation_receipt_digest: lineage.activation_receipt_digest,
            }
        }
        CurrentActivationState::One(CurrentActivation::EmergencyOverride(lineage)) => {
            if !matches!(&lineage.lifecycle, ActivationLifecycle::Current) {
                return Err(DispenseError::ActivationNotCurrent);
            }
            if lineage.medication_artifact_digest != medication_artifact_digest.stored() {
                return Err(DispenseError::ActivationMedicationMismatch);
            }
            require_domain(
                lineage.override_receipt_digest,
                DigestDomain::EmergencyMedicationOverrideReceipt,
            )?;
            require_domain(
                lineage.safety_assessment_digest,
                DigestDomain::MedicationSafetyAssessment,
            )?;
            DispenseActivationProvenance::EmergencyOverride {
                override_receipt_digest: lineage.override_receipt_digest,
                safety_assessment_digest: lineage.safety_assessment_digest,
            }
        }
    };

    Ok(VerifiedActivationForDispense {
        medication_artifact_digest: medication_artifact_digest.stored(),
        activation_state_digest: state_digest.stored(),
        provenance,
        observed_at_micros,
    })
}

/// Evidence resolved by a trusted pharmacy adapter. This proves evidence shape and
/// temporal consistency; deployment trust in the adapter/source remains a higher-layer concern.
pub struct VerifiedPharmacyContext {
    principal: PrincipalBinding,
    pharmacy_id: String,
    pharmacy_record_digest: StoredDigest,
    affiliation_evidence_digest: StoredDigest,
    status_evidence_digest: StoredDigest,
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    revoked_at_micros: Option<i64>,
    resolved_at_micros: i64,
}

impl VerifiedPharmacyContext {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_evidence(
        principal: PrincipalBinding,
        pharmacy_id: String,
        pharmacy_record_digest: VerifiedDigest,
        affiliation_evidence_digest: VerifiedDigest,
        status_evidence_digest: VerifiedDigest,
        valid_from_micros: i64,
        valid_until_micros: Option<i64>,
        revoked_at_micros: Option<i64>,
        resolved_at_micros: i64,
    ) -> Result<Self, DispenseError> {
        if principal.0 == [0u8; 32] {
            return Err(DispenseError::ZeroPrincipal);
        }
        if pharmacy_id.trim().is_empty() {
            return Err(DispenseError::MissingPharmacyId);
        }
        pharmacy_record_digest.require_domain(DigestDomain::PharmacyRecord)?;
        affiliation_evidence_digest.require_domain(DigestDomain::PharmacyAffiliationEvidence)?;
        status_evidence_digest.require_domain(DigestDomain::PharmacyStatusEvidence)?;
        if let Some(valid_until) = valid_until_micros {
            if valid_until <= valid_from_micros {
                return Err(DispenseError::InvalidPharmacyValidityWindow);
            }
        }
        if let Some(revoked_at) = revoked_at_micros {
            if revoked_at < valid_from_micros {
                return Err(DispenseError::PharmacyRevocationPredatesValidity);
            }
        }
        Ok(Self {
            principal,
            pharmacy_id,
            pharmacy_record_digest: pharmacy_record_digest.stored(),
            affiliation_evidence_digest: affiliation_evidence_digest.stored(),
            status_evidence_digest: status_evidence_digest.stored(),
            valid_from_micros,
            valid_until_micros,
            revoked_at_micros,
            resolved_at_micros,
        })
    }

    fn ensure_active_at(&self, at_micros: i64) -> Result<(), DispenseError> {
        if at_micros < self.valid_from_micros {
            return Err(DispenseError::PharmacyContextNotYetValid);
        }
        if self.revoked_at_micros.is_some_and(|revoked| at_micros >= revoked) {
            return Err(DispenseError::PharmacyContextRevoked);
        }
        if self.valid_until_micros.is_some_and(|until| at_micros >= until) {
            return Err(DispenseError::PharmacyContextExpired);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MedicationDispenseRequestV1 {
    pub schema_version: u16,
    pub dispense_id: String,
    pub medication_artifact_digest: StoredDigest,
    pub pharmacy_id: String,
    /// Zero-based fill slot. Slot 0 is the initial dispense.
    pub slot_index: u32,
    pub product: CodeableConcept,
    pub quantity: Quantity,
}

impl MedicationDispenseRequestV1 {
    pub fn validate_against(
        &self,
        medication: &MedicationRequestArtifact,
    ) -> Result<(), DispenseError> {
        if self.schema_version != 1 {
            return Err(DispenseError::UnsupportedRequestVersion(self.schema_version));
        }
        if self.dispense_id.trim().is_empty() {
            return Err(DispenseError::MissingDispenseId);
        }
        if self.pharmacy_id.trim().is_empty() {
            return Err(DispenseError::MissingPharmacyId);
        }
        require_domain(
            self.medication_artifact_digest,
            DigestDomain::MedicationRequestArtifact,
        )?;
        self.product.validate_machine_actionable()?;
        self.quantity.validate_machine_actionable()?;
        if self.quantity.value <= 0.0 {
            return Err(DispenseError::NonPositiveDispenseQuantity);
        }

        let medication_digest = medication
            .verified_digest()
            .map_err(|error| DispenseError::MedicationArtifact(error.to_string()))?;
        if self.medication_artifact_digest != medication_digest.stored() {
            return Err(DispenseError::RequestMedicationMismatch);
        }
        if !same_coded_concept(&self.product, &medication.order().medication) {
            return Err(DispenseError::ProductSubstitutionNotSupported);
        }

        let ordered_quantity = medication
            .order()
            .dispense_quantity
            .as_ref()
            .ok_or(DispenseError::MissingExplicitDispenseQuantity)?;
        if !same_quantity_exact(&self.quantity, ordered_quantity) {
            return Err(DispenseError::NotExactFullFillQuantity);
        }

        let refills = medication
            .order()
            .refills_authorized
            .ok_or(DispenseError::MissingExplicitRefillCount)?;
        let total_slots = refills
            .checked_add(1)
            .ok_or(DispenseError::RefillCountOverflow)?;
        if self.slot_index >= total_slots {
            return Err(DispenseError::FillSlotExceedsAuthorization);
        }
        Ok(())
    }

    pub fn verified_digest(
        &self,
        medication: &MedicationRequestArtifact,
    ) -> Result<VerifiedDigest, DispenseError> {
        self.validate_against(medication)?;
        hash_json(
            DigestDomain::MedicationDispenseRequest,
            REQUEST_SCHEMA_TAG,
            self,
        )
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PriorDispenseEvidence {
    pub slot_index: u32,
    /// Must identify a finalized dispense, never a preflight receipt.
    pub receipt_digest: StoredDigest,
}

impl PriorDispenseEvidence {
    pub fn from_verified_final_receipt(
        slot_index: u32,
        receipt_digest: VerifiedDigest,
    ) -> Result<Self, DispenseError> {
        receipt_digest.require_domain(DigestDomain::MedicationDispenseReceipt)?;
        Ok(Self {
            slot_index,
            receipt_digest: receipt_digest.stored(),
        })
    }
}

/// Local evidence snapshot only. This does not reserve or consume a refill slot.
#[derive(Serialize)]
pub struct DispenseLedgerSnapshotV1 {
    medication_artifact_digest: StoredDigest,
    activation_semantic_receipt_digest: StoredDigest,
    prior_dispenses: Vec<PriorDispenseEvidence>,
    assembled_at_micros: i64,
}

impl DispenseLedgerSnapshotV1 {
    pub fn from_verified_final_receipts(
        medication_artifact_digest: VerifiedDigest,
        activation_semantic_receipt_digest: StoredDigest,
        mut prior_dispenses: Vec<PriorDispenseEvidence>,
        assembled_at_micros: i64,
    ) -> Result<Self, DispenseError> {
        medication_artifact_digest.require_domain(DigestDomain::MedicationRequestArtifact)?;
        require_activation_receipt_domain(activation_semantic_receipt_digest)?;
        let mut receipt_digests = HashSet::new();
        for prior in &prior_dispenses {
            require_domain(prior.receipt_digest, DigestDomain::MedicationDispenseReceipt)?;
            if !receipt_digests.insert(prior.receipt_digest) {
                return Err(DispenseError::DuplicatePriorDispenseReceipt);
            }
        }
        prior_dispenses.sort_by_key(|prior| prior.slot_index);
        for (expected, prior) in prior_dispenses.iter().enumerate() {
            let expected = u32::try_from(expected).map_err(|_| DispenseError::LedgerSlotOverflow)?;
            if prior.slot_index != expected {
                return Err(DispenseError::NonContiguousDispenseLedger);
            }
        }
        Ok(Self {
            medication_artifact_digest: medication_artifact_digest.stored(),
            activation_semantic_receipt_digest,
            prior_dispenses,
            assembled_at_micros,
        })
    }

    pub fn next_candidate_slot(&self) -> Result<u32, DispenseError> {
        u32::try_from(self.prior_dispenses.len()).map_err(|_| DispenseError::LedgerSlotOverflow)
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, DispenseError> {
        hash_json(
            DigestDomain::MedicationDispenseLedgerSnapshot,
            LEDGER_SCHEMA_TAG,
            self,
        )
    }
}

/// Non-cloneable/non-serializable evidence that local clinical, pharmacy, authority,
/// and prior-final-dispense checks passed. It is explicitly NOT permission to release
/// medication because it does not prove exclusive refill-slot allocation.
pub struct MedicationDispensePreflightCapability {
    dispense_id: String,
    dispense_request_digest: StoredDigest,
    medication_artifact_digest: StoredDigest,
    activation_state_digest: StoredDigest,
    activation_provenance: DispenseActivationProvenance,
    activation_observed_at_micros: i64,
    ledger_snapshot_digest: StoredDigest,
    candidate_slot_index: u32,
    pharmacist_principal: PrincipalBinding,
    pharmacy_id: String,
    pharmacy_record_digest: StoredDigest,
    pharmacy_affiliation_evidence_digest: StoredDigest,
    pharmacy_status_evidence_digest: StoredDigest,
    pharmacy_context_resolved_at_micros: i64,
    authority_policy_digest: StoredDigest,
    dispense_policy_digest: StoredDigest,
    evaluated_at_micros: i64,
    supporting_authority_evidence: Vec<[u8; 32]>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MedicationDispensePreflightReceiptV1 {
    pub schema_version: u16,
    pub dispense_id: String,
    pub dispense_request_digest: StoredDigest,
    pub medication_artifact_digest: StoredDigest,
    pub activation_state_digest: StoredDigest,
    pub activation_provenance: DispenseActivationProvenance,
    pub activation_observed_at_micros: i64,
    pub ledger_snapshot_digest: StoredDigest,
    pub candidate_slot_index: u32,
    pub pharmacist_principal: [u8; 32],
    pub pharmacy_id: String,
    pub pharmacy_record_digest: StoredDigest,
    pub pharmacy_affiliation_evidence_digest: StoredDigest,
    pub pharmacy_status_evidence_digest: StoredDigest,
    pub pharmacy_context_resolved_at_micros: i64,
    pub authority_policy_digest: StoredDigest,
    pub dispense_policy_digest: StoredDigest,
    pub evaluated_at_micros: i64,
    pub supporting_authority_evidence: Vec<[u8; 32]>,
}

impl MedicationDispensePreflightReceiptV1 {
    pub fn validate_shape(&self) -> Result<(), DispenseError> {
        if self.schema_version != 1 {
            return Err(DispenseError::UnsupportedPreflightReceiptVersion(
                self.schema_version,
            ));
        }
        if self.dispense_id.trim().is_empty() || self.pharmacy_id.trim().is_empty() {
            return Err(DispenseError::IncompleteReceiptIdentity);
        }
        if self.pharmacist_principal == [0u8; 32] {
            return Err(DispenseError::ZeroPrincipal);
        }
        require_domain(
            self.dispense_request_digest,
            DigestDomain::MedicationDispenseRequest,
        )?;
        require_domain(
            self.medication_artifact_digest,
            DigestDomain::MedicationRequestArtifact,
        )?;
        require_domain(
            self.activation_state_digest,
            DigestDomain::MedicationActivationState,
        )?;
        validate_activation_provenance(&self.activation_provenance)?;
        require_domain(
            self.ledger_snapshot_digest,
            DigestDomain::MedicationDispenseLedgerSnapshot,
        )?;
        require_domain(self.pharmacy_record_digest, DigestDomain::PharmacyRecord)?;
        require_domain(
            self.pharmacy_affiliation_evidence_digest,
            DigestDomain::PharmacyAffiliationEvidence,
        )?;
        require_domain(
            self.pharmacy_status_evidence_digest,
            DigestDomain::PharmacyStatusEvidence,
        )?;
        require_domain(self.authority_policy_digest, DigestDomain::AuthorityPolicy)?;
        require_domain(
            self.dispense_policy_digest,
            DigestDomain::MedicationDispensePolicy,
        )?;
        if self.supporting_authority_evidence.is_empty()
            || self
                .supporting_authority_evidence
                .iter()
                .any(|digest| *digest == [0u8; 32])
        {
            return Err(DispenseError::InvalidAuthorityEvidence);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, DispenseError> {
        self.validate_shape()?;
        hash_json(
            DigestDomain::MedicationDispensePreflightReceipt,
            PREFLIGHT_SCHEMA_TAG,
            self,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub fn authorize_medication_dispense_preflight(
    medication: &MedicationRequestArtifact,
    activation: VerifiedActivationForDispense,
    ledger: &DispenseLedgerSnapshotV1,
    request: &MedicationDispenseRequestV1,
    pharmacy: &VerifiedPharmacyContext,
    authority: AuthorityPermit,
    authority_policy_digest: VerifiedDigest,
    dispense_policy: &DispensePolicyV1,
    authenticated_principal: PrincipalBinding,
    jurisdiction: &JurisdictionCode,
    execution_at_micros: i64,
) -> Result<MedicationDispensePreflightCapability, DispenseError> {
    dispense_policy.validate()?;
    medication
        .validate_for_activation(execution_at_micros)
        .map_err(|error| DispenseError::MedicationArtifact(error.to_string()))?;
    let medication_digest = medication
        .verified_digest()
        .map_err(|error| DispenseError::MedicationArtifact(error.to_string()))?;
    medication_digest.require_domain(DigestDomain::MedicationRequestArtifact)?;

    request.validate_against(medication)?;
    let request_digest = request.verified_digest(medication)?;

    if activation.medication_artifact_digest != medication_digest.stored() {
        return Err(DispenseError::ActivationMedicationMismatch);
    }
    if activation.provenance.is_emergency()
        && !dispense_policy.allow_emergency_override_activation
    {
        return Err(DispenseError::EmergencyActivationNotAllowed);
    }
    verify_freshness(
        activation.observed_at_micros,
        execution_at_micros,
        dispense_policy.max_activation_state_age_micros,
        dispense_policy.max_future_skew_micros,
        DispenseError::ActivationStateFromFuture,
        DispenseError::ActivationStateStale,
    )?;

    if ledger.medication_artifact_digest != medication_digest.stored() {
        return Err(DispenseError::LedgerMedicationMismatch);
    }
    if ledger.activation_semantic_receipt_digest != activation.provenance.semantic_receipt_digest() {
        return Err(DispenseError::LedgerActivationMismatch);
    }
    if request.slot_index != ledger.next_candidate_slot()? {
        return Err(DispenseError::FillSlotDoesNotFollowLedger);
    }
    verify_freshness(
        ledger.assembled_at_micros,
        execution_at_micros,
        dispense_policy.max_ledger_snapshot_age_micros,
        dispense_policy.max_future_skew_micros,
        DispenseError::LedgerSnapshotFromFuture,
        DispenseError::LedgerSnapshotStale,
    )?;
    let ledger_digest = ledger.verified_digest()?;

    pharmacy.ensure_active_at(execution_at_micros)?;
    if pharmacy.principal != authenticated_principal {
        return Err(DispenseError::PharmacyPrincipalMismatch);
    }
    if pharmacy.pharmacy_id != request.pharmacy_id {
        return Err(DispenseError::RequestPharmacyMismatch);
    }
    verify_freshness(
        pharmacy.resolved_at_micros,
        execution_at_micros,
        dispense_policy.max_pharmacy_context_age_micros,
        dispense_policy.max_future_skew_micros,
        DispenseError::PharmacyContextFromFuture,
        DispenseError::PharmacyContextStale,
    )?;

    authority_policy_digest.require_domain(DigestDomain::AuthorityPolicy)?;
    if authority.purpose() != AuthorityPurpose::Dispense {
        return Err(DispenseError::WrongAuthorityPurpose);
    }
    if authority.principal() != authenticated_principal {
        return Err(DispenseError::AuthorityPrincipalMismatch);
    }
    if authority.target_artifact_digest().0 != request_digest.value() {
        return Err(DispenseError::AuthorityTargetMismatch);
    }
    if authority.policy_digest().0 != authority_policy_digest.value() {
        return Err(DispenseError::AuthorityPolicyMismatch);
    }
    if authority.jurisdiction() != jurisdiction {
        return Err(DispenseError::AuthorityJurisdictionMismatch);
    }
    verify_freshness(
        authority.evaluated_at_micros(),
        execution_at_micros,
        dispense_policy.max_authority_age_micros,
        dispense_policy.max_future_skew_micros,
        DispenseError::AuthorityFromFuture,
        DispenseError::AuthorityStale,
    )?;
    if authority.supporting_evidence_digests().is_empty() {
        return Err(DispenseError::InvalidAuthorityEvidence);
    }

    let dispense_policy_digest = dispense_policy.verified_digest()?;
    Ok(MedicationDispensePreflightCapability {
        dispense_id: request.dispense_id.clone(),
        dispense_request_digest: request_digest.stored(),
        medication_artifact_digest: medication_digest.stored(),
        activation_state_digest: activation.activation_state_digest,
        activation_provenance: activation.provenance,
        activation_observed_at_micros: activation.observed_at_micros,
        ledger_snapshot_digest: ledger_digest.stored(),
        candidate_slot_index: request.slot_index,
        pharmacist_principal: authenticated_principal,
        pharmacy_id: pharmacy.pharmacy_id.clone(),
        pharmacy_record_digest: pharmacy.pharmacy_record_digest,
        pharmacy_affiliation_evidence_digest: pharmacy.affiliation_evidence_digest,
        pharmacy_status_evidence_digest: pharmacy.status_evidence_digest,
        pharmacy_context_resolved_at_micros: pharmacy.resolved_at_micros,
        authority_policy_digest: authority_policy_digest.stored(),
        dispense_policy_digest: dispense_policy_digest.stored(),
        evaluated_at_micros: execution_at_micros,
        supporting_authority_evidence: authority
            .supporting_evidence_digests()
            .iter()
            .map(|digest| digest.0)
            .collect(),
    })
}

pub fn consume_dispense_preflight(
    capability: MedicationDispensePreflightCapability,
) -> Result<(MedicationDispensePreflightReceiptV1, VerifiedDigest), DispenseError> {
    let receipt = MedicationDispensePreflightReceiptV1 {
        schema_version: 1,
        dispense_id: capability.dispense_id,
        dispense_request_digest: capability.dispense_request_digest,
        medication_artifact_digest: capability.medication_artifact_digest,
        activation_state_digest: capability.activation_state_digest,
        activation_provenance: capability.activation_provenance,
        activation_observed_at_micros: capability.activation_observed_at_micros,
        ledger_snapshot_digest: capability.ledger_snapshot_digest,
        candidate_slot_index: capability.candidate_slot_index,
        pharmacist_principal: capability.pharmacist_principal.0,
        pharmacy_id: capability.pharmacy_id,
        pharmacy_record_digest: capability.pharmacy_record_digest,
        pharmacy_affiliation_evidence_digest: capability.pharmacy_affiliation_evidence_digest,
        pharmacy_status_evidence_digest: capability.pharmacy_status_evidence_digest,
        pharmacy_context_resolved_at_micros: capability.pharmacy_context_resolved_at_micros,
        authority_policy_digest: capability.authority_policy_digest,
        dispense_policy_digest: capability.dispense_policy_digest,
        evaluated_at_micros: capability.evaluated_at_micros,
        supporting_authority_evidence: capability.supporting_authority_evidence,
    };
    let digest = receipt.verified_digest()?;
    Ok((receipt, digest))
}

fn same_quantity_exact(left: &Quantity, right: &Quantity) -> bool {
    left.system == right.system
        && left.code == right.code
        && left.value.to_bits() == right.value.to_bits()
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

fn validate_activation_provenance(
    provenance: &DispenseActivationProvenance,
) -> Result<(), DispenseError> {
    match provenance {
        DispenseActivationProvenance::Qualified {
            activation_receipt_digest,
        } => require_domain(
            *activation_receipt_digest,
            DigestDomain::MedicationActivationReceipt,
        ),
        DispenseActivationProvenance::EmergencyOverride {
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

fn require_activation_receipt_domain(digest: StoredDigest) -> Result<(), DispenseError> {
    digest.validate_shape()?;
    if !matches!(
        digest.domain,
        DigestDomain::MedicationActivationReceipt
            | DigestDomain::EmergencyMedicationOverrideReceipt
    ) {
        return Err(DispenseError::InvalidActivationReceiptDomain(digest.domain));
    }
    Ok(())
}

fn require_domain(digest: StoredDigest, domain: DigestDomain) -> Result<(), DispenseError> {
    digest.validate_shape()?;
    if digest.domain != domain {
        return Err(DispenseError::DigestDomainMismatch {
            expected: domain,
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
    future_error: DispenseError,
    stale_error: DispenseError,
) -> Result<(), DispenseError> {
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
    schema_tag: &[u8],
    value: &T,
) -> Result<VerifiedDigest, DispenseError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| DispenseError::Serialization(error.to_string()))?;
    let mut framed = Vec::with_capacity(schema_tag.len() + 1 + encoded.len());
    framed.extend_from_slice(schema_tag);
    framed.push(0);
    framed.extend_from_slice(&encoded);
    Ok(hash_canonical_bytes(domain, &framed)?)
}

#[derive(Debug, Error, PartialEq)]
pub enum DispenseError {
    #[error("unsupported dispense policy version {0}")]
    UnsupportedPolicyVersion(u16),
    #[error("dispense policy id is required")]
    MissingPolicyId,
    #[error("invalid positive bounded age policy: {0}")]
    InvalidAgePolicy(&'static str),
    #[error("future-skew allowance must be between zero and five minutes")]
    InvalidFutureSkewPolicy,
    #[error("unsupported dispense request version {0}")]
    UnsupportedRequestVersion(u16),
    #[error("unsupported dispense preflight receipt version {0}")]
    UnsupportedPreflightReceiptVersion(u16),
    #[error("dispense id is required")]
    MissingDispenseId,
    #[error("pharmacy id is required")]
    MissingPharmacyId,
    #[error("dispense preflight receipt identity is incomplete")]
    IncompleteReceiptIdentity,
    #[error("principal binding must be non-zero")]
    ZeroPrincipal,
    #[error("dispense quantity must be positive")]
    NonPositiveDispenseQuantity,
    #[error("dispense request targets another medication artifact")]
    RequestMedicationMismatch,
    #[error("v1 does not infer medication-product substitution")]
    ProductSubstitutionNotSupported,
    #[error("v1 requires an explicit typed dispense quantity on the medication order")]
    MissingExplicitDispenseQuantity,
    #[error("v1 requires an explicit authorized-refill count")]
    MissingExplicitRefillCount,
    #[error("authorized refill count overflows total fill slots")]
    RefillCountOverflow,
    #[error("requested fill slot exceeds the medication order authorization")]
    FillSlotExceedsAuthorization,
    #[error("v1 supports only exact full-fill quantity; partial/over fills are not inferred")]
    NotExactFullFillQuantity,
    #[error("no current qualified/emergency medication activation exists")]
    NoCurrentActivation,
    #[error("multiple current medication activation lineages conflict")]
    ActivationConflict,
    #[error("supplied activation lineage is terminated")]
    ActivationNotCurrent,
    #[error("activation targets another medication artifact")]
    ActivationMedicationMismatch,
    #[error("deployment dispense policy does not permit emergency-origin activation")]
    EmergencyActivationNotAllowed,
    #[error("activation-state observation is from the future")]
    ActivationStateFromFuture,
    #[error("activation-state observation is stale")]
    ActivationStateStale,
    #[error("duplicate prior finalized dispense receipt in ledger snapshot")]
    DuplicatePriorDispenseReceipt,
    #[error("dispense ledger slots must be contiguous from zero")]
    NonContiguousDispenseLedger,
    #[error("dispense ledger slot count exceeds u32")]
    LedgerSlotOverflow,
    #[error("dispense ledger targets another medication")]
    LedgerMedicationMismatch,
    #[error("dispense ledger targets another activation lineage")]
    LedgerActivationMismatch,
    #[error("requested fill slot does not equal the next locally observed candidate slot")]
    FillSlotDoesNotFollowLedger,
    #[error("dispense ledger snapshot is from the future")]
    LedgerSnapshotFromFuture,
    #[error("dispense ledger snapshot is stale")]
    LedgerSnapshotStale,
    #[error("pharmacy evidence validity end must follow start")]
    InvalidPharmacyValidityWindow,
    #[error("pharmacy revocation cannot predate validity")]
    PharmacyRevocationPredatesValidity,
    #[error("pharmacy context is not yet valid")]
    PharmacyContextNotYetValid,
    #[error("pharmacy context is expired")]
    PharmacyContextExpired,
    #[error("pharmacy context is revoked")]
    PharmacyContextRevoked,
    #[error("pharmacy context principal does not match authenticated pharmacist")]
    PharmacyPrincipalMismatch,
    #[error("dispense request targets another pharmacy")]
    RequestPharmacyMismatch,
    #[error("pharmacy context resolution is from the future")]
    PharmacyContextFromFuture,
    #[error("pharmacy context resolution is stale")]
    PharmacyContextStale,
    #[error("authority permit is not for dispensing")]
    WrongAuthorityPurpose,
    #[error("authority principal does not match authenticated pharmacist")]
    AuthorityPrincipalMismatch,
    #[error("authority permit targets another dispense request")]
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
    InvalidActivationReceiptDomain(DigestDomain),
    #[error("digest domain mismatch: expected {expected:?}, got {actual:?}")]
    DigestDomainMismatch {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("medication artifact invalid for dispensing: {0}")]
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

    fn stored(domain: DigestDomain, byte: u8) -> StoredDigest {
        StoredDigest {
            algorithm: DigestAlgorithm::Blake3_256,
            domain,
            value: [byte; 32],
        }
    }

    fn verified(domain: DigestDomain, byte: u8) -> VerifiedDigest {
        hash_canonical_bytes(domain, &[byte]).unwrap()
    }

    fn quantity(value: f64, code: &str) -> Quantity {
        Quantity {
            value,
            display_unit: Some(code.into()),
            system: "http://unitsofmeasure.org".into(),
            code: code.into(),
        }
    }

    #[test]
    fn exact_quantity_ignores_display_but_not_value_or_code() {
        let a = quantity(30.0, "{tablet}");
        let mut b = a.clone();
        b.display_unit = Some("tablets".into());
        assert!(same_quantity_exact(&a, &b));
        b.value = 15.0;
        assert!(!same_quantity_exact(&a, &b));
    }

    #[test]
    fn ledger_requires_contiguous_unique_final_receipts() {
        let medication = verified(DigestDomain::MedicationRequestArtifact, 1);
        let activation = stored(DigestDomain::MedicationActivationReceipt, 2);
        let receipt0 = PriorDispenseEvidence::from_verified_final_receipt(
            0,
            verified(DigestDomain::MedicationDispenseReceipt, 3),
        )
        .unwrap();
        let receipt2 = PriorDispenseEvidence::from_verified_final_receipt(
            2,
            verified(DigestDomain::MedicationDispenseReceipt, 4),
        )
        .unwrap();
        let result = DispenseLedgerSnapshotV1::from_verified_final_receipts(
            medication,
            activation,
            vec![receipt0, receipt2],
            100,
        );
        assert!(matches!(
            result,
            Err(DispenseError::NonContiguousDispenseLedger)
        ));
    }

    #[test]
    fn preflight_receipt_cannot_count_as_prior_final_dispense() {
        let preflight = verified(DigestDomain::MedicationDispensePreflightReceipt, 9);
        assert!(matches!(
            PriorDispenseEvidence::from_verified_final_receipt(0, preflight),
            Err(DispenseError::Integrity(IntegrityError::WrongDomain { .. }))
        ));
    }

    #[test]
    fn policy_rejects_unbounded_freshness() {
        let policy = DispensePolicyV1 {
            schema_version: 1,
            policy_id: "dispense-v1".into(),
            allow_emergency_override_activation: false,
            max_authority_age_micros: MAX_EVIDENCE_AGE_MICROS + 1,
            max_activation_state_age_micros: 1_000,
            max_pharmacy_context_age_micros: 1_000,
            max_ledger_snapshot_age_micros: 1_000,
            max_future_skew_micros: 0,
        };
        assert!(matches!(
            policy.validate(),
            Err(DispenseError::InvalidAgePolicy(_))
        ));
    }

    #[test]
    fn activation_receipt_domains_do_not_accept_dispense_receipt() {
        let error = require_activation_receipt_domain(stored(
            DigestDomain::MedicationDispenseReceipt,
            1,
        ));
        assert!(matches!(
            error,
            Err(DispenseError::InvalidActivationReceiptDomain(
                DigestDomain::MedicationDispenseReceipt
            ))
        ));
    }
}
