# Clinical Population Accountant Canonical Index v1 — Qualification Checklist

This checklist defines required evidence. It is not qualification evidence itself.

## Build/package

- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] workspace builds index integrity/coordinator and canonical-state crate
- [ ] DNA packages both accountant-index zomes
- [ ] exact-head CI completes successfully

## Deterministic anchor

- [ ] same release-policy/accountant-instance/accountant-method tuple -> same anchor
- [ ] changing any tuple member -> different anchor
- [ ] non-release-policy digest kind rejects
- [ ] zero/malformed accountant digests reject
- [ ] synthetic anchor entry itself cannot be committed

## Canonical admission

- [ ] admission target must be ActionHash to valid trusted accountant state
- [ ] base must equal target state's deterministic lineage anchor
- [ ] admission author must equal target state author
- [ ] unrelated third-party admission rejects
- [ ] cross-lineage admission rejects
- [ ] admission-link deletion rejects

## Nested materialization

- [ ] admission membership stable across outer A/B reads
- [ ] every state's correction membership stable across A/B reads
- [ ] every correction set is checked once more after outer membership closure
- [ ] state arriving mid-read causes retry/failure
- [ ] correction arriving mid-read causes retry/failure
- [ ] missing admitted-state target fails closed
- [ ] missing correction target fails closed
- [ ] cross-lineage state/correction fails closed
- [ ] state/correction fan-out bounds execute

## Canonical adapter

- [ ] only nested canonical snapshot is accepted by high-assurance API
- [ ] state/correction count drift rejects
- [ ] duplicate state/correction actions reject
- [ ] invalid observation window rejects
- [ ] cross-lineage state rejects
- [ ] two admitted genesis states -> conflict
- [ ] sibling successors -> conflict
- [ ] missing predecessor -> fail closed
- [ ] one clean chain -> `ObservedCurrentWithinBoundedCanonicalRead`
- [ ] no output type exposes an unqualified global `Current`

## Privacy

- [ ] index key contains no query/output/patient/cohort/private receipt identifier
- [ ] public keyed receipt commitments stay only on the underlying trusted state
- [ ] no cross-key commitment correlator is introduced by indexing

## Runtime / adversarial

- [ ] conductor test: omitted unadmitted state does not enter canonical result
- [ ] conductor test: once conflicting state is admitted and propagated it becomes visible conflict
- [ ] conductor test: outer membership race fails/retries
- [ ] conductor test: inner correction race fails/retries
- [ ] conductor test: third-party admission fails integrity validation
- [ ] conductor test: link deletion fails integrity validation

## Remaining promotion conditions

- [ ] P0 #72 exact cross-zome source-entry-definition proof resolved or explicitly compensated with conductor evidence
- [ ] protected exact receipt -> keyed commitment binding is verifier-owned and tested
- [ ] interactive population release consumes the bounded canonical trusted accountant result, not caller-provided accountant receipts

Do not close P0 #92 solely from source review. Close it only after the exact-head runtime/conductor evidence above exists.