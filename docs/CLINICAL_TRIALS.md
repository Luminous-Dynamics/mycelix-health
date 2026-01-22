# Clinical Trials in Mycelix-Health

## Overview

Mycelix-Health includes comprehensive support for clinical trial management, from study registration through adverse event reporting. This enables decentralized clinical research while maintaining patient sovereignty over their data.

## Why Clinical Trials?

### Traditional Problems
- **Patient Recruitment**: Finding eligible patients is expensive and slow
- **Data Silos**: Patient data trapped in different EHR systems
- **Consent Challenges**: Paper-based, hard to verify
- **Data Quality**: Manual entry, high error rates
- **Follow-up**: Patients lost to follow-up

### Mycelix-Health Solutions
- **Eligibility Matching**: Patients can opt-in to trial matching
- **Unified Data**: Patient controls complete medical history
- **Cryptographic Consent**: Verifiable, revocable, auditable
- **Direct Entry**: Data captured at source
- **Patient Engagement**: Direct communication channel

## Trial Lifecycle

### 1. Study Registration

```rust
ClinicalTrial {
    trial_id: String,
    nct_number: Option<String>,  // ClinicalTrials.gov number
    title: String,
    phase: TrialPhase,           // Phase1, Phase2, Phase3, Phase4
    study_type: StudyType,       // Interventional, Observational
    status: TrialStatus,         // Recruiting, Active, Completed
    principal_investigator: AgentPubKey,
    sponsor: String,
    eligibility: EligibilityCriteria,
    interventions: Vec<Intervention>,
    outcomes: Vec<Outcome>,
    // ... more fields
}
```

### 2. Eligibility Criteria

```rust
EligibilityCriteria {
    min_age: Option<u32>,
    max_age: Option<u32>,
    sex: EligibleSex,
    healthy_volunteers: bool,
    inclusion_criteria: Vec<String>,
    exclusion_criteria: Vec<String>,
}
```

### 3. Patient Matching

Patients can opt-in to receive trial recommendations:

```
Patient Profile + Active Conditions + Medications
                    ↓
           Eligibility Algorithm
                    ↓
         Matching Trials List
                    ↓
    Patient Reviews & Expresses Interest
                    ↓
       Site Coordinator Contacts
```

### 4. Informed Consent

```rust
// Trial-specific consent linked to patient
TrialParticipant {
    participant_id: String,
    trial_hash: ActionHash,
    patient_hash: ActionHash,
    consent_hash: ActionHash,  // Links to Consent zome
    enrollment_date: Timestamp,
    arm_assignment: Option<String>,
    status: ParticipantStatus,
    // ...
}
```

Consent features:
- Electronic signature capture
- Version control for consent documents
- Withdrawal tracking
- Re-consent for protocol amendments

### 5. Data Collection

```rust
TrialVisit {
    visit_id: String,
    participant_hash: ActionHash,
    visit_number: u32,
    visit_name: String,
    scheduled_date: Timestamp,
    actual_date: Option<Timestamp>,
    status: VisitStatus,
    data_points: Vec<DataPoint>,
    protocol_deviations: Vec<ProtocolDeviation>,
}

DataPoint {
    name: String,
    value: String,
    unit: Option<String>,
    collected_at: Timestamp,
    collected_by: AgentPubKey,
    source: DataSource,  // DirectEntry, LabResult, DeviceImport
}
```

### 6. Adverse Event Reporting

```rust
AdverseEvent {
    event_id: String,
    participant_hash: ActionHash,
    event_term: String,
    description: String,
    onset_date: Timestamp,
    severity: AESeverity,        // Mild, Moderate, Severe, LifeThreatening
    seriousness: Vec<SeriousnessCriteria>,
    causality: Causality,        // Related, Possibly, Unlikely, Not
    outcome: AEOutcome,
    action_taken: Vec<ActionTaken>,
    medwatch_submitted: bool,
}
```

Serious adverse events (SAEs) trigger:
- Automatic notification to PI
- IRB reporting workflow
- FDA MedWatch integration
- DSMB alerts if configured

## Regulatory Compliance

### FDA 21 CFR Part 11
Requirements for electronic records:
- ✅ **Audit Trails**: Immutable Holochain source chain
- ✅ **Electronic Signatures**: Cryptographic agent signatures
- ✅ **Access Controls**: Consent-based authorization
- ✅ **Version Control**: Entry updates tracked
- ✅ **Data Integrity**: Hash-linked entries

### ICH E6 Good Clinical Practice
- ✅ **Protocol Compliance**: Deviation tracking
- ✅ **Source Documentation**: Direct data capture
- ✅ **Informed Consent**: Electronic with versioning
- ✅ **Data Quality**: Validation at entry
- ✅ **Safety Reporting**: AE/SAE workflows

