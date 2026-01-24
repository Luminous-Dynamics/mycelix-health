# Mycelix-Health 🏥

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
│   └── dna.yaml           # DNA manifest
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
│   │ # Revolutionary Features (Phase 2)
│   ├── advocate/          # AI Health Advocate system
│   ├── zkhealth/          # Zero-knowledge health proofs
│   ├── twin/              # Digital health twins
│   ├── dividends/         # Data dividend distribution
│   │
│   │ # Clinical Integration (Phase 3)
│   ├── fhir_mapping/      # FHIR R4 resource mapping
│   ├── cds/               # Clinical Decision Support
│   ├── provider_directory/ # Provider directory & NPI
│   ├── telehealth/        # Telehealth sessions
│   │
│   │ # Equity & Access (Phase 4)
│   ├── sdoh/              # Social Determinants of Health
│   ├── mental_health/     # Mental health pathways
│   ├── chronic_care/      # Chronic disease management
│   ├── pediatric/         # Pediatric lifecycle care
│   │
│   │ # Advanced Research (Phase 5)
│   ├── research_commons/  # De-identified data sharing
│   ├── trial_matching/    # Patient-trial matching
│   ├── irb/               # Decentralized IRB
│   ├── federated_learning/ # Privacy-preserving ML
│   ├── population_health/ # Population analytics
│   │
│   │ # Global Scale (Phase 6)
│   ├── ips/               # International Patient Summary
│   ├── i18n/              # Multi-language support
│   ├── disaster_response/ # Emergency healthcare operations
│   ├── verifiable_credentials/ # W3C VC for health credentials
│   └── mobile_support/    # Mobile sync & offline support
│
├── services/
│   └── ehr-gateway/       # EHR integration service (TypeScript)
│       ├── auth/          # SMART on FHIR OAuth2
│       ├── adapters/      # Epic, Cerner, generic FHIR
│       └── sync/          # Pull/push & conflict resolution
│
├── sdk/                   # TypeScript SDK (@mycelix/health-sdk)
├── docs/                  # Documentation
└── tests/                 # Comprehensive tests
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

### SDOH Screening Zome 🆕
Social determinants of health assessment and intervention:
- **Screening Instruments**: PRAPARE, AHC-HRSN, WeCare, custom tools
- **5 SDOH Domains**: Economic stability, education, healthcare, neighborhood, social/community
- **Risk Assessment**: No risk through urgent, with domain-specific scoring
- **Community Resources**: Searchable directory of local services
- **Intervention Tracking**: Referrals, follow-ups, outcome monitoring
- **Patient Summaries**: Aggregate view of social needs and support

### Mental Health Pathway Zome 🆕
Comprehensive mental health support with regulatory compliance:
- **Screening Instruments**: PHQ-9, PHQ-2, GAD-7, C-SSRS, AUDIT, DAST-10, PCL-5, MDQ, EPDS, PSC-17
- **Severity Tracking**: None through severe with crisis level monitoring
- **Safety Plans**: Warning signs, coping strategies, crisis contacts, reasons for living
- **42 CFR Part 2 Consent**: Substance abuse record protection with proper consent management
- **Treatment Planning**: Goals, medications, modalities, therapy notes
- **Crisis Management**: Event logging, emergency contacts, intervention tracking

### Chronic Care Zome 🆕
Specialized management for chronic conditions:
- **Condition Enrollment**: Diabetes, heart failure, COPD, CKD with type-specific data
- **Care Plans**: Goals, medications, self-management tasks, review scheduling
- **Condition Metrics**: HbA1c/glucose (diabetes), LVEF/BNP (heart failure), FEV1 (COPD), eGFR (CKD)
- **Medication Adherence**: Tracking with adherence rate calculations
- **Clinical Alerts**: Info through critical severity with acknowledgment
- **Exacerbation Events**: Track disease flare-ups with severity and triggers
- **Outcome Tracking**: Patient-reported outcomes for quality of life

### Pediatric Zome 🆕
Complete pediatric lifecycle management:
- **Growth Tracking**: Weight, height, head circumference with WHO/CDC percentile calculations
- **Immunizations**: Full CDC schedule, lot tracking, VIS documentation, catch-up scheduling
- **Developmental Milestones**: ASQ-3 based domains (motor, language, cognitive, social-emotional)
- **Well-Child Visits**: Comprehensive visit documentation with screening results
- **Pediatric Conditions**: Age-specific condition tracking with onset/resolution
- **School Health Records**: Sports physicals, accommodations, emergency contacts
- **Adolescent Health**: HEADSS assessment, reproductive health, mental health screening
- **Newborn Records**: Birth details, APGAR, feeding, circumcision, hearing screen

