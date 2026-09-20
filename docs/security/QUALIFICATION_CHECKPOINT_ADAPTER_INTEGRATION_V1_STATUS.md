# Qualification Checkpoint Adapter Integration V1 — Evidence Status

Current state: **SOURCE-STAGED / UNEXECUTED / CI REQUESTED LATER / NO PASS CLAIM**

This subject implements QUAL-EVID-006/#218 on top of QUAL-EVID-004/#217.

## What source exists

- public crate root moved from `src/lib.rs` to `src/integrated.rs`;
- QUAL-EVID-002 implementation retained privately as `mod legacy`;
- exact #217 verified-head artifact vocabulary represented in Rust;
- no public `verified: bool` current-head constructor;
- exact qualification/head/profile-match/time checks before #208 conversion;
- provenance-rich `Profile208CompositionCommitmentToken`;
- unit tests for exact valid conversion and principal negative cases.

## What is not yet proven

No successful exact-head Rust workflow has been observed for this subject yet.

Do not describe this branch as qualified until the dedicated workflow executes successfully on the exact PR head.

A green integration workflow would prove only the Rust composition theorem for consuming an already-verified #217 head. It would **not** independently qualify #217 checkpoint cryptography/durability, #216 external anti-rollback anchoring, #208 production activation, or clinical/regulatory claims.

## Upstream evidence dependencies

This subject depends on the separately qualified semantics of:

- QUAL-EVID-001/#210 governed receipt semantics;
- QUAL-EVID-002/#212 exact #208 adapter profile;
- QUAL-EVID-003/#214 governed Ed25519 qualification/profile evidence;
- QUAL-EVID-004/#217 governed durable current-head checkpoint.

Queued/unexecuted parent workflows remain queued/unexecuted; this child cannot promote them by implication.

## Remaining production blocker

QUAL-EVID-005/#216 is still required before claiming whole-volume rollback resistance for the checkpoint lineage.
