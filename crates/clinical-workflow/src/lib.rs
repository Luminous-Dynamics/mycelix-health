#![deny(unsafe_code)]
//! Exact-target capability composition for Mycelix-Health.
//!
//! A valid credential or qualification result is not sufficient on its own. This
//! crate consumes a verifier-owned `AuthorityPermit` and rebinds it to one exact
//! clinical artifact, purpose, policy, principal, jurisdiction, and execution time.
//! The resulting workflow capability is non-serializable and non-cloneable.

use mycelix_clinical_authority::{
    AuthorityPermit, AuthorityPurpose, EvidenceDigest, JurisdictionCode, PrincipalBinding,
};
use mycelix_clinical_evidence::ClinicalEvidenceCapsule;
use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use mycelix_clinical_promotion::{evaluate_for_clinical_presentation, GateDecision};
use mycelix_clinical_quarantine::{
    ArtifactDigest, DigestAlgorithm as QuarantineDigestAlgorithm, OpaqueCommitment, QuarantineEvent,
    QuarantineState, ResolutionDisposition,
};
use mycelix_medication_semantics::MedicationOrder;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_AUTHORITY_AGE_MICROS: i64 = 86_400_000_000; // 24 hours
const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000; // 5 minutes

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPolicyV1 {
    pub schema_version: u16,
    /// Maximum age of the authority evaluation when converted into a workflow
    /// capability. v1 caps this at 24 hours; deployments should normally be much
    /// stricter for prescribing and other high-impact actions.
    pub max_authority_age_micros: i64,
    /// Allowed clock skew when authority evaluation appears slightly in the future.
    /// v1 caps this at five minutes.
    pub max_future_skew_micros: i64,
}

