# mycelix-clinical-population-release

Experimental privacy-release preflight/qualification primitives for population evidence.

Supported v1 paths:

- fixed/pre-specified static reports with primary + complementary suppression and composition-review evidence;
- interactive differential-privacy releases with exact mechanism descriptors and chained accountant receipts.

This crate does **not** itself establish institutional trust in suppression review, DP implementations, or privacy-accountant state. See `docs/security/CLINICAL_POPULATION_RELEASE_V1.md` and the qualification checklist.
