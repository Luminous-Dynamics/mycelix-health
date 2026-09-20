# Source Status

Current state: **SOURCE-STAGED / CI NOT YET OBSERVED / NO PASS CLAIM**

QUAL-EVID-009B currently contains:

- standalone Rust 1.96 real-crypto verifier crate;
- exact-pinned direct dependencies matching the reviewed Xenia crypto family;
- real Ed25519 verification;
- real ML-DSA-65 verification;
- exact same-transcript requirement for both signatures;
- exact current-enrollment key-pair binding;
- signed verifier provider/instance/key-lineage binding;
- independently derived Health-specific key-lineage commitment;
- deterministic statement and cryptographic-evidence SHA-256 commitments;
- adversarial tests for statement substitution, mixed key pairs, rotation, identity mismatch, and malformed ML-DSA key input.

The transitive Cargo lockfile is not committed in this tranche. The workflow is expected to generate it, preserve its digest/artifact, and run the remainder under `--locked`.

No challenge store, daemon endpoint/capability theorem, trusted-time theorem, product re-verification, or private token mint exists here.
