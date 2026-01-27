/**
 * EHR Gateway Service for Mycelix-Health
 *
 * Provides FHIR R4 compatible integration with external EHR systems
 * including Epic, Cerner, and other SMART on FHIR compliant systems.
 *
 * This package can be used as:
 * 1. A library - import { EhrGateway } from '@mycelix/ehr-gateway'
 * 2. An HTTP server - run `npm start` to start the server
 */

// Core Gateway
export { EhrGateway, type EhrGatewayConfig } from './gateway.js';

// Authentication
export { SmartOnFhirAuth, type SmartAuthConfig } from './auth/smart-on-fhir.js';
export { TokenManager, type TokenInfo } from './auth/token-manager.js';

// EHR Adapters
export { EpicAdapter } from './adapters/epic.js';
export { CernerAdapter } from './adapters/cerner.js';
export { GenericFhirAdapter, type FhirAdapterConfig } from './adapters/generic-fhir.js';

// Sync Services
export { PullService, type PullConfig, type PullOptions, type PullResult } from './sync/pull-service.js';
export { PushService, type PushConfig } from './sync/push-service.js';
export { ConflictResolver, type ConflictResolution } from './sync/conflict-resolver.js';

// Configuration
export {
  loadServerConfig,
  loadConfig,
  loadEhrConnections,
  getTestConfig,
  type ServerConfig,
  type EhrConnectionConfig,
  type AppConfig,
} from './config.js';

// Server (for programmatic use)
export { createApp, type AppContext } from './server.js';

// All types
export * from './types.js';
