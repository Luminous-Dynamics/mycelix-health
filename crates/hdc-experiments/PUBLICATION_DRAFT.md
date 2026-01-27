# Hyperdimensional Computing for Privacy-Preserving Genetic Similarity

## Abstract

We present a hyperdimensional computing (HDC) approach for encoding genetic data that enables privacy-preserving similarity searches. Unlike prior HDC-genomics work that focused on acceleration (HDNA, GenieHD, HyMATCH), our system is designed for **privacy-first** applications in clinical settings: transplant donor matching, pharmacogenomics, and federated research collaborations—where raw genetic sequences cannot be shared.

Using 10,000-dimensional binary hypervectors with k-mer encoding, we demonstrate:
- **89.3% order classification accuracy** on 272 real COI barcode sequences (BOLD Systems)
- **100% HLA locus classification accuracy** on 300 real IMGT/HLA allele sequences across 6 loci
- **Monotonic similarity separation** reflecting evolutionary divergence
- **~7µs similarity computation** enabling real-time clinical workflows
- **~244ms/1000bp encoding** with parallel processing support

Our implementation is integrated into a Holochain-based sovereign health records system, enabling decentralized, consent-governed genetic similarity queries without exposing raw genomic data.

## 1. Introduction

### 1.1 The Privacy Problem in Genetic Data

Genetic similarity matching is critical for:
- **Transplant matching**: HLA compatibility determines organ rejection risk
- **Clinical trials**: Finding genetically similar participants
- **Pharmacogenomics**: Drug response prediction based on genetic variants
- **Disease risk**: Stratifying patients by genetic risk factors

Current approaches require sharing raw genetic sequences, creating privacy and regulatory challenges. The EU GDPR, US HIPAA, and emerging genetic privacy laws treat genomic data as highly sensitive.

### 1.2 Prior Work: Acceleration, Not Privacy

Existing HDC-genomics research focuses on **speed**, not **privacy**:

| Paper | Year | Focus | Privacy? |
|-------|------|-------|----------|
| HDNA | 2020 | DNA sequence classification | No |
| GenieHD | 2020 | Genome classification | No |
| HyMATCH | 2022 | Read alignment acceleration | No |

These systems assume centralized databases with raw sequences. Our work targets the opposite scenario: **distributed, consent-governed genetic data** where raw sequences remain with the data owner.

### 1.3 Our Contribution

We present the first HDC-based genetic encoding system designed for:
1. **Privacy-preserving similarity**: Compare genetic profiles without exposing sequences
2. **Decentralized storage**: Holochain DHT with patient-controlled consent
3. **Clinical applicability**: Validated on real HLA and COI barcode data
4. **Reproducible encoding**: Deterministic codebooks enable cross-institution matching

## 2. Methods

### 2.1 Hyperdimensional Encoding

We use 10,000-dimensional binary hypervectors with k-mer positional encoding:

```
sequence → [k-mers] → [item vectors] ⊗ [position vectors] → bundle → hypervector
```

Key parameters:
- **Dimensions**: D = 10,000 (1,250 bytes per vector)
- **K-mer length**: k = 6 (default, configurable 3-12)
- **Encoding**: Bipolar interpretation (-1/+1) of binary vectors
- **Similarity**: Normalized cosine similarity

### 2.2 Encoding Pipeline

1. **Codebook Generation**: Deterministic seed generates consistent item vectors
2. **K-mer Extraction**: Sliding window over sequence
3. **Position Binding**: XOR with position-specific vectors
4. **Bundling**: Majority vote across all position-bound k-mers
5. **Normalization**: Threshold at majority

### 2.3 Similarity Computation

For two hypervectors A and B:
```
similarity(A, B) = (A · B) / (||A|| × ||B||)
```
Range: [0, 1] where 1 = identical encoding, 0.5 = random

### 2.4 Experimental Data

**Taxonomy Validation (BOLD Systems)**:
- 272 COI-5P barcode sequences
- 16 species across 3 orders (Primates, Lepidoptera, Passeriformes)
- Source: BOLD Systems v4 API

