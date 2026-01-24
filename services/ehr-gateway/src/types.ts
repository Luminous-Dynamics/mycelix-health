/**
 * Core types for EHR Gateway
 */

import { z } from 'zod';

// FHIR Resource Base Types
export const FhirIdentifierSchema = z.object({
  system: z.string(),
  value: z.string(),
  use: z.string().optional(),
  type: z.object({
    text: z.string().optional(),
  }).optional(),
});

export type FhirIdentifier = z.infer<typeof FhirIdentifierSchema>;

export const FhirCodingSchema = z.object({
  system: z.string(),
  code: z.string(),
  display: z.string().optional(),
  version: z.string().optional(),
});

export type FhirCoding = z.infer<typeof FhirCodingSchema>;

export const FhirCodeableConceptSchema = z.object({
  coding: z.array(FhirCodingSchema).optional(),
  text: z.string().optional(),
});

export type FhirCodeableConcept = z.infer<typeof FhirCodeableConceptSchema>;

export const FhirQuantitySchema = z.object({
  value: z.number(),
  unit: z.string(),
  system: z.string().optional(),
  code: z.string().optional(),
  comparator: z.string().optional(),
});

export type FhirQuantity = z.infer<typeof FhirQuantitySchema>;

export const FhirReferenceSchema = z.object({
  reference: z.string().optional(),
  type: z.string().optional(),
  identifier: FhirIdentifierSchema.optional(),
  display: z.string().optional(),
});

export type FhirReference = z.infer<typeof FhirReferenceSchema>;

// FHIR Patient Resource
export const FhirPatientSchema = z.object({
  resourceType: z.literal('Patient'),
  id: z.string().optional(),
  identifier: z.array(FhirIdentifierSchema).optional(),
  active: z.boolean().optional(),
  name: z.array(z.object({
    use: z.string().optional(),
    text: z.string().optional(),
    family: z.string().optional(),
    given: z.array(z.string()).optional(),
    prefix: z.array(z.string()).optional(),
    suffix: z.array(z.string()).optional(),
  })).optional(),
  telecom: z.array(z.object({
    system: z.string().optional(),
    value: z.string().optional(),
    use: z.string().optional(),
    rank: z.number().optional(),
  })).optional(),
  gender: z.string().optional(),
  birthDate: z.string().optional(),
  deceasedBoolean: z.boolean().optional(),
  deceasedDateTime: z.string().optional(),
  address: z.array(z.object({
    use: z.string().optional(),
    type: z.string().optional(),
    text: z.string().optional(),
    line: z.array(z.string()).optional(),
    city: z.string().optional(),
    state: z.string().optional(),
    postalCode: z.string().optional(),
    country: z.string().optional(),
  })).optional(),
  maritalStatus: FhirCodeableConceptSchema.optional(),
  communication: z.array(z.object({
    language: FhirCodeableConceptSchema,
    preferred: z.boolean().optional(),
  })).optional(),
  meta: z.object({
    versionId: z.string().optional(),
    lastUpdated: z.string().optional(),
  }).optional(),
});

export type FhirPatient = z.infer<typeof FhirPatientSchema>;

// FHIR Observation Resource
export const FhirObservationSchema = z.object({
  resourceType: z.literal('Observation'),
  id: z.string().optional(),
  status: z.string(),
  category: z.array(FhirCodeableConceptSchema).optional(),
  code: FhirCodeableConceptSchema,
  subject: FhirReferenceSchema.optional(),
  effectiveDateTime: z.string().optional(),
  issued: z.string().optional(),
  valueQuantity: FhirQuantitySchema.optional(),
  valueCodeableConcept: FhirCodeableConceptSchema.optional(),
  valueString: z.string().optional(),
  valueBoolean: z.boolean().optional(),
  interpretation: z.array(FhirCodeableConceptSchema).optional(),
  referenceRange: z.array(z.object({
    low: FhirQuantitySchema.optional(),
    high: FhirQuantitySchema.optional(),
    type: FhirCodeableConceptSchema.optional(),
    text: z.string().optional(),
  })).optional(),
  note: z.array(z.object({
    text: z.string(),
  })).optional(),
  meta: z.object({
    versionId: z.string().optional(),
    lastUpdated: z.string().optional(),
  }).optional(),
});

export type FhirObservation = z.infer<typeof FhirObservationSchema>;

