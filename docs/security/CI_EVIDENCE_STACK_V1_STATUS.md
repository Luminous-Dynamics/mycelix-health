# CI Evidence Stack V1 — Staged Status

Exact purpose: freeze the review boundary for the off-PR CI evidence infrastructure before it is squashed into one commit over `main`.

The staged stack contains:

- pinned parent-source profile and fail-closed materializer;
- independent parent-source verifier;
- manual-only pinned reproducible CI lane;
- manual-only moving-parent compatibility lane;
- product-subject / qualification-harness separation;
- evidence isolation under `RUNNER_TEMP`;
- canonical final CI receipt builder;
- independent final CI receipt verifier;
- adversarial self-tests for all four evidence programs;
- security contracts for source identity, CI evidence classes, and subject/harness separation.

Authority progression:

```text
ExecutionDiagnosticsOnly
  -> source materialization receipt
  -> independent source verification
  -> locked build/lint/test gates
  -> canonical CI evidence receipt
  -> independent final CI evidence verification
```

Pinned terminal software-evidence authority:

`VerifiedReproducibleRepositoryCIEvidence`

Moving-parent terminal authority:

`VerifiedCompatibilityOnlyObservation`

`VerifiedCompatibilityOnlyObservation` can never satisfy pinned qualification or clinical promotion prerequisites.

This branch remains intentionally **off-PR** and its workflows are **manual-only**. Staging the source does not schedule GitHub Actions work.

Current evidence class for this branch:

`SOURCE_STAGED / STATICALLY_REVIEWED / UNEXECUTED / NOT_QUALIFIED`

The scripts' adversarial suites have been authored but have not received hosted-runner execution evidence in this lineage. Do not describe this infrastructure as CI-qualified until the exact harness itself is reviewed and its tests execute successfully.

This infrastructure is not part of the currently frozen clinical runtime-trust #139/#140 subject and does not repair P0 #141 by itself.

Critical non-claims: software CI evidence is not scientific/clinical validation, treatment authority, legal compliance, or regulatory approval.
