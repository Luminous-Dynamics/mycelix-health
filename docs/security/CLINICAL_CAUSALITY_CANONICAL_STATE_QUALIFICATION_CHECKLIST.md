# Clinical causality canonical state v1 — qualification checklist

This checklist is test/review design, not runtime evidence.

## Build / lint

- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] workspace build/test passes on the exact PR head
- [ ] canonical-state crate tests pass on the exact PR head

## Structural validation

- [ ] zero opaque commitment fails closed
- [ ] deterministic anchor mismatch fails closed
- [ ] unsupported outer read boundary fails closed
- [ ] invalid outer observation window fails closed
- [ ] publication-count mismatch fails closed
- [ ] per-publication correction-count mismatch fails closed
- [ ] total correction-count mismatch/overflow fails closed
- [ ] duplicate publication action fails closed
- [ ] duplicate correction action fails closed
- [ ] cross-commitment publication fails closed
- [ ] cross-commitment correction fails closed
- [ ] cross-publication correction fails closed
- [ ] unsupported correction read boundary fails closed
- [ ] correction observation window outside outer window fails closed

## State semantics

- [ ] empty canonical snapshot returns `NoCanonicalPublicationObserved`
- [ ] lower-level `Current` is exposed only as `ObservedCurrentWithinBoundedRead`
- [ ] conflicting canonical projections remain `ProjectionConflictWithinBoundedRead`
- [ ] typed invalidation/supersession/review/suppression semantics are preserved
- [ ] timestamp/order never selects a winner from conflicting projections
- [ ] only index-admitted publications supplied by the nested snapshot reach the intended high-assurance path

## Runtime provenance / eventual consistency

- [ ] conductor-level tests prove the trusted adapter consumes the runtime materializer output rather than arbitrary deserialized snapshots
- [ ] fabricated structurally valid snapshot is not treated as proof of DHT provenance
- [ ] delayed/unpropagated admission/correction never upgrades the result to global-current truth
- [ ] all user-facing/API names preserve the bounded observation distinction

## Security / privacy

- [ ] no patient, medication, symptom, event, assessor, or stable cross-key identifier is introduced
- [ ] commitment-key rotation remains intentionally unlinkable absent a separately reviewed protected migration proof
- [ ] P0 #72 exact source-entry-definition validation remains a promotion blocker where applicable
- [ ] P0 #83 bounded completeness/read-set semantics remain explicit

## Promotion rule

Do not promote beyond experimental until every applicable item above has executable exact-head evidence. Source review, test design, queued CI, or a mergeable PR is not qualification evidence.
