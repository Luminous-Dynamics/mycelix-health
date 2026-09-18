# ClinicalFact Snapshot Identity v1

## Purpose

`mycelix-clinical-fact-snapshot` defines a serializer-independent identity for one exact, machine-actionable `ClinicalFact` snapshot.

The problem it solves is narrow but important: a clinical evidence binding should not depend on JSON whitespace, object-key order, serializer implementation details, or a caller-provided digest convention.

## Preconditions

A fact receives a snapshot identity only after `ClinicalFact::validate_machine_actionable()` succeeds.

Therefore v1 refuses, among other invalid states:

- missing fact IDs;
- invalid subjects;
- uncoded machine-actionable concepts;
- narrative-only values;
- non-finite numerical values;
- non-UCUM quantitative values;
- invalid ranges or ratios;
- incomplete provenance;
- invalid uncertainty values.

## Canonical framing

The v1 framing is explicit and independent of serde:

- integers: big-endian bytes;
- floating-point values: exact IEEE-754 `f64::to_bits()` bytes;
- strings and byte slices: `u32` length prefix + bytes;
- vectors: `u32` item count, preserving item order;
- options: explicit `0` / `1` presence byte;
- enums: fixed one-byte discriminants;
- snapshot schema version and domain tag are framed explicitly.

The top-level v1 field order is frozen as:

1. snapshot version;
2. snapshot domain tag;
3. `fact_id`;
4. subject;
5. concept;
6. clinical value;
7. effective time;
8. provenance;
9. uncertainty.

Nested object field order follows the Rust domain object order used by the v1 encoder and is part of the framing contract.

### ClinicalValue discriminants

| Tag | Variant |
| ---: | --- |
| `0` | `Quantity` |
| `1` | `CodeableConcept` |
| `2` | `Range` |
| `3` | `Ratio` |
| `4` | `Boolean` |
| `5` | `Integer` |
| `6` | `Decimal` |
| `7` | `DateTimeMicros` |
| `8` | `Reference` |
| `9` | `Narrative` (reserved; current machine-actionable validation rejects it before snapshotting) |

Changing field order, option/vector framing, numeric representation, enum tags, or semantic interpretation requires a new snapshot version/domain. Existing v1 meanings must not be silently changed.

## Bound fields

The identity binds the complete validated fact state, including:

- `fact_id`;
- subject resource type and ID;
- every coding, display/version value and concept text;
- exact clinical value and units;
- effective time;
- source system/resource/type/version;
- recorded time and asserter;
- transformation software/version/operation/input-fact lineage;
- uncertainty/confidence/interpretation.

Vector order is preserved intentionally. Reordering codings or transformation input IDs therefore changes the exact snapshot identity even if a higher-level consumer considers those collections semantically equivalent.

## Digest

The canonical bytes are hashed with BLAKE3 derive-key domain separation under:

`mycelix.health.clinical-fact-snapshot.v1`

The resulting `ClinicalFactSnapshotDigestV1` is intentionally a dedicated type. It is not interchangeable with generic clinical-artifact, FHIR-resource, Symthaea-wire, or other digest domains.

## Intended use in Symthaea interoperability

A future Symthaea evidence crosswalk should bind a typed Symthaea evidence identity to an exact `ClinicalFactSnapshotDigestV1` only after independently establishing that the referenced evidence is this exact validated Mycelix fact.

Do **not** map a raw Symthaea v1 evidence digest directly to a Mycelix fact merely because the 32-byte values happen to match.

## Non-claims

A valid snapshot identity proves exact artifact identity only.

It does not prove:

- the underlying observation is true;
- the source system is trustworthy;
- the fact is current;
- the fact is clinically sufficient;
- a causal relationship;
- model validity;
- patient applicability;
- clinician-presentation authority;
- treatment authority or regulatory clearance.
