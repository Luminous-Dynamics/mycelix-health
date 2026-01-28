# Patient Zome

> Core patient record management for the Mycelix Health network.

## Overview

The Patient zome manages patient demographic information, contact details, medical history references, and agent-to-patient mappings. It serves as the foundational identity layer for all health data in the network.

## Architecture

```
patient/
├── integrity/           # Entry type definitions & validation
│   └── src/
│       └── lib.rs       # Patient, ContactInfo entry types
└── coordinator/         # Business logic & extern functions
    └── src/
        └── lib.rs       # CRUD operations, search, linking
```

## Entry Types

| Entry Type | Description | Links |
|------------|-------------|-------|
| `Patient` | Core patient demographics and identifiers | `PatientToAgent`, `PatientByMrn` |
| `ContactInfo` | Address, phone, email information | (embedded in Patient) |
| `EmergencyContact` | Emergency contact details | (embedded in Patient) |

## Extern Functions

### Create Operations

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `create_patient` | `CreatePatientInput` | `Record` | Create a new patient record |

### Read Operations

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `get_patient` | `ActionHash` | `Option<Patient>` | Get patient by action hash |
| `get_my_patient` | `()` | `Option<Record>` | Get current agent's patient record |
| `get_patient_by_mrn` | `String` | `Option<Record>` | Find patient by MRN |
| `search_patients_by_name` | `SearchByNameInput` | `Vec<Record>` | Search patients by name |

### Update Operations

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `update_patient` | `UpdatePatientInput` | `Record` | Update patient record |
| `add_allergy` | `AddAllergyInput` | `()` | Add allergy to patient |
| `add_medication` | `AddMedicationInput` | `()` | Add medication to patient |

### Delete Operations

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `delete_patient` | `ActionHash` | `ActionHash` | Soft delete patient record |

## Input Types

```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePatientInput {
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,  // ISO 8601 format
    pub mrn: Option<String>,
    pub contact: ContactInfo,
    pub allergies: Option<Vec<String>>,
    pub medications: Option<Vec<String>>,
    pub emergency_contacts: Option<Vec<EmergencyContact>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdatePatientInput {
    pub original_hash: ActionHash,
    pub updates: PatientUpdates,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchByNameInput {
    pub name: String,
    pub limit: Option<usize>,
}
```

## Validation Rules

### Entry Validation

- **first_name**: Required, non-empty string
- **last_name**: Required, non-empty string
- **date_of_birth**: Required, valid ISO 8601 date format (YYYY-MM-DD)
- **mrn**: If provided, must be 4-50 characters, alphanumeric with hyphens
- **matl_trust_score**: Must be between 0.0 and 1.0

### Link Validation

- **PatientToAgent**: Links patient to owning agent for quick lookup
- **PatientByMrn**: Indexed by MRN for unique identifier searches

## Usage Examples

### SDK (TypeScript)

```typescript
import { PatientClient } from '@mycelix/health-sdk';

// Create client
const client = new PatientClient(appClient, 'health');

// Create patient
const record = await client.createPatient({
  first_name: 'John',
  last_name: 'Doe',
  date_of_birth: '1980-01-15',
  mrn: 'MRN-12345',
  contact: {
    address_line1: '123 Main St',
    city: 'Anytown',
    state_province: 'CA',
    postal_code: '12345',
    country: 'USA',
    phone_primary: '+1-555-123-4567',
    email: 'john.doe@example.com',
  },
  allergies: ['Penicillin'],
  medications: ['Metformin 500mg'],
});

// Get patient
const patient = await client.getPatient(record.hash);

// Search by name
const results = await client.searchPatients({ name: 'John', limit: 10 });

// Search by MRN
const byMrn = await client.searchPatients({ mrn: 'MRN-12345' });
```

### Direct Zome Calls

```typescript
const result = await appClient.callZome({
  cap_secret: null,
  role_name: 'health',
  zome_name: 'patient',
  fn_name: 'create_patient',
  payload: {
    first_name: 'John',
    last_name: 'Doe',
    date_of_birth: '1980-01-15',
    contact: { /* ... */ },
  },
});
```

## Dependencies

### Internal Dependencies

- `mycelix-health-shared`: Common types, validation (`validate_mrn`, `validate_confidence_score`)

### External Dependencies

- `hdi ^0.7.0`: Holochain Deterministic Integrity
- `hdk ^0.6.0`: Holochain Development Kit
- `uuid ^1.6.0`: Patient ID generation

## Error Handling

This zome uses typed errors from `mycelix-health-shared`:

```rust
use mycelix_health_shared::validation::{validate_mrn, validate_confidence_score, ValidationResult};

fn validate_patient(patient: &Patient) -> ValidationResult {
    let mut result = ValidationResult::new();

    if let Some(ref mrn) = patient.mrn {
        result.merge(validate_mrn(mrn));
    }

    result.merge(validate_confidence_score(patient.matl_trust_score, "matl_trust_score"));
    // Additional validations...

    result
}
```

Common error types:
- `ValidationError::Required`: first_name, last_name, date_of_birth missing
- `ValidationError::InvalidFormat`: MRN format invalid, date format invalid
- `ValidationError::OutOfRange`: trust score outside 0.0-1.0

## Testing

### Unit Tests

```bash
# Run unit tests
cargo test -p patient_integrity --target x86_64-unknown-linux-gnu
cargo test -p patient_coordinator --target x86_64-unknown-linux-gnu
```

### SDK Tests

```bash
cd sdk
npm test -- tests/patient-client.test.ts
```

## Privacy & Consent

Patient records contain sensitive PHI and require careful handling:

- **Data Sensitivity**: High (PHI/PII)
- **Consent Required**: Yes - via `consent` zome
- **Encryption**: At-rest via Holochain, field-level encryption for sensitive data

Access to patient data is controlled by the `consent` zome. All reads should check:
1. Is the requester the patient themselves?
2. Does the requester have valid consent for the requested data categories?

## FHIR Mapping

| FHIR Resource | Entry Type | Notes |
|---------------|------------|-------|
| `Patient` | `Patient` | Core demographics |
| `RelatedPerson` | `EmergencyContact` | Emergency contacts |
| `Address` | `ContactInfo.address_*` | Address fields |
| `ContactPoint` | `ContactInfo.phone_*`, `email` | Communication details |

## Changelog

### v0.1.0 (2026-01)

- Initial implementation
- Patient CRUD operations
- MRN-based lookup
- Name search
- Allergy and medication management
- Input validation with typed errors

## Related Zomes

- `consent`: Access control and consent management
- `mental_health`: Mental health screening and mood tracking
- `fhir_bridge`: FHIR resource import/export
- `clinical_decision_support`: CDS recommendations based on patient data
