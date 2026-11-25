import { describe, expect, it, vi } from 'vitest';
import {
  authorizeGatewayRouteV1,
  gatewayAuthorizationHttpStatusV1,
  requireGatewayRouteAuthorizationV1,
  type GatewayRouteAuthorizationProviderV1,
} from '../src/auth/gateway-route-guard.js';

const intent = {
  action: 'fhir_export' as const,
  resourceTypes: ['Patient', 'Observation'],
};

function authorizedProvider(): GatewayRouteAuthorizationProviderV1 {
  return {
    authorize: vi.fn(async (_request, requestedIntent) => ({
      authorized: true as const,
      grantId: 'grant-1',
      requestId: 'request-1',
      principal: {
        issuer: 'https://issuer.example.test',
        subject: 'subject-1',
        clientId: 'client-1',
        organizationId: 'org-1',
      },
      tenantId: 'tenant-1',
      patientBinding: 'opaque-subject-1',
      action: requestedIntent.action,
      expiresAtMs: 200_000,
    })),
  };
}

describe('authorizeGatewayRouteV1', () => {
  it('denies when no trusted provider is configured', async () => {
    const result = await authorizeGatewayRouteV1(
      undefined,
      new Request('https://gateway.example.test/fhir/Bundle/patient-1'),
      intent,
    );
    expect(result).toEqual({
      authorized: false,
      reason: 'verification_unavailable',
    });
    expect(gatewayAuthorizationHttpStatusV1('verification_unavailable')).toBe(503);
  });

  it('passes only an exact action match returned by the provider', async () => {
    const provider = authorizedProvider();
    const result = await authorizeGatewayRouteV1(
      provider,
      new Request('https://gateway.example.test/fhir/Bundle/patient-1'),
      intent,
    );
    expect(result.authorized).toBe(true);
    expect(provider.authorize).toHaveBeenCalledOnce();
    expect(provider.authorize).toHaveBeenCalledWith(
      expect.any(Request),
      intent,
    );
  });

  it('rejects an authorizer result for a different route action', async () => {
    const provider: GatewayRouteAuthorizationProviderV1 = {
      authorize: vi.fn(async () => ({
        authorized: true as const,
        grantId: 'grant-1',
        requestId: 'request-1',
        principal: {
          issuer: 'https://issuer.example.test',
          subject: 'subject-1',
          clientId: 'client-1',
          organizationId: 'org-1',
        },
        tenantId: 'tenant-1',
        patientBinding: 'opaque-subject-1',
        action: 'sync_push' as const,
        expiresAtMs: 200_000,
      })),
    };
    const result = await authorizeGatewayRouteV1(
      provider,
      new Request('https://gateway.example.test/fhir/Bundle/patient-1'),
      intent,
    );
    expect(result).toEqual({
      authorized: false,
      reason: 'action_not_permitted',
    });
  });

  it('maps policy rejection to 403 and verifier outage to 503', async () => {
    const denied: GatewayRouteAuthorizationProviderV1 = {
      authorize: vi.fn(async () => ({
        authorized: false as const,
        reason: 'patient_mismatch' as const,
      })),
    };
    const unavailable: GatewayRouteAuthorizationProviderV1 = {
      authorize: vi.fn(async () => {
        throw new Error('revocation registry unavailable');
      }),
    };

    const request = new Request('https://gateway.example.test/fhir/Bundle/patient-1');
    const policy = await requireGatewayRouteAuthorizationV1(denied, request, intent);
    const outage = await requireGatewayRouteAuthorizationV1(unavailable, request, intent);

    expect(policy.authorized).toBe(false);
    if (!policy.authorized) expect(policy.response.status).toBe(403);
    expect(outage.authorized).toBe(false);
    if (!outage.authorized) expect(outage.response.status).toBe(503);
  });

  it('never lets a provider exception or malformed result grant access', async () => {
    const throwing: GatewayRouteAuthorizationProviderV1 = {
      authorize: vi.fn(async () => {
        throw new Error('verification failed');
      }),
    };
    const malformed = {
      authorize: vi.fn(async () => ({ authorized: 'yes' })),
    } as unknown as GatewayRouteAuthorizationProviderV1;
    for (const provider of [throwing, malformed]) {
      const result = await authorizeGatewayRouteV1(
        provider,
        new Request('https://gateway.example.test/fhir/Bundle/patient-1'),
        intent,
      );
      expect(result).toEqual({
        authorized: false,
        reason: 'verification_unavailable',
      });
    }
  });
});
