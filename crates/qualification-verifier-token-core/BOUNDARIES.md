# QUAL-EVID-008 Phase A Boundaries

## This reference may establish

A successful exact-head qualifier may establish only these properties for the first production-facing wrapper:

- final generic accepted-anchor input is a private-field concrete token;
- final state-commitment input is a private-field concrete token bound to one exact canonical transcript;
- final Health security-time input is a private-field concrete token bound to one exact anchor/state;
- no public constructor or deserializer exists for those tokens;
- ordinary downstream safe Rust cannot construct the tokens with a struct literal;
- no caller-supplied `verified: bool` enters this public path;
- no arbitrary generic `G: Verified*` parameter enters `bind_verifier_tokens_to_health`;
- lower #222 mapping semantics are reused unchanged after token admission;
- verifier-bound Health proofs cannot be directly constructed by callers;
- verifier-bound anchored heads can be produced only through wrapper finalize/reconcile functions;
- the wrapper #208 gate accepts only `VerifierAnchoredQualificationHead`.

## Deliberate V1 availability boundary

This crate exposes **no production token mint function**. It therefore cannot by itself make the product path available.

Actual production minting must be added only by qualified verifier code that verifies canonical signed/witnessed evidence and then constructs the private tokens inside this authority crate or an equivalently sealed module.

## Reference traits remain below the boundary

The lower crates still contain open semantic traits including:

- `VerifiedGenericAcceptedAnchorV1`;
- `VerifiedGenericStateCommitmentDeriverV1`;
- `VerifiedHealthSecurityTimeFloorV1`;
- `VerifiedExternalAnchorProof`.

Those are implementation/reference seams, not final product authority. Production callers should depend on this concrete-token wrapper instead.

## Remaining non-claims

This tranche does not establish:

- generic witness signature/quorum verification;
- trusted production time-source verification;
- a production token mint implementation;
- qualification/profile token hardening outside this first slice;
- Health #222 or #220 qualification by implication;
- Patient-v2 activation;
- clinical validity or regulatory compliance.
