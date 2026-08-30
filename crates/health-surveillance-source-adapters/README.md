# Health Surveillance Source Adapters

Pure Rust adapters that convert **already-aggregated** source measurements into the common `health-surveillance-core::SurveillanceObservation` contract.

This crate is intentionally upstream of Holochain publication and downstream of source-specific privacy/aggregation. It has no network I/O and no authority semantics.

## Boundary

Accepted inputs contain only aggregate facts such as:

- positive and tested counts;
- event count and aggregate denominator;
- wastewater/environmental concentration index plus sample count;
- health-system capacity counts;
- aggregate geography/time window;
- source/protocol/provenance identifiers;
- source-supplied uncertainty bounds.

The input API contains no fields for patient IDs, names, DOB, MRN, encounter identifiers, street addresses, coordinates, raw FHIR resources, laboratory line lists, genomes, sequences, or free-form clinical records.

If a deployment begins with patient-level/FHIR data, that data must remain inside its clinical/privacy boundary and be aggregated before calling this crate.

## Adapters

### Laboratory fraction

`adapt_laboratory_fraction` computes `positive_count / tested_count` and emits:

- `SourceKind::LaboratoryAggregate`;
- `MetricKind::FractionPositive`;
- unit `fraction`;
- `cohort_size = tested_count`.

Zero denominators and `positive_count > tested_count` fail closed.

### Syndromic rate

`adapt_syndromic_rate_per_100k` computes an aggregate event rate from an already-aggregated event count and denominator and emits:

- `SourceKind::ClinicalSyndromicAggregate`;
- `MetricKind::RatePer100k`;
- unit `per_100k`.

The adapter does not decide which patient encounters satisfy a syndrome definition.

### Wastewater/environmental concentration

`adapt_concentration` preserves whether the aggregate source is wastewater or another environmental monitoring path and emits `MetricKind::ConcentrationIndex` with a caller-supplied canonical aggregate unit.

Raw assay/genomic payloads are outside this contract.

### Health-system capacity

`adapt_capacity_fraction` emits `SourceKind::HealthSystemCapacityAggregate` and requires explicit `Available` or `Occupied` semantics. The resulting unit is respectively `fraction_available` or `fraction_occupied`, preventing those meanings from being silently conflated.

## Uncertainty

The adapters do **not** invent confidence intervals or probabilities.

Every source supplies its own `BoundedUncertainty`, and the common evidence core verifies that the computed/declared estimate is finite, in range, and inside the supplied interval.

Statistical methods for producing uncertainty belong to separately reviewed source/protocol implementations. This prevents an adapter convenience function from becoming an undocumented epidemiological model.

## Provenance and lineage

`ObservationContext` carries the common source instance, claimed `IndependenceGroup`, aggregate geography/window, reporting time, and `EvidenceProvenance`.

These adapters preserve those claims; they do not authenticate them. Producer authority/status and signed lineage evidence remain separate surveillance layers.

## Non-goals

This crate does not:

- read patient records or FHIR resources;
- de-identify raw clinical datasets;
- publish to Holochain;
- issue producer credentials;
- prove source independence;
- diagnose disease;
- declare outbreaks;
- recommend treatment;
- authorize emergency/public-health actions;
- identify, design, or optimize pathogens.

## Intended flow

```text
private clinical / laboratory / environmental source
                    |
          source-specific aggregation
          privacy + legal review
                    |
                    v
 health-surveillance-source-adapters
                    |
        SurveillanceObservation
                    |
                    v
     aggregate release policy
     producer/status verification
                    |
                    v
       Health Surveillance DNA
                    |
                    v
      lineage evidence + Symthaea
```
