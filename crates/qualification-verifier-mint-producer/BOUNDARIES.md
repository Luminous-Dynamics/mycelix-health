# QUAL-EVID-009C Producer-Engine Boundaries

## This tranche may establish

A successful exact-head qualifier may establish only the isolated producer-engine theorem:

- one-time challenge generation and consume-on-attempt semantics within one process lifetime;
- challenge expiry/replay denial;
- challenge consumption occurs before request/evidence/signature processing;
- request substitution after evidence admission is denied;
- exact Health-state substitution is denied;
- evidence use before its admitted observation floor or after expiry is denied;
- in-process clock rollback is denied after a later time has been observed;
- exact 009A statement construction;
- real Ed25519 + ML-DSA-65 signing over identical statement bytes;
- producer self-verification through the staged 009B verifier and current enrollment;
- live-policy snapshot commitment changes when the hybrid key lineage/generation changes;
- separate hybrid provider attestation over exact receipt/freshness/policy commitments;
- no public production constructor for `VerifierOwnedMintEvidenceV1`.

## This tranche does not establish

It does **not** establish:

- challenge persistence across process restart;
- rollback-resistant challenge storage;
- trusted wall clock / hardware monotonic time;
- HTTP or Unix-socket endpoint security;
- process peer credentials;
- endpoint identity pinning;
- capability-token authentication;
- Origin/CORS policy;
- HSM/TPM/secure-element key custody;
- resistance to a compromised verifier process;
- production construction of `VerifierOwnedMintEvidenceV1` from #222/#224;
- product-side verification of `ProviderAuthenticatedMintReceiptV1`;
- private #224 token minting;
- #208 cutover authority;
- Patient-v2 activation;
- parent PR qualification by implication.

## Fail-closed rule

If a challenge has been consumed and any later request/evidence/crypto/policy step fails, the challenge remains consumed. The engine never restores it.

## Evidence rule

The successful mint API requires `VerifierOwnedMintEvidenceV1`, but this tranche exposes no public production constructor. That is intentional. Availability remains reduced until a concrete verifier-owned evidence adapter is qualified; caller-defined booleans or open `Verified*` traits are not accepted as substitutes.

## Producer receipt rule

`ProviderAuthenticatedMintReceiptV1` is evidence only. It is not prefixed `Verified` and does not implement product authorization. A later consumer-side verifier must authenticate the provider signatures under an independently trusted verifier identity and re-check exact request/state/profile/currentness before any private product token is minted.
