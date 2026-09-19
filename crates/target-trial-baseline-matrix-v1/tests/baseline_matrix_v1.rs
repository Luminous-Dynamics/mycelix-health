use mycelix_clinical_phenotype_v2::{
    ConceptIdentityV2, CoverageEvidenceV2, CoverageStatusV2,
    EvidenceArtifactIdentityV2 as PhenotypeEvidenceIdentityV2,
    PhenotypeEvaluationContextV2, RelativeCriterionV2, RelativeObservationWindowV2,
    RelativePhenotypeDefinitionV2, PHENOTYPE_V2_VERSION,
};
use mycelix_clinical_semantics::{
    CodeableConcept, ClinicalFact, ClinicalValue, Coding, FactProvenance, SubjectRef,
};
use mycelix_target_trial_analysis_contribution_v1::{
    build_fixed_horizon_contribution_v1, FixedHorizonContributionEvidenceV1,
    VerifiedFixedHorizonContributionV1,
};
use mycelix_target_trial_baseline_covariate_v1::{
    build_baseline_covariate_measurement_v1, BaselineCovariateMeasurementReceiptV1,
    BaselineMeasurementEvidenceV1, BaselineMeasurementPolicyV1, BaselineSelectionRuleV1,
    VerifiedBaselineCovariateMeasurementV1, BASELINE_COVARIATE_MEASUREMENT_V1_VERSION,
};
use mycelix_target_trial_baseline_matrix_v1::*;
use mycelix_target_trial_cohort_receipts_v2::{
    build_eligibility_receipt_v2, build_strategy_classification_receipt_v2,
    build_time_zero_receipt_v2, compose_cohort_entry_v2, time_zero_anchor_evidence_v2,
    TimeZeroReceiptV2,
};
use mycelix_target_trial_followup_outcome_v2::{
    build_follow_up_receipt_v2, build_outcome_ascertainment_receipt_v2, FollowUpEndV2,
    OutcomeEvidenceV2,
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

fn protocol() -> TargetTrialProtocolV2 {
    TargetTrialProtocolV2 {
        schema_version: TARGET_TRIAL_PROTOCOL_V2_VERSION,
        protocol_id: "tte:v2:baseline-matrix".to_string(),
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
            contrast: CausalContrastV2::IntentionToTreat,
            effect_measure: EffectMeasureV2::RiskDifference,
            competing_event_strategy: CompetingEventStrategyV2::NotApplicable,
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
        emulation_id: "tte:v2:baseline-matrix:site".to_string(),
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
        baseline_confounders: vec![
            BaselineConfounderV2 {
                confounder_id: "bp".to_string(),
                definition: artifact("confounder", "bp", 31),
                measurement_window_end_offset_micros: -5,
            },
            BaselineConfounderV2 {
                confounder_id: "weight".to_string(),
                definition: artifact("confounder", "weight", 32),
                measurement_window_end_offset_micros: 0,
            },
        ],
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

fn clinical_fact(subject_id: &str, code: &str, version: &str, at: i64, value: ClinicalValue) -> ClinicalFact {
    ClinicalFact {
        fact_id: format!("fact-{subject_id}-{code}-{at}"),
        subject: subject(subject_id),
        concept: CodeableConcept {
            coding: vec![Coding {
                system: if code == "111" || code == "222" {
                    "http://snomed.info/sct".to_string()
                } else {
                    "http://loinc.org".to_string()
                },
                code: code.to_string(),
                display: None,
                version: Some(version.to_string()),
            }],
            text: None,
        },
        value,
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

fn contribution(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    subject_id: &str,
    strategy_id: &str,
    event: bool,
) -> (TimeZeroReceiptV2, VerifiedFixedHorizonContributionV1) {
    let time_zero = build_time_zero_receipt_v2(
        protocol,
        plan,
        subject(subject_id),
        "site",
        100,
        artifact("source-event", &format!("time-zero-{subject_id}"), 40),
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
            source: phenotype_artifact("coverage", &format!("eligibility-{subject_id}"), 41),
        }],
        context_evidence: vec![],
    };
    let eligibility_facts = vec![clinical_fact(
        subject_id,
        "111",
        "2026-09",
        95,
        ClinicalValue::Boolean(true),
    )];
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
        strategy_id,
        100,
        artifact("classification", &format!("{strategy_id}-{subject_id}"), 42),
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
    let follow_up = build_follow_up_receipt_v2(
        protocol,
        plan,
        &entry,
        &time_zero,
        200,
        FollowUpEndV2::PlannedHorizonComplete,
    )
    .unwrap();
    let outcome_definition = outcome_definition();
    let outcome_context = PhenotypeEvaluationContextV2 {
        schema_version: PHENOTYPE_V2_VERSION,
        anchor_micros: 100,
        anchor_evidence: time_zero_anchor_evidence_v2(&time_zero).unwrap(),
        coverage: vec![CoverageEvidenceV2 {
            domain: "clinical".to_string(),
            status: CoverageStatusV2::Complete,
            source: phenotype_artifact("coverage", &format!("outcome-{subject_id}"), 43),
        }],
        context_evidence: vec![],
    };
    let outcome_facts = if event {
        vec![clinical_fact(
            subject_id,
            "222",
            "2026-09",
            120,
            ClinicalValue::Boolean(true),
        )]
    } else {
        vec![]
    };
    let outcome_evidence = OutcomeEvidenceV2 {
        outcome_id: "outcome",
        definition: &outcome_definition,
        context: &outcome_context,
        facts: &outcome_facts,
    };
    let outcome_receipt = build_outcome_ascertainment_receipt_v2(
        protocol,
        plan,
        &entry,
        &time_zero,
        &follow_up,
        &outcome_evidence,
    )
    .unwrap();
    let contribution_evidence = FixedHorizonContributionEvidenceV1 {
        protocol,
        plan,
        cohort_entry: &entry,
        time_zero: &time_zero,
        follow_up: &follow_up,
        outcome_evidence: &outcome_evidence,
        outcome_receipt: &outcome_receipt,
        estimand_id: "primary",
    };
    let verified = build_fixed_horizon_contribution_v1(&contribution_evidence).unwrap();
    (time_zero, verified)
}

fn policy(confounder_id: &str, evidence_byte: u8) -> BaselineMeasurementPolicyV1 {
    let (definition_byte, code, cutoff) = match confounder_id {
        "bp" => (31, "8480-6", -5),
        "weight" => (32, "29463-7", 0),
        _ => panic!("unknown fixture confounder"),
    };
    BaselineMeasurementPolicyV1 {
        schema_version: BASELINE_COVARIATE_MEASUREMENT_V1_VERSION,
        confounder_id: confounder_id.to_string(),
        protocol_definition: artifact("confounder", confounder_id, definition_byte),
        concept: ConceptIdentityV2 {
            system: "http://loinc.org".to_string(),
            code: code.to_string(),
            version: "2.83".to_string(),
        },
        window_start_offset_micros: -50,
        window_end_offset_micros: cutoff,
        selection_rule: BaselineSelectionRuleV1::UniqueOnly,
        policy_evidence: artifact("measurement-policy", confounder_id, evidence_byte),
    }
}

fn measurement(
    protocol: &TargetTrialProtocolV2,
    plan: &TargetTrialEmulationPlanV2,
    time_zero: &TimeZeroReceiptV2,
    policy: &BaselineMeasurementPolicyV1,
    facts: &[ClinicalFact],
    candidate_byte: u8,
) -> VerifiedBaselineCovariateMeasurementV1 {
    let candidate_set = artifact(
        "query-result",
        &format!("{}-{}", policy.confounder_id, time_zero.subject.id),
        candidate_byte,
    );
    build_baseline_covariate_measurement_v1(&BaselineMeasurementEvidenceV1 {
        protocol,
        plan,
        time_zero,
        policy,
        candidate_set_evidence: &candidate_set,
        candidate_facts: facts,
    })
    .unwrap()
}

struct Fixture {
    protocol: TargetTrialProtocolV2,
    plan: TargetTrialEmulationPlanV2,
    time_zero_a: TimeZeroReceiptV2,
    time_zero_b: TimeZeroReceiptV2,
    contributions: Vec<VerifiedFixedHorizonContributionV1>,
}

fn fixture() -> Fixture {
    let protocol = protocol();
    let plan = plan(&protocol);
    let (time_zero_a, contribution_a) = contribution(&protocol, &plan, "patient-a", "a", true);
    let (time_zero_b, contribution_b) = contribution(&protocol, &plan, "patient-b", "b", false);
    Fixture {
        protocol,
        plan,
        time_zero_a,
        time_zero_b,
        contributions: vec![contribution_b, contribution_a],
    }
}

fn complete_measurements(
    fixture: &Fixture,
    patient_b_weight_missing: bool,
) -> Vec<VerifiedBaselineCovariateMeasurementV1> {
    let bp_policy = policy("bp", 50);
    let weight_policy = policy("weight", 51);
    let a_bp = vec![clinical_fact(
        "patient-a",
        "8480-6",
        "2.83",
        90,
        ClinicalValue::Integer(120),
    )];
    let a_weight = vec![clinical_fact(
        "patient-a",
        "29463-7",
        "2.83",
        92,
        ClinicalValue::Integer(75),
    )];
    let b_bp = vec![clinical_fact(
        "patient-b",
        "8480-6",
        "2.83",
        89,
        ClinicalValue::Integer(118),
    )];
    let b_weight = if patient_b_weight_missing {
        vec![]
    } else {
        vec![clinical_fact(
            "patient-b",
            "29463-7",
            "2.83",
            91,
            ClinicalValue::Integer(80),
        )]
    };
    vec![
        measurement(&fixture.protocol, &fixture.plan, &fixture.time_zero_b, &weight_policy, &b_weight, 60),
        measurement(&fixture.protocol, &fixture.plan, &fixture.time_zero_a, &bp_policy, &a_bp, 61),
        measurement(&fixture.protocol, &fixture.plan, &fixture.time_zero_b, &bp_policy, &b_bp, 62),
        measurement(&fixture.protocol, &fixture.plan, &fixture.time_zero_a, &weight_policy, &a_weight, 63),
    ]
}

#[test]
fn complete_cartesian_matrix_is_canonical_and_preserves_missingness() {
    let fixture = fixture();
    let measurements = complete_measurements(&fixture, true);
    let verified = build_baseline_covariate_matrix_v1(
        &fixture.protocol,
        &fixture.plan,
        &fixture.contributions,
        &measurements,
    )
    .unwrap();
    let matrix = verified.matrix();
    assert_eq!(matrix.columns[0].confounder_id, "bp");
    assert_eq!(matrix.columns[1].confounder_id, "weight");
    assert_eq!(matrix.rows[0].subject_id, "patient-a");
    assert_eq!(matrix.rows[1].subject_id, "patient-b");
    assert_eq!(verified.missing_measurement_count(), 1);
}

#[test]
fn missing_cartesian_cell_fails_closed() {
    let fixture = fixture();
    let mut measurements = complete_measurements(&fixture, true);
    measurements.remove(0);
    assert!(matches!(
        build_baseline_covariate_matrix_v1(
            &fixture.protocol,
            &fixture.plan,
            &fixture.contributions,
            &measurements,
        ),
        Err(BaselineMatrixV1Error::MissingMeasurementCell { .. })
    ));
}

#[test]
fn duplicate_cell_fails_closed() {
    let fixture = fixture();
    let mut measurements = complete_measurements(&fixture, true);
    let bp_policy = policy("bp", 50);
    let duplicate_facts = vec![clinical_fact(
        "patient-a",
        "8480-6",
        "2.83",
        90,
        ClinicalValue::Integer(120),
    )];
    measurements.push(measurement(
        &fixture.protocol,
        &fixture.plan,
        &fixture.time_zero_a,
        &bp_policy,
        &duplicate_facts,
        64,
    ));
    assert!(matches!(
        build_baseline_covariate_matrix_v1(
            &fixture.protocol,
            &fixture.plan,
            &fixture.contributions,
            &measurements,
        ),
        Err(BaselineMatrixV1Error::DuplicateMeasurementCell { .. })
    ));
}

#[test]
fn mixed_measurement_policy_for_one_confounder_fails_closed() {
    let fixture = fixture();
    let weight_policy = policy("weight", 51);
    let bp_policy_a = policy("bp", 50);
    let bp_policy_b = policy("bp", 99);
    let measurements = vec![
        measurement(
            &fixture.protocol,
            &fixture.plan,
            &fixture.time_zero_a,
            &bp_policy_a,
            &[clinical_fact("patient-a", "8480-6", "2.83", 90, ClinicalValue::Integer(120))],
            70,
        ),
        measurement(
            &fixture.protocol,
            &fixture.plan,
            &fixture.time_zero_b,
            &bp_policy_b,
            &[clinical_fact("patient-b", "8480-6", "2.83", 89, ClinicalValue::Integer(118))],
            71,
        ),
        measurement(&fixture.protocol, &fixture.plan, &fixture.time_zero_a, &weight_policy, &[], 72),
        measurement(&fixture.protocol, &fixture.plan, &fixture.time_zero_b, &weight_policy, &[], 73),
    ];
    assert!(matches!(
        build_baseline_covariate_matrix_v1(
            &fixture.protocol,
            &fixture.plan,
            &fixture.contributions,
            &measurements,
        ),
        Err(BaselineMatrixV1Error::MixedMeasurementPolicy { confounder_id }) if confounder_id == "bp"
    ));
}

#[test]
fn same_timestamp_but_different_time_zero_evidence_cannot_rebind_measurement() {
    let fixture = fixture();
    let alternate_time_zero = build_time_zero_receipt_v2(
        &fixture.protocol,
        &fixture.plan,
        subject("patient-a"),
        "site",
        100,
        artifact("source-event", "alternate-time-zero-patient-a", 88),
    )
    .unwrap();
    let bp_policy = policy("bp", 50);
    let weight_policy = policy("weight", 51);
    let measurements = vec![
        measurement(
            &fixture.protocol,
            &fixture.plan,
            &alternate_time_zero,
            &bp_policy,
            &[clinical_fact("patient-a", "8480-6", "2.83", 90, ClinicalValue::Integer(120))],
            80,
        ),
        measurement(&fixture.protocol, &fixture.plan, &fixture.time_zero_a, &weight_policy, &[], 81),
        measurement(&fixture.protocol, &fixture.plan, &fixture.time_zero_b, &bp_policy, &[], 82),
        measurement(&fixture.protocol, &fixture.plan, &fixture.time_zero_b, &weight_policy, &[], 83),
    ];
    assert!(matches!(
        build_baseline_covariate_matrix_v1(
            &fixture.protocol,
            &fixture.plan,
            &fixture.contributions,
            &measurements,
        ),
        Err(BaselineMatrixV1Error::MeasurementTimeZeroMismatch { .. })
    ));
}

#[test]
fn missingness_and_canonical_order_are_part_of_matrix_identity() {
    let fixture = fixture();
    let with_missing = complete_measurements(&fixture, true);
    let complete = complete_measurements(&fixture, false);
    let missing_matrix = build_baseline_covariate_matrix_v1(
        &fixture.protocol,
        &fixture.plan,
        &fixture.contributions,
        &with_missing,
    )
    .unwrap();
    let complete_matrix = build_baseline_covariate_matrix_v1(
        &fixture.protocol,
        &fixture.plan,
        &fixture.contributions,
        &complete,
    )
    .unwrap();
    assert_ne!(missing_matrix.digest(), complete_matrix.digest());

    let mut noncanonical_rows = missing_matrix.matrix().clone();
    noncanonical_rows.rows.swap(0, 1);
    assert!(matches!(
        baseline_covariate_matrix_digest_v1(&noncanonical_rows),
        Err(BaselineMatrixV1Error::NonCanonicalRowOrder)
    ));

    let mut noncanonical_columns = missing_matrix.matrix().clone();
    noncanonical_columns.columns.swap(0, 1);
    assert!(matches!(
        baseline_covariate_matrix_digest_v1(&noncanonical_columns),
        Err(BaselineMatrixV1Error::NonCanonicalColumnOrder)
    ));
}
