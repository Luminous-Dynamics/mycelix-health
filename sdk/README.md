# @mycelix/health-sdk

TypeScript SDK for Mycelix-Health Holochain hApp with built-in differential privacy safety.

## Features

- **Type-Safe API** - All types mirror Rust structs exactly
- **Privacy Budget Management** - Client-side "Fuel Gauge" for tracking DP budget
- **Safety Interlocks** - Automatic "Check Budget → Validate → Query" enforcement
- **Full Zome Coverage** - Clients for Patient, Consent, Commons, and Trials zomes

## Installation

```bash
npm install @mycelix/health-sdk
# or
yarn add @mycelix/health-sdk
```

## Quick Start

```typescript
import { MycelixHealthClient } from '@mycelix/health-sdk';

// Connect to Holochain
const health = await MycelixHealthClient.connect({
  url: 'ws://localhost:8888',
  appId: 'mycelix-health',
});

// Create a patient record
const patient = await health.patients.createPatient({
  first_name: 'Jane',
  last_name: 'Doe',
  date_of_birth: '1990-01-15',
  contact: { email: 'jane@example.com' },
});

console.log('Patient created:', patient.hash);
```

## Privacy-Safe Queries

The SDK enforces differential privacy safety at the client level:

```typescript
import { MycelixHealthClient, PrivacyBudgetManager } from '@mycelix/health-sdk';

const health = await MycelixHealthClient.connect();

// Check budget before querying
const status = await health.commons.getBudgetStatus(patientHash, poolHash);
console.log(`Privacy budget: ${status.percentRemaining}% remaining`);

// Get UI-friendly display info
const display = PrivacyBudgetManager.getDisplayInfo(status);
console.log(display.message); // "Warning: 15% budget remaining"

// Execute DP query with automatic safety enforcement
// This will throw if budget is insufficient
const result = await health.commons.countWithPrivacy(
  poolHash,
  patientHash,
  0.5  // epsilon
);

console.log(`Noisy count: ${result.value}`);
console.log(`Epsilon consumed: ${result.epsilon_consumed}`);
```

## API Reference

### MycelixHealthClient

Main entry point providing access to all zome clients.

```typescript
// Connect with options
const health = await MycelixHealthClient.connect({
  url: 'ws://localhost:8888',
  appId: 'mycelix-health',
  roleName: 'mycelix_health',
  debug: true,
  retry: {
    maxAttempts: 3,
    delayMs: 1000,
    backoffMultiplier: 2,
  },
});

// Or use an existing client
import { AppWebsocket } from '@holochain/client';
const client = await AppWebsocket.connect({ url: new URL('ws://localhost:8888') });
const health = MycelixHealthClient.fromClient(client);

// Access zome clients
health.patients   // PatientClient
health.consent    // ConsentClient
health.commons    // CommonsClient
health.trials     // TrialsClient
```

### PatientClient

Patient record management.

```typescript
// Create patient
const patient = await health.patients.createPatient({
  first_name: 'Jane',
  last_name: 'Doe',
  date_of_birth: '1990-01-15',
  contact: { email: 'jane@example.com' },
  emergency_contacts: [{ name: 'John Doe', relationship: 'Spouse', phone: '555-0100' }],
  allergies: ['Penicillin'],
  medications: ['Metformin'],
});

// Get patient by hash
const record = await health.patients.getPatient(patientHash);

// Search patients
const results = await health.patients.searchPatients({ name: 'Jane' });

// Update patient
const updated = await health.patients.updatePatient(patientHash, {
  contact: { email: 'jane.doe@newmail.com' },
});
```

### ConsentClient

Consent grants, revocations, and authorization checks.

```typescript
// Grant consent
const consent = await health.consent.grantConsent(patientHash, {
  grantee: providerPubKey,
  scope: 'ReadOnly',
  data_categories: ['demographics', 'medications'],
  purpose: 'Primary care treatment',
  valid_until: Date.now() * 1000 + 365 * 24 * 60 * 60 * 1000000, // 1 year
});

// Check authorization
const auth = await health.consent.checkAuthorization({
  patient_hash: patientHash,
  requester: providerPubKey,
  action: 'read',
  data_categories: ['medications'],
});

if (auth.authorized) {
  console.log('Access granted via consent:', auth.consent_hash);
}

// Revoke consent
await health.consent.revokeConsent(consentHash);
```

### CommonsClient

Data pools and differentially private queries.

```typescript
// Create a data pool
const pool = await health.commons.createPool({
  name: 'Diabetes Research Pool',
  description: 'Aggregate data for diabetes research',
  data_categories: ['lab_results', 'medications'],
  required_consent_level: 'ResearchOnly',
  default_epsilon: 1.0,
  budget_per_user: 10.0,
  governance_model: 'Democratic',
  min_contributors: 100,
});

// Check if a query can proceed
const canQuery = await health.commons.canQuery(patientHash, poolHash, 0.5);

// Execute count query with DP
const count = await health.commons.countWithPrivacy(poolHash, patientHash, 0.5);

// Execute sum query with DP
const sum = await health.commons.sumWithPrivacy(
  poolHash,
  patientHash,
  0.5,    // epsilon
  1000    // max value (sensitivity)
);

// Execute average query with DP (uses Gaussian mechanism)
const avg = await health.commons.averageWithPrivacy(
  poolHash,
  patientHash,
  0.5,    // epsilon
  1e-6,   // delta
  100,    // value range
  50      // min contributors
);
```

