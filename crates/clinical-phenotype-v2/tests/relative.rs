use mycelix_clinical_phenotype_v2::*;
use mycelix_clinical_semantics::{
    ClinicalFact, ClinicalValue, CodeableConcept, Coding, FactProvenance, SubjectRef,
};

fn artifact(namespace: &str, id: &str, byte: u8) -> EvidenceArtifactIdentityV2 {
    EvidenceArtifactIdentityV2 {
        namespace: namespace.to_string(),
        artifact_id: id.to_string(),
        version: "1".to_string(),
        digest: [byte; 32],
    }
}

fn definition_absent() -> RelativePhenotypeDefinitionV2 {
    RelativePhenotypeDefinitionV2 {
        schema_version: PHENOTYPE_V2_VERSION,
        phenotype_id: "no-prior-event".to_string(),
        version: "1".to_string(),
        subject_resource_type: "Patient".to_string(),
        window: RelativeObservationWindowV2 {
            start_offset_micros: -1_000,
            end_offset_micros: 0,
            required_coverage_domains: vec!["diagnoses".to_string()],
        },
        criterion: RelativeCriterionV2::FactAbsent {
            criterion_id: "no-event".to_string(),
            concept: ConceptIdentityV2 {
                system: "http://snomed.info/sct".to_string(),
                code: "123".to_string(),
                version: "2026-09".to_string(),
            },
            coverage_domain: "diagnoses".to_string(),
        },
        definition_evidence: artifact("phenotype-definition", "no-prior-event", 1),
    }
}

fn context(anchor: i64, status: CoverageStatusV2, evidence_byte: u8) -> PhenotypeEvaluationContextV2 {
    PhenotypeEvaluationContextV2 {
        schema_version: PHENOTYPE_V2_VERSION,
        anchor_micros: anchor,
        anchor_evidence: artifact("time-zero", "anchor", 2),
        coverage: vec![CoverageEvidenceV2 {
            domain: "diagnoses".to_string(),
            status,
            source: artifact("coverage", "diagnoses", evidence_byte),
        }],
        context_evidence: vec![artifact("dataset", "site-a", 4)],
    }
}

fn fact(subject_id: &str, effective_at: i64) -> ClinicalFact {
    ClinicalFact {
        fact_id: format!("fact:{subject_id}:{effective_at}"),
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
        effective_at_micros: effective_at,
        provenance: FactProvenance {
            source_system: "test".to_string(),
            source_resource_type: "Condition".to_string(),
            source_resource_id: format!("condition:{subject_id}:{effective_at}"),
            source_version: Some("1".to_string()),
            recorded_at_micros: effective_at,
            asserted_by: None,
            transformation: None,
        },
        uncertainty: None,
    }
}

#[test]
fn definition_identity_is_independent_of_subject_anchor() {
    let definition = definition_absent();
    let digest = relative_phenotype_definition_digest_v2(&definition).unwrap();
    let c1 = context(10_000, CoverageStatusV2::Complete, 3);
    let c2 = context(20_000, CoverageStatusV2::Complete, 3);
    assert_eq!(digest, relative_phenotype_definition_digest_v2(&definition).unwrap());
    assert_ne!(
        phenotype_evaluation_context_digest_v2(&definition, &c1).unwrap(),
        phenotype_evaluation_context_digest_v2(&definition, &c2).unwrap()
    );
}

#[test]
fn absence_requires_complete_coverage() {
    let definition = definition_absent();
    let complete = evaluate_relative_phenotype_v2(
        &definition,
        &context(10_000, CoverageStatusV2::Complete, 3),
        "p1",
        &[],
    ).unwrap();
    let incomplete = evaluate_relative_phenotype_v2(
        &definition,
        &context(10_000, CoverageStatusV2::Incomplete, 3),
        "p1",
        &[],
    ).unwrap();
    assert_eq!(complete.state, CriterionStateV2::Satisfied);
    assert_eq!(incomplete.state, CriterionStateV2::Indeterminate);
}

#[test]
fn contradictory_positive_fact_refutes_absence_even_with_incomplete_coverage() {
    let definition = definition_absent();
    let evaluation = evaluate_relative_phenotype_v2(
        &definition,
        &context(10_000, CoverageStatusV2::Incomplete, 3),
        "p1",
        &[fact("p1", 9_500)],
    ).unwrap();
    assert_eq!(evaluation.state, CriterionStateV2::NotSatisfied);
}

#[test]
fn fact_outside_derived_window_does_not_refute_absence() {
    let definition = definition_absent();
    let evaluation = evaluate_relative_phenotype_v2(
        &definition,
        &context(10_000, CoverageStatusV2::Complete, 3),
        "p1",
        &[fact("p1", 10_000)],
    ).unwrap();
    assert_eq!(evaluation.window_start_micros, 9_000);
    assert_eq!(evaluation.window_end_micros, 10_000);
    assert_eq!(evaluation.state, CriterionStateV2::Satisfied);
}

#[test]
fn changing_coverage_evidence_changes_context_and_evaluation_identity_not_definition() {
    let definition = definition_absent();
    let d1 = relative_phenotype_definition_digest_v2(&definition).unwrap();
    let c1 = context(10_000, CoverageStatusV2::Complete, 3);
    let c2 = context(10_000, CoverageStatusV2::Complete, 9);
    let e1 = evaluate_relative_phenotype_v2(&definition, &c1, "p1", &[]).unwrap();
    let e2 = evaluate_relative_phenotype_v2(&definition, &c2, "p1", &[]).unwrap();
    assert_eq!(d1, relative_phenotype_definition_digest_v2(&definition).unwrap());
    assert_ne!(e1.evaluation_context_digest, e2.evaluation_context_digest);
    assert_ne!(
        relative_phenotype_evaluation_digest_v2(&e1).unwrap(),
        relative_phenotype_evaluation_digest_v2(&e2).unwrap()
    );
}

#[test]
fn coverage_context_must_match_required_domains_exactly() {
    let definition = definition_absent();
    let mut missing = context(10_000, CoverageStatusV2::Complete, 3);
    missing.coverage.clear();
    assert!(matches!(
        missing.validate_against(&definition),
        Err(PhenotypeV2Error::CoverageDomainSetMismatch)
    ));

    let mut extra = context(10_000, CoverageStatusV2::Complete, 3);
    extra.coverage.push(CoverageEvidenceV2 {
        domain: "labs".to_string(),
        status: CoverageStatusV2::Complete,
        source: artifact("coverage", "labs", 7),
    });
    assert!(matches!(
        extra.validate_against(&definition),
        Err(PhenotypeV2Error::CoverageDomainSetMismatch)
    ));
}

#[test]
fn cross_patient_facts_fail_even_when_outside_window() {
    let definition = definition_absent();
    let error = evaluate_relative_phenotype_v2(
        &definition,
        &context(10_000, CoverageStatusV2::Complete, 3),
        "p1",
        &[fact("p2", 50_000)],
    ).unwrap_err();
    assert!(matches!(error, PhenotypeV2Error::MixedSubjectFacts { .. }));
}

#[test]
fn time_arithmetic_overflow_fails_closed() {
    let definition = definition_absent();
    let error = evaluate_relative_phenotype_v2(
        &definition,
        &context(i64::MIN, CoverageStatusV2::Complete, 3),
        "p1",
        &[],
    ).unwrap_err();
    assert!(matches!(error, PhenotypeV2Error::TimeOverflow));
}
