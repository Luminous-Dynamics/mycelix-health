# QUAL-EVID-009A Boundaries

## This contract may establish

A successful exact-head qualifier may establish only:

- deterministic V1 mint-request canonical bytes;
- deterministic V1 signed-statement canonical bytes;
- explicit Health-specific domain separation;
- fixed-width big-endian integer framing;
- unambiguous optional predecessor/recovery framing;
- exact request binding to product purpose, challenge, nonce and Health qualification state;
- exact statement binding to verifier identity lineage, generic accepted-anchor fields, state-derivation evidence, Health security-time evidence and underlying evidence;
- exact Health lineage/epoch matching at statement construction;
- exact generic-anchor ↔ security-time sequence/digest/state matching;
- statement lifetime bounded by request, generic anchor and security-time evidence;
- fixed Ed25519 + ML-DSA-65 signature container sizes;
- frozen request and statement vectors.

## This contract does not establish

It does **not** establish:

- Ed25519 verification;
- ML-DSA-65 verification;
- current verifier enrollment/trust;
- verifier key custody;
- one-time challenge issuance or consumption;
- replay prevention across daemon restarts;
- endpoint/process identity;
- trusted production time;
- generic witness quorum correctness;
- token minting;
- product-side token admission;
- #224/#222/#220 qualification by implication;
- Patient-v2 activation, clinical validity or regulatory compliance.

## Authority rule

`VerifierMintReceiptV1` is a portable container, not authority. It intentionally has no `Verified` prefix. A later verifier must authenticate both hybrid signatures over `VerifierMintStatementV1::signing_transcript()` and separately prove challenge freshness/current verifier identity before any private production token can be minted.

## Privacy boundary

The contract requires only opaque digests, nonces, counters, timestamps, verifier identity commitments and evidence commitments. It does not require a patient identifier, care payload, clinical assertion, therapy text, spiritual content or raw qualification prose.
