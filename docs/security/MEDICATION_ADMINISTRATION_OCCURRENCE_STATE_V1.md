# Medication Administration Occurrence State v1

Status: **experimental / qualification-required**.

This reducer is the read-side migration for P0 #67. It stops using caller-selected `administration_id` as the canonical clinical conflict key.

## Input trust boundary

The reducer accepts complete-for-purpose sets of:

- already DHT-valid qualified administration publications;
- already DHT-valid administration corrections;
- already DHT-valid occurrence-binding publications;
- already DHT-valid occurrence-binding corrections.

It does not query the DHT and cannot prove that omitted records do not exist.

## Two-stage reduction

Stage 1 reuses `mycelix-medication-administration-state` to derive semantic administration receipt lineages and their correction effects.

Stage 2 reduces occurrence-binding lineages and maps only current semantic administration receipts with current admitted bindings into computable clinical occurrence buckets.

## No fallback identity

A current receipt has only three safe outcomes:

1. exactly one current occurrence digest -> place the receipt in that occurrence;
2. no current admitted occurrence digest -> `Unbound`;
3. more than one current occurrence digest -> `BindingConflict`.

The reducer never falls back to:

- `administration_id`;
- action timestamp;
- recording order;
- latest-write-wins;
- schedule-plan resolver metadata.

## Occurrence conflict semantics

Multiple current semantic administration receipts under the same computable `MedicationAdministrationOccurrence` digest produce `Conflict`.

This remains true even if their human `administration_id` values differ.

Different occurrence-binding evidence/provenance that resolves the same semantic administration receipt to the same occurrence digest is not a clinical conflict.

One current semantic receipt bound to two different occurrence digests is a binding conflict and is counted in neither occurrence until corrected.

## Correction semantics

For both administration publications and occurrence bindings:

- `DuplicateDocumentation` is publication-scoped;
- semantic correction invalidates the semantic lineage;
- explicit corrections may resolve conflict;
- timestamps never select a winner.

A binding attached only to an administration publication that has itself been duplicate-suppressed does not establish the current receipt's occurrence mapping.

## Audit preservation

The returned view retains:

- full administration receipt/correction lineages;
- full occurrence-binding/correction lineages;
- occurrence-level current/conflict state;
- unresolved current receipts.

No evidence has to disappear merely because it no longer contributes to current state.

## Non-claims

This reducer does not prove:

- global DHT query completeness;
- physical administration;
- schedule-resolver trust;
- patient identity beyond the upstream evidence lineage;
- causal relation to downstream outcomes.

## Additional source-type gate

`must_get_valid_record` proves that a referenced record is DHT-valid, and Holochain actions expose their app entry type. Before clinical promotion, the occurrence-attestation zome should additionally prove the referenced source action belongs to the exact expected `QualifiedMedicationAdministration` entry definition rather than relying only on structural deserialization.

Do not hard-code a guessed zome index. Resolve/test the expected scoped entry type against the packaged DNA and conductor.

## Qualification gates

Before promotion require:

- different human IDs + same occurrence -> occurrence conflict;
- no admitted binding -> `Unbound`;
- one receipt + two occurrence digests -> `BindingConflict`;
- multiple binding provenance lineages + same occurrence -> not a clinical conflict;
- semantic occurrence-binding correction resolves binding conflict without last-write-wins;
- duplicate-suppressed source publication does not establish current mapping;
- orphan binding/correction read-set closure failure;
- exact source-zome/entry-definition conductor test;
- incomplete-query tests;
- deterministic output ordering;
- exact-head build/test/fmt/clippy;
- WASM + modified-coordinator adversarial qualification for the admission zome.
