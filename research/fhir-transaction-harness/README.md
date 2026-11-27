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

The script records the exact CapabilityStatement, transaction/replay responses, stale-ETag and concurrent-writer responses, rollback OperationOutcome, and a machine-readable evidence summary. It hashes the checked-in probe-corpus-v1.json, which defines all eight active probes and their expected outcomes; the corpus digest is included in qualification-evidence.json and the workflow harness manifest. PostgreSQL init creates a separate SELECT-only `hapi_observer` role; the final probe queries `HFJ_RESOURCE`, `HFJ_RES_VER`, and `HFJ_SPIDX_TOKEN` directly and checks resource counts/versions plus the absent rollback marker. The observer's result is stored separately from HAPI API read/search results. The Compose bind mount is relative to this Compose file's directory, so the init SQL source is `./init-observer.sql`.

## What is probed

1. Read `/metadata`; hash the exact CapabilityStatement bytes; assert FHIR R4 (`4.0.1`); require transaction support and declared conditional-create/search capabilities.
2. Create one synthetic Patient anchor via conditional create and read it back to obtain the server-assigned ID/version.
3. Submit 128 concurrent conditional-create requests using the same synthetic Patient identifier. Require exactly one HTTP 201 creator and no unexpected statuses. A pinned HAPI 8.12.0 run exposed two transient HTTP 409 OperationOutcomes from a duplicate `HFJ_RES_SEARCH_URL` unique-key collision; the revised harness preserves these responses and retries **only** that exact diagnostic using the identical conditional-create request.
4. Require every qualifying 409 retry to return HTTP 200 and the exact resource ID created by the original winner. Then assert one matching resource via FHIR and one master/history/version-1 row through the read-only PostgreSQL observer. Unknown 409 diagnostics, retry failure, ID mismatch or duplicate rows fail the probe. Initial 409 rate is reported separately rather than hidden.
5. Submit a two-entry FHIR transaction Bundle: create Provenance targeting the pre-created Patient and a linked synthetic Observation.
6. Replay the exact transaction; assert one Provenance and Observation remain and Observation version is unchanged.
7. Send a deterministic stale-ETag update; require the observed FHIR version-conflict response (HAPI returns 409 OperationOutcome) and unchanged resource version.
8. Run two truly concurrent Patient updates from the same valid ETag; require one accepted write and one 409/412 version conflict, with exactly one version increment.
9. Submit a transaction that creates a rollback-marker Patient then attempts an update with impossible If-Match. Require OperationOutcome failure, no marker, and unchanged existing resource version.
10. Use a separate read-only PostgreSQL role to inspect HAPI master/history rows, versions and token-index state; assert role privileges and agreement with API-level effects.

The race probe distinguishes a normal conflict from an internal contention collision. A 409 only counts as recovered when its diagnostic matches the known unique-index collision and the same conditional operation resolves to the already-created resource. This does not convert the first-attempt 409 into a clean success: it remains separately reported and is a target for reducing/ eliminating in later database/server versions.

Post-state is checked through both FHIR and SQL. The observer is independent of the FHIR HTTP API but shares the same PostgreSQL instance, not an independent replica or third-party auditor.
## Direct database observer debugging history

- HAPI-only run #25 passed the FHIR/API probe set: multi-resource transaction, conditional replay, deterministic stale-ETag conflict, concurrent version conflict, and transaction rollback.
- First direct-SQL attempt #33 reached the observer query but failed because the script assumed HFJ_RESOURCE.PID. The live HAPI 8.12.0 schema does not expose that column; the relationships use HFJ_RESOURCE.RES_ID. The current probe records the live information_schema column inventory, requires the needed columns, and joins HFJ_RES_VER.RES_ID / HFJ_SPIDX_TOKEN.RES_ID to HFJ_RESOURCE.RES_ID.
- The next attempt exposed a shell quoting defect in the combined schema-discovery/observer command before services started. These commands are now separate shell operations: first export the schema inventory, then execute the observer SQL. The current exact-head workflow is the first run of that corrected shell structure.
- Until that run completes, direct PostgreSQL observation is not qualified. The earlier HAPI-only #25 result remains valid for its FHIR API probes, but must not be presented as proof of direct SQL read-back.

## Direct database observer debugging history

- HAPI-only run #25 passed the FHIR/API probe set: multi-resource transaction, conditional replay, deterministic stale-ETag conflict, concurrent version conflict, and transaction rollback. This remains the last fully green HAPI-only baseline.
- Direct-SQL run #33 reached the observer query but failed because the script assumed HFJ_RESOURCE.PID. The pinned HAPI 8.12.0 schema does not expose that column; the resource joins use HFJ_RESOURCE.RES_ID. The current script uses RES_ID and records/validates the exact live information_schema column inventory before joining.
- Run #36 failed at the shell syntax preflight before containers started. The schema-discovery psql call and the JSON observer psql call are now separate shell commands; no nested command substitution is used for the schema inventory.
- The newest exact-head run must pass bash -n, then prove the catalog query, SELECT-only role, row/history counts, and rollback absence. Until that happens, direct PostgreSQL observation is not qualified.

## Frozen direct-database result

Runs #45 and #47's read-only PostgreSQL observations are preserved at `evidence/run-45-postgresql-observed.json` and `evidence/run-47-postgresql-observed.json`. It binds the workflow run/artifact IDs, exact source commit, HAPI/PostgreSQL runtime image IDs, PostgreSQL 18.7 version, CapabilityStatement digest, harness file digests, FHIR probe outcomes and direct database row/version counts.

The snapshot reports:
- one Patient master row and two history versions, current version 2;
- one Provenance row/history version;
- one Observation row/history version, current version 1;
- zero rollback-marker token-index rows;
- observer role `hapi_observer`, read-only session, not superuser, no role creation, INSERT, UPDATE or DELETE privileges.

Both snapshots are frozen experimental observations, not permanent certifications. Run #47 repeats the same FHIR and database invariants with the concurrent write winners reversed (A=409, B=200); that is useful evidence that the invariant is independent of which writer wins. Re-execute for changes to HAPI/PostgreSQL versions, Compose configuration, probe code or schema.

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
