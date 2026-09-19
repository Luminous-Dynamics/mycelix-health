use mycelix_clinical_phenotype_v2::{
    ConceptIdentityV2, CoverageEvidenceV2, CoverageStatusV2,
    EvidenceArtifactIdentityV2 as PhenotypeEvidenceIdentityV2,
    PhenotypeEvaluationContextV2, RelativeCriterionV2, RelativeObservationWindowV2,
    RelativePhenotypeDefinitionV2, PHENOTYPE_V2_VERSION,
};
use mycelix_clinical_semantics::{
    CodeableConcept, ClinicalFact, ClinicalValue, Coding, FactProvenance, SubjectRef,
};
use mycelix_target_trial_cohort_receipts_v2::*;
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

fn eligibility_definition() -> RelativePhenotypeDefinitionV2 {
    RelativePhenotypeDefinitionV2 {
        schema_version: PHENOTYPE_V2_VERSION,
        phenotype_id: "eligible".to_string(),
        version: "1".to_string(),
        subject_resource_type: "Patient".to_string(),
        window: RelativeObservationWindowV2 {
            start_offset_micros: -20,
            end_offset_micros: 1,
            required_coverage_domains: vec!["clinical".to_string()],
        },
        criterion: RelativeCriterionV2::FactPresent {
            criterion_id: "required-history".to_string(),
            concept: ConceptIdentityV2 {
                system: "http://snomed.info/sct".to_string(),
                code: "123456".to_string(),
                version: "2026-09".to_string(),
            },
            minimum_count: 1,
            coverage_domain: "clinical".to_string(),
        },
        definition_evidence: phenotype_artifact("phenotype-evidence", "eligible", 1),
    }
}

fn outcome_definition() -> RelativePhenotypeDefinitionV2 {
    let mut value = eligibility_definition();
    value.phenotype_id = "outcome".to_string();
    value.window.start_offset_micros = 0;
    value.window.end_offset_micros = 365;
    value.definition_evidence = phenotype_artifact("phenotype-evidence", "outcome", 2);
    value
}

fn protocol() -> TargetTrialProtocolV2 {
    let eligibility = eligibility_definition();
    let outcome = outcome_definition();
    TargetTrialProtocolV2 {
        schema_version: TARGET_TRIAL_PROTOCOL_V2_VERSION,
        protocol_id: "tte:v2:test".to_string(),
        version: "1".to_string(),
        causal_question: "A versus B".to_string(),
        rationale_evidence: vec![artifact("literature", "rationale", 3)],
        eligibility: RelativePhenotypeRefV2::from_definition(&eligibility).unwrap(),
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
            maximum_duration_micros: 365,
            end_conditions: vec![artifact("study-design", "end", 7)],
        },
        outcomes: vec![OutcomeV2 {
            outcome_id: "outcome".to_string(),
            phenotype: RelativePhenotypeRefV2::from_definition(&outcome).unwrap(),
            primary: true,
        }],
        estimands: vec![CausalEstimandV2 {
            estimand_id: "itt".to_string(),
            treatment_strategy_id: "a".to_string(),
            comparator_strategy_id: "b".to_string(),
            outcome_id: "outcome".to_string(),
            contrast: CausalContrastV2::IntentionToTreat,
            effect_measure: EffectMeasureV2::RiskDifference,
            competing_event_strategy: CompetingEventStrategyV2::NotApplicable,
            estimand_definition: artifact("estimand", "itt", 8),
        }],
        identifying_assumptions: vec![],
        analysis_plan: artifact("analysis", "primary", 9),
        sensitivity_analysis_plans: vec![],
    }
}

