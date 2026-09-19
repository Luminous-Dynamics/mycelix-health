# Clinical causality attestation state v1 qualification checklist

Status: source-complete design; not promotion evidence.

## Required source properties

- [x] Group public lineage by opaque qualified-receipt commitment, never patient/event/drug identifiers.
- [x] Treat identical projections as idempotent publications.
- [x] Preserve same-lineage policy/conclusion disagreement as `ProjectionConflict`.
- [x] `DuplicateAttestation` suppresses only the targeted publication.
- [x] `EnteredInError`, `SourceEvidenceRevoked`, `AssessorAuthorityRevoked`, and `EvidenceTrustRevoked` invalidate semantic lineage.
- [x] `PolicySuperseded` preserves history but removes current status.
- [x] `Other` requires review when no stronger typed invalidation exists.
- [x] Stronger typed invalidation cannot be masked by weaker correction classes.
- [x] Orphan corrections fail closed inside the caller-supplied complete-for-purpose set.
- [x] No last-write-wins causal state.
- [x] No public cross-key receipt correlator is introduced.

## Required executable qualification before promotion

- [ ] Workspace build/test passes on the exact PR head.
- [ ] Reducer unit tests pass on the exact PR head.
- [ ] Adversarial duplicate/conflict/correction-precedence tests pass.
- [ ] Exact-head source preflight/postflight is recorded.
- [ ] Aggregate safety-stack qualification reaches this exact head.
- [ ] Runtime integration demonstrates complete correction materialization for the intended read scope.
- [ ] Key-rotation behavior is exercised and documented as lineage-splitting unless a separately reviewed protected migration proof exists.

## Non-claims

This reducer interprets an explicitly supplied set of already DHT-valid records. It cannot prove that an omitted publication or correction does not exist elsewhere on the DHT. It does not infer population treatment effect, regulatory reportability, or individual clinical truth from network publication alone.
