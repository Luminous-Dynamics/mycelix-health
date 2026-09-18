# FHIR R4 Clinical Promotion Profile v1

Status: **strict experimental promotion profile**. This is not a general-purpose FHIR validator and does not claim clinical validation or regulatory compliance.

## Purpose

FHIR validity and clinical machine-actionability are different questions.

`mycelix-fhir-conformance` sits before the strict Observation projection layer and answers a narrower question:

> Is this Bundle sufficiently unambiguous, final, typed, and attributable for the subset of clinical facts Mycelix-Health v1 currently knows how to promote safely?

Resources rejected from this boundary are not necessarily invalid FHIR. They may need a different workflow, more context, reconciliation, or a richer semantic adapter.

## Bundle profile

v1 accepts `collection` and `searchset` Bundles only. Transaction, batch, history, document, and message Bundles are rejected until their workflow-specific semantics have dedicated handlers.

The profile requires exactly one Patient with a non-empty logical id. Multiple Patient resources are treated as bundle-level ambiguity and fail closed.

Logical `(resourceType, id)` identities must be unique within the promotion input. Although FHIR can represent multiple versions in some Bundle scenarios, the v1 single-patient promotion path does not attempt to infer which version an unversioned clinical reference intended.

FHIR R4 Bundle identity invariants are also checked where relevant:

- version-specific `fullUrl` values are rejected (`bdl-8`);
- duplicate `fullUrl + meta.versionId` identities are rejected in the supported non-history Bundle types (`bdl-7`).

## Observation status policy

FHIR R4 defines multiple valid Observation workflow states. v1 promotes only:

- `final`
- `amended`
- `corrected`

The following remain outside the machine-actionable v1 boundary:

- `registered`
- `preliminary`
- `cancelled`
- `entered-in-error`
- `unknown`

This is a **promotion policy**, not a statement that those statuses are invalid FHIR. Preliminary data can be useful in a dedicated provisional-results workflow. `entered-in-error` especially must not silently survive into a current clinical fact view.

A later design should preserve source Observation status in the semantic fact model and support status-aware supersession/retraction lineage rather than relying only on filtering.

## Clinical time policy

v1 requires `Observation.effectiveDateTime`.

`Observation.issued` is not substituted for the clinical effective time. It describes availability/issuance timing, which belongs to provenance rather than the time the clinical measurement or event actually applies to.

Other valid FHIR effective forms (`effectivePeriod`, `effectiveTiming`, `effectiveInstant`) are explicitly rejected by v1 until they receive typed semantic support. They must not be coerced into a timestamp or string merely to pass the boundary.

## Resource-local vs bundle-fatal failures

The profile distinguishes two failure scopes:

**Bundle-fatal** ambiguity means no fact may promote, for example:

- missing/unsupported Bundle semantics;
- no Patient or multiple Patients;
- duplicate logical resource identity;
- invalid `fullUrl` identity invariants.

**Resource-rejected** failures quarantine one Observation while allowing independent safe Observations to proceed, for example:

- non-promotable status;
- missing effective time;
- cross-patient subject mismatch;
- unresolved subject reference;
- narrative-only value;
- missing UCUM code;
- uncoded clinical concept.

This distinction prevents one malformed measurement from destroying an otherwise usable import while still failing closed when attribution itself is ambiguous.

## Next required work

1. Persist rejected resources in a quarantine/reconciliation ledger rather than only returning issues.
2. Resolve Bundle `fullUrl`, `urn:uuid`, and contained references without guessing.
3. Carry Observation status and amendment/retraction lineage into the clinical semantic model.
4. Support the remaining FHIR `effective[x]` forms as typed temporal semantics.
5. Make the existing Holochain FHIR bridge call this conformance gate before persistence.
6. Add a fixture corpus derived from real EHR exports and official FHIR examples, with PHI removed or synthetic from inception.

The long-term invariant is simple: **parse permissively, promote conservatively, preserve rejected evidence, and make reconciliation explicit.**