impl WorkflowPolicyV1 {
    pub fn validate(&self) -> Result<(), WorkflowError> {
        if self.schema_version != 1 {
            return Err(WorkflowError::UnsupportedWorkflowPolicyVersion(
                self.schema_version,
            ));
        }
        if self.max_authority_age_micros <= 0
            || self.max_authority_age_micros > MAX_AUTHORITY_AGE_MICROS
        {
            return Err(WorkflowError::InvalidAuthorityAgePolicy);
        }
        if self.max_future_skew_micros < 0
            || self.max_future_skew_micros > MAX_FUTURE_SKEW_MICROS
        {
            return Err(WorkflowError::InvalidFutureSkewPolicy);
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, WorkflowError> {
        self.validate()?;
        hash_json(
            DigestDomain::WorkflowPolicy,
            b"mycelix-health/workflow-policy-v1",
            self,
        )
    }
}

/// Result of combining clinical qualification and practitioner authority for one
/// exact evidence capsule. No Clone/Copy/serde implementation is provided.
pub struct ClinicalPresentationCapability {
    capsule_id: String,
    capsule_digest: StoredDigest,
    principal: PrincipalBinding,
    jurisdiction: JurisdictionCode,
    authority_policy_digest: StoredDigest,
    workflow_policy_digest: StoredDigest,
    authorized_at_micros: i64,
    supporting_authority_evidence: Vec<[u8; 32]>,
}

impl ClinicalPresentationCapability {
    pub fn capsule_id(&self) -> &str {
        &self.capsule_id
    }

    pub fn capsule_digest(&self) -> StoredDigest {
        self.capsule_digest
    }

    pub fn principal(&self) -> PrincipalBinding {
        self.principal
    }

    pub fn jurisdiction(&self) -> &JurisdictionCode {
        &self.jurisdiction
    }

    pub fn authorized_at_micros(&self) -> i64 {
        self.authorized_at_micros
    }

    pub fn authority_policy_digest(&self) -> StoredDigest {
        self.authority_policy_digest
    }

    pub fn workflow_policy_digest(&self) -> StoredDigest {
        self.workflow_policy_digest
    }

    pub fn supporting_authority_evidence(&self) -> &[[u8; 32]] {
        &self.supporting_authority_evidence
    }
}

/// Single-owner capability for one exact active medication order candidate.
pub struct MedicationActivationCapability {
    order_id: String,
    order_digest: StoredDigest,
    principal: PrincipalBinding,
    jurisdiction: JurisdictionCode,
    authority_policy_digest: StoredDigest,
    workflow_policy_digest: StoredDigest,
    authorized_at_micros: i64,
    supporting_authority_evidence: Vec<[u8; 32]>,
}

impl MedicationActivationCapability {
    pub fn order_id(&self) -> &str {
        &self.order_id
    }

    pub fn order_digest(&self) -> StoredDigest {
        self.order_digest
    }

    pub fn principal(&self) -> PrincipalBinding {
        self.principal
    }

    pub fn jurisdiction(&self) -> &JurisdictionCode {
        &self.jurisdiction
    }

    pub fn authorized_at_micros(&self) -> i64 {
        self.authorized_at_micros
    }

    pub fn authority_policy_digest(&self) -> StoredDigest {
        self.authority_policy_digest
    }

    pub fn workflow_policy_digest(&self) -> StoredDigest {
        self.workflow_policy_digest
    }

    pub fn supporting_authority_evidence(&self) -> &[[u8; 32]] {
        &self.supporting_authority_evidence
    }
}

/// Single-owner capability for one exact quarantine event and one exact terminal
/// disposition. The safe `resolve` method consumes the capability.
pub struct QuarantineResolutionCapability {
    case_nonce: [u8; 16],
    sequence: u32,
    event_digest: VerifiedDigest,
    disposition: ResolutionDisposition,
    principal: PrincipalBinding,
    authority_policy_digest: StoredDigest,
    workflow_policy_digest: StoredDigest,
    authorized_at_micros: i64,
}

impl QuarantineResolutionCapability {
    pub fn case_nonce(&self) -> [u8; 16] {
        self.case_nonce
    }

    pub fn sequence(&self) -> u32 {
        self.sequence
    }

    pub fn event_digest(&self) -> StoredDigest {
        self.event_digest.stored()
    }

    pub fn disposition(&self) -> ResolutionDisposition {
        self.disposition
    }

    pub fn principal(&self) -> PrincipalBinding {
        self.principal
    }

    pub fn authority_policy_digest(&self) -> StoredDigest {
        self.authority_policy_digest
    }

    pub fn workflow_policy_digest(&self) -> StoredDigest {
        self.workflow_policy_digest
    }

    pub fn authorized_at_micros(&self) -> i64 {
        self.authorized_at_micros
    }

    /// Consume this capability to create exactly one permitted terminal transition.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve(
        self,
        previous: &QuarantineEvent,
        producer_artifact_digest: ArtifactDigest,
        reviewer_authority_commitment: OpaqueCommitment,
        decision_artifact_digest: ArtifactDigest,
        promoted_artifact_digest: Option<ArtifactDigest>,
        recorded_at_micros: i64,
    ) -> Result<QuarantineEvent, WorkflowError> {
        if previous.case_nonce != self.case_nonce || previous.sequence != self.sequence {
            return Err(WorkflowError::QuarantineTargetChanged);
        }
        let actual = hash_quarantine_event(previous)?;
        if actual != self.event_digest {
            return Err(WorkflowError::TargetDigestMismatch);
        }

        QuarantineEvent::resolve(
            previous,
            ArtifactDigest {
                algorithm: QuarantineDigestAlgorithm::Blake3,
                value: self.event_digest.value(),
            },
            producer_artifact_digest,
            reviewer_authority_commitment,
            self.disposition,
            decision_artifact_digest,
            promoted_artifact_digest,
            recorded_at_micros,
        )
        .map_err(|error| WorkflowError::Quarantine(error.to_string()))
    }
}

pub fn authorize_clinical_presentation(
    capsule: &ClinicalEvidenceCapsule,
    authority: AuthorityPermit,
    authority_policy_digest: VerifiedDigest,
    workflow_policy: &WorkflowPolicyV1,
    authenticated_principal: PrincipalBinding,
    jurisdiction: &JurisdictionCode,
    execution_at_micros: i64,
) -> Result<ClinicalPresentationCapability, WorkflowError> {
    let capsule_digest = hash_evidence_capsule(capsule)?;
    let workflow_policy_digest = workflow_policy.verified_digest()?;

    let evidence = verify_authority_binding(
        &authority,
        capsule_digest,
        authority_policy_digest,
        AuthorityPurpose::ClinicalReview,
        authenticated_principal,
        jurisdiction,
        workflow_policy,
        execution_at_micros,
    )?;

    let gate = evaluate_for_clinical_presentation(capsule)
        .map_err(|error| WorkflowError::Qualification(error.to_string()))?;
    if gate.decision != GateDecision::EligibleForClinicalPresentation {
        return Err(WorkflowError::ClinicalQualificationDenied(gate.decision));
    }

    Ok(ClinicalPresentationCapability {
        capsule_id: capsule.capsule_id.clone(),
        capsule_digest: capsule_digest.stored(),
        principal: authenticated_principal,
        jurisdiction: jurisdiction.clone(),
        authority_policy_digest: authority_policy_digest.stored(),
        workflow_policy_digest: workflow_policy_digest.stored(),
        authorized_at_micros: execution_at_micros,
        supporting_authority_evidence: evidence,
    })
}

pub fn authorize_medication_activation(
    order: &MedicationOrder,
    authority: AuthorityPermit,
    authority_policy_digest: VerifiedDigest,
    workflow_policy: &WorkflowPolicyV1,
    authenticated_principal: PrincipalBinding,
    jurisdiction: &JurisdictionCode,
    execution_at_micros: i64,
) -> Result<MedicationActivationCapability, WorkflowError> {
    order
        .validate_active_order_candidate()
        .map_err(|error| WorkflowError::Medication(error.to_string()))?;

    let order_digest = hash_medication_order(order)?;
    let workflow_policy_digest = workflow_policy.verified_digest()?;
    let evidence = verify_authority_binding(
        &authority,
        order_digest,
        authority_policy_digest,
        AuthorityPurpose::Prescribe,
        authenticated_principal,
        jurisdiction,
        workflow_policy,
        execution_at_micros,
    )?;

    Ok(MedicationActivationCapability {
        order_id: order.order_id.clone(),
        order_digest: order_digest.stored(),
        principal: authenticated_principal,
        jurisdiction: jurisdiction.clone(),
        authority_policy_digest: authority_policy_digest.stored(),
        workflow_policy_digest: workflow_policy_digest.stored(),
        authorized_at_micros: execution_at_micros,
        supporting_authority_evidence: evidence,
    })
}

pub fn authorize_quarantine_resolution(
    event: &QuarantineEvent,
    disposition: ResolutionDisposition,
    authority: AuthorityPermit,
    authority_policy_digest: VerifiedDigest,
    workflow_policy: &WorkflowPolicyV1,
    authenticated_principal: PrincipalBinding,
    jurisdiction: &JurisdictionCode,
    execution_at_micros: i64,
) -> Result<QuarantineResolutionCapability, WorkflowError> {
    if event.state != QuarantineState::UnderReview {
        return Err(WorkflowError::QuarantineNotUnderReview);
    }
    let event_digest = hash_quarantine_event(event)?;
    let workflow_policy_digest = workflow_policy.verified_digest()?;
    verify_authority_binding(
        &authority,
        event_digest,
        authority_policy_digest,
        AuthorityPurpose::ClinicalReview,
        authenticated_principal,
        jurisdiction,
        workflow_policy,
        execution_at_micros,
    )?;

    Ok(QuarantineResolutionCapability {
        case_nonce: event.case_nonce,
        sequence: event.sequence,
        event_digest,
        disposition,
        principal: authenticated_principal,
        authority_policy_digest: authority_policy_digest.stored(),
        workflow_policy_digest: workflow_policy_digest.stored(),
        authorized_at_micros: execution_at_micros,
    })
}

pub fn hash_evidence_capsule(
    capsule: &ClinicalEvidenceCapsule,
) -> Result<VerifiedDigest, WorkflowError> {
    hash_json(
        DigestDomain::EvidenceCapsule,
        b"mycelix-health/clinical-evidence-capsule-v1",
        capsule,
    )
}

pub fn hash_medication_order(order: &MedicationOrder) -> Result<VerifiedDigest, WorkflowError> {
    hash_json(
        DigestDomain::MedicationOrder,
        b"mycelix-health/medication-order-v1",
        order,
    )
}

pub fn hash_quarantine_event(event: &QuarantineEvent) -> Result<VerifiedDigest, WorkflowError> {
    hash_json(
        DigestDomain::QuarantineEvent,
        b"mycelix-health/quarantine-event-v1",
        event,
    )
}

fn hash_json<T: Serialize>(
    domain: DigestDomain,
    schema_tag: &[u8],
    value: &T,
) -> Result<VerifiedDigest, WorkflowError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| WorkflowError::CanonicalSerialization(error.to_string()))?;
    let mut framed = Vec::with_capacity(schema_tag.len() + 1 + encoded.len());
    framed.extend_from_slice(schema_tag);
    framed.push(0);
    framed.extend_from_slice(&encoded);
    Ok(hash_canonical_bytes(domain, &framed)?)
}

