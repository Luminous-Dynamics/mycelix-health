use health_surveillance_authority::{ProducerAuthorityGrant, ProducerAuthorityScope};
use health_surveillance_core::{
    CanonicalId, GeographicPrecision, GeographicScope, SignalFamily, SourceKind,
};
use sha2::{Digest, Sha256};

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn canonical_grant() -> ProducerAuthorityGrant {
    let scope = ProducerAuthorityScope::new(
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
    .unwrap();

    ProducerAuthorityGrant::new(
        id("identity-domain:public-health-v1"),
        id("mycelix:schema:health:surveillance-publisher:v1"),
        id("did:mycelix:issuer-a"),
        id("did:mycelix:publisher-a"),
        id("lab-a"),
        scope,
        1_000,
        10_000,
        [7; 32],
    )
    .unwrap()
}

#[test]
fn producer_authority_grant_id_v1_golden_vector() {
    assert_eq!(
        canonical_grant().id().unwrap().to_hex(),
        "ebe6298ab773421b778fbcc5059b4868702517ae542f42fd02bad41f3f7e597b"
    );
}

#[test]
fn producer_authority_signing_transcript_v1_golden_vector() {
    let transcript = canonical_grant().signing_transcript().unwrap();
    assert_eq!(transcript.len(), 351);
    assert_eq!(
        format!("{:x}", Sha256::digest(&transcript)),
        "fb7d91562d13582af3d6b3edc0d8434613da48f0aa46c655d6f8ae51c891e643"
    );
}
