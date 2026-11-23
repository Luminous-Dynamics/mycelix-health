// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { FhirBundleSchema } from '../src/types.js';

const corpusDirectory = new URL('../../../benchmarks/longitudinal-ehr/v1/', import.meta.url);
const readJson = (name: string): any =>
  JSON.parse(readFileSync(new URL(name, corpusDirectory), 'utf8'));

const patientBundle = readJson('patient-001-longitudinal.json');
const contrastBundle = readJson('patient-002-contrast.json');
const wrongPatientBundle = readJson('adversarial-wrong-patient.json');
const positiveUrnBundle = readJson('positive-urn-subject.json');
const positiveAbsoluteBundle = readJson('positive-absolute-subject.json');
const unresolvedAbsoluteBundle = readJson('adversarial-unresolved-absolute.json');
const unresolvedUrnBundle = readJson('adversarial-unresolved-urn.json');
const missingSubjectBundle = readJson('adversarial-missing-subject.json');
const multiPatientBundle = readJson('adversarial-multi-patient.json');
const manifest = readJson('manifest.json');
const taskSet = readJson('tasks.json');

const resources = (bundle: any): any[] =>
  (bundle.entry ?? []).map((entry: any) => entry.resource);

describe('Longitudinal EHR development corpus v1', () => {
  it('parses each FHIR R4 bundle with the gateway resource envelope schema', () => {
    expect(() => FhirBundleSchema.parse(patientBundle)).not.toThrow();
    expect(() => FhirBundleSchema.parse(contrastBundle)).not.toThrow();
    expect(() => FhirBundleSchema.parse(wrongPatientBundle)).not.toThrow();
  });

  it('uses unique UUID-form fullUrls in every Bundle', () => {
    const uuidUrn = /^urn:uuid:[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

    for (const bundle of [patientBundle, contrastBundle, wrongPatientBundle, positiveUrnBundle, positiveAbsoluteBundle, unresolvedAbsoluteBundle, unresolvedUrnBundle, missingSubjectBundle, multiPatientBundle]) {
      const fullUrls = (bundle.entry ?? []).map((entry: any) => entry.fullUrl);
      expect(fullUrls.length).toBeGreaterThan(0);
      expect(new Set(fullUrls).size).toBe(fullUrls.length);
      for (const fullUrl of fullUrls) expect(fullUrl).toMatch(uuidUrn);
    }
  });

  it('keeps the longitudinal subject consistent in the positive bundle', () => {
    const bundleResources = resources(patientBundle);
    const patients = bundleResources.filter(resource => resource.resourceType === 'Patient');
    expect(patients).toHaveLength(1);

    const patientRef = `Patient/${patients[0].id}`;
    const observations = bundleResources.filter(resource => resource.resourceType === 'Observation');
    expect(observations.length).toBeGreaterThan(1);
    for (const observation of observations) {
      expect(observation.subject?.reference).toBe(patientRef);
    }
  });

  it('binds the corrected observation to the prior version through Provenance', () => {
    const bundleResources = resources(patientBundle);
    const corrected = bundleResources.find(resource => resource.id === 'synthetic-observation-hba1c-001');
    const provenance = bundleResources.find(resource => resource.resourceType === 'Provenance');

    expect(corrected.status).toBe('corrected');
    expect(corrected.meta.versionId).toBe('2');
    expect(corrected.effectiveDateTime).toBe('2025-01-10T08:20:00Z');
    expect(corrected.issued).toBe('2025-01-15T14:30:00Z');
    expect(provenance.target).toContainEqual({
      reference: 'Observation/synthetic-observation-hba1c-001/_history/2',
    });
    expect(provenance.entity).toContainEqual({
      role: 'revision',
      what: { reference: 'Observation/synthetic-observation-hba1c-001/_history/1' },
    });
  });

  it('does not conflate shared LOINC coding with method comparability', () => {
    const observations = resources(patientBundle).filter(resource => resource.resourceType === 'Observation');
    const methodCodes = observations.map(observation => observation.method?.coding?.[0]?.code);
    expect(new Set(methodCodes)).toEqual(new Set(['assay-alpha', 'assay-beta']));
    expect(observations.every(observation =>
      observation.code?.coding?.some((coding: any) => coding.system === 'http://loinc.org' && coding.code === '4548-4')
    )).toBe(true);
    expect(taskSet.tasks.find((task: any) => task.task_id === 'METHOD-001')?.expected.required_output)
      .toBe('comparability-unqualified');
  });

  it('contains an explicit wrong-patient negative without mutating the positive fixture', () => {
    const bundleResources = resources(wrongPatientBundle);
    const patient = bundleResources.find(resource => resource.resourceType === 'Patient');
    const observation = bundleResources.find(resource => resource.resourceType === 'Observation');

    expect(patient.id).toBe('synthetic-patient-001');
    expect(observation.subject.reference).toBe('Patient/synthetic-patient-002');
    expect(observation.subject.reference).not.toBe(`Patient/${patient.id}`);
    expect(taskSet.tasks.find((task: any) => task.task_id === 'IDENTITY-001')?.expected.required_outcome)
      .toBe('reject-resource');
  });

  it('distinguishes exact fullUrl subject matches from unresolved absolute and URN references', () => {
    const positiveUrnObservation = resources(positiveUrnBundle).find(resource => resource.resourceType === 'Observation');
    const positiveAbsoluteObservation = resources(positiveAbsoluteBundle).find(resource => resource.resourceType === 'Observation');
    const unresolvedAbsoluteObservation = resources(unresolvedAbsoluteBundle).find(resource => resource.resourceType === 'Observation');
    const unresolvedUrnObservation = resources(unresolvedUrnBundle).find(resource => resource.resourceType === 'Observation');

    expect(positiveUrnObservation.subject.reference).toBe(positiveUrnBundle.entry[0].fullUrl);
    expect(positiveAbsoluteObservation.subject.reference).toBe(positiveAbsoluteBundle.entry[0].fullUrl);
    expect(unresolvedAbsoluteObservation.subject.reference).not.toBe(unresolvedAbsoluteBundle.entry[0].fullUrl);
    expect(unresolvedUrnObservation.subject.reference).not.toBe(unresolvedUrnBundle.entry[0].fullUrl);
  });

  it('keeps missing-subject and multi-patient cases as explicit adversarial fixtures', () => {
    const missingObservation = resources(missingSubjectBundle).find(resource => resource.resourceType === 'Observation');
    const multiPatients = resources(multiPatientBundle).filter(resource => resource.resourceType === 'Patient');

    expect(missingObservation.subject).toBeUndefined();
    expect(multiPatients).toHaveLength(2);
    expect(taskSet.tasks.find((task: any) => task.task_id === 'IDENTITY-MISSING-001')?.expected.required_outcome)
      .toBe('reject-resource');
    expect(taskSet.tasks.find((task: any) => task.task_id === 'IDENTITY-MULTI-001')?.expected.required_outcome)
      .toBe('reject-bundle');
  });

  it('keeps the corpus classified as synthetic development data', () => {
    expect(manifest.fixture_class).toBe('deterministic-synthetic');
    expect(manifest.real_patient_data).toBe(false);
    expect(manifest.external_dataset_derivation).toBe(false);
    expect(manifest.status).toBe('development-fixtures-not-yet-validator-qualified');
    expect(manifest.split).toBe('development');
    expect(new Set(taskSet.tasks.map((task: any) => task.task_id)).size).toBe(taskSet.tasks.length);
  });
});
