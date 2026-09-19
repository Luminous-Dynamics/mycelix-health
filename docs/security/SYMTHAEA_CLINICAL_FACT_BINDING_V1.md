# Symthaea v2 -> Mycelix ClinicalFact Binding v1

## Purpose

This layer verifies one specific typed-evidence namespace:

`mycelix/clinical-fact-snapshot/v1`

A Symthaea v2 inference may claim that an evidence item refers to this namespace. The claim itself is not trusted. Mycelix independently requires the actual `ClinicalFact`, validates it, computes its canonical `ClinicalFactSnapshotDigestV1`, and proves an exact subject/artifact/digest match.

## Verification chain

```text
Symthaea binary v2 bytes
        ↓
independent Mycelix v2 parser
        ↓
typed evidence identity
(namespace + artifact_id + digest)
        ↓
actual Mycelix ClinicalFact
        ↓
ClinicalFact::validate_machine_actionable
        ↓
local ClinicalFactSnapshotDigestV1
        ↓
exact artifact ID + subject + digest equality
        ↓
VerifiedSymthaeaClinicalFactBindingSetV1
```

## Required invariants

For each claim evidence item in `mycelix/clinical-fact-snapshot/v1`:

- `artifact_id` must identify the supplied Mycelix `fact_id`;
- the fact must be supplied exactly once;
- no unreferenced fact inputs are accepted;
- the fact subject must match the inference subject under an explicit subject-namespace crosswalk policy;
- the fact must pass strict machine-actionable validation;
- the locally computed snapshot digest must exactly equal the typed Symthaea evidence digest;
- evidence role is preserved in the verified binding.

At least one Mycelix ClinicalFact evidence reference is required; the verifier does not produce a vacuous success token.

## Subject crosswalk

Subject identifier namespaces are not silently equated.

`SymthaeaFactBindingPolicyV1` explicitly binds an external subject namespace to one Mycelix resource type, for example:

`fhir/Patient -> Patient`

Changing this crosswalk changes the binding-policy digest.

The fact's subject ID must equal the inference subject ID exactly.

## Fail-closed supplied fact set

The caller cannot supply a bag of unrelated clinical records and rely on the adapter to ignore them.

- missing referenced fact -> reject;
- duplicate fact ID input -> reject;
- extra unreferenced fact -> reject;
- subject substitution -> reject;
- snapshot/provenance/value/time/uncertainty substitution -> local snapshot digest mismatch.

## Dedicated output

`VerifiedSymthaeaClinicalFactBindingSetV1` is non-serializable and carries:

- exact verified Symthaea v2 wire digest;
- exact binding-policy digest;
- bound subject ID;
- verified fact bindings with role + exact `ClinicalFactSnapshotDigestV1`.

It is not an admission, clinical-promotion permit, practitioner authority token, or regulatory artifact.

## Why this does not directly emit ClinicalEvidenceCapsule

The existing `FactEvidence.fact_digest` surface is intentionally generic (`ContentDigest`). This binding layer now has a stronger domain-specific snapshot digest type.

Do not weaken the binding by immediately flattening `ClinicalFactSnapshotDigestV1` into a generic digest string. A later capsule adapter should either preserve the typed domain explicitly or evolve the capsule evidence contract before conversion.

## Non-claims

Successful binding proves that the Symthaea evidence claim points to the exact locally validated Mycelix fact snapshot supplied to this operation. It does not prove that the source fact is true, current, clinically sufficient, causally relevant, or authorized for clinical action. It also does not validate the Symthaea model, OOD detector, deployment admission, clinician authority, or treatment decision.
