// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
/**
 * Server Tests
 *
 * Tests for the EHR Gateway HTTP server endpoints.
 */

import { describe, it, expect, beforeAll, afterAll, vi } from 'vitest';

// Mock @holochain/client before importing server
vi.mock('@holochain/client', () => ({
  AppWebsocket: {
    connect: vi.fn(),
  },
}));

import { createApp, type AppContext } from '../src/server.js';
import { getTestConfig } from '../src/config.js';
import type { FhirBundle, IngestReport } from '../src/types.js';

// Mock Holochain client
const mockHolochainClient = {
  callZome: vi.fn(),
};

// Mock gateway
const mockGateway = {
  isConnected: vi.fn(),
  getConnectionIds: vi.fn().mockReturnValue(['test-connection']),
  getConnectionInfo: vi.fn().mockReturnValue({
    system: 'epic',
    baseUrl: 'https://fhir.epic.com',
    activeTokens: 1,
  }),
  getAuthorizationUrl: vi.fn(),
  completeAuthorization: vi.fn(),
  pullPatientData: vi.fn(),
  syncPatient: vi.fn(),
  getPendingConflicts: vi.fn().mockReturnValue([]),
  getConflictStats: vi.fn().mockReturnValue({ total: 0, resolved: 0 }),
  connect: vi.fn(),
  disconnect: vi.fn(),
  resolveConflict: vi.fn(),
};

