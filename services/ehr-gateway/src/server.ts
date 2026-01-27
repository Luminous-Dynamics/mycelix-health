/**
 * EHR Gateway HTTP Server
 *
 * Exposes REST endpoints for EHR integration:
 * - FHIR Bundle ingestion (webhook-compatible)
 * - SMART on FHIR OAuth flow
 * - Patient sync triggers
 * - Health checks
 */

import { Hono } from 'hono';
import { serve } from '@hono/node-server';
import { cors } from 'hono/cors';
import { logger } from 'hono/logger';
import { AppWebsocket, type AppClient } from '@holochain/client';

import { loadServerConfig, type ServerConfig } from './config.js';
import { EhrGateway, type EhrGatewayConfig } from './gateway.js';
import { createFhirRoutes } from './routes/fhir.js';
import { createAuthRoutes } from './routes/auth.js';
import { createSyncRoutes } from './routes/sync.js';
import type { SyncJob, IngestReport } from './types.js';

// ============================================================================
// App Context Types
// ============================================================================

export interface AppContext {
  gateway: EhrGateway | null;
  holochainClient: AppClient | null;
  config: ServerConfig;
  syncJobs: Map<string, SyncJob>;
}

// ============================================================================
// Server Setup
// ============================================================================

/**
 * Create the Hono application with all routes
 */
export function createApp(ctx: AppContext): Hono {
  const app = new Hono();

  // Middleware
  if (ctx.config.enableRequestLogging) {
    app.use('*', logger());
  }

  // CORS
  app.use('*', cors({
    origin: ctx.config.corsOrigins,
    allowMethods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
    allowHeaders: ['Content-Type', 'Authorization', 'X-Source-System'],
    exposeHeaders: ['X-Request-Id'],
    credentials: true,
    maxAge: 86400,
  }));

  // Health check endpoint
  app.get('/health', (c) => {
    return c.json({
      status: 'ok',
      service: 'ehr-gateway',
      version: '0.2.0',
      holochain: ctx.holochainClient ? 'connected' : 'disconnected',
      timestamp: new Date().toISOString(),
    });
  });

  // Ready check (requires Holochain connection)
  app.get('/ready', (c) => {
    if (!ctx.holochainClient) {
      return c.json({ status: 'not ready', reason: 'Holochain not connected' }, 503);
    }
    return c.json({ status: 'ready' });
  });

  // Mount route modules
  app.route('/fhir', createFhirRoutes(ctx));
  app.route('/auth', createAuthRoutes(ctx));
  app.route('/sync', createSyncRoutes(ctx));

  // Error handler
  app.onError((err, c) => {
    console.error('Request error:', err);
    return c.json({
      error: err.message || 'Internal server error',
      timestamp: new Date().toISOString(),
    }, 500);
  });

  // 404 handler
  app.notFound((c) => {
    return c.json({
      error: 'Not found',
      path: c.req.path,
    }, 404);
  });

  return app;
}

/**
 * Connect to Holochain conductor
 */
async function connectHolochain(config: ServerConfig): Promise<AppClient | null> {
  try {
    console.log(`Connecting to Holochain at ${config.holochainUrl}...`);
    const client = await AppWebsocket.connect({
      url: new URL(config.holochainUrl),
    });
    console.log('Connected to Holochain successfully');
    return client;
  } catch (error) {
    console.error('Failed to connect to Holochain:', error);
    console.warn('Server will start without Holochain connection');
    return null;
  }
}

/**
 * Main entry point
 */
async function main(): Promise<void> {
  const config = loadServerConfig();

  console.log('='.repeat(60));
  console.log('EHR Gateway Server starting...');
  console.log('='.repeat(60));
  console.log(`Port: ${config.port}`);
  console.log(`Holochain URL: ${config.holochainUrl}`);
  console.log(`Holochain App ID: ${config.holochainAppId}`);
  console.log(`Log Level: ${config.logLevel}`);
  console.log('='.repeat(60));

  // Connect to Holochain
  const holochainClient = await connectHolochain(config);

  // Create gateway if Holochain is connected
  let gateway: EhrGateway | null = null;
  if (holochainClient) {
    const gatewayConfig: EhrGatewayConfig = {
      holochainClient,
      defaultTimeout: config.defaultTimeout,
      maxRetries: config.maxRetries,
    };
    gateway = new EhrGateway(gatewayConfig);
    console.log('EHR Gateway initialized');
  }

  // Create app context
  const ctx: AppContext = {
    gateway,
    holochainClient,
    config,
    syncJobs: new Map(),
  };

  // Create app
  const app = createApp(ctx);

  // Start server
  const server = serve({
    fetch: app.fetch,
    port: config.port,
    hostname: config.host,
  }, (info) => {
    console.log('='.repeat(60));
    console.log(`EHR Gateway listening on http://${info.address}:${info.port}`);
    console.log('='.repeat(60));
    console.log('Endpoints:');
    console.log('  GET  /health              - Health check');
    console.log('  GET  /ready               - Readiness check');
    console.log('  POST /fhir/Bundle         - Ingest FHIR Bundle');
    console.log('  GET  /auth/smart/launch   - SMART on FHIR launch');
    console.log('  GET  /auth/smart/callback - OAuth callback');
    console.log('  POST /sync/patient/:id    - Trigger patient sync');
    console.log('  GET  /sync/status/:syncId - Check sync status');
    console.log('='.repeat(60));
  });

  // Graceful shutdown
  const shutdown = async () => {
    console.log('\nShutting down gracefully...');

    // Close Holochain connection
    if (holochainClient && 'close' in holochainClient) {
      try {
        await (holochainClient as any).close();
        console.log('Holochain connection closed');
      } catch (error) {
        console.error('Error closing Holochain connection:', error);
      }
    }

    process.exit(0);
  };

  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);
}

// Run only when executed directly (not when imported for testing)
// Check if this file is being run directly via URL comparison
const isMainModule = import.meta.url.endsWith('server.js') ||
                     import.meta.url.endsWith('server.ts');
const isTestEnvironment = process.env.NODE_ENV === 'test' ||
                          process.env.VITEST === 'true';

if (isMainModule && !isTestEnvironment) {
  main().catch((error) => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}

// Export for testing
export { main };
