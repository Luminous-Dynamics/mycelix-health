# Deployment Evidence Adversarial Test Matrix

| Case | Required result |
|---|---|
| WebSocket connected, no signer | Deny runtime call |
| Signed runtime, no authenticated `app_info` DNA | Deny hydration |
| `app_info` DNA differs from runtime DNA | Deny hydration |
| Runtime source-manifest digest differs | Deny hydration |
| Evidence DNA differs from runtime DNA | Deny hydration |
| Coordinator zomes reordered or omitted | Deny evidence |
| Any artifact digest is zero | Deny evidence |
| Unknown signer key ID | Deny evidence |
| Duplicate trusted signer key ID | Deny evidence |
| Trusted signer is revoked | Deny evidence |
| Signature is zero, malformed, or over altered bytes | Deny evidence |
| Source checkout placeholder | Compile, but deny live hydration |
| Valid evidence and unique active signer | Permit records hydration only |
| Consent/privacy repositories remain fixtures | Keep mixed provenance visible |
| Conductor readiness lost after hydration | Cache may display; block new writes |
| Attempt silent key replacement under same ID | Refuse release ceremony |
| Existing ceremony output directory | Refuse overwrite |
| CI unsigned evidence altered before signing | Signature verification fails |

The conductor-backed version of this matrix should run against at least two
agents and two DNA bundles, including one bundle that differs only in a
coordinator WASM, to demonstrate that the explicit artifact list detects the
change even where the DNA identity alone is insufficient.
