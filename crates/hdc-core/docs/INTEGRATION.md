# HDC-Core Integration Guide

This guide explains how to integrate HDC-Core with Holochain zomes, healthcare systems, and the broader Mycelix-Health ecosystem.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Holochain Integration](#holochain-integration)
- [FHIR Bridge Integration](#fhir-bridge-integration)
- [Health DNA Zome Integration](#health-dna-zome-integration)
- [TypeScript SDK Integration](#typescript-sdk-integration)
- [EHR Gateway Integration](#ehr-gateway-integration)
- [Privacy Considerations](#privacy-considerations)
- [Performance Optimization](#performance-optimization)
- [Deployment Patterns](#deployment-patterns)

---

## Architecture Overview

HDC-Core sits at the intersection of genetic data and the Holochain DHT:

```
┌─────────────────────────────────────────────────────────────┐
│                     Mycelix-Health Stack                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────┐ │
│  │ EHR Gateway │────│ FHIR Bridge │────│ Health DNA      │ │
│  │ (TypeScript)│    │   (Zome)    │    │ (41 Zomes)      │ │
│  └─────────────┘    └─────────────┘    └─────────────────┘ │
│         │                 │                    │           │
│         │                 ▼                    ▼           │
│         │          ┌─────────────┐    ┌─────────────────┐ │
│         │          │ hdc_genetics│    │ twin, consent,  │ │
│         │          │   (Zome)    │    │ records, etc.   │ │
│         │          └─────────────┘    └─────────────────┘ │
│         │                 │                               │
│         │                 ▼                               │
│         │          ┌─────────────┐                        │
│         └─────────▶│  HDC-Core   │◀───────────────────────┘
│                    │   (Rust)    │                         │
│                    └─────────────┘                         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Ingest**: FHIR Bundle → fhir_bridge zome → native entries
2. **Encode**: Genetic data → hdc_genetics zome → HDC-Core → hypervectors
3. **Query**: Client → zome call → HDC similarity search → results
4. **Export**: Holochain entries → FHIR Bundle → EHR

---

## Holochain Integration

### Adding HDC-Core to a Zome

In your zome's `Cargo.toml`:

```toml
[dependencies]
hdc-core = { path = "../../../crates/hdc-core", default-features = false, features = ["wasm"] }
```

**Important**: Use the `wasm` feature for Holochain zomes. This disables:
- External RNG (uses deterministic generation)
- GPU acceleration
- Parallel processing

### Storing Hypervectors

Hypervectors serialize to 1,250 bytes. Store efficiently:

```rust
use hdc_core::{Hypervector, Seed, HYPERVECTOR_BYTES};
use hdk::prelude::*;

#[hdk_entry_helper]
#[derive(Clone)]
pub struct GeneticVector {
    /// Compressed hypervector (1,250 bytes)
    pub vector_data: Vec<u8>,

    /// Source identifier
    pub source_id: String,

    /// Encoding metadata
    pub encoding_type: String,
    pub kmer_length: u8,

    /// Creation timestamp
    pub created_at: Timestamp,
}

impl GeneticVector {
    pub fn from_hypervector(hv: &Hypervector, source_id: String, encoding_type: String, k: u8) -> Self {
        Self {
            vector_data: hv.as_bytes().to_vec(),
            source_id,
            encoding_type,
            kmer_length: k,
            created_at: sys_time().unwrap(),
        }
    }

    pub fn to_hypervector(&self) -> ExternResult<Hypervector> {
        Hypervector::from_bytes(self.vector_data.clone())
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))
    }
}
```

### Zome Function Pattern

```rust
use hdk::prelude::*;
use hdc_core::{DnaEncoder, SnpEncoder, Seed};

#[hdk_extern]
pub fn encode_genetic_sequence(input: EncodeSequenceInput) -> ExternResult<ActionHash> {
    // Create deterministic seed from agent + purpose
    let agent = agent_info()?.agent_initial_pubkey;
    let seed_material = format!("{}-sequence-encoding-v1", agent);
    let seed = Seed::from_string(&seed_material);

    // Encode with HDC
    let encoder = DnaEncoder::new(seed, input.kmer_length.unwrap_or(6));
    let encoded = encoder.encode_sequence(&input.sequence)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?;

    // Store the vector
    let entry = GeneticVector::from_hypervector(
        &encoded.vector,
        input.source_id,
        "dna_sequence".to_string(),
        input.kmer_length.unwrap_or(6),
    );

    create_entry(EntryTypes::GeneticVector(entry))
}

#[hdk_extern]
pub fn find_similar_genetics(input: SimilaritySearchInput) -> ExternResult<Vec<SimilarityResult>> {
    // Load query vector
    let query_record = get(input.query_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Query not found".into())))?;
    let query_entry: GeneticVector = query_record.entry().to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid entry".into())))?;
    let query_vector = query_entry.to_hypervector()?;

    // Get all vectors via link (assumes indexed)
    let links = get_links(
        GetLinksInputBuilder::try_new(input.index_anchor, LinkTypes::GeneticVectorIndex)?.build()
    )?;

    // Compute similarities
    let mut results: Vec<SimilarityResult> = links.iter()
        .filter_map(|link| {
            let target_hash = link.target.clone().into_action_hash()?;
            let record = get(target_hash.clone(), GetOptions::default()).ok()??;
            let entry: GeneticVector = record.entry().to_app_option().ok()??;
            let vector = entry.to_hypervector().ok()?;

            let similarity = query_vector.normalized_cosine_similarity(&vector);
            if similarity >= input.threshold {
                Some(SimilarityResult {
                    action_hash: target_hash,
                    similarity,
                    source_id: entry.source_id,
                })
            } else {
                None
            }
        })
        .collect();

    // Sort by similarity descending
    results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
    results.truncate(input.limit);

    Ok(results)
}
```

---

## FHIR Bridge Integration

The `fhir_bridge` zome provides the gateway between FHIR R4 resources and Holochain entries.

### Bundle Ingestion Flow

```rust
// In fhir_bridge coordinator

#[hdk_extern]
pub fn ingest_bundle(input: IngestBundleInput) -> ExternResult<IngestReport> {
    let mut report = IngestReport::new(&input.source_system);

    for entry in input.bundle.entry.iter().flatten() {
        match entry.resource.get("resourceType").and_then(|v| v.as_str()) {
            Some("Patient") => {
                // Parse and create patient entry
                let patient = parse_fhir_patient(&entry.resource)?;

                // Check for genetic observations to encode
                if let Some(genetic_data) = extract_genetic_observations(&patient) {
                    encode_patient_genetics(&patient, &genetic_data)?;
                }

                report.patients_created += 1;
            }
            Some("Observation") => {
                // Check if it's a genetic observation
                if is_genetic_observation(&entry.resource) {
                    handle_genetic_observation(&entry.resource, &mut report)?;
                } else {
                    handle_clinical_observation(&entry.resource, &mut report)?;
                }
            }
            // ... other resource types
            _ => {
                report.unknown_types.push(entry.resource.get("resourceType")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string());
            }
        }
    }

    Ok(report)
}
```

### Genetic Observation Handling

```rust
fn handle_genetic_observation(
    resource: &serde_json::Value,
    report: &mut IngestReport,
) -> ExternResult<()> {
    // Extract genetic data from FHIR Observation
    let code = resource.get("code")
        .and_then(|c| c.get("coding"))
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("code"))
        .and_then(|v| v.as_str());

    match code {
        Some("69548-6") => {
            // LOINC: Genetic variant assessment
            let sequence = extract_sequence_from_observation(resource)?;

            // Call hdc_genetics zome
            call(
                CallTargetCell::Local,
                ZomeName::from("hdc_genetics"),
                FunctionName::from("encode_genetic_observation"),
                None,
                EncodeObservationInput {
                    observation: resource.clone(),
                    sequence,
                },
            )?;

            report.observations_created += 1;
        }
        Some("51963-7") => {
            // LOINC: Medication-related genetic testing
            let diplotypes = extract_pgx_diplotypes(resource)?;

            call(
                CallTargetCell::Local,
                ZomeName::from("hdc_genetics"),
                FunctionName::from("encode_pgx_profile"),
                None,
                EncodePgxInput { diplotypes },
            )?;

            report.observations_created += 1;
        }
        _ => {
            report.observations_skipped += 1;
        }
    }

    Ok(())
}
```

---

## Health DNA Zome Integration

### Cross-Zome Calls

The 41 health DNA zomes can all interact with genetic data:

```rust
// From twin zome - get genetic risk score

#[hdk_extern]
pub fn calculate_genetic_risk(input: RiskCalculationInput) -> ExternResult<GeneticRiskScore> {
    // Get patient's encoded genetics
    let genetic_vectors: Vec<GeneticVector> = call(
        CallTargetCell::Local,
        ZomeName::from("hdc_genetics"),
        FunctionName::from("get_patient_vectors"),
        None,
        input.patient_hash.clone(),
    )?;

    // Get reference population vectors
    let reference_vectors: Vec<ReferenceVector> = call(
        CallTargetCell::Local,
        ZomeName::from("population_health"),
        FunctionName::from("get_risk_reference_vectors"),
        None,
        input.risk_category.clone(),
    )?;

    // Compute risk score via HDC similarity
    let mut risk_scores = Vec::new();
    for patient_vec in &genetic_vectors {
        let hv = patient_vec.to_hypervector()?;

        for ref_vec in &reference_vectors {
            let ref_hv = ref_vec.to_hypervector()?;
            let similarity = hv.normalized_cosine_similarity(&ref_hv);

            risk_scores.push(RiskComponent {
                category: ref_vec.risk_category.clone(),
                similarity,
                weight: ref_vec.weight,
            });
        }
    }

    // Aggregate into final score
    let weighted_sum: f64 = risk_scores.iter()
        .map(|r| r.similarity * r.weight)
        .sum();
    let total_weight: f64 = risk_scores.iter()
        .map(|r| r.weight)
        .sum();

    Ok(GeneticRiskScore {
        patient_hash: input.patient_hash,
        risk_category: input.risk_category,
        score: weighted_sum / total_weight,
        components: risk_scores,
        confidence: calculate_confidence(&genetic_vectors),
    })
}
```

### Consent Integration

All genetic data access should check consent:

```rust
use hdk::prelude::*;

fn check_genetic_data_consent(
    patient_hash: &ActionHash,
    purpose: &str,
    requester: &AgentPubKey,
) -> ExternResult<bool> {
    let consent_result: ConsentCheckResult = call(
        CallTargetCell::Local,
        ZomeName::from("consent"),
        FunctionName::from("check_access"),
        None,
        ConsentCheckInput {
            patient_hash: patient_hash.clone(),
            data_category: "genetic".to_string(),
            purpose: purpose.to_string(),
            requester: requester.clone(),
        },
    )?;

    Ok(consent_result.access_granted)
}

#[hdk_extern]
pub fn get_genetic_similarity(input: SimilarityInput) -> ExternResult<Option<f64>> {
    let agent = agent_info()?.agent_initial_pubkey;

    // Check consent before accessing
    if !check_genetic_data_consent(&input.patient_hash, "similarity_analysis", &agent)? {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Consent not granted for genetic data access".into()
        )));
    }

    // Proceed with similarity computation
    // ...
}
```

---

## TypeScript SDK Integration

### Installing the SDK

```bash
npm install @mycelix/hdc-sdk @holochain/client
```

### Using HDC Functions

```typescript
import { AppClient } from '@holochain/client';
import { HdcClient, GeneticVector, SimilarityResult } from '@mycelix/hdc-sdk';

// Initialize client
const client = await AppClient.connect('ws://localhost:8888');
const hdc = new HdcClient(client, 'health');

// Encode DNA sequence
const vector = await hdc.encodeDnaSequence({
  sequence: 'ATCGATCGATCG',
  sourceId: 'sample-001',
  kmerLength: 6,
});

console.log('Encoded vector hash:', vector.actionHash);

// Find similar genetics
const similar = await hdc.findSimilar({
  queryHash: vector.actionHash,
  threshold: 0.8,
  limit: 10,
});

for (const result of similar) {
  console.log(`${result.sourceId}: ${(result.similarity * 100).toFixed(1)}%`);
}
```

### Encoding SNP Panel

```typescript
const snpPanel = await hdc.encodeSnpPanel({
  snps: [
    { rsid: 'rs1801133', genotype: 1 },
    { rsid: 'rs1801131', genotype: 0 },
    { rsid: 'rs429358', genotype: 0 },
  ],
  sourceId: 'panel-001',
});
```

### Encoding HLA Typing

```typescript
const hlaResult = await hdc.encodeHlaTyping({
  alleles: ['A*01:01', 'A*02:01', 'B*07:02', 'B*08:01'],
  sourceId: 'hla-typing-001',
});

// Find compatible donors
const matches = await hdc.findHlaMatches({
  recipientHash: hlaResult.actionHash,
  minMatchScore: 0.9,
});
```

---

## EHR Gateway Integration

The EHR Gateway connects external healthcare systems to Mycelix-Health.

### Pull Service Configuration

```typescript
// services/ehr-gateway/src/config.ts
export interface GatewayConfig {
  holochain: {
    url: string;
    appId: string;
    roleName: string;
  };
  hdc: {
    defaultKmerLength: number;
    enableAutoEncoding: boolean;
    geneticObservationCodes: string[];
  };
}

const defaultConfig: GatewayConfig = {
  holochain: {
    url: 'ws://localhost:8888',
    appId: 'mycelix-health',
    roleName: 'health',
  },
  hdc: {
    defaultKmerLength: 6,
    enableAutoEncoding: true,
    geneticObservationCodes: [
      '69548-6',  // Genetic variant assessment
      '51963-7',  // Medication-related genetic testing
      '55232-3',  // Genetic disease carrier analysis
    ],
  },
};
```

### Automatic Genetic Encoding

```typescript
// In PullService
async pullPatientData(patientId: string, options: PullOptions): Promise<PullResult> {
  const bundle = await this.fetchPatientBundle(patientId, options);

  // Ingest into Holochain
  const ingestReport = await this.holochainClient.callZome({
    zome_name: 'fhir_bridge',
    fn_name: 'ingest_bundle',
    payload: { bundle, source_system: options.sourceSystem },
  });

  // If enabled, trigger genetic encoding
  if (this.config.hdc.enableAutoEncoding) {
    await this.encodeGeneticObservations(bundle, ingestReport);
  }

  return { ingestReport, bundle };
}

private async encodeGeneticObservations(
  bundle: FhirBundle,
  ingestReport: IngestReport
): Promise<void> {
  const geneticObs = bundle.entry?.filter(e =>
    e.resource?.resourceType === 'Observation' &&
    this.isGeneticObservation(e.resource)
  );

  for (const obs of geneticObs || []) {
    await this.holochainClient.callZome({
      zome_name: 'hdc_genetics',
      fn_name: 'encode_observation',
      payload: { observation: obs.resource },
    });
  }
}
```

---

## Privacy Considerations

### Differential Privacy in Queries

```rust
use hdc_core::{DpHypervector, DpParams, PrivacyBudget};

#[hdk_extern]
pub fn private_similarity_search(input: PrivateSearchInput) -> ExternResult<Vec<NoisySimilarity>> {
    // Load patient's privacy budget
    let mut budget = get_privacy_budget(&input.patient_hash)?;

    let params = DpParams {
        epsilon: 0.5,  // Per-query privacy cost
        delta: 1e-5,
        sensitivity: 1.0,
    };

    // Check if budget allows this query
    if !budget.can_afford(params.epsilon) {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Privacy budget exhausted. Remaining: {}", budget.remaining())
        )));
    }

    // Load query vector and add noise
    let query = get_genetic_vector(&input.query_hash)?;
    let noisy_query = DpHypervector::from_hypervector(query, &params, &mut budget)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?;

    // Compute noisy similarities
    let mut results = Vec::new();
    for target_hash in input.search_space {
        let target = get_genetic_vector(&target_hash)?;
        let noisy_target = DpHypervector::from_hypervector(target, &params, &mut budget)
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?;

        let noisy_sim = noisy_query.private_similarity(&noisy_target, &params, &mut budget)
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?;

        results.push(NoisySimilarity {
            target_hash,
            similarity: noisy_sim,
            epsilon_spent: params.epsilon,
        });
    }

    // Update budget
    update_privacy_budget(&input.patient_hash, &budget)?;

    Ok(results)
}
```

### Zero-Knowledge Proofs

Integrate with `zkhealth` zome for proving genetic properties without revealing data:

```rust
// From zkhealth zome

