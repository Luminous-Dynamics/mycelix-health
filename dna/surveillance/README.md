# Health Surveillance DNA

This DNA is the public, aggregate-only publication boundary for Mycelix Health surveillance evidence.

It is intentionally separate from the patient/clinical Health DNA. Patient records, consent records, individual FHIR resources, raw laboratory records, exact locations, and other sensitive clinical data do not belong in this network.

## Default behavior: publication disabled

`dna.yaml` ships with both authority surfaces disabled:

```yaml
properties:
  release_policy: ~
  producer_authority_policy: ~
```

That is deliberate. A deployment must explicitly pin both a governance-approved aggregate release policy and a producer-authority trust policy before the DHT accepts surveillance publication.

The project does **not** provide universal privacy thresholds or a universal institutional trust list.

## Release-policy identity

`policy_revision` is a human/audit label, not the immutable identity of a release policy.

The integrity zome derives a domain-separated `ReleasePolicyId` over the exact revision label plus cohort threshold, aggregation-window threshold, and maximum geographic precision. Every released observation carries both the readable revision and this exact policy commitment.

Reusing the same revision label with different thresholds therefore creates a different policy identity and cannot be silently substituted.

## Producer-authority policy

The DNA freezes:

- a non-zero maximum producer-grant lifetime;
- one or more exact trusted `(security_domain, issuer_did, credential_schema_id)` tuples;
- one or more exact accepted positive status-endorsement profiles.

v1 issuer DIDs use `did:mycelix:<AgentPubKey>`. The issuer signs two distinct domain-separated transcripts:

1. the broad `ProducerAuthorityGrant` defined by `health-surveillance-authority`;
2. the exact `AuthorizedObservationEndorsement` defined by `health-surveillance-endorsement`.

Every validating peer verifies both detached Ed25519 signatures locally with Holochain's raw signature-verification host function.

The producer grant's subject DID must equal the DHT publisher's `did:mycelix` identity. The grant must cover the observation's exact producer, source kind, signal family, source instance, acquisition protocol, geography, and signed/declared time fields.

## Exact-observation status endorsement

A broad producer grant is no longer enough for the online v1 publication path.

For every released observation, the same trusted issuer must provide a positive endorsement that commits to:

- accepted status-check profile;
- issuer DID;
- exact `ProducerAuthorityGrantId`;
- exact `ObservationId`;
- exact `ReleasePolicyId`;
- exact publisher DID;
- exact producer identity;
- issuer-asserted status-check time;
- opaque status-evidence commitment;
- non-zero issuance nonce.

The endorsement's status-check time must not claim to precede the observation's own report time.

Changing the observation, grant, release policy, publisher, producer, or issuer breaks semantic binding before cryptographic verification. A signature for one observation cannot be replayed as authorization for another.

The public DHT does not copy the underlying credential/revocation record. `status_evidence_commitment` is an opaque commitment to whatever external evidence the trusted status adapter evaluated.

## Time and revocation boundary

The exact-observation endorsement is materially stronger than relying on finite grant lifetime alone: a producer must obtain a new positive issuer assertion for each new observation.

That still does **not** turn a Holochain action timestamp into a globally trusted wall clock. `checked_at_unix_s` is an issuer assertion inside the signed transcript. Consensus does not claim to prove global real time or live revocation by querying another DNA.

A properly operated status adapter should simply refuse to issue a positive endorsement when the credential is revoked, suspended, unknown, stale, unavailable, or otherwise outside its accepted status profile. Existing endorsed observations remain historical evidence; revocation stops new positive endorsements.

For degraded/offline operation, a future deployment profile may deliberately allow short producer grants without per-observation status service access. That must be an explicit weaker resilience profile rather than an accidental fallback when status verification fails.

## Integrity model

For every `ReleasedSurveillanceObservation`, every validating peer independently checks:

1. publisher equals the Holochain action author;
2. release-policy revision and exact `ReleasePolicyId` match DNA properties;
3. aggregate observation revalidates and passes the release policy;
4. stored release assessment exactly matches recomputation;
5. producer grant is structurally valid and within the DNA maximum declared lifetime;
6. grant subject DID equals publisher DID;
7. grant issuer/security-domain/schema tuple is trusted by this DNA;
8. observation lies inside the exact claimed grant scope;
9. status-endorsement profile is accepted by this DNA;
10. endorsement binds the exact grant, observation, release policy, publisher, producer, and issuer;
11. producer-grant signature is exactly 64-byte Ed25519 and verifies over its canonical transcript;
12. status-endorsement signature is exactly 64-byte Ed25519 and verifies over its different canonical transcript.

Released observations remain append-only. Updates and deletes are rejected. v1 exposes no publishable links; indexing/query semantics remain a separate tranche.

## Identity boundaries that remain separate

The two signatures establish publisher authority plus one issuer assertion of acceptable current standing for the exact observation. They do **not** establish:

- that two feeds are statistically or causally independent;
- that `IndependenceGroup` is truthful;
- that an observation's measurements are scientifically correct;
- that an institution should be trusted for unrelated purposes;
- globally trusted time;
- any authority to issue medical/public-health orders.

Lineage-independence attestation, trusted-time evidence, and scientific evidence quality remain separate problems.

## Relationship to Mycelix Identity and Xenia

Mycelix Identity already provides DID, credential-schema, W3C verifiable credential/presentation, signature, challenge/domain, and revocation machinery. An adapter can map a validated credential/status check into the exact producer-grant and observation-endorsement transcripts without forcing DHT consensus to make live cross-DNA calls.

Xenia demonstrates the complementary pattern used here: signer and verifier share small crypto-free transcript definitions rather than independently serializing what they think an authorization means.

The surveillance DNA does not reuse Xenia's remote-session `Viewer/Approver/Operator/Admin` role hierarchy; those are unrelated to institutional public-health producer authority.

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

## Intended stack

```text
private source systems / Health DNA
            |
 source-specific aggregation/privacy
            |
            v
   health-surveillance-core
            |
            v
 health-surveillance-authority
 broad canonical producer scope
            |
            v
 external credential/status adapter
        |              |
 signed producer    exact observation
     grant          status endorsement
        |              |
        +------v-------+
               |
        Health Surveillance DNA
  release + publisher + two issuer proofs
               |
               v
   lineage-independence attestations
               |
               v
     Symthaea health-resilience
   evidence analysis / hypotheses
               |
               v
      human / institutional authority
```

## Building

```bash
cargo build --release \
  -p health-surveillance-endorsement \
  -p surveillance_integrity \
  -p surveillance
```

The repository's `.cargo/config.toml` targets `wasm32-unknown-unknown` by default, matching Holochain zome deployment.

Do not deploy the default null-policy manifest expecting publication to work; it is intentionally fail-closed.
