# HAPI FHIR R4 + PostgreSQL transaction assurance harness

**Purpose:** actively qualify one pinned FHIR R4 server/database configuration against a reproducible synthetic transaction corpus. This is a behavioral probe harness, not a production EHR, a Mycelix adapter qualification, or a clinical-safety certification.

## Pinned runtime

- HAPI JPA Starter v8.12.0-2, multi-platform image-index digest `sha256:0ff86aac4c7bd13a1ee226444a70263bb46ed1c28ef543c30a44f865b80ca04e`.
- PostgreSQL 18.7, multi-platform image-index digest `sha256:64c8b0d31e4ab6ea665e8426ea406a94bf52de3c05b3a4a8bf713704eefa9328`.
- Explicit FHIR R4 configuration and persistent PostgreSQL volume.
- HAPI binds to `127.0.0.1` only. The upstream starter does not provide production authentication/authorization or enterprise audit defaults.

The Compose file mounts `./init-observer.sql` relative to its own directory. That script creates a separate SELECT-only `hapi_observer` role; it does not grant the observer write privileges.

## Run locally

Prerequisites: Docker Engine + Compose plugin, `curl`, `jq`, `sha256sum`.

```bash
docker compose -f research/fhir-transaction-harness/compose.yaml up -d --wait
FHIR_BASE_URL=http://127.0.0.1:8080/fhir \
  bash research/fhir-transaction-harness/probe-transactions.sh
docker compose -f research/fhir-transaction-harness/compose.yaml down -v
```

The versioned `probe-corpus-v1.json` defines eight active probes and is hashed into `qualification-evidence.json`. The workflow fingerprints the Compose file, observer init SQL, corpus and harness script. Every test uses per-run synthetic identifiers and synthetic data; no PHI is used.

## Active probe contract

1. **Capability discovery.** Read the live `/metadata`, store/hash the exact CapabilityStatement bytes, assert FHIR R4 `4.0.1`, and require declared transaction, conditional-create and search capabilities for the interactions used. This is declared capability evidence, not proof of runtime behavior.

2. **Synthetic Patient anchor.** Conditionally create one synthetic Patient and search it back to obtain its server-assigned ID and current version.

