# Symthaea Clinical Evidence Context v1

## Purpose

This layer composes two independently established proof lines:

1. `AdmittedSymthaeaInferenceV2` — the exact Symthaea v2 inference satisfied one exact Mycelix deployment admission policy; and
2. `VerifiedSymthaeaClinicalFactBindingSetV1` — the producer-declared Mycelix ClinicalFact identities were independently rebound to actual locally validated `ClinicalFact` snapshots.

Neither proof is sufficient alone. This crate produces a non-serializable evidence context only when both refer to the same exact inference and remain mutually consistent.

## Same-wire requirement

The admission proof and fact-binding proof must carry exactly the same `VerifiedSymthaeaWireDigestV2`.

A valid admission receipt from inference A cannot be combined with valid fact bindings from inference B.

## Subject requirement

The admitted inference must have a subject and its subject ID must equal the subject ID proved by the ClinicalFact binding set.

## Fact coverage

For claim evidence in namespace:

`mycelix/clinical-fact-snapshot/v1`

composition requires exact coverage between the admitted inference and the independently verified binding set.

The context rechecks:

- artifact ID;
- `fact_id`;
- evidence role;
- exact ClinicalFact snapshot digest;
- duplicate coverage.

Although these properties were checked by predecessor layers, rechecking them at composition prevents accidental proof substitution and makes the dependency explicit.

## Output

`VerifiedSymthaeaEvidenceContextV1` is intentionally non-serializable.

It binds:

- exact Symthaea wire digest;
- exact admission-policy digest;
- exact ClinicalFact-binding-policy digest;
- Mycelix qualification ceiling;
- subject ID;
- Symthaea claim kind;
- evidence stage;
- intended use;
- exact statement;
- typed ClinicalFact evidence;
- a domain-separated context digest.

The context digest uses:

`mycelix.health.symthaea-clinical-evidence-context.v1`

and binds all proof digests plus subject and typed fact evidence.

## Why non-serializable

The context is a proof-composition capability internal to a trusted process. It should not be reconstructible merely by deserializing attacker-controlled bytes containing matching-looking hashes.

Serializable audit receipts can be introduced separately if they are explicitly treated as evidence records rather than authority-bearing capabilities.

## Next boundary

This context can later feed a strict `ClinicalEvidenceCapsuleV2` projection, but only under an explicit projection policy that binds:

- claim-kind -> assertion-kind mapping;
- subject resource type;
- qualification ceiling;
- intended use;
- statement/semantic mapping.

That projection must not increase authority beyond the admission ceiling.

## Non-claims

A valid evidence context does not establish:

- scientific correctness;
- causal validity;
- independent OOD trust;
- evaluator currentness;
- practitioner authorization;
- regulatory clearance;
- clinician-presentation authority;
- diagnosis or treatment authority.
