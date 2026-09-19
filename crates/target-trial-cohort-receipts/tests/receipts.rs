use mycelix_clinical_phenotype::{
    phenotype_definition_digest_v1, ConceptIdentityV1, CoverageEvidenceV1, CoverageStatusV1,
    CriterionV1, EvidenceArtifactIdentityV1 as PhenotypeArtifact, ObservationWindowV1,
    PhenotypeDefinitionV1,
};
use mycelix_clinical_semantics::{
    ClinicalFact, ClinicalValue, CodeableConcept, Coding, FactProvenance, SubjectRef,
};
use mycelix_target_trial_cohort_receipts::*;
use mycelix_target_trial_protocol::{
    target_trial_protocol_digest_v1, BaselineConfounderV1, CausalContrastV1,
    CausalEstimandV1, CompetingEventStrategyV1, DataSourceV1, EffectMeasureV1,
    EvidenceArtifactIdentityV1, FollowUpV1, HypotheticalRandomAssignmentV1,
    IdentifyingAssumptionV1, MaskingV1, OutcomeOperationalizationV1, OutcomeV1,
    PhenotypeDefinitionRefV1, SoftwareEnvironmentV1, StrategyOperationalizationV1,
    TargetTrialEmulationPlanV1, TargetTrialProtocolV1, TreatmentStrategyV1,
    TARGET_TRIAL_PROTOCOL_VERSION,
};

fn artifact(namespace: &str, id: &str, byte: u8) -> EvidenceArtifactIdentityV1 {
    EvidenceArtifactIdentityV1 {
        namespace: namespace.to_string(),
        artifact_id: id.to_string(),
        version: "1".to_string(),
        digest: [byte; 32],
    }
}

fn definition() -> PhenotypeDefinitionV1 {
    PhenotypeDefinitionV1 {
        schema_version: 1,
        phenotype_id: "eligible".to_string(),
        version: "1".to_string(),
        subject_resource_type: "Patient".to_string(),
        window: ObservationWindowV1 {
            start_micros: 0,
            end_micros: 2_000,
            coverage: vec![CoverageEvidenceV1 {
                domain: "diagnoses".to_string(),
                status: CoverageStatusV1::Complete,
                source: PhenotypeArtifact {
                    namespace: "coverage".to_string(),
                    artifact_id: "site-a".to_string(),
                    version: "1".to_string(),
                    digest: [31; 32],
                },
            }],
        },
        criterion: CriterionV1::FactPresent {
            criterion_id: "entry-condition".to_string(),
            concept: ConceptIdentityV1 {
                system: "http://snomed.info/sct".to_string(),
                code: "123".to_string(),
                version: "2026-09".to_string(),
            },
            minimum_count: 1,
            coverage_domain: "diagnoses".to_string(),
        },
        definition_evidence: PhenotypeArtifact {
            namespace: "phenotype-definition".to_string(),
            artifact_id: "eligible".to_string(),
            version: "1".to_string(),
            digest: [32; 32],
        },
    }
}

fn fact(subject_id: &str) -> ClinicalFact {
    ClinicalFact {
        fact_id: format!("fact:{subject_id}"),
        subject: SubjectRef {
            resource_type: "Patient".to_string(),
            id: subject_id.to_string(),
        },
        concept: CodeableConcept {
            coding: vec![Coding {
                system: "http://snomed.info/sct".to_string(),
                code: "123".to_string(),
                display: None,
                version: Some("2026-09".to_string()),
            }],
            text: None,
        },
        value: ClinicalValue::Boolean(true),
        effective_at_micros: 1_000,
        provenance: FactProvenance {
            source_system: "test-ehr".to_string(),
            source_resource_type: "Condition".to_string(),
            source_resource_id: format!("condition:{subject_id}"),
            source_version: Some("1".to_string()),
            recorded_at_micros: 1_100,
            asserted_by: None,
            transformation: None,
        },
        uncertainty: None,
    }
}

