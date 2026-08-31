# health-surveillance-core

Pure-Rust evidence contracts for privacy-preserving, aggregate public-health surveillance in Mycelix Health.

## Purpose

This crate defines what later storage, federation, and Symthaea reasoning layers are allowed to treat as public-health surveillance evidence. It is deliberately upstream of outbreak detection and downstream of source-specific aggregation.

The core distinctions are:

- **aggregate evidence is not an individual clinical record**;
- **aggregate contributing-unit count is not necessarily unique-human cardinality**;
- **measurement uncertainty is not source trust**;
- **source independence is not source agreement**;
- **content identity is not authenticity**;
- **release eligibility is not a proof of privacy**;
- **evidence is not a diagnosis, outbreak declaration, or action authorization**.

## v1 evidence envelope

`SurveillanceObservation` records:

- a broad signal family such as respiratory or gastrointestinal;
- aggregate source kind (clinical syndromic, laboratory, wastewater, environmental, absenteeism, or health-system capacity);
- a stable aggregate source instance;
- an explicit `IndependenceGroup` for derivative/correlated-source accounting;
- coded geographic scope with coarse-to-fine precision, but no address or coordinates;
- a bounded observation window and report time;
- a source/protocol-defined aggregate contributing-unit count, stored under the inherited v1 wire name `cohort_size`;
- typed metric kind, estimate, unit, and bounded uncertainty;
- producer/protocol/revision/upstream provenance and an algorithm-tagged, non-zero source-record digest.

The schema intentionally has no patient identifier, name, date of birth, exact address, latitude/longitude, raw genome, pathogen sequence, treatment recommendation, or emergency authority field.

Identity-significant structured wire types reject unknown fields where forward-compatible silent interpretation would be unsafe. Nested windows, metrics, uncertainty envelopes, and provenance are revalidated at the `SurveillanceObservation` boundary rather than trusted merely because they arrived inside a public struct.

## Aggregate count semantics

The v1 field name `cohort_size` is retained for wire and identity compatibility. New code should prefer `SurveillanceObservation::contributing_unit_count()` when it wants the numeric value without implying a human cohort.

The meaning of that count comes from the source/acquisition protocol. Depending on the source it may represent, for example:

- aggregate laboratory tests or another tested denominator;
- already-aggregated visits, events, or a protocol-defined syndromic denominator;
- wastewater/environmental samples or contributing aggregate sampling units;
- health-system beds, staffed units, or other capacity units;
- another explicitly documented aggregate contributing unit.

The count therefore does **not** establish the number of unique people represented by the evidence unless a separately reviewed source protocol establishes that exact interpretation. It must not be converted into a population size, privacy guarantee, or generic confidence weight merely because the field is non-zero or large.

Changing the serialized field name or adding identity-significant count-semantics metadata would be a v1 identity-contract change and requires an explicit `ObservationId` version decision. Source/profile-specific privacy semantics are being designed separately rather than silently changing the frozen encoding.

## Content identity

Every validated observation receives a domain-separated SHA-256 semantic identity under:

`mycelix-health-surveillance-observation-v1`

The digest commits to all identity-significant v1 fields, including provenance, source-record digest **algorithm + bytes**, and independence grouping. Floating-point values are hashed by validated IEEE-754 bit pattern with negative zero canonicalized to positive zero.

A SHA-256 source digest and BLAKE3 source digest with the same 32 bytes therefore remain different provenance claims and produce different observation identities.

This is **content identity only**. It does not prove who produced the evidence, that the producer is trusted, or that the source record is authentic. Authentication/signature authority belongs to later Mycelix/Xenia integration.

## Aggregate release boundary

`AggregateReleasePolicy` can require:

- a minimum source/protocol-defined aggregate contributing-unit count;
- a minimum aggregation window;
- a maximum geographic precision.

The resulting `ReleaseAssessment` is bound to the exact `ObservationId` and exact policy values.

