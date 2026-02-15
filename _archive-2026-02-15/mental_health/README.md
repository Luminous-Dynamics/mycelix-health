# Mental Health Zome

> Behavioral health management with enhanced privacy, validated screening instruments, and crisis protocols.

## Overview

The Mental Health zome provides comprehensive behavioral health functionality including standardized screening instruments (PHQ-9, GAD-7, AUDIT, etc.), mood tracking, safety planning, crisis event reporting, and 42 CFR Part 2 compliant consent management for substance abuse records.

## Architecture

```
mental_health/
├── integrity/           # Entry type definitions & validation
│   └── src/
│       └── lib.rs       # Screening, MoodEntry, SafetyPlan, etc.
└── coordinator/         # Business logic & extern functions
    └── src/
        └── lib.rs       # Screening creation, mood tracking, crisis protocols
```

## Entry Types

| Entry Type | Description | Links |
|------------|-------------|-------|
| `Screening` | Standardized mental health screening (PHQ-9, GAD-7, etc.) | `PatientToScreenings` |
| `MoodEntry` | Daily mood tracking entry | `PatientToMoodEntries` |
| `SafetyPlan` | Crisis safety plan | `PatientToSafetyPlan` |
| `CrisisEvent` | Crisis event report | `PatientToCrisisEvents` |
| `Part2Consent` | 42 CFR Part 2 consent for substance abuse records | `PatientToPart2Consents` |
| `TherapyNote` | Encrypted therapy session notes | `PatientToTherapyNotes` |

## Extern Functions

### Screening Operations

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `create_screening` | `CreateScreeningInput` | `Record` | Create a new screening with auto-scoring |
| `get_patient_screenings` | `ActionHash` | `Vec<Record>` | Get all screenings for patient |
| `get_patient_screenings_paginated` | `GetScreeningsInput` | `PaginatedResult<Record>` | Get screenings with pagination |

### Mood Tracking

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `create_mood_entry` | `CreateMoodEntryInput` | `Record` | Create daily mood entry |
| `get_mood_entries` | `ActionHash` | `Vec<Record>` | Get all mood entries |
| `get_mood_entries_paginated` | `GetMoodEntriesInput` | `PaginatedResult<Record>` | Get entries with pagination |
| `get_recent_mood_entries` | `GetRecentMoodInput` | `Vec<Record>` | Get most recent N entries |

### Safety Planning

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `create_safety_plan` | `CreateSafetyPlanInput` | `Record` | Create or update safety plan |
| `get_safety_plan` | `ActionHash` | `Option<Record>` | Get current safety plan |

### Crisis Management

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `report_crisis_event` | `ReportCrisisEventInput` | `Record` | Report a crisis event |
| `get_crisis_history` | `ActionHash` | `Vec<Record>` | Get crisis event history |

### 42 CFR Part 2 Consent

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `create_part2_consent` | `CreatePart2ConsentInput` | `Record` | Create Part 2 consent |
| `revoke_part2_consent` | `ActionHash` | `Record` | Revoke Part 2 consent |
| `get_part2_consents` | `ActionHash` | `Vec<Record>` | Get all Part 2 consents |

### Therapy Notes

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `create_therapy_note` | `CreateTherapyNoteInput` | `Record` | Create encrypted therapy note |

## Supported Screening Instruments

| Instrument | Questions | Score Range | Use Case |
|------------|-----------|-------------|----------|
| **PHQ-9** | 9 | 0-27 | Depression screening |
| **GAD-7** | 7 | 0-21 | Anxiety screening |
| **PHQ-2** | 2 | 0-6 | Depression quick screen |
| **AUDIT** | 10 | 0-40 | Alcohol use screening |
| **CSSRS** | Variable | N/A | Suicide risk assessment |

### Score Interpretation

```rust
// PHQ-9 Depression Severity
0-4:   None-minimal depression
5-9:   Mild depression
10-14: Moderate depression (follow-up recommended)
15-19: Moderately severe depression
20-27: Severe depression

// GAD-7 Anxiety Severity
0-4:   Minimal anxiety
5-9:   Mild anxiety
10-14: Moderate anxiety (follow-up recommended)
15-21: Severe anxiety
```

## Input Types

```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateScreeningInput {
    pub patient_hash: ActionHash,
    pub instrument: MentalHealthInstrument,
    pub responses: Vec<(String, u8)>,  // (question_id, score)
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateMoodEntryInput {
    pub patient_hash: ActionHash,
    pub mood_score: u8,        // 1-10
    pub anxiety_score: u8,     // 1-10
    pub sleep_quality: u8,     // 1-10
    pub sleep_hours: Option<f32>,
    pub energy_level: u8,      // 1-10
    pub triggers: Vec<String>,
    pub coping_strategies_used: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateSafetyPlanInput {
    pub patient_hash: ActionHash,
    pub warning_signs: Vec<String>,
    pub coping_strategies: Vec<String>,
    pub support_contacts: Vec<SupportContact>,
    pub professional_contacts: Vec<ProfessionalContact>,
    pub safe_environment_steps: Vec<String>,
    pub reasons_for_living: Vec<String>,
}
```

