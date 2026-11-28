// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import {
  FhirBundleSchema,
  FhirIngestBundleSchema,
} from '../src/types.js';

describe('FHIR R4 Bundle schema boundaries', () => {
  it('accepts an empty searchset as a valid upstream EHR response', () => {
    const parsed = FhirBundleSchema.parse({
      resourceType: 'Bundle',
      type: 'searchset',
      total: 0,
      entry: [],
    });

    expect(parsed.type).toBe('searchset');
    expect(parsed.entry).toEqual([]);
  });

  it('rejects an empty Bundle only at the ingestion boundary', () => {
    const emptySearchset = {
      resourceType: 'Bundle',
      type: 'searchset',
      total: 0,
      entry: [],
    };

    expect(FhirBundleSchema.safeParse(emptySearchset).success).toBe(true);
    expect(FhirIngestBundleSchema.safeParse(emptySearchset).success).toBe(false);
  });

  it('preserves unknown Bundle, entry, signature and resource extension fields', () => {
    const signature = {
      type: [{ system: 'urn:iso-astm:E1762-95:2013', code: '1.2.840.10065.1.12.1.1' }],
      when: '2026-11-28T10:00:00Z',
      who: { reference: 'Practitioner/signer' },
      data: 'c2lnbmF0dXJlLWZpeHR1cmU=',
    };
    const extension = [{
      url: 'https://example.org/fhir/StructureDefinition/test-extension',
      valueString: 'preserve-me',
    }];
    const input = {
      resourceType: 'Bundle',
      type: 'collection',
      meta: { profile: ['https://example.org/fhir/StructureDefinition/test-profile'] },
      signature,
      entry: [{
        fullUrl: 'urn:uuid:patient-001',
        provenanceTag: 'source-preserved',
        resource: {
          resourceType: 'Patient',
          id: 'patient-001',
          extension,
          _birthDate: { extension: [{ url: 'data-absent-reason', valueCode: 'masked' }] },
        },
      }],
    };

    const parsed = FhirIngestBundleSchema.parse(input);
    expect(parsed.meta).toEqual(input.meta);
    expect(parsed.signature).toEqual(signature);
    expect(parsed.entry?.[0].provenanceTag).toBe('source-preserved');
    expect(parsed.entry?.[0].resource.extension).toEqual(extension);
    expect(parsed.entry?.[0].resource._birthDate).toEqual({
      extension: [{ url: 'data-absent-reason', valueCode: 'masked' }],
    });
  });

  it('rejects duplicate fullUrl values and preserves relative Bundle link URIs', () => {
    const invalid = {
      resourceType: 'Bundle',
      type: 'collection',
      entry: [
        { fullUrl: 'urn:uuid:duplicate', resource: { resourceType: 'Patient', id: 'p1' } },
        { fullUrl: 'urn:uuid:duplicate', resource: { resourceType: 'Observation', id: 'o1' } },
      ],
    };
    expect(FhirBundleSchema.safeParse(invalid).success).toBe(false);

    const valid = {
      resourceType: 'Bundle',
      type: 'collection',
      link: [{ relation: 'self', url: 'Bundle?patient=123' }],
      entry: [{ resource: { resourceType: 'Patient', id: 'p1' } }],
    };
    expect(FhirBundleSchema.parse(valid).link?.[0].url).toBe('Bundle?patient=123');
  });

  it('enforces transaction and document Bundle invariants', () => {
    const missingRequest = {
      resourceType: 'Bundle',
      type: 'transaction',
      entry: [{ resource: { resourceType: 'Patient', id: 'p1' } }],
    };
    expect(FhirBundleSchema.safeParse(missingRequest).success).toBe(false);

    const invalidDocument = {
      resourceType: 'Bundle',
      type: 'document',
      entry: [{ resource: { resourceType: 'Patient', id: 'p1' } }],
    };
    expect(FhirBundleSchema.safeParse(invalidDocument).success).toBe(false);

    const validDocument = {
      resourceType: 'Bundle',
      type: 'document',
      entry: [{ resource: { resourceType: 'Composition', id: 'composition-1' } }],
    };
    expect(FhirBundleSchema.safeParse(validDocument).success).toBe(true);
  });
});
