use mycelix_clinical_phenotype_v2::{
    CoverageEvidenceV2, CoverageStatusV2, EvidenceArtifactIdentityV2 as PhenotypeEvidenceIdentityV2,
    PhenotypeEvaluationContextV2, RelativeCriterionV2, RelativeObservationWindowV2,
    RelativePhenotypeDefinitionV2, ConceptIdentityV2, PHENOTYPE_V2_VERSION,
};
use mycelix_clinical_semantics::{
    CodeableConcept, ClinicalFact, ClinicalValue, Coding, FactProvenance, SubjectRef,
};
use mycelix_target_trial_analysis_contribution_v1::*;
use mycelix_target_trial_cohort_receipts_v2::{
    build_eligibility_receipt_v2, build_strategy_classification_receipt_v2,
    build_time_zero_receipt_v2, compose_cohort_entry_v2, time_zero_anchor_evidence_v2,
    CohortEntryReceiptV2, TimeZeroReceiptV2,
};
use mycelix_target_trial_followup_outcome_v2::{
    build_follow_up_receipt_v2, build_outcome_ascertainment_receipt_v2,
    FollowUpEndV2, FollowUpReceiptV2, OutcomeAscertainmentReceiptV2, OutcomeEvidenceV2,
};
use mycelix_target_trial_protocol_v2::*;

fn artifact(namespace: &str, id: &str, byte: u8) -> EvidenceArtifactIdentityV2 {
    EvidenceArtifactIdentityV2 {
        namespace: namespace.to_string(),
        artifact_id: id.to_string(),
        version: "1".to_string(),
        digest: [byte; 32],
    }
}

fn phenotype_artifact(namespace: &str, id: &str, byte: u8) -> PhenotypeEvidenceIdentityV2 {
    PhenotypeEvidenceIdentityV2 {
        namespace: namespace.to_string(),
        artifact_id: id.to_string(),
        version: "1".to_string(),
        digest: [byte; 32],
    }
}

fn phenotype(id: &str, code: &str, start: i64, end: i64, byte: u8) -> RelativePhenotypeDefinitionV2 {
    RelativePhenotypeDefinitionV2 {
        schema_version: PHENOTYPE_V2_VERSION,
        phenotype_id: id.to_string(),
        version: "1".to_string(),
        subject_resource_type: "Patient".to_string(),
        window: RelativeObservationWindowV2 {
            start_offset_micros: start,
            end_offset_micros: end,
            required_coverage_domains: vec!["clinical".to_string()],
        },
        criterion: RelativeCriterionV2::FactPresent {
            criterion_id: format!("{id}-criterion"),
            concept: ConceptIdentityV2 {
                system: "http://snomed.info/sct".to_string(),
                code: code.to_string(),
                version: "2026-09".to_string(),
            },
            minimum_count: 1,
            coverage_domain: "clinical".to_string(),
        },
        definition_evidence: phenotype_artifact("phenotype-evidence", id, byte),
    }
}

fn eligibility_definition() -> RelativePhenotypeDefinitionV2 {
    phenotype("eligible", "111", -20, 1, 1)
}

fn outcome_definition() -> RelativePhenotypeDefinitionV2 {
    phenotype("outcome", "222", 0, 100, 2)
}

fn protocol(
    contrast: CausalContrastV2,
    effect_measure: EffectMeasureV2,
    competing_event_strategy: CompetingEventStrategyV2,
) -> TargetTrialProtocolV2 {
    TargetTrialProtocolV2 {
        schema_version: TARGET_TRIAL_PROTOCOL_V2_VERSION,
        protocol_id: "tte:v2:analysis-contribution".to_string(),
        version: "1".to_string(),
        causal_question: "A versus B".to_string(),
        rationale_evidence: vec![artifact("literature", "rationale", 3)],
        eligibility: RelativePhenotypeRefV2::from_definition(&eligibility_definition()).unwrap(),
        treatment_strategies: vec![
            TreatmentStrategyV2 {
                strategy_id: "a".to_string(),
                label: "A".to_string(),
                intervention_definition: artifact("treatment", "a", 4),
            },
            TreatmentStrategyV2 {
                strategy_id: "b".to_string(),
                label: "B".to_string(),
                intervention_definition: artifact("treatment", "b", 5),
            },
        ],
        assignment: HypotheticalRandomAssignmentV2 {
            allocation_description: "hypothetical random assignment".to_string(),
            masking: MaskingV2::OpenLabel,
        },
        time_zero_definition: artifact("study-design", "time-zero", 6),
        follow_up: FollowUpV2 {
            maximum_duration_micros: 100,
            end_conditions: vec![artifact("study-design", "declared-end", 7)],
        },
        outcomes: vec![OutcomeV2 {
            outcome_id: "outcome".to_string(),
            phenotype: RelativePhenotypeRefV2::from_definition(&outcome_definition()).unwrap(),
            primary: true,
        }],
        estimands: vec![CausalEstimandV2 {
            estimand_id: "primary".to_string(),
            treatment_strategy_id: "a".to_string(),
            comparator_strategy_id: "b".to_string(),
            outcome_id: "outcome".to_string(),
            contrast,
            effect_measure,
            competing_event_strategy,
            estimand_definition: artifact("estimand", "primary", 8),
        }],
        identifying_assumptions: vec![],
        analysis_plan: artifact("analysis", "primary", 9),
        sensitivity_analysis_plans: vec![],
    }
}

