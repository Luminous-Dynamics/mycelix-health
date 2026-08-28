# Health Surveillance Observation Endorsement

This crate defines the canonical, crypto-free contract for a **positive status endorsement of one exact aggregate surveillance observation**.

It exists because a broad producer credential and a finite grant do not solve revocation/freshness by themselves. A validating DHT peer should not make a live cross-DNA query whose answer can vary by peer, and a Holochain action timestamp should not be treated as a globally trusted wall clock.

The stronger online profile is therefore:

1. a producer has a signed `ProducerAuthorityGrant`;
2. the producer constructs one exact aggregate `SurveillanceObservation`;
3. a credential/status authority checks the producer's current standing under an explicit status-check profile;
4. the authority signs an `AuthorizedObservationEndorsement` bound to that exact observation;
5. the surveillance DNA verifies the endorsement locally from the signed bytes it receives.

## Exact binding

The endorsement commits to:

- status-check profile;
- status issuer DID;
- exact `ProducerAuthorityGrantId`;
- exact `ObservationId`;
- exact release-policy identity;
- exact publisher DID;
- exact producer identity;
- the issuer-claimed status-check timestamp;
- an opaque commitment to the external status evidence evaluated;
- a non-zero issuance nonce.

Changing the observation, grant, release policy, publisher, producer, or issuer changes the semantic object and invalidates reuse.

## Positive-only semantics

`AuthorizedObservationEndorsement` is intentionally a positive assertion. The public surveillance network does not need a generalized registry of revoked/suspended/denied actors.

A status adapter should only issue this endorsement after its configured checks pass. Revoked, suspended, unknown, unavailable, or otherwise unacceptable status should result in **no positive endorsement**.

This keeps denial/status detail outside the public aggregate evidence layer unless a deployment deliberately chooses to expose it elsewhere.

## Wall-clock boundary

`checked_at_unix_s` is an issuer assertion carried inside the signed transcript. The semantic binding requires it not to predate the observation's own `reported_at_unix_s`.

The crate does **not** claim that a Holochain action timestamp proves global real time, and it does not define endorsement expiry from DHT time. Online freshness comes from requiring a new status-authority endorsement for each new observation.

## Offline / degraded resilience

A future deployment may deliberately support a degraded-network profile where short producer grants can be used without a live status authority. That should be an explicit policy profile with visibly weaker revocation guarantees, not an accidental fallback when status verification is unavailable.

## Privacy and scope

This contract contains no patient data, raw clinical records, pathogen sequences, or individual identifiers beyond institutional/publisher DIDs. `status_evidence_commitment` is opaque and should commit to external verification evidence without copying sensitive credential/status material into the surveillance DHT.

## Independence remains separate

An exact positive status endorsement authenticates publisher authority for one observation. It does **not** authenticate the observation's `IndependenceGroup` as scientifically independent from another source. Lineage/corroboration attestation remains a separate contract and review tranche.
