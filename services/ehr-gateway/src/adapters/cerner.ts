/**
 * Cerner (Oracle Health) EHR Adapter
 *
 * Extends the generic FHIR adapter with Cerner-specific functionality
 * and API optimizations.
 */

import { GenericFhirAdapter, type FhirAdapterConfig, type FhirSearchParams } from './generic-fhir.js';
import type { TokenInfo } from '../auth/token-manager.js';
import type { FhirBundle, FhirPatient } from '../types.js';

export interface CernerAdapterConfig extends FhirAdapterConfig {
  tenantId?: string;
}

/**
 * Cerner-specific extensions and optimizations
 */
export class CernerAdapter extends GenericFhirAdapter {
  private cernerConfig: CernerAdapterConfig;

  // Cerner's standard sandbox URL
  static readonly SANDBOX_URL = 'https://fhir-ehr-code.cerner.com/r4/ec2458f2-1e24-41c8-b71b-0e701af7583d';

  constructor(config: CernerAdapterConfig) {
    super(config);
    this.cernerConfig = config;
  }

  /**
   * Build Cerner-specific headers
   */
  protected override buildHeaders(tokenInfo: TokenInfo): Record<string, string> {
    const headers = super.buildHeaders(tokenInfo);

    // Cerner may require additional headers
    if (this.cernerConfig.tenantId) {
      headers['X-Cerner-Tenant-Id'] = this.cernerConfig.tenantId;
    }

    return headers;
  }

  /**
   * Get patient by Cerner Person ID
   */
  async getPatientByPersonId(personId: string, tokenInfo: TokenInfo): Promise<FhirPatient | null> {
    const bundle = await this.searchPatients(
      { identifier: `urn:oid:2.16.840.1.113883.6.1000|${personId}` },
      tokenInfo
    );

    if (bundle.entry && bundle.entry.length > 0) {
      return bundle.entry[0].resource as FhirPatient;
    }

    return null;
  }

  /**
   * Get patient by MRN
   */
  async getPatientByMRN(mrn: string, tokenInfo: TokenInfo): Promise<FhirPatient | null> {
    const bundle = await this.searchPatients(
      { identifier: `urn:oid:2.16.840.1.113883.3.13.6|${mrn}` },
      tokenInfo
    );

    if (bundle.entry && bundle.entry.length > 0) {
      return bundle.entry[0].resource as FhirPatient;
    }

    return null;
  }

  /**
   * Get patient's allergies
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

    // Cerner uses date parameter with prefixes
    if (startDate && endDate) {
      params.date = [`ge${startDate}`, `le${endDate}`];
    } else if (startDate) {
      params.date = `ge${startDate}`;
    } else if (endDate) {
      params.date = `le${endDate}`;
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
   * Get patient's care plans
   */
  async getCarePlans(patientId: string, tokenInfo: TokenInfo): Promise<FhirBundle> {
    const query = this.buildSearchParams({
      patient: patientId,
      status: 'active',
    });
    return this.fhirRequest<FhirBundle>(`/CarePlan?${query}`, tokenInfo);
  }

  /**
   * Get patient's goals
   */
  async getGoals(patientId: string, tokenInfo: TokenInfo): Promise<FhirBundle> {
    const query = this.buildSearchParams({
      patient: patientId,
      'lifecycle-status': 'active',
    });
    return this.fhirRequest<FhirBundle>(`/Goal?${query}`, tokenInfo);
  }

  /**
   * Get patient's documents
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
   * Get patient's clinical notes
   */
  async getClinicalNotes(patientId: string, tokenInfo: TokenInfo): Promise<FhirBundle> {
    // Clinical notes are typically DocumentReference with specific category
    return this.getDocuments(
      patientId,
      tokenInfo,
      'http://loinc.org|34117-2' // Clinical note LOINC code
    );
  }

  /**
   * Get related persons for a patient
   */
  async getRelatedPersons(patientId: string, tokenInfo: TokenInfo): Promise<FhirBundle> {
    const query = this.buildSearchParams({
      patient: patientId,
    });
    return this.fhirRequest<FhirBundle>(`/RelatedPerson?${query}`, tokenInfo);
  }

  /**
   * Get patient's coverage (insurance)
   */
  async getCoverage(patientId: string, tokenInfo: TokenInfo): Promise<FhirBundle> {
    const query = this.buildSearchParams({
      patient: patientId,
      status: 'active',
    });
    return this.fhirRequest<FhirBundle>(`/Coverage?${query}`, tokenInfo);
  }

  /**
   * Create a Cerner-compatible patient summary bundle
   */
  async getPatientSummary(patientId: string, tokenInfo: TokenInfo): Promise<{
    patient: FhirPatient | null;
    conditions: FhirBundle;
    medications: FhirBundle;
    allergies: FhirBundle;
    vitals: FhirBundle;
    carePlans: FhirBundle;
  }> {
    const [patient, conditions, medications, allergies, vitals, carePlans] = await Promise.all([
      this.getPatient(patientId, tokenInfo).catch(() => null),
      this.getActiveConditions(patientId, tokenInfo),
      this.getActiveMedications(patientId, tokenInfo),
      this.getAllergies(patientId, tokenInfo),
      this.getPatientVitals(patientId, tokenInfo),
      this.getCarePlans(patientId, tokenInfo),
    ]);

    return {
      patient,
      conditions,
      medications,
      allergies,
      vitals,
      carePlans,
    };
  }

  /**
   * Cerner-specific: Get write-back capable resources
   * Cerner has specific requirements for write operations
   */
  async getWriteBackCapabilities(tokenInfo: TokenInfo): Promise<string[]> {
    const capability = await this.getCapabilityStatement(tokenInfo);
    const writeCapable: string[] = [];

    // Parse CapabilityStatement for write-capable resources
    const rest = (capability as { rest?: Array<{ resource?: Array<{ type: string; interaction?: Array<{ code: string }> }> }> }).rest?.[0];
    if (rest?.resource) {
      for (const resource of rest.resource) {
        const interactions = resource.interaction?.map(i => i.code) || [];
        if (interactions.includes('create') || interactions.includes('update')) {
          writeCapable.push(resource.type);
        }
      }
    }

    return writeCapable;
  }
}
