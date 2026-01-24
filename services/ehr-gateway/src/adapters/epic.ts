/**
 * Epic EHR Adapter
 *
 * Extends the generic FHIR adapter with Epic-specific functionality
 * and API optimizations.
 */

import { GenericFhirAdapter, type FhirAdapterConfig, type FhirSearchParams } from './generic-fhir.js';
import type { TokenInfo } from '../auth/token-manager.js';
import type { FhirBundle, FhirPatient } from '../types.js';

export interface EpicAdapterConfig extends FhirAdapterConfig {
  epicClientId: string;
  useSandbox?: boolean;
}

/**
 * Epic-specific extensions and optimizations
 */
export class EpicAdapter extends GenericFhirAdapter {
  private epicConfig: EpicAdapterConfig;

  // Epic's standard sandbox and production base URLs
  static readonly SANDBOX_URL = 'https://fhir.epic.com/interconnect-fhir-oauth/api/FHIR/R4';
  static readonly PRODUCTION_URL_TEMPLATE = 'https://{epic-fqdn}/interconnect-fhir-oauth/api/FHIR/R4';

  constructor(config: EpicAdapterConfig) {
    super(config);
    this.epicConfig = config;
  }

  /**
   * Build Epic-specific headers
   */
  protected override buildHeaders(tokenInfo: TokenInfo): Record<string, string> {
    return {
      ...super.buildHeaders(tokenInfo),
      'Epic-Client-ID': this.epicConfig.epicClientId,
    };
  }

  /**
   * Search patients with Epic's extended parameters
   */
  async searchPatientsExtended(
    params: FhirSearchParams & {
      'family:exact'?: string;
      'given:exact'?: string;
      birthdate?: string;
      mrn?: string;
    },
    tokenInfo: TokenInfo
  ): Promise<FhirBundle> {
    // Epic supports searching by MRN through identifier
    if (params.mrn) {
      params.identifier = `urn:oid:1.2.840.114350.1.13.0.1.7.5.737384.14|${params.mrn}`;
      delete params.mrn;
    }

    return this.searchPatients(params, tokenInfo);
  }

  /**
   * Get patient by Epic's internal identifier (FHIR ID)
   */
  async getPatientByFhirId(fhirId: string, tokenInfo: TokenInfo): Promise<FhirPatient> {
    return this.getPatient(fhirId, tokenInfo);
  }

  /**
   * Get patient by MRN (Medical Record Number)
   */
  async getPatientByMRN(mrn: string, tokenInfo: TokenInfo): Promise<FhirPatient | null> {
    const bundle = await this.searchPatientsExtended({ mrn }, tokenInfo);

    if (bundle.entry && bundle.entry.length > 0) {
      return bundle.entry[0].resource as FhirPatient;
    }

    return null;
  }

  /**
   * Get patient's care team
   */
  async getCareTeam(patientId: string, tokenInfo: TokenInfo): Promise<FhirBundle> {
    const query = this.buildSearchParams({ patient: patientId });
    return this.fhirRequest<FhirBundle>(`/CareTeam?${query}`, tokenInfo);
  }

  /**
   * Get patient's allergies (AllergyIntolerance)
   */
  async getAllergies(patientId: string, tokenInfo: TokenInfo): Promise<FhirBundle> {
    const query = this.buildSearchParams({
      patient: patientId,
      'clinical-status': 'active',
    });
    return this.fhirRequest<FhirBundle>(`/AllergyIntolerance?${query}`, tokenInfo);
  }

  /**
   * Get patient's immunizations
   */
  async getImmunizations(patientId: string, tokenInfo: TokenInfo): Promise<FhirBundle> {
    const query = this.buildSearchParams({
      patient: patientId,
      _sort: '-date',
    });
    return this.fhirRequest<FhirBundle>(`/Immunization?${query}`, tokenInfo);
  }

  /**
   * Get patient's appointments
   */
  async getAppointments(
    patientId: string,
    tokenInfo: TokenInfo,
    startDate?: string,
    endDate?: string
  ): Promise<FhirBundle> {
    const params: FhirSearchParams = {
      patient: patientId,
      _sort: 'date',
    };

    if (startDate) params.date = `ge${startDate}`;
    if (endDate) {
      params.date = params.date
        ? [params.date as string, `le${endDate}`]
        : `le${endDate}`;
    }

    const query = this.buildSearchParams(params);
    return this.fhirRequest<FhirBundle>(`/Appointment?${query}`, tokenInfo);
  }

  /**
   * Get patient's encounters
   */
  async getEncounters(
    patientId: string,
    tokenInfo: TokenInfo,
    status?: string
  ): Promise<FhirBundle> {
    const params: FhirSearchParams = {
      patient: patientId,
      _sort: '-date',
    };

    if (status) params.status = status;

    const query = this.buildSearchParams(params);
    return this.fhirRequest<FhirBundle>(`/Encounter?${query}`, tokenInfo);
  }

  /**
   * Get patient's diagnostic reports
   */
  async getDiagnosticReports(
    patientId: string,
    tokenInfo: TokenInfo,
    category?: string
  ): Promise<FhirBundle> {
    const params: FhirSearchParams = {
      patient: patientId,
      _sort: '-date',
    };

    if (category) params.category = category;

    const query = this.buildSearchParams(params);
    return this.fhirRequest<FhirBundle>(`/DiagnosticReport?${query}`, tokenInfo);
  }

  /**
   * Get patient's documents (DocumentReference)
   */
  async getDocuments(
    patientId: string,
    tokenInfo: TokenInfo,
    type?: string
  ): Promise<FhirBundle> {
    const params: FhirSearchParams = {
      patient: patientId,
      _sort: '-date',
    };

    if (type) params.type = type;

    const query = this.buildSearchParams(params);
    return this.fhirRequest<FhirBundle>(`/DocumentReference?${query}`, tokenInfo);
  }

  /**
   * Get binary document content
   */
  async getBinaryContent(binaryId: string, tokenInfo: TokenInfo): Promise<Blob> {
    const url = `${this.config.baseUrl}/Binary/${binaryId}`;
    const headers = this.buildHeaders(tokenInfo);

    const response = await fetch(url, {
      headers: {
        ...headers,
        Accept: '*/*',
      },
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch binary: ${response.status}`);
    }

    return response.blob();
  }

  /**
   * Get patient's procedures
   */
  async getProcedures(patientId: string, tokenInfo: TokenInfo): Promise<FhirBundle> {
    const query = this.buildSearchParams({
      patient: patientId,
      _sort: '-date',
    });
    return this.fhirRequest<FhirBundle>(`/Procedure?${query}`, tokenInfo);
  }

  /**
   * Create an Epic-compatible patient summary bundle
   */
  async getPatientSummary(patientId: string, tokenInfo: TokenInfo): Promise<{
    patient: FhirPatient | null;
    conditions: FhirBundle;
    medications: FhirBundle;
    allergies: FhirBundle;
    vitals: FhirBundle;
  }> {
    const [patient, conditions, medications, allergies, vitals] = await Promise.all([
      this.getPatient(patientId, tokenInfo).catch(() => null),
      this.getActiveConditions(patientId, tokenInfo),
      this.getActiveMedications(patientId, tokenInfo),
      this.getAllergies(patientId, tokenInfo),
      this.getPatientVitals(patientId, tokenInfo),
    ]);

    return {
      patient,
      conditions,
      medications,
      allergies,
      vitals,
    };
  }
}