## Validation

The mental health zome uses comprehensive input validation:

```rust
// Screening responses are validated against instrument specs
let validation = validate_screening_responses(&instrument_name, &input.responses);
validation.into_result()?;

// Mood entry scores are validated (1-10 range)
let mut validation = validate_mood_entry_scores(
    input.mood_score,
    input.anxiety_score,
    input.sleep_quality,
    input.energy_level
);
validation.merge(validate_sleep_hours(input.sleep_hours));
validation.into_result()?;
```

## Usage Examples

### SDK (TypeScript)

```typescript
import { MentalHealthClient } from '@mycelix/health-sdk';

const client = new MentalHealthClient(appClient, 'health');

// Create PHQ-9 screening
const screening = await client.createScreening({
  patient_hash: patientHash,
  instrument: 'PHQ9',
  responses: [
    ['q1', 2], ['q2', 1], ['q3', 3], ['q4', 2], ['q5', 1],
    ['q6', 2], ['q7', 1], ['q8', 0], ['q9', 1]
  ],
  notes: 'Patient reports improved sleep',
});

// Screening result includes auto-calculated score and severity
console.log(screening.total_score);     // 13
console.log(screening.severity);        // "Moderate"
console.log(screening.follow_up_needed); // true

// Create mood entry
const mood = await client.createMoodEntry({
  patient_hash: patientHash,
  mood_score: 6,
  anxiety_score: 4,
  sleep_quality: 7,
  sleep_hours: 7.5,
  energy_level: 5,
  triggers: ['work stress'],
  coping_strategies_used: ['deep breathing', 'walk'],
});

// Get mood trends
const recentMoods = await client.getRecentMoodEntries({
  patient_hash: patientHash,
  count: 30,
});

// Create safety plan
const safetyPlan = await client.createSafetyPlan({
  patient_hash: patientHash,
  warning_signs: ['isolating', 'not sleeping', 'increased irritability'],
  coping_strategies: ['call friend', 'go for walk', 'practice breathing'],
  support_contacts: [{ name: 'Jane', phone: '555-1234', relationship: 'friend' }],
  reasons_for_living: ['family', 'pets', 'future goals'],
});
```

## Privacy & Consent

Mental health data has the **highest sensitivity level** and requires:

- **Explicit Consent**: `DataCategory::MentalHealth` must be explicitly granted
- **42 CFR Part 2**: Substance abuse records require additional Part 2 consent
- **Break-the-Glass**: Emergency access logged and audited
- **Encryption**: Therapy notes encrypted at rest
- **Minimal Exposure**: Only authorized providers see full records

### Consent Check Flow

```rust
// All operations require authorization check
require_authorization(
    patient_hash.clone(),
    DataCategory::MentalHealth,
    Permission::Read,
)?;

// All access is logged
log_data_access(
    patient_hash.clone(),
    agent_info()?.agent_initial_pubkey,
    DataCategory::MentalHealth,
    "read",
    Some(record.action_address().clone()),
)?;
```

## Testing

### Unit Tests

```bash
cargo test -p mental_health_integrity --target x86_64-unknown-linux-gnu
cargo test -p mental_health_coordinator --target x86_64-unknown-linux-gnu
```

### SDK Tests

```bash
cd sdk
npm test -- tests/mental-health-client.test.ts
```

## Crisis Protocols

When a screening indicates elevated risk:

1. **Follow-up flag** is set on the screening record
2. **Crisis event** can be reported if needed
3. **Safety plan** is surfaced in client applications
4. **Emergency contacts** from safety plan are accessible
5. **Audit trail** captures all crisis-related access

## Dependencies

### Internal Dependencies

- `mycelix-health-shared`: Validation, authorization, pagination

### External Dependencies

- `hdi ^0.7.0`: Holochain Deterministic Integrity
- `hdk ^0.6.0`: Holochain Development Kit

## Changelog

### v0.1.0 (2026-01)

- Initial implementation
- PHQ-9, GAD-7, PHQ-2, AUDIT instrument support
- Mood tracking with validation
- Safety planning
- Crisis event reporting
- 42 CFR Part 2 consent management
- Input validation with typed errors
- Pagination support for queries

## Related Zomes

- `consent`: Authorization for mental health data access
- `patient`: Patient demographics linked to mental health records
- `clinical_decision_support`: CDS recommendations for mental health
- `fhir_bridge`: FHIR export of mental health observations
