use mycelix_clinical_integrity::{DigestAlgorithm, DigestDomain, StoredDigest};
use mycelix_clinical_population_evidence::*;

fn digest(domain: DigestDomain, seed: u8) -> StoredDigest {
    StoredDigest {
        algorithm: DigestAlgorithm::Blake3_256,
        domain,
        value: [seed; 32],
    }
}

fn generic(seed: u8) -> StoredDigest {
    digest(DigestDomain::ClinicalArtifact, seed)
}

fn person_cohort(id: &str, exposure_seed: u8, event_count: u64) -> PopulationCohortSummaryV1 {
    PopulationCohortSummaryV1 {
        schema_version: 1,
        cohort_id: id.into(),
        population_definition_digest: generic(1),
        exposure_definition_digest: generic(exposure_seed),
        outcome_definition_digest: generic(3),
        source_dataset_digest: generic(4),
        index_time_definition_digest: generic(5),
        followup_definition_digest: generic(6),
        deduplication_policy_digest: generic(7),
        strata_definition_digest: None,
        denominator: PopulationDenominatorV1::PersonsAtRisk {
            observed_subjects: 1_000,
            subjects_with_outcome: event_count,
        },
    }
}

#[test]
fn signal_candidate_cannot_bypass_minimum_case_policy() {
    let value = SafetySignalCandidateV1 {
        schema_version: 1,
        signal_id: "signal-a".into(),
        exposure_definition_digest: generic(1),
        outcome_definition_digest: generic(2),
        exact_case_set_digest: generic(3),
        source_evidence_digest: generic(4),
        detection_method: SignalDetectionMethodV1::Disproportionality,
        case_count: 2,
        distinct_subject_count: Some(2),
        method_result_digest: generic(5),
        review_status: SignalReviewStatusV1::Candidate,
        policy: SafetySignalPolicyV1 {
            schema_version: 1,
            policy_id: "signal-policy".into(),
            minimum_case_count: 3,
            require_distinct_subject_count: true,
        },
        detected_at_micros: 10,
    };

    assert!(matches!(
        value.validate(),
        Err(PopulationEvidenceError::InsufficientCasesForSignalPolicy)
    ));
}

#[test]
fn distinct_subject_count_cannot_exceed_case_count() {
    let value = SafetySignalCandidateV1 {
        schema_version: 1,
        signal_id: "signal-a".into(),
        exposure_definition_digest: generic(1),
        outcome_definition_digest: generic(2),
        exact_case_set_digest: generic(3),
        source_evidence_digest: generic(4),
        detection_method: SignalDetectionMethodV1::SpontaneousCaseReview,
        case_count: 3,
        distinct_subject_count: Some(4),
        method_result_digest: generic(5),
        review_status: SignalReviewStatusV1::ValidatedForReview,
        policy: SafetySignalPolicyV1 {
            schema_version: 1,
            policy_id: "signal-policy".into(),
            minimum_case_count: 1,
            require_distinct_subject_count: true,
        },
        detected_at_micros: 10,
    };

    assert!(matches!(
        value.validate(),
        Err(PopulationEvidenceError::InvalidDistinctSubjectCount)
    ));
}

#[test]
fn risk_ratio_rejects_person_time_denominator() {
    let mut exposed = person_cohort("exposed", 10, 15);
    exposed.denominator = PopulationDenominatorV1::PersonTimeMicros {
        person_time_micros: 1_000_000,
        outcome_events: 15,
    };
    let mut comparator = person_cohort("comparator", 11, 10);
    comparator.denominator = PopulationDenominatorV1::PersonTimeMicros {
        person_time_micros: 2_000_000,
        outcome_events: 10,
    };

    let value = DenominatorBasedAssociationV1 {
        schema_version: 1,
        association_id: "association-a".into(),
        exposed,
        comparator,
        measure: PopulationAssociationMeasureV1::RiskRatio,
        statistical_method_digest: generic(20),
        exact_analysis_result_digest: generic(21),
        policy: PopulationAssociationPolicyV1::strict_default("association-policy"),
        analyzed_at_micros: 20,
    };

    assert!(matches!(
        value.validate(),
        Err(PopulationEvidenceError::MeasureIncompatibleWithDenominator)
    ));
}