// FHIR Condition Resource
export const FhirConditionSchema = z.object({
  resourceType: z.literal('Condition'),
  id: z.string().optional(),
  clinicalStatus: FhirCodeableConceptSchema.optional(),
  verificationStatus: FhirCodeableConceptSchema.optional(),
  category: z.array(FhirCodeableConceptSchema).optional(),
  severity: FhirCodeableConceptSchema.optional(),
  code: FhirCodeableConceptSchema.optional(),
  bodySite: z.array(FhirCodeableConceptSchema).optional(),
  subject: FhirReferenceSchema,
  onsetDateTime: z.string().optional(),
  abatementDateTime: z.string().optional(),
  recordedDate: z.string().optional(),
  recorder: FhirReferenceSchema.optional(),
  asserter: FhirReferenceSchema.optional(),
  note: z.array(z.object({
    text: z.string(),
  })).optional(),
  meta: z.object({
    versionId: z.string().optional(),
    lastUpdated: z.string().optional(),
  }).optional(),
});

export type FhirCondition = z.infer<typeof FhirConditionSchema>;

// FHIR MedicationRequest Resource
export const FhirMedicationRequestSchema = z.object({
  resourceType: z.literal('MedicationRequest'),
  id: z.string().optional(),
  status: z.string(),
  intent: z.string(),
  medicationCodeableConcept: FhirCodeableConceptSchema.optional(),
  medicationReference: FhirReferenceSchema.optional(),
  subject: FhirReferenceSchema,
  requester: FhirReferenceSchema.optional(),
  reasonCode: z.array(FhirCodeableConceptSchema).optional(),
  dosageInstruction: z.array(z.object({
    sequence: z.number().optional(),
    text: z.string().optional(),
    patientInstruction: z.string().optional(),
    timing: z.object({
      code: FhirCodeableConceptSchema.optional(),
    }).optional(),
    route: FhirCodeableConceptSchema.optional(),
    doseAndRate: z.array(z.object({
      doseQuantity: FhirQuantitySchema.optional(),
    })).optional(),
  })).optional(),
  dispenseRequest: z.object({
    quantity: FhirQuantitySchema.optional(),
    numberOfRepeatsAllowed: z.number().optional(),
    validityPeriod: z.object({
      start: z.string().optional(),
      end: z.string().optional(),
    }).optional(),
  }).optional(),
  authoredOn: z.string().optional(),
  note: z.array(z.object({
    text: z.string(),
  })).optional(),
  meta: z.object({
    versionId: z.string().optional(),
    lastUpdated: z.string().optional(),
  }).optional(),
});

export type FhirMedicationRequest = z.infer<typeof FhirMedicationRequestSchema>;

// FHIR Bundle
export const FhirBundleSchema = z.object({
  resourceType: z.literal('Bundle'),
  id: z.string().optional(),
  type: z.string(),
  total: z.number().optional(),
  timestamp: z.string().optional(),
  entry: z.array(z.object({
    fullUrl: z.string().optional(),
    resource: z.any(),
    search: z.object({
      mode: z.string().optional(),
      score: z.number().optional(),
    }).optional(),
    request: z.object({
      method: z.string(),
      url: z.string(),
    }).optional(),
    response: z.object({
      status: z.string(),
      location: z.string().optional(),
    }).optional(),
  })).optional(),
  link: z.array(z.object({
    relation: z.string(),
    url: z.string(),
  })).optional(),
});

export type FhirBundle = z.infer<typeof FhirBundleSchema>;

// EHR System Types
export type EhrSystem = 'epic' | 'cerner' | 'allscripts' | 'meditech' | 'generic';

export interface EhrEndpoint {
  system: EhrSystem;
  baseUrl: string;
  authUrl: string;
  tokenUrl: string;
  clientId: string;
  scopes: string[];
}

// Sync Types
export type SyncDirection = 'pull' | 'push' | 'bidirectional';

export interface SyncResult {
  success: boolean;
  resourceType: string;
  resourceId: string;
  direction: SyncDirection;
  timestamp: Date;
  errors: string[];
}

export interface ConflictInfo {
  resourceType: string;
  resourceId: string;
  localVersion: string;
  remoteVersion: string;
  localData: unknown;
  remoteData: unknown;
  conflictType: 'update' | 'delete' | 'create';
}
