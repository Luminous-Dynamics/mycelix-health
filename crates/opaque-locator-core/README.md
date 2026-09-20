# Mycelix Opaque Locator Core

Reference semantics for PATIENT-PRIV-004 (#198).

## Goal

Prevent the protected-storage migration from replacing a clear patient hash with
one long-lived pseudonymous patient handle.

The public discovery shape is deliberately only:

```text
OpaqueLocatorId -> ProtectedManifestEnvelopeId
```

The public edge does not contain subject identity, DID/MRN, audience, purpose,
record kind/category, selected-record list, or exact expiry.

## Protected manifest

The manifest is intended to be encrypted inside ProtectedEnvelopeV2. Its
plaintext binds the real subject/context/policy/capability/audience/purpose,
finite validity, provenance, generation, completeness semantics, selected
EnvelopeIds, and protected predecessor lineage.

The manifest is a discovery projection. It is not a clinical source of truth.

## Private locator grant

A recipient obtains locator material only through a separately authorized path.
The private grant binds the exact locator, expected manifest, expected envelope
context, policy, capability, audience, purpose, key epoch, generation and finite
validity window.

Resolution rejects locator, manifest, context, policy, key-epoch and time
substitution before any higher layer treats the manifest as authorized.

## Cardinality minimization

V1 intentionally models one public locator edge to one encrypted manifest rather
than one stable patient/context anchor with N public record edges. The N selected
record references remain inside protected manifest plaintext.

This reduces semantic DHT fanout. It does not claim resistance to traffic/timing
analysis.

## Rotation

Rotation within one authority context:

- requires exactly the next generation;
- cannot move issuance time backwards;
- cannot extend expiry;
- preserves context, policy, capability, audience, purpose and key epoch.

Changing any preserved authority field requires a new grant/authority proof.

`LocatorRegistry` also rejects reuse of one locator ID across different context
handles. This is a structural anti-linkability rule; randomness/unlinkability of
the actual locator generator remains a separate cryptographic qualification.

## Manifest lineage

Generation 1 must not name a predecessor. Later generations must name the exact
prior manifest, and the registry rejects missing, conflicting or out-of-order
lineage.

Exact same `(context, generation, manifest ID)` registration is idempotent.

## Deliberate boundaries

This crate does not:

- generate random locators;
- prove cryptographic unlinkability;
- encrypt manifests;
- hash/sign manifest IDs;
- verify CARE-CAP signatures or scope of practice;
- store Holochain links;
- eliminate timing/traffic leakage;
- recall locators or legacy DHT observations already seen;
- make regulatory-compliance claims.

Those remain separate proof lines.

## Composition

The intended dependency direction is:

```text
VaultSession (#175)
    -> CARE-CAP (#177)
    -> private locator grant (#198 / this core)
    -> public locator edge
    -> ProtectedEnvelopeV2 manifest (#161)
    -> protected EnvelopeId set
    -> CARE-DISC projection (#173)
```

Patient v2 (#188) must also satisfy the Tier-1 link-topology classification gate
(#192 / #197) before production cutover.
