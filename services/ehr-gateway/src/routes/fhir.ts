/**
 * FHIR Bundle Routes
 *
 * Handles FHIR Bundle ingestion from external EHR systems.
 * Compatible with EHR webhook notifications.
 */

import { Hono } from 'hono';
import { z } from 'zod';
import type { AppContext } from '../server.js';
import { FhirBundleSchema, type FhirBundle, type IngestReport } from '../types.js';
import { PullService } from '../sync/pull-service.js';

/**
 * Create FHIR routes
 */
export function createFhirRoutes(ctx: AppContext): Hono {
  const app = new Hono();

  /**
   * POST /fhir/Bundle
   *
   * Ingest a FHIR Bundle into Holochain.
   *
   * Headers:
   *   X-Source-System: Source system identifier (e.g., "epic-mychart")
   *   Content-Type: application/fhir+json or application/json
   *
   * Body: FHIR R4 Bundle
   *
   * Returns: IngestReport
   */
  app.post('/Bundle', async (c) => {
    // Check Holochain connection
    if (!ctx.holochainClient) {
      return c.json({
        error: 'Holochain not connected',
        message: 'The server is not connected to Holochain. Please try again later.',
      }, 503);
    }

    // Get source system from header
    const sourceSystem = c.req.header('X-Source-System') || 'unknown';

    // Parse and validate bundle
    let bundle: FhirBundle;
    try {
      const body = await c.req.json();

      // Validate it's a Bundle
      if (body.resourceType !== 'Bundle') {
        return c.json({
          error: 'Invalid resource type',
          message: `Expected Bundle, got ${body.resourceType}`,
        }, 400);
      }

      // Parse with Zod for validation
      bundle = FhirBundleSchema.parse(body);
    } catch (error) {
      if (error instanceof z.ZodError) {
        return c.json({
          error: 'Invalid FHIR Bundle',
          details: error.errors,
        }, 400);
      }
      return c.json({
        error: 'Invalid JSON',
        message: (error as Error).message,
      }, 400);
    }

    // Create a temporary PullService for ingestion
    const pullService = new PullService({
      holochainClient: ctx.holochainClient,
      fhirAdapter: null as any, // Not needed for direct bundle ingestion
      defaultSourceSystem: sourceSystem,
    });

    try {
      // Ingest the bundle
      const report = await pullService.ingestBundle(bundle, sourceSystem);

      // Return the ingestion report
      return c.json({
        status: 'success',
        source_system: sourceSystem,
        report,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      console.error('Bundle ingestion failed:', error);
      return c.json({
        error: 'Ingestion failed',
        message: (error as Error).message,
        source_system: sourceSystem,
      }, 500);
    }
  });

  /**
   * GET /fhir/Bundle/:patientHash
   *
   * Export patient data as a FHIR Bundle.
   *
   * Query params:
   *   sections: Comma-separated list of sections to include
   *             (conditions, medications, allergies, observations, procedures, immunizations)
   *
   * Returns: FHIR R4 Bundle
   */
  app.get('/Bundle/:patientHash', async (c) => {
    if (!ctx.holochainClient) {
      return c.json({
        error: 'Holochain not connected',
      }, 503);
    }

    const patientHash = c.req.param('patientHash');
    const sectionsParam = c.req.query('sections');
    const includeSections = sectionsParam ? sectionsParam.split(',') : [];

    try {
      const result = await ctx.holochainClient.callZome({
        cap_secret: undefined,
        role_name: 'health',
        zome_name: 'fhir_bridge',
        fn_name: 'export_patient_fhir',
        payload: {
          patient_hash: patientHash,
          include_sections: includeSections,
        },
      });

      const exportResult = result as { bundle: string; resource_count: number; export_timestamp: number };

      // Parse the bundle string and return as JSON
      const bundle = JSON.parse(exportResult.bundle);

      // Set FHIR content type
      c.header('Content-Type', 'application/fhir+json');

      return c.json(bundle);
    } catch (error) {
      console.error('Export failed:', error);
      return c.json({
        error: 'Export failed',
        message: (error as Error).message,
      }, 500);
    }
  });

  /**
   * GET /fhir/metadata
   *
   * Returns FHIR capability statement (conformance).
   * Useful for EHR systems that query server capabilities.
   */
  app.get('/metadata', (c) => {
    c.header('Content-Type', 'application/fhir+json');

    return c.json({
      resourceType: 'CapabilityStatement',
      status: 'active',
      date: new Date().toISOString(),
      publisher: 'Mycelix Health',
      kind: 'instance',
      software: {
        name: 'Mycelix EHR Gateway',
        version: '0.2.0',
      },
      implementation: {
        description: 'Mycelix Health FHIR Gateway - Sovereign Health Records',
        url: ctx.config.oauthCallbackBaseUrl || 'http://localhost:3000',
      },
      fhirVersion: '4.0.1',
      format: ['json'],
      rest: [{
        mode: 'server',
        resource: [
          {
            type: 'Patient',
            interaction: [
              { code: 'read' },
              { code: 'search-type' },
            ],
          },
          {
            type: 'Observation',
            interaction: [
              { code: 'read' },
              { code: 'search-type' },
            ],
          },
          {
            type: 'Condition',
            interaction: [
              { code: 'read' },
              { code: 'search-type' },
            ],
          },
          {
            type: 'MedicationRequest',
            interaction: [
              { code: 'read' },
              { code: 'search-type' },
            ],
          },
          {
            type: 'AllergyIntolerance',
            interaction: [
              { code: 'read' },
              { code: 'search-type' },
            ],
          },
          {
            type: 'Immunization',
            interaction: [
              { code: 'read' },
              { code: 'search-type' },
            ],
          },
          {
            type: 'Procedure',
            interaction: [
              { code: 'read' },
              { code: 'search-type' },
            ],
          },
          {
            type: 'Bundle',
            interaction: [
              { code: 'create' },
            ],
          },
        ],
      }],
    });
  });

  return app;
}
