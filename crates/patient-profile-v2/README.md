# Mycelix Protected Patient Profile V2

Reference semantics for PATIENT-PRIV-002 (#186).

This crate freezes the first Tier-1 protected-storage migration boundary. It does **not** encrypt data or modify the packaged patient zome.

## Decomposition

The old public `Patient` aggregate is not copied wholesale into ciphertext.

`ProtectedPatientProfileV2` contains only protected identity/demographic/contact/provenance information. In particular it has no canonical arrays of:

- allergies;
- conditions;
- medications.

Those fields route to their clinical domains during migration.

The old cross-hApp identity hash routes to a protected identity-binding migration. The old MATL score routes to a legacy reliability observation/recomputation path rather than becoming a patient self/profile fact.

## Exact v1 field routing

The reference enum contains all 18 fields of the current v1 `Patient` entry and an exhaustive `route_legacy_field` match. The compiler therefore requires an explicit disposition for every modeled field.

A production adapter should additionally compare the real integrity-zome schema against this routing inventory so a new source field cannot be omitted silently.

## Discovery

The v2 default is:

- explicit capability/reference discovery only;
- no global patient directory;
- no clear DID→patient reverse lookup.

Authorized actors receive exact opaque profile references through separately qualified relationship/capability flows.

## One-way cutover

The state machine is:

`LegacyV1Writable -> Prepared -> V2Active -> LegacyV1ReadOnly`

After `V2Active`:

- new writes target protected v2 only;
- protected-read failure never falls back to v1 plaintext;
- a conflicting second migration is rejected;
- an identical exact migration registration is idempotent.

## Historical exposure

A migration receipt always records:

`legacy_public_exposure_may_persist = true`

The constructor provides no way to set it false. A v1 tombstone/update cannot prove that data previously published to peers has been recalled from every peer, backup or observation.

## Deliberate boundaries

This crate does not:

- implement encryption or Holochain storage;
- implement ProtectedEnvelopeV2 serialization;
- verify CARE-CAP authority;
- create a distributed patient directory;
- migrate real patient records;
- claim old DHT-visible data can be erased retroactively;
- make HIPAA/Part 2/GDPR/POPIA compliance claims.

It is a representation/migration theorem that production adapters can bind to qualified envelope/capability/disclosure layers.

Tracks #186 and #164.
