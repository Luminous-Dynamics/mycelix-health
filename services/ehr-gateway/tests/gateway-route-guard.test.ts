import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it, vi } from 'vitest';
import { GATEWAY_ACTIONS_V1 } from '../src/auth/gateway-request-context.js';
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
      sourceConnectionId: 'connection-1',
      purpose: 'treatment',
      recipientId: 'mycelix-health',
      requestedResourceTypes: [...(requestedIntent.resourceTypes ?? [])],
      action: requestedIntent.action,
      expiresAtMs: 200_000,
      resolvedSourcePatientId: 'ehr-patient-1',
      resolvedMycelixPatientHash: 'patient-1',
      credentialHandle: 'credential-handle-1',
      resolvedJobId: 'job-1',
      resolvedConflictId: 'conflict-1',
    })),
  };
}

describe('gateway route action contract', () => {
  it('keeps the TypeScript action set in parity with the JSON schema', () => {
    const schemaPath = fileURLToPath(
      new URL('../schemas/gateway-request-context-v1.schema.json', import.meta.url),
    );
    const schema = JSON.parse(readFileSync(schemaPath, 'utf8')) as {
      properties: { action: { enum: string[] } };
    };
    expect([...GATEWAY_ACTIONS_V1].sort()).toEqual([...schema.properties.action.enum].sort());
    expect(GATEWAY_ACTIONS_V1).toEqual(expect.arrayContaining([
      'fhir_ingest',
      'fhir_export',
      'sync_pull',
      'sync_push',
      'sync_status',
      'sync_job_list',
      'sync_job_delete',
      'sync_bidirectional',
      'sync_conflicts_read',
      'sync_conflict_resolve',
    ]));
  });
});

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

  it('requires explicit server-resolved resource IDs for collection routes', async () => {
    const noScope = authorizedProvider();
    const denied = await authorizeGatewayRouteV1(
      noScope,
      new Request('https://gateway.example.test/sync/jobs'),
      { action: 'sync_job_list', requireScopedResourceList: true },
    );
    expect(denied).toEqual({
      authorized: false,
      reason: 'resource_type_not_permitted',
    });

    const scoped: GatewayRouteAuthorizationProviderV1 = {
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
        sourceConnectionId: 'connection-1',
        purpose: 'treatment',
        recipientId: 'mycelix-health',
        requestedResourceTypes: [...(requestedIntent.resourceTypes ?? [])],
        action: requestedIntent.action,
        expiresAtMs: 200_000,
        resolvedSourcePatientId: 'ehr-patient-1',
        resolvedMycelixPatientHash: 'patient-1',
        credentialHandle: 'credential-handle-1',
        resolvedJobId: 'job-1',
        resolvedConflictId: 'conflict-1',
        authorizedResourceIds: ['job-visible-1'],
      })),
    };
    const allowed = await authorizeGatewayRouteV1(
      scoped,
      new Request('https://gateway.example.test/sync/jobs'),
      { action: 'sync_job_list', requireScopedResourceList: true },
    );
    expect(allowed.authorized).toBe(true);
    if (allowed.authorized) {
      expect(allowed.authorizedResourceIds).toEqual(['job-visible-1']);
    }
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
