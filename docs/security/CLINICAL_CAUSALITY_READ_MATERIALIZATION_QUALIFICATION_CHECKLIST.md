# Clinical causality read materialization v1 — qualification checklist

This checklist is test design and review guidance, not runtime evidence.

## Build / lint

- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] workspace build/test passes on the exact PR head
- [ ] causal coordinator WASM/package build passes

## Stable-double-read behavior

- [ ] zero-correction snapshot succeeds and reports count 0
- [ ] one valid correction materializes and reports count 1
- [ ] duplicate links to the same correction action are idempotent
- [ ] a new correction target appearing between reads fails closed
- [ ] a target disappearing/changing between reads fails closed
- [ ] an unfetchable correction target fails closed
- [ ] a structurally wrong correction target fails closed
- [ ] a correction targeting another attestation fails closed
- [ ] a correction crossing opaque receipt-commitment lineage fails closed
- [ ] non-ActionHash link targets fail closed
- [ ] more than 256 observed correction targets fails closed

## Epistemic boundary

- [ ] returned snapshot always carries `NetworkBackedStableDoubleRead`
- [ ] no API field claims `Complete`, `Global`, or `Authoritative`
- [ ] tests/documentation prove a stable double-read cannot establish absence of an unpropagated correction
- [ ] downstream adapters preserve the read-boundary class rather than stripping it
- [ ] downstream APIs do not rename bounded-read state to generic/global `Current`
- [ ] callers cannot silently upgrade the snapshot into global-current truth

## Adversarial conductor / DHT tests

- [ ] ordinary valid attestation + correction path
- [ ] malicious coordinator cannot create an integrity-invalid correction/link
- [ ] delayed correction propagation is represented as a bounded snapshot, not global absence
- [ ] two different publications sharing one opaque receipt commitment remain discoverability-sensitive under P0 #83
- [ ] exact source-entry-definition validation remains blocked on P0 #72 where applicable

## Promotion blockers

Do not promote this lane beyond experimental until:

1. exact-head CI executes successfully;
2. conductor-level adversarial tests cover the stable-double-read behavior;
3. P0 #72 exact source-entry-definition checks are resolved for cross-zome dependencies;
4. P0 #83 defines the canonical completeness/discovery contract for all publications sharing one commitment;
5. downstream consumers preserve the bounded read classification end-to-end;
6. privacy review confirms no patient/event/drug identifiers or cross-key correlator were introduced.
