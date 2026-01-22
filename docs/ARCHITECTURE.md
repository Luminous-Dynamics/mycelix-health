# Mycelix-Health Architecture

## Overview

Mycelix-Health is built on Holochain, providing a truly decentralized, agent-centric healthcare data platform. Each patient runs their own source chain, ensuring complete data sovereignty.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Mycelix-Health hApp                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │   Patient    │  │   Provider   │  │   Records    │              │
│  │    Zome      │  │    Zome      │  │    Zome      │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
│                                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │ Prescriptions│  │   Consent    │  │   Trials     │              │
│  │    Zome      │  │    Zome      │  │    Zome      │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
│                                                                      │
│  ┌──────────────┐  ┌──────────────┐                                 │
│  │  Insurance   │  │   Bridge     │                                 │
│  │    Zome      │  │    Zome      │                                 │
│  └──────────────┘  └──────────────┘                                 │
│                                                                      │
├─────────────────────────────────────────────────────────────────────┤
│                        Health DNA                                    │
├─────────────────────────────────────────────────────────────────────┤
│                     Holochain Conductor                              │
└─────────────────────────────────────────────────────────────────────┘
```

## Zome Dependencies

```
                    ┌─────────────────────┐
                    │  patient_integrity  │
                    └─────────┬───────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│   provider    │   │    records    │   │    consent    │
└───────┬───────┘   └───────┬───────┘   └───────────────┘
        │                   │
        │    ┌──────────────┤
        │    │              │
        ▼    ▼              ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│ prescriptions │   │    trials     │   │   insurance   │
└───────────────┘   └───────────────┘   └───────────────┘
        │                   │                   │
        └───────────────────┼───────────────────┘
                            │
                            ▼
                    ┌───────────────┐
                    │    bridge     │
                    └───────────────┘
```

## Data Flow

### Patient Registration
```
User → Patient Zome → Create Patient Entry → Link to Global Index
                   → Link to Mycelix Identity (optional)
```

### Medical Record Creation
```
Provider → Check Consent → Records Zome → Create Encounter
                                       → Create Diagnoses
                                       → Create Procedures
                                       → Link All to Patient
```

### Prescription Workflow
```
Provider → Prescriptions Zome → Create Prescription
                             → Check Drug Interactions
                             → Send to Pharmacy
Pharmacy → Fill Prescription → Update Refills
Patient  → Record Adherence
```

### Consent Management
```
Patient → Consent Zome → Create Consent Directive
                      → Define Scope (data categories, time, grantees)
                      → Set Permissions (read, write, share)

Provider → Check Authorization → Access if Authorized
                              → Log Access
```

## Entry Type Hierarchy

### Patient Domain
```
Patient
├── PatientIdentityLink
└── PatientHealthSummary
```

### Provider Domain
```
Provider
├── License
├── BoardCertification
└── ProviderPatientRelationship
```

### Records Domain
```
Encounter
├── Diagnosis
├── ProcedurePerformed
├── LabResult
├── ImagingStudy
└── VitalSigns
```

### Prescriptions Domain
```
Prescription
├── PrescriptionFill
├── MedicationAdherence
└── DrugInteractionAlert

Pharmacy (standalone)
```

### Consent Domain
```
Consent
├── DataAccessRequest
├── DataAccessLog
├── EmergencyAccess
└── AuthorizationDocument
```

### Trials Domain
```
ClinicalTrial
├── TrialParticipant
├── TrialVisit
└── AdverseEvent
```

### Insurance Domain
```
InsurancePlan
├── Claim
├── PriorAuthorization
├── EligibilityCheck
└── ExplanationOfBenefits
```

### Bridge Domain
```
HealthBridgeRegistration
├── HealthDataQuery
├── HealthDataResponse
├── ProviderVerificationRequest
├── ProviderVerificationResult
├── HealthEpistemicClaim
└── HealthReputationFederation
```

## Link Types

Each zome defines specific link types for efficient querying:

| Zome | Key Links |
|------|-----------|
| Patient | PatientToRecords, PatientToConsents, AllPatients |
| Provider | ProviderToLicenses, ProviderToPatients, ProvidersBySpecialty |
| Records | PatientToEncounters, CriticalResults |
| Prescriptions | PatientToPrescriptions, ControlledSubstances |
| Consent | PatientToConsents, ActiveConsents, RevokedConsents |
| Trials | TrialToParticipants, RecruitingTrials |
| Insurance | PatientToPlans, PendingClaims |
| Bridge | ActiveRegistrations, EntityToClaims |

## Trust Model

### MATL Integration
- Every entity has a `matl_trust_score` (0.0-1.0)
- Scores aggregate from multiple sources
- Byzantine-tolerant with up to 45% malicious actors

### Epistemic Classification
- E0: Unverified (patient-reported)
- E1: Verified (provider observation)
- E2: Replicated (test confirmed)
- E3: Consensus (multi-provider agreement)

### Provider Trust Factors
- License verification status
- Board certifications
- Patient outcome data
- Peer attestations
- Insurance network participation

## Security Architecture

### Data Protection
```
┌─────────────────────────────────────────────┐
│              Patient's Device                │
│  ┌───────────────────────────────────────┐  │
│  │          Source Chain                  │  │
│  │  ┌─────────────────────────────────┐  │  │
│  │  │  Encrypted Sensitive Data       │  │  │
│  │  │  (Demographics, Diagnoses)      │  │  │
│  │  └─────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────┐  │  │
│  │  │  Access Control Entries         │  │  │
│  │  │  (Consents, Audit Logs)         │  │  │
│  │  └─────────────────────────────────┘  │  │
│  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

### Access Control
1. **Consent Check**: Every data access requires valid consent
2. **Audit Logging**: All access is logged immutably
3. **Emergency Override**: Break-glass with mandatory review
4. **Expiration**: Time-bound consents auto-expire

### Validation Rules
- Patient ID required and unique
- Medical codes (ICD-10, CPT, LOINC) validated
- MATL scores constrained to [0.0, 1.0]
- Controlled substances require DEA number
- Critical results require acknowledgment

## Integration Points

### External Systems
- **EHR Systems**: FHIR API bridge
- **Lab Systems**: HL7v2/FHIR results import
- **Pharmacy Systems**: NCPDP SCRIPT integration
- **Insurance**: X12 EDI claim submission

### Mycelix Ecosystem
- **Identity**: Decentralized identity verification
- **DeSci**: Research publication and peer review
- **Finance**: Healthcare payment processing
- **Justice**: Dispute resolution
- **Marketplace**: Medical supplies

## Scalability Considerations

### Agent-Centric Model
- Each patient stores only their data
- Providers store only their credentials
- No central bottleneck

### DHT Distribution
- Records distributed across network
- Redundancy through gossip protocol
- Local-first with eventual consistency

### Sharding Strategy
- Separate DNA instances per region (optional)
- Bridge zome connects regions
- Privacy-preserving cross-region queries

## Future Considerations

### Federated Learning
- Train models on distributed patient data
- Privacy-preserving aggregation
- Research without data exposure

### Zero-Knowledge Proofs
- Prove eligibility without revealing data
- Age verification for trials
- Insurance coverage confirmation

### Interoperability
- SMART on FHIR apps
- Apple Health / Google Fit
- Wearable device integration