fn plan(protocol: &TargetTrialProtocolV2) -> TargetTrialEmulationPlanV2 {
    TargetTrialEmulationPlanV2 {
        schema_version: TARGET_TRIAL_PROTOCOL_V2_VERSION,
        emulation_id: "tte:v2:analysis-contribution:site".to_string(),
        version: "1".to_string(),
        protocol_digest: target_trial_protocol_digest_v2(protocol).unwrap().into_bytes(),
        observational_design_statement: "Observational classification at time zero; participants are not randomized.".to_string(),
        data_sources: vec![DataSourceV2 {
            source_id: "site".to_string(),
            original_purpose: "routine care".to_string(),
            source_type: "EHR".to_string(),
            setting: "health system".to_string(),
            geography: "site".to_string(),
            period_start_micros: 1,
            period_end_micros: 1_000,
            source_identity: artifact("data-source", "site", 10),
        }],
        eligibility_operationalization: artifact("operationalization", "eligibility", 11),
        strategy_operationalizations: vec![
            StrategyOperationalizationV2 {
                strategy_id: "a".to_string(),
                classification_rule: artifact("operationalization", "strategy-a", 12),
            },
            StrategyOperationalizationV2 {
                strategy_id: "b".to_string(),
                classification_rule: artifact("operationalization", "strategy-b", 13),
            },
        ],
        time_zero_operationalization: artifact("operationalization", "time-zero", 14),
        outcome_operationalizations: vec![OutcomeOperationalizationV2 {
            outcome_id: "outcome".to_string(),
            operationalization: artifact("operationalization", "outcome", 15),
        }],
        baseline_confounders: vec![],
        censoring_plan: artifact("analysis", "censoring", 16),
        adherence_plan: None,
        time_varying_confounding_plan: None,
        missing_data_plan: artifact("analysis", "missing", 17),
        estimator_plan: artifact("analysis", "estimator", 18),
        software_environment: SoftwareEnvironmentV2 {
            software: artifact("software", "analysis", 19),
            environment: artifact("environment", "analysis", 20),
        },
        vocabulary_and_mapping_artifacts: vec![],
    }
}

fn subject(id: &str) -> SubjectRef {
    SubjectRef {
        resource_type: "Patient".to_string(),
        id: id.to_string(),
    }
}

fn fact(subject_id: &str, code: &str, at: i64) -> ClinicalFact {
    ClinicalFact {
        fact_id: format!("fact-{subject_id}-{code}-{at}"),
        subject: subject(subject_id),
        concept: CodeableConcept {
            coding: vec![Coding {
                system: "http://snomed.info/sct".to_string(),
                code: code.to_string(),
                display: None,
                version: Some("2026-09".to_string()),
            }],
            text: None,
        },
        value: ClinicalValue::Boolean(true),
        effective_at_micros: at,
        provenance: FactProvenance {
            source_system: "test".to_string(),
            source_resource_type: "Observation".to_string(),
            source_resource_id: format!("obs-{subject_id}-{code}-{at}"),
            source_version: Some("1".to_string()),
            recorded_at_micros: at,
            asserted_by: None,
            transformation: None,
        },
        uncertainty: None,
    }
}

struct Chain {
    protocol: TargetTrialProtocolV2,
    plan: TargetTrialEmulationPlanV2,
    time_zero: TimeZeroReceiptV2,
    entry: CohortEntryReceiptV2,
    follow_up: FollowUpReceiptV2,
    outcome_definition: RelativePhenotypeDefinitionV2,
    outcome_context: PhenotypeEvaluationContextV2,
    outcome_facts: Vec<ClinicalFact>,
    outcome_receipt: OutcomeAscertainmentReceiptV2,
}

