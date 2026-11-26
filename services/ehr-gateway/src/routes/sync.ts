// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
/**
 * Sync Routes
 *
 * Handles patient data synchronization triggers and status tracking.
 * Supports both synchronous and asynchronous sync operations.
 */

import { Hono } from 'hono';
import type { AppContext } from '../server.js';
import { requireGatewayRouteAuthorizationV1 } from '../auth/gateway-route-guard.js';

const SUPPORTED_SYNC_RESOURCE_TYPES = [
  'Patient',
  'Observation',
  'Condition',
  'MedicationRequest',
  'AllergyIntolerance',
  'Immunization',
  'Procedure',
] as const;

/**
 * Create sync routes
 */
export function createSyncRoutes(ctx: AppContext): Hono {
  const app = new Hono();

  /**
   * POST /sync/patient/:patientId
   *
   * Trigger a patient data sync from an EHR.
   *
   * URL params:
   *   patientId: Patient ID in the EHR system
   *
   * Query params:
   *   connection: Connection ID (required)
   *   tokenKey: Key for stored token (required)
   *   resourceTypes: Comma-separated resource types (optional)
   *   sourceSystem: Override source system identifier (optional)
   *   async: If true, return immediately with a job ID (optional)
   *
   * Returns: SyncResult or job ID (if async)
   */
  app.post('/patient/:patientId', async (c) => {
    const resourceTypesParam = c.req.query('resourceTypes');
    const requestedResourceTypes = resourceTypesParam
      ? resourceTypesParam.split(',').map((value) => value.trim()).filter(Boolean)
      : [...SUPPORTED_SYNC_RESOURCE_TYPES];
    if (
      requestedResourceTypes.length === 0
      || requestedResourceTypes.some((resourceType) =>
        !(SUPPORTED_SYNC_RESOURCE_TYPES as readonly string[]).includes(resourceType)
      )
    ) {
      return c.json({ error: 'Unsupported or empty resourceTypes selection' }, 400);
    }
    const authorization = await requireGatewayRouteAuthorizationV1(
      ctx.gatewayRouteAuthorizer,
      c.req.raw,
      { action: 'sync_pull', resourceTypes: requestedResourceTypes },
    );
    if (!authorization.authorized) return authorization.response;

    if (!ctx.gateway) {
      return c.json({ error: 'Gateway not initialized' }, 503);
    }

    const requestedPatientId = c.req.param('patientId');
    const requestedConnectionId = c.req.query('connection');
    const connectionId = authorization.authorization.sourceConnectionId;
    const patientId = authorization.authorization.resolvedSourcePatientId!;
    const credentialHandle = authorization.authorization.credentialHandle!;
    const resourceTypes = authorization.authorization.requestedResourceTypes;
    const isAsync = c.req.query('async') === 'true';

    // Legacy selectors are lookup hints only. The downstream adapter always
    // receives the canonical patient, connection, credential and source returned
    // by the trusted authorization provider.
    if (requestedPatientId !== patientId
      || (requestedConnectionId && requestedConnectionId !== connectionId)) {
      return c.json({ error: 'Requested patient or connection does not match resolved authorization' }, 403);
    }
    if (!ctx.gateway.isConnected(connectionId)) {
      return c.json({ error: 'Authorized source connection is unavailable' }, 503);
    }
    const sourceSystem = ctx.gateway.getConnectionInfo(connectionId).system;

    // Background execution remains disabled until the worker has its own fresh,
    // consume-once authorization receipt and durable operation reconciliation.
    if (isAsync) {
      return c.json({
        error: 'Background sync unavailable',
        message: 'Async execution is disabled until worker-time authorization and replay protection are configured.',
      }, 503);
    }

    try {
      const result = await ctx.gateway.pullPatientData(
        connectionId,
        patientId,
        credentialHandle,
        {
          resourceTypes,
          sourceSystem,
        },
      );

      return c.json({
        status: 'success',
        patient_id: patientId,
        connection_id: connectionId,
        source_system: sourceSystem,
        results: result,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      console.error('Sync failed:', error);
      return c.json({
        error: 'Sync failed',
        message: (error as Error).message,
        patient_id: patientId,
        connection_id: connectionId,
      }, 500);
    }
  });

  /**
   * GET /sync/status/:jobId
   *
   * Check the status of an async sync job.
   *
   * Returns: SyncJob status
   */
  app.get('/status/:jobId', async (c) => {
    const authorization = await requireGatewayRouteAuthorizationV1(
      ctx.gatewayRouteAuthorizer,
      c.req.raw,
      { action: 'sync_status', resourceTypes: [] },
    );
    if (!authorization.authorized) return authorization.response;

    const requestedJobId = c.req.param('jobId');
    const jobId = authorization.authorization.resolvedJobId!;
    if (requestedJobId !== jobId) {
      return c.json({ error: 'Job selector does not match authorized job' }, 403);
    }
    const job = ctx.syncJobs.get(jobId);

    if (!job) {
      return c.json({
        error: 'Job not found',
        job_id: jobId,
      }, 404);
    }

    return c.json({
      job_id: job.id,
      connection_id: job.connectionId,
      patient_id: job.patientId,
      status: job.status,
      started_at: job.startedAt.toISOString(),
      completed_at: job.completedAt?.toISOString(),
      results: job.results,
      ingest_report: job.ingestReport,
      error: job.error,
    });
  });

  /**
   * GET /sync/jobs
   *
   * List all sync jobs.
   *
   * Query params:
   *   status: Filter by status (pending, running, completed, failed)
   *   limit: Maximum number of jobs to return (default 100)
   *
   * Returns: Array of SyncJob summaries
   */
  app.get('/jobs', async (c) => {
    const authorization = await requireGatewayRouteAuthorizationV1(
      ctx.gatewayRouteAuthorizer,
      c.req.raw,
      { action: 'sync_job_list', resourceTypes: [], requireScopedResourceList: true },
    );
    if (!authorization.authorized) return authorization.response;

    const statusFilter = c.req.query('status');
    const limit = parseInt(c.req.query('limit') || '100', 10);

    const visibleJobIds = new Set(authorization.authorizedResourceIds ?? []);
    let jobs = Array.from(ctx.syncJobs.values())
      .filter((job) => visibleJobIds.has(job.id));

    // Filter by status
    if (statusFilter) {
      jobs = jobs.filter(job => job.status === statusFilter);
    }

    // Sort by start time (newest first)
    jobs.sort((a, b) => b.startedAt.getTime() - a.startedAt.getTime());

    // Count only authorized jobs before pagination; never leak global totals.
    const authorizedTotal = jobs.length;
    jobs = jobs.slice(0, limit);

    return c.json({
      total: authorizedTotal,
      returned: jobs.length,
      jobs: jobs.map(job => ({
        id: job.id,
        connection_id: job.connectionId,
        patient_id: job.patientId,
        status: job.status,
        started_at: job.startedAt.toISOString(),
        completed_at: job.completedAt?.toISOString(),
        has_errors: !!job.error,
      })),
    });
  });

  /**
   * DELETE /sync/jobs/:jobId
   *
   * Delete a sync job from the tracking list.
   * Only completed or failed jobs can be deleted.
   *
   * Returns: Success message
   */
  app.delete('/jobs/:jobId', async (c) => {
    const authorization = await requireGatewayRouteAuthorizationV1(
      ctx.gatewayRouteAuthorizer,
      c.req.raw,
      { action: 'sync_job_delete', resourceTypes: [] },
    );
    if (!authorization.authorized) return authorization.response;

    const requestedJobId = c.req.param('jobId');
    const jobId = authorization.authorization.resolvedJobId!;
    if (requestedJobId !== jobId) {
      return c.json({ error: 'Job selector does not match authorized job' }, 403);
    }
    const job = ctx.syncJobs.get(jobId);

    if (!job) {
      return c.json({
        error: 'Job not found',
        job_id: jobId,
      }, 404);
    }

    if (job.status === 'pending' || job.status === 'running') {
      return c.json({
        error: 'Cannot delete active job',
        message: 'Job is still running. Wait for completion or implement cancellation.',
        status: job.status,
      }, 409);
    }

    ctx.syncJobs.delete(jobId);

    return c.json({
      status: 'success',
      message: 'Job deleted',
      job_id: jobId,
    });
  });

  /**
   * POST /sync/bidirectional/:patientId
   *
   * Perform bidirectional sync (pull from EHR, then push local changes).
   *
   * URL params:
   *   patientId: Patient ID in the EHR system
   *
   * Query params:
   *   connection: Connection ID (required)
   *   tokenKey: Key for stored token (required)
   *   patientHash: Holochain patient hash (required for push)
   *
   * Returns: Combined sync results
   */
  app.post('/bidirectional/:patientId', async (c) => {
    const authorization = await requireGatewayRouteAuthorizationV1(
      ctx.gatewayRouteAuthorizer,
      c.req.raw,
      {
        action: 'sync_bidirectional',
        resourceTypes: [
          'Patient', 'Observation', 'Condition', 'MedicationRequest',
          'AllergyIntolerance', 'Immunization', 'Procedure',
        ],
      },
    );
    if (!authorization.authorized) return authorization.response;

    if (!ctx.gateway) {
      return c.json({
        error: 'Gateway not initialized',
      }, 503);
    }

    const requestedPatientId = c.req.param('patientId');
    const requestedConnectionId = c.req.query('connection');
    const requestedPatientHash = c.req.query('patientHash');
    const connectionId = authorization.authorization.sourceConnectionId;
    const patientId = authorization.authorization.resolvedSourcePatientId!;
    const patientHash = authorization.authorization.resolvedMycelixPatientHash!;
    const credentialHandle = authorization.authorization.credentialHandle!;
    const resourceTypes = authorization.authorization.requestedResourceTypes;

    if (
      requestedPatientId !== patientId
      || requestedPatientHash !== patientHash
      || (requestedConnectionId && requestedConnectionId !== connectionId)
    ) {
      return c.json({ error: 'Requested patient or connection does not match resolved authorization' }, 403);
    }
    if (!ctx.gateway.isConnected(connectionId)) {
      return c.json({ error: 'Authorized source connection is unavailable' }, 503);
    }
    const sourceSystem = ctx.gateway.getConnectionInfo(connectionId).system;

    try {
      const hashBytes = new Uint8Array(Buffer.from(patientHash, 'base64'));
      if (hashBytes.length !== 39) {
        return c.json({ error: 'Resolved patient hash is malformed' }, 403);
      }

      const result = await ctx.gateway.syncPatient(
        connectionId,
        patientId,
        hashBytes,
        credentialHandle,
        {
          pull: { resourceTypes, sourceSystem },
        },
      );

      return c.json({
        status: 'success',
        patient_id: patientId,
        connection_id: connectionId,
        source_system: sourceSystem,
        pull_results: result.pullResults,
        push_results: result.pushResults,
        conflicts: result.conflicts,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      console.error('Bidirectional sync failed:', error);
      return c.json({
        error: 'Sync failed',
        message: (error as Error).message,
      }, 500);
    }
  });

  /**
   * GET /sync/conflicts
   *
   * List pending sync conflicts.
   *
   * Returns: Array of ConflictInfo
   */
  app.get('/conflicts', async (c) => {
    const authorization = await requireGatewayRouteAuthorizationV1(
      ctx.gatewayRouteAuthorizer,
      c.req.raw,
      { action: 'sync_conflicts_read', resourceTypes: [], requireScopedResourceList: true },
    );
    if (!authorization.authorized) return authorization.response;

    if (!ctx.gateway) {
      return c.json({
        error: 'Gateway not initialized',
      }, 503);
    }

    const visibleConflictKeys = new Set(authorization.authorizedResourceIds ?? []);
    const conflicts = ctx.gateway.getPendingConflicts().filter((conflict) =>
      visibleConflictKeys.has(JSON.stringify([conflict.resourceType, conflict.resourceId]))
    );

    return c.json({
      // Do not return unfiltered global conflict counts or statistics.
      pending_count: conflicts.length,
      conflicts,
    });
  });

  /**
   * POST /sync/conflicts/:conflictId/resolve
   *
   * Resolve a sync conflict.
   *
   * Body:
   *   {
   *     strategy: 'local_wins' | 'remote_wins' | 'most_recent' | 'merge' | 'manual',
   *     resolved_by: string,
   *     manual_data?: any (required if strategy is 'manual')
   *   }
   *
   * Returns: Resolution result
   */
  app.post('/conflicts/:conflictId/resolve', async (c) => {
    const authorization = await requireGatewayRouteAuthorizationV1(
      ctx.gatewayRouteAuthorizer,
      c.req.raw,
      { action: 'sync_conflict_resolve', resourceTypes: [] },
    );
    if (!authorization.authorized) return authorization.response;

    if (!ctx.gateway) {
      return c.json({
        error: 'Gateway not initialized',
      }, 503);
    }

    const requestedConflictId = c.req.param('conflictId');
    const conflictId = authorization.authorization.resolvedConflictId!;
    if (requestedConflictId !== conflictId) {
      return c.json({ error: 'Conflict selector does not match authorized conflict' }, 403);
    }
    const body = await c.req.json();

    const { strategy, manual_data } = body;
    const resolvedBy = authorization.authorization.principal.subject;

    if (!strategy) {
      return c.json({
        error: 'Missing required fields',
        required: ['strategy'],
      }, 400);
    }

    const validStrategies = ['local_wins', 'remote_wins', 'most_recent', 'merge', 'manual'];
    if (!validStrategies.includes(strategy)) {
      return c.json({
        error: 'Invalid strategy',
        valid_strategies: validStrategies,
      }, 400);
    }

    if (strategy === 'manual' && !manual_data) {
      return c.json({
        error: 'manual_data required for manual strategy',
      }, 400);
    }

    try {
      await ctx.gateway.resolveConflict(conflictId, strategy, resolvedBy, manual_data);

      return c.json({
        status: 'success',
        conflict_id: conflictId,
        strategy,
        resolved_by: resolvedBy,
      });
    } catch (error) {
      console.error('Conflict resolution failed:', error);
      return c.json({
        error: 'Resolution failed',
        message: (error as Error).message,
      }, 500);
    }
  });

  return app;
}
