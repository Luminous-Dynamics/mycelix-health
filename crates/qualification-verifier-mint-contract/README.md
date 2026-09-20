# Qualification Verifier Mint Contract

This crate freezes the portable V1 request and signed-statement bytes for an isolated verifier that will eventually authorize the private-field tokens introduced by QUAL-EVID-008 / #224.

It follows the same claim separation used by Xenia Forge authentication:

```text
portable canonical contract
!= real signature verification
!= challenge freshness / single-use
!= verifier endpoint identity
!= product token mint
```

## Request

`VerifierMintRequestV1` binds:

- exact token profile;
- exact product request digest;
- verifier-issued challenge;
- independent request nonce;
- semantic qualification-lineage digest;
- #217 checkpoint digest + epoch;
- #217 head sequence;
- lineage-state commitment;
- #217 verified-head digest;
- request validity window.

No raw PHI or care payload is required.

## Signed statement

`VerifierMintStatementV1` additionally binds:

- exact request commitment;
- verifier provider namespace / instance / key-lineage commitments;
- the complete generic accepted-anchor fields required by #222/#224;
- generic state-derivation evidence commitment;
- complete Health security-time fields;
- underlying evidence commitment;
- statement validity window;
- signature suite.

The statement constructor independently requires the generic anchor to match the requested Health lineage/epoch and requires the security-time evidence to match the exact generic anchor sequence/digest/state.

## Canonical framing

The domains are:

```text
MYCELIX-HEALTH-QUALIFICATION-VERIFIER-MINT-REQUEST-V1\0
MYCELIX-HEALTH-QUALIFICATION-VERIFIER-MINT-STATEMENT-V1\0
```

Integers are fixed-width big-endian. Optional digests use a one-byte presence tag followed by 32 bytes only when present. Frozen integration vectors pin both complete byte streams.

## Signature envelope

`HybridVerifierSignatureV1` carries fixed-size Ed25519 and ML-DSA-65 signature bytes and identifies the suite as `Ed25519MlDsa65` through the signed statement.

This crate does **not** verify those signatures.

`VerifierMintReceiptV1` is deliberately not named `Verified*`. Constructing a receipt proves only that values fit the portable shape.

## Follow-up

- 009B: real hybrid verification of these exact statement bytes;
- 009C: isolated daemon challenge consumption, endpoint/capability binding and key custody;
- 009D: product-side re-verification that mints the private #224 tokens.

Tracks #226/#225/#224 and Xenia Forge auth #345/#352.
