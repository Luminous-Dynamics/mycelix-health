# Consent Zome

> HIPAA-compliant consent management and access control for patient health data.

## Overview

The Consent zome manages patient data access authorizations, implementing granular consent directives that control who can access what data under which conditions. It provides the authorization layer for all health data operations in the Mycelix network.

## Architecture

```
consent/
├── integrity/           # Entry type definitions & validation
│   └── src/
│       └── lib.rs       # Consent, DataAccessRequest, DataAccessLog entries
└── coordinator/         # Business logic & extern functions
    └── src/
        └── lib.rs       # Authorization checks, consent CRUD, audit logging
```

## Entry Types

| Entry Type | Description | Links |
|------------|-------------|-------|
| `Consent` | Consent directive with scope, grantee, permissions | `PatientToConsents`, `ActiveConsents`, `RevokedConsents` |
| `DataAccessRequest` | Request for data access (pending approval) | `PatientToAccessRequests` |
| `DataAccessLog` | Audit log of data access events | `PatientToAccessLogs` |

## Extern Functions

### Consent Management

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `create_consent` | `Consent` | `Record` | Create a new consent directive |
| `get_patient_consents` | `ActionHash` | `Vec<Record>` | Get all consents for a patient |
| `get_active_consents` | `ActionHash` | `Vec<Record>` | Get only active consents |
| `revoke_consent` | `RevokeConsentInput` | `Record` | Revoke an existing consent |

### Authorization

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `check_authorization` | `AuthorizationCheckInput` | `AuthorizationResult` | Check if access is authorized |

### Access Requests & Logging

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `create_access_request` | `DataAccessRequest` | `Record` | Create a data access request |
| `log_data_access` | `DataAccessLog` | `Record` | Log a data access event |

## Core Types

### Consent

```rust
pub struct Consent {
    pub patient_hash: ActionHash,
    pub grantee: ConsentGrantee,
    pub scope: ConsentScope,
    pub permissions: Vec<DataPermission>,
    pub status: ConsentStatus,
    pub valid_from: Timestamp,
    pub valid_until: Option<Timestamp>,
    pub created_at: Timestamp,
    pub revoked_at: Option<Timestamp>,
    pub revocation_reason: Option<String>,
}

pub enum ConsentGrantee {
    Agent(AgentPubKey),        // Specific agent
    Role(String),              // Role-based (e.g., "primary_care")
    Organization(String),      // Organization-based
    EmergencyAccess,           // Emergency override
}

pub enum ConsentStatus {
    Active,
    Revoked,
    Expired,
    Pending,
}
```

### ConsentScope

```rust
pub struct ConsentScope {
    pub data_categories: Vec<DataCategory>,
    pub exclusions: Vec<DataCategory>,
    pub purpose: String,
    pub restrictions: Vec<String>,
}

pub enum DataCategory {
    All,
    Demographics,
    Allergies,
    Medications,
    Diagnoses,
    Procedures,
    LabResults,
    ImagingStudies,
    VitalSigns,
    Immunizations,
    MentalHealth,
    SubstanceAbuse,
    SexualHealth,
    GeneticData,
    FinancialData,
}
```

### DataPermission

```rust
pub enum DataPermission {
    Read,
    Write,
    Share,
    Export,
    Delete,
}
```

## Usage Examples

### SDK (TypeScript)

```typescript
import { ConsentClient } from '@mycelix/health-sdk';

// Create client
const client = new ConsentClient(appClient, 'health');

// Grant consent to a provider
const consent = await client.grantConsent(patientHash, {
  grantee: providerAgentKey,
  scope: 'Read',
  data_categories: ['Demographics', 'Medications', 'Allergies'],
  purpose: 'Primary care treatment',
  valid_until: Date.now() + 365 * 24 * 60 * 60 * 1000, // 1 year
});

// Check authorization
const authResult = await client.checkAuthorization({
  patient_hash: patientHash,
  requester: providerAgentKey,
  action: 'read',
  data_categories: ['Medications'],
});

if (authResult.authorized) {
  // Access granted
}

// Revoke consent
await client.revokeConsent(consentHash);

// List all consents
const consents = await client.listPatientConsents(patientHash);
```

