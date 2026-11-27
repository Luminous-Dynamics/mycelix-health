# HAPI FHIR R4 + PostgreSQL transaction probe harness

**Purpose:** run a real, persistent FHIR R4 server and observe declared and active transaction behavior. This is a qualification harness, not a production EHR or a complete Mycelix qualification.

## Pinned runtime

- HAPI JPA Starter tag `hapiproject/hapi:v8.12.0-2`, multi-platform index digest `sha256:0ff86aac4c7bd13a1ee226444a70263bb46ed1c28ef543c30a44f865b80ca04e`.
- PostgreSQL 18.7, multi-platform index digest `sha256:64c8b0d31e4ab6ea665e8426ea406a94bf52de3c05b3a4a8bf713704eefa9328`.
- HAPI is explicitly configured for R4 and persistent PostgreSQL. Its port is bound to `127.0.0.1` only.

PostgreSQL 18+ uses the versioned data-directory layout (`/var/lib/postgresql/18/docker`); the Compose volume is mounted at `/var/lib/postgresql`. The image pins are part of the experimental identity; do not use floating tags.

## Run locally

Prerequisites: Docker Engine + Compose plugin, `curl`, `jq`, and `sha256sum`.

```bash
docker compose -f research/fhir-transaction-harness/compose.yaml up -d --wait
FHIR_BASE_URL=http://127.0.0.1:8080/fhir \
  bash research/fhir-transaction-harness/probe-transactions.sh
docker compose -f research/fhir-transaction-harness/compose.yaml down -v
```

The script records the exact CapabilityStatement, transaction/replay responses, stale-ETag and concurrent-writer responses, rollback OperationOutcome, and a machine-readable evidence summary. PostgreSQL init creates a separate SELECT-only `hapi_observer` role; the final probe queries `HFJ_RESOURCE`, `HFJ_RES_VER`, and `HFJ_SPIDX_TOKEN` directly and checks resource counts/versions plus the absent rollback marker. The observer's result is stored separately from HAPI API read/search results. The Compose bind mount is relative to this Compose file's directory, so the init SQL source is `./init-observer.sql`.

## What is probed

1. Read `/metadata`; hash the exact CapabilityStatement bytes; assert FHIR R4 (`4.0.1`); require transaction support and declared conditional-create/search capabilities for the resource types used.
2. Create a synthetic Patient anchor via the declared conditional-create interaction. Search it back through FHIR to obtain the server-assigned ID and current version.
3. Submit a two-entry FHIR transaction Bundle:
   - Create a Provenance resource with a normal `target=Patient/{id}` reference and conditional-create query.
   - Create a synthetic Observation bound to that Patient, using conditional create by `Observation.identifier`.
4. Replay the exact two-entry transaction. Assert exactly one matching Provenance and Observation remain. Check that the Observation version remains unchanged. This is in-server resource-cardinality/replay evidence, not distributed exactly-once.
5. Send a deterministic stale-ETag update and require an HTTP 409/412 version conflict plus an unchanged resource version, recording the exact status. The FHIR R4 version-aware update example describes 412 on ETag mismatch, while the HAPI 8.12.0 image under test returns 409 OperationOutcome for a stale ETag; the harness records the exact observed status and accepts the R4 conflict classes without disguising the server's behavior.
6. Run two concurrent Patient updates using the same valid `If-Match` version. Require one accepted update and one FHIR version-conflict response (HTTP 409/412). Preserve both raw responses and statuses before assertions; server-side 5xx/unique-index failures are evidence of a qualification failure, not a passing conflict.
7. Submit a transaction that creates a rollback-marker Patient and then attempts a Patient update with an impossible `If-Match`. Require an OperationOutcome failure, verify the marker is absent, and verify the original Patient version did not advance.
8. As a separate read-only PostgreSQL role, directly inspect HAPI master/version rows and the rollback marker's token index. Require the DB counts/versions to agree with the API observations.

The race probe intentionally distinguishes a deterministic stale precondition from two truly overlapping updates: a single 409/412 conflict for the former does not prove the database can serialize the latter. Post-state is checked twice: through the FHIR API and independently through PostgreSQL using a SELECT-only `hapi_observer` role. The SQL probe verifies exactly one master row and one history version for Provenance/Observation, Patient history at version 2, Observation version 1, and no token-index row for the rolled-back Patient identifier. The observer has no INSERT/UPDATE/DELETE privileges and runs with `default_transaction_read_only=on`. It still shares the same PostgreSQL instance as HAPI; it is independent of the FHIR API, not an independent physical database or third-party auditor.

## Interpretation

A probe pass establishes only the behavior observed for this exact image, configuration, test corpus and runtime. It does not qualify Mycelix's consent/revocation fence, gateway operation ledger, cross-system recovery or remote recipient effect. The generated evidence deliberately leaves overall qualification at `defer`.

The earlier `target:identifier` conditional query was rejected by this HAPI image during transaction parsing (`HAPI-1250`). The current harness avoids that modifier and uses the ordinary declared Provenance `target` search against the server-assigned Patient id.

## Limitations and safety

The upstream HAPI JPA starter does not provide a production authentication/authorization implementation or enterprise audit configuration by default. This stack is loopback-only and synthetic-data-only; do not expose it to a network or use real patient data.

FHIR transaction atomicity is local to one supporting FHIR server. It does not establish atomic commit across Mycelix identity, consent, gateway, outbox, a second EHR or a recipient. Not yet tested here:
- consent revocation at the durable commit boundary;
- Mycelix operation-ledger recovery after crashes/lost acknowledgements;
- independent recipient-side durable effect witness;
- independently signed runtime qualification receipt;
- SMART/OAuth, access control, enterprise audit, or privacy/legal compliance.

No PHI, clinical-safety, HIPAA / 42 CFR Part 2, regulatory-certification, production-readiness or distributed exactly-once claim is made.

## Industry conformance baseline

Run the applicable Inferno US Core Test Kit separately for the supported profile claims. Inferno conformance does not replace operational transaction probes; these probes do not replace Inferno conformance.
