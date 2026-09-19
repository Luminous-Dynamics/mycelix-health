# Clinical causality commitment index v1 — qualification checklist

This checklist is design/test guidance, not runtime qualification evidence.

## Build / packaging

- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] exact-head workspace build/test succeeds
- [ ] commitment-index integrity/coordinator WASM builds
- [ ] DNA packages both new zomes with correct dependency wiring

## Deterministic anchor

- [ ] same scheme + commitment value yields same anchor hash
- [ ] changed commitment value changes anchor hash
- [ ] changed commitment scheme changes anchor hash
- [ ] zero commitment is rejected
- [ ] committed synthetic anchor entries are rejected

## Admission validation

- [ ] correct attestation author can admit exact attestation under exact commitment anchor
- [ ] third-party admission is rejected
- [ ] wrong commitment anchor is rejected
- [ ] non-EntryHash base is rejected
- [ ] non-ActionHash target is rejected
- [ ] missing/invalid target is rejected
- [ ] structurally malformed causal attestation target is rejected
- [ ] link deletion is rejected
- [ ] exact source-entry-definition proof is covered under P0 #72 before promotion

## Canonical discovery

- [ ] zero-admission snapshot succeeds
- [ ] one admitted attestation materializes
- [ ] duplicate admission links to same target are idempotent
- [ ] multiple admitted attestations for one commitment all materialize
- [ ] changed target set between network reads fails closed
- [ ] unavailable target fails closed
- [ ] cross-commitment target fails closed
- [ ] more than 256 observed links fails closed
- [ ] returned snapshot always carries `NetworkBackedStableDoubleRead`

## Canonical-state integration

- [ ] high-assurance canonical reducer consumes only index-admitted publications
- [ ] unindexed but otherwise valid attestation remains explicitly noncanonical
- [ ] conflicting index-admitted publications for one commitment become conflict, never last-write-wins
- [ ] publication correction materialization remains bounded by #84 semantics
- [ ] read-boundary metadata is preserved end-to-end

## Privacy

- [ ] index exposes no patient/event/drug/symptom/dose/assessor identifiers
- [ ] deterministic base derives only from already-public opaque commitment
- [ ] no stable cross-key correlator is introduced
- [ ] key-rotation continuity requires separate protected migration evidence

## Promotion blockers

Do not promote beyond experimental until:

1. exact-head CI executes successfully;
2. conductor adversarial tests cover admission and discovery;
3. P0 #72 exact source-entry-definition proof is resolved;
4. a canonical-state adapter prevents unindexed publications from entering the high-assurance reducer;
5. P0 #83's bounded completeness contract is preserved end-to-end;
6. privacy review confirms the index does not create a new sensitive correlation surface.
