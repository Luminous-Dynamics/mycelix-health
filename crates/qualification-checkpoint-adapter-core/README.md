# Mycelix Qualification Checkpoint Adapter Core

QUAL-EVID-006 / #218 reference for consuming a QUAL-EVID-004/#217 governed current-head checkpoint at the exact Patient-v2 #208 composition-commitment seam.

## Product-facing theorem

```text
#217 verified head
+
#210/#214 governed qualification
+
#212 exact product profile
+
local receipt admissibility
+
current consumption time
=
Profile208CompositionCommitmentToken
```

Every `+` above is an independent required condition, not a confidence score.

## Why this crate exists

`mycelix-qualification-adapter-profile-core` is the lower QUAL-EVID-002 reference. It intentionally modeled current-head verification with an adapter boolean while the durable checkpoint protocol did not yet exist.

This crate is the product-facing successor boundary. It depends on that lower reference internally but does **not** re-export its raw `VerifiedLineageHead` type or boolean constructor.

The only public head type is `VerifiedQualificationHeadCheckpoint`, matching the identity emitted by #217.

## Remaining trust boundary

This crate does not re-run #217's Ed25519/quorum/store verification. `from_qualified_checkpoint_report(...)` is the typed adapter boundary for a report already produced by the independently qualified #217 verifier.

Whole-volume rollback resistance remains separately blocked on QUAL-EVID-005/#216.