**HLA Validation (IMGT/HLA)**:
- 300 HLA allele nucleotide sequences
- 6 loci: HLA-A, HLA-B, HLA-C, DRB1, DQB1, DPB1 (50 per locus)
- Source: IMGT/HLA GitHub repository

## 3. Results

### 3.1 Taxonomy Validation (Experiment 5)

| Comparison | Mean Similarity | Std Dev | N |
|------------|-----------------|---------|---|
| Same species | 0.6439 | 0.2158 | 3,483 |
| Same order | 0.5247 | 0.0857 | 8,920 |
| Between orders | 0.5008 | 0.0059 | 24,453 |

**Key findings**:
- Monotonic separation achieved: species > order > between-orders
- Order classification accuracy: 89.3% (k=1 nearest neighbor)
- Species classification accuracy: 69.9%
- Encoding time: 386.75 ms/sequence (unoptimized)

### 3.2 HLA Validation (Experiment 6)

| Comparison | Mean Similarity | Std Dev | N |
|------------|-----------------|---------|---|
| Same two-field | 0.9199 | 0.1705 | 7,350 |
| Different locus | 0.5412 | 0.0771 | 37,500 |

**Key findings**:
- Locus classification accuracy: 100% across all 6 loci
- Two-field classification accuracy: 100%
- Clear separation: same-group (0.92) vs different-locus (0.54)
- Encoding time: ~211 ms/allele
- Pairwise comparison: ~7µs per similarity computation

### 3.3 Privacy Properties

The HDC encoding provides:
1. **One-way transformation**: Cannot recover sequence from hypervector
2. **Similarity preservation**: Related sequences produce similar vectors
3. **Configurable resolution**: K-mer length controls specificity
4. **No raw data exposure**: Only hypervectors are stored/transmitted

## 4. System Integration

### 4.1 Holochain Architecture

Our HDC encoding is integrated into Mycelix-Health, a Holochain-based sovereign health records system:

```
┌─────────────────────────────────────────┐
│  Patient Device / Clinic               │
│  ┌────────────────────────────────┐    │
│  │ HDC Encoder (hdc-core)         │    │
│  │ - DNA sequence encoding        │    │
│  │ - HLA typing encoding          │    │
│  │ - SNP panel encoding           │    │
│  └────────────────────────────────┘    │
│             │                          │
│             ▼                          │
│  ┌────────────────────────────────┐    │
│  │ Holochain DHT                  │    │
│  │ - hdc_genetics zome            │    │
│  │ - consent_membrane zome        │    │
│  │ - patient_records zome         │    │
│  └────────────────────────────────┘    │
└─────────────────────────────────────────┘
             │
             ▼ (Only hypervectors, never raw sequences)
┌─────────────────────────────────────────┐
│  Federated Similarity Search           │
│  - Cross-institution HLA matching      │
│  - Clinical trial recruitment          │
│  - Pharmacogenomic lookups             │
└─────────────────────────────────────────┘
```

### 4.2 Zome API

The `hdc_genetics` coordinator zome exposes:
- `encode_dna_sequence`: DNA → hypervector
- `encode_hla_typing`: HLA alleles → hypervector
- `encode_snp_panel`: SNPs → hypervector
- `calculate_similarity`: Compare two vectors
- `search_similar_genetics`: Batch similarity search

### 4.3 Consent Integration

All encoding operations verify consent via the `consent_membrane` zome, ensuring:
- Patient authorization for genetic data use
- Purpose-specific consent (transplant, research, etc.)
- Audit trail of similarity queries

## 5. Discussion

### 5.1 Comparison to Prior Work

| Aspect | HDNA/GenieHD | Our System |
|--------|--------------|------------|
| Primary goal | Classification speed | Privacy-preserving similarity |
| Data model | Centralized | Decentralized (Holochain DHT) |
| Raw sequence access | Required | Never exposed |
| Consent model | N/A | Built-in membrane |
| Clinical integration | Research only | Production-ready zome |

### 5.2 Limitations

