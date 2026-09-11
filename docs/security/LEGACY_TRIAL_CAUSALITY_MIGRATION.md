# Legacy trial causality migration

P0 migration note for the legacy `trials::AdverseEvent` model.

The current record carries one `Causality` enum directly on the adverse-event entry, while its integrity validation only enforces basic event/seriousness shape. The high-assurance path must not treat that enum as self-proving causal evidence.

## Required migration

- Preserve existing legacy adverse-event entries as historical reports.
- Adapt the original causality value only as an `ExternalCausalityScaleLabelV1` with exact source/version provenance.
- Never map `DefinitelyRelated`, `ProbablyRelated`, or `PossiblyRelated` directly to a stronger internal causal conclusion.
- Build a fresh `ObservedClinicalEventV1` + `ExposureAssociationV1` + `CausalAssessmentV1` from independently verified evidence.
- Keep reportability of the adverse event independent from causal certainty.
- Do not rewrite historical reports in place.

A later PR should add the adapter and adversarial migration tests before the legacy `causality` field is used by any qualified downstream workflow.