#[test]
fn denominator_association_accepts_distinct_comparable_person_cohorts() {
    let value = DenominatorBasedAssociationV1 {
        schema_version: 1,
        association_id: "association-a".into(),
        exposed: person_cohort("exposed", 10, 15),
        comparator: person_cohort("comparator", 11, 10),
        measure: PopulationAssociationMeasureV1::RiskRatio,
        statistical_method_digest: generic(20),
        exact_analysis_result_digest: generic(21),
        policy: PopulationAssociationPolicyV1::strict_default("association-policy"),
        analyzed_at_micros: 20,
    };

    let verified = value.verified_digest().unwrap();
    assert_eq!(verified.domain(), DigestDomain::ClinicalPopulationAssociation);
}

fn strict_effect_policy() -> CausalEffectPolicyV1 {
    CausalEffectPolicyV1 {
        schema_version: 1,
        policy_id: "effect-policy".into(),
        allowed_designs: vec![CausalStudyDesignV1::TargetTrialEmulation],
        require_sensitivity_analysis: true,
        require_negative_control_review: true,
    }
}

#[test]
fn causal_effect_requires_population_association_domain() {
    let value = CausalEffectEstimateV1 {
        schema_version: 1,
        estimate_id: "effect-a".into(),
        association_digest: digest(DigestDomain::ClinicalSafetySignalCandidate, 1),
        design: CausalStudyDesignV1::TargetTrialEmulation,
        design_specification_digest: generic(2),
        confounder_strategy_digest: generic(3),
        diagnostics_digest: generic(4),
        exact_estimator_result_digest: generic(5),
        sensitivity_analysis_digest: Some(generic(6)),
        negative_control_review_digest: Some(generic(7)),
        policy: strict_effect_policy(),
        estimated_at_micros: 30,
    };

    assert!(matches!(
        value.validate(),
        Err(PopulationEvidenceError::WrongDigestDomain {
            expected: DigestDomain::ClinicalPopulationAssociation,
            ..
        })
    ));
}

#[test]
fn causal_effect_policy_can_require_sensitivity_and_negative_controls() {
    let base = CausalEffectEstimateV1 {
        schema_version: 1,
        estimate_id: "effect-a".into(),
        association_digest: digest(DigestDomain::ClinicalPopulationAssociation, 1),
        design: CausalStudyDesignV1::TargetTrialEmulation,
        design_specification_digest: generic(2),
        confounder_strategy_digest: generic(3),
        diagnostics_digest: generic(4),
        exact_estimator_result_digest: generic(5),
        sensitivity_analysis_digest: None,
        negative_control_review_digest: Some(generic(7)),
        policy: strict_effect_policy(),
        estimated_at_micros: 30,
    };

    assert!(matches!(
        base.validate(),
        Err(PopulationEvidenceError::MissingSensitivityAnalysis)
    ));

    let mut without_negative_control = base;
    without_negative_control.sensitivity_analysis_digest = Some(generic(6));
    without_negative_control.negative_control_review_digest = None;
    assert!(matches!(
        without_negative_control.validate(),
        Err(PopulationEvidenceError::MissingNegativeControlReview)
    ));
}

#[test]
fn replication_rejects_duplicate_findings() {
    let finding = digest(DigestDomain::ClinicalPopulationAssociation, 9);
    let value = PopulationReplicationV1 {
        schema_version: 1,
        replication_id: "replication-a".into(),
        finding_kind: ReplicatedFindingKindV1::DenominatorBasedAssociation,
        finding_digests: vec![finding, finding],
        independence_evidence_digest: generic(30),
        replication_method_digest: generic(31),
        conclusion: ReplicationConclusionV1::Concordant,
        assessed_at_micros: 40,
    };

    assert!(matches!(
        value.validate(),
        Err(PopulationEvidenceError::DuplicateReplicationFinding)
    ));
}

#[test]
fn replication_rejects_mixed_finding_classes() {
    let value = PopulationReplicationV1 {
        schema_version: 1,
        replication_id: "replication-a".into(),
        finding_kind: ReplicatedFindingKindV1::CausalEffectEstimate,
        finding_digests: vec![
            digest(DigestDomain::ClinicalCausalEffectEstimate, 1),
            digest(DigestDomain::ClinicalPopulationAssociation, 2),
        ],
        independence_evidence_digest: generic(30),
        replication_method_digest: generic(31),
        conclusion: ReplicationConclusionV1::Mixed,
        assessed_at_micros: 40,
    };

    assert!(matches!(
        value.validate(),
        Err(PopulationEvidenceError::WrongDigestDomain {
            expected: DigestDomain::ClinicalCausalEffectEstimate,
            ..
        })
    ));
}