fn protocol(definition: &PhenotypeDefinitionV1) -> TargetTrialProtocolV1 {
    TargetTrialProtocolV1 {
        schema_version: TARGET_TRIAL_PROTOCOL_VERSION,
        protocol_id: "tte:receipts".to_string(),
        version: "1".to_string(),
        causal_question: "A versus B".to_string(),
        rationale_evidence: vec![artifact("literature", "rationale", 1)],
        eligibility: PhenotypeDefinitionRefV1 {
            phenotype_id: definition.phenotype_id.clone(),
            version: definition.version.clone(),
            digest: phenotype_definition_digest_v1(definition).unwrap().into_bytes(),
        },
        treatment_strategies: vec![
            TreatmentStrategyV1 {
                strategy_id: "a".to_string(),
                label: "A".to_string(),
                intervention_definition: artifact("treatment", "a", 2),
            },
            TreatmentStrategyV1 {
                strategy_id: "b".to_string(),
                label: "B".to_string(),
                intervention_definition: artifact("treatment", "b", 3),
            },
        ],
        assignment: HypotheticalRandomAssignmentV1 {
            allocation_description: "hypothetical random assignment".to_string(),
            masking: MaskingV1::OpenLabel,
        },
        time_zero_definition: artifact("design", "time-zero", 4),
        follow_up: FollowUpV1 {
            maximum_duration_micros: 10_000,
            end_conditions: vec![artifact("design", "follow-up-end", 5)],
        },
        outcomes: vec![OutcomeV1 {
            outcome_id: "outcome".to_string(),
            phenotype: PhenotypeDefinitionRefV1 {
                phenotype_id: "outcome".to_string(),
                version: "1".to_string(),
                digest: [6; 32],
            },
            assessment_window_start_offset_micros: 0,
            assessment_window_end_offset_micros: 10_000,
            primary: true,
        }],
        estimands: vec![CausalEstimandV1 {
            estimand_id: "itt".to_string(),
            treatment_strategy_id: "a".to_string(),
            comparator_strategy_id: "b".to_string(),
            outcome_id: "outcome".to_string(),
            contrast: CausalContrastV1::IntentionToTreat,
            effect_measure: EffectMeasureV1::RiskDifference,
            competing_event_strategy: CompetingEventStrategyV1::NotApplicable,
            estimand_definition: artifact("estimand", "itt", 7),
        }],
        identifying_assumptions: vec![IdentifyingAssumptionV1 {
            assumption_id: "exchangeability".to_string(),
            statement: "declared assumption".to_string(),
            related_variable_definitions: vec![],
        }],
        analysis_plan: artifact("analysis", "primary", 8),
        sensitivity_analysis_plans: vec![],
    }
}

fn plan(protocol: &TargetTrialProtocolV1) -> TargetTrialEmulationPlanV1 {
    TargetTrialEmulationPlanV1 {
        schema_version: TARGET_TRIAL_PROTOCOL_VERSION,
        emulation_id: "tte:receipts:site-a".to_string(),
        version: "1".to_string(),
        protocol_digest: target_trial_protocol_digest_v1(protocol).unwrap().into_bytes(),
        observational_design_statement: "Observational classification; no random assignment.".to_string(),
        data_sources: vec![DataSourceV1 {
            source_id: "site-a".to_string(),
            original_purpose: "routine care".to_string(),
            source_type: "EHR".to_string(),
            setting: "health system".to_string(),
            geography: "site-a".to_string(),
            period_start_micros: 0,
            period_end_micros: 10_000,
            source_identity: artifact("data-source", "site-a", 9),
        }],
        eligibility_operationalization: artifact("operationalization", "eligibility", 10),
        strategy_operationalizations: vec![
            StrategyOperationalizationV1 {
                strategy_id: "a".to_string(),
                classification_rule: artifact("operationalization", "strategy-a", 11),
            },
            StrategyOperationalizationV1 {
                strategy_id: "b".to_string(),
                classification_rule: artifact("operationalization", "strategy-b", 12),
            },
        ],
        time_zero_operationalization: artifact("operationalization", "time-zero", 13),
        outcome_operationalizations: vec![OutcomeOperationalizationV1 {
            outcome_id: "outcome".to_string(),
            operationalization: artifact("operationalization", "outcome", 14),
        }],
        baseline_confounders: vec![BaselineConfounderV1 {
            confounder_id: "age".to_string(),
            definition: artifact("confounder", "age", 15),
            measurement_window_end_offset_micros: 0,
        }],
        censoring_plan: artifact("analysis", "censoring", 16),
        adherence_plan: None,
        time_varying_confounding_plan: None,
        missing_data_plan: artifact("analysis", "missing", 17),
        estimator_plan: artifact("analysis", "estimator", 18),
        software_environment: SoftwareEnvironmentV1 {
            software: artifact("software", "analysis", 19),
            environment: artifact("environment", "analysis", 20),
        },
        vocabulary_and_mapping_artifacts: vec![],
    }
}

