# Qualification Verifier Token Core

This crate is the production-facing type boundary for the first Mycelix Health qualification anti-rollback vertical slice.

The lower QUAL-EVID crates use public `Verified*` traits as semantic reference/test seams. A downstream crate can implement such a trait for its own type, so trait implementation alone is not cryptographic or governance authority.

This crate changes the product-facing shape to:

```text
qualified verifier
    -> private-field concrete token
    -> exact #222 semantic mapping
    -> verifier-bound Health anchor proof
    -> verifier-bound reconciliation
    -> verifier-bound anchored head
    -> verifier-bound #208 product gate
```

## V1 safety property

The three semantic inputs are concrete private-field types:

- `VerifiedGenericAcceptedAnchorTokenV1`;
- `VerifiedGenericStateCommitmentTokenV1`;
- `VerifiedHealthSecurityTimeTokenV1`.

They expose no public constructor or deserializer. Ordinary downstream safe Rust cannot construct them with a struct literal because their fields are private.

The crate also wraps #220 outputs in `VerifierBoundHealthAnchorProofV1` and `VerifierAnchoredQualificationHead`, so a head obtained by calling the lower open reference seam with a caller-defined fake proof cannot simply be relabeled as verifier-bound authority.

## Deliberate fail-closed limitation

There is **no production mint path in V1**.

Tests can construct tokens from inside the crate to prove the semantic chain, but no public API can mint them. A later child must integrate actual qualified verifier logic into this crate (or an authority module owned by it) before the product path can be available in production.

That is intentional:

```text
verifier unavailable
    -> no token
    -> no authority
```

not:

```text
verifier unavailable
    -> caller implements Verified trait
    -> authority
```

## Remaining authority work

This tranche closes only the generic accepted-anchor / generic commitment / Health security-time self-assertion path. Earlier qualification/profile artifacts still require the broader audit tracked by Health #223.

Tracks Health #223, #222/#221, #220/#216, and Luminous generic transparency #1962/#2038.
