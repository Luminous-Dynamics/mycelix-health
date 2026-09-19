use mycelix_clinical_phenotype_v2::ConceptIdentityV2;
use mycelix_clinical_semantics::{
    CodeableConcept, ClinicalFact, ClinicalValue, Coding, FactProvenance, SubjectRef,
};
use mycelix_target_trial_baseline_covariate_v1::*;
use mycelix_target_trial_cohort_receipts_v2::{build_time_zero_receipt_v2, TimeZeroReceiptV2};
use mycelix_target_trial_protocol_v2::*;

fn artifact(namespace: &str, id: &str, byte: u8) -> EvidenceArtifactIdentityV2 {
    EvidenceArtifactIdentityV2 {
        namespace: namespace.to_string(),
        artifact_id: id.to_string(),
        version: "1".to_string(),
        digest: [byte; 32],
    }
}

fn protocol() -> TargetTrialProtocolV2 {
    use mycelix_clinical_phenotype_v2::{
        EvidenceArtifactIdentityV2 as PhenotypeEvidenceIdentityV2, RelativeCriterionV2,
        RelativeObservationWindowV2, RelativePhenotypeDefinitionV2, PHENOTYPE_V2_VERSION,
    };
    let phenotype = RelativePhenotypeDefinitionV2 {
        schema_version: PHENOTYPE_V2_VERSION,
        phenotype_id: "eligible".to_string(),
        version: "1".to_string(),
        subject_resource_type: "Patient".to_string(),
        window: RelativeObservationWindowV2 {
            start_offset_micros: -100,
            end_offset_micros: 1,
            required_coverage_domains: vec!["clinical".to_string()],
        },
        criterion: RelativeCriterionV2::FactPresent {
            criterion_id: "eligible".to_string(),
            concept: ConceptIdentityV2 {
                system: "http://snomed.info/sct".to_string(),
                code: "111".to_string(),
                version: "2026-09".to_string(),
            },
            minimum_count: 1,
            coverage_domain: "clinical".to_string(),
        },
        definition_evidence: PhenotypeEvidenceIdentityV2 {
            namespace: "phenotype-evidence".to_string(),
            artifact_id: "eligible".to_string(),
            version: "1".to_string(),
            digest: [1; 32],
        },
    };
    TargetTrialProtocolV2 {
        schema_version: TARGET_TRIAL_PROTOCOL_V2_VERSION,
        protocol_id: "baseline-v1".to_string(),
        version: "1".to_string(),
        causal_question: "A versus B".to_string(),
        rationale_evidence: vec![artifact("literature", "rationale", 2)],
        eligibility: RelativePhenotypeRefV2::from_definition(&phenotype).unwrap(),
        treatment_strategies: vec![
            TreatmentStrategyV2 {
                strategy_id: "a".to_string(),
                label: "A".to_string(),
                intervention_definition: artifact("treatment", "a", 3),
            },
            TreatmentStrategyV2 {
                strategy_id: "b".to_string(),
                label: "B".to_string(),
                intervention_definition: artifact("treatment", "b", 4),
            },
        ],
        assignment: HypotheticalRandomAssignmentV2 {
            allocation_description: "hypothetical random assignment".to_string(),
            masking: MaskingV2::OpenLabel,
        },
        time_zero_definition: artifact("design", "time-zero", 5),
        follow_up: FollowUpV2 {
            maximum_duration_micros: 100,
            end_conditions: vec![artifact("design", "end", 6)],
        },
        outcomes: vec![OutcomeV2 {
            outcome_id: "outcome".to_string(),
            phenotype: RelativePhenotypeRefV2::from_definition(&phenotype).unwrap(),
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
            estimand_definition: artifact("estimand", "primary", 7),
        }],
        identifying_assumptions: vec![],
        analysis_plan: artifact("analysis", "primary", 8),
        sensitivity_analysis_plans: vec![],
    }
}

