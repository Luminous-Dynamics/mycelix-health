// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

import type {
  GatewayActionV1,
  GatewayAuthorizationFailureV1,
  GatewayAuthorizationResultV1,
} from './gateway-request-context.js';

/**
 * Route intent is a policy selector only. Patient IDs, job IDs and conflict
 * IDs in the HTTP request are untrusted lookup hints; the provider must resolve
 * them through server-side connection/job/conflict registries before creating
 * a patient/tenant-bound GatewayRequestContextV1.
 */
export interface GatewayRouteIntentV1 {
  action: GatewayActionV1;
  resourceTypes?: readonly string[];
}

export interface GatewayRouteAuthorizationProviderV1 {
  /**
   * Authenticate the HTTP principal, resolve untrusted URL/body selectors to
   * registered opaque bindings, construct a short-lived signed context, load
   * current authority, and invoke authorizeGatewayRequestV1. Never derive
   * principal, tenant, patient, source, purpose or recipient authority from
   * caller-controlled headers/body fields.
   */
  authorize(
    request: Request,
    intent: GatewayRouteIntentV1,
  ): Promise<GatewayAuthorizationResultV1>;
}

export type GatewayRouteGuardResultV1 =
  | { authorized: true; authorization: Extract<GatewayAuthorizationResultV1, { authorized: true }> }
  | { authorized: false; reason: GatewayAuthorizationFailureV1 };

function failure(reason: GatewayAuthorizationFailureV1): GatewayRouteGuardResultV1 {
  return { authorized: false, reason };
}

/**
 * Fail-closed route guard. No provider, thrown verifier, malformed provider
 * result, or action mismatch means no route-side read or mutation may proceed.
 * This guard does not itself authenticate; that remains the provider's job.
 */
export async function authorizeGatewayRouteV1(
  provider: GatewayRouteAuthorizationProviderV1 | null | undefined,
  request: Request,
  intent: GatewayRouteIntentV1,
): Promise<GatewayRouteGuardResultV1> {
  if (!provider || !intent || !Array.isArray(intent.resourceTypes ?? [])) {
    return failure('verification_unavailable');
  }

  try {
    const result = await provider.authorize(request.clone(), {
      action: intent.action,
      resourceTypes: intent.resourceTypes ? [...intent.resourceTypes] : [],
    });

    if (!result || typeof result !== 'object' || typeof result.authorized !== 'boolean') {
      return failure('verification_unavailable');
    }
    if (result.authorized === false) {
      return failure(result.reason);
    }
    if (result.action !== intent.action) {
      return failure('action_not_permitted');
    }
    if (!result.requestId || !result.tenantId || !result.patientBinding) {
      return failure('invalid_context');
    }
    return { authorized: true, authorization: result };
  } catch {
    return failure('verification_unavailable');
  }
}

/** Map missing verifier/registry infrastructure to 503, policy denials to 403. */
export function gatewayAuthorizationHttpStatusV1(
  reason: GatewayAuthorizationFailureV1,
): 403 | 503 {
  return reason === 'verification_unavailable' ? 503 : 403;
}

/** Return a standards-neutral HTTP denial response for protected route handlers. */
export async function requireGatewayRouteAuthorizationV1(
  provider: GatewayRouteAuthorizationProviderV1 | null | undefined,
  request: Request,
  intent: GatewayRouteIntentV1,
): Promise<{ authorized: true } | { authorized: false; response: Response }> {
  const result = await authorizeGatewayRouteV1(provider, request, intent);
  if (result.authorized) {
    return { authorized: true };
  }
  const status = gatewayAuthorizationHttpStatusV1(result.reason);
  return {
    authorized: false,
    response: new Response(
      JSON.stringify({
        error: status === 503 ? 'Authorization service unavailable' : 'Forbidden',
      }),
      {
        status,
        headers: { 'content-type': 'application/json; charset=utf-8' },
      },
    ),
  };
}
