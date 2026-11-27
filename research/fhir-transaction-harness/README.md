# HAPI FHIR R4 + PostgreSQL transaction probe harness

**Purpose:** run a real, persistent FHIR R4 server and observe transaction semantics. This is a test harness, not a production EHR or a complete Mycelix qualification.

## Pinned runtime

- HAPI JPA Starter tag `hapiproject/hapi:v8.12.0-2`, multi-platform index digest `sha256:0ff86aac4c7bd13a1ee226444a70263bb46ed1c28ef543c30a44f865b80ca04e`.
- PostgreSQL 18.7, multi-platform index digest `sha256:64c8b0d31e4ab6ea665e8426ea406a94bf52de3c05b3a4a8bf713704eefa9328`.
- HAPI is explicitly configured for FHIR R4 and PostgreSQL. The endpoint is bound to `127.0.0.1` only.

PostgreSQL 18+ uses the versioned data-directory layout (`/var/lib/postgresql/18/docker`); the Compose volume is therefore mounted at `/var/lib/postgresql`, not the pre-18 `/var/lib/postgresql/data` path. The digest pins are part of the experimental identity; do not replace them with `:latest`. The amd64 manifests are recorded in research issue #371.

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

1. Read `/metadata`, record the exact bytes' SHA-256, assert FHIR R4 version `4.0.1`, and require the server to declare transaction interaction plus conditional create for Patient and Provenance.
2. Submit a FHIR R4 transaction Bundle with a synthetic Patient using conditional create by `Patient.identifier`, and a linked Provenance whose `target` Reference carries both the transaction `fullUrl` and a synthetic `Reference.identifier`. The Provenance conditional create and follow-up search use the standard `target:identifier` reference modifier—not a non-existent Provenance `identifier` search parameter.
3. Replay the same transaction and check that exactly one Patient and one Provenance remain. Capture Patient version movement separately; this is a cardinality/replay test, not proof that the replay was a no-op or that distributed execution is exactly-once.
4. Run two concurrent Patient updates with the same `If-Match` version. Require one successful update and one HTTP 412 stale-version rejection.
5. Submit a transaction that first creates a rollback marker and then attempts an update with an impossible `If-Match` value. Require an OperationOutcome failure, verify the marker is absent, and verify the prior Patient version did not advance.

The follow-up reads are performed through the same HAPI FHIR service. They are not an independent database observer and must not be described as such. The generated evidence deliberately leaves overall qualification at `defer`.

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