3. **128-way conditional-create contention.** Launch 128 requests for the same synthetic Patient identifier, in batches of up to 32 parallel requests. Require exactly one HTTP 201 creator and no unclassified status. Preserve every original request response and status.

   **Observed HAPI 8.12.0 behavior:** the first 128-way run (#55, workflow 46045883432) returned 1 HTTP 201, 125 HTTP 200 and 2 HTTP 409 OperationOutcomes. The two 409 diagnostics both reported HAPI-0550 / PostgreSQL duplicate-key violation on `hfj_res_search_url_pkey` for the same conditional search URL. All 126 successful response bodies named the same Patient ID (`1001`). The old harness stopped at its strict 1/127/0 assertion, before it performed the final FHIR search or direct SQL observation. This is frozen at `evidence/run-55-conditional-create-contention.json` with the original workflow artifact digest. It is **not** a passing result and does not prove final database cardinality.

   **Recovery contract under test now:** retry only a 409 OperationOutcome whose diagnostic contains the exact `hfj_res_search_url_pkey` constraint, using the identical conditional-create body and `If-None-Exist` identifier. Require HTTP 200 and the exact same Patient ID returned by the original HTTP 201. Any unknown 409, any other status, retry failure or resource-ID mismatch fails closed. Report initial 409 rate separately; do not hide transient contention errors.

4. **FHIR transaction and replay.** Submit a two-entry transaction Bundle: Provenance with a standard `target=Patient/{id}` reference and a linked synthetic Observation. Replay the exact Bundle. Require one Provenance, one Observation and unchanged Observation version. This is in-server transaction/replay evidence, not distributed exactly-once.

5. **Deterministic stale-version precondition.** Send a Patient update with a deliberately stale ETag. The pinned HAPI server returns HTTP 409 OperationOutcome; the harness accepts only the recognized 409/412 version-conflict class and confirms the resource version did not advance.

6. **Concurrent update conflict.** Submit two simultaneous Patient updates with the same current `If-Match` version. Require exactly one accepted update and one HTTP 409/412 conflict, with exactly one resource-version increment. Preserve both response bodies and status codes before assertions.

7. **Atomic rollback.** Submit one FHIR transaction that creates a rollback-marker Patient and then attempts an update with an impossible `If-Match`. Require an OperationOutcome failure, no rollback marker in persisted state, and no version change to the pre-existing Patient.

8. **Direct PostgreSQL observation.** Connect as `hapi_observer`, not the HAPI application role. Export the pinned runtime's actual `information_schema.columns` inventory; require the columns used by the probe; then query `HFJ_RESOURCE`, `HFJ_RES_VER` and `HFJ_SPIDX_TOKEN` directly. Require the database evidence to agree with the FHIR observations, including exactly one conditional-create Patient at version 1 and one history row after contention recovery.

The SQL observer must prove: session read-only, not superuser, cannot create roles, cannot INSERT/UPDATE/DELETE. It is independent of the FHIR HTTP API but reads the same PostgreSQL instance; it is not an independent replica or third-party auditor.

## Verified baseline before the 128-way increment

Three independent exact-head runs of the previous FHIR + direct SQL observer suite passed:

- Run #45, workflow 46022783456, source `c594b6f2be0a95e3b6957954843e2740d25b0658`, evidence `evidence/run-45-postgresql-observed.json`.
- Run #47, workflow 46023436149, source `d3f88912729fe5b89510d86b5be1356aa7726588`, evidence `evidence/run-47-postgresql-observed.json`.
- Run #49, workflow 46024161495, source `5e67b576ed2c425f27793ce2f1c5b53c26d29c77`, evidence `evidence/run-49-postgresql-observed.json`.

Each passed the two-entry Provenance+Observation transaction, exact conditional replay, deterministic stale ETag, one-winner concurrent update, rollback and direct PostgreSQL row/history/token-index checks. The read-only observer saw Patient master/history 1/2 at version 2, Provenance 1/1, Observation 1/1 at version 1 and zero rollback-marker token-index rows. Run #47 reversed which concurrent update won, while preserving the exactly-one-winner invariant.

Those results qualify only the prior test set. They do not retroactively qualify the new 128-way conditional-create behavior.

## Current 128-way conditional-create result

- Run #55 (workflow 46045883432, source 4edf655cedec667acbd25ea8ec9f263f3977dfcb) observed 1 × 201, 125 × 200 and 2 × 409. Both 409s reported HAPI-0550 / hfj_res_search_url_pkey. The earlier harness failed before final search/SQL observation. Frozen at evidence/run-55-conditional-create-contention.json.
- Run #62 (workflow 46047236740, source 8c26c5cffbe78b4787889f70c6af6f24cd97a2ff) passed the revised recovery probe. It observed 1 × 201, 126 × 200 and 1 known 409; the exact same conditional-create request retried to HTTP 200 for the original Patient ID 1001. Final FHIR search returned one match. Direct SQL confirmed one Patient master row, one history row and version 1. The observer was read-only, non-superuser and unable to mutate/create roles. Full frozen evidence: evidence/run-62-postgresql-conditional-create-recovered.json.
- The first-attempt 409 rate in #62 was 0.78125% (1/128). It remains reported as transient contention, not treated as a clean initial success. The accepted outcome is that the conflict is known, same-request retry is safe/idempotent, final effect cardinality is one, and no attempt remains unresolved.

Run #62 also passed the other seven probes: two-resource Provenance+Observation transaction, exact replay with unchanged Observation version, stale ETag handling, same-version concurrent update conflict, transaction rollback, CapabilityStatement and read-only direct SQL observation. Overall Mycelix qualification remains defer for the separate system-level gaps below.

## Remaining Mycelix qualification boundary

Overall Mycelix qualification remains `defer`. This harness does not exercise the Mycelix consent/revocation commit fence, gateway/outbox crash or lost-acknowledgement recovery, independent recipient-side durable-effect witness, production identity/authentication, enterprise audit, or an independently issued qualification receipt.

FHIR transaction atomicity is local to one supporting FHIR server. It does not establish cross-service atomic commit. No PHI, clinical-safety, HIPAA / 42 CFR Part 2, regulatory-certification, production-readiness or distributed exactly-once claim is made.

## Industry conformance baseline

Run the applicable Inferno US Core Test Kit for the profile(s) Mycelix intends to claim. Inferno profile conformance complements, but does not replace, active transaction, revocation, recovery and recipient-effect probes. Equally, this custom harness does not establish US Core or SMART conformance.
