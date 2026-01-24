# @mycelix/health-sdk

TypeScript SDK for Mycelix-Health Holochain hApp with built-in differential privacy safety.

## Features

- **Type-Safe API** - All types mirror Rust structs exactly
- **Privacy Budget Management** - Client-side "Fuel Gauge" for tracking DP budget
- **Safety Interlocks** - Automatic "Check Budget → Validate → Query" enforcement
- **Comprehensive Zome Coverage** - Clients for all 12 zomes:
  - **Core**: Patient, Consent, Commons, Trials
  - **Phase 3**: FHIR Mapping, CDS, Provider Directory, Telehealth
  - **Phase 4**: SDOH, Mental Health, Chronic Care, Pediatric
- **Accessibility Support** - WCAG 2.1 compliant with screen reader utilities

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
// Core
health.patients       // PatientClient
health.consent        // ConsentClient
health.commons        // CommonsClient
health.trials         // TrialsClient
// Phase 3 - Clinical Integration
health.fhirMapping    // FhirMappingClient
health.cds            // CdsClient
health.providerDirectory  // ProviderDirectoryClient
health.telehealth     // TelehealthClient
// Phase 4 - Equity & Access
health.sdoh           // SdohClient
health.mentalHealth   // MentalHealthClient
health.chronicCare    // ChronicCareClient
health.pediatric      // PediatricClient
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

### Phase 4 - Equity & Access Zomes

#### SdohClient

Social Determinants of Health screening and interventions.

```typescript
import { ScreeningInstrument, RiskLevel, SdohDomain } from '@mycelix/health-sdk';

// Create SDOH screening
const screening = await health.sdoh.createScreening({
  patientHash,
  instrument: ScreeningInstrument.PRAPARE,
  responses: [
    { questionId: 'food_1', questionText: 'Food insecurity', response: 'yes', riskIndicated: true },
  ],
});

// Get patient's SDOH summary
const summary = await health.sdoh.getPatientSdohSummary(patientHash);
console.log(`Overall risk: ${summary.overallRiskLevel}`);
console.log(`Domains at risk: ${summary.domainsAtRisk.join(', ')}`);

// Create an intervention referral
const intervention = await health.sdoh.createIntervention({
  screeningHash,
  patientHash,
  resourceHash,
  notes: 'Referred to food pantry',
});
```

#### MentalHealthClient

Mental health screening, safety plans, and 42 CFR Part 2 consent.

```typescript
import { MentalHealthInstrument, CrisisLevel, Part2ConsentType } from '@mycelix/health-sdk';

// Create PHQ-9 screening
const screening = await health.mentalHealth.createScreening({
  patientHash,
  instrument: MentalHealthInstrument.PHQ9,
  responses: [
    ['q1', 2], ['q2', 1], ['q3', 0], // ... all 9 questions
  ],
});

// Create a safety plan
const safetyPlan = await health.mentalHealth.createSafetyPlan({
  patientHash,
  warningSigns: ['Feeling hopeless', 'Isolating from friends'],
  internalCopingStrategies: ['Deep breathing', 'Going for a walk'],
  peopleForDistraction: [{ name: 'Friend', phone: '555-1234' }],
  professionalsToContact: [{ name: 'Therapist', phone: '555-5678' }],
  crisisLine988: true,
  reasonsForLiving: ['Family', 'Pets'],
});

// 42 CFR Part 2 consent for substance abuse records
const part2Consent = await health.mentalHealth.createPart2Consent({
  patientHash,
  consentType: Part2ConsentType.GeneralDisclosure,
  disclosingProgram: 'Recovery Center',
  recipientName: 'Primary Care Provider',
  purpose: 'Coordinate care',
  substancesCovered: ['Alcohol', 'Opioids'],
});
```

#### ChronicCareClient

Chronic disease management for diabetes, heart failure, COPD, and CKD.

```typescript
import { DiabetesType, NYHAClass, AlertSeverity } from '@mycelix/health-sdk';

// Enroll patient in chronic care program
const enrollment = await health.chronicCare.enrollPatient({
  patientHash,
  condition: { type: 'Diabetes', data: DiabetesType.Type2 },
  diagnosisDate: Date.now() * 1000,
  primaryProviderHash: providerHash,
});

// Create care plan
const carePlan = await health.chronicCare.createCarePlan({
  enrollmentHash,
  patientHash,
  condition: { type: 'Diabetes', data: DiabetesType.Type2 },
  goals: [{ goalId: 'hba1c', description: 'Reduce HbA1c to 7%', targetValue: '7' }],
  medications: ['Metformin 500mg twice daily'],
  selfManagementTasks: ['Check blood sugar daily', 'Log meals'],
  nextReviewDate: Date.now() * 1000 + 90 * 24 * 60 * 60 * 1000000, // 90 days
});

// Record diabetes metrics
await health.chronicCare.recordDiabetesMetrics({
  patientHash,
  measurementDate: Date.now() * 1000,
  fastingGlucose: 120,
  hba1c: 7.2,
  hypoglycemicEvents: 0,
  hyperglycemicEvents: 1,
});

// Check medication adherence
const adherence = await health.chronicCare.getAdherenceRate({
  patientHash,
  medicationName: 'Metformin',
});
console.log(`Adherence: ${(adherence.adherenceRate * 100).toFixed(1)}%`);
```

