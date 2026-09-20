# Qualification Checkpoint Adapter Integration V1

Status: source-staged reference. Tracks QUAL-EVID-006 / #218.

## Purpose

This contract removes the final public raw-currentness hop from the first Patient-v2 qualification vertical slice.

The final design uses two layers above the unchanged QUAL-EVID-002/#212 reference:

```text
#212 adapter-profile reference
        ↓
qualification-checkpoint-adapter-core
        ↓
qualification-product-authority-core
        ↓
exact #208 product token
```

The checkpoint-adapter layer binds the exact identity emitted by QUAL-EVID-004/#217 into the lower #212 semantics. The final product-authority layer additionally consumes the exact QUAL-EVID-003/#214 profile-match artifact, preventing a caller from supplying a bare "expected profile-match" digest.

## Final product theorem

```text
#214 governed profile-match artifact
        +
#217 governed current-head artifact
        +
#210 governed qualification
        +
#210 local receipt admissibility
        +
#212 exact profile/freshness semantics
        +
monotonic product-consumption time
        ↓
Profile208ProductAuthorityToken
```

Every term is independently required.

## Exact #214 artifact vocabulary

`VerifiedProfile208Match` binds:

- schema/report kind;
- exact `Patient208CompositionCommitment` seam identity;
- adapter-profile digest;
- qualification-bundle digest;
- exact qualification receipt;
- exact semantic `QualificationLineageBinding`;
- governance-policy digest;
- trust-store digest;
- deployment-evidence digest;
- profile-match time;
- exact domain-separated profile-match commitment.

The final gate requires this artifact to match the configured #208 profile and governed qualification exactly.

## Exact #217 artifact vocabulary

The internal integration type `VerifiedQualificationHeadCheckpoint` and final product wrapper `VerifiedCurrentHead208` bind:

- schema version `1`;
- report kind `mycelix-health-verified-qualification-head-checkpoint-v1`;
- exact semantic `QualificationLineageBinding`;
- lineage-binding digest;
- head receipt digest + sequence;
- checkpoint digest + signed checkpoint-bundle digest + epoch;
- exact #214 profile-match commitment;
- lineage-state commitment;
- governance-policy digest;
- trust-store digest;
- deployment-evidence digest;
- checkpoint verification time + expiry;
- verified-head report digest.

No type in the final product crate contains a `verified: bool` authority field.

## Cross-artifact checks

Before conversion, the final gate requires:

- profile artifact profile digest equals the configured #208 profile;
- profile semantic lineage equals the profile and qualification lineage;
- profile receipt equals the governed qualification receipt;
- profile policy/trust/deployment equal the qualification identities;
- profile-match time lies inside the qualification lifetime and is not in the future;
- head lineage/receipt/policy/trust/deployment equal the profile artifact;
- head profile-match commitment equals the exact #214 artifact commitment;
- checkpoint verification does not precede the profile match;
- head is current at consumption time.

The lower #218 checkpoint integration then requires the same head identity to match the governed qualification, and QUAL-EVID-002 independently rechecks profile binding, ledger admissibility, receipt freshness and monotonic consumption time.

## Public API boundary

`mycelix-qualification-adapter-profile-core` remains the unchanged QUAL-EVID-002 reference and still contains its explicitly non-production raw current-head adapter.

`mycelix-qualification-checkpoint-adapter-core` confines that lower adapter behind a complete #217 artifact check.

`mycelix-qualification-product-authority-core` is the intended product-facing API. It does not re-export the lower current-head type, takes no verification boolean, and takes no caller-selected bare expected profile-match digest.

## Token provenance

`Profile208ProductAuthorityToken` preserves:

- governed receipt/profile/semantic-lineage identity;
- policy/trust/deployment identity;
- qualification-bundle provenance;
- profile-match commitment + match time;
- checkpoint digest/bundle/epoch;
- lineage-state commitment;
- verified-head digest and validity window.

Downstream #208 composition therefore receives a typed authority result with explainable evidence ancestry rather than an unexplained `true` value.

## Explicit adapter boundary

The Rust wrappers do **not** independently verify #214/#217 Ed25519 signatures. Their `from_qualified_*_report(...)` constructors are typed adapter boundaries for strict reports already emitted by the independently governed Python/OpenSSL verifier stack.

A future native verifier may internalize those cryptographic checks without changing the final #208 token theorem.

## Remaining rollback blocker

QUAL-EVID-005/#216 remains required before claiming whole-volume rollback resistance.

```text
exact signed-artifact consumption
!= external anti-rollback anchoring
```

## Non-claims

This subject does not establish:

- the truth of the underlying qualification theorem;
- native Rust Ed25519 verification;
- external anti-rollback protection;
- trusted production clock correctness;
- production private-key custody;
- #208 durable activation;
- Patient-v2 activation;
- clinical validity;
- legal/regulatory compliance.
