# Clinical Integrity Digest v1

## Purpose

Clinical artifact identity must say **what was hashed, under which semantic domain, and by which algorithm**. A bare 32-byte value is not enough.

`mycelix-clinical-integrity` defines the v1 integrity contract used by high-assurance clinical workflow boundaries.

## Algorithm

v1 uses **BLAKE3-256 derive-key mode** with the fixed context:

`mycelix.health.clinical-integrity.v1`

The framed hash input is:

1. frame version byte (`1`)
2. big-endian domain-label length (`u16`)
3. exact domain label bytes
4. big-endian canonical-payload length (`u64`)
5. exact canonical artifact bytes

The domain is therefore part of the cryptographic input, not metadata attached after hashing.

## Domain separation

v1 domains include clinical artifacts, evidence capsules, medication orders, quarantine events/decisions, authority policies, issuer-trust policies, credential/status/issuer evidence, and workflow policy.

The same canonical bytes under two different domains must produce different identities.

## Stored versus verified digests

`StoredDigest` is serializable and may cross a wire/storage boundary. It is only a **claim**.

`VerifiedDigest` intentionally does not implement serde. It can be produced only by:

- locally hashing canonical bytes; or
- re-verifying a `StoredDigest` against the exact canonical bytes.

Safe workflow code should depend on `VerifiedDigest`, not on untrusted stored bytes.

## Canonicalization

This crate deliberately does not claim that arbitrary JSON is canonical.

Each owning artifact defines its own v1 canonical byte contract before calling `hash_canonical_bytes`. The initial workflow adapters use compact serde JSON over fixed Rust struct schemas and prepend an explicit schema tag before hashing. Any schema/field-order/encoding migration that changes canonical bytes changes the artifact identity and must be treated as a new versioned contract.

Long-term cross-language interoperability should move these artifact encodings to a separately specified canonical serialization profile rather than assuming generic JSON equivalence.

## Sensitive payloads

This digest contract is for integrity identities of artifacts that are appropriate to identify deterministically.

Rejected/raw clinical payloads that could enable confirmation attacks continue to use the separate **keyed commitment** boundary defined by the clinical quarantine layer. A BLAKE3 integrity digest is not a substitute for a keyed privacy-preserving commitment.

## Non-claims

A correct digest proves byte identity under this contract. It does not establish semantic correctness, clinical validity, authorization, consent, or regulatory compliance.
