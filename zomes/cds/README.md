# CDS (Clinical Decision Support) Zome

> Real-time clinical decision support including drug interactions, allergy checks, clinical alerts, and guideline compliance tracking.

## Overview

The CDS zome provides comprehensive clinical decision support functionality including drug-drug interaction checking, drug-allergy conflict detection, clinical alert management, evidence-based guideline compliance tracking, and pharmacogenomics profiling.

## Architecture

```
cds/
├── integrity/           # Entry type definitions & validation
│   └── src/
│       └── lib.rs       # DrugInteraction, ClinicalAlert, etc.
└── coordinator/         # Business logic & extern functions
    └── src/
        └── lib.rs       # Interaction checks, alerts, guidelines
```

## Entry Types

| Entry Type | Description | Links |
|------------|-------------|-------|
| `DrugInteraction` | Drug-drug interaction record | `DrugToInteractions`, `AllDrugInteractions` |
| `DrugAllergyInteraction` | Drug-allergy conflict | `DrugToAllergyInteractions` |
| `ClinicalAlert` | Patient-specific clinical alert | `PatientToAlerts`, `UnacknowledgedAlerts` |
| `ClinicalGuideline` | Evidence-based clinical guideline | `ActiveGuidelines`, `ConditionToGuidelines` |
| `PatientGuidelineStatus` | Patient's compliance with guideline | `PatientToGuidelineStatuses` |
| `InteractionCheckResult` | Recorded interaction check result | `PatientToInteractionChecks` |
| `PgxProfile` | Pharmacogenomics profile | `PatientToPgxProfile` |
| `PgxRecommendation` | Gene-drug recommendation | `GeneToDrugRecommendations` |

## Extern Functions

### Drug Interactions

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `create_drug_interaction` | `DrugInteraction` | `Record` | Create interaction record |
| `check_drug_interactions` | `CheckDrugInteractionsInput` | `Vec<FoundInteraction>` | Check for interactions |

### Allergy Conflicts

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `create_drug_allergy_interaction` | `DrugAllergyInteraction` | `Record` | Create allergy conflict |
| `check_allergy_conflicts` | `CheckAllergyConflictsInput` | `Vec<FoundAllergyConflict>` | Check allergy conflicts |

### Clinical Alerts

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `create_clinical_alert` | `CreateAlertInput` | `Record` | Create patient alert |
| `get_patient_alerts` | `GetPatientAlertsInput` | `Vec<Record>` | Get patient alerts |
| `acknowledge_alert` | `AcknowledgeAlertInput` | `Record` | Acknowledge an alert |
| `resolve_alert` | `ResolveAlertInput` | `Record` | Resolve an alert |

### Clinical Guidelines

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `create_clinical_guideline` | `ClinicalGuideline` | `Record` | Create guideline |
| `get_all_active_guidelines` | `()` | `Vec<Record>` | Get all active guidelines |
| `get_guidelines_for_condition` | `GetGuidelinesForConditionInput` | `Vec<Record>` | Get guidelines for condition |
| `update_patient_guideline_status` | `PatientGuidelineStatus` | `Record` | Update compliance status |
| `get_patient_guideline_statuses` | `GetPatientGuidelineStatusInput` | `Vec<Record>` | Get patient statuses |

### Interaction Checking

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `perform_interaction_check` | `InteractionCheckRequest` | `Record` | Full interaction check |

### Pharmacogenomics

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `create_pgx_profile` | `CreatePgxProfileInput` | `Record` | Create PGx profile |
| `get_pgx_recommendations` | `GetPgxRecommendationsInput` | `Vec<Record>` | Get drug recommendations |

## Core Types

### DrugInteraction

```rust
pub struct DrugInteraction {
    pub drug_a_rxnorm: String,
    pub drug_a_name: String,
    pub drug_b_rxnorm: String,
    pub drug_b_name: String,
    pub severity: InteractionSeverity,
    pub description: String,
    pub mechanism: Option<String>,
    pub clinical_significance: Option<String>,
    pub management: Option<String>,
    pub evidence_level: EvidenceLevel,
    pub source: String,
}

pub enum InteractionSeverity {
    Contraindicated,  // Never combine
    Major,            // Serious - use alternative if possible
    Moderate,         // Monitor closely
    Minor,            // Usually safe, aware of potential
}

pub enum EvidenceLevel {
    Established,      // Well-documented
    Probable,         // Likely based on studies
    Suspected,        // Case reports suggest
    Theoretical,      // Pharmacological basis
}
```

### ClinicalAlert

```rust
pub struct ClinicalAlert {
    pub patient_hash: ActionHash,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub triggered_by: Option<String>,
    pub recommendations: Vec<String>,
    pub status: AlertStatus,
    pub acknowledged_by: Option<AgentPubKey>,
    pub acknowledged_at: Option<Timestamp>,
    pub resolved_at: Option<Timestamp>,
    pub resolution_notes: Option<String>,
}

pub enum AlertType {
    DrugInteraction,
    AllergyConflict,
    DuplicateTherapy,
    ContraindicatedCondition,
    DoseWarning,
    GuidelineDeviation,
    LabValueCritical,
    PgxWarning,
}
```

### PgxProfile

