// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
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

// Gateway request authority and fail-closed route-guard contracts (production provider pending)
export {
  GATEWAY_ACTIONS_V1,
  GATEWAY_CONTEXT_CLOCK_SKEW_MS,
  GATEWAY_CONTEXT_MAX_TTL_MS,
  authorizeGatewayRequestV1,
  type GatewayActionV1,
  type GatewayAuthorizationFailureV1,
  type GatewayAuthorizationResultV1,
  type GatewayAuthorityGrantV1,
  type GatewayPrincipalV1,
  type GatewayReceiptVerifierV1,
  type GatewayRequestContextV1,
} from './auth/gateway-request-context.js';

export {
  authorizeGatewayRouteV1,
  gatewayAuthorizationHttpStatusV1,
  requireGatewayRouteAuthorizationV1,
  type GatewayRouteAuthorizationProviderV1,
  type GatewayRouteAuthorizationResultV1,
  type GatewayRouteIntentV1,
} from './auth/gateway-route-guard.js';

// Durable operation-ledger contract. A production store and external adapter
// are required before this coordinator can be used for protected mutations.
export {
  deriveGatewayOperationKeyV1,
  GatewayOperationCoordinatorV1,
  type GatewayOperationIntentV1,
  type GatewayOperationKeyInputV1,
  type GatewayOperationReceiptV1,
  type GatewayOperationStateV1,
  type GatewayOperationStoreV1,
  type GatewayOperationReauthorizerV1,
  type GatewayExternalOperationAdapterV1,
  type GatewayOperationResultV1,
} from './auth/gateway-operation-ledger.js';

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

// Caching
export {
  Cache,
  TokenCache,
  MetadataCache,
  ResourceCache,
  EhrCacheManager,
  type CacheConfig,
  type CacheEntry,
  type CacheStats,
  type EhrCacheConfig,
  type CachedToken,
  type FhirResource,
  type FhirCapabilityStatement,
} from './cache.js';

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
