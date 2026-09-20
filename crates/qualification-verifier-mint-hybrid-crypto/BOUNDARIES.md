# QUAL-EVID-009B Boundaries

## This verifier may establish

A successful exact-head execution may establish only:

- Ed25519 verification over the exact 009A V1 signing transcript;
- ML-DSA-65 verification over the exact same transcript;
- no classical-only fallback;
- fixed ML-DSA-65 public-key/signature sizes of 1952/3309 bytes;
- exact presented hybrid key pair matching one supplied current enrollment;
- exact signed verifier provider/instance/key-lineage commitments matching that enrollment;
- key-lineage commitment independently derived from the presented public-key pair under a Health-specific domain;
- statement substitution invalidating the hybrid proof;
- mixed individually valid keys failing joint enrollment binding;
- old keys failing after enrollment rotation;
- domain-separated statement and cryptographic-evidence commitments.

## This verifier does not establish

It does **not** establish:

- challenge issuance, freshness, or single-use consumption;
- replay prevention across verifier-daemon restart;
- verifier endpoint/process identity;
- verifier capability/token authentication;
- private-key custody or hardware isolation;
- trusted production time;
- cryptographic correctness of request/state/evidence commitments beyond the exact bytes signed;
- generic witness quorum correctness;
- product-side receipt re-verification;
- #224 private-token minting;
- parent PR qualification by implication;
- Patient-v2 activation, clinical validity, or regulatory compliance.

## Reproducibility boundary

Direct dependencies are exact-pinned, but the transitive lockfile is generated during qualification rather than committed in this tranche. Therefore a green 009B run is real-crypto execution evidence for the generated graph identified by the emitted lockfile digest; it is not yet an exact frozen release graph.

A later exact-lock child must commit and requalify the observed lockfile before production/release use.

## Enrollment boundary

`CurrentVerifierEnrollmentV1` is supplied by an external policy/daemon authority. 009B proves exact equality to that supplied enrollment; it does not prove the enrollment itself is current, authorized, or durably stored. 009C owns that daemon/policy theorem.