```rust
pub struct PgxProfile {
    pub patient_hash: ActionHash,
    pub gene_results: Vec<GeneResult>,
    pub testing_date: Timestamp,
    pub lab_source: String,
}

pub struct GeneResult {
    pub gene: String,           // e.g., "CYP2D6"
    pub diplotype: String,      // e.g., "*1/*4"
    pub phenotype: String,      // e.g., "Intermediate Metabolizer"
    pub activity_score: Option<f32>,
}
```

## Usage Examples

### SDK (TypeScript)

```typescript
import { CdsClient } from '@mycelix/health-sdk';

const client = new CdsClient(appClient, 'health');

// Check drug interactions
const interactions = await client.checkDrugInteractions({
  rxnorm_codes: ['197361', '310965', '856917'],  // Metformin, Lisinopril, Warfarin
});

for (const interaction of interactions) {
  console.log(`${interaction.drug_a_name} + ${interaction.drug_b_name}`);
  console.log(`Severity: ${interaction.severity}`);
  console.log(`Management: ${interaction.management}`);
}

// Check allergy conflicts
const allergyConflicts = await client.checkAllergyConflicts({
  patient_hash: patientHash,
  medication_rxnorm_codes: ['856917'],  // Warfarin
});

// Get patient alerts
const alerts = await client.getPatientAlerts({
  patient_hash: patientHash,
  include_resolved: false,
});

// Acknowledge alert
await client.acknowledgeAlert({
  alert_hash: alertHash,
  acknowledger_notes: 'Reviewed with physician',
});

// Check pharmacogenomics recommendations
const pgxRecs = await client.getPgxRecommendations({
  patient_hash: patientHash,
  medication_rxnorm: '856917',
});
```

### Prescription Workflow Integration

```typescript
// When prescribing a new medication
async function checkPrescriptionSafety(
  patientHash: ActionHash,
  newMedicationRxnorm: string,
  currentMedications: string[]
) {
  // 1. Check drug interactions
  const allMeds = [...currentMedications, newMedicationRxnorm];
  const interactions = await client.checkDrugInteractions({
    rxnorm_codes: allMeds,
  });

  const severeInteractions = interactions.filter(
    i => i.severity === 'Contraindicated' || i.severity === 'Major'
  );

  if (severeInteractions.length > 0) {
    // Create alert
    await client.createClinicalAlert({
      patient_hash: patientHash,
      alert_type: 'DrugInteraction',
      severity: 'High',
      title: `Drug interaction detected with ${newMedicationRxnorm}`,
      description: severeInteractions.map(i => i.description).join('; '),
      recommendations: severeInteractions.map(i => i.management),
    });
  }

  // 2. Check allergy conflicts
  const allergyConflicts = await client.checkAllergyConflicts({
    patient_hash: patientHash,
    medication_rxnorm_codes: [newMedicationRxnorm],
  });

  // 3. Check PGx recommendations
  const pgxRecs = await client.getPgxRecommendations({
    patient_hash: patientHash,
    medication_rxnorm: newMedicationRxnorm,
  });

  return {
    interactions,
    allergyConflicts,
    pgxRecommendations: pgxRecs,
    isSafe: severeInteractions.length === 0 && allergyConflicts.length === 0,
  };
}
```

## Workflow Diagrams

### Interaction Check Flow

```
┌──────────────┐     ┌─────────────┐     ┌──────────────┐
│  New Rx      │────▶│  CDS Zome   │────▶│  Drug DB     │
│  Request     │     │             │     │  Lookups     │
└──────────────┘     │ check_drug_ │     └──────────────┘
                     │ interactions│
                     │             │     ┌──────────────┐
                     │             │────▶│  Allergy     │
                     │             │     │  Checks      │
                     └─────────────┘     └──────────────┘
                            │
                            ▼
                     ┌─────────────┐
                     │  Results    │
                     │  + Alerts   │
                     └─────────────┘
```

## Dependencies

### Internal Dependencies

- `mycelix-health-shared`: Authorization, logging, anchors

### External Dependencies

- `hdi ^0.7.0`: Holochain Deterministic Integrity
- `hdk ^0.6.0`: Holochain Development Kit

## Data Sources

The CDS zome can be populated with interaction data from:

- **RxNorm**: Drug identifiers and relationships
- **DrugBank**: Drug interaction database
- **CPIC**: Clinical Pharmacogenetics Implementation Consortium
- **FDA Drug Labels**: Official drug information

## Privacy & Consent

All CDS operations respect consent:

- `require_authorization()` called before accessing patient data
- `log_data_access()` records all data access
- PGx data requires explicit `GeneticData` consent

## Testing

### Unit Tests

```bash
cargo test -p cds_integrity --target x86_64-unknown-linux-gnu
cargo test -p cds_coordinator --target x86_64-unknown-linux-gnu
```

### SDK Tests

```bash
cd sdk
npm test -- tests/cds-client.test.ts
```

## Changelog

### v0.1.0 (2026-01)

- Initial implementation
- Drug-drug interaction checking
- Drug-allergy conflict detection
- Clinical alert management
- Guideline compliance tracking
- Pharmacogenomics profiling
- CDS Hooks integration support

## Related Zomes

- `patient`: Patient records and medications
- `consent`: Authorization for CDS data access
- `fhir_bridge`: FHIR CDS Hooks integration
- `mental_health`: Mental health screening triggers
- `hdc_genetics`: Genetic variant data for PGx
