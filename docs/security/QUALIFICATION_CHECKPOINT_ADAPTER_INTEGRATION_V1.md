# Qualification Checkpoint Adapter Integration V1

Status: source-staged reference. Tracks QUAL-EVID-006 / #218.

## Purpose

This contract removes the final public `verified: bool` current-head hop from the first Patient-v2 qualification vertical slice.

The integrated Rust crate consumes the exact identity emitted by QUAL-EVID-004/#217 and independently cross-checks it against the governed qualification, exact #208 profile, local qualification ledger, expected QUAL-EVID-003/#214 profile-match commitment, and current consumption time.

```text
#217 governed verified head
        +
#210/#214 governed qualification
        +
#212 exact #208 product profile
        +
local admissibility/current time
        ↓
Profile208CompositionCommitmentToken
```

## Exact #217 artifact vocabulary

The public `VerifiedQualificationHeadCheckpoint` binds:

- schema version `1`;
- report kind `mycelix-health-verified-qualification-head-checkpoint-v1`;
- exact semantic `QualificationLineageBinding`;
- lineage-binding digest;
- head receipt digest;
- head receipt sequence;
- checkpoint digest;
- signed checkpoint-bundle digest;
- checkpoint epoch;
- exact #214 profile-match commitment;
- lineage-state commitment;
- governance-policy digest;
- trust-store digest;
- deployment-evidence digest;
- checkpoint verification time;
- checkpoint expiry;
- verified-head report digest.

The type contains no `verified` flag.

Construction rejects unknown schema/report kind, zero head sequence, zero checkpoint epoch, and non-positive verification windows.

## Product-seam checks

Before conversion the integrated #208 gate requires:

- qualification kind is `CompositionCommitmentQualification`;
- checkpoint semantic lineage equals qualification lineage;
- checkpoint policy equals qualification policy;
- checkpoint trust store equals qualification trust store;
- checkpoint deployment equals qualification deployment;
- checkpoint receipt equals qualification receipt;
- checkpoint profile-match commitment equals the independently supplied expected #214 commitment;
- checkpoint verification time is not in the future;
- checkpoint has not expired.

The private QUAL-EVID-002 gate then independently rechecks:

- exact adapter-profile semantic lineage;
- exact profile policy/trust/deployment;
- local qualification-ledger binding;
- local receipt admissibility;
- receipt issuance/expiry;
- stricter profile consumption age;
- monotonic consumption time.

## Public API boundary

The original `src/lib.rs` remains source-staged as the QUAL-EVID-002 implementation and is compiled inside a private `legacy` module.

`Cargo.toml` now points the crate root to `src/integrated.rs`.

Therefore this symbol is not exported from the product-facing crate:

```text
VerifiedLineageHead::from_checkpoint_adapter(..., verified: bool)
```

The private call is used only after the stronger #217 artifact has passed all public integration checks.

## Token provenance

The resulting non-cloneable `Profile208CompositionCommitmentToken` retains:

- receipt/profile/semantic-lineage identity;
- policy/trust/deployment identity;
- checkpoint digest;
- lineage-binding digest;
- head sequence;
- checkpoint-bundle digest;
- checkpoint epoch;
- #214 profile-match commitment;
- lineage-state commitment;
- verified-head digest;
- head verification/expiry time;
- qualification issuance/expiry time.

This prevents downstream #208 composition from reducing current-head authority to an unexplained boolean or single opaque hash.

## Explicit trust boundary

This Rust integration does **not** independently verify #217's Ed25519 checkpoint signatures or durable checkpoint store. `from_qualified_checkpoint_report(...)` is the typed adapter boundary for a report already emitted by the independently qualified #217 verifier.

That is stronger than a naked boolean because the complete authority-bearing identity must match, but it remains an adapter boundary.

A future native verifier may move #217 verification into Rust without changing the #208 token theorem.

## Remaining rollback blocker

QUAL-EVID-005/#216 remains required before claiming whole-volume rollback resistance.

```text
exact #217 artifact consumption
!= external anti-rollback anchoring
```

## Non-claims

This subject does not establish:

- the truth of the underlying qualification theorem;
- checkpoint-signature verification inside Rust;
- external anti-rollback protection;
- trusted production clock correctness;
- production private-key custody;
- #208 durable activation;
- Patient-v2 activation;
- clinical validity;
- legal/regulatory compliance.
