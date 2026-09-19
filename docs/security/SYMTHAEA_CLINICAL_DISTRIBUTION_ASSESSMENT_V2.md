# Symthaea Clinical Distribution Assessment V2

## Purpose

This contract closes the cross-inference trust gap between the Symthaea v2 model-assertion line and Mycelix distribution/OOD assurance.

The legacy `ClinicalDistributionAssessmentV1` is bound to a serialized `ClinicalEvidenceCapsule`. That contract remains valid for its existing users, but it is not the correct binding object for a probabilistic Symthaea v2 model assertion because the newer semantic line deliberately avoids manufacturing a four-state `EvaluationState` from a probability.

`mycelix-symthaea-clinical-distribution-assessment-v2` therefore binds distribution evidence directly to `VerifiedSymthaeaModelAssertionV1`.

## Exact assertion lineage

Every assessment carries all of:

- exact model-assertion digest;
- exact verified Symthaea v2 wire digest;
- exact verified evidence-context digest;
- exact Symthaea deployment-admission-policy digest;
- exact subject identity;
- serializer-independent exact model-identity digest.

The redundancy is intentional. The assertion digest already commits to upstream identities, but retaining each identity independently lets downstream trust composition detect substitution directly rather than depending on hidden reconstruction.

## Distribution evidence

The assessment additionally binds:

- exact detector/evaluator artifact identity;
- reference population;
- typed reference-domain identity;
- explicit status: `NotRun`, `Unavailable`, `Indeterminate`, `InDistribution`, or `OutOfDistribution`;
- optional numerical detector boundary;
- exact detector/output evidence when a detector produced a result;
- assessment time.

`Indeterminate`, `InDistribution`, and `OutOfDistribution` require exact assessment evidence.

When a numeric boundary is supplied for `InDistribution` or `OutOfDistribution`, the numerical relation must agree with the declared status.

## Distribution policy

`SymthaeaClinicalDistributionPolicyV2` independently pins:

- detector/evaluator artifact;
- reference population;
- typed reference domain;
- maximum assessment age;
- maximum allowed future skew.

Its identity is binary framed and domain separated.

## Serializer-independent identities

JSON is permitted as a transport representation for the serializable assessment/policy structs, but JSON bytes are not evidence identity.

Model, policy, and assessment identities use explicit binary framing and domain-separated BLAKE3 contexts.

Consequently serializer formatting, map ordering, or whitespace cannot redefine the proof identity.

## Validated structural proof

Successful validation returns non-serializable `ValidatedSymthaeaClinicalDistributionAssessmentV2` retaining the exact assertion/wire/context/admission/model/detector/reference/status/policy identities.

This is only structural OOD evidence.

It does **not** establish:

- that the evaluator is trusted by the deployment;
- that evaluator trust is current;
- global absence of revocation;
- model correctness;
- calibration adequacy;
- threshold/decision-rule validity;
- clinical utility;
- diagnosis or treatment authority;
- patient-alert authority;
- clinician-presentation authority;
- regulatory status.

## Required next trust layer

The runtime evaluator-trust line must be adapted to consume this exact structural proof. A stronger receipt must require the exact detector and structural policy plus the already-separated runtime guarantees:

1. deployment evaluator admission;
2. bounded known-revocation observation;
3. short-lived positive evaluator lease/currentness;
4. exact admission-evidence agreement.

Only a receipt binding that full chain may later be composed with the decision-rule result into a shadow-clinical artifact.

## Relationship to P0 #136

This contract implements the structural binding half of #136. The P0 remains open until the runtime evaluator-trust/currentness line is bound to this assessment and exact-head qualification evidence exists.
