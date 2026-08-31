use surveillance_integrity::{
    GeographicPrecision, ReleasePolicyProperties, configured_release_policy_from_properties,
};

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}

#[test]
fn release_policy_id_v1_golden_vector() {
    let configured = configured_release_policy_from_properties(&ReleasePolicyProperties {
        policy_revision: "district-release-v1".to_string(),
        min_cohort_size: 50,
        min_window_s: 3_600,
        max_geographic_precision: GeographicPrecision::District,
    })
    .unwrap();

    // Independently recomputed from the v1 domain-separated policy encoding.
    // A different value requires an explicit release-policy identity version.
    assert_eq!(
        hex(configured.policy_id.as_bytes()),
        "881c02d0b10d921a2983790a48bbfd281e8a801295c5ce5d698fb7e8d26158ea"
    );
}
