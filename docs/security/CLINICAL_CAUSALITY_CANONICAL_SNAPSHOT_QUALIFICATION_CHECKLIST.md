# Clinical causality canonical snapshot v1 — qualification checklist

This checklist is test design/review guidance, not runtime evidence.

## Build / lint

- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] exact-head workspace tests pass
- [ ] `clinical_causality_index` WASM/package builds

## Outer index closure

- [ ] zero-admission commitment returns an empty bounded snapshot
- [ ] one admitted publication returns one publication snapshot
- [ ] multiple admitted publications all materialize deterministically
- [ ] new admission between index reads A/B fails closed
- [ ] disappearing/changing observed target between A/B fails closed
- [ ] duplicate links to the same attestation action are idempotent
- [ ] >256 observed index links fails closed

## Inner correction closure

- [ ] zero-correction publication succeeds
- [ ] one/multiple corrections materialize in deterministic action-hash order
- [ ] new correction between inner reads A/B fails closed
- [ ] wrong-attestation correction fails closed
- [ ] cross-commitment correction fails closed
- [ ] unavailable/malformed correction target fails closed
- [ ] >256 observed corrections for one attestation fails closed

## Final closure pass

- [ ] correction arriving after a publication's inner read B but before final closure is detected
- [ ] unchanged correction sets survive final closure
- [ ] total correction count >4,096 fails closed
- [ ] integer overflow in total correction accounting fails closed

## Canonicality

- [ ] only index-admitted publications appear in returned snapshot
- [ ] valid but unindexed attestation is absent from canonical snapshot
- [ ] admission under another commitment is rejected at integrity validation
- [ ] third-party admission is rejected at integrity validation

## Epistemic boundary

- [ ] result always carries `NestedNetworkBackedStableReadsWithFinalClosureChecks`
- [ ] no field/API claims global completeness or global current truth
- [ ] eventual-consistency/race limitations remain explicit
- [ ] downstream reducer preserves the exact bounded-read class

## Promotion blockers

Do not promote beyond experimental until:

1. exact-head CI executes successfully;
2. conductor-level race/adversarial tests exercise outer, inner, and final closure;
3. P0 #72 exact source-entry-definition proof is resolved;
4. canonical reducer accepts this snapshot type and refuses arbitrary/unindexed publication sets;
5. P0 #83 non-upgradable completeness semantics are preserved end-to-end;
6. privacy review confirms no new sensitive correlation surface.
