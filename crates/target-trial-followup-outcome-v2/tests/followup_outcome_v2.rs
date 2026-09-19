use mycelix_clinical_phenotype_v2::{
    ConceptIdentityV2, CoverageEvidenceV2, CoverageStatusV2,
    EvidenceArtifactIdentityV2 as PhenotypeEvidenceIdentityV2,
    PhenotypeEvaluationContextV2, RelativeCriterionV2, RelativeObservationWindowV2,
    RelativePhenotypeDefinitionV2, PHENOTYPE_V2_VERSION,
};
use mycelix_clinical_semantics::{
    CodeableConcept, ClinicalFact, ClinicalValue, Coding, FactProvenance, SubjectRef,
};
use mycelix_target_trial_cohort_receipts_v2::{
    build_eligibility_receipt_v2, build_strategy_classification_receipt_v2,
    build_time_zero_receipt_v2, compose_cohort_entry_v2, time_zero_anchor_evidence_v2,
    CohortEntryReceiptV2, TimeZeroReceiptV2,
};
use mycelix_target_trial_followup_outcome_v2::*;
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

fn protocol() -> TargetTrialProtocolV2 {
    TargetTrialProtocolV2 {
        schema_version: TARGET_TRIAL_PROTOCOL_V2_VERSION,
        protocol_id: "tte:v2:followup".to_string(),
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
        emulation_id: "tte:v2:followup:site".to_string(),
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

fn subject() -> SubjectRef {
    SubjectRef {
        resource_type: "Patient".to_string(),
        id: "patient-1".to_string(),
    }
}

fn fact(code: &str, at: i64) -> ClinicalFact {
    ClinicalFact {
        fact_id: format!("fact-{code}-{at}"),
        subject: subject(),
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
            source_resource_id: format!("obs-{code}-{at}"),
            source_version: Some("1".to_string()),
            recorded_at_micros: at,
            asserted_by: None,
            transformation: None,
        },
        uncertainty: None,
    }
}

struct EntryFixture {
    time_zero: TimeZeroReceiptV2,
    entry: CohortEntryReceiptV2,
}

fn entry_fixture(protocol: &TargetTrialProtocolV2, plan: &TargetTrialEmulationPlanV2) -> EntryFixture {
    let time_zero = build_time_zero_receipt_v2(
        protocol,
        plan,
        subject(),
        "site",
        100,
        artifact("source-event", "time-zero", 21),
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
            source: phenotype_artifact("coverage", "eligibility", 22),
        }],
        context_evidence: vec![],
    };
    let eligibility_facts = vec![fact("111", 95)];
    let eligibility = build_eligibility_receipt_v2(
        protocol,
        plan,
        &time_zero,
        &eligibility_definition,
        &eligibility_context,
        &eligibility_facts,
    )
    .unwrap();
    let strategy = build_strategy_classification_receipt_v2(
        protocol,
        plan,
        &time_zero,
        "a",
        100,
        artifact("classification-evidence", "a", 23),
    )
    .unwrap();
    let entry = compose_cohort_entry_v2(
        protocol,
        plan,
        &time_zero,
        &eligibility_definition,
        &eligibility_context,
        &eligibility_facts,
        &eligibility,
        &strategy,
    )
    .unwrap();
    EntryFixture { time_zero, entry }
}

fn outcome_context(
    time_zero: &TimeZeroReceiptV2,
    status: CoverageStatusV2,
) -> PhenotypeEvaluationContextV2 {
    PhenotypeEvaluationContextV2 {
        schema_version: PHENOTYPE_V2_VERSION,
        anchor_micros: time_zero.time_zero_micros,
        anchor_evidence: time_zero_anchor_evidence_v2(time_zero).unwrap(),
        coverage: vec![CoverageEvidenceV2 {
            domain: "clinical".to_string(),
            status,
            source: phenotype_artifact("coverage", "outcome", 24),
        }],
        context_evidence: vec![],
    }
}

fn outcome_evidence<'a>(
    definition: &'a RelativePhenotypeDefinitionV2,
    context: &'a PhenotypeEvaluationContextV2,
    facts: &'a [ClinicalFact],
) -> OutcomeEvidenceV2<'a> {
    OutcomeEvidenceV2 {
        outcome_id: "outcome",
        definition,
        context,
        facts,
    }
}

fn censored_follow_up(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    fixture: &EntryFixture,
    observed_until: i64,
) -> FollowUpReceiptV2 {
    build_follow_up_receipt_v2(
        protocol,
        plan,
        &fixture.entry,
        &fixture.time_zero,
        observed_until,
        FollowUpEndV2::ObservationalCensoring {
            censoring_plan: plan.censoring_plan.clone(),
            event_evidence: artifact("followup-event", "censoring", 25),
        },
    )
    .unwrap()
}

