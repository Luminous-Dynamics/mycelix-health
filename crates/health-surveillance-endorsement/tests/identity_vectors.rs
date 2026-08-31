use health_surveillance_authority::{ProducerAuthorityGrant, ProducerAuthorityScope};
use health_surveillance_core::{
    BoundedUncertainty, CanonicalId, EvidenceProvenance, GeographicPrecision, GeographicScope,
    IndependenceGroup, MetricKind, ObservationWindow, ObservedMetric, SignalFamily, SourceKind,
    SourceRecordDigest, SurveillanceObservation,
};
use health_surveillance_endorsement::AuthorizedObservationEndorsement;
use sha2::{Digest, Sha256};

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
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

fn grant() -> ProducerAuthorityGrant {
    ProducerAuthorityGrant::new(
        id("public-health.za.gp"),
        id("mycelix:schema:surveillance-producer:v1"),
        id("did:mycelix:issuer-a"),
        id("did:mycelix:publisher-a"),
        id("lab-a"),
        ProducerAuthorityScope::new(
            vec![SourceKind::LaboratoryAggregate],
            vec![SignalFamily::Respiratory],
            vec![id("lab-feed-a")],
            vec![id("aggregate-protocol-v1")],
            vec![
                GeographicScope::new(
                    "health-district",
                    "district-17",
                    GeographicPrecision::District,
                )
                .unwrap(),
            ],
        )
        .unwrap(),
        9_000,
        20_000,
        [1; 32],
    )
    .unwrap()
}

fn endorsement_at(checked_at_unix_s: i64) -> AuthorizedObservationEndorsement {
    let observation = observation();
    let grant = grant();
    AuthorizedObservationEndorsement::new(
        id("mycelix-vc-active-status-v1"),
        grant.issuer_did().clone(),
        grant.id().unwrap(),
        observation.id().unwrap(),
        [3; 32],
        id("did:mycelix:publisher-a"),
        id("lab-a"),
        checked_at_unix_s,
        [9; 32],
        [8; 32],
    )
    .unwrap()
}

fn endorsement() -> AuthorizedObservationEndorsement {
    endorsement_at(13_701)
}

#[test]
fn observation_endorsement_id_v1_golden_vector() {
    // Independently recomputed from the canonical v1 endorsement encoding.
    // A different value requires an explicit endorsement identity version.
    assert_eq!(
        endorsement().id().unwrap().to_hex(),
        "93984c8ee8a19c48514734244e83348d56ded1687bae2faaab2e467f1b861294"
    );
}

#[test]
fn observation_endorsement_signing_transcript_v1_golden_vector() {
    let transcript = endorsement().signing_transcript().unwrap();
    assert_eq!(transcript.len(), 324);
    assert_eq!(
        format!("{:x}", Sha256::digest(&transcript)),
        "2b67e5583d6949f383dc88cf18f189df6203d708c97a43d7c39b21573299d64e"
    );
}

#[test]
fn signed_check_time_is_identity_and_signature_significant() {
    let original = endorsement_at(13_701);
    let changed_time = endorsement_at(13_702);

    assert_ne!(original.id().unwrap(), changed_time.id().unwrap());
    assert_ne!(
        original.signing_transcript().unwrap(),
        changed_time.signing_transcript().unwrap()
    );
}
