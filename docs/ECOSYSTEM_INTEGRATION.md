# Mycelix Ecosystem Integration

## Overview

Mycelix-Health is designed to work seamlessly with the broader Mycelix ecosystem, enabling cross-hApp data sharing, reputation federation, and coordinated workflows.

## Ecosystem Map

```
                    ┌─────────────────────────┐
                    │    Mycelix-Identity     │
                    │  (Identity Foundation)  │
                    └───────────┬─────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
        ▼                       ▼                       ▼
┌───────────────┐     ┌─────────────────┐     ┌───────────────┐
│ Mycelix-Health│◄───►│  Mycelix-Core   │◄───►│ Mycelix-DeSci │
│  (Healthcare) │     │(Bridge Protocol)│     │  (Research)   │
└───────┬───────┘     └────────┬────────┘     └───────┬───────┘
        │                      │                       │
        │              ┌───────┴───────┐              │
        │              │               │              │
        ▼              ▼               ▼              ▼
┌───────────────┐ ┌─────────┐ ┌───────────────┐ ┌───────────────┐
│Mycelix-Finance│ │ MATL    │ │Mycelix-Justice│ │Mycelix-EduNet │
│  (Payments)   │ │(Trust)  │ │  (Disputes)   │ │  (Learning)   │
└───────────────┘ └─────────┘ └───────────────┘ └───────────────┘
```

## Integration Points

### 1. Mycelix-Identity

**Purpose**: Decentralized identity verification for patients and providers

**Health → Identity**
- Link patient profiles to verified identities
- Provider credential attestations
- Emergency contact verification

**Identity → Health**
- Identity proofs for patient registration
- KYC for controlled substance prescribing
- Age verification for clinical trials

**API Example**
```typescript
// Link patient to Mycelix identity
const link = await healthClient.callZome({
  zome_name: 'patient',
  fn_name: 'link_patient_to_identity',
  payload: {
    patient_hash: patientActionHash,
    identity_provider: 'mycelix-identity',
    verification_method: 'did_verification',
    confidence_score: 0.95,
  },
});
```

### 2. Mycelix-DeSci

**Purpose**: Decentralized science platform for research publication

**Health → DeSci**
- Clinical trial results publication
- Researcher profile linking
- Peer review of medical claims

**DeSci → Health**
- Research evidence for treatment decisions
- Epistemic classification of findings
- Citation of source studies

**Integration Flow**
```
Trial Completion → Results Analysis
                         ↓
              DeSci Publication Creation
                         ↓
              Peer Review Process
                         ↓
         Epistemic Level Assignment (E1-E3)
                         ↓
       Health Claim Updated with Evidence
```

### 3. Mycelix-Finance

**Purpose**: Decentralized payment processing

**Health → Finance**
- Healthcare payment requests
- Insurance claim payments
- Research participant compensation

**Finance → Health**
- Payment confirmations
- Escrow for clinical trials
- Grant fund disbursement

**Use Cases**
- Patient pays provider directly
- Insurance processes claim payment
- Trial compensates participants
- Researcher receives grant funds

### 4. Mycelix-Justice

**Purpose**: Dispute resolution and arbitration

**Health → Justice**
- Medical malpractice claims
- Insurance coverage disputes
- Research misconduct cases

**Justice → Health**
- Dispute resolutions
- Credential suspensions
- Compensation awards

**Dispute Types**
| Category | Example |
|----------|---------|
| Provider | Malpractice allegation |
| Insurance | Claim denial appeal |
| Research | Protocol violation |
| Consent | Unauthorized access |

### 5. Mycelix-Marketplace

**Purpose**: Decentralized commerce platform

**Health → Marketplace**
- Medical equipment orders
- Pharmaceutical supplies
- Lab testing services

**Marketplace → Health**
- Order fulfillment tracking
- Product authenticity verification
- Supply chain provenance

## Bridge Protocol

The Bridge zome implements the standard Mycelix bridge protocol:

### Registration
```rust
HealthBridgeRegistration {
    registration_id: String,
    mycelix_identity_hash: ActionHash,
    happ_id: "mycelix-health",
    capabilities: Vec<HealthCapability>,
    federated_data: Vec<FederatedDataType>,
    minimum_trust_score: f64,
}
```

### Capabilities Offered
- `PatientLookup`: Find patients by identity
- `ProviderVerification`: Verify provider credentials
- `RecordSharing`: Share medical records
- `ConsentVerification`: Check consent status
- `ClaimsSubmission`: Submit insurance claims
- `TrialEnrollment`: Enroll in clinical trials
- `EpistemicClaims`: Create/verify medical claims
- `ReputationFederation`: Share trust scores

### Data Federation

#### Query Flow
```
Requesting hApp → HealthDataQuery → Health Bridge
                                        ↓
                              Consent Verification
                                        ↓
                              Data Retrieval
                                        ↓
                              HealthDataResponse
                                        ↓
                              Audit Log Entry
```

