# Qualification Verifier Mint Producer V1

This standalone crate is the producer-engine tranche of QUAL-EVID-009C.

It composes the portable 009A contract with the real 009B hybrid verifier and adds the producer-side freshness/policy ceremony before returning a portable provider-authenticated receipt.

## Core ordering

```text
observe monotonic process time
        ↓
consume exact one-time challenge
        ↓
validate request window
        ↓
validate verifier-owned evidence is bound to
  exact request commitment + exact Health state
        ↓
construct exact 009A statement
        ↓
Ed25519 AND ML-DSA-65 sign same statement bytes
        ↓
self-verify through 009B + current enrollment
        ↓
bind challenge-consumption + policy-snapshot commitments
        ↓
hybrid-sign producer attestation
        ↓
ProviderAuthenticatedMintReceiptV1
```

The challenge is removed before request/evidence/signature processing. A later failure never restores it.

## Evidence authority boundary

`VerifierOwnedMintEvidenceV1` has no public production constructor in this tranche. Therefore external application code cannot manufacture the anchor/time evidence required by the successful mint path.

A later integration must create that type only from concrete verifier-owned results for the generic anchor, Health state-commitment derivation, anti-rollback/current-head evidence and Health security-time proof.

## Producer envelope

The provider attestation is domain-separated from the 009A mint statement and binds:

- exact mint-receipt commitment;
- logical verifier-id commitment;
- exact challenge-consumption commitment;
- exact live-policy snapshot commitment;
- producer issue/expiry times.

Both provider signatures cover those same exact bytes.

## Software key holder

The first reference owns Ed25519 and ML-DSA-65 signing keys in the isolated process. It intentionally does not claim HSM/TPM/secure-element custody or compromise resistance. The public key lineage is independently derived by 009B from the exact hybrid pair.

## Current storage/time boundary

V1 uses:

- an in-memory one-time challenge store;
- an in-process monotonic observation floor;
- caller-supplied `now_micros` whose external trust is not established here.

A process restart therefore does not preserve challenge-consumption state or the prior time floor. The daemon/persistence child must close that boundary before production use.

## Product boundary

`ProviderAuthenticatedMintReceiptV1` is portable evidence, not product authority. Product code must independently verify the provider envelope, exact request/state/profile/currentness and trusted verifier identity before constructing any private token from #224.

## No transport theorem

This crate contains no HTTP, Unix socket, peer-credential, origin-allowlist or capability-token implementation. Those remain a separate daemon adapter so transport/process identity can be qualified independently from the producer state machine.
