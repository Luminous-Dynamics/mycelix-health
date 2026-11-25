# Gateway Request Context v1 — authority contract

**Status:** typed policy-evaluation primitive and denial-matrix tests added; runtime authentication, consent verification, and route integration are not yet implemented.  
**Security issue:** #289.  
**Date:** 2026-11-25.

## Purpose

An authenticated HTTP request must not become a universal Holochain gateway identity. A request that reads, imports, exports, synchronizes, or discloses a patient's information must carry an explicit, short-lived, subject-scoped authority context. The gateway may propose the operation; an independent verifier and authority registry must decide whether it is allowed.

The code in 'src/auth/gateway-request-context.ts' defines:
- 'GatewayPrincipalV1': issuer, subject, client ID, and organization ID from a verified authentication mechanism and trusted issuer/client mapping;
- 'GatewayRequestContextV1': tenant, opaque patient binding, registered source connection, purpose, recipient, action, resource types, short validity window, and digests of authentication/consent receipts;
- 'GatewayAuthorityGrantV1': registry-backed scope, subject, purpose, recipient, action/resource constraints, validity, and revocation;
- 'GatewayReceiptVerifierV1': injected verification callbacks for authentication evidence and independent consent/authority evidence;
- 'authorizeGatewayRequestV1': fail-closed comparison and policy evaluation.

JSON schemas are in 'schemas/gateway-request-context-v1.schema.json' and 'schemas/gateway-authority-grant-v1.schema.json'.

## Policy rules

Authorization requires exact agreement on:
1. issuer + subject + client + organization;
2. tenant and opaque patient binding;
3. registered source connection;
4. purpose and recipient;
5. action and every requested resource type;
6. grant validity and revocation status.

The request context cannot outlive its authority grant and is limited to five minutes. Clock skew is bounded. Missing context, malformed grant, absent verifier, unavailable issuer/JWKS/consent registry, failed receipt verification, expiry, revocation, or any scope mismatch denies the request.

A digest is a binding value, not proof. The injected verifier must authenticate the original signed identity/consent evidence against separately configured trust roots and current authority state. Do not implement these callbacks as 'return true', caller-provided flags, or request headers.

## Required production adapters

**Authentication verifier**
- For user-facing EHR launch, validate the selected OIDC/SMART flow and bind the resulting identity to a configured issuer and verified organization/practitioner mapping.
- For autonomous backend access, use SMART Backend Services or an equivalent reviewed machine identity; verify client assertion signature, issuer, subject, audience, expiry, key status/rotation and pre-authorized scopes.
- Where mTLS is used, verify the peer certificate chain and map its identity to the same principal registry.
- Never infer authority from 'X-Source-System', raw 'patientId', request body roles, caller-supplied organization names, or a raw AgentPubKey.

**Consent/authority verifier**
- Resolve the consent/capability from the independently maintained registry.
- Verify grant issuer/signature, patient/tenant binding, purpose, recipient, allowed action/resource types, validity, and revocation.
- A Holochain zome must independently validate the gateway/provider capability or signed context; the HTTP gateway's successful check alone must not authorize a universal service agent.

**Route integration**
- Protect FHIR ingest/export, sync pull/push, sync status, job listing/deletion, and conflict routes.
- Scope status/job access to the same principal, tenant and patient as the originating operation. An unguessable job ID is not an authorization token.
- Remove token/refresh secret material from query strings. Prefer server-side token handles bound to the verified principal, connection, scope and expiry.
- Derive source principal from the verified principal/connection registry; do not use a caller-supplied source label as provenance.
- Redact query values, credentials, patient identifiers and payloads from application and proxy logs.

## Explicit boundary and non-claims

This module is **not wired into 'createApp' or the FHIR/sync routes** and does not yet authenticate a JWT, verify mTLS, retrieve consent, or change runtime behavior. The injected verifier interface is a contract, not a deployment-ready trust root. The existing gateway remains blocked for network-exposed deployment by #289.

Tests use fake verifier callbacks to exercise the pure policy function. They do not prove JWT, JWKS, mTLS, SMART, consent-registry, Holochain capability, tenant isolation, or multi-agent security. Those require separate integration tests and independent security review.

## Acceptance matrix

- Anonymous/missing principal -> deny before patient data operations.
- Wrong issuer, subject, client or organization -> deny.
- Wrong tenant, patient binding, source connection, purpose or recipient -> deny.
- Action or resource type outside pre-authorized grant -> deny.
- Expired, future-issued, overlong, revoked, or not-yet-valid context/grant -> deny.
- Missing or malformed authentication/consent receipt digest -> deny.
- Authentication/consent verifier false, absent, throws, or is unavailable -> deny.
- Valid, exact, current, patient-scoped request -> allow with a grant-bound short expiry.

## Standards alignment

For autonomous backend EHR access, see the [HL7 SMART App Launch v2.2 Backend Services profile](https://hl7.org/fhir/smart-app-launch/STU2.2/backend-services.html). It assumes the client is pre-authorized by the resource server and requires client authentication using a signed assertion. SMART scopes constrain what the server already authorizes; they do not replace this gateway's tenant/patient/purpose/recipient policy or downstream Holochain checks.