#[allow(clippy::too_many_arguments)]
fn verify_authority_binding(
    authority: &AuthorityPermit,
    target_digest: VerifiedDigest,
    authority_policy_digest: VerifiedDigest,
    expected_purpose: AuthorityPurpose,
    authenticated_principal: PrincipalBinding,
    jurisdiction: &JurisdictionCode,
    workflow_policy: &WorkflowPolicyV1,
    execution_at_micros: i64,
) -> Result<Vec<[u8; 32]>, WorkflowError> {
    authority_policy_digest.require_domain(DigestDomain::AuthorityPolicy)?;
    workflow_policy.validate()?;

    if authority.target_artifact_digest().0 != target_digest.value() {
        return Err(WorkflowError::TargetDigestMismatch);
    }
    if authority.policy_digest().0 != authority_policy_digest.value() {
        return Err(WorkflowError::AuthorityPolicyDigestMismatch);
    }
    if authority.purpose() != expected_purpose {
        return Err(WorkflowError::WrongAuthorityPurpose {
            expected: expected_purpose,
            actual: authority.purpose(),
        });
    }
    if authority.principal() != authenticated_principal {
        return Err(WorkflowError::PrincipalMismatch);
    }
    if authority.jurisdiction() != jurisdiction {
        return Err(WorkflowError::JurisdictionMismatch);
    }

    let delta = execution_at_micros as i128 - authority.evaluated_at_micros() as i128;
    if delta < -(workflow_policy.max_future_skew_micros as i128) {
        return Err(WorkflowError::AuthorityEvaluationFromFuture);
    }
    if delta > workflow_policy.max_authority_age_micros as i128 {
        return Err(WorkflowError::AuthorityEvaluationStale);
    }

    Ok(authority
        .supporting_evidence_digests()
        .iter()
        .map(|digest: &EvidenceDigest| digest.0)
        .collect())
}

