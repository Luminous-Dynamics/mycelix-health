# Clinical Distribution Runtime Currentness v1

## Purpose

This tranche closes the semantic gap between two independent runtime observations:

1. a bounded admission/revocation observation from `clinical_distribution_trust`; and
2. a bounded positive evaluator lease observation from `clinical_distribution_lease`.

Neither observation is sufficient alone.

`mycelix-clinical-distribution-currentness` composes them into a short-lived, non-serializable `VerifiedDistributionEvaluatorCurrentnessV1` only when they describe the same exact evaluator admission lineage and are both fresh at composition time.

## Core rule

```text
no observed revocation != current evaluator trust
positive lease alone     != current evaluator trust

(no observed revocation observation)
                +
(root-issued positive lease observation)
                +
(exact lineage agreement)
                +
(freshness / temporal proximity)
                ↓
VerifiedDistributionEvaluatorCurrentnessV1
```

The output is an ephemeral runtime proof object, not a durable credential.

## Exact binding

Successful composition requires exact agreement on:

- evaluator admission action hash;
- admission proposal digest;
- detector/evaluator artifact identity;
- structural distribution-policy digest;
- evaluator trust-policy digest;
- external admission-evidence digest;
- stable network-backed read boundaries.

The currentness digest additionally binds:

- selected positive lease action hash;
- currentness policy digest;
- both observation completion timestamps;
- composition timestamp;
- currentness expiry.

Digest domains are serializer-independent and binary framed.

## Revocation semantics

The revocation snapshot must report zero observed revocation targets and its count must agree with its materialized revocation vector.

This does **not** mean global revocation absence has been proven. The currentness proof means only that:

- the qualified observation path saw no revocation in its bounded stable read; and
- an exact root-issued positive lease for the same admission was also current and observed.

## Freshness

`DistributionEvaluatorCurrentnessPolicyV1` sets:

- maximum observation age; and
- maximum difference between the two observation-completion timestamps.

Hard implementation ceilings are:

- observation age <= 60 seconds;
- inter-observation gap <= 30 seconds.

Deployments may require substantially smaller values.

An observation that completes after the supplied composition time is rejected. The trusted adapter should therefore supply composition time from the same qualified runtime clock domain used for the conductor observations.

## Automatic expiry

The proof expires at the earliest of:

- selected positive lease expiry;
- evaluator admission expiry, when bounded;
- revocation-observation freshness expiry;
- lease-observation freshness expiry.

An expired proof cannot become active again.

## Provenance boundary

This crate is intentionally pure Rust. It can prove consistency between supplied observation values, but it cannot independently prove that a caller actually obtained those values from a particular conductor/cell/DNA.

Therefore the function `compose_distribution_evaluator_currentness_v1(...)` is valid for production trust only behind a separately qualified adapter that proves:

- exact conductor/cell/DNA identity;
- exact coordinator/zome function identity;
- exact source app-entry definitions for referenced records;
- no caller substitution of fabricated serialized observation structs.

This limitation is explicit rather than hidden behind the opaque output type.

## Non-serializable output

`VerifiedDistributionEvaluatorCurrentnessV1` intentionally has no serde implementation.

It retains:

- admission action hash;
- selected lease action hash;
- admission proposal digest;
- detector identity;
- distribution and trust policy digests;
- external admission-evidence digest;
- currentness policy digest;
- verification time;
- expiry time;
- domain-separated currentness digest.

It must not be persisted and replayed as a bearer credential.

## Authority containment

This proof does not establish:

- detector scientific validity;
- model effectiveness;
- patient applicability;
- in-distribution status by itself;
- calibrated model validity;
- decision-rule validity;
- practitioner authority;
- regulatory status;
- clinician-presentation authority;
- diagnosis or treatment authority.

The next trust layer must bind this proof into the existing structural distribution assessment and evaluator trust receipt rather than continuing to use admission-only trust for model-backed presentation.