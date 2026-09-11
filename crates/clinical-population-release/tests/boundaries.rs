use mycelix_clinical_integrity::{DigestAlgorithm, DigestDomain, StoredDigest};
use mycelix_clinical_population_release::*;

fn digest(domain: DigestDomain, seed: u8) -> StoredDigest {
    StoredDigest {
        algorithm: DigestAlgorithm::Blake3_256,
        domain,
        value: [seed; 32],
    }
}

fn fraction(numerator: u64, denominator: u64) -> ReducedFractionV1 {
    ReducedFractionV1 {
        numerator,
        denominator,
    }
}

fn loss(epsilon_n: u64, epsilon_d: u64, delta_n: u64, delta_d: u64) -> PrivacyLossV1 {
    PrivacyLossV1 {
        epsilon: fraction(epsilon_n, epsilon_d),
        delta: fraction(delta_n, delta_d),
    }
}

fn policy() -> PopulationReleasePolicyV1 {
    PopulationReleasePolicyV1 {
        schema_version: 1,
        policy_id: "population-release-policy-v1".into(),
        static_minimum_cell_count: 11,
        max_static_report_cells: 10_000,
        allowed_dp_mechanisms: vec![DpMechanismKindV1::Gaussian],
        max_per_release_privacy_loss: loss(1, 2, 1, 100_000),
        max_cumulative_privacy_loss: loss(2, 1, 1, 1_000),
        max_interactive_queries: 100,
        accountant_instance_digest: digest(DigestDomain::ClinicalArtifact, 1),
        accountant_method_digest: digest(DigestDomain::ClinicalArtifact, 2),
    }
}

fn static_evidence() -> StaticSuppressionEvidenceV1 {
    StaticSuppressionEvidenceV1 {
        schema_version: 1,
        source_finding_kind: PopulationFindingKindV1::DenominatorBasedAssociation,
        source_finding_digest: digest(DigestDomain::ClinicalPopulationAssociation, 10),
        report_specification_digest: digest(DigestDomain::ClinicalArtifact, 11),
        raw_aggregate_digest: digest(DigestDomain::ClinicalArtifact, 12),
        public_output_digest: digest(DigestDomain::ClinicalArtifact, 13),
        total_report_cells: 40,
        minimum_cell_threshold: 11,
        primary_suppression_count: 3,
        complementary_suppression_count: 2,
        primary_suppression_evidence_digest: digest(DigestDomain::ClinicalArtifact, 14),
        complementary_suppression_evidence_digest: digest(DigestDomain::ClinicalArtifact, 15),
        composition_review_evidence_digest: digest(DigestDomain::ClinicalArtifact, 16),
    }
}

fn mechanism(policy: &PopulationReleasePolicyV1) -> DifferentialPrivacyMechanismV1 {
    let mechanism = DifferentialPrivacyMechanismV1 {
        schema_version: 1,
        mechanism_id: "gaussian-count-v1".into(),
        mechanism_kind: DpMechanismKindV1::Gaussian,
        implementation_version: "1.0.0".into(),
        implementation_digest: digest(DigestDomain::ClinicalArtifact, 20),
        privacy_unit_definition_digest: digest(DigestDomain::ClinicalArtifact, 21),
        adjacency_definition_digest: digest(DigestDomain::ClinicalArtifact, 22),
        contribution_bounds_digest: digest(DigestDomain::ClinicalArtifact, 23),
        sensitivity_analysis_digest: digest(DigestDomain::ClinicalArtifact, 24),
        randomness_source_evidence_digest: digest(DigestDomain::ClinicalArtifact, 25),
        release_privacy_loss: loss(1, 10, 1, 1_000_000),
    };
    mechanism.validate(policy).unwrap();
    mechanism
}

fn genesis(policy: &PopulationReleasePolicyV1) -> PrivacyAccountantReceiptV1 {
    PrivacyAccountantReceiptV1 {
        schema_version: 1,
        accountant_instance_digest: policy.accountant_instance_digest,
        accountant_method_digest: policy.accountant_method_digest,
        sequence: 0,
        previous_receipt_digest: None,
        cumulative_privacy_loss: PrivacyLossV1::ZERO,
        query_count: 0,
        last_query_spec_digest: None,
        last_mechanism_digest: None,
        last_output_digest: None,
        accountant_evidence_digest: digest(DigestDomain::ClinicalArtifact, 30),
        observed_at_micros: 100,
    }
}

