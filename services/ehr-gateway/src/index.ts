/**
 * EHR Gateway Service for Mycelix-Health
 *
 * Provides FHIR R4 compatible integration with external EHR systems
 * including Epic, Cerner, and other SMART on FHIR compliant systems.
 */

export { EhrGateway, type EhrGatewayConfig } from './gateway.js';
export { SmartOnFhirAuth, type SmartAuthConfig } from './auth/smart-on-fhir.js';
export { TokenManager, type TokenInfo } from './auth/token-manager.js';
export { EpicAdapter } from './adapters/epic.js';
export { CernerAdapter } from './adapters/cerner.js';
export { GenericFhirAdapter, type FhirAdapterConfig } from './adapters/generic-fhir.js';
export { PullService, type PullConfig } from './sync/pull-service.js';
export { PushService, type PushConfig } from './sync/push-service.js';
export { ConflictResolver, type ConflictResolution } from './sync/conflict-resolver.js';

export * from './types.js';
