/**
 * Generic FHIR R4 Adapter
 *
 * Base adapter for FHIR R4 compliant EHR systems.
 * Provides common operations that work with any FHIR server.
 */

import type { TokenInfo } from '../auth/token-manager.js';
import {
  type FhirPatient,
  type FhirObservation,
  type FhirCondition,
  type FhirMedicationRequest,
  type FhirBundle,
  FhirPatientSchema,
  FhirObservationSchema,
  FhirConditionSchema,
  FhirMedicationRequestSchema,
  FhirBundleSchema,
} from '../types.js';

export interface FhirAdapterConfig {
  baseUrl: string;
  timeout?: number;
  maxRetries?: number;
  retryDelay?: number;
}

export interface FhirSearchParams {
  [key: string]: string | string[] | undefined;
}

export class GenericFhirAdapter {
  protected config: FhirAdapterConfig;

  constructor(config: FhirAdapterConfig) {
    this.config = {
      timeout: 30000,
      maxRetries: 3,
      retryDelay: 1000,
      ...config,
    };
  }

  /**
   * Build request headers with authorization
   */
  protected buildHeaders(tokenInfo: TokenInfo): Record<string, string> {
    return {
      'Authorization': `${tokenInfo.tokenType} ${tokenInfo.accessToken}`,
      'Accept': 'application/fhir+json',
      'Content-Type': 'application/fhir+json',
    };
  }

