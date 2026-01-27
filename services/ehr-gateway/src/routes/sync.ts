/**
 * Sync Routes
 *
 * Handles patient data synchronization triggers and status tracking.
 * Supports both synchronous and asynchronous sync operations.
 */

import { Hono } from 'hono';
import { randomUUID } from 'crypto';
import type { AppContext } from '../server.js';
import type { SyncJob, IngestReport } from '../types.js';
import { PullService } from '../sync/pull-service.js';

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
    if (!ctx.gateway) {
      return c.json({
        error: 'Gateway not initialized',
      }, 503);
    }

    const patientId = c.req.param('patientId');
    const connectionId = c.req.query('connection');
    const tokenKey = c.req.query('tokenKey');
    const resourceTypesParam = c.req.query('resourceTypes');
    const sourceSystem = c.req.query('sourceSystem');
    const isAsync = c.req.query('async') === 'true';

    // Validate required params
    if (!connectionId) {
      return c.json({
        error: 'Missing connection parameter',
      }, 400);
    }

    if (!tokenKey) {
      return c.json({
        error: 'Missing tokenKey parameter',
      }, 400);
    }

    // Check connection exists
    if (!ctx.gateway.isConnected(connectionId)) {
      return c.json({
        error: 'Connection not found',
        connection_id: connectionId,
      }, 404);
    }

    // Parse resource types
    const resourceTypes = resourceTypesParam
      ? resourceTypesParam.split(',')
      : undefined;

    // For async operations, create a job and return immediately
    if (isAsync) {
      const jobId = randomUUID();
      const job: SyncJob = {
        id: jobId,
        connectionId,
        patientId,
        status: 'pending',
        startedAt: new Date(),
      };
      ctx.syncJobs.set(jobId, job);

      // Start sync in background
      runSyncInBackground(ctx, job, tokenKey, resourceTypes, sourceSystem);

      return c.json({
        status: 'accepted',
        job_id: jobId,
        message: 'Sync started in background',
        status_url: `/sync/status/${jobId}`,
      }, 202);
    }

    // Synchronous sync
    try {
      const result = await ctx.gateway.pullPatientData(
        connectionId,
        patientId,
        tokenKey,
        {
          resourceTypes,
          sourceSystem,
        }
      );

      return c.json({
        status: 'success',
        patient_id: patientId,
        connection_id: connectionId,
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
  app.get('/status/:jobId', (c) => {
    const jobId = c.req.param('jobId');
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
  app.get('/jobs', (c) => {
    const statusFilter = c.req.query('status');
    const limit = parseInt(c.req.query('limit') || '100', 10);

    let jobs = Array.from(ctx.syncJobs.values());

    // Filter by status
    if (statusFilter) {
      jobs = jobs.filter(job => job.status === statusFilter);
    }

    // Sort by start time (newest first)
    jobs.sort((a, b) => b.startedAt.getTime() - a.startedAt.getTime());

    // Apply limit
    jobs = jobs.slice(0, limit);

    return c.json({
      total: ctx.syncJobs.size,
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
  app.delete('/jobs/:jobId', (c) => {
    const jobId = c.req.param('jobId');
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
    if (!ctx.gateway) {
      return c.json({
        error: 'Gateway not initialized',
      }, 503);
    }

    const patientId = c.req.param('patientId');
    const connectionId = c.req.query('connection');
    const tokenKey = c.req.query('tokenKey');
    const patientHash = c.req.query('patientHash');

    if (!connectionId || !tokenKey || !patientHash) {
      return c.json({
        error: 'Missing required parameters',
        required: ['connection', 'tokenKey', 'patientHash'],
      }, 400);
    }

    if (!ctx.gateway.isConnected(connectionId)) {
      return c.json({
        error: 'Connection not found',
        connection_id: connectionId,
      }, 404);
    }

    try {
      // Convert patientHash string to Uint8Array
      const hashBytes = new Uint8Array(
        Buffer.from(patientHash, 'base64')
      );

      const result = await ctx.gateway.syncPatient(
        connectionId,
        patientId,
        hashBytes,
        tokenKey
      );

      return c.json({
        status: 'success',
        patient_id: patientId,
        connection_id: connectionId,
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
  app.get('/conflicts', (c) => {
    if (!ctx.gateway) {
      return c.json({
        error: 'Gateway not initialized',
      }, 503);
    }

    const conflicts = ctx.gateway.getPendingConflicts();
    const stats = ctx.gateway.getConflictStats();

    return c.json({
      pending_count: conflicts.length,
      conflicts,
      statistics: stats,
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
    if (!ctx.gateway) {
      return c.json({
        error: 'Gateway not initialized',
      }, 503);
    }

    const conflictId = c.req.param('conflictId');
    const body = await c.req.json();

    const { strategy, resolved_by, manual_data } = body;

    if (!strategy || !resolved_by) {
      return c.json({
        error: 'Missing required fields',
        required: ['strategy', 'resolved_by'],
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
      await ctx.gateway.resolveConflict(conflictId, strategy, resolved_by, manual_data);

      return c.json({
        status: 'success',
        conflict_id: conflictId,
        strategy,
        resolved_by,
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

/**
 * Run sync operation in background and update job status
 */
async function runSyncInBackground(
  ctx: AppContext,
  job: SyncJob,
  tokenKey: string,
  resourceTypes?: string[],
  sourceSystem?: string
): Promise<void> {
  // Update job status
  job.status = 'running';
  ctx.syncJobs.set(job.id, job);

  try {
    const result = await ctx.gateway!.pullPatientData(
      job.connectionId,
      job.patientId,
      tokenKey,
      {
        resourceTypes,
        sourceSystem,
      }
    );

    // Update job with results
    job.status = 'completed';
    job.completedAt = new Date();
    job.results = result;
    ctx.syncJobs.set(job.id, job);
  } catch (error) {
    // Update job with error
    job.status = 'failed';
    job.completedAt = new Date();
    job.error = (error as Error).message;
    ctx.syncJobs.set(job.id, job);
  }
}
