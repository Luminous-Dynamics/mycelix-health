use mycelix_clinical_phenotype_v2::{
    ConceptIdentityV2, EvidenceArtifactIdentityV2 as PhenotypeEvidenceV2,
    RelativeCriterionV2, RelativeObservationWindowV2, RelativePhenotypeDefinitionV2,
    PHENOTYPE_V2_VERSION,
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

fn phenotype_artifact(namespace: &str, id: &str, byte: u8) -> PhenotypeEvidenceV2 {
    PhenotypeEvidenceV2 {
        namespace: namespace.to_string(),
        artifact_id: id.to_string(),
        version: "1".to_string(),
        digest: [byte; 32],
    }
}

fn concept(code: &str) -> ConceptIdentityV2 {
    ConceptIdentityV2 {
        system: "http://snomed.info/sct".to_string(),
        code: code.to_string(),
        version: "2026-09".to_string(),
    }
}

fn relative_phenotype(id: &str, start: i64, end: i64, byte: u8) -> RelativePhenotypeDefinitionV2 {
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
            criterion_id: "criterion".to_string(),
            concept: concept("123456"),
            minimum_count: 1,
            coverage_domain: "clinical".to_string(),
        },
        definition_evidence: phenotype_artifact("phenotype-evidence", id, byte),
    }
}

fn protocol() -> TargetTrialProtocolV2 {
    let eligibility = relative_phenotype("eligible", -365, 1, 1);
    let outcome = relative_phenotype("outcome", 0, 365, 2);
    TargetTrialProtocolV2 {
        schema_version: TARGET_TRIAL_PROTOCOL_V2_VERSION,
        protocol_id: "tte:v2:test".to_string(),
        version: "1".to_string(),
        causal_question: "What is the risk difference under strategy A versus B?".to_string(),
        rationale_evidence: vec![artifact("literature", "rationale", 3)],
        eligibility: RelativePhenotypeRefV2::from_definition(&eligibility).unwrap(),
        treatment_strategies: vec![
            TreatmentStrategyV2 {
                strategy_id: "a".to_string(),
                label: "initiate A".to_string(),
                intervention_definition: artifact("treatment", "a", 4),
            },
            TreatmentStrategyV2 {
                strategy_id: "b".to_string(),
                label: "initiate B".to_string(),
                intervention_definition: artifact("treatment", "b", 5),
            },
        ],
        assignment: HypotheticalRandomAssignmentV2 {
            allocation_description: "1:1 random assignment in the hypothetical trial".to_string(),
            masking: MaskingV2::OpenLabel,
        },
        time_zero_definition: artifact("study-design", "time-zero", 6),
        follow_up: FollowUpV2 {
            maximum_duration_micros: 365,
            end_conditions: vec![artifact("study-design", "follow-up-end", 7)],
        },
        outcomes: vec![OutcomeV2 {
            outcome_id: "outcome".to_string(),
            phenotype: RelativePhenotypeRefV2::from_definition(&outcome).unwrap(),
            primary: true,
        }],
        estimands: vec![CausalEstimandV2 {
            estimand_id: "itt-rd".to_string(),
            treatment_strategy_id: "a".to_string(),
            comparator_strategy_id: "b".to_string(),
            outcome_id: "outcome".to_string(),
            contrast: CausalContrastV2::IntentionToTreat,
            effect_measure: EffectMeasureV2::RiskDifference,
            competing_event_strategy: CompetingEventStrategyV2::NotApplicable,
            estimand_definition: artifact("estimand", "itt-rd", 8),
        }],
        identifying_assumptions: vec![IdentifyingAssumptionV2 {
            assumption_id: "exchangeability".to_string(),
            statement: "Measured baseline adjustment is specified separately; this statement does not assert the assumption is true.".to_string(),
            related_variable_definitions: vec![],
        }],
        analysis_plan: artifact("analysis-plan", "primary", 9),
        sensitivity_analysis_plans: vec![artifact("analysis-plan", "sensitivity", 10)],
    }
}