### HIPAA
- ✅ **Minimum Necessary**: Scoped consent for research
- ✅ **Authorization**: Explicit research consent
- ✅ **Accounting of Disclosures**: Access logs

## DeSci Integration

Mycelix-Health connects clinical trials to the decentralized science ecosystem:

### Research Publication Flow
```
Trial Completion → Results Analysis → DeSci Publication
                                   → Peer Review
                                   → Epistemic Classification
                                   → Citation Network
```

### Researcher Profiles
- Link to Mycelix-DeSci researcher profiles
- Track h-index and publication history
- Verify institutional affiliations
- MATL trust scoring for researchers

### Data Sharing
- Anonymized datasets for meta-analysis
- Federated learning on distributed data
- Patient consent for secondary use
- Attribution and citation tracking

## API Examples

### Register a Trial
```typescript
const trial = await client.callZome({
  cell_id: healthCell,
  zome_name: 'trials',
  fn_name: 'create_trial',
  payload: {
    trial_id: 'TRIAL-2026-001',
    nct_number: 'NCT05123456',
    title: 'Phase 3 Study of Novel Treatment',
    phase: 'Phase3',
    study_type: 'Interventional',
    status: 'Recruiting',
    principal_investigator: myPubKey,
    sponsor: 'Luminous Therapeutics',
    target_enrollment: 500,
    eligibility: {
      min_age: 18,
      max_age: 65,
      sex: 'All',
      healthy_volunteers: false,
      inclusion_criteria: ['Confirmed diagnosis of X'],
      exclusion_criteria: ['Pregnancy', 'Active malignancy'],
    },
    // ...
  },
});
```

### Check Eligibility
```typescript
const result = await client.callZome({
  cell_id: healthCell,
  zome_name: 'trials',
  fn_name: 'check_eligibility',
  payload: {
    trial_hash: trialActionHash,
    patient_hash: patientActionHash,
    patient_age: 45,
  },
});

// Result:
// {
//   eligible: true,
//   reasons: [],
//   trial_title: 'Phase 3 Study of Novel Treatment',
//   trial_phase: 'Phase3',
// }
```

### Enroll Participant
```typescript
const participant = await client.callZome({
  cell_id: healthCell,
  zome_name: 'trials',
  fn_name: 'enroll_participant',
  payload: {
    participant_id: 'PART-001',
    trial_hash: trialActionHash,
    patient_hash: patientActionHash,
    consent_hash: trialConsentHash,
    enrollment_date: Date.now() * 1000,
    arm_assignment: 'Treatment',
    status: 'Enrolled',
    blinded: true,
    screening_passed: true,
    enrollment_site: 'Site A',
    primary_contact: coordinatorPubKey,
  },
});
```

### Report Adverse Event
```typescript
const ae = await client.callZome({
  cell_id: healthCell,
  zome_name: 'trials',
  fn_name: 'report_adverse_event',
  payload: {
    event_id: 'AE-001',
    participant_hash: participantActionHash,
    trial_hash: trialActionHash,
    event_term: 'Headache',
    description: 'Mild headache starting 2 hours post-dose',
    onset_date: Date.now() * 1000,
    severity: 'Mild',
    seriousness: [],
    is_serious: false,
    is_unexpected: false,
    causality: 'PossiblyRelated',
    outcome: 'Recovered',
    action_taken: ['NoneRequired'],
    reported_by: myPubKey,
    reported_at: Date.now() * 1000,
    medwatch_submitted: false,
  },
});
```

## Patient Experience

### Finding Trials
1. Patient enables trial matching in settings
2. System checks eligibility against recruiting trials
3. Patient receives notifications of matches
4. Patient reviews trial details and contacts
5. Site coordinator reaches out

### Participating
1. Complete informed consent (electronic)
2. Screening visit scheduled
3. Baseline data collected from existing records
4. Study visits with direct data entry
5. Ongoing communication through secure channel

### Withdrawing
1. Patient can withdraw at any time
2. Reason recorded (optional)
3. Follow-up data collection stops
4. Safety monitoring may continue
5. Data collected remains (per consent)

## Future Enhancements

### Decentralized IRB
- Community-based ethics review
- Transparent deliberation process
- MATL-weighted voting
- Appeal mechanisms

### Smart Trial Design
- Adaptive designs with automated dose adjustment
- Platform trials with multiple arms
- Real-world evidence integration
- Patient-reported outcomes apps

### Token Incentives
- Compensation for participation
- Data contribution rewards
- Research milestone tokens
- Community governance rights
