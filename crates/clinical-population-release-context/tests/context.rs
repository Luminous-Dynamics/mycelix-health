use mycelix_clinical_integrity::{DigestAlgorithm, DigestDomain, StoredDigest};
use mycelix_clinical_population_release::{
    DifferentialPrivacyMechanismV1, DpMechanismKindV1, InteractiveDpReleaseRequestV1,
    PopulationFindingKindV1, PopulationReleaseArtifactKindV1, PopulationReleaseDigestV1,
    PopulationReleaseModeV1, PopulationReleasePolicyV1, PopulationReleaseRequestV1,
    PrivacyAccountantReceiptV1, PrivacyLossV1, ReducedFractionV1,
};
use mycelix_clinical_population_release_context::{
    interactive_release_context_digest, InteractivePopulationReleaseContextV1,
    PopulationReleaseContextError,
};

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

fn mechanism(policy: &PopulationReleasePolicyV1) -> DifferentialPrivacyMechanismV1 {
    DifferentialPrivacyMechanismV1 {
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
    }
}

fn request() -> PopulationReleaseRequestV1 {
    let policy = policy();
    let mechanism = mechanism(&policy);
    let query = digest(DigestDomain::ClinicalArtifact, 31);
    let output = digest(DigestDomain::ClinicalArtifact, 32);
    let mechanism_digest = mechanism.verified_digest(&policy).unwrap();
    let before = PrivacyAccountantReceiptV1 {
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
    };
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
fn context_uses_dedicated_integrity_domain() {
    let digest = interactive_release_context_digest(&request()).unwrap();
    assert_eq!(digest.domain, DigestDomain::ClinicalPopulationReleaseContext);
}

#[test]
fn source_finding_substitution_changes_context_identity() {
    let original = request();
    let original_digest = interactive_release_context_digest(&original).unwrap();
    let mut substituted = original;
    let PopulationReleaseModeV1::InteractiveDifferentialPrivacy(ref mut interactive) =
        substituted.mode
    else {
        unreachable!()
    };
    interactive.source_finding_digest =
        digest(DigestDomain::ClinicalCausalEffectEstimate, 99);
    let substituted_digest = interactive_release_context_digest(&substituted).unwrap();
    assert_ne!(original_digest, substituted_digest);
}

#[test]
fn output_schema_substitution_changes_context_identity() {
    let original = request();
    let original_digest = interactive_release_context_digest(&original).unwrap();
    let mut substituted = original;
    let PopulationReleaseModeV1::InteractiveDifferentialPrivacy(ref mut interactive) =
        substituted.mode
    else {
        unreachable!()
    };
    interactive.output_schema_digest = digest(DigestDomain::ClinicalArtifact, 98);
    let substituted_digest = interactive_release_context_digest(&substituted).unwrap();
    assert_ne!(original_digest, substituted_digest);
}

#[test]
fn static_request_is_not_an_interactive_release_context() {
    let context = InteractivePopulationReleaseContextV1 {
        schema_version: 2,
        release_policy_digest: PopulationReleaseDigestV1 {
            kind: PopulationReleaseArtifactKindV1::ReleasePolicy,
            value: [1; 32],
        },
        source_finding_kind: PopulationFindingKindV1::CausalEffectEstimate,
        source_finding_digest: digest(DigestDomain::ClinicalCausalEffectEstimate, 2),
        query_specification_digest: digest(DigestDomain::ClinicalArtifact, 3),
        output_schema_digest: digest(DigestDomain::ClinicalArtifact, 4),
        public_output_digest: digest(DigestDomain::ClinicalArtifact, 5),
        mechanism_digest: PopulationReleaseDigestV1 {
            kind: PopulationReleaseArtifactKindV1::DpMechanismDescriptor,
            value: [6; 32],
        },
    };
    assert!(matches!(
        context.verified_digest(),
        Err(PopulationReleaseContextError::UnsupportedVersion(2))
    ));
}
