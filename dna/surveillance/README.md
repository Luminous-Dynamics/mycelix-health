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

The DNA also freezes:

- a non-zero maximum producer-grant lifetime;
- one or more exact trusted `(security_domain, issuer_did, credential_schema_id)` tuples.

v1 issuer DIDs use `did:mycelix:<AgentPubKey>`. The issuer signs the exact domain-separated transcript defined by `health-surveillance-authority`; every validating peer verifies that detached Ed25519 signature locally with Holochain's raw signature-verification host function.

The subject DID inside the signed grant must equal the DHT publisher's `did:mycelix` identity. The grant must also cover the observation's exact producer, source kind, signal family, source instance, acquisition protocol, geography, and signed/declared time fields.

This means a valid signature for one publisher/feed/protocol/district cannot be replayed as authority for another.

## Time and revocation boundary

Grant validity is encoded as a finite signed interval and the DNA caps its duration. The current integrity path compares that interval against the observation's declared time and the Holochain action timestamp.

Those timestamps are authenticated/chain-ordered claims, **not a globally trusted wall clock**. A finite grant therefore limits the semantic validity interval of the signed authority but must not be advertised as equivalent to real-time credential expiration or revocation.

The stronger follow-up is an issuer-signed, exact-observation endorsement/status receipt that commits to the grant ID, observation ID, release-policy ID, and publisher after the issuer has checked current credential status. A revoked producer then cannot obtain authorization for a new observation, while DHT validation can remain deterministic and self-contained.

A later trusted-time evidence profile can additionally support claim-bearing real-time freshness/expiry statements. Until then, v1's time fields are explicit evidence claims with bounded semantics, not global-time proof.

## Integrity model

For every `ReleasedSurveillanceObservation`, every validating peer independently checks:

1. publisher equals the Holochain action author;
2. release-policy revision and exact `ReleasePolicyId` match DNA properties;
3. the aggregate observation revalidates and passes the release policy;
4. the stored release assessment exactly matches recomputation;
5. the producer grant is structurally valid and within the DNA maximum declared lifetime;
6. grant subject DID equals the publisher DID;
7. grant issuer/security-domain/schema tuple is trusted by this DNA;
8. the observation lies inside the exact claimed grant scope;
9. the detached signature is exactly 64-byte Ed25519 and verifies over the canonical signing transcript.

Released observations remain append-only. Updates and deletes are rejected. v1 exposes no publishable links; indexing/query semantics remain a separate tranche.

## Identity boundaries that remain separate

A trusted producer grant establishes permission for a subject to publish a defined aggregate feed under a defined producer identity. It does **not** establish:

- that two feeds are statistically or causally independent;
- that `IndependenceGroup` is truthful;
- that an observation's measurements are scientifically correct;
- that an institution should be trusted for unrelated purposes;
- real-time credential/revocation status;
- any authority to issue medical/public-health orders.

Lineage-independence attestation, credential-status evidence, trusted-time evidence, and scientific evidence quality remain separate problems.

## Relationship to Mycelix Identity and Xenia

Mycelix Identity already provides DID, credential-schema, W3C verifiable credential/presentation, signature, challenge/domain, and revocation machinery. An Identity adapter can issue/present a credential whose claims commit to the exact `ProducerAuthorityGrant` transcript.

Xenia demonstrates the complementary pattern used here: signer and verifier share one small crypto-free transcript definition rather than independently serializing what they think an authorization means.

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
 canonical signed producer scope
            |
            v
 Health Surveillance DNA
 release + publisher + issuer proof
            |
            v
 exact-observation status endorsement
 + lineage/time evidence upgrades
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
cargo build --release -p surveillance_integrity -p surveillance
```

The repository's `.cargo/config.toml` targets `wasm32-unknown-unknown` by default, matching Holochain zome deployment.

Do not deploy the default null-policy manifest expecting publication to work; it is intentionally fail-closed.
