# Official HL7 FHIR R4 Core Validation — v1

This lane validates a small synthetic Bundle corpus using the HL7-maintained FHIR Validator CLI. It is independent of the Mycelix Rust Bundle preflight.

## Pinned toolchain

- Validator CLI: 7.0.6
- Artifact: `validator_cli.jar`
- Release: https://github.com/hapifhir/org.hl7.fhir.core/releases/tag/7.0.6
- SHA-256: `2686590539f0b3ba7683149b25d78249f6886c04e2c326e884c5049d8947df89`
- Java: 17
- FHIR core version: 4.0.1
- Terminology server: `n/a`

The lock file is `fhir-validator-lock-v1.json`. The runner verifies the exact artifact hash before execution and emits a separate OperationOutcome and console transcript for every fixture.

## Run

From the repository root:

```bash
bash ./scripts/run-official-fhir-r4-validator.sh
```

The script requires Java 17, `curl`, `jq` and `sha256sum`. The validator artifact is cached under `~/.cache/mycelix-health/fhir-validator/7.0.6` by CI.

## Why this is separate from the bridge benchmark

FHIR R4 Bundle rules and the Mycelix ingestion profile are different contracts.

Examples:
- A Collection entry without a fullUrl violates the Bundle identity rules, even though Bundle.entry.fullUrl has element cardinality 0..1.
- A transaction POST entry without fullUrl may be valid when the resource has no persistent identity.
- A zero-entry Collection is represented by omitting `entry`; explicitly encoding `"entry": []` is invalid FHIR JSON because empty arrays are not serialized for empty repeating elements.
- A Bundle can be valid FHIR R4 but unsupported by this ingestion endpoint, which currently accepts only a deliberately narrow Collection profile.
- Two entries can share the same fullUrl when their resource meta.versionId values are distinct; the unversioned reference to multiple versions remains potentially ambiguous.
- FHIR core validation does not prove that the embedded Patient is the intended patient, that the issuer is authenticated, that the caller is authorized, or that a clinical assertion is true.

The Rust preflight therefore enforces a stricter application profile and fail-closed patient binding. This official validator lane checks the base R4 4.0.1 core structure/invariants only. No US Core, IPS, SMART, clinical terminology, Holochain authorization, privacy or production interoperability conformance is claimed.

## Metrics and evidence

The runner records:
- validator release and artifact hash;
- Java and FHIR versions;
- corpus SHA-256;
- expected valid/invalid result per case;
- error/fatal issue count from each OperationOutcome;
- per-case console output and OperationOutcome JSON;
- total cases processed.

A passing lane demonstrates only that the selected synthetic fixtures are classified as expected by the pinned validator. It is not a full FHIR conformance statement for the bridge or for any production data.