fn receipts(subject_id: &str) -> (
    TargetTrialProtocolV1,
    TargetTrialEmulationPlanV1,
    EligibilityReceiptV1,
    TimeZeroReceiptV1,
    StrategyClassificationReceiptV1,
) {
    let definition = definition();
    let protocol = protocol(&definition);
    let plan = plan(&protocol);
    let eligibility = build_eligibility_receipt_v1(
        &protocol,
        &plan,
        &definition,
        subject_id,
        &[fact(subject_id)],
        2_000,
        artifact("eligibility-anchor", subject_id, 21),
    )
    .unwrap();
    let time_zero = build_time_zero_receipt_v1(
        &protocol,
        &plan,
        SubjectRef {
            resource_type: "Patient".to_string(),
            id: subject_id.to_string(),
        },
        "site-a",
        2_000,
        artifact("time-zero-evidence", subject_id, 22),
    )
    .unwrap();
    let strategy = build_strategy_classification_receipt_v1(
        &protocol,
        &plan,
        &time_zero,
        "a",
        2_000,
        artifact("classification-evidence", subject_id, 23),
    )
    .unwrap();
    (protocol, plan, eligibility, time_zero, strategy)
}

#[test]
fn aligned_subject_receipts_compose() {
    let (protocol, plan, eligibility, time_zero, strategy) = receipts("p1");
    let entry = compose_cohort_entry_v1(&protocol, &plan, &eligibility, &time_zero, &strategy).unwrap();
    assert_eq!(entry.time_zero_micros, 2_000);
    assert_eq!(entry.strategy_id, "a");
}

#[test]
fn classification_after_time_zero_is_rejected() {
    let definition = definition();
    let protocol = protocol(&definition);
    let plan = plan(&protocol);
    let time_zero = build_time_zero_receipt_v1(
        &protocol,
        &plan,
        SubjectRef { resource_type: "Patient".to_string(), id: "p1".to_string() },
        "site-a",
        2_000,
        artifact("time-zero-evidence", "p1", 22),
    ).unwrap();
    assert!(matches!(
        build_strategy_classification_receipt_v1(
            &protocol, &plan, &time_zero, "a", 2_001, artifact("classification", "p1", 23)
        ),
        Err(TargetTrialCohortError::ClassificationTimeZeroMismatch)
    ));
}

#[test]
fn cross_subject_receipts_are_rejected() {
    let (protocol, plan, eligibility, time_zero, _) = receipts("p1");
    let (_, _, _, _, strategy_p2) = receipts("p2");
    assert!(matches!(
        compose_cohort_entry_v1(&protocol, &plan, &eligibility, &time_zero, &strategy_p2),
        Err(TargetTrialCohortError::SubjectMismatch)
    ));
}

