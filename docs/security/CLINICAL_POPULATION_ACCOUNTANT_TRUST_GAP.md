# P0 Design Note: Trusted Population Privacy Accountant Admission

## Gap

`mycelix-clinical-population-release` binds interactive releases to an exact accountant instance/method and verifies predecessor/sequence/query/output/mechanism/budget continuity.

However, `PrivacyAccountantReceiptV1` is a serializable pure-Rust artifact. A malicious in-process caller can manufacture a structurally coherent genesis and successor chain using the policy-bound public instance/method digests.

Therefore structural chain validation is **not** institutional trust in the accountant state.

## Required follow-up

Before interactive population release can be promoted:

1. establish a verifier-owned accountant state root outside caller control;
2. admit the exact accountant implementation/version/method under deployment policy;
3. bind each accepted successor to the previous trusted state;
4. reject alternate/forked genesis for the same accountant instance;
5. make query/output/mechanism replay semantics explicit;
6. preserve cumulative epsilon/delta and query-count continuity;
7. make accountant revocation/supersession append-only and auditable;
8. execute fork/replay/concurrent-query adversarial tests;
9. qualify the exact DP implementation against NIST SP 800-226 hazards and assumptions.

A likely runtime pattern is the same one used elsewhere in the health stack: a private verified transition is consumed into a privacy-minimized receipt, then a DNA/runtime-pinned authority attests one exact successor state. The exact design should be reviewed before implementation.

## Promotion rule

Until this boundary is implemented, interactive DP release is experimental preflight only. Do not describe a structurally valid `PrivacyAccountantReceiptV1` as trusted privacy-budget state.
