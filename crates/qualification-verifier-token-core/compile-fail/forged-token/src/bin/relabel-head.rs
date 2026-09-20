use mycelix_qualification_verifier_token_core::VerifierAnchoredQualificationHead;

fn main() {
    // This fixture MUST NOT compile. Even if downstream code somehow obtains a
    // lower #220 AnchoredQualificationHead through an open reference trait seam,
    // it must not be able to relabel that object as verifier-bound authority.
    let _laundered = VerifierAnchoredQualificationHead {
        inner: todo!("lower #220 head supplied by attacker-controlled code"),
    };
}
