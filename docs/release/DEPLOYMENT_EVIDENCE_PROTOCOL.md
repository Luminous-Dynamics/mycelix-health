# Mycelix Health Deployment Evidence Protocol v1

## Purpose

A successful WebSocket connection proves only that a browser reached a
conductor. A signed zome response proves that an agent called a cell. Neither
fact alone proves that the cell runs the reviewed Mycelix Health release.

Deployment Evidence v1 binds the clinical runtime to four independently checked
identities:

1. the Health source release manifest;
2. the authenticated cell DNA hash returned by `app_info`;
3. the runtime descriptor returned by the patient coordinator zome;
4. a post-build Ed25519-signed artifact manifest trusted by the portal build.

Clinical hydration fails unless all four agree.

## Why evidence is post-build

The final DNA and WASM digests cannot be embedded into the same WASMs without a
recursive content-addressing dependency. The source contract is therefore
compiled into the runtime, while exact artifact identities are generated after
all WASMs and the DNA bundle have been built.

The DNA hash is not treated as a substitute for the explicit zome list. The
signed evidence records every integrity and coordinator WASM SHA-256 digest in
canonical Health v1 order. This matters for deployments where coordinator code
may be replaced without changing the integrity-zome identity.

## Canonical signed statement

The Ed25519 signature covers deterministic bytes with domain:

```text
MYCELIX-HEALTH-RELEASE-ARTIFACT-V1
```

The statement includes:

- release ID and wire schema version;
- SHA-256 of `release/health-v1.json`;
- schema migration epoch;
- source revision and build timestamp;
- installed DNA hash;
- SHA-256 of the packed DNA bundle;
- ordered integrity/coordinator names and WASM SHA-256 digests.

The Rust wire crate, TypeScript SDK, Python release tool, and portal verifier use
the same canonical representation and golden digest vector.

## Browser acceptance algorithm

The portal performs these checks before clinical decryption:

1. require authenticated transport and an authorized zome-call signer;
2. read the role DNA hash from authenticated `app_info`;
3. call `patient.health_check` and validate the compiled source contract;
4. require the runtime DNA hash to equal the `app_info` DNA hash;
5. parse the evidence and compiled signer trust store;
6. require exactly one active signer matching `signer_key_id`;
7. validate the source digest, migration epoch, release boundary, zome order,
   and nonzero artifact digests;
8. require the evidence DNA hash to equal the runtime DNA hash;
9. verify the Ed25519 signature over canonical evidence bytes;
10. record the signer, source revision, and evidence SHA-256 in portal
    provenance before enabling the typed records repository.

Any absence, ambiguity, malformed value, inactive signer, duplicate signer,
unknown signer, signature failure, or identity mismatch denies live hydration.
There is no fixture fallback that becomes writable.

## Source checkout state

The repository intentionally ships with:

- `UNRESOLVED-HEALTH-DNA` evidence;
- a zero signature;
- an empty trusted signer store.

This state is parseable but cryptographically invalid. It lets ordinary source
builds compile while ensuring they cannot represent themselves as a production
clinical deployment. `scripts/check-deployment-evidence.py` recognizes only this
exact fail-closed placeholder or a completely valid signed deployment.

## Offline signing ceremony

CI builds the exact release artifacts and emits:

- unsigned evidence JSON;
- canonical evidence bytes;
- a report containing evidence and DNA-bundle SHA-256 values.

The private release key is never required in CI. An offline operator verifies
the artifacts, signs the canonical bytes, and runs the `ceremony` command. That
command verifies the public/private key relationship, merges the signer without
silent key replacement, and publishes a complete evidence directory only after
all checks succeed.

## Key rotation and revocation

- A key ID identifies one immutable public key.
- Reusing an existing key ID with different key bytes is refused.
- Rotation uses a new key ID and a reviewed trust-store update.
- Old keys may remain with `revoked` status for historical verification, but
  they cannot authorize a new portal session.
- Trust-store updates are security-sensitive source changes and require the same
  review standard as consent and encryption code.

## Rollback policy

A valid signature does not by itself make an old release acceptable. A later
campaign should add an operator-configurable minimum migration epoch and a
monotonic release ledger. Deployment Evidence v1 exposes the migration epoch and
source revision needed for that policy but does not claim global rollback
prevention.

## Explicit non-guarantees

Deployment Evidence v1 does not yet provide:

- independent browser verification of raw Holochain action signatures;
- transparency-log inclusion or witness cosigning;
- globally monotonic rollback prevention;
- reproducible-build equality across independent builders;
- hardware-backed release-key custody;
- countersigned clinical mutations.

It does guarantee that the current portal will not promote data to authenticated
clinical provenance unless its authenticated cell, runtime source contract,
trusted release signer, and exact packaged artifact statement agree.
