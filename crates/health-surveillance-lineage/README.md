# Health Surveillance Lineage

Canonical, crypto-free lineage attestations for aggregate public-health surveillance evidence.

## Why this is not an `is_independent` boolean

Two feeds can differ in one way and still share a hidden upstream dependency:

- different dashboards backed by the same dataset;
- different laboratories sampling the same population frame;
- different sensors operated by one control domain;
- different reports produced by the same transformation pipeline.

A global signed boolean such as `independent: true` would be easy to overinterpret and hard to audit.

This crate instead represents **dimension-specific known lineage facts**.

## Lineage dimensions

`LineageDescriptor` carries explicit knowledge for:

- upstream roots/datasets/feeds;
- sampling frames/populations;
- collection/sensor/site systems;
- measurement/instrument/laboratory systems;
- processing/software/transformation pipelines;
- operator/control domains.

Each dimension is either:

- `Known(non-empty canonical ID set)`; or
- `Unknown`.

Unknown is never represented by an empty set and is never treated as evidence of separation.

## Exact observation binding

`EvidenceLineageAttestation` binds:

- security domain;
- attestation profile;
- attestor DID;
- exact `ObservationId`;
- the dimension-specific descriptor;
- attestor-asserted assessment time;
- opaque evidence commitment;
- non-zero issuance nonce.

Changing any observation field that participates in `ObservationId`—including the producer's own `IndependenceGroup` claim—breaks the attestation binding.

## Comparison semantics

Pairwise comparison returns one relation per dimension:

- `SharedKnown` — both attestations know identifiers and share at least one;
- `NoKnownOverlap` — both know non-empty identifier sets and those sets are disjoint;
- `Unknown` — at least one side lacks knowledge.

`NoKnownOverlap` is intentionally **not** named `Independent`. It says only what the attested identifier sets establish on that dimension.

The comparison also reports whether both attestations use the same security domain and assessment profile. A downstream policy may require compatible profiles before using even `NoKnownOverlap` as corroboration evidence.

## Trust boundary

This crate performs no cryptographic verification and grants no authority. A later surveillance lineage-integrity tranche should define a DNA-bound allowlist for lineage attestors separately from producer-authority issuers.

That separation matters: being authorized to publish a laboratory feed does not automatically make the laboratory, its credential issuer, or its operator authoritative about causal/statistical independence.

## Intended downstream use

Symthaea should consume lineage attestations as **evidence about source diversity**, not as ground truth. A conservative reasoning policy can then:

1. down-weight or merge evidence with known shared upstream dimensions;
2. abstain from independence-sensitive conclusions when dimensions are unknown;
3. distinguish no-known-overlap from actual demonstrated independence;
4. show which dimensions support or weaken corroboration;
5. preserve the original signed attestations in the reasoning receipt.

## Non-goals

No patient data, pathogen data, epidemiological diagnosis, treatment advice, outbreak declaration, emergency authority, or autonomous action.