describe('EHR Gateway Server', () => {
  let app: ReturnType<typeof createApp>;
  let ctx: AppContext;

  beforeAll(() => {
    ctx = {
      gateway: mockGateway as any,
      holochainClient: mockHolochainClient as any,
      config: getTestConfig(),
      syncJobs: new Map(),
      // This is a test-only allow fixture. Production must inject a verifier-
      // backed provider; the server itself defaults to deny.
      gatewayRouteAuthorizer: {
        authorize: vi.fn(async (_request, intent) => ({
          authorized: true as const,
          grantId: 'test-grant',
          requestId: 'test-request',
          principal: {
            issuer: 'https://idp.example.test',
            subject: 'test-subject',
            clientId: 'test-client',
            organizationId: 'test-org',
          },
          tenantId: 'test-tenant',
          patientBinding: 'opaque-test-patient',
          sourceConnectionId: 'test-connection',
          purpose: 'treatment',
          recipientId: 'mycelix-test',
          requestedResourceTypes: [...(intent.resourceTypes ?? [])],
          action: intent.action,
          expiresAtMs: Date.now() + 60_000,
          resolvedSourcePatientId: '123',
          resolvedMycelixPatientHash: 'uhCEkINI4cBkNicMiyEvxrUXKy71OIGWXqXJagdCunS2SlmaipQ1j',
          credentialHandle: 'test-credential-handle',
          resolvedJobId: 'unguessable-job-id',
          resolvedConflictId: 'test-conflict-id',
          ...(intent.requireScopedResourceList
            ? { authorizedResourceIds: [] }
            : {}),
        })),
      },
    };
    app = createApp(ctx);
  });

  afterAll(() => {
    vi.clearAllMocks();
  });

  describe('Health Endpoints', () => {
    it('GET /health returns ok status', async () => {
      const res = await app.request('/health');
      const body = await res.json();

      expect(res.status).toBe(200);
      expect(body.status).toBe('ok');
      expect(body.service).toBe('ehr-gateway');
      expect(body.holochain).toBe('connected');
    });

    it('GET /ready returns ready when Holochain is connected', async () => {
      const res = await app.request('/ready');
      const body = await res.json();

      expect(res.status).toBe(200);
      expect(body.status).toBe('ready');
    });

    it('GET /ready returns 503 when Holochain is not connected', async () => {
      const ctxNoHolo: AppContext = {
        ...ctx,
        holochainClient: null,
      };
      const appNoHolo = createApp(ctxNoHolo);

      const res = await appNoHolo.request('/ready');
      const body = await res.json();

      expect(res.status).toBe(503);
      expect(body.status).toBe('not ready');
    });
  });

  describe('Fail-closed route authorization', () => {
    it('denies FHIR and sync data routes when no trusted authorizer is configured', async () => {
      const appWithoutAuthorizer = createApp({
        ...ctx,
        gatewayRouteAuthorizer: undefined,
      });

      const blockedRequests = [
        appWithoutAuthorizer.request('/fhir/Bundle', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ resourceType: 'Bundle', type: 'collection' }),
        }),
        appWithoutAuthorizer.request('/fhir/Bundle/patient-123'),
        appWithoutAuthorizer.request('/sync/status/unguessable-job-id'),
        appWithoutAuthorizer.request('/sync/jobs'),
        appWithoutAuthorizer.request('/sync/conflicts'),
      ];
      const responses = await Promise.all(blockedRequests);

      expect(responses.every((response) => response.status === 503)).toBe(true);
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
      expect(mockGateway.getPendingConflicts).not.toHaveBeenCalled();
    });

    it('keeps health and FHIR capability metadata public without a data authorizer', async () => {
      const appWithoutAuthorizer = createApp({
        ...ctx,
        gatewayRouteAuthorizer: undefined,
      });
      expect((await appWithoutAuthorizer.request('/health')).status).toBe(200);
      expect((await appWithoutAuthorizer.request('/fhir/metadata')).status).toBe(200);
    });
  });

  describe('FHIR Bundle Endpoints', () => {
    it('POST /fhir/Bundle ingests a valid bundle', async () => {
      const mockReport: IngestReport = {
        source_system: 'test-ehr',
        total_processed: 5,
        patients_created: 1,
        patients_updated: 0,
        conditions_created: 2,
        conditions_skipped: 0,
        medications_created: 2,
        medications_skipped: 0,
        allergies_created: 0,
        allergies_skipped: 0,
        immunizations_created: 0,
        immunizations_skipped: 0,
        observations_created: 0,
        observations_skipped: 0,
        procedures_created: 0,
        procedures_skipped: 0,
        unknown_types: [],
        parse_errors: [],
      };

      mockGateway.isConnected.mockReturnValueOnce(true);
      mockHolochainClient.callZome.mockResolvedValueOnce(mockReport);

      const bundle: FhirBundle = {
        resourceType: 'Bundle',
        type: 'collection',
        entry: [
          {
            fullUrl: 'Patient/123',
            resource: {
              resourceType: 'Patient',
              id: '123',
              name: [{ family: 'Test', given: ['User'] }],
            },
          },
        ],
      };

      const res = await app.request('/fhir/Bundle', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Source-System': 'attacker-controlled-source',
        },
        body: JSON.stringify(bundle),
      });

      const body = await res.json();

      expect(res.status).toBe(200);
      expect(body.status).toBe('success');
      expect(body.source_system).toBe('epic');
      expect(body.report).toBeDefined();
    });

    it('POST /fhir/Bundle rejects a Patient that differs from the resolved source patient', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      mockHolochainClient.callZome.mockClear();
      const bundle: FhirBundle = {
        resourceType: 'Bundle',
        type: 'collection',
        entry: [{
          fullUrl: 'Patient/attacker-patient',
          resource: {
            resourceType: 'Patient',
            id: 'attacker-patient',
          },
        }],
      };
      const res = await app.request('/fhir/Bundle', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-Source-System': 'spoofed' },
        body: JSON.stringify(bundle),
      });
      expect(res.status).toBe(403);
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
    });

    it('POST /fhir/Bundle rejects absolute Patient references from another EHR origin', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      mockHolochainClient.callZome.mockClear();
      const bundle = {
        resourceType: 'Bundle',
        type: 'collection',
        entry: [
          {
            fullUrl: 'Patient/123',
            resource: { resourceType: 'Patient', id: '123' },
          },
          {
            fullUrl: 'Observation/obs-1',
            resource: {
              resourceType: 'Observation',
              id: 'obs-1',
              subject: { reference: 'https://attacker.example/Patient/123' },
            },
          },
        ],
      };
      const res = await app.request('/fhir/Bundle', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(bundle),
      });
      expect(res.status).toBe(403);
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
    });

    it('POST /fhir/Bundle resolves matching urn:uuid Patient references through Bundle.fullUrl', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      mockHolochainClient.callZome.mockResolvedValueOnce({
        source_system: 'epic',
        total_processed: 2,
        patients_created: 1,
        patients_updated: 0,
        conditions_created: 0,
        conditions_skipped: 0,
        medications_created: 0,
        medications_skipped: 0,
        allergies_created: 0,
        allergies_skipped: 0,
        immunizations_created: 0,
        immunizations_skipped: 0,
        observations_created: 1,
        observations_skipped: 0,
        procedures_created: 0,
        procedures_skipped: 0,
        diagnostic_reports_created: 0,
        diagnostic_reports_skipped: 0,
        care_plans_created: 0,
        care_plans_skipped: 0,
        unknown_types: [],
        parse_errors: [],
      });
      const bundle = {
        resourceType: 'Bundle',
        type: 'collection',
        entry: [
          {
            fullUrl: 'urn:uuid:patient-123',
            resource: { resourceType: 'Patient', id: '123' },
          },
          {
            fullUrl: 'urn:uuid:observation-1',
            resource: {
              resourceType: 'Observation',
              id: 'observation-1',
              subject: { reference: 'urn:uuid:patient-123' },
            },
          },
        ],
      };
      const res = await app.request('/fhir/Bundle', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(bundle),
      });
      expect(res.status).toBe(200);
      expect(mockHolochainClient.callZome).toHaveBeenCalledTimes(1);
    });

    it('POST /fhir/Bundle rejects urn:uuid references to a different Patient', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      mockHolochainClient.callZome.mockClear();
      const bundle = {
        resourceType: 'Bundle',
        type: 'collection',
        entry: [
          {
            fullUrl: 'Patient/123',
            resource: { resourceType: 'Patient', id: '123' },
          },
          {
            fullUrl: 'urn:uuid:patient-other',
            resource: { resourceType: 'Patient', id: 'other-patient' },
          },
          {
            fullUrl: 'urn:uuid:observation-1',
            resource: {
              resourceType: 'Observation',
              id: 'observation-1',
              subject: { reference: 'urn:uuid:patient-other' },
            },
          },
        ],
      };
      const res = await app.request('/fhir/Bundle', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(bundle),
      });
      expect(res.status).toBe(403);
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
    });

    it('POST /fhir/Bundle rejects patient-scoped resources with no subject or patient reference', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      mockHolochainClient.callZome.mockClear();
      const bundle = {
        resourceType: 'Bundle',
        type: 'collection',
        entry: [
          { fullUrl: 'Patient/123', resource: { resourceType: 'Patient', id: '123' } },
          {
            fullUrl: 'Observation/obs-without-subject',
            resource: { resourceType: 'Observation', id: 'obs-without-subject' },
          },
        ],
      };

      const res = await app.request('/fhir/Bundle', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(bundle),
      });

      expect(res.status).toBe(403);
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
    });

    it('POST /fhir/Bundle rejects malformed and empty Patient reference fields', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      mockHolochainClient.callZome.mockClear();
      const bundle = {
        resourceType: 'Bundle',
        type: 'collection',
        entry: [
          { fullUrl: 'Patient/123', resource: { resourceType: 'Patient', id: '123' } },
          {
            fullUrl: 'Observation/obs-malformed-subject',
            resource: {
              resourceType: 'Observation',
              id: 'obs-malformed-subject',
              subject: { reference: 123 },
            },
          },
        ],
      };

      const res = await app.request('/fhir/Bundle', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(bundle),
      });

      expect(res.status).toBe(403);
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
    });

    it('POST /fhir/Bundle rejects explicitly null Patient reference fields', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      mockHolochainClient.callZome.mockClear();
      const bundle = {
        resourceType: 'Bundle',
        type: 'collection',
        entry: [
          { fullUrl: 'Patient/123', resource: { resourceType: 'Patient', id: '123' } },
          {
            fullUrl: 'Observation/obs-null-subject',
            resource: {
              resourceType: 'Observation',
              id: 'obs-null-subject',
              subject: null,
              patient: { reference: 'Patient/123' },
            },
          },
        ],
      };

      const res = await app.request('/fhir/Bundle', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(bundle),
      });

      expect(res.status).toBe(403);
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
    });

    it('POST /fhir/Bundle rejects conflicting subject and patient references', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      mockHolochainClient.callZome.mockClear();
      const bundle = {
        resourceType: 'Bundle',
        type: 'collection',
        entry: [
          { fullUrl: 'Patient/123', resource: { resourceType: 'Patient', id: '123' } },
          {
            fullUrl: 'Observation/obs-conflicting-subject',
            resource: {
              resourceType: 'Observation',
              id: 'obs-conflicting-subject',
              subject: { reference: 'Patient/123' },
              patient: { reference: 'Patient/other-patient' },
            },
          },
        ],
      };

      const res = await app.request('/fhir/Bundle', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(bundle),
      });

      expect(res.status).toBe(403);
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
    });

    it('POST /fhir/Bundle rejects identifier-only Patient references', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      mockHolochainClient.callZome.mockClear();
      const bundle = {
        resourceType: 'Bundle',
        type: 'collection',
        entry: [
          { fullUrl: 'Patient/123', resource: { resourceType: 'Patient', id: '123' } },
          {
            fullUrl: 'Observation/obs-identifier-only',
            resource: {
              resourceType: 'Observation',
              id: 'obs-identifier-only',
              subject: { identifier: { system: 'urn:test:ehr-patient', value: '123' } },
            },
          },
        ],
      };

      const res = await app.request('/fhir/Bundle', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(bundle),
      });

      expect(res.status).toBe(403);
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
    });

    it('POST /fhir/Bundle rejects Patient references with unresolved identifier metadata', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      mockHolochainClient.callZome.mockClear();
      const bundle = {
        resourceType: 'Bundle',
        type: 'collection',
        entry: [
          { fullUrl: 'Patient/123', resource: { resourceType: 'Patient', id: '123' } },
          {
            fullUrl: 'Observation/obs-reference-with-identifier',
            resource: {
              resourceType: 'Observation',
              id: 'obs-reference-with-identifier',
              subject: {
                reference: 'Patient/123',
                identifier: { system: 'urn:test:ehr-patient', value: 'other-patient' },
              },
            },
          },
        ],
      };

      const res = await app.request('/fhir/Bundle', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(bundle),
      });

      expect(res.status).toBe(403);
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
    });

    it('POST /fhir/Bundle rejects invalid bundle', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      const res = await app.request('/fhir/Bundle', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ resourceType: 'Patient' }),
      });

      const body = await res.json();

      expect(res.status).toBe(400);
      expect(body.error).toBe('Invalid resource type');
    });

    it('GET /fhir/Bundle exports only the resolved patient and authorized sections', async () => {
      const resolvedHash = 'uhCEkINI4cBkNicMiyEvxrUXKy71OIGWXqXJagdCunS2SlmaipQ1j';
      mockHolochainClient.callZome.mockResolvedValueOnce({
        bundle: JSON.stringify({ resourceType: 'Bundle', type: 'collection', entry: [] }),
        resource_count: 0,
        export_timestamp: Date.now(),
      });

      const res = await app.request('/fhir/Bundle/' + resolvedHash + '?sections=conditions');
      expect(res.status).toBe(200);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(expect.objectContaining({
        fn_name: 'export_patient_fhir',
        payload: { patient_hash: resolvedHash, include_sections: ['Patient', 'Condition'] },
      }));
    });

    it('GET /fhir/Bundle allows Patient-only authority without widening to other sections', async () => {
      const patientOnlyHash = 'uhCEkINI4cBkNicMiyEvxrUXKy71OIGWXqXJagdCunS2SlmaipQ1j';
      const patientOnlyApp = createApp({
        ...ctx,
        gatewayRouteAuthorizer: {
          authorize: vi.fn(async () => ({
            authorized: true as const,
            grantId: 'patient-only-grant',
            requestId: 'patient-only-request',
            principal: {
              issuer: 'https://idp.example.test',
              subject: 'test-subject',
              clientId: 'test-client',
              organizationId: 'test-org',
            },
            tenantId: 'test-tenant',
            patientBinding: 'opaque-test-patient',
            sourceConnectionId: 'test-connection',
            purpose: 'treatment',
            recipientId: 'mycelix-test',
            requestedResourceTypes: ['Patient'],
            action: 'fhir_export' as const,
            expiresAtMs: Date.now() + 60_000,
            resolvedMycelixPatientHash: patientOnlyHash,
          })),
        },
      });
      mockHolochainClient.callZome.mockResolvedValueOnce({
        bundle: JSON.stringify({ resourceType: 'Bundle', type: 'collection', entry: [] }),
        resource_count: 0,
        export_timestamp: Date.now(),
      });

      const res = await patientOnlyApp.request('/fhir/Bundle/' + patientOnlyHash);
      expect(res.status).toBe(200);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(expect.objectContaining({
        fn_name: 'export_patient_fhir',
        payload: { patient_hash: patientOnlyHash, include_sections: ['Patient'] },
      }));
    });

    it('GET /fhir/Bundle denies unsupported sections before calling the zome', async () => {
      mockHolochainClient.callZome.mockClear();
      const res = await app.request('/fhir/Bundle/uhCEkINI4cBkNicMiyEvxrUXKy71OIGWXqXJagdCunS2SlmaipQ1j?sections=allergies');
      expect(res.status).toBe(403);
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
    });

    it('GET /fhir/Bundle rejects a path hash that differs from the verified patient binding', async () => {
      mockHolochainClient.callZome.mockClear();
      const res = await app.request('/fhir/Bundle/attacker-selected-patient');
      expect(res.status).toBe(403);
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
    });

    it('GET /fhir/metadata returns capability statement', async () => {
      const res = await app.request('/fhir/metadata');
      const body = await res.json();

      expect(res.status).toBe(200);
      expect(body.resourceType).toBe('CapabilityStatement');
      expect(body.fhirVersion).toBe('4.0.1');
    });
  });

  describe('Auth Endpoints', () => {
    it('GET /auth/connections lists connections', async () => {
      const res = await app.request('/auth/connections');
      const body = await res.json();

      expect(res.status).toBe(200);
      expect(body.total).toBe(1);
      expect(body.connections).toHaveLength(1);
      expect(body.connections[0].id).toBe('test-connection');
    });

    it('GET /auth/smart/launch requires connection parameter', async () => {
      const res = await app.request('/auth/smart/launch');
      const body = await res.json();

      expect(res.status).toBe(400);
      expect(body.error).toBe('Missing connection parameter');
    });

    it('GET /auth/smart/launch redirects to auth URL', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      mockGateway.getAuthorizationUrl.mockResolvedValueOnce(
        'https://fhir.epic.com/authorize?client_id=test'
      );

      const res = await app.request('/auth/smart/launch?connection=test-connection');

      expect(res.status).toBe(302);
      expect(res.headers.get('Location')).toContain('fhir.epic.com');
    });

    it('GET /auth/smart/callback handles error from EHR', async () => {
      const res = await app.request(
        '/auth/smart/callback?error=access_denied&error_description=User%20denied'
      );
      const body = await res.json();

      expect(res.status).toBe(400);
      expect(body.error).toBe('Authorization denied');
    });
  });

  describe('Sync Endpoints', () => {
    it('POST /sync/patient/:id uses the resolved connection without trusting a query selector', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      mockGateway.pullPatientData.mockResolvedValueOnce([]);

      const res = await app.request('/sync/patient/123?tokenKey=attacker-token&sourceSystem=attacker-source', { method: 'POST' });
      const body = await res.json();

      expect(res.status).toBe(200);
      expect(body.patient_id).toBe('123');
      expect(body.connection_id).toBe('test-connection');
      expect(mockGateway.pullPatientData).toHaveBeenCalledWith(
        'test-connection',
        '123',
        'test-credential-handle',
        expect.objectContaining({ sourceSystem: 'epic' }),
      );
    });

    it('POST /sync/patient/:id rejects a caller connection selector that differs from the verified binding', async () => {
      mockGateway.pullPatientData.mockClear();
      const res = await app.request(
        '/sync/patient/123?connection=unknown&tokenKey=attacker-token',
        { method: 'POST' }
      );
      const body = await res.json();

      expect(res.status).toBe(403);
      expect(body.error).toContain('does not match resolved authorization');
      expect(mockGateway.pullPatientData).not.toHaveBeenCalled();
    });

    it('POST /sync/patient/:id rejects a patient selector that differs from the verified binding', async () => {
      mockGateway.pullPatientData.mockClear();
      const res = await app.request('/sync/patient/other-patient?connection=test-connection', {
        method: 'POST',
      });
      expect(res.status).toBe(403);
      expect(mockGateway.pullPatientData).not.toHaveBeenCalled();
    });

    it('POST /sync/bidirectional uses resolved patient/hash/credential and never auto-resolves conflicts', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      mockGateway.syncPatient.mockResolvedValueOnce({
        pullResults: [],
        pushResults: [],
        conflicts: [{ id: 'conflict-visible' }] as never,
      });

      const patientHash = 'uhCEkINI4cBkNicMiyEvxrUXKy71OIGWXqXJagdCunS2SlmaipQ1j';
      const res = await app.request(
        '/sync/bidirectional/123?connection=test-connection&tokenKey=attacker-token&patientHash=' + patientHash,
        { method: 'POST' },
      );

      const responseBody = await res.json();
      expect(res.status, JSON.stringify(responseBody)).toBe(200);
      expect(mockGateway.syncPatient).toHaveBeenCalledWith(
        'test-connection',
        '123',
        expect.any(Uint8Array),
        'test-credential-handle',
        expect.objectContaining({
          pull: expect.objectContaining({ sourceSystem: 'epic' }),
          push: expect.objectContaining({ resourceTypes: expect.any(Array) }),
          autoResolveConflicts: false,
        }),
      );
    });

    it('POST /sync/patient/:id denies async execution until worker-time authorization exists', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);

      const res = await app.request(
        '/sync/patient/123?connection=test-connection&tokenKey=token1&async=true',
        { method: 'POST' }
      );
      const body = await res.json();

      expect(res.status).toBe(503);
      expect(body.error).toBe('Background sync unavailable');
      expect(mockGateway.pullPatientData).not.toHaveBeenCalled();
      expect(ctx.syncJobs.size).toBe(0);
    });

    it('GET /sync/jobs lists sync jobs', async () => {
      const res = await app.request('/sync/jobs');
      const body = await res.json();

      expect(res.status).toBe(200);
      expect(body.jobs).toBeDefined();
      expect(Array.isArray(body.jobs)).toBe(true);
    });

    it('filters job and conflict collections to explicitly authorized resource IDs', async () => {
      const startedAt = new Date('2026-01-01T00:00:00.000Z');
      const visibleJob = {
        id: 'job-visible',
        connectionId: 'test-connection',
        patientId: 'patient-visible',
        status: 'completed' as const,
        startedAt,
      };
      const hiddenJob = {
        ...visibleJob,
        id: 'job-hidden',
        patientId: 'patient-hidden',
      };
      const scopedGateway = {
        ...mockGateway,
        getPendingConflicts: vi.fn().mockReturnValue([
          {
            resourceType: 'Observation',
            resourceId: 'resource-visible',
            localVersion: '1',
            remoteVersion: '2',
            localData: { note: 'visible' },
            remoteData: { note: 'visible-remote' },
            conflictType: 'update',
          },
          {
            resourceType: 'Observation',
            resourceId: 'resource-hidden',
            localVersion: '1',
            remoteVersion: '2',
            localData: { note: 'must-not-leak' },
            remoteData: { note: 'must-not-leak' },
            conflictType: 'update',
          },
        ]),
        getConflictStats: vi.fn(),
      };
      const scopedContext: AppContext = {
        ...ctx,
        gateway: scopedGateway as any,
        syncJobs: new Map([
          [visibleJob.id, visibleJob as any],
          [hiddenJob.id, hiddenJob as any],
        ]),
        gatewayRouteAuthorizer: {
          authorize: vi.fn(async (_request, intent) => ({
            authorized: true as const,
            grantId: 'scoped-grant',
            requestId: 'scoped-request',
            principal: {
              issuer: 'https://idp.example.test',
              subject: 'test-subject',
              clientId: 'test-client',
              organizationId: 'test-org',
            },
            tenantId: 'test-tenant',
            patientBinding: 'opaque-test-patient',
            sourceConnectionId: 'test-connection',
            purpose: 'treatment',
            recipientId: 'mycelix-test',
            requestedResourceTypes: [...(intent.resourceTypes ?? [])],
            action: intent.action,
            expiresAtMs: Date.now() + 60_000,
            resolvedSourcePatientId: '123',
            resolvedMycelixPatientHash: 'patient-123',
            credentialHandle: 'test-credential-handle',
            resolvedJobId: 'job-visible',
            resolvedConflictId: 'conflict-visible',
            authorizedResourceIds: intent.action === 'sync_job_list'
              ? ['job-visible']
              : [JSON.stringify(['Observation', 'resource-visible'])],
          })),
        },
      };
      const scopedApp = createApp(scopedContext);

      const jobsResponse = await scopedApp.request('/sync/jobs');
      const jobsBody = await jobsResponse.json();
      expect(jobsResponse.status).toBe(200);
      expect(jobsBody.total).toBe(1);
      expect(jobsBody.jobs.map((job: { id: string }) => job.id)).toEqual(['job-visible']);

      const conflictsResponse = await scopedApp.request('/sync/conflicts');
      const conflictsBody = await conflictsResponse.json();
      expect(conflictsResponse.status).toBe(200);
      expect(conflictsBody.pending_count).toBe(1);
      expect(conflictsBody.conflicts.map((item: { resourceId: string }) => item.resourceId))
        .toEqual(['resource-visible']);
      expect(JSON.stringify(conflictsBody)).not.toContain('must-not-leak');
      expect(conflictsBody.statistics).toBeUndefined();
    });

    it('GET /sync/conflicts lists pending conflicts', async () => {
      const res = await app.request('/sync/conflicts');
      const body = await res.json();

      expect(res.status).toBe(200);
      expect(body.pending_count).toBeDefined();
      expect(body.conflicts).toBeDefined();
    });
  });

  describe('Error Handling', () => {
    it('returns 404 for unknown routes', async () => {
      const res = await app.request('/unknown/path');
      const body = await res.json();

      expect(res.status).toBe(404);
      expect(body.error).toBe('Not found');
    });
  });
});
