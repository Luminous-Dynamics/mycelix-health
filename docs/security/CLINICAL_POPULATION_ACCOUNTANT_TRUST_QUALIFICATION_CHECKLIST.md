# Clinical Population Accountant Trust v1 — Qualification Checklist

This checklist is evidence design, not qualification evidence by itself.

## Build / package

- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] workspace build includes `population_accountant_integrity`, `population_accountant`, and `mycelix-clinical-population-accountant-state`
- [ ] DNA packaging includes both accountant zomes
- [ ] exact-head CI completes successfully

## Fail-closed DNA configuration

- [ ] packaged default has `population_accountant.root_authorities: []`
- [ ] empty roots reject verifier authorizations
- [ ] duplicate roots reject configuration
- [ ] authorization duration above 15 minutes rejects
- [ ] malformed/missing accountant DNA properties fail closed

## Exact authorization

- [ ] only DNA-pinned root authors verifier authorization
- [ ] state author must equal authorization grantee
- [ ] state action timestamp must fall inside authorization window
- [ ] authorization binds exact state projection digest
- [ ] policy, accountant instance/method, sequence, predecessor and receipt commitment all match exactly
- [ ] authorization replay against changed state ID/projection fails

## Lineage

- [ ] genesis requires sequence/query count/privacy loss = 0 and no predecessor
- [ ] successor requires exact predecessor action
- [ ] successor cannot cross release policy/accountant instance/accountant method
- [ ] sequence and query count are contiguous
- [ ] cumulative epsilon/delta never decrease
- [ ] successor cannot reuse predecessor receipt commitment
- [ ] missing predecessor fails closed

## Corrections

- [ ] state/update/delete are rejected
- [ ] corrections are root-authored, append-only and exact-target
- [ ] correction link author equals correction author
- [ ] correction cannot cross private receipt commitment
- [ ] `Other` requires rationale commitment

## Reducer

- [ ] single genesis -> observed current within supplied read
- [ ] multiple genesis -> conflict
- [ ] sibling successors -> conflict
- [ ] multiple live leaves -> conflict
- [ ] missing predecessor -> error
- [ ] invalidated/superseded/review-required ancestor prevents descendant current promotion
- [ ] no timestamp/insertion-order winner exists
- [ ] duplicate correction IDs/actions reject
- [ ] mixed accountant lineages reject

## Privacy

- [ ] public state does not expose query specification or released output digest
- [ ] private receipt identity is a nonzero keyed commitment
- [ ] commitment secret is never packaged in DNA/DHT

## Remaining promotion blockers

- [ ] P0 #72 exact cross-zome source-entry-definition proof is resolved or accepted with compensating conductor evidence
- [ ] P0 #92 canonical accountant state discovery/materialization is implemented
- [ ] protected binding from exact `PrivacyAccountantReceiptV1` to public keyed commitment is verifier-owned
- [ ] interactive release consumes a trusted canonical accountant capability rather than caller-supplied receipts
- [ ] conductor tests cover fork, replay, concurrent successors, correction race and stale state

No interactive DP release should be described as trusted/canonical until the remaining blockers are closed.