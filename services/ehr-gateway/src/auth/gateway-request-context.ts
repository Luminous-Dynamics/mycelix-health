// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

/**
 * Typed request-authority contract for the EHR gateway.
 *
 * This module does not authenticate JWTs, verify mTLS, or retrieve consent
 * records. Those operations are injected through GatewayReceiptVerifierV1.
 * Callers must supply verifiers backed by configured trust roots and a
 * separately maintained authority/consent registry. Missing or failed
 * verifiers are deny outcomes.
 */

export const GATEWAY_ACTIONS_V1 = [
  'fhir_ingest',
  'fhir_export',
  'sync_pull',
  'sync_push',
  'sync_status',
  'sync_job_list',
] as const;

export type GatewayActionV1 = (typeof GATEWAY_ACTIONS_V1)[number];

export interface GatewayPrincipalV1 {
  /** Exact configured issuer URI of the authenticated principal. */
  issuer: string;
  /** Subject identifier from the verified authentication context. */
  subject: string;
  /** Authenticated client identifier, not a caller-supplied label. */
  clientId: string;
  /** Organization identity resolved from a trusted issuer/client mapping. */
  organizationId: string;
}

export interface GatewayRequestContextV1 {
  version: 1;
  requestId: string;
  principal: GatewayPrincipalV1;
  tenantId: string;
  /** Opaque registered subject binding; never a raw patient ID. */
  patientBinding: string;
  /** Server-side EHR connection registry ID. */
  sourceConnectionId: string;
  purpose: string;
  /** Intended destination/recipient identity, not a free-form URL. */
  recipientId: string;
  action: GatewayActionV1;
  requestedResourceTypes: string[];
  issuedAtMs: number;
  expiresAtMs: number;
  /** Digest of the authentication evidence verified by the injected verifier. */
  authenticationReceiptDigest: string;
  /** Digest of the consent/authority evidence verified by the injected verifier. */
  consentReceiptDigest: string;
}

export interface GatewayAuthorityGrantV1 {
  grantId: string;
  principal: GatewayPrincipalV1;
  tenantId: string;
  patientBinding: string;
  sourceConnectionId: string;
  purpose: string;
  recipientId: string;
  allowedActions: readonly GatewayActionV1[];
  allowedResourceTypes: readonly string[];
  notBeforeMs: number;
  expiresAtMs: number;
  revokedAtMs: number | null;
}

export interface GatewayReceiptVerifierV1 {
  /**
   * Verify the actual authentication mechanism (e.g. issuer/JWKS JWT
   * validation or a verified mTLS identity) and bind it to context.principal.
   */
  verifyPrincipal(context: GatewayRequestContextV1): Promise<boolean>;
  /**
   * Verify the consent/authority receipt against the independent current
   * authority registry. A digest string alone is never proof of consent.
   */
  verifyConsent(
    context: GatewayRequestContextV1,
    grant: GatewayAuthorityGrantV1,
  ): Promise<boolean>;
}

export type GatewayAuthorizationFailureV1 =
  | 'invalid_context'
  | 'invalid_receipt_digest'
  | 'context_not_yet_valid'
  | 'context_expired'
  | 'context_ttl_exceeded'
  | 'principal_unverified'
  | 'consent_unverified'
  | 'grant_not_yet_valid'
  | 'grant_expired'
  | 'grant_revoked'
  | 'principal_mismatch'
  | 'tenant_mismatch'
  | 'patient_mismatch'
  | 'source_connection_mismatch'
  | 'purpose_mismatch'
  | 'recipient_mismatch'
  | 'action_not_permitted'
  | 'resource_type_not_permitted'
  | 'verification_unavailable';

export type GatewayAuthorizationResultV1 =
  | {
      authorized: true;
      grantId: string;
      requestId: string;
      principal: GatewayPrincipalV1;
      tenantId: string;
      patientBinding: string;
      action: GatewayActionV1;
      expiresAtMs: number;
    }
  | {
      authorized: false;
      reason: GatewayAuthorizationFailureV1;
    };

export const GATEWAY_CONTEXT_MAX_TTL_MS = 5 * 60 * 1000;
export const GATEWAY_CONTEXT_CLOCK_SKEW_MS = 30 * 1000;

function nonEmpty(value: unknown): value is string {
  return typeof value === 'string' && value.trim().length > 0;
}

function isDigest(value: unknown): value is string {
  return typeof value === 'string' && /^[A-Fa-f0-9]{64}$/.test(value);
}

function isFiniteTime(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value);
}

