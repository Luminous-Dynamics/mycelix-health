use mycelix_clinical_phenotype_v2::{
    ConceptIdentityV2, CoverageEvidenceV2, CoverageStatusV2,
    EvidenceArtifactIdentityV2 as PhenotypeEvidenceIdentityV2,
    PhenotypeEvaluationContextV2, RelativeCriterionV2, RelativeObservationWindowV2,
    RelativePhenotypeDefinitionV2, PHENOTYPE_V2_VERSION,
};
use mycelix_clinical_semantics::{
    CodeableConcept, ClinicalFact, ClinicalValue, Coding, FactProvenance, Quantity, SubjectRef,
};
use mycelix_target_trial_analysis_contribution_v1::{
    build_fixed_horizon_contribution_v1, FixedHorizonContributionEvidenceV1,
};
use mycelix_target_trial_baseline_covariate_v1::{
    build_baseline_covariate_measurement_v1, BaselineMeasurementEvidenceV1,
    BaselineMeasurementPolicyV1, BaselineSelectionRuleV1,
    BASELINE_COVARIATE_MEASUREMENT_V1_VERSION,
};
use mycelix_target_trial_baseline_matrix_v1::{
    build_baseline_covariate_matrix_v1, VerifiedBaselineCovariateMatrixV1,
};
use mycelix_target_trial_cohort_receipts_v2::{
    build_eligibility_receipt_v2, build_strategy_classification_receipt_v2,
    build_time_zero_receipt_v2, compose_cohort_entry_v2, time_zero_anchor_evidence_v2,
};
use mycelix_target_trial_covariate_encoding_v1::*;
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
        protocol_id: "tte:v2:encoding".to_string(),
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
            end_conditions: vec![artifact("study-design", "end", 7)],
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
        emulation_id: "tte:v2:encoding:site".to_string(),
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
            source_identity: artifact("source", "site", 10),
        }],
        eligibility_operationalization: artifact("op", "eligibility", 11),
        strategy_operationalizations: vec![
            StrategyOperationalizationV2 {
                strategy_id: "a".to_string(),
                classification_rule: artifact("op", "a", 12),
            },
            StrategyOperationalizationV2 {
                strategy_id: "b".to_string(),
                classification_rule: artifact("op", "b", 13),
            },
        ],
        time_zero_operationalization: artifact("op", "time-zero", 14),
        outcome_operationalizations: vec![OutcomeOperationalizationV2 {
            outcome_id: "outcome".to_string(),
            operationalization: artifact("op", "outcome", 15),
        }],
        baseline_confounders: vec![BaselineConfounderV2 {
            confounder_id: "x".to_string(),
            definition: artifact("confounder", "x", 16),
            measurement_window_end_offset_micros: -5,
        }],
        censoring_plan: artifact("analysis", "censoring", 17),
        adherence_plan: None,
        time_varying_confounding_plan: None,
        missing_data_plan: artifact("analysis", "missing", 18),
        estimator_plan: artifact("analysis", "estimator", 19),
        software_environment: SoftwareEnvironmentV2 {
            software: artifact("software", "analysis", 20),
            environment: artifact("environment", "analysis", 21),
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

fn fact(code: &str, system: &str, version: &str, at: i64, value: ClinicalValue) -> ClinicalFact {
    ClinicalFact {
        fact_id: format!("fact-{code}-{at}"),
        subject: subject(),
        concept: CodeableConcept {
            coding: vec![Coding {
                system: system.to_string(),
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
            source_resource_id: format!("obs-{code}-{at}"),
            source_version: Some("1".to_string()),
            recorded_at_micros: at,
            asserted_by: None,
            transformation: None,
        },
        uncertainty: None,
    }
}

struct EncodingFixture {
    baseline: VerifiedBaselineCovariateMatrixV1,
    source_fact: Option<ClinicalFact>,
}

fn fixture(value: Option<ClinicalValue>) -> EncodingFixture {
    let protocol = protocol();
    let plan = plan(&protocol);
    let time_zero = build_time_zero_receipt_v2(
        &protocol,
        &plan,
        subject(),
        "site",
        100,
        artifact("source-event", "time-zero", 22),
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
            source: phenotype_artifact("coverage", "eligibility", 23),
        }],
        context_evidence: vec![],
    };
    let eligibility_facts = vec![fact(
        "111",
        "http://snomed.info/sct",
        "2026-09",
        95,
        ClinicalValue::Boolean(true),
    )];
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
        artifact("classification", "a", 24),
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
    let follow_up = build_follow_up_receipt_v2(
        &protocol,
        &plan,
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
            source: phenotype_artifact("coverage", "outcome", 25),
        }],
        context_evidence: vec![],
    };
    let outcome_facts = vec![];
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
    let contribution = build_fixed_horizon_contribution_v1(&FixedHorizonContributionEvidenceV1 {
        protocol: &protocol,
        plan: &plan,
        cohort_entry: &entry,
        time_zero: &time_zero,
        follow_up: &follow_up,
        outcome_evidence: &outcome_evidence,
        outcome_receipt: &outcome_receipt,
        estimand_id: "primary",
    })
    .unwrap();

    let policy = BaselineMeasurementPolicyV1 {
        schema_version: BASELINE_COVARIATE_MEASUREMENT_V1_VERSION,
        confounder_id: "x".to_string(),
        protocol_definition: artifact("confounder", "x", 16),
        concept: ConceptIdentityV2 {
            system: "http://loinc.org".to_string(),
            code: "1234-5".to_string(),
            version: "2.83".to_string(),
        },
        window_start_offset_micros: -50,
        window_end_offset_micros: -5,
        selection_rule: BaselineSelectionRuleV1::UniqueOnly,
        policy_evidence: artifact("measurement-policy", "x", 26),
    };
    let source_fact = value.map(|value| fact("1234-5", "http://loinc.org", "2.83", 90, value));
    let candidate_facts = source_fact.iter().cloned().collect::<Vec<_>>();
    let candidate_set = artifact("query-result", "x-patient-1", 27);
    let measurement = build_baseline_covariate_measurement_v1(&BaselineMeasurementEvidenceV1 {
        protocol: &protocol,
        plan: &plan,
        time_zero: &time_zero,
        policy: &policy,
        candidate_set_evidence: &candidate_set,
        candidate_facts: &candidate_facts,
    })
    .unwrap();
    let baseline = build_baseline_covariate_matrix_v1(&protocol, &plan, &[contribution], &[measurement]).unwrap();
    EncodingFixture { baseline, source_fact }
}