#[hdk_extern]
pub fn prove_genetic_trait(input: TraitProofInput) -> ExternResult<ZkProof> {
    // Get genetic vector (never exposed)
    let vector = get_genetic_vector(&input.genetic_hash)?;

    // Compute trait presence using HDC
    let trait_reference = get_trait_reference_vector(&input.trait_id)?;
    let similarity = vector.normalized_cosine_similarity(&trait_reference);
    let trait_present = similarity >= input.threshold;

    // Generate ZK proof that trait is present/absent without revealing vector
    let proof = generate_membership_proof(
        &vector,
        &trait_reference,
        trait_present,
        &input.proof_params,
    )?;

    Ok(ZkProof {
        trait_id: input.trait_id,
        result: trait_present,
        proof_data: proof,
        // Vector data never leaves zome
    })
}
```

---

## Performance Optimization

### Indexing Strategies

```rust
// Create locality-sensitive hash buckets for fast search

fn create_lsh_index(vectors: &[GeneticVector]) -> ExternResult<Vec<ActionHash>> {
    let num_bands = 20;
    let rows_per_band = 50;  // Total: 1000 bits per signature

    let mut bucket_hashes = Vec::new();

    for vector in vectors {
        let hv = vector.to_hypervector()?;

        // Create band signatures
        for band in 0..num_bands {
            let start = band * rows_per_band;
            let end = start + rows_per_band;

            // Extract band bits and hash to bucket
            let mut band_signature = Vec::new();
            for i in start..end {
                band_signature.push(hv.get_bit(i));
            }

            let bucket_id = hash_band_signature(&band_signature);
            let bucket_path = format!("lsh-index/{}/{}", band, bucket_id);

            // Link vector to bucket
            create_link(
                anchor_hash(&bucket_path)?,
                vector.action_hash.clone(),
                LinkTypes::LshBucket,
                (),
            )?;
        }
    }

    Ok(bucket_hashes)
}

