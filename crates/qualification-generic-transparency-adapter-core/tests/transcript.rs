use mycelix_qualification_anti_rollback_anchor_core as health;
use mycelix_qualification_generic_transparency_adapter_core::{
    HEALTH_GENERIC_DOMAIN_ID_V1, STATE_TRANSCRIPT_DOMAIN_V1, canonical_health_state_transcript,
};
use mycelix_qualification_product_authority_core as product;

fn b(value: u8) -> [u8; 32] {
    [value; 32]
}

#[test]
fn health_state_transcript_v1_has_exact_frozen_order_and_widths() {
    let state = health::ExternalAnchorState::new(
        product::LineageBindingDigest::new(b(1)).unwrap(),
        product::CheckpointDigest::new(b(2)).unwrap(),
        0x0102_0304_0506_0708,
        0x1112_1314_1516_1718,
        product::LineageStateCommitmentDigest::new(b(3)).unwrap(),
        product::VerifiedHeadDigest::new(b(4)).unwrap(),
    )
    .unwrap();

    let actual = canonical_health_state_transcript(state);
    let mut expected = Vec::new();
    expected.extend_from_slice(STATE_TRANSCRIPT_DOMAIN_V1);
    expected.extend_from_slice(&HEALTH_GENERIC_DOMAIN_ID_V1);
    expected.extend_from_slice(&b(1));
    expected.extend_from_slice(&b(2));
    expected.extend_from_slice(&0x0102_0304_0506_0708_u64.to_be_bytes());
    expected.extend_from_slice(&0x1112_1314_1516_1718_u64.to_be_bytes());
    expected.extend_from_slice(&b(3));
    expected.extend_from_slice(&b(4));

    assert_eq!(actual, expected);
    assert_eq!(actual.len(), STATE_TRANSCRIPT_DOMAIN_V1.len() + 32 * 5 + 16);
}
