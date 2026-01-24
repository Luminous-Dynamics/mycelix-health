/**
 * Provider Directory Zome Client
 *
 * Client for healthcare provider registration, search, and NPI verification
 * in Mycelix-Health.
 */

import type { AppClient, ActionHash, AgentPubKey, Timestamp } from '@holochain/client';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

// Provider Status
export type ProviderStatus = 'Active' | 'Inactive' | 'Suspended' | 'Pending';

// Affiliation Type
export type AffiliationType = 'Primary' | 'Secondary' | 'Consulting' | 'Admitting' | 'Referring';

// Person Name
export interface PersonName {
  family: string;
  given: string[];
  prefix?: string[];
  suffix?: string[];
  display?: string;
}

// Practice Location
export interface PracticeLocation {
  name: string;
  address_line1: string;
  address_line2?: string;
  city: string;
  state: string;
  postal_code: string;
  country: string;
  phone?: string;
  fax?: string;
  email?: string;
  hours?: string;
  accepting_new_patients: boolean;
}

// Provider Profile
export interface ProviderProfile {
  hash?: ActionHash;
  agent_key: AgentPubKey;
  npi: string;
  name: PersonName;
  credentials: string[];
  specialties: string[];
  taxonomy_codes: string[];
  practice_locations: PracticeLocation[];
  telehealth_available: boolean;
  languages: string[];
  accepting_new_patients: boolean;
  status: ProviderStatus;
  verified: boolean;
  verification_date?: Timestamp;
  created_at: Timestamp;
  updated_at: Timestamp;
}

// NPI Verification Result
export interface NpiVerificationResult {
  npi: string;
  valid: boolean;
  entity_type?: 'Individual' | 'Organization';
  name?: PersonName;
  specialties?: string[];
  addresses?: PracticeLocation[];
  verified_at: Timestamp;
  error?: string;
}

// Provider Affiliation
export interface ProviderAffiliation {
  hash?: ActionHash;
  provider_hash: ActionHash;
  organization_name: string;
  organization_npi?: string;
  affiliation_type: AffiliationType;
  department?: string;
  start_date: Timestamp;
  end_date?: Timestamp;
  is_active: boolean;
}

// Search Criteria
export interface ProviderSearchCriteria {
  name?: string;
  specialty?: string;
  location?: {
    city?: string;
    state?: string;
    postal_code?: string;
    radius_miles?: number;
  };
  telehealth_only?: boolean;
  accepting_new_patients?: boolean;
  languages?: string[];
  limit?: number;
  offset?: number;
}

// Input types
export interface RegisterProviderInput {
  npi: string;
  name: PersonName;
  credentials: string[];
  specialties: string[];
  taxonomy_codes?: string[];
  practice_locations: PracticeLocation[];
  telehealth_available?: boolean;
  languages?: string[];
  accepting_new_patients?: boolean;
}

export interface UpdateProviderInput {
  provider_hash: ActionHash;
  name?: PersonName;
  credentials?: string[];
  specialties?: string[];
  practice_locations?: PracticeLocation[];
  telehealth_available?: boolean;
  languages?: string[];
  accepting_new_patients?: boolean;
  status?: ProviderStatus;
}

export interface AddAffiliationInput {
  provider_hash: ActionHash;
  organization_name: string;
  organization_npi?: string;
  affiliation_type: AffiliationType;
  department?: string;
  start_date?: Timestamp;
}

/**
 * Provider Directory Zome Client
 */
export class ProviderDirectoryClient {
  private readonly roleName: string;
  private readonly zomeName = 'provider_directory';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Register a new provider
   */
  async registerProvider(input: RegisterProviderInput): Promise<ActionHash> {
    return this.call<ActionHash>('register_provider', {
      ...input,
      taxonomy_codes: input.taxonomy_codes ?? [],
      telehealth_available: input.telehealth_available ?? false,
      languages: input.languages ?? ['en'],
      accepting_new_patients: input.accepting_new_patients ?? true,
    });
  }

  /**
   * Get provider profile by hash
   */
  async getProvider(providerHash: ActionHash): Promise<ProviderProfile | null> {
    return this.call<ProviderProfile | null>('get_provider', providerHash);
  }

  /**
   * Get provider profile by NPI
   */
  async getProviderByNpi(npi: string): Promise<ProviderProfile | null> {
    return this.call<ProviderProfile | null>('get_provider_by_npi', npi);
  }

  /**
   * Get the current agent's provider profile
   */
  async getMyProviderProfile(): Promise<ProviderProfile | null> {
    return this.call<ProviderProfile | null>('get_my_provider_profile', null);
  }

  /**
   * Update provider profile
   */
  async updateProvider(input: UpdateProviderInput): Promise<ActionHash> {
    return this.call<ActionHash>('update_provider', input);
  }

  /**
   * Search for providers
   */
  async searchProviders(criteria: ProviderSearchCriteria): Promise<ProviderProfile[]> {
    return this.call<ProviderProfile[]>('search_providers', {
      ...criteria,
      limit: criteria.limit ?? 50,
      offset: criteria.offset ?? 0,
    });
  }

  /**
   * Get telehealth-enabled providers
   */
  async getTelehealthProviders(specialty?: string): Promise<ProviderProfile[]> {
    return this.call<ProviderProfile[]>('get_telehealth_providers', { specialty });
  }

  /**
   * Get providers by specialty
   */
  async getProvidersBySpecialty(specialty: string, limit = 50): Promise<ProviderProfile[]> {
    return this.searchProviders({ specialty, limit });
  }

  /**
   * Verify an NPI number
   */
  async verifyNpi(npi: string): Promise<NpiVerificationResult> {
    return this.call<NpiVerificationResult>('verify_npi', npi);
  }

  /**
   * Add a provider affiliation
   */
  async addAffiliation(input: AddAffiliationInput): Promise<ActionHash> {
    return this.call<ActionHash>('add_provider_affiliation', input);
  }

  /**
   * Get provider affiliations
   */
  async getProviderAffiliations(providerHash: ActionHash): Promise<ProviderAffiliation[]> {
    return this.call<ProviderAffiliation[]>('get_provider_affiliations', providerHash);
  }

  /**
   * End a provider affiliation
   */
  async endAffiliation(affiliationHash: ActionHash): Promise<void> {
    await this.call<void>('end_affiliation', affiliationHash);
  }

  /**
   * Set provider status
   */
  async setProviderStatus(providerHash: ActionHash, status: ProviderStatus): Promise<void> {
    await this.call<void>('set_provider_status', {
      provider_hash: providerHash,
      status,
    });
  }

  /**
   * Get providers accepting new patients in a location
   */
  async getAcceptingProviders(city: string, state: string, specialty?: string): Promise<ProviderProfile[]> {
    const criteria: ProviderSearchCriteria = {
      location: { city, state },
      accepting_new_patients: true,
    };
    if (specialty) {
      criteria.specialty = specialty;
    }
    return this.searchProviders(criteria);
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
        `Provider Directory zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