### Research Commons Zome 🆕
Privacy-preserving research data sharing:
- **De-identification Methods**: HIPAA Safe Harbor, Expert Determination, k-anonymity, differential privacy
- **Dataset Management**: Create, search, and manage de-identified research datasets
- **Access Agreements**: Formal agreements with approved uses and restrictions
- **Contribution Tracking**: Record and audit all data contributions
- **Data Quality**: Automated quality reports with completeness and consistency metrics
- **Usage Auditing**: Complete audit trail of all data access

### Trial Matching Zome 🆕
Intelligent patient-trial matching:
- **Eligibility Criteria**: Structured inclusion/exclusion criteria with comparison operators
- **Matching Profiles**: Patient data mapped to matchable attributes (diagnoses, labs, vitals)
- **Patient Preferences**: Travel distance, visit frequency, placebo acceptance, language preferences
- **Match Results**: Scored matches with criteria met/not met/indeterminate
- **Notifications**: Patient-controlled notifications for new matching opportunities
- **Provider Review**: Workflow for provider recommendation before patient contact

### Decentralized IRB Zome 🆕
Distributed ethics review for research:
- **Protocol Submissions**: Full protocol submission with risk assessment and consent documents
- **IRB Members**: Reviewer profiles with roles, credentials, and expertise
- **Review Workflow**: Individual reviews with votes, comments, required modifications
- **Meeting Management**: Quorum tracking, attendance, protocol agenda
- **Decision Recording**: Vote tallies, conditions, approval duration, chair signature
- **Continuing Review**: Annual progress reports with enrollment and adverse event tracking

### Federated Learning Zome 🆕
Privacy-preserving distributed machine learning:
- **Learning Projects**: Define tasks with model architecture and aggregation strategy
- **Participant Management**: Join projects with sample counts and public keys
- **Training Rounds**: Coordinate distributed training with learning parameters
- **Model Updates**: Encrypted gradient submissions with noise for differential privacy
- **Aggregation**: FedAvg, FedProx, Secure Aggregation, Byzantine-resilient options
- **Privacy Budgets**: Track epsilon consumption across training rounds
- **Model Evaluation**: Validation metrics with confusion matrices and confidence intervals

### Population Health Zome 🆕
Aggregate health analytics with privacy:
- **Population Statistics**: Prevalence, incidence, mortality with confidence intervals
- **Health Indicators**: Composite scores for geographic regions with benchmarks
- **Disease Surveillance**: Case counts vs expected with automatic anomaly detection
- **Public Health Alerts**: Severity-graded alerts with recommendations and acknowledgment
- **Disparity Analysis**: Stratified analysis by race, income, education with trend tracking
- **Quality Indicators**: HEDIS-style measures with star ratings and percentiles
- **Differential Privacy**: All statistics protected with configurable epsilon

### International Patient Summary (IPS) Zome 🆕
Cross-border healthcare data exchange following HL7 IPS standard:
- **IPS Document**: Complete patient summary with FHIR Bundle export
- **8 Clinical Sections**: Allergies, medications, problems, immunizations, procedures, medical devices, results, advance directives
- **Coding Systems**: SNOMED CT, RxNorm, LOINC, ICD-10, ATC, CVX
- **Cross-Border Sharing**: Track IPS shares with purpose and jurisdiction
- **Translation Support**: Automatic translation of summaries
- **Validation**: Ensure compliance with HL7 IPS implementation guide

### Internationalization (i18n) Zome 🆕
Multi-language support for global healthcare:
- **Locale Management**: BCP 47 tags with country/language/script variants
- **Translation System**: Source strings, translations, plural forms
- **Medical Terminology**: Domain-specific medical term translations
- **User Preferences**: Per-user locale settings with fallback chains
- **Translation Memory**: Fuzzy matching for translation reuse
- **Glossary**: Authoritative medical terminology with sources
- **RTL Support**: Right-to-left language rendering

