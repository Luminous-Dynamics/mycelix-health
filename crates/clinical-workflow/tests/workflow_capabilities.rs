use mycelix_clinical_authority::{
    evaluate_authority, AuthorityPermit, AuthorityPurpose, AuthorityRequest, CredentialKind,
    EvidenceDigest, JurisdictionCode, PrincipalBinding, ResolvedCredentialEvidence, ScopeCode,
};
use mycelix_clinical_integrity::{hash_canonical_bytes, DigestDomain, VerifiedDigest};
use mycelix_clinical_quarantine::{
    ArtifactDigest, CommitmentScheme, DigestAlgorithm, OpaqueCommitment, QuarantineEvent,
    QuarantineReasonCode, QuarantineScope, ResolutionDisposition,
};
use mycelix_clinical_semantics::{CodeableConcept, Coding, Quantity, Ratio, SubjectRef, UCUM_SYSTEM};
use mycelix_clinical_workflow::{
    authorize_medication_activation, authorize_quarantine_resolution, hash_medication_order,
    hash_quarantine_event, WorkflowError, WorkflowPolicyV1,
};
use mycelix_medication_semantics::{
    AdministrationTiming, DosageInstruction, DoseAmount, MedicationOrder, MedicationOrderProvenance,
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

fn policy_digest(bytes: &[u8]) -> VerifiedDigest {
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

fn expect_workflow_error<T>(result: Result<T, WorkflowError>) -> WorkflowError {
    match result {
        Ok(_) => panic!("expected workflow denial"),
        Err(error) => error,
    }
}

#[test]
fn medication_authority_cannot_cross_order_boundary() {
    let p = principal(1);
    let policy = policy_digest(b"prescribing-policy-v1");
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

    let error = expect_workflow_error(authorize_medication_activation(
        &order_b,
        authority,
        policy,
        &workflow_policy(),
        p,
        &jurisdiction(),
        200,
    ));
    assert!(matches!(error, WorkflowError::TargetDigestMismatch));
}

#[test]
fn clinical_review_authority_cannot_be_replayed_as_prescribing() {
    let p = principal(2);
    let policy = policy_digest(b"review-policy-v1");
    let order = medication_order("order-a");
    let authority = authority_for(
        hash_medication_order(&order).unwrap(),
        policy,
        AuthorityPurpose::ClinicalReview,
        p,
        "prescribe",
        100,
    );

    let error = expect_workflow_error(authorize_medication_activation(
        &order,
        authority,
        policy,
        &workflow_policy(),
        p,
        &jurisdiction(),
        200,
    ));
    assert!(matches!(error, WorkflowError::WrongAuthorityPurpose { .. }));
}

#[test]
fn authenticated_principal_substitution_is_rejected() {
    let authorized = principal(3);
    let attacker = principal(4);
    let policy = policy_digest(b"prescribing-policy-v1");
    let order = medication_order("order-a");
    let authority = authority_for(
        hash_medication_order(&order).unwrap(),
        policy,
        AuthorityPurpose::Prescribe,
        authorized,
        "prescribe",
        100,
    );

    let error = expect_workflow_error(authorize_medication_activation(
        &order,
        authority,
        policy,
        &workflow_policy(),
        attacker,
        &jurisdiction(),
        200,
    ));
    assert!(matches!(error, WorkflowError::PrincipalMismatch));
}

#[test]
fn stale_authority_cannot_be_converted_to_fresh_workflow_capability() {
    let p = principal(5);
    let policy = policy_digest(b"prescribing-policy-v1");
    let order = medication_order("order-a");
    let authority = authority_for(
        hash_medication_order(&order).unwrap(),
        policy,
        AuthorityPurpose::Prescribe,
        p,
        "prescribe",
        0,
    );
    let strict = WorkflowPolicyV1 {
        schema_version: 1,
        max_authority_age_micros: 10,
        max_future_skew_micros: 0,
    };

    let error = expect_workflow_error(authorize_medication_activation(
        &order,
        authority,
        policy,
        &strict,
        p,
        &jurisdiction(),
        11,
    ));
    assert!(matches!(error, WorkflowError::AuthorityEvaluationStale));
}

#[test]
fn successful_medication_capability_is_bound_to_exact_digest() {
    let p = principal(6);
    let policy = policy_digest(b"prescribing-policy-v1");
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

#[test]
fn quarantine_capability_rejects_changed_event() {
    let pending = QuarantineEvent::new_pending(
        [1; 16],
        OpaqueCommitment {
            scheme: CommitmentScheme::HmacSha256,
            value: [2; 32],
        },
        QuarantineScope::ClinicalFact,
        vec![QuarantineReasonCode::SemanticValidationFailure],
        ArtifactDigest {
            algorithm: DigestAlgorithm::Blake3,
            value: [3; 32],
        },
        100,
    )
    .unwrap();
    let review = QuarantineEvent::begin_review(
        &pending,
        ArtifactDigest {
            algorithm: DigestAlgorithm::Blake3,
            value: [4; 32],
        },
        ArtifactDigest {
            algorithm: DigestAlgorithm::Blake3,
            value: [5; 32],
        },
        OpaqueCommitment {
            scheme: CommitmentScheme::HmacSha256,
            value: [6; 32],
        },
        110,
    )
    .unwrap();

    let p = principal(7);
    let policy = policy_digest(b"clinical-review-policy-v1");
    let authority = authority_for(
        hash_quarantine_event(&review).unwrap(),
        policy,
        AuthorityPurpose::ClinicalReview,
        p,
        "clinical-review",
        120,
    );
    let capability = authorize_quarantine_resolution(
        &review,
        ResolutionDisposition::Discard,
        authority,
        policy,
        &workflow_policy(),
        p,
        &jurisdiction(),
        130,
    )
    .unwrap();

    let mut changed = review.clone();
    changed.sequence += 1;
    let error = expect_workflow_error(capability.resolve(
        &changed,
        ArtifactDigest {
            algorithm: DigestAlgorithm::Blake3,
            value: [7; 32],
        },
        OpaqueCommitment {
            scheme: CommitmentScheme::HmacSha256,
            value: [8; 32],
        },
        ArtifactDigest {
            algorithm: DigestAlgorithm::Blake3,
            value: [9; 32],
        },
        None,
        140,
    ));
    assert!(matches!(error, WorkflowError::QuarantineTargetChanged));
}
