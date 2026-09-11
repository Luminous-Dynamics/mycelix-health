# Clinical Population Interactive Release V1 — Qualification Checklist

Status: **not qualified**

## Pure gate

- [ ] `cargo test -p mycelix-clinical-population-interactive-release` passes on the exact PR head.
- [ ] `cargo clippy -p mycelix-clinical-population-interactive-release --all-targets -- -D warnings` passes.
- [ ] exact canonical before -> after transition authorizes.
- [ ] wrong commitment key fails closed.
- [ ] HMAC-SHA-256 and keyed-BLAKE3 commitment helpers both execute and produce scheme-distinct commitments.
- [ ] zero commitment key fails closed.
- [ ] wrong release-policy lineage fails closed.
- [ ] wrong accountant instance/method lineage fails closed.
- [ ] no canonical state fails closed.
- [ ] multiple admitted genesis states fail as conflict.
- [ ] sibling successor states fail as conflict.
- [ ] review-required/revoked ancestry cannot authorize.
- [ ] missing trusted predecessor fails closed.
- [ ] public/private sequence mismatch fails closed.
- [ ] public/private query-count mismatch fails closed.
- [ ] public/private cumulative privacy-loss mismatch fails closed.
- [ ] after-state predecessor must be the exact trusted before-state action.
- [ ] protected before receipt must match the public keyed commitment.
- [ ] protected after receipt must match the public keyed commitment.
- [ ] commitment scheme change inside one transition fails closed.
- [ ] request query/mechanism/output substitution still fails through the lower-level validator.
- [ ] canonical snapshot older than the software freshness ceiling fails closed.
- [ ] capability consumption after its software release window fails closed.
- [ ] static release requests cannot mint a trusted interactive capability.

## Runtime / conductor

- [ ] #92 canonical-accountant discovery race tests execute successfully.
- [ ] canonical snapshot is materialized with network-backed reads on the exact lineage.
- [ ] admitted sibling fork is visible to the gate after propagation and blocks release.
- [ ] mid-read state/correction arrival forces retry rather than release authority.
- [ ] production release executor supplies a trusted runtime timestamp to capability consumption.
- [ ] release executor has no code path accepting the lower-level generic capability for interactive DP.
- [ ] private receipt commitment key is held outside DHT/DNA/public logs and cannot be read through ordinary app APIs.
- [ ] key rotation/migration behavior is separately qualified before use.

## Privacy / cryptography review

- [ ] commitment preimage framing is independently reviewed for unambiguous domain separation.
- [ ] HMAC-SHA-256 implementation path is reviewed against the selected key-management policy.
- [ ] keyed-BLAKE3 implementation path is reviewed against the selected key-management policy.
- [ ] commitment keys are at least 256 bits of deployment-generated secret material.
- [ ] no query/output/patient/cohort identifiers are added to public accountant-state discovery.
- [ ] logs/errors do not emit commitment keys or full protected accountant receipts.

## Release-authority migration blocker

- [ ] legacy `authorize_population_release()` interactive capability is removed, deprecated, or made impossible for a production interactive executor to consume.
- [ ] documentation consistently calls the lower-level interactive result preflight evidence, not final release authority.
- [ ] only `TrustedInteractivePopulationReleaseCapabilityV1` can reach a production interactive release sink.

## Promotion rule

Do not promote broad interactive DP release until all executable tests above pass on the exact frozen head, #92 conductor evidence exists, the legacy/preflight authority ambiguity is closed, and the exact DP mechanism/accountant implementation used in deployment has its own qualification evidence.
