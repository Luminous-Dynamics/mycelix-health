/**
 * Disaster Response Zome Client
 *
 * Client for emergency health coordination during disasters and public health emergencies.
 * Part of Phase 6 - Global Scale.
 */

import type { AppClient, ActionHash, Timestamp } from '@holochain/client';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

// Enums
export enum DisasterType {
  NaturalDisaster = 'NaturalDisaster',
  Pandemic = 'Pandemic',
  MassCasualty = 'MassCasualty',
  ChemicalBiological = 'ChemicalBiological',
  Nuclear = 'Nuclear',
  InfrastructureFailure = 'InfrastructureFailure',
  Other = 'Other',
}

export enum DisasterStatus {
  Declared = 'Declared',
  Active = 'Active',
  Stabilizing = 'Stabilizing',
  Recovery = 'Recovery',
  Resolved = 'Resolved',
}

export enum ResourceType {
  Medical = 'Medical',
  Personnel = 'Personnel',
  Equipment = 'Equipment',
  Supplies = 'Supplies',
  Transportation = 'Transportation',
  Shelter = 'Shelter',
  Communication = 'Communication',
}

export enum ResourceStatus {
  Available = 'Available',
  Deployed = 'Deployed',
  InTransit = 'InTransit',
  Exhausted = 'Exhausted',
}

export enum TriageCategory {
  Immediate = 'Immediate',
  Delayed = 'Delayed',
  Minor = 'Minor',
  Expectant = 'Expectant',
  Deceased = 'Deceased',
}

// Types
export interface DisasterDeclaration {
  disaster_hash: ActionHash;
  name: string;
  disaster_type: DisasterType;
  status: DisasterStatus;
  affected_region: string;
  center_coordinates?: { latitude: number; longitude: number };
  radius_km?: number;
  declared_at: Timestamp;
  declaring_authority: string;
  estimated_affected_population?: number;
}

export interface EmergencyResource {
  resource_hash: ActionHash;
  disaster_hash?: ActionHash;
  resource_type: ResourceType;
  description: string;
  quantity: number;
  unit: string;
  location: string;
  status: ResourceStatus;
  owner_org: string;
  available_from?: Timestamp;
  available_until?: Timestamp;
}

export interface ResourceRequest {
  request_hash: ActionHash;
  disaster_hash: ActionHash;
  requesting_org: string;
  requesting_location: string;
  resource_type: ResourceType;
  description: string;
  quantity_needed: number;
  unit: string;
  priority: number;
  justification: string;
  fulfilled: boolean;
  fulfilled_by?: ActionHash;
}

export interface MassTriageRecord {
  triage_hash: ActionHash;
  disaster_hash: ActionHash;
  patient_identifier: string;
  category: TriageCategory;
  chief_complaint: string;
  vital_signs?: string;
  interventions: string[];
  disposition: string;
  location: string;
  triaged_at: Timestamp;
  triaged_by: ActionHash;
}

export interface EvacuationOrder {
  order_hash: ActionHash;
  disaster_hash: ActionHash;
  zone: string;
  order_type: 'mandatory' | 'voluntary' | 'shelter_in_place';
  issued_at: Timestamp;
  effective_from: Timestamp;
  expires_at?: Timestamp;
  instructions: string;
  evacuation_routes: string[];
  shelter_locations: string[];
}

// Input types
export interface DeclareDisasterInput {
  name: string;
  disaster_type: DisasterType;
  affected_region: string;
  center_coordinates?: { latitude: number; longitude: number };
  radius_km?: number;
  declaring_authority: string;
  estimated_affected_population?: number;
}

export interface RegisterResourceInput {
  disaster_hash?: ActionHash;
  resource_type: ResourceType;
  description: string;
  quantity: number;
  unit: string;
  location: string;
  owner_org: string;
  available_from?: Timestamp;
  available_until?: Timestamp;
}

export interface RequestResourcesInput {
  disaster_hash: ActionHash;
  requesting_org: string;
  requesting_location: string;
  resource_type: ResourceType;
  resource_description: string;
  quantity_needed: number;
  unit: string;
  priority: number;
  justification: string;
}