#### PediatricClient

Pediatric care including growth, immunizations, and developmental milestones.

```typescript
import { VaccineType, DevelopmentalDomain, MilestoneStatus } from '@mycelix/health-sdk';

// Record growth measurement
const growth = await health.pediatric.recordGrowth({
  patientHash,
  ageMonths: 12,
  weightKg: 9.5,
  heightCm: 75,
  headCircumferenceCm: 46,
});

// Calculate percentiles
const percentiles = await health.pediatric.calculateGrowthPercentiles({
  ageMonths: 12,
  sex: 'female',
  weightKg: 9.5,
  heightCm: 75,
  headCircumferenceCm: 46,
});
console.log(`Weight: ${percentiles.weightPercentile}th percentile`);

// Record immunization
await health.pediatric.recordImmunization({
  patientHash,
  vaccineType: VaccineType.DTaP,
  vaccineName: 'DTaP (Diphtheria, Tetanus, Pertussis)',
  lotNumber: 'LOT123',
  expirationDate: Date.now() * 1000 + 365 * 24 * 60 * 60 * 1000000,
  doseNumber: 2,
  dosesInSeries: 5,
  administeredAt: 'Right thigh',
  visGiven: true,
});

// Check immunization status
const status = await health.pediatric.getImmunizationStatus({
  patientHash,
  ageMonths: 12,
});
console.log(`Up to date: ${status.upToDate}`);
console.log(`Missing: ${status.missingVaccines.join(', ')}`);

// Record developmental milestone
await health.pediatric.recordMilestone({
  patientHash,
  ageMonths: 12,
  domain: DevelopmentalDomain.GrossMotor,
  milestoneName: 'Walks with support',
  expectedAgeMonths: 12,
  status: MilestoneStatus.Achieved,
  referralMade: false,
});
```

### Accessibility Support

WCAG 2.1 compliant utilities for building accessible healthcare UIs.

```typescript
import {
  formatPatientForScreenReader,
  formatPrivacyBudgetForScreenReader,
  checkContrast,
  getRiskLevelLabel,
  ReadingLevel,
  formatMedicalTermForScreenReader,
} from '@mycelix/health-sdk/accessibility';

// Format patient for screen reader
const patient = await health.patients.getPatient(hash);
const accessible = formatPatientForScreenReader(patient);
console.log(accessible.ariaLabel); // "Patient John Doe"
console.log(accessible.summary); // Brief description

// Get accessible risk labels
const risk = getRiskLevelLabel('HighRisk');
// { label: 'High risk, intervention recommended', ariaLive: 'assertive' }

// Check color contrast for WCAG compliance
const contrast = checkContrast('#000000', '#FFFFFF');
console.log(`Ratio: ${contrast.ratio.toFixed(2)}:1`);
console.log(`Passes AA: ${contrast.passesAA}`);
console.log(`Passes AAA: ${contrast.passesAAA}`);

// Format medical terms at appropriate reading level
const term = formatMedicalTermForScreenReader(
  'hypertension',
  'I10',
  ReadingLevel.Elementary
);
// "high blood pressure"

// Privacy budget for screen readers
const budgetInfo = formatPrivacyBudgetForScreenReader(1.5, 10.0);
console.log(budgetInfo.ariaLabel); // "Privacy budget: 15% remaining..."
console.log(budgetInfo.explanation); // User-friendly explanation
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

  // Core Enums
  ConsentScope,
  TrialPhase,
  TrialStatus,
  GovernanceModel,
  NoiseMechanism,

  // SDOH types
  SdohScreening,
  SdohIntervention,
  CommunityResource,
} from '@mycelix/health-sdk';

// SDOH enums
import {
  ScreeningInstrument,
  SdohDomain,
  SdohCategory,
  RiskLevel,
  InterventionStatus,
  ResourceType,
} from '@mycelix/health-sdk';

// Mental Health types & enums
import type {
  MentalHealthScreening,
  SafetyPlan,
  Part2Consent,
} from '@mycelix/health-sdk';

import {
  MentalHealthInstrument,
  Severity,
  CrisisLevel,
  TreatmentModality,
  SafetyPlanStatus,
  SubstanceCategory,
  Part2ConsentType,
} from '@mycelix/health-sdk';

// Chronic Care types & enums
import type {
  ChronicDiseaseEnrollment,
  ChronicCarePlan,
  DiabetesMetrics,
  HeartFailureMetrics,
  COPDMetrics,
} from '@mycelix/health-sdk';

import {
  DiabetesType,
  NYHAClass,
  GOLDStage,
  CKDStage,
  AlertSeverity,
} from '@mycelix/health-sdk';

// Pediatric types & enums
import type {
  GrowthMeasurement,
  ImmunizationRecord,
  DevelopmentalMilestone,
  WellChildVisit,
} from '@mycelix/health-sdk';

import {
  VaccineType,
  ImmunizationStatus,
  DevelopmentalDomain,
  MilestoneStatus,
  FeedingType,
} from '@mycelix/health-sdk';

// Accessibility types
import {
  ReadingLevel,
  HealthStatusCategory,
} from '@mycelix/health-sdk';
```

## Requirements

- Node.js 18+
- Holochain conductor running with mycelix-health hApp installed
- `@holochain/client` ^0.18.0

## License

MIT
