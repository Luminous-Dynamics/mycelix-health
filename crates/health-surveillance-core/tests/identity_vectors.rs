use health_surveillance_core::{
    AggregateReleasePolicy, BoundedUncertainty, CanonicalId, Digest32Algorithm, EvidenceBundle,
    EvidenceProvenance, GeographicPrecision, GeographicScope, IndependenceGroup,
    LineageDiversityPolicy, LineageDiversityStatus, MetricKind, ObservationWindow, ObservedMetric,
    SignalFamily, SourceKind, SourceRecordDigest, SurveillanceObservation,
};

fn observation_vector_fixture() -> SurveillanceObservation {
    SurveillanceObservation::new(
        SignalFamily::Respiratory,
        SourceKind::WastewaterAggregate,
        "ww-feed-17",
        IndependenceGroup::new("ww-lab-lineage-17").unwrap(),
        GeographicScope::new(
            "health-district",
            "district-17",
            GeographicPrecision::District,
        )
        .unwrap(),
        ObservationWindow::new(10_000, 20_000).unwrap(),
        21_000,
        500,
        ObservedMetric::new(
            MetricKind::ConcentrationIndex,
            1.25,
            BoundedUncertainty::new(1.0, 1.5).unwrap(),
            "normalized_concentration",
        )
        .unwrap(),
        EvidenceProvenance::new(
            "lab-a",
            "ww-protocol-v1",
            "rev-1",
            Some("sampler-aggregate-17"),
            SourceRecordDigest::new(Digest32Algorithm::Sha256, [3; 32]).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn bundle_observation(
    source_kind: SourceKind,
    source: &str,
    independence_group: &str,
    window: ObservationWindow,
    digest_byte: u8,
) -> SurveillanceObservation {
    SurveillanceObservation::new(
        SignalFamily::Respiratory,
        source_kind,
        source,
        IndependenceGroup::new(independence_group).unwrap(),
        GeographicScope::new("district", "d17", GeographicPrecision::District).unwrap(),
        window,
        window.end_unix_s + 60,
        200,
        ObservedMetric::new(
            MetricKind::FractionPositive,
            0.2,
            BoundedUncertainty::new(0.1, 0.3).unwrap(),
            "fraction",
        )
        .unwrap(),
        EvidenceProvenance::new(
            source,
            "protocol-v1",
            "rev-1",
            Some(independence_group),
            SourceRecordDigest::sha256([digest_byte; 32]).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn observation_id_v1_golden_vector() {
    let observation = observation_vector_fixture();
    // The neutral accessor is exactly the legacy v1 wire value; it changes no
    // serialization or identity bytes and makes no unique-human-cardinality claim.
    assert_eq!(
        observation.contributing_unit_count(),
        observation.cohort_size
    );
    assert_eq!(observation.contributing_unit_count(), 500);

    let id = observation.id().unwrap();
    assert!(id.matches_observation(&observation).unwrap());

    let mut substituted = observation.clone();
    substituted.provenance.source_revision = CanonicalId::new("rev-2").unwrap();
    assert!(!id.matches_observation(&substituted).unwrap());

    // Independently recomputed from the documented v1 domain-separated byte
    // encoding. A change to this value is an evidence-identity version change.
    assert_eq!(
        id.to_hex(),
        "4b9bde15d8c1466932a38c2eb50245e3d119656293dfbb15baaf4d0a3b8260ae"
    );
}

#[test]
fn release_policy_neutral_count_accessor_and_receipt_recompute_match_v1() {
    let observation = observation_vector_fixture();
    let policy = AggregateReleasePolicy::new(50, 3_600, GeographicPrecision::District).unwrap();
    assert_eq!(
        policy.min_contributing_unit_count(),
        policy.min_cohort_size()
    );
    assert_eq!(policy.min_contributing_unit_count(), 50);

    let assessment = policy.assess(&observation).unwrap();
    assert!(assessment.verifies_for_observation(&observation).unwrap());

    let mut substituted = observation;
    substituted.provenance.source_revision = CanonicalId::new("rev-2").unwrap();
    assert!(!assessment.verifies_for_observation(&substituted).unwrap());
}

#[test]
fn evidence_bundle_id_v1_golden_vector() {
    let laboratory = bundle_observation(
        SourceKind::LaboratoryAggregate,
        "lab-a",
        "lineage-a",
        ObservationWindow::new(1_000, 2_000).unwrap(),
        1,
    );
    let wastewater = bundle_observation(
        SourceKind::WastewaterAggregate,
        "ww-a",
        "lineage-b",
        ObservationWindow::new(1_500, 2_500).unwrap(),
        2,
    );

    let bundle = EvidenceBundle::new(vec![laboratory, wastewater]).unwrap();
    let id = bundle.id().unwrap();
    assert!(id.matches_bundle(&bundle).unwrap());
    assert_eq!(
        id.to_hex(),
        "71f3ae6ffcfc3e377765bd06d8cf44396c485ebb8e07ddb00ba01052e9e2769b"
    );

    let policy = LineageDiversityPolicy::new(2, 2).unwrap();
    let assessment = bundle.assess_lineage_diversity(policy).unwrap();
    assert!(assessment.verifies_for_bundle(&bundle).unwrap());

    let mut forged = assessment;
    forged.status = LineageDiversityStatus::Insufficient;
    assert!(!forged.verifies_for_bundle(&bundle).unwrap());
}