#[test]
fn substituted_data_source_identity_is_rejected() {
    let (protocol, plan, eligibility, mut time_zero, strategy) = receipts("p1");
    time_zero.data_source_identity = artifact("data-source", "evil", 90);
    assert!(matches!(
        compose_cohort_entry_v1(&protocol, &plan, &eligibility, &time_zero, &strategy),
        Err(TargetTrialCohortError::DataSourceIdentityMismatch)
    ));
}

#[test]
fn substituted_time_zero_operationalization_is_rejected() {
    let (protocol, plan, eligibility, mut time_zero, strategy) = receipts("p1");
    time_zero.time_zero_operationalization = artifact("operationalization", "wrong", 91);
    assert!(matches!(
        compose_cohort_entry_v1(&protocol, &plan, &eligibility, &time_zero, &strategy),
        Err(TargetTrialCohortError::TimeZeroOperationalizationMismatch)
    ));
}

#[test]
fn substituted_eligibility_operationalization_is_rejected() {
    let (protocol, plan, mut eligibility, time_zero, strategy) = receipts("p1");
    eligibility.eligibility_operationalization = artifact("operationalization", "wrong", 92);
    assert!(matches!(
        compose_cohort_entry_v1(&protocol, &plan, &eligibility, &time_zero, &strategy),
        Err(TargetTrialCohortError::EligibilityOperationalizationMismatch)
    ));
}

#[test]
fn substituted_strategy_rule_is_rejected() {
    let (protocol, plan, eligibility, time_zero, mut strategy) = receipts("p1");
    strategy.classification_rule = artifact("operationalization", "wrong", 93);
    assert!(matches!(
        compose_cohort_entry_v1(&protocol, &plan, &eligibility, &time_zero, &strategy),
        Err(TargetTrialCohortError::StrategyOperationalizationMismatch)
    ));
}

#[test]
fn exact_time_zero_receipt_is_required() {
    let (protocol, plan, eligibility, time_zero, mut strategy) = receipts("p1");
    strategy.time_zero_receipt_digest = [94; 32];
    assert!(matches!(
        compose_cohort_entry_v1(&protocol, &plan, &eligibility, &time_zero, &strategy),
        Err(TargetTrialCohortError::TimeZeroReceiptMismatch)
    ));
}

#[test]
fn cohort_manifest_rejects_duplicate_subject() {
    let (protocol, plan, eligibility, time_zero, strategy) = receipts("p1");
    let entry = compose_cohort_entry_v1(&protocol, &plan, &eligibility, &time_zero, &strategy).unwrap();
    assert!(matches!(
        assemble_cohort_manifest_v1(
            &protocol,
            &plan,
            "cohort",
            "1",
            &[entry.clone(), entry],
            artifact("software", "assembler", 24),
            artifact("environment", "assembler", 25),
            3_000,
        ),
        Err(TargetTrialCohortError::DuplicateCohortSubject)
    ));
}

#[test]
fn cohort_manifest_identity_is_order_independent_through_constructor() {
    let (protocol, plan, e1, t1, s1) = receipts("p1");
    let (_, _, e2, t2, s2) = receipts("p2");
    let entry1 = compose_cohort_entry_v1(&protocol, &plan, &e1, &t1, &s1).unwrap();
    let entry2 = compose_cohort_entry_v1(&protocol, &plan, &e2, &t2, &s2).unwrap();
    let first = assemble_cohort_manifest_v1(
        &protocol,
        &plan,
        "cohort",
        "1",
        &[entry1.clone(), entry2.clone()],
        artifact("software", "assembler", 24),
        artifact("environment", "assembler", 25),
        3_000,
    ).unwrap();
    let second = assemble_cohort_manifest_v1(
        &protocol,
        &plan,
        "cohort",
        "1",
        &[entry2, entry1],
        artifact("software", "assembler", 24),
        artifact("environment", "assembler", 25),
        3_000,
    ).unwrap();
    assert_eq!(
        cohort_manifest_digest_v1(&first).unwrap(),
        cohort_manifest_digest_v1(&second).unwrap()
    );
}
