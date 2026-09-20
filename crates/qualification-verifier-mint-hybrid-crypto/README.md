# Qualification Verifier Mint Hybrid Crypto

This crate performs real Ed25519 + ML-DSA-65 verification over the exact canonical `VerifierMintStatementV1` bytes frozen by QUAL-EVID-009A.

It intentionally mirrors the claim separation already used by Xenia Forge authentication:

```text
valid hybrid signatures
!= current verifier enrollment
!= challenge freshness / single-use
!= daemon endpoint identity
!= product token mint authority
```

## Cryptography

The direct cryptographic dependencies are exact-pinned to the same versions resolved by the reviewed Xenia stack:

- `ed25519-dalek = 2.2.0`;
- `ml-dsa = 0.1.1`;
- `sha2 = 0.10.9`.

ML-DSA-65 uses:

- public key: 1952 bytes;
- signature: 3309 bytes.

There is no classical-only fallback. Both signatures must verify over exactly:

```text
VerifierMintStatementV1::signing_transcript()
```

## Positive evidence types

`VerifiedVerifierMintSignaturesV1` proves only both signatures over one exact statement with one exact presented hybrid key pair.

`VerifiedVerifierMintCryptographyV1` additionally proves that pair equals one supplied current `CurrentVerifierEnrollmentV1`, and that the verifier provider/instance/key-lineage commitments inside the signed statement equal that enrollment.

The key-lineage commitment is derived by this crate from the exact public-key pair under the Health-specific domain:

```text
MYCELIX-HEALTH-QUALIFICATION-VERIFIER-KEY-LINEAGE-V1\0
```

## Exact 009A identity layout

009A intentionally exposed no verifier-identity getters because it was a byte-contract tranche. 009B therefore reads the three signed identity commitments from their frozen V1 canonical position (offset 204, length 96). The 009A full-byte vectors and 009B tests together make a layout drift an explicit protocol-version event.

## Reproducibility state

The direct dependency versions are exact-pinned, but this source tranche does **not** yet commit its generated transitive `Cargo.lock`.

The qualification workflow therefore:

1. generates the lockfile before source execution;
2. records its SHA-256;
3. uploads the generated lockfile as workflow evidence;
4. runs metadata/tests/Clippy under `--locked` after generation.

A later exact-lock child should commit the observed graph before any release/product use.

## Non-claims

This crate does not establish challenge issuance/consumption, replay prevention across daemon restart, endpoint/process identity, verifier key custody, trusted time, request/state/evidence commitment correctness beyond the bytes signed, or product token minting.

Tracks #228/#226/#225/#224 and Xenia Forge auth #352.
