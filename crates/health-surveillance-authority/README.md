# health-surveillance-authority

Crypto-free canonical producer-authority contracts for Mycelix public-health surveillance.

## Why this exists

The surveillance DNA can already prove which Holochain agent authored a released entry and which aggregate release policy admitted it. That is not the same as proving that the agent is an authorized laboratory, hospital network, wastewater operator, or other institutional producer.

Mycelix Identity already has DID, credential-schema, verifiable-credential/presentation, signature, challenge/domain, and revocation machinery. Xenia independently demonstrates a useful security pattern: put the exact domain-separated signed transcript in a small shared crate so the signer and verifier cannot drift apart.

This crate applies that pattern to surveillance **without importing either product's runtime or crypto implementation**.

## What v1 defines

`ProducerAuthorityGrant` names:

- an explicit identity/security domain;
- an external credential-schema ID;
- issuer DID;
- subject/publisher DID;
- exact surveillance `producer` identity;
- finite validity interval;
- non-zero issuance nonce;
- exact allowed source kinds;
- exact allowed signal families;
- exact allowed source instances;
- exact allowed acquisition protocols;
- exact allowed aggregate geographies.

The scope collections are bounded, canonicalized, and duplicate-rejecting. Reordering the same set does not change grant identity or the signing transcript.

## Two identities, deliberately

`ProducerAuthorityGrantId` is a domain-separated SHA-256 semantic content identity.

`ProducerAuthorityGrant::signing_transcript()` is a separately domain-separated byte transcript intended for an external signature/VC adapter. It is constructed from semantic fields directly rather than serde/JSON bytes.

Changing the security domain, issuer, subject, producer, time bounds, nonce, credential schema, or any scope value changes the signed meaning.

## Claimed scope is not verified authority

`assess_claimed_scope()` checks whether a `SurveillanceObservation` fits the payload's declared producer/source/protocol/geography/signal/time scope.

Its result is intentionally named `ClaimedScopeAssessment`.

It does **not** establish:

- that the grant was signed;
- that the issuer is trusted;
- that a VC schema is genuine or active;
- that the credential is unexpired/unrevoked outside the grant's own time window;
- that an institution actually owns the source feed;
- that `IndependenceGroup` is truthful;
- that the observation itself is scientifically correct.

A later adapter must cryptographically verify the exact signing transcript and bind issuer trust before the surveillance DNA can treat the grant as authenticated authority.

## Independence remains separate

Producer publication authority does not authenticate `IndependenceGroup` in v1. A laboratory being authorized to publish does not prove that two of its feeds are causally/statistically independent.

This is intentional. Independence/lineage attestation needs its own evidence contract so publisher authorization cannot be mistaken for corroboration quality.

## Intended integration

```text
Mycelix Identity VC / Xenia-style signer
                |
                | signs exact transcript
                v
health-surveillance-authority
 canonical scope + grant identity
                |
         crypto verifier adapter
                |
                v
Health Surveillance DNA
  publisher key + policy + authority
                |
                v
Symthaea health-resilience
```

The verifier adapter should fail closed on unsupported signature algorithms, unknown identity security domains, issuer/trust-root mismatch, signature failure, credential-status uncertainty, subject/publisher mismatch, or scope mismatch.

## Non-goals

No public-health operational role hierarchy, no diagnosis, no treatment, no outbreak declaration, no emergency powers, no patient identity, and no pathogen-design functionality.