fn lsh_search(query: &Hypervector, threshold: f64) -> ExternResult<Vec<ActionHash>> {
    let num_bands = 20;
    let rows_per_band = 50;

    let mut candidates = HashSet::new();

    // Check each band bucket
    for band in 0..num_bands {
        let start = band * rows_per_band;
        let end = start + rows_per_band;

        let mut band_signature = Vec::new();
        for i in start..end {
            band_signature.push(query.get_bit(i));
        }

        let bucket_id = hash_band_signature(&band_signature);
        let bucket_path = format!("lsh-index/{}/{}", band, bucket_id);

        let links = get_links(
            GetLinksInputBuilder::try_new(anchor_hash(&bucket_path)?, LinkTypes::LshBucket)?.build()
        )?;

        for link in links {
            if let Some(hash) = link.target.into_action_hash() {
                candidates.insert(hash);
            }
        }
    }

    // Verify candidates with exact similarity
    let mut results: Vec<_> = candidates.into_iter()
        .filter_map(|hash| {
            let record = get(hash.clone(), GetOptions::default()).ok()??;
            let entry: GeneticVector = record.entry().to_app_option().ok()??;
            let vector = entry.to_hypervector().ok()?;
            let sim = query.normalized_cosine_similarity(&vector);
            if sim >= threshold { Some((hash, sim)) } else { None }
        })
        .collect();

    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    Ok(results.into_iter().map(|(h, _)| h).collect())
}
```

### Caching

```rust
use std::collections::HashMap;
use std::sync::RwLock;