#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("unsupported workflow policy version {0}")]
    UnsupportedWorkflowPolicyVersion(u16),
    #[error("workflow authority age must be > 0 and <= 24 hours")]
    InvalidAuthorityAgePolicy,
    #[error("workflow future-skew allowance must be between 0 and 5 minutes")]
    InvalidFutureSkewPolicy,
    #[error("failed to serialize exact artifact under its v1 canonical JSON contract: {0}")]
    CanonicalSerialization(String),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
    #[error("authority target digest does not match exact workflow artifact")]
    TargetDigestMismatch,
    #[error("authority policy digest does not match the verified policy identity")]
    AuthorityPolicyDigestMismatch,
    #[error("authority purpose mismatch: expected {expected:?}, got {actual:?}")]
    WrongAuthorityPurpose {
        expected: AuthorityPurpose,
        actual: AuthorityPurpose,
    },
    #[error("authority principal does not match authenticated workflow principal")]
    PrincipalMismatch,
    #[error("authority jurisdiction does not match workflow jurisdiction")]
    JurisdictionMismatch,
    #[error("authority evaluation is too far in the future for workflow policy")]
    AuthorityEvaluationFromFuture,
    #[error("authority evaluation is stale for workflow policy")]
    AuthorityEvaluationStale,
    #[error("clinical qualification gate denied presentation: {0:?}")]
    ClinicalQualificationDenied(GateDecision),
    #[error("clinical qualification validation failed: {0}")]
    Qualification(String),
    #[error("medication semantics rejected activation: {0}")]
    Medication(String),
    #[error("quarantine item must be under review before terminal resolution")]
    QuarantineNotUnderReview,
    #[error("quarantine target changed after capability issuance")]
    QuarantineTargetChanged,
    #[error("quarantine transition failed: {0}")]
    Quarantine(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_authority::{
        evaluate_authority, AuthorityRequest, CredentialKind, ResolvedCredentialEvidence, ScopeCode,
    };
    use mycelix_clinical_integrity::DigestDomain;
    use mycelix_clinical_semantics::{CodeableConcept, Coding, Quantity, Ratio, SubjectRef, UCUM_SYSTEM};
    use mycelix_medication_semantics::{
        AdministrationTiming, DosageInstruction, DoseAmount, MedicationOrderProvenance,
        MedicationRequestIntent, MedicationRequestStatus, RequesterAuthorityRef,
    };

    fn principal(byte: u8) -> PrincipalBinding {
        PrincipalBinding([byte; 32])
    }

    fn jurisdiction() -> JurisdictionCode {
        JurisdictionCode {
            system: "urn:iso:std:iso:3166-2".into(),
            code: "US-TX".into(),
        }
    }

    fn scope(code: &str) -> ScopeCode {
        ScopeCode {
            system: "https://mycelix.health/authority-scope/v1".into(),
            code: code.into(),
        }
    }

    fn authority_policy_digest(bytes: &[u8]) -> VerifiedDigest {
        hash_canonical_bytes(DigestDomain::AuthorityPolicy, bytes).unwrap()
    }

    fn workflow_policy() -> WorkflowPolicyV1 {
        WorkflowPolicyV1 {
            schema_version: 1,
            max_authority_age_micros: 60_000_000,
            max_future_skew_micros: 1_000_000,
        }
    }

    fn credential(
        p: PrincipalBinding,
        scope_code: &str,
        seed: u8,
    ) -> ResolvedCredentialEvidence {
        ResolvedCredentialEvidence::from_verified_record(
            CredentialKind::PractitionerLicense,
            p,
            vec![jurisdiction()],
            vec![scope(scope_code)],
            0,
            Some(10_000_000_000),
            None,
            EvidenceDigest([seed; 32]),
            EvidenceDigest([seed.wrapping_add(1); 32]),
            EvidenceDigest([seed.wrapping_add(2); 32]),
        )
        .unwrap()
    }

    fn authority_for(
        target: VerifiedDigest,
        policy: VerifiedDigest,
        purpose: AuthorityPurpose,
        p: PrincipalBinding,
        scope_code: &str,
        evaluated_at: i64,
    ) -> AuthorityPermit {
        let request = AuthorityRequest {
            principal: p,
            target_artifact_digest: EvidenceDigest(target.value()),
            policy_digest: EvidenceDigest(policy.value()),
            purpose,
            jurisdiction: jurisdiction(),
            required_credential_kinds: vec![CredentialKind::PractitionerLicense],
            required_scopes: vec![scope(scope_code)],
            evaluated_at_micros: evaluated_at,
        };
        evaluate_authority(&request, &[credential(p, scope_code, 11)])
            .unwrap()
            .into_permit()
            .expect("authority should be granted")
    }

    fn concept(system: &str, code: &str) -> CodeableConcept {
        CodeableConcept {
            coding: vec![Coding {
                system: system.into(),
                code: code.into(),
                display: None,
                version: None,
            }],
            text: None,
        }
    }

    fn quantity(value: f64, code: &str) -> Quantity {
        Quantity {
            value,
            display_unit: Some(code.into()),
            system: UCUM_SYSTEM.into(),
            code: code.into(),
        }
    }

    fn medication_order(id: &str) -> MedicationOrder {
        MedicationOrder {
            order_id: id.into(),
            subject: SubjectRef {
                resource_type: "Patient".into(),
                id: "patient-a".into(),
            },
            medication: concept("http://www.nlm.nih.gov/research/umls/rxnorm", "860975"),
            product_strength: Some(Ratio {
                numerator: quantity(500.0, "mg"),
                denominator: quantity(1.0, "{tablet}"),
            }),
            status: MedicationRequestStatus::Active,
            intent: MedicationRequestIntent::Order,
            dosage: vec![DosageInstruction {
                sequence: Some(1),
                narrative_sig: Some("Take one tablet twice daily".into()),
                patient_instruction: None,
                timing: AdministrationTiming::Scheduled {
                    frequency: 2,
                    period: quantity(1.0, "d"),
                },
                route: concept("http://snomed.info/sct", "26643006"),
                method: None,
                site: None,
                dose: DoseAmount::Quantity(quantity(1.0, "{tablet}")),
                rate: None,
                max_dose_per_period: Some(Ratio {
                    numerator: quantity(2.0, "{tablet}"),
                    denominator: quantity(1.0, "d"),
                }),
                max_dose_per_administration: Some(quantity(1.0, "{tablet}")),
                max_dose_per_lifetime: None,
            }],
            dispense_quantity: Some(quantity(60.0, "{tablet}")),
            refills_authorized: Some(1),
            requester: Some(RequesterAuthorityRef {
                requester_reference: "Practitioner/123".into(),
                credential_reference: Some("mycelix:credential:abc".into()),
            }),
            authored_at_micros: Some(1),
            provenance: MedicationOrderProvenance {
                source_system: "https://ehr.example/fhir".into(),
                source_resource_id: id.into(),
                source_version: Some("1".into()),
                recorded_at_micros: 1,
            },
        }
    }

    #[test]
    fn medication_capability_is_bound_to_exact_order_digest() {
        let p = principal(1);
        let policy = authority_policy_digest(b"prescribing-policy-v1");
        let order_a = medication_order("order-a");
        let order_b = medication_order("order-b");
        let authority = authority_for(
            hash_medication_order(&order_a).unwrap(),
            policy,
            AuthorityPurpose::Prescribe,
            p,
            "prescribe",
            100,
        );

        let error = authorize_medication_activation(
            &order_b,
            authority,
            policy,
            &workflow_policy(),
            p,
            &jurisdiction(),
            200,
        )
        .unwrap_err();
        assert!(matches!(error, WorkflowError::TargetDigestMismatch));
    }

    #[test]
    fn clinical_review_authority_cannot_be_replayed_as_prescribing_authority() {
        let p = principal(2);
        let policy = authority_policy_digest(b"clinical-review-policy-v1");
        let order = medication_order("order-a");
        let authority = authority_for(
            hash_medication_order(&order).unwrap(),
            policy,
            AuthorityPurpose::ClinicalReview,
            p,
            "prescribe",
            100,
        );
        let error = authorize_medication_activation(
            &order,
            authority,
            policy,
            &workflow_policy(),
            p,
            &jurisdiction(),
            200,
        )
        .unwrap_err();
        assert!(matches!(error, WorkflowError::WrongAuthorityPurpose { .. }));
    }

    #[test]
    fn wrong_authenticated_principal_is_rejected() {
        let authorized = principal(3);
        let attacker = principal(4);
        let policy = authority_policy_digest(b"prescribing-policy-v1");
        let order = medication_order("order-a");
        let authority = authority_for(
            hash_medication_order(&order).unwrap(),
            policy,
            AuthorityPurpose::Prescribe,
            authorized,
            "prescribe",
            100,
        );
        let error = authorize_medication_activation(
            &order,
            authority,
            policy,
            &workflow_policy(),
            attacker,
            &jurisdiction(),
            200,
        )
        .unwrap_err();
        assert!(matches!(error, WorkflowError::PrincipalMismatch));
    }

    #[test]
    fn stale_authority_is_rejected() {
        let p = principal(5);
        let policy = authority_policy_digest(b"prescribing-policy-v1");
        let order = medication_order("order-a");
        let authority = authority_for(
            hash_medication_order(&order).unwrap(),
            policy,
            AuthorityPurpose::Prescribe,
            p,
            "prescribe",
            0,
        );
        let error = authorize_medication_activation(
            &order,
            authority,
            policy,
            &WorkflowPolicyV1 {
                schema_version: 1,
                max_authority_age_micros: 10,
                max_future_skew_micros: 0,
            },
            p,
            &jurisdiction(),
            11,
        )
        .unwrap_err();
        assert!(matches!(error, WorkflowError::AuthorityEvaluationStale));
    }

    #[test]
    fn correct_medication_authority_produces_exact_capability() {
        let p = principal(6);
        let policy = authority_policy_digest(b"prescribing-policy-v1");
        let order = medication_order("order-a");
        let digest = hash_medication_order(&order).unwrap();
        let authority = authority_for(
            digest,
            policy,
            AuthorityPurpose::Prescribe,
            p,
            "prescribe",
            100,
        );
        let capability = authorize_medication_activation(
            &order,
            authority,
            policy,
            &workflow_policy(),
            p,
            &jurisdiction(),
            200,
        )
        .unwrap();
        assert_eq!(capability.order_id(), "order-a");
        assert_eq!(capability.order_digest().value, digest.value());
    }
}
