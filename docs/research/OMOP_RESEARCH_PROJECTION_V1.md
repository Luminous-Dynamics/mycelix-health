# OMOP Research Projection V1

## Purpose

Provide a conservative, provenance-preserving bridge from validated Mycelix clinical facts into OMOP CDM 5.5 research representations.

This is a **secondary-use research/ETL boundary**. It is not a clinical decision-support boundary and it does not grant diagnosis, treatment, presentation, prescribing, dispensing, administration, or autonomous action authority.

## External target

V1 targets OMOP CDM **5.5**.

The OMOP projection is intentionally explicit about vocabulary mapping. Mycelix does not infer an OMOP `concept_id` from a source string, code display, LOINC code, SNOMED code, or UCUM unit by convention.

The deployment/research pipeline must supply an exact mapping policy whose identity includes:

- policy ID;
- OMOP CDM version;
- vocabulary release ID;
- digest of the exact vocabulary snapshot used for mapping;
- exact source coding system/code/version;
- Standard Measurement concept ID;
- source concept ID;
- Measurement Type concept ID;
- explicit UCUM -> OMOP Unit concept bindings.

## V1 scope

V1 projects only a machine-actionable `ClinicalFact` whose value is:

`ClinicalValue::Quantity`

and whose quantity is UCUM-bound.

The result is a research-only representation of an OMOP `MEASUREMENT` event.

V1 deliberately does **not** project:

- conditions;
- drug exposures;
- procedures;
- devices;
- specimens;
- narrative observations;
- categorical observations;
- ranges/ratios as if they were measured scalar results;
- diagnoses or inferred phenotypes;
- cohort membership;
- database primary keys;
- visits or provider joins.

Those require independent domain semantics rather than a generic fallback table.

## Core theorem

A successful projection requires all of:

1. the source `ClinicalFact` passes `validate_machine_actionable()`;
2. the source fact subject exactly matches an explicit OMOP person binding;
3. the source value is a strict UCUM quantity;
4. exactly one policy source-code binding matches one source coding;
5. the coding version matches exactly (`None` does not wildcard a version);
6. exactly one explicit UCUM unit mapping exists;
7. all target concept IDs satisfy the v1 structural constraints;
8. the source code and unit fit the OMOP source-value length boundary;
9. an exact serializer-independent `ClinicalFactSnapshotDigestV1` is recomputed;
10. the exact mapping policy and vocabulary snapshot identities are bound into the projection.

Any missing or ambiguous mapping fails closed.

## Identity

The policy and projected research record both have serializer-independent, domain-separated BLAKE3 identities.

The projection identity binds:

- OMOP contract/CDM version;
- person ID;
- Measurement concept/type/unit IDs;
- exact IEEE-754 numerical value;
- event time;
- source code/source concept ID;
- UCUM source unit;
- original Mycelix fact ID;
- exact ClinicalFact snapshot digest;
- exact projection-policy digest;
- exact vocabulary-snapshot digest;
- exact person-binding namespace/evidence digest.

Changing the clinical value, vocabulary snapshot, mapping policy, patient binding, target concept, unit mapping, or event time therefore changes the projection identity.

## Person binding and privacy

V1 does not derive OMOP `person_id` from a Mycelix patient identifier.

A research boundary supplies:

- the exact Mycelix subject;
- a positive OMOP person ID;
- a binding namespace;
- evidence digest for the person-linkage/pseudonymization process.

This permits a deployment to use a study-specific or privacy-preserving person mapping instead of exporting direct patient identifiers by default.

## Vocabulary discipline

The mapping policy must identify the exact vocabulary snapshot used to resolve OMOP concept IDs.

A mapping that worked against one vocabulary snapshot is not silently treated as equivalent under a later snapshot. The snapshot digest is part of both policy and projection identity.

Likewise, source coding versions are exact. Terminology drift must be reviewed/mapped rather than treated as an implicit upgrade.

## Non-claims

`ResearchProjectionOnly` does not establish:

- that the source fact is clinically true or complete;
- that an OMOP mapping is clinically correct merely because its IDs are structurally valid;
- cohort eligibility;
- causal effect;
- biomarker validity;
- diagnosis;
- prognosis;
- treatment indication;
- medical-device status;
- clinical effectiveness or safety;
- legal/regulatory compliance;
- clinician/patient presentation authority;
- autonomous clinical action.

The research ETL/mapping policy remains responsible for validating vocabulary/domain correctness against the actual OMOP vocabulary snapshot.

## Qualification targets

Before promotion beyond staged research code:

- `cargo fmt --check -p mycelix-clinical-omop-projection`;
- `cargo clippy -p mycelix-clinical-omop-projection --all-targets -- -D warnings`;
- `cargo test -p mycelix-clinical-omop-projection`;
- exact workspace/lock consistency;
- adversarial tests for source coding version drift, duplicate/ambiguous mapping, unit substitution, patient rebinding, unsupported value coercion, policy/vocabulary substitution, and fact mutation;
- independent mapping fixtures derived from a real pinned OMOP vocabulary snapshot;
- external ETL conformance tests against an OMOP CDM 5.5 instance;
- data-quality checks before using projected data for cohort characterization, estimation, or prediction.

## Next research tranches

After V1 is qualified:

1. categorical/general `OBSERVATION` projection with explicit domain/value-concept mapping;
2. CONDITION/DRUG/PROCEDURE adapters from their domain-specific Mycelix records rather than generic ClinicalFact coercion;
3. observation-period construction with explicit completeness semantics;
4. provenance-preserving visit/provider linkage;
5. cohort/phenotype definitions that bind exact OMOP vocabulary + projection identities;
6. target-trial-emulation contracts that bind eligibility, treatment strategies, time zero, outcomes, censoring, confounding assumptions, and estimands;
7. reproducible export manifests and round-trip lineage back to Mycelix evidence.