#[test]
fn planned_horizon_must_end_exactly_at_protocol_horizon() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let fixture = entry_fixture(&protocol, &plan);
    assert!(matches!(
        build_follow_up_receipt_v2(
            &protocol,
            &plan,
            &fixture.entry,
            &fixture.time_zero,
            199,
            FollowUpEndV2::PlannedHorizonComplete,
        ),
        Err(FollowUpOutcomeV2Error::PlannedHorizonMismatch)
    ));
    assert!(build_follow_up_receipt_v2(
        &protocol,
        &plan,
        &fixture.entry,
        &fixture.time_zero,
        200,
        FollowUpEndV2::PlannedHorizonComplete,
    )
    .is_ok());
}

#[test]
fn undeclared_protocol_end_condition_is_rejected() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let fixture = entry_fixture(&protocol, &plan);
    assert!(matches!(
        build_follow_up_receipt_v2(
            &protocol,
            &plan,
            &fixture.entry,
            &fixture.time_zero,
            150,
            FollowUpEndV2::ProtocolEndCondition {
                end_condition: artifact("study-design", "not-declared", 26),
                event_evidence: artifact("followup-event", "end", 27),
            },
        ),
        Err(FollowUpOutcomeV2Error::UndeclaredProtocolEndCondition)
    ));
}

#[test]
fn censoring_plan_substitution_is_rejected() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let fixture = entry_fixture(&protocol, &plan);
    assert!(matches!(
        build_follow_up_receipt_v2(
            &protocol,
            &plan,
            &fixture.entry,
            &fixture.time_zero,
            150,
            FollowUpEndV2::ObservationalCensoring {
                censoring_plan: artifact("analysis", "different-censoring", 28),
                event_evidence: artifact("followup-event", "censoring", 29),
            },
        ),
        Err(FollowUpOutcomeV2Error::CensoringPlanMismatch)
    ));
}

#[test]
fn positive_outcome_before_early_censoring_can_be_satisfied() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let fixture = entry_fixture(&protocol, &plan);
    let follow_up = censored_follow_up(&protocol, &plan, &fixture, 150);
    let definition = outcome_definition();
    let context = outcome_context(&fixture.time_zero, CoverageStatusV2::Incomplete);
    let facts = vec![fact("222", 120)];
    let evidence = outcome_evidence(&definition, &context, &facts);
    let receipt = build_outcome_ascertainment_receipt_v2(
        &protocol,
        &plan,
        &fixture.entry,
        &fixture.time_zero,
        &follow_up,
        &evidence,
    )
    .unwrap();
    assert_eq!(
        receipt.state,
        mycelix_clinical_phenotype_v2::CriterionStateV2::Satisfied
    );
}

#[test]
fn no_outcome_under_early_censoring_remains_indeterminate() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let fixture = entry_fixture(&protocol, &plan);
    let follow_up = censored_follow_up(&protocol, &plan, &fixture, 150);
    let definition = outcome_definition();
    let context = outcome_context(&fixture.time_zero, CoverageStatusV2::Incomplete);
    let evidence = outcome_evidence(&definition, &context, &[]);
    let receipt = build_outcome_ascertainment_receipt_v2(
        &protocol,
        &plan,
        &fixture.entry,
        &fixture.time_zero,
        &follow_up,
        &evidence,
    )
    .unwrap();
    assert_eq!(
        receipt.state,
        mycelix_clinical_phenotype_v2::CriterionStateV2::Indeterminate
    );
}

#[test]
fn complete_coverage_claim_is_rejected_when_followup_truncates_outcome_window() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let fixture = entry_fixture(&protocol, &plan);
    let follow_up = censored_follow_up(&protocol, &plan, &fixture, 150);
    let definition = outcome_definition();
    let context = outcome_context(&fixture.time_zero, CoverageStatusV2::Complete);
    assert!(matches!(
        build_outcome_ascertainment_receipt_v2(
            &protocol,
            &plan,
            &fixture.entry,
            &fixture.time_zero,
            &follow_up,
            &outcome_evidence(&definition, &context, &[]),
        ),
        Err(FollowUpOutcomeV2Error::CompleteCoverageAfterTruncatedFollowUp)
    ));
}

#[test]
fn fact_exactly_at_followup_end_is_not_observed() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let fixture = entry_fixture(&protocol, &plan);
    let follow_up = censored_follow_up(&protocol, &plan, &fixture, 150);
    let definition = outcome_definition();
    let context = outcome_context(&fixture.time_zero, CoverageStatusV2::Incomplete);
    let facts = vec![fact("222", 150)];
    let evidence = outcome_evidence(&definition, &context, &facts);
    let receipt = build_outcome_ascertainment_receipt_v2(
        &protocol,
        &plan,
        &fixture.entry,
        &fixture.time_zero,
        &follow_up,
        &evidence,
    )
    .unwrap();
    assert_eq!(
        receipt.state,
        mycelix_clinical_phenotype_v2::CriterionStateV2::Indeterminate
    );
}

