# Consent Authorization Policy Boundary v0.1

**Status:** source-level policy refactor and deterministic unit qualification. Not a live Holochain integration or clinical certification.

**Tracking:** [EQC production adapter issue #360](https://github.com/Luminous-Dynamics/mycelix-health/issues/360).

## Why the policy is split

Health authorization has three layers that must not be collapsed into one test:

1. **Pure consent matching** — deterministic matching of a consent against actor, category, permission and evaluation time.
2. **Stateful consent orchestration** — retrieving active consents, resolving patient-authored break-glass capabilities and writing emergency notifications.
3. **Shared zome enforcement** — patient self-access, Holochain call/decode handling, sensitive-category second checks, and fail-closed propagation into the data zomes.

The new `consent_integrity::consent_matches_authorization` function is layer 1. It is called by the production `consent::check_authorization` entrypoint and by the existing deterministic test simulator. That removes the old duplicate matching implementation without turning a unit test into a claim about Holochain networking or conductor behavior.

## Current pure predicate

The deterministic helper checks:

- consent status is Active;
- no revocation timestamp exists;
- the expiry is absent or strictly later than the evaluation timestamp (`expires_at == now` is denied);
- actor matches an Agent grantee, or the grantee is Public;
- category is covered by All or exact category;
- the category is not excluded;
- requested permission is present;
- the current legal-representative restriction for SubstanceAbuse and SexualHealth is preserved.

Provider and Organization grantees remain denied until provider-key resolution and organization-membership checks are implemented. The legacy EmergencyAccess enum does not authorize; break-glass remains a separate patient-authored capability path in the coordinator.

## Explicit boundary and known gaps

This helper deliberately preserves current behavior instead of silently expanding the policy:

- `ConsentScope.date_range` and `encounter_hashes` are not inputs to the current authorization request and are not enforced by this predicate. That requires a separate request-context and data-access contract; see [the scope-enforcement follow-up](https://github.com/Luminous-Dynamics/mycelix-health/issues/361).
- `ConsentGrantee::Public` currently matches a covered category and permission. The authorization request does not itself prove that returned data are anonymized. Public-grant use therefore needs a purpose/anonymization enforcement boundary; see [the public-consent follow-up](https://github.com/Luminous-Dynamics/mycelix-health/issues/362).
- The pure helper does not query Holochain, enumerate records, call other zomes, or emit access logs.
- The shared wrapper applies additional sensitive-category checks for MentalHealth, SubstanceAbuse and SexualHealth. A unit pass here does not qualify that second layer.
- Emergency access is checked separately against a patient-authored capability and notification path. A unit pass here does not qualify break-glass operations.
- Holochain transport, decode, authentication and countersigning errors must continue to fail closed.

The refactor is constrained to the existing predicate. Any change to the policy semantics needs a separate reviewed design and regression suite.

## Reproduction

```bash
cargo test --locked -p consent_integrity
cargo test --locked -p consent
```

The integration work that invokes a built DNA and the real `require_authorization` call path is tracked separately in issue #360. Synthetic fixtures only; no real patient data.
