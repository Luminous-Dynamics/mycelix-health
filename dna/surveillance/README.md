# Health Surveillance DNA

This DNA is the public, aggregate-only publication boundary for Mycelix Health surveillance evidence.

It is intentionally separate from the patient/clinical Health DNA. Patient records, consent records, individual FHIR resources, raw laboratory records, exact locations, and other sensitive clinical data do not belong in this network.

## Default behavior: publication disabled

`dna.yaml` ships with:

```yaml
properties:
  release_policy: ~
```

That is deliberate. With no configured release policy, the integrity zome rejects surveillance publication. A deployment must create/instantiate a DNA whose properties contain a governance-approved policy revision and non-zero structural release thresholds.

The project does **not** provide universal privacy thresholds. Appropriate cohort size, aggregation windows, geographic precision, differential-privacy requirements, and legal/governance review depend on the deployment and source pipeline.

## Policy identity

`policy_revision` is a human/audit label, not the immutable identity of a release policy.

The integrity zome derives a domain-separated `ReleasePolicyId` over the exact revision label plus cohort threshold, aggregation-window threshold, and maximum geographic precision. Every released observation carries both the readable revision and this exact policy commitment.

Reusing the same revision label with different thresholds therefore creates a different policy identity and cannot be silently substituted into an existing released entry.

## Integrity model

For every `ReleasedSurveillanceObservation`, every validating peer independently checks:

1. the publisher field equals the Holochain action author;
2. the entry's `policy_revision` equals the revision frozen into DNA properties;
3. the entry's `policy_id` equals the immutable commitment derived from those exact DNA properties;
4. the underlying `SurveillanceObservation` satisfies the evidence-core invariants;
5. the DNA's release policy deterministically reassesses the observation;
6. the observation passes that policy;
7. the stored `ReleaseAssessment` exactly equals the recomputed assessment.

Released observations are append-only in v1. Updates and deletes are rejected.

No links are publishable in v1. Indexing/query semantics will be designed separately so convenience indexes cannot become an unreviewed evidence-authority layer.

## What publisher binding means

The Holochain action author authenticates the agent key that published an entry to this DHT. It does **not** prove that the observation's human-readable producer/protocol/upstream provenance is institutionally authentic.

A later authenticated-provenance tranche must bind those claims to Mycelix/Xenia identity and credential evidence. Until then, producer and independence-lineage metadata remain evidence claims carried by the observation, not verified institutional attestations.

## Non-goals

This DNA does not:

- store patient-level medical records;
- diagnose disease;
- identify or engineer pathogens;
- declare outbreaks;
- recommend treatment;
- issue public-health orders;
- grant emergency privileges;
- give Symthaea autonomous response authority.

It only creates a stronger distributed publication boundary for aggregate evidence.

## Intended stack

```text
private source systems / Health DNA
            |
      source-specific aggregation
            |
      stronger privacy mechanisms
       where source requires them
            |
            v
   health-surveillance-core
            |
      DNA-bound release gate
            |
            v
     Health Surveillance DNA
            |
   authenticated provenance (next)
            |
            v
     Symthaea health-resilience
   evidence analysis / hypotheses
            |
            v
    human / institutional authority
```

## Building

The zomes are workspace members:

```bash
cargo build --release -p surveillance_integrity -p surveillance
```

The repository's `.cargo/config.toml` targets `wasm32-unknown-unknown` by default, matching Holochain zome deployment.

To package the DNA after selecting deployment properties, use the Holochain `hc dna pack` flow appropriate to the pinned Holochain 0.6 toolchain. Do not deploy the default null-policy manifest expecting publication to work; it is intentionally fail-closed.