fn chain(
    subject_id: &str,
    contrast: CausalContrastV2,
    effect_measure: EffectMeasureV2,
    competing_event_strategy: CompetingEventStrategyV2,
    coverage: CoverageStatusV2,
    event: bool,
    full_follow_up: bool,
) -> Chain {
    let protocol = protocol(contrast, effect_measure, competing_event_strategy);
    let plan = plan(&protocol);
    let time_zero = build_time_zero_receipt_v2(
        &protocol,
        &plan,
        subject(subject_id),
        "site",
        100,
        artifact("source-event", &format!("time-zero-{subject_id}"), 21),
    )
    .unwrap();
    let eligibility_definition = eligibility_definition();
    let eligibility_context = PhenotypeEvaluationContextV2 {
        schema_version: PHENOTYPE_V2_VERSION,
        anchor_micros: 100,
        anchor_evidence: time_zero_anchor_evidence_v2(&time_zero).unwrap(),
        coverage: vec![CoverageEvidenceV2 {
            domain: "clinical".to_string(),
            status: CoverageStatusV2::Complete,
            source: phenotype_artifact("coverage", &format!("eligibility-{subject_id}"), 22),
        }],
        context_evidence: vec![],
    };
    let eligibility_facts = vec![fact(subject_id, "111", 95)];
    let eligibility = build_eligibility_receipt_v2(
        &protocol,
        &plan,
        &time_zero,
        &eligibility_definition,
        &eligibility_context,
        &eligibility_facts,
    )
    .unwrap();
    let strategy = build_strategy_classification_receipt_v2(
        &protocol,
        &plan,
        &time_zero,
        "a",
        100,
        artifact("classification-evidence", &format!("a-{subject_id}"), 23),
    )
    .unwrap();
    let entry = compose_cohort_entry_v2(
        &protocol,
        &plan,
        &time_zero,
        &eligibility_definition,
        &eligibility_context,
        &eligibility_facts,
        &eligibility,
        &strategy,
    )
    .unwrap();
    let observed_until = if full_follow_up { 200 } else { 150 };
    let end = if full_follow_up {
        FollowUpEndV2::PlannedHorizonComplete
    } else {
        FollowUpEndV2::ObservationalCensoring {
            censoring_plan: plan.censoring_plan.clone(),
            event_evidence: artifact("followup-event", &format!("censoring-{subject_id}"), 24),
        }
    };
    let follow_up = build_follow_up_receipt_v2(
        &protocol,
        &plan,
        &entry,
        &time_zero,
        observed_until,
        end,
    )
    .unwrap();
    let outcome_definition = outcome_definition();
    let outcome_context = PhenotypeEvaluationContextV2 {
        schema_version: PHENOTYPE_V2_VERSION,
        anchor_micros: 100,
        anchor_evidence: time_zero_anchor_evidence_v2(&time_zero).unwrap(),
        coverage: vec![CoverageEvidenceV2 {
            domain: "clinical".to_string(),
            status: coverage,
            source: phenotype_artifact("coverage", &format!("outcome-{subject_id}"), 25),
        }],
        context_evidence: vec![],
    };
    let outcome_facts = if event { vec![fact(subject_id, "222", 120)] } else { vec![] };
    let outcome_evidence = OutcomeEvidenceV2 {
        outcome_id: "outcome",
        definition: &outcome_definition,
        context: &outcome_context,
        facts: &outcome_facts,
    };
    let outcome_receipt = build_outcome_ascertainment_receipt_v2(
        &protocol,
        &plan,
        &entry,
        &time_zero,
        &follow_up,
        &outcome_evidence,
    )
    .unwrap();
    Chain {
        protocol,
        plan,
        time_zero,
        entry,
        follow_up,
        outcome_definition,
        outcome_context,
        outcome_facts,
        outcome_receipt,
    }
}

fn contribution(chain: &Chain) -> Result<VerifiedFixedHorizonContributionV1, AnalysisContributionV1Error> {
    let outcome_evidence = OutcomeEvidenceV2 {
        outcome_id: "outcome",
        definition: &chain.outcome_definition,
        context: &chain.outcome_context,
        facts: &chain.outcome_facts,
    };
    let evidence = FixedHorizonContributionEvidenceV1 {
        protocol: &chain.protocol,
        plan: &chain.plan,
        cohort_entry: &chain.entry,
        time_zero: &chain.time_zero,
        follow_up: &chain.follow_up,
        outcome_evidence: &outcome_evidence,
        outcome_receipt: &chain.outcome_receipt,
        estimand_id: "primary",
    };
    build_fixed_horizon_contribution_v1(&evidence)
}

