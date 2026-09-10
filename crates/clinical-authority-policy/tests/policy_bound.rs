use mycelix_clinical_authority::{
    AuthorityDecision, CredentialKind, EvidenceDigest, JurisdictionCode, PrincipalBinding,
    ResolvedCredentialEvidence, ScopeCode,
};
use mycelix_clinical_authority_policy::{
    evaluate_policy_bound_authority, AuthorityPolicyV1, AuthorityPurposeV1,
    CredentialRequirementV1,
};

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

fn principal() -> PrincipalBinding {
    PrincipalBinding([7u8; 32])
}

fn credential() -> ResolvedCredentialEvidence {
    ResolvedCredentialEvidence::from_verified_record(
        CredentialKind::PractitionerLicense,
        principal(),
        vec![jurisdiction()],
        vec![scope("clinical.review"), scope("causality.review")],
        100,
        Some(10_000),
        None,
        EvidenceDigest([1u8; 32]),
        EvidenceDigest([2u8; 32]),
        EvidenceDigest([3u8; 32]),
    )
    .unwrap()
}

fn policy(scopes: Vec<ScopeCode>) -> AuthorityPolicyV1 {
    AuthorityPolicyV1 {
        schema_version: 1,
        policy_id: "causal-review-v1".into(),
        purpose: AuthorityPurposeV1::ClinicalReview,
        jurisdiction: jurisdiction(),
        required_credentials: vec![CredentialRequirementV1::PractitionerLicense],
        required_scopes: scopes,
    }
}

#[test]
fn bound_policy_enforces_its_exact_scope_set() {
    let policy = policy(vec![scope("clinical.review"), scope("causality.review")]);
    let evaluation = evaluate_policy_bound_authority(
        &policy,
        principal(),
        EvidenceDigest([9u8; 32]),
        &[credential()],
        1_000,
    )
    .unwrap();
    assert_eq!(evaluation.decision, AuthorityDecision::Granted);
    let permit = evaluation.into_permit().expect("granted evaluation has permit");
    assert_eq!(permit.policy_digest(), policy.verified_digest().unwrap().stored());
}

#[test]
fn adding_an_unmet_scope_changes_policy_and_denies_authority() {
    let weaker = policy(vec![scope("clinical.review"), scope("causality.review")]);
    let stronger = policy(vec![
        scope("clinical.review"),
        scope("causality.review"),
        scope("causality.specialist-review"),
    ]);
    assert_ne!(
        weaker.verified_digest().unwrap().stored(),
        stronger.verified_digest().unwrap().stored()
    );

    let evaluation = evaluate_policy_bound_authority(
        &stronger,
        principal(),
        EvidenceDigest([9u8; 32]),
        &[credential()],
        1_000,
    )
    .unwrap();
    assert_eq!(evaluation.decision, AuthorityDecision::Denied);
    assert!(evaluation.into_permit().is_none());
}

#[test]
fn purpose_is_part_of_policy_identity() {
    let review = policy(vec![scope("clinical.review")]);
    let mut prescribe = review.clone();
    prescribe.purpose = AuthorityPurposeV1::Prescribe;
    assert_ne!(
        review.verified_digest().unwrap().stored(),
        prescribe.verified_digest().unwrap().stored()
    );
}