fn emulation(protocol: &TargetTrialProtocolV2) -> TargetTrialEmulationPlanV2 {
    TargetTrialEmulationPlanV2 {
        schema_version: TARGET_TRIAL_PROTOCOL_V2_VERSION,
        emulation_id: "tte:v2:test:omop".to_string(),
        version: "1".to_string(),
        protocol_digest: target_trial_protocol_digest_v2(protocol).unwrap().into_bytes(),
        observational_design_statement: "Observational classification at time zero; participants are not randomized.".to_string(),
        data_sources: vec![DataSourceV2 {
            source_id: "site-a".to_string(),
            original_purpose: "routine clinical care".to_string(),
            source_type: "EHR-derived OMOP CDM".to_string(),
            setting: "health system".to_string(),
            geography: "site-a".to_string(),
            period_start_micros: 1,
            period_end_micros: 10_000,
            source_identity: artifact("omop/source", "site-a", 11),
        }],
        eligibility_operationalization: artifact("operationalization", "eligibility", 12),
        strategy_operationalizations: vec![
            StrategyOperationalizationV2 {
                strategy_id: "a".to_string(),
                classification_rule: artifact("operationalization", "strategy-a", 13),
            },
            StrategyOperationalizationV2 {
                strategy_id: "b".to_string(),
                classification_rule: artifact("operationalization", "strategy-b", 14),
            },
        ],
        time_zero_operationalization: artifact("operationalization", "time-zero", 15),
        outcome_operationalizations: vec![OutcomeOperationalizationV2 {
            outcome_id: "outcome".to_string(),
            operationalization: artifact("operationalization", "outcome", 16),
        }],
        baseline_confounders: vec![BaselineConfounderV2 {
            confounder_id: "age".to_string(),
            definition: artifact("confounder", "age", 17),
            measurement_window_end_offset_micros: 0,
        }],
        censoring_plan: artifact("analysis-plan", "censoring", 18),
        adherence_plan: None,
        time_varying_confounding_plan: None,
        missing_data_plan: artifact("analysis-plan", "missing", 19),
        estimator_plan: artifact("analysis-plan", "estimator", 20),
        software_environment: SoftwareEnvironmentV2 {
            software: artifact("software", "analysis", 21),
            environment: artifact("environment", "analysis", 22),
        },
        vocabulary_and_mapping_artifacts: vec![artifact("omop/vocabulary", "snapshot", 23)],
    }
}

#[test]
fn relative_phenotype_identity_is_part_of_protocol_identity() {
    let original = protocol();
    let mut changed = original.clone();
    changed.eligibility.digest[0] ^= 1;
    assert_ne!(
        target_trial_protocol_digest_v2(&original).unwrap(),
        target_trial_protocol_digest_v2(&changed).unwrap()
    );
}

#[test]
fn wrong_relative_phenotype_namespace_is_rejected() {
    let mut value = protocol();
    value.eligibility.namespace = "mycelix/clinical-phenotype-definition/v1".to_string();
    assert!(matches!(
        value.validate(),
        Err(TargetTrialV2Error::WrongRelativePhenotypeNamespace)
    ));
}

#[test]
fn emulation_cannot_rebind_to_changed_protocol() {
    let original = protocol();
    let plan = emulation(&original);
    let mut changed = original.clone();
    changed.causal_question.push_str(" changed");
    assert!(matches!(
        plan.validate_against(&changed),
        Err(TargetTrialV2Error::ProtocolDigestMismatch)
    ));
}

#[test]
fn all_strategies_must_be_operationalized_exactly_once() {
    let protocol = protocol();
    let mut plan = emulation(&protocol);
    plan.strategy_operationalizations.pop();
    assert!(matches!(
        plan.validate_against(&protocol),
        Err(TargetTrialV2Error::IncompleteStrategyOperationalization)
    ));
}

#[test]
fn all_outcomes_must_be_operationalized_exactly_once() {
    let protocol = protocol();
    let mut plan = emulation(&protocol);
    plan.outcome_operationalizations.clear();
    assert!(matches!(
        plan.validate_against(&protocol),
        Err(TargetTrialV2Error::IncompleteOutcomeOperationalization)
    ));
}

#[test]
fn post_time_zero_baseline_confounder_is_rejected() {
    let protocol = protocol();
    let mut plan = emulation(&protocol);
    plan.baseline_confounders[0].measurement_window_end_offset_micros = 1;
    assert!(matches!(
        plan.validate_against(&protocol),
        Err(TargetTrialV2Error::PostBaselineConfounder)
    ));
}

#[test]
fn outcome_relative_window_changes_protocol_identity_through_definition_digest() {
    let mut first = relative_phenotype("outcome", 0, 365, 30);
    let mut second = first.clone();
    second.window.end_offset_micros = 730;
    let mut p1 = protocol();
    p1.outcomes[0].phenotype = RelativePhenotypeRefV2::from_definition(&first).unwrap();
    let mut p2 = p1.clone();
    p2.outcomes[0].phenotype = RelativePhenotypeRefV2::from_definition(&second).unwrap();
    assert_ne!(
        target_trial_protocol_digest_v2(&p1).unwrap(),
        target_trial_protocol_digest_v2(&p2).unwrap()
    );
    first.window.end_offset_micros = 365;
}

#[test]
fn observational_statement_is_part_of_emulation_identity() {
    let protocol = protocol();
    let original = emulation(&protocol);
    let mut changed = original.clone();
    changed.observational_design_statement.push_str(" additional detail");
    assert_ne!(
        target_trial_emulation_plan_digest_v2(&protocol, &original).unwrap(),
        target_trial_emulation_plan_digest_v2(&protocol, &changed).unwrap()
    );
}