lazy_static::lazy_static! {
    static ref VECTOR_CACHE: RwLock<HashMap<ActionHash, Hypervector>> = RwLock::new(HashMap::new());
}

fn get_cached_vector(hash: &ActionHash) -> ExternResult<Hypervector> {
    // Check cache first
    if let Some(hv) = VECTOR_CACHE.read().unwrap().get(hash) {
        return Ok(hv.clone());
    }

    // Load from DHT
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Not found".into())))?;
    let entry: GeneticVector = record.entry().to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid".into())))?;
    let hv = entry.to_hypervector()?;

    // Cache for future use
    VECTOR_CACHE.write().unwrap().insert(hash.clone(), hv.clone());

    Ok(hv)
}
```

---

## Deployment Patterns

### Development

```bash
# Start Holochain sandbox
hc sandbox generate workdir
hc sandbox run -p 8888

# Build zomes
cd dna/zomes
cargo build --release --target wasm32-unknown-unknown

# Package DNA
hc dna pack dna/workdir --output dist/health.dna
hc app pack workdir --output dist/mycelix-health.happ
```

### Production

```yaml
# docker-compose.yml
version: '3.8'

services:
  holochain:
    image: holochain/holochain:0.5
    volumes:
      - holochain-data:/holochain
      - ./dist:/happs
    ports:
      - "8888:8888"
    environment:
      - HOLOCHAIN_ADMIN_PORT=8888

  ehr-gateway:
    build: ./services/ehr-gateway
    ports:
      - "3000:3000"
    environment:
      - HOLOCHAIN_URL=ws://holochain:8888
      - HDC_KMER_LENGTH=6
    depends_on:
      - holochain