fn plan(protocol: &TargetTrialProtocolV2) -> TargetTrialEmulationPlanV2 {
    TargetTrialEmulationPlanV2 {
        schema_version: TARGET_TRIAL_PROTOCOL_V2_VERSION,
        emulation_id: "tte:v2:test:site".to_string(),
        version: "1".to_string(),
        protocol_digest: target_trial_protocol_digest_v2(protocol).unwrap().into_bytes(),
        observational_design_statement: "Observational classification at time zero; no random assignment is claimed.".to_string(),
        data_sources: vec![DataSourceV2 {
            source_id: "site".to_string(),
            original_purpose: "care".to_string(),
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
                classification_rule: artifact("operationalization", "a", 12),
            },
            StrategyOperationalizationV2 {
                strategy_id: "b".to_string(),
                classification_rule: artifact("operationalization", "b", 13),
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

fn subject() -> SubjectRef {
    SubjectRef {
        resource_type: "Patient".to_string(),
        id: "patient-1".to_string(),
    }
}

fn fact(at: i64) -> ClinicalFact {
    ClinicalFact {
        fact_id: format!("fact-{at}"),
        subject: subject(),
        concept: CodeableConcept {
            coding: vec![Coding {
                system: "http://snomed.info/sct".to_string(),
                code: "123456".to_string(),
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
            source_resource_id: format!("obs-{at}"),
            source_version: Some("1".to_string()),
            recorded_at_micros: at,
            asserted_by: None,
            transformation: None,
        },
        uncertainty: None,
    }
}

fn time_zero(protocol: &TargetTrialProtocolV2, plan: &TargetTrialEmulationPlanV2) -> TimeZeroReceiptV2 {
    build_time_zero_receipt_v2(
        protocol,
        plan,
        subject(),
        "site",
        100,
        artifact("source-event", "time-zero", 21),
    )
    .unwrap()
}

fn eligibility_context(time_zero: &TimeZeroReceiptV2, status: CoverageStatusV2) -> PhenotypeEvaluationContextV2 {
    PhenotypeEvaluationContextV2 {
        schema_version: PHENOTYPE_V2_VERSION,
        anchor_micros: time_zero.time_zero_micros,
        anchor_evidence: time_zero_anchor_evidence_v2(time_zero).unwrap(),
        coverage: vec![CoverageEvidenceV2 {
            domain: "clinical".to_string(),
            status,
            source: phenotype_artifact("coverage", "clinical", 22),
        }],
        context_evidence: vec![],
    }
}

#[test]
fn valid_time_zero_to_eligibility_to_strategy_chain_composes() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan);
    let definition = eligibility_definition();
    let context = eligibility_context(&time_zero, CoverageStatusV2::Incomplete);
    let facts = vec![fact(95)];
    let eligibility = build_eligibility_receipt_v2(
        &protocol, &plan, &time_zero, &definition, &context, &facts,
    )
    .unwrap();
    let strategy = build_strategy_classification_receipt_v2(
        &protocol,
        &plan,
        &time_zero,
        "a",
        100,
        artifact("classification-evidence", "a", 23),
    )
    .unwrap();
    let entry = compose_cohort_entry_v2(
        &protocol,
        &plan,
        &time_zero,
        &definition,
        &context,
        &facts,
        &eligibility,
        &strategy,
    )
    .unwrap();
    assert_eq!(entry.time_zero_micros, 100);
    assert_eq!(entry.strategy_id, "a");
}

#[test]
fn positive_eligibility_can_be_established_under_incomplete_coverage() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan);
    let definition = eligibility_definition();
    let context = eligibility_context(&time_zero, CoverageStatusV2::Incomplete);
    assert!(build_eligibility_receipt_v2(
        &protocol,
        &plan,
        &time_zero,
        &definition,
        &context,
        &[fact(95)],
    )
    .is_ok());
}

#[test]
fn changed_time_zero_evidence_breaks_eligibility_anchor_replay() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let original = time_zero(&protocol, &plan);
    let definition = eligibility_definition();
    let context = eligibility_context(&original, CoverageStatusV2::Complete);
    let facts = vec![fact(95)];
    let eligibility = build_eligibility_receipt_v2(
        &protocol, &plan, &original, &definition, &context, &facts,
    )
    .unwrap();

    let changed = build_time_zero_receipt_v2(
        &protocol,
        &plan,
        subject(),
        "site",
        100,
        artifact("source-event", "different-time-zero-evidence", 24),
    )
    .unwrap();
    assert!(matches!(
        verify_eligibility_receipt_v2(
            &protocol,
            &plan,
            &changed,
            &definition,
            &context,
            &facts,
            &eligibility,
        ),
        Err(TargetTrialCohortV2Error::EligibilityAnchorEvidenceMismatch)
    ));
}

#[test]
fn eligibility_context_with_wrong_anchor_timestamp_is_rejected() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan);
    let definition = eligibility_definition();
    let mut context = eligibility_context(&time_zero, CoverageStatusV2::Complete);
    context.anchor_micros += 1;
    assert!(matches!(
        build_eligibility_receipt_v2(
            &protocol,
            &plan,
            &time_zero,
            &definition,
            &context,
            &[fact(95)],
        ),
        Err(TargetTrialCohortV2Error::EligibilityAnchorTimeMismatch)
    ));
}

#[test]
fn strategy_classification_after_time_zero_is_rejected() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan);
    assert!(matches!(
        build_strategy_classification_receipt_v2(
            &protocol,
            &plan,
            &time_zero,
            "a",
            101,
            artifact("classification-evidence", "a", 25),
        ),
        Err(TargetTrialCohortV2Error::ClassificationTimeZeroMismatch)
    ));
}

#[test]
fn strategy_rule_substitution_fails_live_verification() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan);
    let mut receipt = build_strategy_classification_receipt_v2(
        &protocol,
        &plan,
        &time_zero,
        "a",
        100,
        artifact("classification-evidence", "a", 26),
    )
    .unwrap();
    receipt.classification_rule = artifact("operationalization", "substituted", 27);
    assert!(matches!(
        verify_strategy_receipt_v2(&protocol, &plan, &time_zero, &receipt),
        Err(TargetTrialCohortV2Error::StrategyRuleMismatch)
    ));
}

#[test]
fn time_zero_data_source_substitution_fails_live_verification() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let mut receipt = time_zero(&protocol, &plan);
    receipt.data_source_identity = artifact("data-source", "substituted", 28);
    assert!(matches!(
        verify_time_zero_receipt_v2(&protocol, &plan, &receipt),
        Err(TargetTrialCohortV2Error::DataSourceIdentityMismatch)
    ));
}

#[test]
fn duplicate_subjects_are_rejected_by_simple_v2_manifest() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan);
    let definition = eligibility_definition();
    let context = eligibility_context(&time_zero, CoverageStatusV2::Complete);
    let facts = vec![fact(95)];
    let eligibility = build_eligibility_receipt_v2(
        &protocol, &plan, &time_zero, &definition, &context, &facts,
    )
    .unwrap();
    let strategy = build_strategy_classification_receipt_v2(
        &protocol,
        &plan,
        &time_zero,
        "a",
        100,
        artifact("classification-evidence", "a", 29),
    )
    .unwrap();
    let entry = compose_cohort_entry_v2(
        &protocol,
        &plan,
        &time_zero,
        &definition,
        &context,
        &facts,
        &eligibility,
        &strategy,
    )
    .unwrap();
    assert!(matches!(
        assemble_cohort_manifest_v2(
            &protocol,
            &plan,
            "cohort",
            "1",
            &[entry.clone(), entry],
            artifact("software", "assembler", 30),
            artifact("environment", "assembler", 31),
            200,
        ),
        Err(TargetTrialCohortV2Error::DuplicateCohortSubject)
    ));
}
