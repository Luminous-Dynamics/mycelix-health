# Clinical Evidence Capsule v2

## Purpose

`mycelix-clinical-evidence-v2` preserves semantic artifact identity at the clinical evidence boundary.

The v1 evidence capsule remains useful and unchanged, but its `FactEvidence.fact_digest` is a generic `ContentDigest { algorithm, value }`. That cannot prove which canonicalization/domain produced a digest. For model-backed clinical inference, this is too weak: an identical BLAKE3 byte string must not be interpreted interchangeably as a ClinicalFact snapshot, FHIR resource, Symthaea wire artifact, model-calibration artifact, or another representation.

v2 is additive. It wraps a fully validated v1 `ClinicalEvidenceCapsule` with a strict typed fact-evidence layer.

## Typed evidence identity

`TypedEvidenceIdentityV1` contains:

- explicit identity schema version;
- canonicalization/digest namespace;
- artifact ID;
- typed digest algorithm;
- fixed-width digest bytes.

The namespace is part of identity.

For exact Mycelix clinical facts, v2 freezes:

`mycelix/clinical-fact-snapshot/v1`

and requires `artifact_id == fact_id`.

## Exactly one fact-identity path

A v2 capsule rejects any wrapped v1 `FactEvidence.fact_digest` that is present.

This is deliberate. Carrying both:

- a generic legacy digest, and
- a typed v2 identity

would create two potentially conflicting identities for one fact and permit downgrade/ambiguity.

The v1 fact entry therefore supplies only `fact_id` + evidence role. v2 supplies the exact typed artifact identity.

## Complete coverage

`typed_fact_evidence` must cover `core.fact_evidence` exactly:

- same number of entries;
- every core fact ID has exactly one typed entry;
- no duplicate typed fact IDs;
- fact ID equals typed artifact ID;
- evidence role is identical;
- typed ClinicalFact evidence uses only the ClinicalFact snapshot namespace;
- zero digests are rejected.

## Relation to Symthaea

The Symthaea ClinicalFact binding tranche (#118) independently verifies that a producer-declared `mycelix/clinical-fact-snapshot/v1` identity corresponds to an actual locally validated Mycelix `ClinicalFact` and locally recomputed `ClinicalFactSnapshotDigestV1`.

This v2 capsule preserves that typed identity downstream. It does not itself prove that a producer claim was verified by #118; a separate adapter/receipt-composition layer must consume the non-serializable verification result rather than constructing identities from untrusted producer bytes alone.

## Promotion containment

`ClinicalEvidenceCapsuleV2` deliberately does not duplicate or expose a `promotion_state()` method.

A validated typed capsule proves representation integrity only. Clinical qualification, OOD trust, evaluator currentness, practitioner authorization and presentation permission remain separate proof lines.

## Compatibility

Existing v1 callers remain unchanged. v2 is intended for stricter model-backed/evidence-sensitive paths.

No automatic conversion from arbitrary v1 fact digests exists.

## Non-claims

A valid v2 capsule does not establish:

- source truth;
- source trust;
- recency;
- scientific validity;
- causal validity;
- model calibration or effectiveness;
- in-distribution operation;
- regulatory clearance;
- clinician-presentation authority;
- diagnosis or treatment authority.