#[test]
fn full_followup_with_complete_coverage_can_establish_not_satisfied() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let fixture = entry_fixture(&protocol, &plan);
    let follow_up = build_follow_up_receipt_v2(
        &protocol,
        &plan,
        &fixture.entry,
        &fixture.time_zero,
        200,
        FollowUpEndV2::PlannedHorizonComplete,
    )
    .unwrap();
    let definition = outcome_definition();
    let context = outcome_context(&fixture.time_zero, CoverageStatusV2::Complete);
    let evidence = outcome_evidence(&definition, &context, &[]);
    let receipt = build_outcome_ascertainment_receipt_v2(
        &protocol,
        &plan,
        &fixture.entry,
        &fixture.time_zero,
        &follow_up,
        &evidence,
    )
    .unwrap();
    assert_eq!(
        receipt.state,
        mycelix_clinical_phenotype_v2::CriterionStateV2::NotSatisfied
    );
}

#[test]
fn wrong_outcome_definition_is_rejected() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let fixture = entry_fixture(&protocol, &plan);
    let follow_up = censored_follow_up(&protocol, &plan, &fixture, 150);
    let mut definition = outcome_definition();
    definition.window.end_offset_micros = 99;
    let context = outcome_context(&fixture.time_zero, CoverageStatusV2::Incomplete);
    assert!(matches!(
        build_outcome_ascertainment_receipt_v2(
            &protocol,
            &plan,
            &fixture.entry,
            &fixture.time_zero,
            &follow_up,
            &outcome_evidence(&definition, &context, &[]),
        ),
        Err(FollowUpOutcomeV2Error::OutcomeDefinitionMismatch)
    ));
}

#[test]
fn changed_followup_receipt_breaks_outcome_replay() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let fixture = entry_fixture(&protocol, &plan);
    let original_follow_up = censored_follow_up(&protocol, &plan, &fixture, 150);
    let definition = outcome_definition();
    let context = outcome_context(&fixture.time_zero, CoverageStatusV2::Incomplete);
    let facts = vec![fact("222", 120)];
    let evidence = outcome_evidence(&definition, &context, &facts);
    let receipt = build_outcome_ascertainment_receipt_v2(
        &protocol,
        &plan,
        &fixture.entry,
        &fixture.time_zero,
        &original_follow_up,
        &evidence,
    )
    .unwrap();
    let changed_follow_up = censored_follow_up(&protocol, &plan, &fixture, 160);
    assert!(matches!(
        verify_outcome_ascertainment_receipt_v2(
            &protocol,
            &plan,
            &fixture.entry,
            &fixture.time_zero,
            &changed_follow_up,
            &evidence,
            &receipt,
        ),
        Err(FollowUpOutcomeV2Error::OutcomeReceiptMismatch)
    ));
}

#[test]
fn outcome_window_starting_before_time_zero_is_rejected() {
    let definition = phenotype("outcome", "222", -1, 100, 30);
    let mut protocol = protocol();
    protocol.outcomes[0].phenotype =
        RelativePhenotypeRefV2::from_definition(&definition).unwrap();
    let plan = plan(&protocol);
    let fixture = entry_fixture(&protocol, &plan);
    let follow_up = build_follow_up_receipt_v2(
        &protocol,
        &plan,
        &fixture.entry,
        &fixture.time_zero,
        200,
        FollowUpEndV2::PlannedHorizonComplete,
    )
    .unwrap();
    let context = outcome_context(&fixture.time_zero, CoverageStatusV2::Complete);
    let evidence = outcome_evidence(&definition, &context, &[]);

    assert!(matches!(
        build_outcome_ascertainment_receipt_v2(
            &protocol,
            &plan,
            &fixture.entry,
            &fixture.time_zero,
            &follow_up,
            &evidence,
        ),
        Err(FollowUpOutcomeV2Error::OutcomeWindowPrecedesTimeZero)
    ));
}

#[test]
fn outcome_window_extending_beyond_protocol_followup_is_rejected() {
    let definition = phenotype("outcome", "222", 0, 101, 31);
    let mut protocol = protocol();
    protocol.outcomes[0].phenotype =
        RelativePhenotypeRefV2::from_definition(&definition).unwrap();
    let plan = plan(&protocol);
    let fixture = entry_fixture(&protocol, &plan);
    let follow_up = build_follow_up_receipt_v2(
        &protocol,
        &plan,
        &fixture.entry,
        &fixture.time_zero,
        200,
        FollowUpEndV2::PlannedHorizonComplete,
    )
    .unwrap();
    let context = outcome_context(&fixture.time_zero, CoverageStatusV2::Complete);
    let evidence = outcome_evidence(&definition, &context, &[]);

    assert!(matches!(
        build_outcome_ascertainment_receipt_v2(
            &protocol,
            &plan,
            &fixture.entry,
            &fixture.time_zero,
            &follow_up,
            &evidence,
        ),
        Err(FollowUpOutcomeV2Error::OutcomeWindowExceedsProtocolFollowUp)
    ));
}