fn plan(protocol: &TargetTrialProtocolV2) -> TargetTrialEmulationPlanV2 {
    TargetTrialEmulationPlanV2 {
        schema_version: TARGET_TRIAL_PROTOCOL_V2_VERSION,
        emulation_id: "baseline-v1-site".to_string(),
        version: "1".to_string(),
        protocol_digest: target_trial_protocol_digest_v2(protocol).unwrap().into_bytes(),
        observational_design_statement: "observational".to_string(),
        data_sources: vec![DataSourceV2 {
            source_id: "site".to_string(),
            original_purpose: "care".to_string(),
            source_type: "EHR".to_string(),
            setting: "clinic".to_string(),
            geography: "site".to_string(),
            period_start_micros: 1,
            period_end_micros: 1_000,
            source_identity: artifact("source", "site", 9),
        }],
        eligibility_operationalization: artifact("op", "eligibility", 10),
        strategy_operationalizations: vec![
            StrategyOperationalizationV2 {
                strategy_id: "a".to_string(),
                classification_rule: artifact("op", "a", 11),
            },
            StrategyOperationalizationV2 {
                strategy_id: "b".to_string(),
                classification_rule: artifact("op", "b", 12),
            },
        ],
        time_zero_operationalization: artifact("op", "time-zero", 13),
        outcome_operationalizations: vec![OutcomeOperationalizationV2 {
            outcome_id: "outcome".to_string(),
            operationalization: artifact("op", "outcome", 14),
        }],
        baseline_confounders: vec![BaselineConfounderV2 {
            confounder_id: "bp".to_string(),
            definition: artifact("confounder", "bp", 15),
            measurement_window_end_offset_micros: -5,
        }],
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

fn time_zero(protocol: &TargetTrialProtocolV2, plan: &TargetTrialEmulationPlanV2, subject_id: &str) -> TimeZeroReceiptV2 {
    build_time_zero_receipt_v2(
        protocol,
        plan,
        SubjectRef { resource_type: "Patient".to_string(), id: subject_id.to_string() },
        "site",
        100,
        artifact("event", &format!("tz-{subject_id}"), 21),
    )
    .unwrap()
}

fn policy(rule: BaselineSelectionRuleV1) -> BaselineMeasurementPolicyV1 {
    BaselineMeasurementPolicyV1 {
        schema_version: BASELINE_COVARIATE_MEASUREMENT_V1_VERSION,
        confounder_id: "bp".to_string(),
        protocol_definition: artifact("confounder", "bp", 15),
        concept: ConceptIdentityV2 {
            system: "http://loinc.org".to_string(),
            code: "8480-6".to_string(),
            version: "2.83".to_string(),
        },
        window_start_offset_micros: -50,
        window_end_offset_micros: -5,
        selection_rule: rule,
        policy_evidence: artifact("measurement-policy", "bp", 22),
    }
}

fn fact(subject_id: &str, at: i64, value: i64, version: &str) -> ClinicalFact {
    ClinicalFact {
        fact_id: format!("bp-{subject_id}-{at}-{value}"),
        subject: SubjectRef { resource_type: "Patient".to_string(), id: subject_id.to_string() },
        concept: CodeableConcept {
            coding: vec![Coding {
                system: "http://loinc.org".to_string(),
                code: "8480-6".to_string(),
                display: None,
                version: Some(version.to_string()),
            }],
            text: None,
        },
        value: ClinicalValue::Integer(value),
        effective_at_micros: at,
        provenance: FactProvenance {
            source_system: "test".to_string(),
            source_resource_type: "Observation".to_string(),
            source_resource_id: format!("obs-{subject_id}-{at}"),
            source_version: Some("1".to_string()),
            recorded_at_micros: at,
            asserted_by: None,
            transformation: None,
        },
        uncertainty: None,
    }
}

fn evidence<'a>(
    protocol: &'a TargetTrialProtocolV2,
    plan: &'a TargetTrialEmulationPlanV2,
    time_zero: &'a TimeZeroReceiptV2,
    policy: &'a BaselineMeasurementPolicyV1,
    facts: &'a [ClinicalFact],
    candidate_set: &'a EvidenceArtifactIdentityV2,
) -> BaselineMeasurementEvidenceV1<'a> {
    BaselineMeasurementEvidenceV1 {
        protocol,
        plan,
        time_zero,
        policy,
        candidate_set_evidence: candidate_set,
        candidate_facts: facts,
    }
}

#[test]
fn unique_measurement_is_selected() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan, "p1");
    let policy = policy(BaselineSelectionRuleV1::UniqueOnly);
    let facts = vec![fact("p1", 90, 120, "2.83")];
    let candidate_set = artifact("query-result", "bp-p1", 23);
    let verified = build_baseline_covariate_measurement_v1(&evidence(
        &protocol, &plan, &time_zero, &policy, &facts, &candidate_set,
    ))
    .unwrap();
    assert!(matches!(verified.receipt().state, BaselineMeasurementStateV1::Selected { effective_at_micros: 90, .. }));
}

