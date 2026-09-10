# Authority policy requirement-binding gap

Status: P0 cross-cutting security/assurance issue.

## Problem

The current `clinical-authority::AuthorityRequest` accepts these values independently:

- `policy_digest`;
- `purpose`;
- `jurisdiction`;
- `required_credential_kinds`;
- `required_scopes`.

`evaluate_authority` validates the supplied requirements but does not derive/recompute `policy_digest` from them. A caller that controls request construction could therefore pair an expected-looking policy digest with a weaker credential/scope requirement set unless an outer workflow reconstructs and verifies the exact policy.

## Immediate containment for clinical causality

The causality qualification layer must not accept a preconstructed `AuthorityPermit` as sufficient proof. It will own a typed `CausalAssessorAuthorityPolicyV1`, hash the exact allowed jurisdictions/credential requirements/scope requirements under the `AuthorityPolicy` domain, construct the authority request itself, and then invoke `evaluate_authority`.

## Required global fix

Introduce a canonical authority-policy artifact in `clinical-authority` and an evaluator API that derives the permit policy digest from that exact policy artifact. Deprecate/qualify the free-form request path for high-assurance workflows.

Add adversarial tests proving that changing credential kinds, scopes, purpose, or jurisdiction policy changes the policy identity and cannot be paired with an old digest.

A later migration should move medication activation, quarantine, dispensing, administration, and other high-assurance consumers onto the policy-bound API.
