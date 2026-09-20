# Qualification Lineage Checkpoint V1 — Source Status

**SOURCE-STAGED / UNEXECUTED / CI REQUESTED / NO PASS CLAIM.**

This branch source-stages QUAL-EVID-004 (#215) as a stacked child of the real governed Ed25519 qualification/profile work in #214.

No local executable PASS is claimed. Qualification requires the exact-head GitHub Actions workflow to execute successfully.

A future green workflow would establish only the tested reference theorem:

- governed Ed25519 checkpoint signatures and quorum verification;
- deterministic lineage-state/fork reconstruction;
- exact #214 profile-match commitment binding;
- strict authority-bearing schema rejection;
- local durable checkpoint epoch/predecessor/nonce/time semantics;
- historical signed-bundle re-verification after restart;
- fail-closed fork/inactive/expired/tampered state handling;
- emission of a non-product-authority verified-head artifact for the latest clean checkpoint.

It would **not** establish:

- truth of the underlying qualified theorem;
- successful execution/qualification of every parent Rust workflow unless separately demonstrated;
- production trusted-time correctness;
- production key custody;
- PQ signatures;
- distributed/global consensus;
- fork resolution;
- whole-volume rollback resistance;
- #212 product-seam conversion;
- Patient-v2 cutover/activation;
- legal/regulatory compliance.

Whole-volume rollback resistance is explicitly tracked by QUAL-EVID-005 (#216).
