# Surveillance Lineage Zomes

These zomes publish signed, append-only lineage evidence about observations that have already crossed the main Health Surveillance DNA publication boundary.

## Why a separate zome

Lineage evidence has different availability and authority semantics from base surveillance evidence:

- an observation may be urgently useful before lineage audit finishes;
- absence of a lineage attestation means `Unknown`, not invalid observation;
- producer credentials do not imply lineage-auditor authority;
- multiple trusted auditors may disagree, and their statements must remain separately visible;
- lineage evidence should never mutate the original released observation.

For those reasons, lineage is additive rather than a required field on `ReleasedSurveillanceObservation`.

## DNA policy

`lineage_attestor_policy` is independent from `producer_authority_policy`.

A lineage trust tuple is exactly:

- `security_domain`;
- `attestor_did`;
- `attestation_profile_id`.

v1 attestor DIDs must be `did:mycelix:<AgentPubKey>` and the attestor signs the canonical transcript from `health-surveillance-lineage` with Ed25519.

If `lineage_attestor_policy` is absent, lineage-attestation publication fails closed. Base observation publication is unaffected.

## Relay vs attestor

`ReleasedLineageAttestation` carries both:

- `submitted_by` — the Holochain agent that put the record on this DHT;
- `attestor_did` inside the signed attestation — the authority that made the lineage claim.

These identities may differ. A signed lineage statement can therefore be relayed without transferring attestor authority to the relay.

## Exact target binding

Each lineage record references the action hash of an existing released observation. Integrity resolves a valid record and requires the attestation's `ObservationId` to match the referenced observation exactly.

Changing provenance, source revision, metric data, or even the producer's own `IndependenceGroup` claim changes `ObservationId` and prevents reuse of an existing lineage signature.

## Cross-zome wire mirror

The lineage integrity zome currently decodes the referenced observation through `ReleasedSurveillanceObservationMirror`, a read-only serialization mirror built entirely from the shared core/authority/endorsement types.

This avoids linking the main integrity-zome implementation into the lineage WASM, but it creates an explicit schema-coupling obligation: breaking changes to `ReleasedSurveillanceObservation` must update this mirror and receive end-to-end DNA qualification in the same change.

Once the released-observation schema is qualified and stable, the preferred cleanup is to move the shared wire payload into a dedicated shared release-record crate used by both integrity zomes.

## No indexes in v1

The lineage zome deliberately creates no DHT links yet. It supports submit/get-by-action-hash only.

A later indexing tranche can add observation-to-attestation discovery after link validation, cardinality, conflict, and query semantics are reviewed. Until then, convenience indexing cannot become an unreviewed source of evidence authority.

## Conflicting attestations

There is no update/delete path. If two trusted attestors disagree, both records remain immutable evidence. Consumers should surface the conflict and apply explicit trust/profile policy rather than silently selecting one.

## What this does not prove

A valid lineage signature proves that a DNA-trusted attestor made the dimension-specific statement it signed. It does not automatically prove causal independence, statistical independence, measurement correctness, outbreak status, diagnosis, treatment need, or emergency authority.
