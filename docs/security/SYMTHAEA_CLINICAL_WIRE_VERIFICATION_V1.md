# Symthaea Clinical Wire Verification V1

## Status

Draft independent verifier for the Symthaea `ClinicalInferenceEnvelopeV1` wire contract.

This layer proves only structural and wire-format compatibility. It is not an admission policy and carries no clinical authority.

## Independence rule

`mycelix-symthaea-clinical-wire` has **no dependency on the Symthaea repository or crate**.

Mycelix independently mirrors the public v1 wire schema and independently implements:

- enum/string mappings;
- schema-version checks;
- field-level structural validation;
- evidence and execution identity invariants;
- calibration/distribution invariants;
- canonical JSON verification;
- BLAKE3 derive-key framing for the v1 wire identity.

A git revision, Rust dependency, or successful shared-type deserialization is therefore not being used as the cross-repository trust boundary.

## Canonical representation

Accepted bytes must:

1. deserialize under Mycelix's strict v1 mirror (`deny_unknown_fields`);
2. satisfy independent structural validation;
3. serialize back to the exact same byte sequence.

This rejects:

- unknown fields;
- whitespace variants;
- reordered keys;
- alternate textual representations;
- malformed or semantically incomplete v1 structures.

Canonicality is deliberately byte-level because the wire digest is intended to identify the exact admitted artifact, not an implementation-dependent interpretation of it.

## Shared conformance vector

Both repositories contain the same compact canonical file:

`fixtures/clinical_inference_wire_v1.json`

Each repository's own test suite must parse and reserialize that exact byte vector independently.

The vector is a compatibility witness, not clinical evidence. Changing the v1 schema, enum encoding, field ordering, JSON encoding, or digest framing must fail cross-repository conformance unless a deliberately versioned migration is introduced.

## Wire digest

Mycelix reproduces Symthaea's v1 framing exactly:

- BLAKE3 derive-key context: `symthaea.clinical.inference-envelope-wire.v1`;
- wire identity version `1` as big-endian `u16`;
- schema-tag length as big-endian `u16`;
- schema tag `symthaea/clinical-inference-envelope/v1`;
- canonical byte length as big-endian `u64`;
- exact canonical bytes.

The result is kept as a Symthaea-specific wire digest type rather than silently being converted into a Mycelix clinical-authority digest.

## Deliberate non-equivalences

```text
valid Symthaea wire bytes
    != accepted Symthaea deployment lineage
    != trusted model execution
    != in-distribution operation
    != clinically qualified evidence
    != clinician-presentation authority
```

The next layer must be an explicit Mycelix admission policy that independently pins accepted engine/model/schema/intended-use/evidence constraints and sets a qualification ceiling.

## Non-claims

This verifier does not establish:

- authenticity of the producer;
- scientific correctness;
- model calibration or generalization;
- detector validity;
- patient applicability;
- clinical effectiveness;
- regulatory clearance;
- diagnosis/treatment authority;
- permission to present an inference to a clinician or patient.
