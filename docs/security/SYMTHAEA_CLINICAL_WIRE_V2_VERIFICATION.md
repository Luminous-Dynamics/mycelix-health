# Independent Symthaea Clinical Wire v2 Verification

## Purpose

`mycelix-symthaea-clinical-wire-v2` independently verifies the published Symthaea clinical inference binary v2 contract without depending on the Symthaea repository or crate.

This is a trust-boundary property: compatibility must be reproduced from the public framing contract and frozen vector rather than inherited from shared implementation code.

## Shared conformance vector

Symthaea and Mycelix carry the exact same textual hex fixture:

`clinical_inference_wire_v2.hex`

GitHub blob SHA in both repositories:

`d8f359ef04be4d4f12ecdf49427781c5af190701`

The decoded fixture is 1260 bytes.

## Independently implemented checks

The Mycelix verifier independently implements:

- `SYMCLN2\0` magic validation;
- wire version 1;
- envelope schema version 2;
- big-endian integer decoding;
- exact IEEE-754 float decoding;
- fixed enum discriminants;
- option tags 0/1;
- bounded UTF-8 strings;
- bounded vectors;
- truncation/trailing-byte rejection;
- typed evidence identities;
- typed subject-binding evidence;
- typed model training/evaluation/calibration lineage;
- typed inference calibration evidence;
- typed OOD detector evidence;
- typed execution inputs;
- execution-input uniqueness;
- claim evidence/execution binding;
- subject-binding/execution binding;
- calibrated inference/model evidence identity agreement;
- known distribution state requiring detector evidence;
- the published Symthaea BLAKE3 wire-digest framing.

## No shared-code shortcut

The verifier does not import:

- `symthaea-clinical`;
- Symthaea's v2 parser;
- Symthaea's v2 semantic types;
- Symthaea's v1 wire verifier.

Its local mirror types exist only to represent and verify the external contract.

## Digest boundary

`VerifiedSymthaeaWireDigestV2` remains a Symthaea-wire identity.

It must not be reinterpreted as:

- `ClinicalFactSnapshotDigestV1`;
- a Mycelix clinical-artifact digest;
- a FHIR-resource digest;
- an authority/promotion receipt.

Digest domains are not interchangeable merely because each contains 32 bytes.

## Typed evidence and ClinicalFact binding

A v2 evidence identity whose namespace is:

`mycelix/clinical-fact-snapshot/v1`

states which canonicalization/digest contract the producer claims to reference. It does **not** by itself prove that a corresponding validated Mycelix `ClinicalFact` exists.

A later bridge must independently:

1. obtain the exact Mycelix `ClinicalFact`;
2. validate it as machine-actionable;
3. compute `ClinicalFactSnapshotDigestV1` locally;
4. require artifact ID / subject / digest agreement with the v2 evidence identity;
5. only then create `FactEvidence` for a Mycelix capsule.

The namespace is a required type boundary, not a trust assertion.

## Admission remains separate

Successful v2 wire verification does not mean the deployment accepts:

- that engine;
- that model;
- those model lineages;
- that evidence namespace;
- that intended use;
- that patient binding;
- that OOD detector;
- that qualification level.

A separate v2 admission policy must make those decisions.

## Authority remains separate

This verifier cannot mint a `ClinicalPresentationPermit` and does not establish diagnosis, treatment, prescribing, dispensing, administration, regulatory clearance, or autonomous clinical action.