### Direct Zome Calls

```typescript
// Create consent
const consent = await appClient.callZome({
  cap_secret: null,
  role_name: 'health',
  zome_name: 'consent',
  fn_name: 'create_consent',
  payload: {
    patient_hash: patientHash,
    grantee: { Agent: providerKey },
    scope: {
      data_categories: ['Demographics', 'Medications'],
      exclusions: ['MentalHealth'],
      purpose: 'Treatment coordination',
      restrictions: [],
    },
    permissions: ['Read'],
    status: 'Active',
    valid_from: Date.now() * 1000,
    valid_until: null,
  },
});

// Check authorization
const result = await appClient.callZome({
  cap_secret: null,
  role_name: 'health',
  zome_name: 'consent',
  fn_name: 'check_authorization',
  payload: {
    patient_hash: patientHash,
    requestor: requesterKey,
    data_category: 'Medications',
    permission: 'Read',
    is_emergency: false,
  },
});
```

## Authorization Flow

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│  Requester  │────▶│ check_auth   │────▶│  Consent    │
│  (Agent)    │     │ (consent     │     │  Records    │
└─────────────┘     │  zome)       │     └─────────────┘
                    └──────────────┘
                           │
                           ▼
                    ┌──────────────┐
                    │ Authorization│
                    │ Result       │
                    │ - authorized │
                    │ - consent_id │
                    │ - permissions│
                    └──────────────┘
```

## Dependencies

### Internal Dependencies

- `mycelix-health-shared`: Common types (DataCategory, DataPermission)

### External Dependencies

- `hdi ^0.7.0`: Holochain Deterministic Integrity
- `hdk ^0.6.0`: Holochain Development Kit

## Privacy & Consent Model

The consent zome implements a **patient-centric consent model**:

- **Grantor**: Always the patient (or their legal representative)
- **Grantee**: Agent, role, organization, or emergency access
- **Scope**: Specific data categories with optional exclusions
- **Time-bounded**: Optional expiration dates
- **Revocable**: Patient can revoke at any time
- **Auditable**: All access logged for transparency

### Data Sensitivity Levels

| Category | Sensitivity | Default Access |
|----------|-------------|----------------|
| Demographics | Medium | Opt-in |
| Medications | High | Explicit consent |
| Mental Health | Very High | Explicit consent + restrictions |
| Genetic Data | Very High | Explicit consent + restrictions |
| Financial | High | Explicit consent |

### Emergency Access

When `is_emergency: true` is set, the system will indicate if emergency override is available even without explicit consent. Emergency access is always logged and requires justification.

## Testing

### Unit Tests

```bash
cargo test -p consent_integrity --target x86_64-unknown-linux-gnu
cargo test -p consent_coordinator --target x86_64-unknown-linux-gnu
```

### SDK Tests

```bash
cd sdk
npm test -- tests/consent-client.test.ts
```

## HIPAA Compliance

This zome supports HIPAA compliance through:

- **Minimum Necessary**: Consent scopes limit access to needed data
- **Individual Rights**: Patients control their consent directives
- **Audit Controls**: All access logged via `DataAccessLog`
- **Access Controls**: Authorization checked before data access
- **Integrity**: Immutable audit trail on Holochain DHT

## Changelog

### v0.1.0 (2026-01)

- Initial implementation
- Consent CRUD operations
- Authorization checking
- Data access logging
- Emergency access support

## Related Zomes

- `patient`: Patient records protected by consent
- `mental_health`: Sensitive data requiring explicit consent
- `fhir_bridge`: External data sharing controlled by consent
- `clinical_decision_support`: CDS access controlled by consent
