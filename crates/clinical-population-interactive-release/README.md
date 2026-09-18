# mycelix-clinical-population-interactive-release

High-assurance authorization boundary for interactive differential-privacy population releases.

This crate does **not** generate DP noise and does not replace the population-release preflight. It composes three independently reviewed layers:

1. exact DP request/mechanism/output + protected accountant-receipt validation;
2. DNA-rooted trusted population-accountant states;
3. canonically admitted, bounded conflict-preserving accountant-state discovery.

Only after those layers agree does it mint `TrustedInteractivePopulationReleaseCapabilityV1`.

The capability is intentionally a distinct Rust type from the lower-level `PopulationReleaseCapabilityV1`. Production interactive release sinks should accept only the trusted capability.

See `docs/security/CLINICAL_POPULATION_INTERACTIVE_RELEASE_V1.md` and the qualification checklist for threat model and promotion requirements.