#[test]
fn observed_event_is_preserved() {
    let chain = chain(
        "patient-a",
        CausalContrastV2::IntentionToTreat,
        EffectMeasureV2::RiskDifference,
        CompetingEventStrategyV2::NotApplicable,
        CoverageStatusV2::Incomplete,
        true,
        false,
    );
    let verified = contribution(&chain).unwrap();
    assert_eq!(verified.receipt().state, BinaryContributionStateV1::ObservedEvent);
    assert!(verified.is_binary_observed());
}

#[test]
fn complete_non_event_is_preserved() {
    let chain = chain(
        "patient-a",
        CausalContrastV2::IntentionToTreat,
        EffectMeasureV2::RiskRatio,
        CompetingEventStrategyV2::NotApplicable,
        CoverageStatusV2::Complete,
        false,
        true,
    );
    let verified = contribution(&chain).unwrap();
    assert_eq!(verified.receipt().state, BinaryContributionStateV1::ObservedNonEvent);
    assert!(verified.is_binary_observed());
}

#[test]
fn indeterminate_outcome_is_not_silently_made_non_event() {
    let chain = chain(
        "patient-a",
        CausalContrastV2::IntentionToTreat,
        EffectMeasureV2::RiskDifference,
        CompetingEventStrategyV2::NotApplicable,
        CoverageStatusV2::Incomplete,
        false,
        false,
    );
    let verified = contribution(&chain).unwrap();
    assert_eq!(verified.receipt().state, BinaryContributionStateV1::OutcomeIndeterminate);
    assert!(!verified.is_binary_observed());
}

#[test]
fn hazard_ratio_requires_event_time_layer() {
    let chain = chain(
        "patient-a",
        CausalContrastV2::IntentionToTreat,
        EffectMeasureV2::HazardRatio,
        CompetingEventStrategyV2::NotApplicable,
        CoverageStatusV2::Incomplete,
        true,
        false,
    );
    assert!(matches!(contribution(&chain), Err(AnalysisContributionV1Error::UnsupportedEffectMeasure)));
}

#[test]
fn per_protocol_requires_future_adherence_evidence() {
    let chain = chain(
        "patient-a",
        CausalContrastV2::PerProtocol,
        EffectMeasureV2::RiskDifference,
        CompetingEventStrategyV2::NotApplicable,
        CoverageStatusV2::Incomplete,
        true,
        false,
    );
    assert!(matches!(contribution(&chain), Err(AnalysisContributionV1Error::UnsupportedCausalContrast)));
}

#[test]
fn competing_risk_estimand_requires_explicit_future_semantics() {
    let chain = chain(
        "patient-a",
        CausalContrastV2::IntentionToTreat,
        EffectMeasureV2::RiskDifference,
        CompetingEventStrategyV2::CompetingRiskEstimand,
        CoverageStatusV2::Incomplete,
        true,
        false,
    );
    assert!(matches!(contribution(&chain), Err(AnalysisContributionV1Error::UnsupportedCompetingEventStrategy)));
}

#[test]
fn manifest_is_canonical_and_preserves_indeterminate_members() {
    let b = chain(
        "patient-b",
        CausalContrastV2::IntentionToTreat,
        EffectMeasureV2::RiskDifference,
        CompetingEventStrategyV2::NotApplicable,
        CoverageStatusV2::Incomplete,
        false,
        false,
    );
    let a = chain(
        "patient-a",
        CausalContrastV2::IntentionToTreat,
        EffectMeasureV2::RiskDifference,
        CompetingEventStrategyV2::NotApplicable,
        CoverageStatusV2::Incomplete,
        true,
        false,
    );
    let b = contribution(&b).unwrap();
    let a = contribution(&a).unwrap();
    let manifest = build_fixed_horizon_analysis_manifest_v1(&[b, a]).unwrap();
    assert_eq!(manifest.contributions[0].subject_id, "patient-a");
    assert_eq!(manifest.contributions[1].subject_id, "patient-b");
    assert_eq!(manifest.contributions[1].state, BinaryContributionStateV1::OutcomeIndeterminate);
}

#[test]
fn duplicate_subject_contribution_is_rejected() {
    let chain = chain(
        "patient-a",
        CausalContrastV2::IntentionToTreat,
        EffectMeasureV2::RiskDifference,
        CompetingEventStrategyV2::NotApplicable,
        CoverageStatusV2::Incomplete,
        true,
        false,
    );
    let first = contribution(&chain).unwrap();
    let second = contribution(&chain).unwrap();
    assert!(matches!(
        build_fixed_horizon_analysis_manifest_v1(&[first, second]),
        Err(AnalysisContributionV1Error::DuplicateSubjectContribution { .. })
    ));
}
