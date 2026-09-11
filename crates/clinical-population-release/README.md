# mycelix-clinical-population-release

Experimental privacy-release preflight/qualification primitives for population evidence.

Supported v1 paths:

- fixed/pre-specified static reports with primary + complementary suppression and composition-review evidence;
- interactive differential-privacy **preflight validation** with exact mechanism descriptors and chained accountant receipts.

Authority is intentionally split:

- `authorize_static_population_release()` may mint the base `PopulationReleaseCapabilityV1` only for static pre-specified reports;
- interactive DP requests may validate here, but this crate refuses to mint base release authority for them;
- production interactive release must use the separate trusted canonical-accountant gate (`mycelix-clinical-population-interactive-release`).

The compatibility function `authorize_population_release()` is also static-only and returns `InteractiveReleaseRequiresTrustedGate` for interactive requests. There is no compatibility conversion from the base capability to trusted interactive authority.

This crate does **not** itself establish institutional trust in suppression review, DP implementations, or privacy-accountant state. See `docs/security/CLINICAL_POPULATION_RELEASE_V1.md` and the qualification checklist.
