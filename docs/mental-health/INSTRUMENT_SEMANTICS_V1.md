# Mental Health Instrument Semantics v1

Status: draft foundation for MH-001

## Purpose

Define a fail-closed contract for mental-health screening instruments before score interpretation is allowed to influence any downstream workflow.

This document deliberately separates a screening result from diagnosis, treatment authority, crisis authority, and clinical truth.

## Core non-equivalences

- instrument response != validated score
- validated score != diagnosis
- score band != disorder severity
- positive screen != diagnosis
- crisis indicator != crisis determination
- scorer implementation != evidence that a cutoff is appropriate for every population
- instrument registration != permission to use the instrument in every setting or jurisdiction

## Instrument identity

Every scored instrument MUST bind an exact `InstrumentSpecRef` containing at least:

- namespace;
- instrument identifier;
- instrument/version or revision;
- scoring revision;
- intended-use class;
- population/applicability metadata;
- provenance/source reference;
- usage/licensing policy reference when applicable.

An implementation MUST NOT infer any of these fields from an enum variant alone.

## Fail-closed scoring

A response set may be scored only when:

1. an exact registered instrument specification exists;
2. the response schema matches that specification;
3. the scoring algorithm is explicitly registered for that exact specification;
4. required items are present or the specification defines how missingness is handled;
5. the intended use and population are permitted by the local policy;
6. any required usage/licensing conditions are satisfied.

Otherwise the disposition is explicitly `Unscored` / `Unsupported` / `Incomplete`; no generic severity must be fabricated.

## Required output separation

A scored result should distinguish at least:

- raw/derived score;
- score interpretation/band when the exact specification defines one;
- screening disposition such as positive/negative/indeterminate when the exact specification supports it;
- missingness;
- scorer/spec identity;
- provenance;
- evaluation time;
- optional crisis/safety signal evidence as a separate field or artifact.

The data model should not represent a screening score as a diagnosis.

## Custom instruments

`Custom` instruments are never eligible for generic score interpretation.

A custom instrument must first be registered as a versioned specification with an explicit scoring contract. Until then, Mycelix may preserve responses as data but must not assign a standardized severity or positive/negative screening disposition.

## Crisis / suicide instruments

Crisis or suicide screening is not a generic severity scale. Any safety signal must preserve the exact instrument/version, responses/evidence used, applicable triage policy, and its own authority boundary.

A safety signal by itself must not mint diagnosis, treatment, emergency disclosure, or contact authority.

## Migration rule

The existing generic fallback in `interpret_score(...)` must be removed before unsupported or custom instruments are exposed as scored clinical results.

Existing stored records should not be silently reinterpreted under new scoring semantics. If historical records are migrated, retain their original scorer/version semantics and provenance.

## Qualification targets

MH-001 should include tests proving at least:

- unsupported instrument => no generic severity;
- custom instrument => no generic severity without registered spec;
- scorer version substitution changes identity;
- population/intended-use substitution fails closed when policy requires exact applicability;
- missing required responses do not receive a fabricated score;
- screening disposition cannot construct a diagnosis object;
- crisis/safety evidence remains distinct from generic score interpretation.

## Evidence boundary

A successful software qualification establishes only the behavior of the scoring/identity contract. It does not establish clinical validity, diagnostic accuracy, treatment effectiveness, crisis-prediction accuracy, regulatory status, or appropriateness for a specific patient population.
