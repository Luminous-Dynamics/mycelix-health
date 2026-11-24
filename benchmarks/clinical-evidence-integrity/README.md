# Clinical Evidence Integrity Benchmark — FHIR Subject Binding v1

This is the first executable slice of the wider programme in Health issue #288.

## Scope

This frozen, fully synthetic corpus evaluates the FHIR bridge's single-patient
subject-binding boundary. It contains 16 deterministic positive and negative
controls covering relative, absolute, URN and version-specific references;
source/target Bundle identity disagreement; duplicate fullUrls; wrong patients;
missing subjects; malformed references; wrong resource type; and multi-patient
Bundles.

It tests the coordinator's pre-ingest identity policy, not clinical correctness,
de-identification, real EHR interoperability, deployed Holochain authorization,
or diagnostic performance.

## Focused host-runner qualification

The full workspace CI includes unrelated TypeScript, cargo-deny and integration
jobs. To keep the FHIR evidence contracts independently executable, the
dedicated FHIR Evidence Contract Benchmarks workflow runs only the FHIR
coordinator's subject-binding, temporal-fidelity and Bundle-preflight corpus
tests on the host target.

Run the same focused suite locally from the repository root:

    ./scripts/run-clinical-evidence-fhir-corpora.sh

The script prints the exact source commit, Rust version, host target and SHA-256
of each frozen corpus before running all tests whose name ends in v1_corpus.
This signal is separate from full workspace qualification and Holochain
conductor Sweettests.

## FHIR Bundle preflight v1

The coordinator's declared ingestion profile is limited to Bundle.type =
collection. The 24-case synthetic corpus covers valid URN/RESTful entries, distinct versions
sharing one version-independent fullUrl, and malformed envelopes including
missing fullUrl, duplicate (fullUrl, meta.versionId), missing resource fields,
URL/resource identity disagreement, unsupported Bundle types, and forbidden
request/response/search metadata.

Run the preflight corpus from the repository root:

```bash
./scripts/run-fhir-bundle-preflight-benchmark.sh
```

The preflight is invoked before Patient or clinical-resource writes. A rejected
envelope may persist only an unlinked rejection IngestReport. The Sweettest for
a malformed collection checks that no Patient writes are reported.

## Run

From the repository root in the supported Mycelix-Health submodule workspace:

\`\`\`bash
./scripts/run-fhir-subject-binding-benchmark.sh
\`\`\`

The script invokes only the named pure-Rust corpus test and explicitly targets
the host triple. The test reads the checked-in JSON corpus with
\`include_str!\`; it does not fetch data or call a model.

Equivalent command:

\`\`\`bash
HOST="$(rustc -vV | sed -n 's/^host: //p')"
cargo test -p fhir_bridge --target "$HOST" fhir_subject_binding_v1_corpus
\`\`\`

## Reporting

The test asserts every case's expected subject-binding result and coordinator
accept/reject outcome. It also asserts minimum positive and negative control
counts so a vacuous all-reject implementation cannot pass.

For a published benchmark run, record:
- exact repository commit SHA;
- SHA-256 of this corpus file;
- Rust compiler version and target triple;
- test command and exit code;
- total cases, accepts, rejects and per-case output;
- whether the full Holochain Sweettest suite ran separately.

The benchmark corpus is immutable within this release. Any case, policy or
expected-outcome change requires a new versioned corpus ID and manifest.

## Nonclaims

A pass demonstrates only that the tested resolver and coordinator policy match
this finite synthetic fixture set. It does not establish complete FHIR R4
conformance, SMART security, issuer authenticity, patient matching across
institutions, clinical validity, privacy, production readiness or regulatory
compliance.


## Observation temporal fidelity v1

The temporal suite is a separate fixture corpus:
\`observation-temporal-fidelity-v1.json\`.

It evaluates independent event and issue times, timezone normalization,
microsecond exactness, absent issue time, and fail-closed treatment of
unsupported \`effective[x]\` choices and lossy precision. The pure helper and
corpus test are part of the FHIR coordinator unit suite.

Run both first slices from the repository root:

\`\`\`bash
HOST="$(rustc -vV | sed -n 's/^host: //p')"
cargo test -p fhir_bridge --target "$HOST" fhir_subject_binding_v1_corpus
cargo test -p fhir_bridge --target "$HOST" observation_temporal_fidelity_v1_corpus
\`\`\`

A successful test result demonstrates only exact behavior for these finite,
synthetic fixtures. It does not establish temporal correctness for every FHIR
date/time primitive, clinical validity or longitudinal reasoning quality.
