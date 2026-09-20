# Qualification Receipt Ed25519 Verifier v1

Status: source-staged reference for QUAL-EVID-003 (#213).

## Purpose

Provide the first real cryptographic/governance verifier behind the governed qualification semantics from #210 and the product adapter profile from #212.

The layering is deliberately:

```text
Rust-compatible qualification statement
        ↓
Ed25519 governed verification (#213)
        ↓
exact product-profile match (#213 bridge)
        ↓
current-head + consumption-time gate (#212)
        ↓
typed #208 composition token
```

No earlier stage may claim the authority of a later stage.

## Canonical statement

`scripts/qualification-receipt.py` reproduces the exact binary transcript frozen by `qualification-receipt-core` under:

`MYCELIX-HEALTH-QUALIFICATION-RECEIPT-V1\0`

A committed golden vector binds a 394-byte `CompositionCommitmentQualification` statement to SHA-256:

`98621a93db931616186ae1f8c6d1788baa6f29da9b2e00b0931132d07f526389`

The Python representation is not signed as ambiguous JSON. The signature path always canonicalizes back into the exact binary transcript first.

## Attestation signature

Each approver signs a second domain-separated transcript:

`MYCELIX-HEALTH-QUALIFICATION-ATTESTATION-V1\0`

binding:

- SHA-256 of the exact canonical qualification transcript;
- approver ID;
- signer-key ID;
- attestation timestamp;
- full canonical qualification transcript.

This prevents an attacker from changing the freshness-bearing `attested_at_micros` after the signature was created.

## Governance

The generic verifier enforces:

- finite receipt-kind registry;
- finite admission-scope registry;
- exact policy digest and trust-store digest carried by the signed statement;
- required deployment evidence where policy demands it;
- bounded statement lifetime;
- statement currentness at verification;
- distinct approvers and signer keys;
- signer trust-store membership;
- active/non-revoked/non-compromised signer at attestation time;
- attestation inside statement window;
- no future attestation;
- maximum attestation age;
- minimum approver threshold;
- minimum organization diversity;
- required role coverage;
- real Ed25519 verification using OpenSSL.

## Generic evidence != product authority

The generic policy deliberately does not duplicate every product-semantic field frozen in #212.

For example, two authorized approvers can sign a structurally valid `CompositionCommitmentQualification` naming the wrong #208 contract. That evidence can be authentic while still being unusable by #208.

`scripts/qualification-receipt-profile.py` therefore independently matches an already verified bundle against the exact `Patient208CompositionCommitment` profile:

- receipt kind;
- contract digest/version;
- qualification lineage;
- subject;
- context;
- admission scope;
- claim profile;
- governance policy;
- trust store;
- deployment evidence;
- stricter consumption age.

Its output explicitly states:

- current lineage head is **not established**;
- product seam authority is **not established**.

Those remaining authority checks belong to #212's typed gate.

## Bundle reconstruction

An assembled qualification bundle contains the exact statement, statement digest, verified approvals, verified roles/organizations, policy/trust-store digests, verification time, bundle ID/digest, and bounded claims.

`verify` reconstructs the complete bundle from the embedded signatures plus independently supplied policy/trust store and requires object equality. Mutation of supposedly verified derived fields therefore fails.

## Key handling

Private approval keys are never committed.

The qualification workflow generates ephemeral Ed25519 test keys in the runner. Production/private governance keys are expected to remain in an offline or separately controlled signing environment, following the existing release/remediation evidence pattern.

## Replay boundary

This verifier authenticates one governed receipt bundle. It does not itself persist nonce consumption, monotonic lineage heads, or global ordering.

Those remain separate theorems in #210/#212 and future durable checkpoint integration.

## Non-claims

A successful v1 verification does not prove:

- truth of the underlying security/scientific/clinical theorem;
- current-head status for the qualification lineage;
- product-seam authority;
- hardware-backed key custody;
- post-quantum signatures;
- public transparency-log inclusion;
- global consensus ordering;
- legal or regulatory compliance.
