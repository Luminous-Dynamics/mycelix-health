# Mycelix Qualification Product Authority Core

Final product-facing QUAL-EVID-006/#218 boundary for the first Patient-v2 #208 composition seam.

## Public evidence inputs

The gate accepts only:

- `VerifiedProfile208Match` — the exact QUAL-EVID-003/#214 profile-match identity; and
- `VerifiedCurrentHead208` — the exact QUAL-EVID-004/#217 governed current-head identity.

It does not accept a caller-selected `verified` bit or a bare expected profile-match digest.

## Composition theorem

```text
#214 governed profile match
+
#217 governed current head
+
#210 governed qualification
+
#210 local admissibility
+
#212 exact product profile/freshness
+
monotonic product-consumption time
=
Profile208ProductAuthorityToken
```

All identities must agree exactly on the relevant semantic lineage, receipt, policy, trust store, deployment and profile-match commitment.

## Lower layers

The crate intentionally depends on:

- `mycelix-qualification-checkpoint-adapter-core`, which binds the #217 head to #212; and
- `mycelix-qualification-receipt-core`, which supplies governed qualification/ledger types.

Those lower crates are implementation dependencies. Their lower-level current-head/profile-match adapter inputs are not re-exported here.

## Remaining boundary

The typed constructors are adapter boundaries for reports already produced by the independently governed #214/#217 verifiers. This crate does not independently re-run Ed25519 verification.

Whole-volume rollback resistance remains blocked on QUAL-EVID-005/#216.
