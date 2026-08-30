use health_surveillance_core::{
    BoundedUncertainty, CanonicalId, EvidenceProvenance, GeographicPrecision, GeographicScope,
    IndependenceGroup, MetricKind, ObservationWindow, ObservedMetric, SignalFamily, SourceKind,
    SourceRecordDigest, SurveillanceObservation,
};
use health_surveillance_lineage::{
    EvidenceLineageAttestation, LineageDescriptor, LineageKnowledge,
};
use sha2::{Digest, Sha256};

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn known(values: &[&str]) -> LineageKnowledge {
    LineageKnowledge::known(values.iter().map(|value| id(value))).unwrap()
}

fn observation() -> SurveillanceObservation {
    SurveillanceObservation::new(
        SignalFamily::Respiratory,
        SourceKind::LaboratoryAggregate,
        "lab-feed-a",
        IndependenceGroup::new("lineage-a").unwrap(),
        GeographicScope::new(
            "health-district",
            "district-17",
            GeographicPrecision::District,
        )
        .unwrap(),
        ObservationWindow::new(10_000, 13_600).unwrap(),
        13_700,
        100,
        ObservedMetric::new(
            MetricKind::FractionPositive,
            0.20,
            BoundedUncertainty::new(0.15, 0.25).unwrap(),
            "fraction",
        )
        .unwrap(),
        EvidenceProvenance::new(
            "lab-a",
            "aggregate-protocol-v1",
            "rev-1",
            Some("upstream-lab-a"),
            SourceRecordDigest::sha256([7; 32]).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn attestation() -> EvidenceLineageAttestation {
    let descriptor = LineageDescriptor::new(
        known(&["root-a"]),
        known(&["sample-a"]),
        known(&["collection:district-17"]),
        known(&["instrument:lab-analyzer-a"]),
        known(&["pipeline:aggregate-v1"]),
        known(&["control:org-a"]),
    )
    .unwrap();

    EvidenceLineageAttestation::new(
        id("lineage-domain:public-health-v1"),
        id("lineage-profile:multi-dimension-v1"),
        id("did:mycelix:lineage-auditor-a"),
        observation().id().unwrap(),
        descriptor,
        13_800,
        [4; 32],
        [5; 32],
    )
    .unwrap()
}

#[test]
fn lineage_attestation_id_v1_golden_vector() {
    assert_eq!(
        attestation().id().unwrap().to_hex(),
        "42aaef49957c5067bb838085f4c8ada81db9528ad0e9ba8d214c0bfed12c155b"
    );
}

#[test]
fn lineage_attestation_signing_transcript_v1_golden_vector() {
    let transcript = attestation().signing_transcript().unwrap();
    assert_eq!(transcript.len(), 420);
    assert_eq!(
        format!("{:x}", Sha256::digest(&transcript)),
        "8ac9405273c20bf508b168371242755276e04e695b2d97f86e5d2cc5e90dca64"
    );
}
