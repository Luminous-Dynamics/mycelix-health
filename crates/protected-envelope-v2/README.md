# Protected Envelope V2 — Reference Semantics

Status: **source-staged / non-cryptographic reference core** for #159.

This crate freezes the representation boundary for a future metadata-minimized protected-health envelope. It is intentionally standalone and is not registered in the parent workspace, so it does not change the production dependency graph, `Cargo.lock`, Holochain DNA, or existing clinical-assurance subjects.

## What it establishes

The DHT-clear `PublicHeaderV2` contains only:

- envelope version;
- numeric cryptographic-suite registry ID;
- opaque envelope ID;
- opaque context-scoped handle;
- opaque policy handle;
- key epoch;
- nonce.

It deliberately contains no:

- patient/agent identity;
- record kind or diagnosis;
- mental-health/SUD/spiritual/crisis category;
- care role or recipient identity;
- consent purpose;
- exact human-readable event timestamp;
- stable key fingerprint.

Every clear field is included in a deterministic, domain-separated `aad_bytes()` transcript intended to be authenticated by a separately qualified AEAD implementation.

Fine semantic information belongs in `ProtectedInnerV2`, which is intended to be serialized inside authenticated encryption.

`AccessCapsuleV1` is a separate type so key distribution/revocation can be qualified independently. This crate does **not** say an access capsule is safe to publish publicly.

## Why not reuse the existing envelopes unchanged?

The current repository has multiple overlapping encrypted representations:

- `records::EncryptedRecord` authenticates its clear metadata but exposes stable patient/category/type/time fields;
- `consent::EncryptedHealthEntry` exposes patient/category/type/key-version/time fields and mixes storage with consent/decryption-audit concepts;
- `health-crypto::PqEncryptedRecord` models a PQ hybrid format but leaves semantic category/type clear and does not authenticate those fields as AEAD AAD in the current helper.

Whole-person care raises the metadata stakes. Merely knowing that a particular patient has a `PsychotherapyNote`, `CrisisEvent`, `SubstanceUseCounselingNote`, or `PastoralCareNote` can itself be sensitive.

## Non-goals

This crate does not implement or qualify:

- XChaCha20-Poly1305;
- ML-KEM or any other KEM;
- HKDF/KDF behavior;
- Holochain persistence;
- patient/context-handle derivation;
- consent authorization;
- capability/key-wrap delivery;
- revocation;
- clinical semantics;
- regulatory/legal compliance.

Those are separate proof lines.

## Required qualification before integration

At minimum:

1. format + Clippy + tests in the repository's declared Rust/Nix environment;
2. external/adversarial review of the metadata privacy model;
3. a concrete cryptographic suite with standard KDF and AEAD AAD binding;
4. nonce-generation/uniqueness evidence;
5. context/policy-handle derivation with explicit linkability analysis;
6. separately qualified key-wrap/access-capsule delivery;
7. conductor-level multi-agent tests against the exact packaged DNA;
8. migration tests from v1 formats without silently upgrading their privacy claims;
9. exact source/toolchain/WASM/DNA evidence identities.

A passing reference-core test suite is not encryption or privacy proof by itself.