### TrialsClient

Clinical trial management.

```typescript
// Create a trial
const trial = await health.trials.createTrial({
  trial_id: 'NCT12345678',
  title: 'Phase 3 Diabetes Drug Trial',
  description: 'Testing new diabetes medication',
  sponsor: 'Acme Pharma',
  phase: 'Phase3',
  eligibility_criteria: {
    min_age: 18,
    max_age: 65,
    conditions: ['Type 2 Diabetes'],
    exclusions: ['Pregnancy', 'Kidney Disease'],
  },
  target_enrollment: 500,
  start_date: Date.now() * 1000,
});

// Check eligibility
const eligibility = await health.trials.checkEligibility(trialHash, patientHash);
if (eligibility.eligible) {
  // Enroll patient
  const enrollment = await health.trials.enrollPatient(
    trialHash,
    patientHash,
    consentHash
  );
}

// Report adverse event
await health.trials.reportAdverseEvent({
  trial_hash: trialHash,
  patient_hash: patientHash,
  event_type: 'Nausea',
  severity: 'Mild',
  description: 'Patient reported mild nausea after dose',
  onset_date: Date.now() * 1000,
  related_to_treatment: true,
});
```

### PrivacyBudgetManager

Client-side budget tracking and validation utilities.

```typescript
import { PrivacyBudgetManager, RECOMMENDED_EPSILON } from '@mycelix/health-sdk';

// Calculate status from ledger entry
const status = PrivacyBudgetManager.calculateStatus(ledgerEntry);

// Validate a query can proceed (throws if not)
PrivacyBudgetManager.validateQuery(status, epsilon);

// Estimate remaining queries
const remaining = PrivacyBudgetManager.estimateRemainingQueries(status, 0.5);
console.log(`Can answer ${remaining} more queries at epsilon=0.5`);

// Calculate optimal epsilon for N queries
const optimalEpsilon = PrivacyBudgetManager.calculateOptimalEpsilon(status, 10);
console.log(`Use epsilon=${optimalEpsilon} to answer 10 queries`);

// Get UI display info
const display = PrivacyBudgetManager.getDisplayInfo(status);
// {
//   severity: 'warning',
//   message: 'Warning: 15% budget remaining',
//   remaining: '1.5000',
//   total: '10.0000',
//   percentRemaining: 15,
//   canQuery: true,
// }

// Simulate future consumption
const simulated = PrivacyBudgetManager.simulateConsumption(status, [0.5, 0.5, 0.5]);
console.log(`After 3 queries: ${simulated.percentRemaining}% remaining`);

// Use recommended epsilon values
const epsilon = RECOMMENDED_EPSILON.HIGH_SENSITIVITY; // 0.5
```

## Error Handling

All SDK errors extend `HealthSdkError`:

```typescript
import { HealthSdkError, HealthSdkErrorCode } from '@mycelix/health-sdk';

try {
  await health.commons.countWithPrivacy(poolHash, patientHash, 0.5);
} catch (error) {
  if (error instanceof HealthSdkError) {
    switch (error.code) {
      case HealthSdkErrorCode.BUDGET_EXHAUSTED:
        console.log('Privacy budget exhausted:', error.details);
        break;
      case HealthSdkErrorCode.INVALID_EPSILON:
        console.log('Invalid epsilon value:', error.details);
        break;
      case HealthSdkErrorCode.UNAUTHORIZED:
        console.log('No consent for this operation');
        break;
      default:
        console.log('SDK error:', error.message);
    }
  }
}
```

## Type Exports

All types are exported for use in your application:

```typescript
import type {
  // Core types
  Patient,
  Consent,
  ClinicalTrial,
  DataPool,

  // Privacy types
  BudgetLedgerEntry,
  PrivacyBudgetStatus,
  DpQueryParams,
  DpQueryResult,

  // Enums
  ConsentScope,
  TrialPhase,
  TrialStatus,
  GovernanceModel,
  NoiseMechanism,

  // Config
  MycelixHealthConfig,
} from '@mycelix/health-sdk';
```

## Requirements

- Node.js 18+
- Holochain conductor running with mycelix-health hApp installed
- `@holochain/client` ^0.18.0

## Development

```bash
# Install dependencies
npm install

# Run tests
npm test

# Build
npm run build

# Type check
npm run typecheck
```

## Publishing

The SDK is published to npm automatically when a GitHub release is created, or can be triggered manually.

To publish manually:

```bash
# Ensure you're logged into npm
npm login

# Build and test
npm run build
npm test

# Publish (scoped package requires --access public)
npm publish --access public
```

## License

MIT
