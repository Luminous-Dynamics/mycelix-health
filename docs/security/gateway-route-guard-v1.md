# EHR Gateway Route Guard v1

**Status:** fail-closed route wiring and test contract. No production identity, JWT/JWKS, mTLS, consent, grant registry, signature verifier, or route-authorizer implementation is supplied by this change.

## Protected route surface

The following data routes now call the injected `GatewayRouteAuthorizationProviderV1` before any Holochain read/mutation or sensitive response:

| HTTP route | Required action |
|---|---|
| `POST /fhir/Bundle` | `fhir_ingest` |
| `GET /fhir/Bundle/:patientHash` | `fhir_export` |
| `POST /sync/patient/:patientId` | `sync_pull` |
| `POST /sync/bidirectional/:patientId` | `sync_bidirectional` |
| `GET /sync/status/:jobId` | `sync_status` |
| `GET /sync/jobs` | `sync_job_list` |
| `DELETE /sync/jobs/:jobId` | `sync_job_delete` |
| `GET /sync/conflicts` | `sync_conflicts_read` |
| `POST /sync/conflicts/:conflictId/resolve` | `sync_conflict_resolve` |

Health/readiness and `GET /fhir/metadata` are intentionally public.

If no provider is configured, the route guard returns HTTP 503 before the handler proceeds. An explicit policy rejection returns 403. Provider exceptions, malformed responses, wrong-action responses, and verifier/registry outages never grant access.

## Authorizer contract

The route authorizer must:
1. Authenticate the actual HTTP caller using configured issuer/JWKS, SMART, mTLS, or another approved mechanism.
2. Treat path, query, body and headers as untrusted selectors only. In particular, `patientId`, `patientHash`, `connection`, `sourceSystem`, `resolved_by` and job/conflict IDs never establish identity or authority.
3. Resolve selectors to registered tenant, opaque patient binding and source-connection identities using server-side registries.
4. Construct a short-lived signed `GatewayRequestContextV1`, load the current grant, verify consent/purpose/recipient/action/resource type, and consume the context request ID atomically.
5. Recheck grant expiry/revocation at request time. A result from a previous request or prior probe offer is not reusable authorization.

The route guard itself is not an authenticator. The injected provider is a privileged server component and must only be implemented using the verifier and authority-registry contracts from gateway request-context v1.

## Collection/list response scoping

A successful `sync_job_list` or `sync_conflicts_read` authorization must include `authorizedResourceIds` produced by the server-side authority resolver.

- Job lists use exact registered sync-job IDs.
- Conflict lists use `JSON.stringify([resourceType, resourceId])` as the stable resource selector.

The handlers filter before serialization. They never return unfiltered global totals or conflict statistics. Missing collection scope denies the request. An empty authorized ID set returns an empty collection, not an unfiltered fallback.

Job status/delete and conflict resolution authorizers must resolve the supplied identifier against the server-side job/conflict registry and bind that resource to the caller's current subject/tenant/connection grant. An unguessable job ID is not a capability.

## Explicitly not covered

The SMART launch/callback and EHR connection administration routes (`/auth/connections`, `/auth/connect`, disconnect, launch and callback) are a separate organization-admin/OAuth-state boundary. They are not made safe by patient-scoped route authorization. Until an organization-scoped admin verifier and replay-safe OAuth state contract are connected, keep these routes on a loopback-only administrative interface and do not expose them publicly. See gateway P0 issue #289 and loopback/CORS PR #297.

The current application startup does not supply a production route authorizer. Consequently, protected FHIR and sync operations are intentionally unavailable (503) until an independently reviewed provider is configured. This prevents accidental operation under the old unauthenticated route behavior, but is not proof of complete gateway security.

## Qualification

The gateway test suite covers:
- default-deny behavior when no provider exists;
- provider exception, malformed result, explicit denial and action mismatch;
- no Holochain/conflict-store access after a default-deny result;
- health and capability metadata remaining available;
- job/conflict collection filtering by server-authorized IDs;
- action/schema parity through TypeScript type checking.

Passing these contract tests does not establish production authentication, cryptographic verification, consent correctness, route completeness outside the listed surface, Holochain zome-side authorization, or security-review completion.