fn policy(
    baseline: &VerifiedBaselineCovariateMatrixV1,
    rule: CovariateEncodingRuleV1,
    byte: u8,
) -> BaselineCovariateEncodingPolicyV1 {
    BaselineCovariateEncodingPolicyV1 {
        schema_version: TARGET_TRIAL_COVARIATE_ENCODING_V1_VERSION,
        confounder_id: "x".to_string(),
        measurement_policy_digest: baseline.matrix().columns[0].measurement_policy_digest,
        rule,
        policy_evidence: artifact("encoding-policy", "x", byte),
    }
}

fn source_slice(fixture: &EncodingFixture) -> Vec<ClinicalFact> {
    fixture.source_fact.iter().cloned().collect()
}

#[test]
fn integer_encoding_preserves_exact_integer() {
    let fixture = fixture(Some(ClinicalValue::Integer(120)));
    let policy = policy(&fixture.baseline, CovariateEncodingRuleV1::Integer, 30);
    let encoded = build_encoded_baseline_covariate_matrix_v1(
        &fixture.baseline,
        &[policy],
        &source_slice(&fixture),
    )
    .unwrap();
    assert_eq!(encoded.matrix().rows[0].cells[0].value, EncodedCovariateValueV1::Integer(120));
}

#[test]
fn boolean_decimal_and_exact_ucum_quantity_are_explicit() {
    let boolean = fixture(Some(ClinicalValue::Boolean(true)));
    let boolean_policy = policy(&boolean.baseline, CovariateEncodingRuleV1::Boolean, 31);
    let encoded_boolean = build_encoded_baseline_covariate_matrix_v1(
        &boolean.baseline,
        &[boolean_policy],
        &source_slice(&boolean),
    )
    .unwrap();
    assert_eq!(encoded_boolean.matrix().rows[0].cells[0].value, EncodedCovariateValueV1::Boolean(true));

    let decimal = fixture(Some(ClinicalValue::Decimal(1.25)));
    let decimal_policy = policy(&decimal.baseline, CovariateEncodingRuleV1::Decimal, 32);
    let encoded_decimal = build_encoded_baseline_covariate_matrix_v1(
        &decimal.baseline,
        &[decimal_policy],
        &source_slice(&decimal),
    )
    .unwrap();
    assert_eq!(encoded_decimal.matrix().rows[0].cells[0].value, EncodedCovariateValueV1::DecimalBits(1.25f64.to_bits()));

    let quantity = fixture(Some(ClinicalValue::Quantity(Quantity::ucum(75.0, "kg"))));
    let quantity_policy = policy(
        &quantity.baseline,
        CovariateEncodingRuleV1::QuantityUcumExact { unit_code: "kg".to_string() },
        33,
    );
    let encoded_quantity = build_encoded_baseline_covariate_matrix_v1(
        &quantity.baseline,
        &[quantity_policy],
        &source_slice(&quantity),
    )
    .unwrap();
    assert_eq!(
        encoded_quantity.matrix().rows[0].cells[0].value,
        EncodedCovariateValueV1::QuantityUcumExact {
            value_bits: 75.0f64.to_bits(),
            unit_code: "kg".to_string(),
        }
    );
}

