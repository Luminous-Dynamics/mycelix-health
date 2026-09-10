# Authority Policy Binding v1

Status: experimental design contract. Source review is not runtime or clinical qualification.

## Problem

The legacy `AuthorityRequest` accepts `policy_digest`, `purpose`, `jurisdiction`, `required_credential_kinds`, and `required_scopes` as independently supplied values. `evaluate_authority` evaluates the requirements but does not prove that the supplied policy digest was derived from those exact requirements.

A high-assurance caller must therefore not treat an arbitrary `AuthorityPermit` plus an expected policy digest as proof that the intended policy requirements were enforced.

## V1 containment

Add a separate policy-bound authority layer rather than silently changing the legacy API.

A typed `AuthorityPolicyV1` contains the exact:

- purpose;
- jurisdiction;
- required credential kinds;
- required scopes;
- schema/version identity.

The policy is canonicalized and hashed under the domain-separated `AuthorityPolicy` digest domain. The policy-bound evaluator constructs the legacy `AuthorityRequest` internally from that policy and one exact target/principal/time. Callers cannot separately supply weaker requirement vectors alongside the policy digest.

The output `PolicyBoundAuthorityPermit` is non-cloneable and non-serializable and exposes the exact policy digest and the underlying decision evidence needed by downstream composition.

## Security invariant

`authority policy digest == hash(exact purpose + jurisdiction + credential requirements + scope requirements)`

for every policy-bound permit.

## Migration

Existing `AuthorityPermit` remains available for compatibility. High-assurance clinical workflows should migrate to `PolicyBoundAuthorityPermit` and eventually deprecate direct construction/evaluation through caller-supplied `AuthorityRequest`.

## Non-claims

This layer does not establish the external legitimacy of a credential issuer; that remains the issuer-trust boundary. It does not prove DHT provenance of credential/status evidence. It does not make a deserialized receipt into authority. It does not replace deployment-pinned policy selection; the caller/runtime must still choose the expected typed policy from a trusted configuration boundary.
