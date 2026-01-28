# FHIR Bridge Zome

> FHIR R4 interoperability layer for ingesting and exporting health data to/from external EHR systems.

## Overview

The FHIR Bridge zome provides bidirectional data exchange between Mycelix-Health and external EHR systems using the FHIR R4 standard. It handles bundle ingestion, resource mapping, deduplication, and FHIR-compliant exports.

## Architecture

```
fhir_bridge/
├── integrity/           # Entry type definitions & validation
│   └── src/
│       └── lib.rs       # IngestReport, FHIR mapping entries
└── coordinator/         # Business logic & extern functions
    └── src/
        └── lib.rs       # Bundle ingestion, export, validation
```

## Entry Types

| Entry Type | Description | Links |
|------------|-------------|-------|
| `IngestReport` | Record of bundle ingestion results | `IngestReports` anchor |
| `FhirPatientMapping` | Maps external patient ID to internal hash | `SourcePatient` anchor |
| `FhirObservationMapping` | Maps external observation ID | `PatientToObservations` |
| `FhirConditionMapping` | Maps external condition ID | `PatientToConditions` |
| `FhirMedicationMapping` | Maps external medication ID | `PatientToMedications` |

## Extern Functions

### Ingestion

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `ingest_bundle` | `IngestBundleInput` | `IngestReport` | Ingest a complete FHIR Bundle |

### Export

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `export_patient_fhir` | `ExportPatientInput` | `ExportResult` | Export patient data as FHIR Bundle |

### Validation

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `validate_fhir_resource` | `JsonValue` | `bool` | Validate a FHIR resource |

## Supported FHIR Resources

| FHIR Resource | Direction | Internal Mapping |
|---------------|-----------|------------------|
| `Patient` | In/Out | `Patient` entry |
| `Observation` | In/Out | Observation mapping |
| `Condition` | In/Out | Condition mapping |
| `MedicationRequest` | In/Out | Medication mapping |
| `AllergyIntolerance` | In/Out | Allergy entries |
| `Immunization` | In/Out | Immunization entries |
| `Procedure` | In/Out | Procedure entries |
| `DiagnosticReport` | In/Out | Diagnostic report mapping |
| `CarePlan` | In/Out | Care plan mapping |

## Input/Output Types

### IngestBundleInput

```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct IngestBundleInput {
    /// The FHIR Bundle as JSON
    pub bundle: JsonValue,
    /// Source system identifier (e.g., "epic-mychart", "cerner-millennium")
    pub source_system: String,
}
```

### IngestReport

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IngestReport {
    pub report_id: String,
    pub source_system: String,
    pub ingested_at: Timestamp,
    pub total_processed: u32,
    pub patients_created: u32,
    pub patients_updated: u32,
    pub conditions_created: u32,
    pub conditions_skipped: u32,
    pub medications_created: u32,
    pub medications_skipped: u32,
    pub allergies_created: u32,
    pub allergies_skipped: u32,
    pub immunizations_created: u32,
    pub immunizations_skipped: u32,
    pub observations_created: u32,
    pub observations_skipped: u32,
    pub procedures_created: u32,
    pub procedures_skipped: u32,
    pub diagnostic_reports_created: u32,
    pub diagnostic_reports_skipped: u32,
    pub care_plans_created: u32,
    pub care_plans_skipped: u32,
    pub unknown_types: Vec<String>,
    pub parse_errors: Vec<String>,
}
```

### ExportPatientInput

```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct ExportPatientInput {
    pub patient_hash: ActionHash,
    /// Which resource types to include
    pub resource_types: Option<Vec<String>>,
    /// Export format (default: FHIR R4 Bundle)
    pub format: Option<ExportFormat>,
}

pub enum ExportFormat {
    FhirR4Bundle,
    USCoreBundle,
    CCDADocument,
    IPSBundle,
}
```

## Usage Examples

### Ingesting a FHIR Bundle

```typescript
import { AppClient } from '@holochain/client';

// Ingest a bundle from Epic
const report = await appClient.callZome({
  cap_secret: null,
  role_name: 'health',
  zome_name: 'fhir_bridge',
  fn_name: 'ingest_bundle',
  payload: {
    bundle: {
      resourceType: 'Bundle',
      type: 'collection',
      entry: [
        {
          fullUrl: 'Patient/12345',
          resource: {
            resourceType: 'Patient',
            id: '12345',
            name: [{ family: 'Doe', given: ['John'] }],
            birthDate: '1980-01-15',
          },
        },
        {
          fullUrl: 'Condition/67890',
          resource: {
            resourceType: 'Condition',
            id: '67890',
            subject: { reference: 'Patient/12345' },
            code: {
              coding: [{ system: 'http://snomed.info/sct', code: '73211009' }],
            },
          },
        },
      ],
    },
    source_system: 'epic-mychart',
  },
});

