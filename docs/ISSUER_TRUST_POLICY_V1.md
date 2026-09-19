# Issuer Trust Policy v1

## Purpose

Health credential author-binding proves which agent issued a credential. It does not prove that the issuer was authorized to issue that credential kind.

`mycelix-issuer-trust` defines the missing trust-root and delegation semantics for credential issuers.

## Central invariant

> No issuer may become trusted solely because it says it is trusted or because it signs its own credential/grant.

A successful issuer-authority decision must trace to an explicit trust anchor under one exact policy digest.

## Trust anchors

A `TrustAnchor` binds:

- issuer principal binding;
- allowed credential kinds;
- exact jurisdictions;
- validity window;
- maximum delegation depth;
- exact trust-policy digest;
- anchor evidence digest.

Trust anchors should come from an independently governed/configured policy source. They are not learned from the candidate issuer's own credential record.

## Verified delegation grants

`VerifiedDelegationGrant` is intentionally not serializable. A trusted Holochain/external adapter must first verify:

- grant record integrity;
- grantor author/signature binding;
- grantee identity;
- current revocation/status evidence;
- validity window.

Only then is the grant admitted to trust-path evaluation.

A grant binds:

- grantor and grantee;
- allowed credential kinds;
- exact jurisdictions;
- validity/revocation state;
- bounded child delegation depth;
- grant-record digest;
- status-check evidence digest.

Self-delegation is rejected.

## Bounded delegation

Delegation depth is monotonically decreasing.

If a parent has remaining depth `N`, a child grant may confer at most `N - 1` additional delegation hops.

The evaluator caps maximum policy depth at 8. This prevents accidental/unbounded transitive trust and constrains graph traversal.

Example:

`root(depth=2) -> state board(depth=1) -> delegated board service(depth=0)`

The depth-0 issuer may issue the allowed credential but cannot delegate issuer authority onward.

## Exact scope

v1 has no wildcard behavior.

Every trust hop must allow the requested:

- credential kind; and
- jurisdiction.

A California-only delegation cannot authorize Texas practitioner licenses. A research-ethics delegation cannot authorize practitioner licenses.

## Temporal and revocation semantics

Trust anchors and grants are evaluated at one exact time.

Expired, not-yet-valid, and revoked grants do not form a trust path.

Historical evaluation can therefore preserve the distinction between an issuer that was authorized at the time of issuance and one whose authority was revoked later.

## Cycle resistance

The evaluator tracks visited `(principal, remaining_depth)` states. Cyclic grants that have no path to a valid trust anchor cannot bootstrap trust.

A graph such as:

`A -> B -> A`

has no authority unless one of those states is independently reachable from an applicable anchor under the same policy.

## Determinism and resource bounds

Applicable anchors and candidate grants are ordered by evidence digest before traversal so equivalent input sets produce a stable selected path.

v1 caps:

- trust anchors: 64;
- delegation grants: 4096;
- delegation depth: 8.

This keeps issuer verification bounded even when evaluating hostile or malformed trust graphs.

## Receipt

Successful evaluation returns a non-serializable, non-cloneable `IssuerAuthorityReceipt` bound to:

- exact issuer;
- credential kind;
- jurisdiction;
- evaluation time;
- trust-policy digest;
- selected path length;
- ordered evidence digests for the anchor and every delegation/status hop.

A practitioner-credential adapter can use this receipt as evidence that the credential issuer had an authorized trust path. The clinical authority layer then separately verifies the practitioner's holder binding, credential status, jurisdiction, and action scope.

## Separation of powers

The intended chain is:

`trust policy/root`
→ `issuer authority receipt`
→ `verified practitioner credential`
→ `clinical authority evaluation`
→ `single-use action permit`

No layer is allowed to self-assert the output of the previous one.

## Current integration gap

The current health credentials zome does not yet contain typed issuer-trust grants/anchors. This crate is therefore the pure policy engine first.

A follow-on Holochain tranche should define author-bound trust-policy/grant entries and immutable revocation/supersession semantics rather than embedding trust decisions inside generic credential JSON.

## Non-goals

v1 does not:

- decide which real-world boards/regulators are legitimate;
- fetch government licensing registries;
- infer trust from a DID alone;
- permit unbounded delegation;
- permit wildcard jurisdiction/credential-kind authority;
- replace jurisdiction-specific legal policy;
- make the chosen trust-anchor governance automatically correct.

## Next

1. Define Holochain trust-anchor/delegation/revocation entries with strict author binding.
2. Require grantor authority at DHT validation where feasible, not only coordinator code.
3. Bind trust-policy version/supersession to exact predecessor lineage.
4. Build issuer trust receipts into `ResolvedCredentialEvidence` construction.
5. Add external-registry adapters for jurisdictions where authoritative licensing sources exist.
6. Add independent policy review/audit for root-anchor changes because root compromise is the highest-impact trust event.