Policy thresholds are private, constructor-validated, and revalidated during deserialization, so a zero-threshold policy cannot bypass the invariants through a struct literal or wire payload.

The numeric count floor has only the semantics of the observation's source/acquisition protocol. A laboratory-test threshold, wastewater-sample threshold, and capacity-unit threshold are not interchangeable privacy claims. Deployments must not describe a passing generic count floor as proving that an equivalent number of unique people contributed.

Passing this policy is only a structural release gate. It does **not** claim formal k-anonymity, differential privacy, resistance to all linkage attacks, HIPAA/GDPR compliance, or universal safety. Source-specific pipelines may require differential privacy, population/catchment safeguards, suppression rules, or stronger review before publication.

No universal policy thresholds are embedded in the crate. Thresholds must be selected and versioned by the deploying public-health/privacy governance process.

## Freshness

`FreshnessPolicy` evaluates evidence age explicitly and rejects a report timestamp that lies in the evaluator's future. Stale evidence remains evidence, but it is never silently upgraded to fresh evidence.

## Evidence bundles

`EvidenceBundle` is canonical and order-independent. v1 requires:

1. at least one validated observation;
2. no duplicate observation content identities;
3. one signal family;
4. one exact geographic scope;
5. a positive-width overlapping time window across every observation.

Bundles retain the complete observations rather than only their hashes, reject unknown top-level wire fields, and receive their own domain-separated identity under:

`mycelix-health-surveillance-bundle-v1`

## Source independence

Two dashboards can repeat the same laboratory feed while looking like two sources. `IndependenceGroup` prevents that presentation-layer duplication from automatically becoming two independent lineages.

`LineageDiversityPolicy` evaluates the number of unique independence groups and unique source kinds. Its non-zero thresholds are constructor- and deserialization-validated. The resulting assessment is bound to the exact bundle and policy.

A result of `MeetsPolicy` means only that the evidence set meets the declared structural diversity requirement. It does **not** establish:

- statistical agreement;
- causal independence;
- source authenticity;
- outbreak existence;
- severity;
- a recommended intervention.

The independence group itself is currently a producer-supplied provenance claim. Later authenticated federation must verify or attest those lineage claims where stronger guarantees are required.

## Intended architecture

```text
source-specific aggregation
        |
        v
health-surveillance-core
  validation / uncertainty
  provenance / independence
  release / freshness
  canonical evidence bundles
        |
        v
Mycelix surveillance zome (future)
        |
        v
Symthaea health-resilience reasoning (future)
  baselines / change detection
  competing hypotheses
  model uncertainty
  capacity forecasts
        |
        v
human / institutional authority
```

The future Symthaea layer should consume these contracts rather than inventing a detached medical-AI confidence score. Measurement uncertainty, source authenticity, source independence, sampling bias, model uncertainty, forecast uncertainty, and decision authority should remain separable. In particular, the legacy `cohort_size` field must not be mapped directly to population size or used as a generic confidence multiplier without source/profile semantics.

## Next tranches

1. **Surveillance zome** — enforce aggregate release policy before DHT publication and preserve policy/evidence receipts.
2. **Authenticated provenance** — bind producer and lineage claims to Mycelix/Xenia identity/credential evidence.
3. **Privacy upgrades** — source/profile-specific privacy admissibility, optional differential-privacy release mechanisms, and budget accounting where appropriate.
4. **FHIR/wastewater/capacity adapters** — source-specific transformations into the aggregate evidence contract without exposing patient-level data.
5. **Symthaea health-resilience** — auditable baselines, anomaly/change-point candidates, competing hypotheses, abstention, and calibrated evaluation.
6. **Catastrophic resilience simulation** — pandemic through extreme infrastructure-loss scenarios, without pathogen-design functionality or autonomous emergency authority.

## Validation

The focused workflow qualifies the immutable feature head with formatting, tests, Clippy with warnings denied, doc tests, and a WASM-target check for this crate. Full repository CI remains additive integration evidence.