volumes:
  holochain-data:
```

### Monitoring

```rust
// Metrics for HDC operations

#[hdk_extern]
pub fn get_hdc_metrics(_: ()) -> ExternResult<HdcMetrics> {
    let encoding_count = get_link_count(anchor_hash("metrics/encodings")?)?;
    let similarity_queries = get_link_count(anchor_hash("metrics/queries")?)?;

    Ok(HdcMetrics {
        total_encodings: encoding_count,
        total_queries: similarity_queries,
        cache_hit_rate: get_cache_stats().hit_rate,
        avg_query_time_ms: get_timing_stats().avg_ms,
    })
}
```

---

## Troubleshooting

### Common Issues

**"InvalidDimension" error**
- Ensure hypervector data is exactly 1,250 bytes
- Check for serialization issues

**"SequenceTooShort" error**
- Sequence must be at least k characters long
- Use smaller k-mer length for short sequences

**Slow similarity searches**
- Implement LSH indexing (see above)
- Use caching for frequently accessed vectors
- Consider batch operations

**Privacy budget exhausted**
- Increase total budget for patient
- Use larger epsilon (less privacy, more queries)
- Aggregate queries to reduce budget consumption

---

## Next Steps

- [API Reference](./API.md) - Complete API documentation
- [Benchmarks](../benches/) - Performance testing
- [Examples](../examples/) - Working code samples