#[test]
fn missing_measurement_stays_missing_without_source_fact() {
    let fixture = fixture(None);
    let policy = policy(&fixture.baseline, CovariateEncodingRuleV1::Integer, 34);
    let encoded = build_encoded_baseline_covariate_matrix_v1(&fixture.baseline, &[policy], &[]).unwrap();
    assert_eq!(encoded.missing_count(), 1);
    assert_eq!(encoded.matrix().rows[0].cells[0].value, EncodedCovariateValueV1::Missing);
    assert_eq!(encoded.matrix().rows[0].cells[0].selected_fact_snapshot_digest, None);
}

#[test]
fn source_fact_set_must_match_exact_selected_snapshot_set() {
    let fixture = fixture(Some(ClinicalValue::Integer(120)));
    let policy = policy(&fixture.baseline, CovariateEncodingRuleV1::Integer, 35);
    assert!(matches!(
        build_encoded_baseline_covariate_matrix_v1(&fixture.baseline, &[policy.clone()], &[]),
        Err(CovariateEncodingV1Error::SourceFactSetMismatch)
    ));

    let mut extra = source_slice(&fixture);
    extra.push(fact("1234-5", "http://loinc.org", "2.83", 89, ClinicalValue::Integer(119)));
    assert!(matches!(
        build_encoded_baseline_covariate_matrix_v1(&fixture.baseline, &[policy], &extra),
        Err(CovariateEncodingV1Error::SourceFactSetMismatch)
    ));
}

#[test]
fn patient_rebinding_changes_snapshot_identity_and_fails_closed() {
    let fixture = fixture(Some(ClinicalValue::Integer(120)));
    let encoding_policy = policy(&fixture.baseline, CovariateEncodingRuleV1::Integer, 40);
    let mut rebound = fixture.source_fact.clone().unwrap();
    rebound.subject.id = "patient-2".to_string();
    assert!(matches!(
        build_encoded_baseline_covariate_matrix_v1(
            &fixture.baseline,
            &[encoding_policy],
            &[rebound],
        ),
        Err(CovariateEncodingV1Error::SourceFactSetMismatch)
    ));
}

#[test]
fn value_variant_and_quantity_unit_mismatch_fail_closed() {
    let integer = fixture(Some(ClinicalValue::Integer(120)));
    let wrong_variant = policy(&integer.baseline, CovariateEncodingRuleV1::Decimal, 36);
    assert!(matches!(
        build_encoded_baseline_covariate_matrix_v1(
            &integer.baseline,
            &[wrong_variant],
            &source_slice(&integer),
        ),
        Err(CovariateEncodingV1Error::ClinicalValueVariantMismatch { .. })
    ));

    let quantity = fixture(Some(ClinicalValue::Quantity(Quantity::ucum(75.0, "kg"))));
    let wrong_unit = policy(
        &quantity.baseline,
        CovariateEncodingRuleV1::QuantityUcumExact { unit_code: "g".to_string() },
        37,
    );
    assert!(matches!(
        build_encoded_baseline_covariate_matrix_v1(
            &quantity.baseline,
            &[wrong_unit],
            &source_slice(&quantity),
        ),
        Err(CovariateEncodingV1Error::QuantityUnitMismatch { .. })
    ));
}

#[test]
fn measurement_policy_substitution_fails_closed() {
    let fixture = fixture(Some(ClinicalValue::Integer(120)));
    let mut encoding_policy = policy(&fixture.baseline, CovariateEncodingRuleV1::Integer, 38);
    encoding_policy.measurement_policy_digest = [99; 32];
    assert!(matches!(
        build_encoded_baseline_covariate_matrix_v1(
            &fixture.baseline,
            &[encoding_policy],
            &source_slice(&fixture),
        ),
        Err(CovariateEncodingV1Error::MeasurementPolicyDigestMismatch(_))
    ));
}

#[test]
fn tampered_serialized_value_cannot_pass_rebuild_verification() {
    let fixture = fixture(Some(ClinicalValue::Decimal(1.25)));
    let encoding_policy = policy(&fixture.baseline, CovariateEncodingRuleV1::Decimal, 39);
    let source = source_slice(&fixture);
    let verified = build_encoded_baseline_covariate_matrix_v1(
        &fixture.baseline,
        &[encoding_policy.clone()],
        &source,
    )
    .unwrap();
    let mut tampered = verified.matrix().clone();
    tampered.rows[0].cells[0].value = EncodedCovariateValueV1::DecimalBits(f64::NAN.to_bits());
    assert!(verify_encoded_baseline_covariate_matrix_v1(
        &fixture.baseline,
        &[encoding_policy],
        &source,
        &tampered,
    )
    .is_err());
}