console.log(`Processed ${report.total_processed} resources`);
console.log(`Created ${report.patients_created} patients`);
console.log(`Created ${report.conditions_created} conditions`);
```

### Exporting Patient Data

```typescript
// Export as FHIR R4 Bundle
const exportResult = await appClient.callZome({
  cap_secret: null,
  role_name: 'health',
  zome_name: 'fhir_bridge',
  fn_name: 'export_patient_fhir',
  payload: {
    patient_hash: patientHash,
    resource_types: ['Patient', 'Condition', 'MedicationRequest'],
    format: 'FhirR4Bundle',
  },
});

// exportResult.bundle contains the FHIR Bundle JSON
```

### EHR Gateway Integration

The FHIR bridge is typically used through the EHR Gateway service:

```typescript
import { EhrGateway } from '@mycelix/ehr-gateway';

const gateway = new EhrGateway({ holochainClient });

// Pull data from Epic
const results = await gateway.pullPatientData(
  'epic-connection',
  'patient-123',
  'token-key'
);

// The gateway internally calls fhir_bridge.ingest_bundle
```

## Deduplication

The FHIR bridge prevents duplicate imports using anchors:

```
source_system + fhir_resource_id → Unique anchor
```

When a resource with the same source + ID is imported again:
- Existing mapping is found via anchor lookup
- Record is updated if changed, skipped if identical
- `*_skipped` counters track deduplicated resources

## Data Flow

```
┌──────────────┐     ┌───────────────┐     ┌──────────────┐
│  External    │     │  FHIR Bridge  │     │  Internal    │
│  EHR (FHIR)  │────▶│  Zome         │────▶│  Zomes       │
└──────────────┘     │               │     │  (patient,   │
                     │ ingest_bundle │     │   consent,   │
                     │               │     │   etc.)      │
                     └───────────────┘     └──────────────┘
                            │
                            ▼
                     ┌───────────────┐
                     │ IngestReport  │
                     │ (audit trail) │
                     └───────────────┘
```

## Dependencies

### Internal Dependencies

- `mycelix-health-shared`: Authorization, logging, anchors
- `fhir_mapping_integrity`: FHIR mapping entry types
- `patient_integrity`: Patient entry types (cross-zome)

### External Dependencies

- `hdi ^0.7.0`: Holochain Deterministic Integrity
- `hdk ^0.6.0`: Holochain Development Kit
- `serde_json`: JSON parsing

## Error Handling

Parse errors are captured in the `IngestReport`:

```rust
// Invalid resource types are logged
report.unknown_types.push(resource_type);

// Parse failures are logged with details
report.parse_errors.push(format!("Failed to parse {}: {}", id, error));
```

The ingestion continues even with errors, processing all valid resources.

## Testing

### Unit Tests

```bash
cargo test -p fhir_bridge_integrity --target x86_64-unknown-linux-gnu
cargo test -p fhir_bridge_coordinator --target x86_64-unknown-linux-gnu
```

### Integration Tests

```bash
cd tests/sweettest
cargo test fhir_bridge --target x86_64-unknown-linux-gnu
```

## FHIR Compliance

- **FHIR R4**: Full support for R4 resources
- **US Core**: Export in US Core profile format
- **IPS**: International Patient Summary export
- **C-CDA**: Clinical Document Architecture export

## Privacy & Consent

All FHIR operations respect consent:

- **Import**: Requires write permission for patient data
- **Export**: Requires read permission + consent for sharing
- **Audit**: All access logged via `log_data_access`

## Changelog

### v0.1.0 (2026-01)

- Initial implementation
- Bundle ingestion with 9 resource types
- Deduplication via source anchors
- FHIR R4 export
- IngestReport audit trail
- DiagnosticReport and CarePlan support

## Related Zomes

- `patient`: Patient records created from FHIR Patient resources
- `consent`: Controls access to imported/exported data
- `clinical_decision_support`: Uses FHIR data for CDS
- `fhir_mapping`: Stores external ID mappings