#### Query Example
```typescript
const query = await bridgeClient.callZome({
  zome_name: 'bridge',
  fn_name: 'query_federated_data',
  payload: {
    query_id: 'Q-001',
    requesting_agent: myPubKey,
    requesting_happ: 'mycelix-desci',
    patient_identity_hash: identityHash,
    data_types: ['MedicalHistory', 'Medications'],
    purpose: 'Research',
    consent_hash: researchConsentHash,
  },
});
```

## MATL Trust Integration

### Trust Score Sources

| Source | Weight | Description |
|--------|--------|-------------|
| Provider Credentials | 0.25 | Verified licenses, certifications |
| Patient Outcomes | 0.30 | Treatment success rates |
| Peer Attestations | 0.20 | Provider-to-provider endorsements |
| Patient Feedback | 0.15 | Direct patient ratings |
| Compliance | 0.10 | Protocol adherence, audit results |

### Federation Flow
```rust
HealthReputationFederation {
    federation_id: String,
    entity_hash: ActionHash,
    entity_type: HealthEntityType,
    scores: Vec<FederatedScore>,
    aggregated_score: f64,
    aggregated_at: Timestamp,
}

FederatedScore {
    source_happ: String,      // e.g., "mycelix-identity"
    score: f64,               // 0.0 - 1.0
    weight: f64,              // Importance factor
    score_type: String,       // e.g., "verification"
    timestamp: Timestamp,
}
```

### Aggregation Algorithm
```rust
// Weighted average with recency decay
aggregated_score = Σ(score_i × weight_i × decay_i) / Σ(weight_i × decay_i)

where decay_i = e^(-λ × age_days)
```

## Epistemic Claims

### Health Claim Types
- `Diagnosis`: Medical diagnosis claim
- `Treatment`: Treatment efficacy claim
- `Outcome`: Patient outcome claim
- `ProviderCompetency`: Provider skill claim
- `FacilityQuality`: Healthcare facility claim
- `MedicationEfficacy`: Drug effectiveness claim
- `AdverseEvent`: Side effect report
- `ResearchFinding`: Study result claim

### Classification
```rust
EpistemicClassification {
    empirical_level: u8,    // E0-E3
    materiality_level: u8,  // M0-M3
    normative_level: u8,    // N0-N3
}
```

### Verification Flow
```
Claim Creation → Evidence Attachment → Peer Verification
                                            ↓
                                  Multi-Party Attestation
                                            ↓
                                  Epistemic Level Assignment
                                            ↓
                                  MATL Score Update
```

## Cross-hApp Workflows

### 1. Clinical Trial → Research Publication

```
1. Trial completes → Health triggers completion event
2. Results compiled → Health exports data to DeSci
3. Publication created → DeSci manages peer review
4. Review complete → DeSci assigns E-level
5. Claim updated → Health receives verified claim
6. Trust updated → MATL propagates scores
```

### 2. Insurance Dispute → Resolution

```
1. Claim denied → Health records denial
2. Patient appeals → Health creates dispute
3. Dispute filed → Justice receives case
4. Evidence shared → Health provides records (with consent)
5. Arbitration → Justice conducts process
6. Resolution → Health updates claim status
7. Payment (if won) → Finance processes payment
```

### 3. Provider Verification → Identity Proof

```
1. Provider registers → Health creates profile
2. License claimed → Health records credentials
3. Verification request → Identity receives request
4. External verification → Identity checks sources
5. Attestation created → Identity provides proof
6. Trust updated → Health incorporates score
7. Badge displayed → Health shows verification
```

## Security Considerations

### Cross-hApp Data Sharing
- All sharing requires active consent
- Minimum necessary principle enforced
- Audit logs for all cross-hApp access
- Trust score requirements configurable

### Privacy Preservation
- Data stays on patient's source chain
- Queries return references, not data
- Zero-knowledge proofs for eligibility
- Encryption for sensitive categories

### Attack Vectors
| Vector | Mitigation |
|--------|------------|
| Replay attacks | Timestamps and nonces |
| Data exfiltration | Consent-based encryption |
| Sybil attacks | MATL reputation requirements |
| Collusion | Byzantine fault tolerance (45%) |

## Implementation Checklist

### Required Integrations
- [ ] Mycelix-Identity: Patient/provider verification
- [ ] Mycelix-Core: Bridge protocol registration
- [ ] MATL: Trust score federation

### Optional Integrations
- [ ] Mycelix-DeSci: Research publications
- [ ] Mycelix-Finance: Healthcare payments
- [ ] Mycelix-Justice: Dispute resolution
- [ ] Mycelix-Marketplace: Medical supplies
- [ ] Mycelix-EduNet: Provider education

### Testing Requirements
- [ ] Bridge protocol conformance tests
- [ ] Cross-hApp query/response tests
- [ ] Consent verification tests
- [ ] Trust aggregation tests
- [ ] Byzantine tolerance tests
