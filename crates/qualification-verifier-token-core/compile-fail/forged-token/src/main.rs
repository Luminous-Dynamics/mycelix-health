use mycelix_qualification_verifier_token_core::VerifiedHealthSecurityTimeTokenV1;

fn main() {
    // This fixture MUST NOT compile. A downstream crate must not be able to
    // construct an authority-bearing token with caller-selected fields.
    let _forged = VerifiedHealthSecurityTimeTokenV1 {
        anchor_sequence: 1,
        anchor_digest: [1; 32],
        state_commitment: [2; 32],
        proof_digest: [3; 32],
        security_time_floor_micros: 10,
        observed_at_micros: 20,
        expires_at_micros: 30,
    };
}
