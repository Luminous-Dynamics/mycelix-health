# Mycelix-Health 🏥

[![CI](https://github.com/Luminous-Dynamics/mycelix-health/actions/workflows/ci.yml/badge.svg)](https://github.com/Luminous-Dynamics/mycelix-health/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![SDK](https://img.shields.io/badge/SDK-TypeScript-3178C6)](sdk/)

**Decentralized Healthcare Infrastructure for Patient-Controlled Medical Records**

Mycelix-Health is a Holochain-based healthcare application that puts patients in control of their medical data while enabling seamless, privacy-preserving sharing with healthcare providers, researchers, and insurers.

## Vision

Healthcare data belongs to patients, not institutions. Mycelix-Health enables:

- **Patient Sovereignty**: You own and control your complete medical history
- **Selective Sharing**: Granular consent for who sees what data
- **Provider Verification**: MATL trust scoring for healthcare providers
- **Clinical Trial Integration**: Connect patients with research opportunities
- **Cross-hApp Federation**: Share reputation across the Mycelix ecosystem
- **AI Health Advocacy**: Your personal AI that knows your health history and advocates for you
- **Zero-Knowledge Health Proofs**: Prove health status without revealing private data
- **Digital Health Twins**: Personalized physiological models for "what if" scenarios
- **Data Dividends**: Share in the value created when your data advances medicine

## Features

### Core Capabilities

| Feature | Description |
|---------|-------------|
| **Patient Profiles** | Demographics, allergies, medications, conditions |
| **Provider Management** | Credentials, licenses, certifications with verification |
| **Medical Records** | Encounters, diagnoses, procedures, lab results, imaging |
| **Prescriptions** | E-prescribing, refills, drug interactions, adherence |
| **Consent Management** | Granular, time-bound, revocable consent directives |
| **Clinical Trials** | Study enrollment, data collection, adverse events |
| **Insurance** | Plans, claims, prior authorizations, EOBs |
| **Mycelix Bridge** | Cross-hApp data federation and reputation |

### Revolutionary Features (Phase 2) 🚀

| Feature | Description |
|---------|-------------|
| **AI Health Advocate** | Personal AI that knows your health history, prepares you for appointments, tracks provider outcomes, and advocates on your behalf |
| **ZK Health Proofs** | Zero-knowledge proofs to verify health status (vaccinations, drug tests, physicals) without revealing underlying health data |
| **Health Twin** | Digital physiological model of your body for predictions, simulations, and "what if" scenarios |
| **Data Dividends** | When your data contributes to drug discoveries or AI models, you receive fair compensation through transparent attribution chains |

### Compliance & Standards

- **HIPAA Alignment**: Privacy, security, and audit requirements
- **HL7 FHIR**: Standard medical terminology and data models
- **FDA 21 CFR Part 11**: Clinical trial electronic records
- **ICD-10/CPT/LOINC**: Standard medical coding systems
- **RxNorm/NDC**: Medication identification standards

## Architecture

```
mycelix-health/
├── dna/
│   └── dna.yaml           # DNA manifest with 12 zomes
├── happ.yaml              # hApp manifest
├── zomes/
│   ├── patient/           # Patient identity & demographics
│   │   ├── integrity/     # Entry types & validation
│   │   └── coordinator/   # Extern functions
│   ├── provider/          # Provider credentials & licensing
│   ├── records/           # Medical records & results
│   ├── prescriptions/     # Medication management
│   ├── consent/           # Access authorization
│   ├── trials/            # Clinical research
│   ├── insurance/         # Claims & coverage
│   ├── bridge/            # Mycelix federation
│   │
│   │ # Revolutionary Features
│   ├── advocate/          # AI Health Advocate system
│   ├── zkhealth/          # Zero-knowledge health proofs
│   ├── twin/              # Digital health twins
│   └── dividends/         # Data dividend distribution
├── docs/                  # Documentation
└── tests/                 # 441 comprehensive tests
```

## Quick Start

### Prerequisites

- NixOS or Linux with Nix
- Holochain 0.4.x
- HDK 0.6 / HDI 0.7

### Build

```bash
# Enter development environment
nix develop

# Build all zomes
cargo build --release --target wasm32-unknown-unknown

# Package DNA
hc dna pack dna/

# Package hApp
hc app pack .
```

### Development

```bash
# Run tests
cargo test

# Run conductor tests
hc sandbox generate --num-sandboxes 2
hc sandbox run 0
```

## TypeScript SDK

The `@mycelix/health-sdk` provides type-safe TypeScript bindings for building frontend applications.

### Installation

```bash
npm install @mycelix/health-sdk
# or
yarn add @mycelix/health-sdk
```

### Quick Example

```typescript
import { PatientClient, PrivacyBudgetManager, HealthSdkError } from '@mycelix/health-sdk';

// Create a patient
const patient = await client.patient.createPatient({
  name: 'Jane Doe',
  date_of_birth: new Date('1985-03-15'),
  gender: 'female',
  blood_type: 'A+',
});

// Manage differential privacy budgets
const budgetManager = new PrivacyBudgetManager({
  total_epsilon: 10.0,
  consumed_epsilon: 0.0,
  query_count: 0,
});

// Check if query is safe
if (budgetManager.canQuery(1.0)) {
  const stats = await client.commons.queryPoolStats(poolHash, 'mean', 1.0);
}
```

### Key Features

- **Full Type Safety**: Complete TypeScript definitions for all zome calls
- **Privacy Budget Management**: Client-side differential privacy helpers
- **Error Handling**: Custom `HealthSdkError` with typed error codes
- **Holochain Integration**: Works with `@holochain/client`

See [`sdk/README.md`](sdk/README.md) for complete documentation.

## Zome Overview

### Patient Zome
Manages patient profiles with:
- Demographics and contact information
- Allergies with severity tracking
- Medical conditions and medications
- Mycelix identity linking
- MATL trust scoring

### Provider Zome
Handles healthcare provider credentialing:
- NPI and DEA registration
- Medical licenses with expiration tracking
- Board certifications
- Practice locations
- Epistemic classification for claims

### Records Zome
Comprehensive medical records:
- Encounters with diagnoses and procedures
- Lab results with LOINC codes
- Imaging studies with DICOM links
- Vital signs with range validation
- Critical result alerting

### Prescriptions Zome
Full prescription lifecycle:
- E-prescribing with RxNorm
- Controlled substance tracking
- Pharmacy management
- Drug interaction alerts
- Medication adherence

### Consent Zome
Granular access control:
- Data category permissions (14 HIPAA categories)
- Time-bound consents with automatic expiration
- Emergency access (break-glass) with mandatory audit
- Comprehensive audit logging
- HIPAA authorization documents

**NEW: Consent Delegation System**
- Healthcare proxy with legal documentation
- Caregiver access for family members
- Temporary delegation (travel, recovery)
- Legal guardian support for minors
- Sub-delegation capabilities

**NEW: Patient Notification System**
- Real-time alerts for data access
- Plain-language summaries ("Dr. Smith viewed your medications")
- Configurable priorities (immediate, daily, weekly, silent)
- Trusted provider silent lists
- Email, push, and SMS delivery options

**NEW: Care Team Templates**
- One-click consent for common scenarios
- 8 pre-built system templates:
  - Primary Care Team
  - Specialist Referral
  - Hospital Admission
  - Emergency Department
  - Mental Health Provider
  - Pharmacy Access
  - Insurance & Billing
  - Telehealth Visit
- Custom organization templates
- Automatic expiration management

### Trials Zome
Clinical research support:
- Trial registration (NCT numbers)
- Participant enrollment
- Visit scheduling and data collection
- Protocol deviation tracking
- Adverse event reporting (MedWatch)

### Insurance Zome
Claims and coverage:
- Plan management with coordination
- Claims submission and tracking
- Prior authorization workflows
- Eligibility verification
- Explanation of Benefits

### Bridge Zome
Mycelix ecosystem integration:
- Cross-hApp data queries
- Provider verification requests
- Epistemic claim federation
- Reputation aggregation
- Trust score integration

### Commons Zome 🔒
Privacy-preserving health data commons with **formal Differential Privacy**:
- **Data Pools**: Create themed data pools (research, public health, etc.)
- **Patient Contributions**: Contribute health metrics with mathematically guaranteed privacy
- **DP Queries**: Aggregate statistics (mean, count, sum) with Laplace/Gaussian noise
- **Budget Tracking**: Per-patient epsilon budgets with automatic exhaustion protection
- **Cryptographic RNG**: `getrandom`-based randomness for provable security
- **Composition Theorems**: Both basic and advanced composition for tight bounds

**Privacy Guarantees**:
| Mechanism | Privacy | Use Case |
|-----------|---------|----------|
| Laplace | (ε, 0)-DP | Count, Sum queries |
| Gaussian | (ε, δ)-DP | Mean queries with tighter bounds |

### AI Health Advocate Zome 🆕
Your personal health ally:
- **Appointment Preparation**: AI analyzes your history, suggests questions, identifies medications to discuss
- **Health Insights**: Pattern recognition across your data with evidence-based recommendations
- **Provider Reviews**: Track outcomes with specific providers, see aggregate ratings
- **Health Alerts**: Drug interaction warnings, abnormal results, preventive care reminders
- **Second Opinions**: Facilitate getting additional expert perspectives
- **Medication Checks**: Drug-drug interactions, duplicate therapy detection, contraindications

### ZK Health Proofs Zome 🆕
Privacy-preserving health verification using zkSTARK proofs:
- **15 Proof Types**: Vaccination status, drug screen clear, employment physical, BMI range, etc.
- **Zero Knowledge**: Prove claims without revealing underlying health data
- **Post-Quantum Security**: 128+ bit security with Kyber-1024 compatible proofs
- **Trusted Attestors**: Verified healthcare providers attest to health facts
- **Verifiable Credentials**: Employers, schools, insurers can verify without seeing PHI
- **Selective Disclosure**: Prove exactly what's needed, nothing more

### Health Twin Zome 🆕
Digital physiological models for personalized medicine:
- **Physiological Modeling**: Cardiovascular, metabolic, respiratory, renal systems
- **Continuous Learning**: Integrates data from wearables, labs, visits
- **Simulations**: "What if I exercise 30 min daily?" with projected outcomes
- **Predictions**: Risk assessments, metric forecasts with confidence intervals
- **Trajectories**: Track trends over time with projections
- **Model Confidence**: Always shows uncertainty - never overconfident

### Data Dividends Zome 🆕
Fair compensation when your data creates value:
- **Contribution Tracking**: Detailed records of what data you share
- **Usage Monitoring**: See exactly how your data is used
- **Attribution Chains**: Immutable lineage from contribution to discovery
- **Revenue Events**: When research produces value, patients share
- **Fair Calculation**: Weighted by data quality, quantity, uniqueness
- **Patient Control**: Specify permitted uses, exclude prohibited purposes
- **Transparency**: Full visibility into dividend calculations

## Ecosystem Integration

Mycelix-Health integrates with other Mycelix hApps:

| hApp | Integration |
|------|-------------|
| **Mycelix-Identity** | Patient and provider identity verification |
| **Mycelix-DeSci** | Research publication linking |
| **Mycelix-Finance** | Healthcare payment processing |
| **Mycelix-Justice** | Medical malpractice dispute resolution |
| **Mycelix-Marketplace** | Medical equipment and supplies |

## Trust & Reputation

Mycelix-Health uses the Multi-Agent Trust Layer (MATL) for:

- **Provider Trust**: Based on outcomes, feedback, credentials
- **Data Quality**: Epistemic classification (E0-E3)
- **Claims Verification**: Multi-party attestation
- **Byzantine Fault Tolerance**: Up to 45% malicious actors

### Epistemic Levels

| Level | Description | Example |
|-------|-------------|---------|
| E0 | Patient-reported | "I feel dizzy" |
| E1 | Provider observation | Physical exam finding |
| E2 | Test confirmed | Lab result |
| E3 | Multi-provider consensus | Tumor board decision |

## Security

- **Agent-centric**: Data stored on patient's device
- **End-to-end encryption**: For sensitive data at rest
- **Audit trails**: Immutable access logs
- **Break-glass**: Emergency access with mandatory review
- **Consent enforcement**: Cryptographic access control

### Field-Level Encryption
Sensitive PHI fields are encrypted individually:
- SSN and financial data
- Mental health and substance abuse notes
- Sexual health and genetic data
- Biometric identifiers

### Key Management
- Secure key generation and derivation
- Key wrapping for storage
- Automatic rotation policies
- Multi-version support for backwards compatibility

### Access Control Enforcement
All data access requires:
1. Patient self-access verification, OR
2. Valid active consent with matching scope, OR
3. Emergency override with mandatory justification
4. Automatic audit logging for all access attempts

## Roadmap

### Phase 1: Foundation ✅
- ✅ Core zome implementation (8 zomes)
- ✅ Basic patient/provider workflows
- ✅ Consent management
- ✅ Access control enforcement
- ✅ Field-level encryption
- ✅ Key management
- ✅ 292 comprehensive tests

### Phase 1.5: Patient Experience ✅
- ✅ Consent delegation system
- ✅ Patient notification system
- ✅ Care team templates
- 🚧 SDK integration

### Phase 2: Revolutionary Features ✅
- ✅ **AI Health Advocate**: Personal AI for appointment prep, insights, alerts
- ✅ **ZK Health Proofs**: Zero-knowledge proofs with 15 proof types
- ✅ **Health Twin MVP**: Digital physiological modeling and simulations
- ✅ **Data Dividends**: Fair compensation system with attribution chains
- ✅ 441 comprehensive tests (149 new for revolutionary features)

### Phase 3: Clinical Integration
- 📋 Full FHIR R4 compatibility
- 📋 EHR integration APIs
- 📋 Clinical decision support
- 📋 Provider directory
- 📋 Telehealth support

### Phase 4: Equity & Access
- 📋 SDOH screening integration
- 📋 Mental health pathway
- 📋 Chronic disease modules
- 📋 Pediatric lifecycle management
- 📋 Accessibility features

### Phase 5: Advanced Research
- 📋 De-identified data commons
- 📋 Clinical trial matching
- 📋 Decentralized IRB support
- 📋 Federated learning
- 📋 Population health analytics

### Phase 6: Global Scale
- 📋 International standards (IPS)
- 📋 Multi-language expansion
- 📋 Disaster response mode
- 📋 Verifiable credentials
- 📋 Mobile applications

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Apache 2.0 - See [LICENSE](LICENSE)

## Contact

- **Website**: [mycelix.net](https://mycelix.net)
- **Email**: health@mycelix.net
- **GitHub**: [Luminous-Dynamics/mycelix-health](https://github.com/Luminous-Dynamics/mycelix-health)

---

*"Returning healthcare data sovereignty to patients through decentralized trust."*
