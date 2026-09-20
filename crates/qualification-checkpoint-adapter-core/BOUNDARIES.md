# Boundaries

This crate establishes exact Rust product-seam consumption semantics for an already-verified #217 head.

It proves neither checkpoint cryptography nor durable rollback resistance.

## In scope

- exact #217 head schema identity;
- nonzero head sequence/checkpoint epoch;
- exact semantic lineage/receipt/policy/trust/deployment cross-checks;
- exact #214 profile-match commitment cross-check;
- head currentness;
- local #210 ledger admissibility;
- #212 profile binding/freshness/monotonic-time checks;
- provenance-rich non-cloneable #208 token.

## Out of scope

- Ed25519 signature verification;
- checkpoint quorum verification;
- checkpoint store durability;
- whole-volume rollback protection (#216);
- native JSON parsing of the #217 report;
- trusted production clock;
- Patient-v2 activation;
- clinical/legal conclusions.

## Deliberate lower-reference dependency

The lower `mycelix-qualification-adapter-profile-core` still contains its original source-staged boolean current-head adapter. This wrapper never re-exports that type. Its only internal use occurs after the stronger #217 artifact passes all checks in this crate.