  /**
   * Make a FHIR request with retry logic
   */
  protected async fhirRequest<T>(
    path: string,
    tokenInfo: TokenInfo,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.config.baseUrl}${path}`;
    const headers = this.buildHeaders(tokenInfo);

    let lastError: Error | null = null;

    for (let attempt = 0; attempt < (this.config.maxRetries || 3); attempt++) {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(
          () => controller.abort(),
          this.config.timeout || 30000
        );

        const response = await fetch(url, {
          ...options,
          headers: { ...headers, ...options.headers },
          signal: controller.signal,
        });

        clearTimeout(timeoutId);

        if (!response.ok) {
          const errorBody = await response.text();
          throw new Error(`FHIR request failed: ${response.status} - ${errorBody}`);
        }

        return await response.json() as T;
      } catch (error) {
        lastError = error as Error;

        if (attempt < (this.config.maxRetries || 3) - 1) {
          await this.delay(this.config.retryDelay || 1000);
        }
      }
    }

    throw lastError || new Error('FHIR request failed');
  }

  /**
   * Delay helper for retries
   */
  protected delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  /**
   * Build search query string
   */
  protected buildSearchParams(params: FhirSearchParams): string {
    const searchParams = new URLSearchParams();

    for (const [key, value] of Object.entries(params)) {
      if (value === undefined) continue;

      if (Array.isArray(value)) {
        for (const v of value) {
          searchParams.append(key, v);
        }
      } else {
        searchParams.set(key, value);
      }
    }

    return searchParams.toString();
  }

  // Patient Operations

  async getPatient(patientId: string, tokenInfo: TokenInfo): Promise<FhirPatient> {
    const data = await this.fhirRequest<unknown>(`/Patient/${patientId}`, tokenInfo);
    return FhirPatientSchema.parse(data);
  }

  async searchPatients(
    params: FhirSearchParams,
    tokenInfo: TokenInfo
  ): Promise<FhirBundle> {
    const query = this.buildSearchParams(params);
    const data = await this.fhirRequest<unknown>(`/Patient?${query}`, tokenInfo);
    return FhirBundleSchema.parse(data);
  }

  // Observation Operations

  async getObservation(observationId: string, tokenInfo: TokenInfo): Promise<FhirObservation> {
    const data = await this.fhirRequest<unknown>(`/Observation/${observationId}`, tokenInfo);
    return FhirObservationSchema.parse(data);
  }

  async searchObservations(
    params: FhirSearchParams,
    tokenInfo: TokenInfo
  ): Promise<FhirBundle> {
    const query = this.buildSearchParams(params);
    const data = await this.fhirRequest<unknown>(`/Observation?${query}`, tokenInfo);
    return FhirBundleSchema.parse(data);
  }

  async getPatientObservations(
    patientId: string,
    tokenInfo: TokenInfo,
    category?: string,
    code?: string
  ): Promise<FhirBundle> {
    const params: FhirSearchParams = {
      patient: patientId,
      _sort: '-date',
    };

    if (category) params.category = category;
    if (code) params.code = code;

    return this.searchObservations(params, tokenInfo);
  }

  async getPatientVitals(patientId: string, tokenInfo: TokenInfo): Promise<FhirBundle> {
    return this.getPatientObservations(patientId, tokenInfo, 'vital-signs');
  }

  async getPatientLabResults(patientId: string, tokenInfo: TokenInfo): Promise<FhirBundle> {
    return this.getPatientObservations(patientId, tokenInfo, 'laboratory');
  }

  // Condition Operations

  async getCondition(conditionId: string, tokenInfo: TokenInfo): Promise<FhirCondition> {
    const data = await this.fhirRequest<unknown>(`/Condition/${conditionId}`, tokenInfo);
    return FhirConditionSchema.parse(data);
  }

  async searchConditions(
    params: FhirSearchParams,
    tokenInfo: TokenInfo
  ): Promise<FhirBundle> {
    const query = this.buildSearchParams(params);
    const data = await this.fhirRequest<unknown>(`/Condition?${query}`, tokenInfo);
    return FhirBundleSchema.parse(data);
  }

  async getPatientConditions(
    patientId: string,
    tokenInfo: TokenInfo,
    clinicalStatus?: string
  ): Promise<FhirBundle> {
    const params: FhirSearchParams = {
      patient: patientId,
    };

    if (clinicalStatus) params['clinical-status'] = clinicalStatus;

    return this.searchConditions(params, tokenInfo);
  }

  async getActiveConditions(patientId: string, tokenInfo: TokenInfo): Promise<FhirBundle> {
    return this.getPatientConditions(patientId, tokenInfo, 'active');
  }

  // Medication Operations

  async getMedicationRequest(
    medicationRequestId: string,
    tokenInfo: TokenInfo
  ): Promise<FhirMedicationRequest> {
    const data = await this.fhirRequest<unknown>(
      `/MedicationRequest/${medicationRequestId}`,
      tokenInfo
    );
    return FhirMedicationRequestSchema.parse(data);
  }

  async searchMedicationRequests(
    params: FhirSearchParams,
    tokenInfo: TokenInfo
  ): Promise<FhirBundle> {
    const query = this.buildSearchParams(params);
    const data = await this.fhirRequest<unknown>(`/MedicationRequest?${query}`, tokenInfo);
    return FhirBundleSchema.parse(data);
  }

  async getPatientMedications(
    patientId: string,
    tokenInfo: TokenInfo,
    status?: string
  ): Promise<FhirBundle> {
    const params: FhirSearchParams = {
      patient: patientId,
    };

    if (status) params.status = status;

    return this.searchMedicationRequests(params, tokenInfo);
  }

  async getActiveMedications(patientId: string, tokenInfo: TokenInfo): Promise<FhirBundle> {
    return this.getPatientMedications(patientId, tokenInfo, 'active');
  }

  // Bundle Operations

  async executeBundle(bundle: FhirBundle, tokenInfo: TokenInfo): Promise<FhirBundle> {
    const data = await this.fhirRequest<unknown>('/', tokenInfo, {
      method: 'POST',
      body: JSON.stringify(bundle),
    });
    return FhirBundleSchema.parse(data);
  }

  // Patient Everything Operation

  async getPatientEverything(
    patientId: string,
    tokenInfo: TokenInfo,
    start?: string,
    end?: string
  ): Promise<FhirBundle> {
    const params: FhirSearchParams = {};
    if (start) params.start = start;
    if (end) params.end = end;

    const query = this.buildSearchParams(params);
    const path = `/Patient/${patientId}/$everything${query ? `?${query}` : ''}`;

    const data = await this.fhirRequest<unknown>(path, tokenInfo);
    return FhirBundleSchema.parse(data);
  }

  // Capability Statement

  async getCapabilityStatement(tokenInfo?: TokenInfo): Promise<unknown> {
    if (tokenInfo) {
      return this.fhirRequest<unknown>('/metadata', tokenInfo);
    }

    const response = await fetch(`${this.config.baseUrl}/metadata`, {
      headers: { Accept: 'application/fhir+json' },
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch capability statement: ${response.status}`);
    }

    return response.json();
  }

  // Write Operations

  async createResource<T extends { resourceType: string }>(
    resource: T,
    tokenInfo: TokenInfo
  ): Promise<T> {
    return this.fhirRequest<T>(`/${resource.resourceType}`, tokenInfo, {
      method: 'POST',
      body: JSON.stringify(resource),
    });
  }

  async updateResource<T extends { resourceType: string; id?: string }>(
    resource: T,
    tokenInfo: TokenInfo
  ): Promise<T> {
    if (!resource.id) {
      throw new Error('Resource must have an id for update');
    }

    return this.fhirRequest<T>(`/${resource.resourceType}/${resource.id}`, tokenInfo, {
      method: 'PUT',
      body: JSON.stringify(resource),
    });
  }

  async deleteResource(
    resourceType: string,
    resourceId: string,
    tokenInfo: TokenInfo
  ): Promise<void> {
    await this.fhirRequest<unknown>(`/${resourceType}/${resourceId}`, tokenInfo, {
      method: 'DELETE',
    });
  }
}
