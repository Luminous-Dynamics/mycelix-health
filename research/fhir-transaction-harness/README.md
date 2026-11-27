# HAPI FHIR R4 + PostgreSQL transaction probe harness

**Purpose:** run a real, persistent FHIR R4 server and observe transaction semantics. This is a test harness, not a production EHR or a complete Mycelix qualification.

## Pinned runtime

- HAPI JPA Starter tag `hapiproject/hapi:v8.12.0-2`, multi-platform index digest `sha256:0ff86aac4c7bd13a1ee226444a70263bb46ed1c28ef543c30a44f865b80ca04e`.
- PostgreSQL 18.7, multi-platform index digest `sha256:64c8b0d31e4ab6ea665e8426ea406a94bf52de3c05b3a4a8bf713704eefa9328`.
- HAPI is explicitly configured for FHIR R4 and PostgreSQL. The endpoint is bound to `127.0.0.1` only.

The digest pins are part of the experimental identity; do not replace them with `:latest`. The amd64 manifests are recorded in research issue #371.

## Run locally

Prerequisites: Docker Engine with the Compose plugin, `curl`, `jq`, and `sha256sum`.

```bash
docker compose -f research/fhir-transaction-harness/compose.yaml up -d --wait
FHIR_BASE_URL=http://127.0.0.1:8080/fhir \
  bash research/fhir-transaction-harness/probe-transactions.sh
docker compose -f research/fhir-transaction-harness/compose.yaml down -v
```

The script writes raw CapabilityStatement, request/response Bundles and a machine-readable evidence summary under `./artifacts/fhir-qualification-<UTC timestamp>/`. Evidence includes the CapabilityStatement digest and the observed HTTP/pre-post state.

## What is probed

1. Read `/metadata`, record the exact bytes' SHA-256, assert FHIR R4 version `4.0.1`, and confirm that transaction interaction is declared.
2. Submit a transaction Bundle containing a synthetic Patient and linked Provenance.
3. Replay the same Bundle with conditional create and assert exactly one Patient and one Provenance remain.
4. Cause an `If-Match` version precondition failure inside a transaction after a preceding create; independently search the database through the FHIR read API and assert the first create rolled back and the pre-existing Patient version did not change.

A passed probe means only that this specific image/configuration behaved as observed for these synthetic fixtures. The generated evidence deliberately says overall `defer`.

## Important limitations

The upstream HAPI JPA starter explicitly ships without a security implementation and without enterprise logging by default. This Compose stack therefore binds to loopback, uses synthetic test data only, and must never be exposed to a network or given real patient data.

FHIR transaction atomicity applies inside one supporting FHIR server. It does not establish atomic commit across Mycelix identity, consent, gateway, outbox, another EHR, or a recipient. The local probe also does not exercise:
- consent-revocation races at the durable commit boundary;
- Mycelix idempotency-ledger crash recovery or unknown outcomes;
- recipient-side durable effect observations;
- independent component qualification, key registry or cryptographic signatures;
- SMART/OAuth authorization, audit deployment, or legal/privacy compliance.

Those remain separate qualification obligations in #356, #365 and #371. No PHI, clinical-safety, HIPAA/42 CFR Part 2, regulatory-certification or production-readiness claim is made.

## Industry conformance baseline

Run the official Inferno US Core Test Kit separately for the applicable implementation guide/profile claims. Inferno conformance results do not replace these operational transaction probes; transaction probes do not replace Inferno conformance tests.
