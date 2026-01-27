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
          'X-Source-System': 'test-ehr',
        },
        body: JSON.stringify(bundle),
      });

      const body = await res.json();

      expect(res.status).toBe(200);
      expect(body.status).toBe('success');
      expect(body.source_system).toBe('test-ehr');
      expect(body.report).toBeDefined();
    });

    it('POST /fhir/Bundle rejects invalid bundle', async () => {
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
    it('POST /sync/patient/:id requires connection parameter', async () => {
      const res = await app.request('/sync/patient/123', {
        method: 'POST',
      });
      const body = await res.json();

      expect(res.status).toBe(400);
      expect(body.error).toBe('Missing connection parameter');
    });

    it('POST /sync/patient/:id returns 404 for unknown connection', async () => {
      mockGateway.isConnected.mockReturnValueOnce(false);

      const res = await app.request(
        '/sync/patient/123?connection=unknown&tokenKey=token1',
        { method: 'POST' }
      );
      const body = await res.json();

      expect(res.status).toBe(404);
      expect(body.error).toBe('Connection not found');
    });

    it('POST /sync/patient/:id with async=true returns job ID', async () => {
      mockGateway.isConnected.mockReturnValueOnce(true);
      mockGateway.pullPatientData.mockResolvedValueOnce([]);

      const res = await app.request(
        '/sync/patient/123?connection=test-connection&tokenKey=token1&async=true',
        { method: 'POST' }
      );
      const body = await res.json();

      expect(res.status).toBe(202);
      expect(body.status).toBe('accepted');
      expect(body.job_id).toBeDefined();
      expect(body.status_url).toContain('/sync/status/');
    });

    it('GET /sync/jobs lists sync jobs', async () => {
      const res = await app.request('/sync/jobs');
      const body = await res.json();

      expect(res.status).toBe(200);
      expect(body.jobs).toBeDefined();
      expect(Array.isArray(body.jobs)).toBe(true);
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
