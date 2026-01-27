/**
 * Configuration System for EHR Gateway Server
 *
 * Loads configuration from environment variables with sensible defaults.
 * Uses Zod for type-safe validation.
 */

import { z } from 'zod';

/**
 * Server configuration schema
 */
export const ServerConfigSchema = z.object({
  /** HTTP server port (0 means random available port) */
  port: z.number().int().min(0).max(65535).default(3000),

  /** Host to bind to */
  host: z.string().default('0.0.0.0'),

  /** Holochain WebSocket URL */
  holochainUrl: z.string().url().default('ws://localhost:8888'),

  /** Holochain app ID */
  holochainAppId: z.string().default('mycelix-health'),

  /** Default timeout for operations in milliseconds */
  defaultTimeout: z.number().int().min(1000).default(30000),

  /** Maximum retries for failed operations */
  maxRetries: z.number().int().min(0).max(10).default(3),

  /** CORS allowed origins (comma-separated string or array) */
  corsOrigins: z.union([
    z.string().transform(s => s.split(',')),
    z.array(z.string()),
  ]).default(['*']),

  /** Log level */
  logLevel: z.enum(['debug', 'info', 'warn', 'error']).default('info'),

  /** Enable request logging */
  enableRequestLogging: z.boolean().default(true),

  /** OAuth callback base URL (for SMART on FHIR redirects) */
  oauthCallbackBaseUrl: z.string().url().optional(),
});

export type ServerConfig = z.infer<typeof ServerConfigSchema>;

/**
 * EHR Connection configuration schema
 */
export const EhrConnectionConfigSchema = z.object({
  /** Connection identifier */
  id: z.string(),

  /** EHR system type */
  system: z.enum(['epic', 'cerner', 'allscripts', 'meditech', 'generic']),

  /** FHIR base URL */
  baseUrl: z.string().url(),

  /** OAuth authorization URL */
  authUrl: z.string().url(),

  /** OAuth token URL */
  tokenUrl: z.string().url(),

  /** OAuth client ID */
  clientId: z.string(),

  /** OAuth client secret (optional for public clients) */
  clientSecret: z.string().optional(),

  /** OAuth scopes */
  scopes: z.array(z.string()).default([
    'launch',
    'patient/Patient.read',
    'patient/Observation.read',
    'patient/Condition.read',
    'patient/MedicationRequest.read',
    'patient/AllergyIntolerance.read',
    'patient/Immunization.read',
    'patient/Procedure.read',
  ]),

  /** Default source system identifier for ingestion */
  sourceSystemId: z.string(),
});

export type EhrConnectionConfig = z.infer<typeof EhrConnectionConfigSchema>;

/**
 * Full application configuration
 */
export const AppConfigSchema = z.object({
  server: ServerConfigSchema,
  ehrConnections: z.array(EhrConnectionConfigSchema).default([]),
});

export type AppConfig = z.infer<typeof AppConfigSchema>;

/**
 * Load server configuration from environment variables
 */
export function loadServerConfig(): ServerConfig {
  return ServerConfigSchema.parse({
    port: parseInt(process.env.EHR_GATEWAY_PORT || process.env.PORT || '3000', 10),
    host: process.env.EHR_GATEWAY_HOST || '0.0.0.0',
    holochainUrl: process.env.HOLOCHAIN_URL || 'ws://localhost:8888',
    holochainAppId: process.env.HOLOCHAIN_APP_ID || 'mycelix-health',
    defaultTimeout: parseInt(process.env.DEFAULT_TIMEOUT || '30000', 10),
    maxRetries: parseInt(process.env.MAX_RETRIES || '3', 10),
    corsOrigins: process.env.CORS_ORIGINS || '*',
    logLevel: process.env.LOG_LEVEL || 'info',
    enableRequestLogging: process.env.ENABLE_REQUEST_LOGGING !== 'false',
    oauthCallbackBaseUrl: process.env.OAUTH_CALLBACK_BASE_URL,
  });
}

/**
 * Load EHR connections from environment variables
 *
 * Environment variables should be prefixed with EHR_CONNECTION_{ID}_
 * Example:
 *   EHR_CONNECTION_EPIC_SYSTEM=epic
 *   EHR_CONNECTION_EPIC_BASE_URL=https://fhir.epic.com/interconnect-fhir-oauth/api/FHIR/R4
 *   etc.
 */
export function loadEhrConnections(): EhrConnectionConfig[] {
  const connections: EhrConnectionConfig[] = [];

  // Find all unique connection IDs from environment variables
  const connectionIds = new Set<string>();
  for (const key of Object.keys(process.env)) {
    const match = key.match(/^EHR_CONNECTION_([^_]+)_/);
    if (match) {
      connectionIds.add(match[1]);
    }
  }

  for (const id of connectionIds) {
    const prefix = `EHR_CONNECTION_${id}_`;
    const system = process.env[`${prefix}SYSTEM`];
    const baseUrl = process.env[`${prefix}BASE_URL`];
    const authUrl = process.env[`${prefix}AUTH_URL`];
    const tokenUrl = process.env[`${prefix}TOKEN_URL`];
    const clientId = process.env[`${prefix}CLIENT_ID`];
    const clientSecret = process.env[`${prefix}CLIENT_SECRET`];
    const scopes = process.env[`${prefix}SCOPES`];
    const sourceSystemId = process.env[`${prefix}SOURCE_SYSTEM_ID`];

    if (system && baseUrl && authUrl && tokenUrl && clientId) {
      try {
        const config = EhrConnectionConfigSchema.parse({
          id: id.toLowerCase(),
          system,
          baseUrl,
          authUrl,
          tokenUrl,
          clientId,
          clientSecret: clientSecret || undefined,
          scopes: scopes ? scopes.split(',') : undefined,
          sourceSystemId: sourceSystemId || `${system}-${id.toLowerCase()}`,
        });
        connections.push(config);
      } catch (error) {
        console.warn(`Failed to load EHR connection ${id}:`, error);
      }
    }
  }

  return connections;
}

/**
 * Load full application configuration
 */
export function loadConfig(): AppConfig {
  return {
    server: loadServerConfig(),
    ehrConnections: loadEhrConnections(),
  };
}

/**
 * Get configuration with defaults for testing
 */
export function getTestConfig(overrides?: Partial<ServerConfig>): ServerConfig {
  return ServerConfigSchema.parse({
    port: 0, // Random available port
    host: '127.0.0.1',
    holochainUrl: 'ws://localhost:8888',
    holochainAppId: 'test-app',
    defaultTimeout: 5000,
    maxRetries: 1,
    corsOrigins: ['*'],
    logLevel: 'error',
    enableRequestLogging: false,
    ...overrides,
  });
}
