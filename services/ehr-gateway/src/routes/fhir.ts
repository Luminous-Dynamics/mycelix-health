// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
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
import { requireGatewayRouteAuthorizationV1 } from '../auth/gateway-route-guard.js';

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
   *   X-Source-System: ignored for provenance; source identity is resolved from the authenticated connection.
   *   Content-Type: application/fhir+json or application/json
   *
   * Body: FHIR R4 Bundle
   *
   * Returns: IngestReport
   */
  app.post('/Bundle', async (c) => {
    const authorization = await requireGatewayRouteAuthorizationV1(
      ctx.gatewayRouteAuthorizer,
      c.req.raw,
      {
        action: 'fhir_ingest',
        resourceTypes: [
          'Patient', 'Observation', 'Condition', 'MedicationRequest', 'MedicationStatement',
          'AllergyIntolerance', 'Immunization', 'Procedure', 'DiagnosticReport', 'CarePlan',
        ],
      },
    );
    if (!authorization.authorized) return authorization.response;

    // Resolve the EHR source from the verified request context. Never trust
    // X-Source-System as provenance metadata.
    if (!ctx.holochainClient || !ctx.gateway) {
      return c.json({
        error: 'Gateway unavailable',
        message: 'A registered EHR connection and Holochain are required for verified ingestion.',
      }, 503);
    }
    const sourceConnectionId = authorization.authorization.sourceConnectionId;
    if (!ctx.gateway.isConnected(sourceConnectionId)) {
      return c.json({ error: 'Authorized source connection is unavailable' }, 503);
    }
    const sourceSystem = ctx.gateway.getConnectionInfo(sourceConnectionId).system;
    const authorizedResourceTypes = new Set(authorization.authorization.requestedResourceTypes);

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

      // A Bundle authorization must cover every contained resource type and
      // every declared Patient reference. Do not let a broad Bundle selector
      // become permission for arbitrary patient resources.
      const entries = Array.isArray(bundle.entry) ? bundle.entry : [];
      for (const entry of entries) {
        const resource = entry?.resource as {
          resourceType?: unknown;
          id?: unknown;
          subject?: { reference?: unknown };
          patient?: { reference?: unknown };
          context?: { reference?: unknown };
        } | undefined;
        if (!resource || typeof resource.resourceType !== 'string'
          || !authorizedResourceTypes.has(resource.resourceType)) {
          return c.json({ error: 'Bundle contains an unauthorized resource type' }, 403);
        }
        if (resource.resourceType === 'Patient'
          && typeof resource.id === 'string'
          && resource.id !== authorization.authorization.resolvedSourcePatientId) {
          return c.json({ error: 'Bundle Patient does not match the authorized source patient' }, 403);
        }
        const patientReferences = [
          resource.subject?.reference,
          resource.patient?.reference,
          resource.context?.reference,
        ].filter((value): value is string => typeof value === 'string');
        if (patientReferences.some((reference) => {
          const match = /^Patient\/([^/]+)(?:\/|$)/.exec(reference);
          return match !== null && match[1] !== authorization.authorization.resolvedSourcePatientId;
        })) {
          return c.json({ error: 'Bundle contains a resource for a different patient' }, 403);
        }
      }
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
    const authorization = await requireGatewayRouteAuthorizationV1(
      ctx.gatewayRouteAuthorizer,
      c.req.raw,
      {
        action: 'fhir_export',
        resourceTypes: ['Patient', 'Observation', 'Condition', 'MedicationRequest'],
      },
    );
    if (!authorization.authorized) return authorization.response;

    if (!ctx.holochainClient) {
      return c.json({
        error: 'Holochain not connected',
      }, 503);
    }

    const requestedPatientHash = c.req.param('patientHash');
    const patientHash = authorization.authorization.resolvedMycelixPatientHash!;
    if (requestedPatientHash !== patientHash) {
      return c.json({ error: 'Requested patient does not match the authorized patient binding' }, 403);
    }

    // The fhir_bridge zome accepts FHIR resource type names, not gateway
    // section aliases. Restrict to the three sections the zome actually exports;
    // unknown aliases must not result in its fallback DataCategory::All behavior.
    const sectionToResourceType: Record<string, string> = {
      conditions: 'Condition',
      medications: 'MedicationRequest',
      observations: 'Observation',
    };
    const authorizedTypes = new Set(authorization.authorization.requestedResourceTypes);
    const sectionsParam = c.req.query('sections');
    const requestedSectionAliases = sectionsParam
      ? sectionsParam.split(',').map((section) => section.trim()).filter(Boolean)
      : Object.entries(sectionToResourceType)
        .filter(([, resourceType]) => authorizedTypes.has(resourceType))
        .map(([section]) => section);
    if (
      requestedSectionAliases.some((section) =>
        !sectionToResourceType[section] || !authorizedTypes.has(sectionToResourceType[section]!)
      )
    ) {
      // The current FHIR zome treats an empty include_sections list as DataCategory::All.
      // Never allow an empty query/default grant to fall through to a blanket export.
      return c.json({ error: 'At least one explicitly authorized FHIR section is required' }, 403);
    }
    // Patient is part of every FHIR export. With no additional authorized
    // section, this is a legitimate demographics-only export, not DataCategory::All.
    const includeSections = [
      'Patient',
      ...requestedSectionAliases.map((section) => sectionToResourceType[section]!),
    ];

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