export interface RecordTriageInput {
  disaster_hash: ActionHash;
  patient_identifier: string;
  category: TriageCategory;
  chief_complaint: string;
  vital_signs?: string;
  interventions: string[];
  disposition: string;
  location: string;
}

/**
 * Disaster Response Zome Client
 */
export class DisasterResponseClient {
  private readonly roleName: string;
  private readonly zomeName = 'disaster_response';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Declare a disaster
   */
  async declareDisaster(input: DeclareDisasterInput): Promise<ActionHash> {
    return this.call<ActionHash>('declare_disaster', input);
  }

  /**
   * Get a disaster by hash
   */
  async getDisaster(disasterHash: ActionHash): Promise<DisasterDeclaration | null> {
    return this.call<DisasterDeclaration | null>('get_disaster', disasterHash);
  }

  /**
   * Get active disasters
   */
  async getActiveDisasters(): Promise<DisasterDeclaration[]> {
    return this.call<DisasterDeclaration[]>('get_active_disasters', null);
  }

  /**
   * Update disaster status
   */
  async updateDisasterStatus(disasterHash: ActionHash, status: DisasterStatus): Promise<ActionHash> {
    return this.call<ActionHash>('update_disaster_status', {
      disaster_hash: disasterHash,
      status,
    });
  }

  /**
   * Register an emergency resource
   */
  async registerResource(input: RegisterResourceInput): Promise<ActionHash> {
    return this.call<ActionHash>('register_resource', input);
  }

  /**
   * Get available resources
   */
  async getAvailableResources(resourceType?: ResourceType): Promise<EmergencyResource[]> {
    return this.call<EmergencyResource[]>('get_available_resources', resourceType ?? null);
  }

  /**
   * Request resources
   */
  async requestResources(input: RequestResourcesInput): Promise<ActionHash> {
    return this.call<ActionHash>('request_resources', input);
  }

  /**
   * Get pending requests for a disaster
   */
  async getPendingRequests(disasterHash: ActionHash): Promise<ResourceRequest[]> {
    return this.call<ResourceRequest[]>('get_pending_requests', disasterHash);
  }

  /**
   * Fulfill a resource request
   */
  async fulfillRequest(requestHash: ActionHash, resourceHash: ActionHash): Promise<void> {
    return this.call<void>('fulfill_request', {
      request_hash: requestHash,
      resource_hash: resourceHash,
    });
  }

  /**
   * Record triage
   */
  async recordTriage(input: RecordTriageInput): Promise<ActionHash> {
    return this.call<ActionHash>('record_triage', input);
  }

  /**
   * Get triage records for a disaster
   */
  async getTriageRecords(disasterHash: ActionHash): Promise<MassTriageRecord[]> {
    return this.call<MassTriageRecord[]>('get_triage_records', disasterHash);
  }

  /**
   * Get triage summary
   */
  async getTriageSummary(
    disasterHash: ActionHash
  ): Promise<Record<TriageCategory, number>> {
    return this.call<Record<TriageCategory, number>>('get_triage_summary', disasterHash);
  }

  /**
   * Issue evacuation order
   */
  async issueEvacuationOrder(
    disasterHash: ActionHash,
    zone: string,
    orderType: 'mandatory' | 'voluntary' | 'shelter_in_place',
    instructions: string,
    evacuationRoutes: string[],
    shelterLocations: string[]
  ): Promise<ActionHash> {
    return this.call<ActionHash>('issue_evacuation_order', {
      disaster_hash: disasterHash,
      zone,
      order_type: orderType,
      instructions,
      evacuation_routes: evacuationRoutes,
      shelter_locations: shelterLocations,
    });
  }

  /**
   * Get active evacuation orders
   */
  async getActiveEvacuationOrders(disasterHash: ActionHash): Promise<EvacuationOrder[]> {
    return this.call<EvacuationOrder[]>('get_active_evacuation_orders', disasterHash);
  }

  private async call<T>(fnName: string, payload: unknown): Promise<T> {
    try {
      const result = await this.client.callZome({
        role_name: this.roleName,
        zome_name: this.zomeName,
        fn_name: fnName,
        payload,
      });
      return result as T;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      throw new HealthSdkError(
        HealthSdkErrorCode.ZOME_CALL_FAILED,
        `Disaster Response zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
