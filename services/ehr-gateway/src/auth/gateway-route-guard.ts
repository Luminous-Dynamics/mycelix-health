// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

import {
  GATEWAY_ACTIONS_V1,
  type GatewayActionV1,
  type GatewayAuthorizationFailureV1,
  type GatewayAuthorizationResultV1,
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
  /** Require explicit resource IDs for collection/list responses. */
  requireScopedResourceList?: boolean;
}

export type GatewayRouteAuthorizationResultV1 = GatewayAuthorizationResultV1 & {
  /**
   * Server-resolved IDs that this principal may see for list routes.
   * Job lists use job IDs. Conflict lists use JSON.stringify([resourceType, resourceId]).
   */
  authorizedResourceIds?: readonly string[];
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
  ): Promise<GatewayRouteAuthorizationResultV1>;
}

export type GatewayRouteGuardResultV1 =
  | {
      authorized: true;
      authorization: Extract<GatewayRouteAuthorizationResultV1, { authorized: true }>;
      authorizedResourceIds?: readonly string[];
    }
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
  if (
    !provider
    || !intent
    || !GATEWAY_ACTIONS_V1.includes(intent.action)
    || !Array.isArray(intent.resourceTypes ?? [])
    || (intent.resourceTypes ?? []).some((value) => typeof value !== 'string' || value.trim().length === 0)
  ) {
    return failure('verification_unavailable');
  }

  try {
    const result = await provider.authorize(request.clone(), {
      ...intent,
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
    if (
      intent.requireScopedResourceList
      && (!Array.isArray(result.authorizedResourceIds)
        || result.authorizedResourceIds.some((id) => typeof id !== 'string' || id.length === 0))
    ) {
      return failure('resource_type_not_permitted');
    }
    return {
      authorized: true,
      authorization: result,
      authorizedResourceIds: result.authorizedResourceIds,
    };
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
): Promise<
  | {
      authorized: true;
      authorization: Extract<GatewayRouteAuthorizationResultV1, { authorized: true }>;
      authorizedResourceIds?: readonly string[];
    }
  | { authorized: false; response: Response }
> {
  const result = await authorizeGatewayRouteV1(provider, request, intent);
  if (result.authorized) {
    return {
      authorized: true,
      authorization: result.authorization,
      authorizedResourceIds: result.authorizedResourceIds,
    };
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
