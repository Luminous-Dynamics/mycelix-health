# Boundaries

## In scope

- exact QUAL-EVID-003/#214 profile-match artifact identity;
- exact QUAL-EVID-004/#217 current-head artifact identity;
- exact cross-artifact lineage/receipt/policy/trust/deployment/profile-match agreement;
- profile-match time within qualification lifetime and not in the future;
- checkpoint verification not earlier than the bound profile match;
- lower QUAL-EVID-006 checkpoint integration;
- lower QUAL-EVID-002 profile/ledger/receipt freshness checks;
- product-level monotonic attempt-time fence;
- non-cloneable provenance-rich final #208 token.

## Out of scope

- native Rust parsing of JSON reports;
- native Rust Ed25519/quorum verification;
- durable product-consumption time persistence;
- external checkpoint anti-rollback anchor (#216);
- Patient-v2 activation;
- clinical/legal conclusions.

## Adapter truth

`VerifiedProfile208Match::from_qualified_profile_report(...)` and
`VerifiedCurrentHead208::from_qualified_checkpoint_report(...)` represent strict
adapter boundaries from independently verified #214/#217 artifacts. They are not
cryptographic verification functions.

No boolean verification flag exists in this crate's public API.
