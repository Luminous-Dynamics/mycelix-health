# Mycelix Health Longitudinal EHR Benchmark — v1 development corpus

This directory contains the first deterministic, synthetic FHIR R4 fixture slice for the longitudinal EHR and evidence-assurance benchmark.

## Scope

The fixtures exercise four narrow engineering properties:

1. **Temporal reconstruction:** keep `Observation.effectiveDateTime` separate from `Observation.issued` and encounter/medication timestamps.
2. **Correction provenance:** preserve a corrected result and its referenced prior version through a FHIR `Provenance` revision entity.
3. **Measurement comparability:** do not infer a directly comparable trend merely because two observations share a LOINC code when the assay-method identities differ.
4. **Patient integrity:** reject an observation whose explicit `Patient/<id>` subject conflicts with the Patient established by the incoming Bundle.

## Data provenance and privacy

- Every fixture is hand-authored and deterministic.
- All patient identifiers are synthetic labels.
- No real patient records, real names, contact details, or clinical source documents are included.
- These fixtures are not derived from MIMIC, Synthea, or another external patient dataset.
- The numeric values are chosen only to exercise data semantics. They are not clinical reference values or treatment targets.

## Files

- `manifest.json`: benchmark identity, FHIR release, scope and fixture inventory.
- `tasks.json`: task prompts and exact expected structural/temporal properties.
- `patient-001-longitudinal.json`: encounter, condition, medication, corrected laboratory result, provenance and method transition.
- `patient-002-contrast.json`: separate patient identity and comparison observation.
- `adversarial-wrong-patient.json`: negative bundle with an explicit cross-patient subject mismatch.

## Current qualification state

**Development fixtures only.** The files have not yet been run through the pinned HL7 FHIR R4 validator or a clinician review. A JSON parser accepting these files is not FHIR conformance, and the expected outcomes are structural/temporal invariants rather than clinical gold labels.

Before this corpus is used for public benchmark claims:

1. Freeze the exact validator version and R4 core package.
2. Validate positive fixtures and document any deliberately permitted profile warnings.
3. Verify the wrong-patient negative using the actual FHIR bridge and adversarial ingestion test.
4. Review correction and method-comparability semantics with a qualified laboratory professional.
5. Add more subjects and patient-level development/validation/test splits; never split individual encounters from one subject across train and test.
6. Publish content hashes and exact Git commit in each corpus release.

## Scoring rule

A patient-identity violation, lost correction lineage, or silent effective/issued-time substitution is a hard failure. Do not trade those failures against latency or aggregate task success.

A model must abstain or return an explicit unqualified state when measurement method comparability is not established. This benchmark does not score diagnostic correctness.
