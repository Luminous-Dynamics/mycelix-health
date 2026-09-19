# Mycelix Care Capability Core

Reference semantics for CARE-CAP-001 (#163).

This crate is intentionally isolated from the production `mycelix-health` workspace. It freezes what a care authorization **means** before choosing signature, identity, key-wrapping, Holochain, Xenia, or selective-disclosure adapters.

## Boundary

```text
private CareCapabilityV1
          |
          | monotonic delegation only
          v
 attenuated CareCapabilityV1
          |
          | active + not revoked + exact grantee/policy/key epoch
          v
 AccessCapsuleBindingV1
          |
          | future crypto adapter
          v
 wrapped content-key capsule
```

Public DHT material is represented separately as `CapabilityCommitmentV1`, which has no clear care purpose, grantee, claim selector, diagnosis, record type, or role.

## Deliberately strict v1 delegation

Delegation may only reduce authority. A child:

- selects a subset of parent claims;
- selects a subset of parent actions;
- cannot outlive the parent;
- cannot start before the parent;
- must reduce remaining delegation depth;
- cannot weaken `no_further_disclosure`;
- uses the parent grantee as its issuer;
- keeps the exact subject context, policy handle, purpose and key epoch.

Changing purpose, policy, subject context or key epoch requires a fresh authority proof rather than being hidden inside delegation.

## Revocation semantics

Revocation prevents this reference state machine from authorizing **new** access-capsule bindings after the revocation time.

It does not and cannot claim to erase:

- plaintext already viewed;
- keys already legitimately delivered;
- exports/screenshots/copies already made.

Key rotation creates a new epoch, and this v1 requires a capability explicitly bound to that exact epoch.

## Emergency capability

`EmergencyCapabilityV1` is a separate family. It is:

- finite and capped at 60 minutes;
- explicit-claim selected;
- read-only;
- non-delegable;
- unable to express research or model-training rights.

The reference type does not decide who may legally issue emergency authority; issuer-proof qualification remains separate.

## Non-claims

This crate does **not** verify signatures, credentials, identity, legal consent, Holochain capabilities, ML-KEM, content-key wrapping, proxy re-encryption, or public commitment hashing. `IssuerProofRef` and commitment digests are opaque adapter references only.

A future green reference run therefore proves structural authorization/attenuation/revocation semantics only.