fn interactive_request() -> PopulationReleaseRequestV1 {
    let policy = policy();
    let mechanism = mechanism(&policy);
    let before = genesis(&policy);
    let query = digest(DigestDomain::ClinicalArtifact, 31);
    let output = digest(DigestDomain::ClinicalArtifact, 32);
    let mechanism_digest = mechanism.verified_digest(&policy).unwrap();
    let after = PrivacyAccountantReceiptV1 {
        schema_version: 1,
        accountant_instance_digest: policy.accountant_instance_digest,
        accountant_method_digest: policy.accountant_method_digest,
        sequence: 1,
        previous_receipt_digest: Some(before.verified_digest(&policy).unwrap()),
        cumulative_privacy_loss: loss(1, 10, 1, 1_000_000),
        query_count: 1,
        last_query_spec_digest: Some(query),
        last_mechanism_digest: Some(mechanism_digest),
        last_output_digest: Some(output),
        accountant_evidence_digest: digest(DigestDomain::ClinicalArtifact, 33),
        observed_at_micros: 110,
    };
    PopulationReleaseRequestV1 {
        schema_version: 1,
        release_id: "interactive-release-1".into(),
        policy,
        mode: PopulationReleaseModeV1::InteractiveDifferentialPrivacy(
            InteractiveDpReleaseRequestV1 {
                source_finding_kind: PopulationFindingKindV1::CausalEffectEstimate,
                source_finding_digest: digest(DigestDomain::ClinicalCausalEffectEstimate, 34),
                query_specification_digest: query,
                output_schema_digest: digest(DigestDomain::ClinicalArtifact, 35),
                public_output_digest: output,
                mechanism,
                accountant_before: before,
                accountant_after: after,
            },
        ),
        requested_at_micros: 120,
    }
}

#[test]
fn software_static_floor_rejects_threshold_ten() {
    let mut policy = policy();
    policy.static_minimum_cell_count = 10;
    assert!(matches!(
        policy.validate(),
        Err(PopulationReleaseError::StaticCellThresholdBelowSoftwareFloor)
    ));
}

#[test]
fn static_release_requires_distinct_suppression_and_composition_evidence() {
    let policy = policy();
    let mut evidence = static_evidence();
    evidence.complementary_suppression_evidence_digest =
        evidence.primary_suppression_evidence_digest;
    assert!(matches!(
        evidence.validate(&policy),
        Err(PopulationReleaseError::SuppressionEvidenceNotIndependentArtifacts)
    ));
}

#[test]
fn qualified_static_request_mints_consumable_release_receipt() {
    let request = PopulationReleaseRequestV1 {
        schema_version: 1,
        release_id: "static-release-1".into(),
        policy: policy(),
        mode: PopulationReleaseModeV1::StaticPreSpecifiedReport(StaticReleaseRequestV1 {
            suppression: static_evidence(),
        }),
        requested_at_micros: 100,
    };
    let receipt = authorize_population_release(&request)
        .unwrap()
        .into_receipt(101)
        .unwrap();
    assert_eq!(
        receipt.mode,
        PopulationReleaseModeClassV1::StaticPreSpecifiedReport
    );
    receipt.verified_digest().unwrap();
}

#[test]
fn interactive_release_binds_exact_accountant_query_mechanism_and_output() {
    let request = interactive_request();
    authorize_population_release(&request).unwrap();

    let mut substituted = request.clone();
    let PopulationReleaseModeV1::InteractiveDifferentialPrivacy(ref mut dp) = substituted.mode
    else {
        unreachable!()
    };
    dp.public_output_digest = digest(DigestDomain::ClinicalArtifact, 99);
    assert!(matches!(
        authorize_population_release(&substituted),
        Err(PopulationReleaseError::AccountantTransitionDoesNotBindRelease)
    ));
}

#[test]
fn accountant_predecessor_substitution_fails_closed() {
    let mut request = interactive_request();
    let PopulationReleaseModeV1::InteractiveDifferentialPrivacy(ref mut dp) = request.mode else {
        unreachable!()
    };
    dp.accountant_after.previous_receipt_digest = Some(PopulationReleaseDigestV1 {
        kind: PopulationReleaseArtifactKindV1::PrivacyAccountantReceipt,
        value: [77; 32],
    });
    assert!(matches!(
        authorize_population_release(&request),
        Err(PopulationReleaseError::AccountantPredecessorMismatch)
    ));
}

#[test]
fn wrong_source_finding_domain_fails_closed() {
    let mut request = interactive_request();
    let PopulationReleaseModeV1::InteractiveDifferentialPrivacy(ref mut dp) = request.mode else {
        unreachable!()
    };
    dp.source_finding_digest = digest(DigestDomain::ClinicalSafetySignalCandidate, 55);
    assert!(matches!(
        authorize_population_release(&request),
        Err(PopulationReleaseError::WrongSourceFindingDomain { .. })
    ));
}

#[test]
fn noncanonical_fraction_is_rejected() {
    let fraction = ReducedFractionV1 {
        numerator: 2,
        denominator: 4,
    };
    assert!(matches!(
        fraction.validate(),
        Err(PopulationReleaseError::NonCanonicalFraction)
    ));
}

#[test]
fn cumulative_budget_cannot_exceed_bound_policy() {
    let mut request = interactive_request();
    let PopulationReleaseModeV1::InteractiveDifferentialPrivacy(ref mut dp) = request.mode else {
        unreachable!()
    };
    dp.accountant_after.cumulative_privacy_loss = loss(3, 1, 1, 1_000_000);
    assert!(matches!(
        authorize_population_release(&request),
        Err(PopulationReleaseError::CumulativePrivacyBudgetExceeded)
    ));
}