function samePrincipal(left: GatewayPrincipalV1, right: GatewayPrincipalV1): boolean {
  return left.issuer === right.issuer
    && left.subject === right.subject
    && left.clientId === right.clientId
    && left.organizationId === right.organizationId;
}

function validPrincipal(principal: GatewayPrincipalV1): boolean {
  return nonEmpty(principal.issuer)
    && nonEmpty(principal.subject)
    && nonEmpty(principal.clientId)
    && nonEmpty(principal.organizationId);
}

function fail(reason: GatewayAuthorizationFailureV1): GatewayAuthorizationResultV1 {
  return { authorized: false, reason };
}

/**
 * Evaluate a short-lived gateway request against an independently verified
 * principal and a current, patient-scoped authority grant.
 *
 * All checks fail closed. A grant match never substitutes for authenticating
 * the caller, and receipt digests are not accepted without verifier callbacks.
 */
export async function authorizeGatewayRequestV1(
  context: GatewayRequestContextV1,
  grant: GatewayAuthorityGrantV1 | null,
  nowMs: number,
  verifier: GatewayReceiptVerifierV1 | null,
): Promise<GatewayAuthorizationResultV1> {
  if (
    context.version !== 1
    || !nonEmpty(context.requestId)
    || !validPrincipal(context.principal)
    || !nonEmpty(context.tenantId)
    || !nonEmpty(context.patientBinding)
    || !nonEmpty(context.sourceConnectionId)
    || !nonEmpty(context.purpose)
    || !nonEmpty(context.recipientId)
    || !GATEWAY_ACTIONS_V1.includes(context.action)
    || !Array.isArray(context.requestedResourceTypes)
    || context.requestedResourceTypes.some((value) => !nonEmpty(value))
    || !isFiniteTime(context.issuedAtMs)
    || !isFiniteTime(context.expiresAtMs)
    || !isFiniteTime(nowMs)
    || context.expiresAtMs <= context.issuedAtMs
  ) {
    return fail('invalid_context');
  }

  if (
    !isDigest(context.authenticationReceiptDigest)
    || !isDigest(context.consentReceiptDigest)
  ) {
    return fail('invalid_receipt_digest');
  }

  if (context.issuedAtMs > nowMs + GATEWAY_CONTEXT_CLOCK_SKEW_MS) {
    return fail('context_not_yet_valid');
  }
  if (context.expiresAtMs <= nowMs) {
    return fail('context_expired');
  }
  if (context.expiresAtMs - context.issuedAtMs > GATEWAY_CONTEXT_MAX_TTL_MS) {
    return fail('context_ttl_exceeded');
  }

  if (!verifier || !grant) {
    return fail('verification_unavailable');
  }

  try {
    if (!(await verifier.verifyPrincipal(context))) {
      return fail('principal_unverified');
    }
  } catch {
    return fail('verification_unavailable');
  }

  if (grant.revokedAtMs !== null && grant.revokedAtMs <= nowMs) {
    return fail('grant_revoked');
  }
  if (nowMs < grant.notBeforeMs || context.issuedAtMs < grant.notBeforeMs) {
    return fail('grant_not_yet_valid');
  }
  if (grant.expiresAtMs <= nowMs || context.expiresAtMs > grant.expiresAtMs) {
    return fail('grant_expired');
  }

  try {
    if (!(await verifier.verifyConsent(context, grant))) {
      return fail('consent_unverified');
    }
  } catch {
    return fail('verification_unavailable');
  }

  if (!samePrincipal(context.principal, grant.principal)) {
    return fail('principal_mismatch');
  }
  if (context.tenantId !== grant.tenantId) {
    return fail('tenant_mismatch');
  }
  if (context.patientBinding !== grant.patientBinding) {
    return fail('patient_mismatch');
  }
  if (context.sourceConnectionId !== grant.sourceConnectionId) {
    return fail('source_connection_mismatch');
  }
  if (context.purpose !== grant.purpose) {
    return fail('purpose_mismatch');
  }
  if (context.recipientId !== grant.recipientId) {
    return fail('recipient_mismatch');
  }
  if (!grant.allowedActions.includes(context.action)) {
    return fail('action_not_permitted');
  }

  const allowedResourceTypes = new Set(grant.allowedResourceTypes);
  if (context.requestedResourceTypes.some((resourceType) => !allowedResourceTypes.has(resourceType))) {
    return fail('resource_type_not_permitted');
  }

  return {
    authorized: true,
    grantId: grant.grantId,
    requestId: context.requestId,
    principal: context.principal,
    tenantId: context.tenantId,
    patientBinding: context.patientBinding,
    action: context.action,
    expiresAtMs: Math.min(context.expiresAtMs, grant.expiresAtMs),
  };
}