### Disaster Response Zome 🆕
Emergency healthcare operations for mass casualty events:
- **Disaster Declaration**: Multi-level severity (1-5), affected area, emergency contacts
- **START Triage**: Immediate/Delayed/Minor/Expectant/Deceased categorization
- **Resource Management**: Personnel, supplies, equipment, beds, transport tracking
- **Patient Tracking**: Missing, displaced, evacuated, at shelter/facility status
- **Emergency Access**: Break-glass access with consent waiver and audit
- **Shelter Health**: Capacity, medical staff, special needs, health alerts
- **Resource Requests**: Priority-based resource request and fulfillment

### Verifiable Credentials Zome 🆕
W3C Verifiable Credentials standard for health credentials:
- **15+ Credential Types**: Vaccination, lab results, medical license, insurance, patient identity
- **W3C VC Data Model**: JSON-LD context, proof types, credential status
- **Issuer Management**: Trusted issuer registry with key verification
- **Holder Wallet**: Personal credential storage with favorites and categories
- **Presentations**: Selective disclosure, presentation requests, verifier flow
- **Revocation**: Batch revocation registries, status checks
- **Trust Registry**: Hierarchical trust chains for issuer verification
- **Health-Specific Claims**: Vaccination records, lab results, medical licenses

### Mobile Support Zome 🆕
Mobile-optimized healthcare operations:
- **Device Management**: Multi-device registration, platform-specific handling
- **Offline Sync**: Checkpoint-based resumable sync with priority queuing
- **Conflict Resolution**: Server/client wins, last-write-wins, manual merge
- **Delta Sync**: Incremental changes for bandwidth optimization
- **Push Notifications**: FCM/APNS support with quiet hours and type filtering
- **QR Codes**: Device pairing, record sharing, emergency access
- **Biometric Auth**: Fingerprint/face authentication integration
- **Emergency Snapshot**: Offline-available critical health info (allergies, medications, emergency contacts)
- **Bandwidth Tracking**: Usage monitoring for data-constrained environments

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

### Phase 3: Clinical Integration ✅
- ✅ **FHIR R4 Mapping**: Patient bundles, observations, conditions, medications with LOINC/SNOMED/ICD-10 terminology
- ✅ **EHR Gateway**: SMART on FHIR authentication, Epic/Cerner adapters, bidirectional sync
- ✅ **Clinical Decision Support**: Drug interactions (RxNorm), allergy checks, clinical alerts, guidelines
- ✅ **Provider Directory**: NPI verification, provider search, affiliations, accepting patients
- ✅ **Telehealth**: Session scheduling, waiting room, documentation, provider availability

### Phase 4: Equity & Access ✅
- ✅ **SDOH Screening**: PRAPARE/AHC-HRSN instruments, 5 domains, community resources, intervention tracking
- ✅ **Mental Health Pathway**: PHQ-9/GAD-7/C-SSRS screenings, safety plans, 42 CFR Part 2 consent
- ✅ **Chronic Disease Management**: Diabetes, heart failure, COPD, CKD with condition-specific metrics
- ✅ **Pediatric Lifecycle**: Growth tracking, CDC immunizations, developmental milestones, well-child visits
- ✅ **Accessibility (WCAG 2.1)**: ARIA labels, screen reader support, contrast checking, reading level adaptation

### Phase 5: Advanced Research ✅
- ✅ **Research Commons**: De-identified data sharing with HIPAA Safe Harbor/Expert Determination, k-anonymity, differential privacy
- ✅ **Trial Matching**: Eligibility criteria matching, patient preferences, automatic notifications
- ✅ **Decentralized IRB**: Protocol submissions, reviewer management, voting, continuing reviews
- ✅ **Federated Learning**: Privacy-preserving ML with FedAvg/SecureAggregation, differential privacy budgets
- ✅ **Population Health**: Aggregate statistics, surveillance reports, public health alerts, disparity analyses

### Phase 6: Global Scale ✅
- ✅ **International Patient Summary (IPS)**: HL7 IPS standard with FHIR Bundle export, 8 clinical sections, cross-border sharing
- ✅ **Internationalization (i18n)**: BCP 47 locales, translation memory, medical terminology, RTL support
- ✅ **Disaster Response**: Multi-severity declarations, START triage, resource management, emergency access
- ✅ **Verifiable Credentials**: W3C VC standard with 15+ credential types, issuer/holder/verifier workflows, revocation
- ✅ **Mobile Support**: Offline sync, conflict resolution, push notifications, QR codes, emergency snapshots

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