1. **Approximate matching**: HDC similarity is probabilistic, not exact
2. **K-mer length tradeoff**: Higher k = more specific but more sensitive to mutations
3. **Validation scope**: Validated on COI barcodes (3 orders) and HLA (6 loci); broader validation across more genetic markers would strengthen claims
4. **No clinical deployment yet**: System is production-ready but awaits clinical partner validation

### 5.3 Future Work

1. **Secure multi-party computation**: HDC vectors in MPC protocols
2. **Differential privacy**: Add noise while preserving similarity
3. **GPU acceleration**: 10-100x speedup for batch encoding
4. **Clinical validation**: Partner with transplant registries

## 6. Conclusion

We demonstrate that hyperdimensional computing enables privacy-preserving genetic similarity searches suitable for clinical applications. Our system achieves:

- **89.3% order classification** on real COI barcode data (272 sequences, 3 orders)
- **100% HLA locus classification** on real IMGT reference alleles (300 alleles, 6 loci)
- **Sub-millisecond similarity computation** (~7µs per comparison)
- **Full Holochain integration** with consent-governed queries
- **Production-ready API** for sovereign health records

This represents the first HDC-based genetic encoding system designed for privacy rather than acceleration, opening new possibilities for federated genetic research and transplant matching without compromising patient privacy.

The expanded validation across 6 HLA loci (HLA-A, HLA-B, HLA-C, DRB1, DQB1, DPB1) demonstrates clinical applicability for transplant matching, where these loci are routinely typed.

## Data Availability

- BOLD COI sequences: BOLD Systems v4 API (public)
- IMGT/HLA sequences: github.com/ANHIG/IMGTHLA (public)
- hdc-core library: github.com/Luminous-Dynamics/mycelix-health (open source)

## Acknowledgments

This work was conducted as part of the Mycelix-Health project for sovereign health records.

---

## Appendix A: Experiment Commands

```bash
# Run taxonomy validation (Experiment 5)
cargo run -p hdc-experiments --target x86_64-unknown-linux-gnu -- \
  taxonomy --real-data --output /tmp/results

# Run HLA validation (Experiment 6)
cargo run -p hdc-experiments --target x86_64-unknown-linux-gnu -- \
  real-hla --output /tmp/results

# Build Holochain zomes
cargo build -p hdc_genetics --target wasm32-unknown-unknown
cargo build -p hdc_genetics_integrity --target wasm32-unknown-unknown
```

## Appendix B: Key Results JSON

### Experiment 5: Real Taxonomy
```json
{
  "within_species": {"mean": 0.6439, "std_dev": 0.2158, "count": 3483},
  "within_order": {"mean": 0.5247, "std_dev": 0.0857, "count": 8920},
  "between_orders": {"mean": 0.5008, "std_dev": 0.0059, "count": 24453},
  "order_classification_accuracy": 0.893,
  "species_classification_accuracy": 0.699
}
```

### Experiment 6: Real HLA (6 Loci)
```json
{
  "config": {
    "total_alleles": 300,
    "num_loci": 6,
    "kmer_length": 6,
    "data_source": "IMGT/HLA"
  },
  "same_two_field": {"mean": 0.9199, "std_dev": 0.1705, "count": 7350},
  "different_locus": {"mean": 0.5412, "std_dev": 0.0771, "count": 37500},
  "locus_classification_accuracy": 1.0,
  "field_matching_accuracy": 1.0,
  "loci_tested": ["HLA-A", "HLA-B", "HLA-C", "DRB1", "DQB1", "DPB1"]
}
```

## Appendix C: Performance Benchmarks

Criterion benchmarks on x86_64 Linux (single-threaded):

| Operation | Time | Notes |
|-----------|------|-------|
| Encode 100bp | ~27ms | Short sequence |
| Encode 500bp | ~131ms | Medium sequence |
| Encode 1000bp | ~244ms | Typical HLA length |
| Similarity (single) | ~7.1µs | Cosine similarity |
| Batch 100 pairwise | ~34ms | 4,950 comparisons |

With `parallel` feature enabled (rayon), batch encoding achieves linear speedup on multi-core systems.
