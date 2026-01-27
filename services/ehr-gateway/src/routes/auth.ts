/**
 * SMART on FHIR Authentication Routes
 *
 * Handles the OAuth2 authorization flow for SMART on FHIR.
 * Enables secure connection to Epic, Cerner, and other EHR systems.
 */

import { Hono } from 'hono';
import type { AppContext } from '../server.js';

/**
 * Create authentication routes
 */
export function createAuthRoutes(ctx: AppContext): Hono {
  const app = new Hono();

  /**
   * GET /auth/smart/launch
   *
   * Initiates the SMART on FHIR authorization flow.
   *
   * Query params:
   *   connection: Connection ID (configured in environment)
   *   launch: Optional SMART launch context (for EHR launch)
   *   redirect_uri: Optional custom redirect URI
   *
   * Returns: 302 redirect to EHR authorization URL
   */
  app.get('/smart/launch', async (c) => {
    if (!ctx.gateway) {
      return c.json({
        error: 'Gateway not initialized',
        message: 'EHR Gateway is not connected to Holochain',
      }, 503);
    }

    const connectionId = c.req.query('connection');
    const launchContext = c.req.query('launch');
    const redirectUri = c.req.query('redirect_uri');

    if (!connectionId) {
      return c.json({
        error: 'Missing connection parameter',
        message: 'Please provide a connection ID',
      }, 400);
    }

    // Check if connection exists
    if (!ctx.gateway.isConnected(connectionId)) {
      return c.json({
        error: 'Connection not found',
        message: `No active connection with ID: ${connectionId}`,
        available_connections: ctx.gateway.getConnectionIds(),
      }, 404);
    }

    try {
      // Build authorization URL
      const authUrl = await ctx.gateway.getAuthorizationUrl(connectionId, launchContext);

      // Add custom redirect_uri if provided
      if (redirectUri) {
        const url = new URL(authUrl);
        url.searchParams.set('redirect_uri', redirectUri);
        return c.redirect(url.toString());
      }

      return c.redirect(authUrl);
    } catch (error) {
      console.error('Failed to build authorization URL:', error);
      return c.json({
        error: 'Authorization failed',
        message: (error as Error).message,
      }, 500);
    }
  });

  /**
   * GET /auth/smart/callback
   *
   * Handles the OAuth2 callback from the EHR.
   *
   * Query params:
   *   code: Authorization code from EHR
   *   state: State parameter (contains connection ID)
   *   error: Optional error from EHR
   *   error_description: Optional error description
   *
   * Returns: JSON with token info or error
   */
  app.get('/smart/callback', async (c) => {
    if (!ctx.gateway) {
      return c.json({
        error: 'Gateway not initialized',
      }, 503);
    }

    // Check for errors from EHR
    const errorParam = c.req.query('error');
    if (errorParam) {
      const errorDescription = c.req.query('error_description') || 'Unknown error';
      return c.json({
        error: 'Authorization denied',
        error_code: errorParam,
        message: errorDescription,
      }, 400);
    }

    const code = c.req.query('code');
    const state = c.req.query('state');

    if (!code) {
      return c.json({
        error: 'Missing authorization code',
        message: 'The EHR did not return an authorization code',
      }, 400);
    }

    if (!state) {
      return c.json({
        error: 'Missing state parameter',
        message: 'The state parameter is required for security',
      }, 400);
    }

    // Parse state to get connection ID
    // State format: connectionId:randomNonce
    const [connectionId] = state.split(':');

    if (!connectionId || !ctx.gateway.isConnected(connectionId)) {
      return c.json({
        error: 'Invalid state',
        message: 'The state parameter contains an invalid connection ID',
      }, 400);
    }

    try {
      // Exchange code for tokens
      const tokenInfo = await ctx.gateway.completeAuthorization(connectionId, code, state);

      // Return success (in production, you might redirect to a frontend)
      return c.json({
        status: 'success',
        message: 'Authorization successful',
        connection_id: connectionId,
        patient_id: tokenInfo.patientId,
        scopes: tokenInfo.scope,
        expires_at: tokenInfo.expiresAt.toISOString(),
        // Don't return the actual tokens to the client for security
        token_stored: true,
      });
    } catch (error) {
      console.error('Token exchange failed:', error);
      return c.json({
        error: 'Token exchange failed',
        message: (error as Error).message,
      }, 500);
    }
  });

  /**
   * GET /auth/connections
   *
   * List all active EHR connections.
   *
   * Returns: Array of connection info
   */
  app.get('/connections', (c) => {
    if (!ctx.gateway) {
      return c.json({
        error: 'Gateway not initialized',
      }, 503);
    }

    const connectionIds = ctx.gateway.getConnectionIds();
    const connections = connectionIds.map(id => {
      try {
        return {
          id,
          ...ctx.gateway!.getConnectionInfo(id),
        };
      } catch {
        return { id, error: 'Failed to get connection info' };
      }
    });

    return c.json({
      total: connections.length,
      connections,
    });
  });

  /**
   * GET /auth/connections/:id
   *
   * Get info about a specific connection.
   *
   * Returns: Connection info
   */
  app.get('/connections/:id', (c) => {
    if (!ctx.gateway) {
      return c.json({
        error: 'Gateway not initialized',
      }, 503);
    }

    const connectionId = c.req.param('id');

    if (!ctx.gateway.isConnected(connectionId)) {
      return c.json({
        error: 'Connection not found',
        connection_id: connectionId,
      }, 404);
    }

    try {
      const info = ctx.gateway.getConnectionInfo(connectionId);
      return c.json({
        id: connectionId,
        ...info,
      });
    } catch (error) {
      return c.json({
        error: 'Failed to get connection info',
        message: (error as Error).message,
      }, 500);
    }
  });

  /**
   * DELETE /auth/connections/:id
   *
   * Disconnect from an EHR.
   *
   * Returns: Success message
   */
  app.delete('/connections/:id', (c) => {
    if (!ctx.gateway) {
      return c.json({
        error: 'Gateway not initialized',
      }, 503);
    }

    const connectionId = c.req.param('id');

    if (!ctx.gateway.isConnected(connectionId)) {
      return c.json({
        error: 'Connection not found',
        connection_id: connectionId,
      }, 404);
    }

    ctx.gateway.disconnect(connectionId);

    return c.json({
      status: 'success',
      message: `Disconnected from ${connectionId}`,
    });
  });

  /**
   * POST /auth/connect
   *
   * Establish a new EHR connection.
   *
   * Body:
   *   {
   *     connectionId: string,
   *     endpoint: EhrEndpoint,
   *     authConfig: SmartAuthConfig
   *   }
   *
   * Returns: Success message
   */
  app.post('/connect', async (c) => {
    if (!ctx.gateway) {
      return c.json({
        error: 'Gateway not initialized',
      }, 503);
    }

    try {
      const body = await c.req.json();

      const { connectionId, endpoint, authConfig } = body;

      if (!connectionId || !endpoint || !authConfig) {
        return c.json({
          error: 'Missing required fields',
          required: ['connectionId', 'endpoint', 'authConfig'],
        }, 400);
      }

      // Check if already connected
      if (ctx.gateway.isConnected(connectionId)) {
        return c.json({
          error: 'Connection already exists',
          connection_id: connectionId,
        }, 409);
      }

      // Establish connection
      await ctx.gateway.connect(connectionId, {
        endpoint,
        authConfig,
      });

      return c.json({
        status: 'success',
        message: `Connected to ${endpoint.system}`,
        connection_id: connectionId,
      });
    } catch (error) {
      console.error('Connection failed:', error);
      return c.json({
        error: 'Connection failed',
        message: (error as Error).message,
      }, 500);
    }
  });

  return app;
}
