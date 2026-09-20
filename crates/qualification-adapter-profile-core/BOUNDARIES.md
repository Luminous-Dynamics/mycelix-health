# Qualification adapter profile boundaries

This reference core consumes already-governed qualifications and already-verified lineage-head assertions. Neither input is self-authenticating merely because a Rust constructor exists.

Production integration must independently qualify at least:

1. QUAL-EVID-001 governed receipt verification (#209/#210);
2. exact adapter-profile digest/configuration authenticity;
3. current lineage-head checkpoint authenticity, freshness and durable ordering;
4. trusted time and restart continuity for the seam-consumption clock;
5. deployment-evidence identity;
6. product integration that consumes the typed seam token and exposes no raw-boolean bypass.

`VerifiedLineageHead::from_checkpoint_adapter(..., verified=true)` is a reference/testing seam. Product/UI/zome callers must not be allowed to mint it directly.

`AdapterProfileDigest` is an opaque profile identity, not a hash implementation. Its computation and governance are external qualification theorems.

A model, MATL score, CI status, Git branch, pull-request merge, or UI toggle cannot create a seam token.

A green reference workflow establishes software semantics only. It does not prove the external receipt/profile/checkpoint facts are true.
