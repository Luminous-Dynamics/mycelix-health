# Clinical Causality Trial Adapter v1

Status: migration-only / experimental.

## Purpose

This adapter preserves the exact legacy `trials::AdverseEvent::causality` value and source action context without treating the old label as causal evidence.

Historical trial records may remain readable and auditable while the high-assurance path requires a fresh evidence-first assessment.

## Safety invariant

`legacy causality label != qualified causal conclusion`

There is intentionally no API in this crate that maps:

- `DefinitelyRelated`;
- `ProbablyRelated`;
- `PossiblyRelated`;
- `UnlikelyRelated`;
- `NotRelated`;
- `Unknown`

into `CausalConclusionV1`.

All six values are preserved verbatim as `ExternalCausalityScaleLabelV1` under:

- system: `urn:mycelix-health:trials:legacy-causality`;
- version: `1`.

The migration state is always `RequiresFreshEvidenceAssessment`.

## Source lineage

The migration object preserves the exact source action hash, trial hash, participant hash, event ID, and event term. It is serializable for audit/export but intentionally not deserializable as a trusted construction path.

The caller is still responsible for obtaining the legacy event through a trusted/DHT-valid read path. Merely constructing or serializing this import object does not prove the source action exists or belongs to the exact legacy adverse-event entry definition.

## Fresh assessment

A qualified replacement assessment must independently establish:

1. typed observed clinical facts;
2. patient-subject binding;
3. exact exposure/administration occurrence lineage;
4. temporal association;
5. required alternative/concomitant reviews;
6. supporting/challenging evidence;
7. assessment method/version;
8. assessor authority and evidence-source trust at later runtime boundaries.

The preserved legacy label may be attached only as external/source metadata.

## Non-claims

- no automatic causal reinterpretation;
- no proof of legacy event DHT validity by this pure adapter;
- no regulatory-reporting interpretation;
- no clinical qualification;
- no population-level signal inference.