#[test]
fn empty_candidate_set_is_missing_not_clinical_absence() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan, "p1");
    let policy = policy(BaselineSelectionRuleV1::UniqueOnly);
    let candidate_set = artifact("query-result", "bp-p1-empty", 24);
    let verified = build_baseline_covariate_measurement_v1(&evidence(
        &protocol, &plan, &time_zero, &policy, &[], &candidate_set,
    ))
    .unwrap();
    assert_eq!(verified.receipt().state, BaselineMeasurementStateV1::MissingMeasurement);
}

#[test]
fn unique_only_rejects_multiple_measurements() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan, "p1");
    let policy = policy(BaselineSelectionRuleV1::UniqueOnly);
    let facts = vec![fact("p1", 80, 118, "2.83"), fact("p1", 90, 120, "2.83")];
    let candidate_set = artifact("query-result", "bp-p1", 25);
    assert!(matches!(
        build_baseline_covariate_measurement_v1(&evidence(&protocol, &plan, &time_zero, &policy, &facts, &candidate_set)),
        Err(BaselineCovariateV1Error::AmbiguousMultipleCandidates)
    ));
}

#[test]
fn latest_rule_selects_latest_measurement() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan, "p1");
    let policy = policy(BaselineSelectionRuleV1::LatestAtOrBeforeEnd);
    let facts = vec![fact("p1", 80, 118, "2.83"), fact("p1", 90, 120, "2.83")];
    let candidate_set = artifact("query-result", "bp-p1", 26);
    let verified = build_baseline_covariate_measurement_v1(&evidence(
        &protocol, &plan, &time_zero, &policy, &facts, &candidate_set,
    ))
    .unwrap();
    assert!(matches!(verified.receipt().state, BaselineMeasurementStateV1::Selected { effective_at_micros: 90, .. }));
}

#[test]
fn tied_latest_measurements_fail_closed() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan, "p1");
    let policy = policy(BaselineSelectionRuleV1::LatestAtOrBeforeEnd);
    let facts = vec![fact("p1", 90, 118, "2.83"), fact("p1", 90, 120, "2.83")];
    let candidate_set = artifact("query-result", "bp-p1", 27);
    assert!(matches!(
        build_baseline_covariate_measurement_v1(&evidence(&protocol, &plan, &time_zero, &policy, &facts, &candidate_set)),
        Err(BaselineCovariateV1Error::AmbiguousSelectionTie)
    ));
}

#[test]
fn post_baseline_or_too_late_measurement_is_rejected() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan, "p1");
    let policy = policy(BaselineSelectionRuleV1::UniqueOnly);
    let facts = vec![fact("p1", 99, 120, "2.83")];
    let candidate_set = artifact("query-result", "bp-p1", 28);
    assert!(matches!(
        build_baseline_covariate_measurement_v1(&evidence(&protocol, &plan, &time_zero, &policy, &facts, &candidate_set)),
        Err(BaselineCovariateV1Error::CandidateOutsideBaselineWindow)
    ));
}

#[test]
fn terminology_version_substitution_is_rejected() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan, "p1");
    let policy = policy(BaselineSelectionRuleV1::UniqueOnly);
    let facts = vec![fact("p1", 90, 120, "2.82")];
    let candidate_set = artifact("query-result", "bp-p1", 29);
    assert!(matches!(
        build_baseline_covariate_measurement_v1(&evidence(&protocol, &plan, &time_zero, &policy, &facts, &candidate_set)),
        Err(BaselineCovariateV1Error::CandidateConceptMismatch)
    ));
}

#[test]
fn cross_patient_candidate_is_rejected() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan, "p1");
    let policy = policy(BaselineSelectionRuleV1::UniqueOnly);
    let facts = vec![fact("p2", 90, 120, "2.83")];
    let candidate_set = artifact("query-result", "bp-p1", 30);
    assert!(matches!(
        build_baseline_covariate_measurement_v1(&evidence(&protocol, &plan, &time_zero, &policy, &facts, &candidate_set)),
        Err(BaselineCovariateV1Error::SubjectMismatch)
    ));
}

#[test]
fn policy_cannot_extend_past_protocol_baseline_cutoff() {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = time_zero(&protocol, &plan, "p1");
    let mut policy = policy(BaselineSelectionRuleV1::UniqueOnly);
    policy.window_end_offset_micros = -1;
    let candidate_set = artifact("query-result", "bp-p1", 31);
    assert!(matches!(
        build_baseline_covariate_measurement_v1(&evidence(&protocol, &plan, &time_zero, &policy, &[], &candidate_set)),
        Err(BaselineCovariateV1Error::WindowExceedsProtocolBaselineCutoff)
    ));
}
