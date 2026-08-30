use health_surveillance_core::{
    BoundedUncertainty, CanonicalId, EvidenceProvenance, GeographicPrecision, GeographicScope,
    IndependenceGroup, ObservationWindow, SignalFamily, SourceRecordDigest,
};
use health_surveillance_source_adapters::{
    LaboratoryFractionInput, ObservationContext, adapt_laboratory_fraction,
};

fn context(digest_byte: u8) -> ObservationContext {
    ObservationContext::new(
        CanonicalId::new("lab-feed-a").unwrap(),
        IndependenceGroup::new("lab-feed-a-lineage").unwrap(),
        GeographicScope::new(
            "health-district",
            "district-17",
            GeographicPrecision::District,
        )
        .unwrap(),
        ObservationWindow::new(10_000, 13_600).unwrap(),
        13_700,
        EvidenceProvenance::new(
            "producer-a",
            "aggregate-protocol-v1",
            "rev-1",
            Some("upstream-a"),
            SourceRecordDigest::sha256([digest_byte; 32]).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn adapt(digest_byte: u8) -> health_surveillance_core::SurveillanceObservation {
    adapt_laboratory_fraction(LaboratoryFractionInput {
        context: context(digest_byte),
        signal: SignalFamily::Respiratory,
        positive_count: 20,
        tested_count: 100,
        uncertainty: BoundedUncertainty::new(0.15, 0.25).unwrap(),
    })
    .unwrap()
}

#[test]
fn identical_derived_metric_with_different_source_commitment_has_different_identity() {
    let a = adapt(1);
    let b = adapt(2);

    // The adapter result is the same numerical aggregate, but the upstream
    // evidence commitment is deliberately identity-significant. The adapter is
    // not a lossless archive of all source-side aggregate inputs.
    assert_eq!(a.metric, b.metric);
    assert_eq!(a.cohort_size, b.cohort_size);
    assert_ne!(a.id().unwrap(), b.id().unwrap());
}
