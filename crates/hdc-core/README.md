# HDC-Core: Hyperdimensional Computing for Genomics

High-performance library for encoding genetic data into hyperdimensional vectors. Enables privacy-preserving similarity search, pharmacogenomics analysis, and patient matching at scale.

## Features

- **10,000-dimensional binary hypervectors** for genetic data encoding
- **DNA/RNA sequence encoding** with k-mer decomposition
- **VCF variant encoding** with WES/WGS scale support (streaming, parallel)
- **Pharmacogenomics** with CPIC-compliant star allele encoding
- **Ancestry-informed PGx** with population-specific allele frequencies
- **HLA typing** for transplant compatibility matching
- **Differential privacy** with configurable epsilon (ε-DP)
- **GPU acceleration** for bulk similarity computation
- **Batch processing** with parallel encoding support

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
hdc-core = { version = "0.1", features = ["dp"] }
```

### DNA Sequence Encoding

```rust
use hdc_core::{DnaEncoder, Seed};

let seed = Seed::from_string("my-experiment-v1");
let encoder = DnaEncoder::new(seed, 6);  // k=6

let seq1 = "ACGTACGTACGTACGTACGT";
let seq2 = "ACGTACGTACGTACGTACGT";

let enc1 = encoder.encode_sequence(seq1)?;
let enc2 = encoder.encode_sequence(seq2)?;

let similarity = enc1.vector.hamming_similarity(&enc2.vector);
println!("Similarity: {:.3}", similarity);  // ~1.0 for identical
```

### VCF File Encoding

```rust
use hdc_core::{VcfReader, VcfEncoder, Seed};
use std::fs::File;

let file = File::open("patient.vcf")?;
let mut reader = VcfReader::new(file)?;
let variants = reader.read_variants()?;

let seed = Seed::from_string("clinical-v1");
let encoder = VcfEncoder::new(seed);
let encoded = encoder.encode_variants(&variants)?;

println!("Encoded {} variants", variants.len());
```

### Whole Genome Scale VCF (Streaming)

```rust
use hdc_core::vcf::{WgsVcfEncoder, WgsEncodingConfig};
use hdc_core::Seed;

let config = WgsEncodingConfig::default()
    .with_chunk_size(50_000)
    .with_parallel(true)
    .with_pass_only(true);

let encoder = WgsVcfEncoder::new(Seed::from_string("wgs-study"), config);
let result = encoder.encode_file("genome.vcf.gz")?;

println!("Encoded {} variants across {} chromosomes",
    result.total_variants,
    result.chromosome_vectors.len()
);
println!("Processing time: {}ms", result.stats.processing_time_ms);
```

### Pharmacogenomics (Star Alleles)

```rust
use hdc_core::{StarAlleleEncoder, Seed};

let seed = Seed::from_string("pgx-v1");
let encoder = StarAlleleEncoder::new(seed);

// Encode patient's CYP2D6 diplotype
let diplotype = encoder.encode_diplotype("CYP2D6", "*1", "*4")?;
println!("{}: {} (AS={})",
    diplotype.to_notation(),
    diplotype.phenotype,
    diplotype.activity_score
);  // "CYP2D6 *1/*4: Intermediate Metabolizer (AS=1.0)"

// Check drug interaction
let profile = encoder.encode_profile(&[
    ("CYP2D6", "*1", "*4"),
    ("CYP2C19", "*1", "*1"),
])?;

if let Some(prediction) = encoder.predict_drug_interaction(&profile, "codeine") {
    println!("Codeine: {:?}", prediction.recommendation);
}
```

### Ancestry-Informed Pharmacogenomics

```rust
use hdc_core::{AncestryInformedEncoder, Ancestry, Seed};

let encoder = AncestryInformedEncoder::new(Seed::from_string("ancestry-pgx"));

// East Asian patient with CYP2D6*10 (common in this population)
let profile = encoder.encode_profile_with_ancestry(&[
    ("CYP2D6", "*10", "*10"),
    ("CYP2C19", "*2", "*3"),
], &Ancestry::EastAsian)?;

// Get ancestry-aware dosing guidance
if let Some(guidance) = encoder.get_dosing_guidance("codeine", &profile) {
    println!("Drug: {}", guidance.drug);
    println!("Adjustment: {:?}", guidance.adjustment);
    println!("Reasoning: {}", guidance.reasoning);
    println!("Confidence: {:.1}%", guidance.confidence * 100.0);
    for note in &guidance.considerations {
        println!("  - {}", note);
    }
}
```

### Batch Processing

```rust
use hdc_core::batch::{BatchEncoder, BatchConfig, BatchQueryBuilder};
use hdc_core::Seed;

// High-throughput sequence encoding
let config = BatchConfig::default()
    .with_parallel(true)
    .with_kmer_length(6);

let encoder = BatchEncoder::new(Seed::from_string("batch-v1"), config);

let sequences = vec![
    "ACGTACGTACGTACGT",
    "TGCATGCATGCATGCA",
    // ... thousands more
];

let result = encoder.encode_sequences(&sequences)?;
println!("Encoded {} sequences in {}ms",
    result.success_count(),
    result.stats.processing_time_ms
);

// Pairwise similarity matrix
let vectors = encoder.encode_to_vectors(&sequences)?;
let matrix = encoder.pairwise_similarity(&vectors);
println!("Average similarity: {:.3}", matrix.average_similarity());

// Find similar pairs
for (i, j, sim) in matrix.pairs_above_threshold(0.95) {
    println!("Pair ({}, {}): {:.3}", i, j, sim);
}
```

### Differential Privacy

```rust
use hdc_core::{Hypervector, Seed};
use hdc_core::differential_privacy::{DpParams, DpHypervector};

let seed = Seed::from_string("test");
let original = Hypervector::random(&seed, "patient-genome");

// Apply differential privacy (ε=1.0)
let params = DpParams::new(1.0);
let dp_vector = DpHypervector::from_hypervector(&original, &params)?;

// Use the noisy vector for similarity queries
let noisy = dp_vector.as_hypervector();
println!("Original popcount: {}", original.popcount());
println!("Noisy popcount: {}", noisy.popcount());
```

### GPU-Accelerated Similarity

```rust
use hdc_core::{Hypervector, Seed};
use hdc_core::gpu::GpuSimilarityEngine;

let seed = Seed::from_string("gpu-test");

// Create query and database vectors
let query = Hypervector::random(&seed, "query");
let database: Vec<_> = (0..10000)
    .map(|i| Hypervector::random(&seed, &format!("item-{}", i)))
    .collect();

// Initialize GPU engine
let engine = GpuSimilarityEngine::new()?;

// Compute similarities in parallel on GPU
let similarities = engine.batch_similarity(&query, &database)?;

// Find top matches
let mut indexed: Vec<_> = similarities.iter().enumerate().collect();
indexed.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
println!("Top 5 matches: {:?}", &indexed[..5]);
```

## Features

| Feature | Description | Default |
|---------|-------------|---------|
| `std` | Standard library support | ✓ |
| `dp` | Differential privacy with RNG | ✓ |
| `parallel` | Parallel processing with rayon | |
| `gpu` | GPU acceleration with wgpu | |
| `gzip` | Read .vcf.gz files with flate2 | |
| `wasm` | WebAssembly compatibility | |

## Performance

Benchmarks on AMD Ryzen 9 7950X:

| Operation | Throughput | Comparison |
|-----------|------------|------------|
| DNA Encoding (k=6) | 69 seq/s | - |
| Similarity Computation | 4.5M ops/s | 1000x faster than BLAST |
| VCF Variant Encoding | 15K variants/s | - |
| GPU Batch Similarity | 100M ops/s | 20x faster than CPU |

## Supported Data Types

### Genetic Data
- DNA/RNA sequences (k-mer encoding)
- VCF variants (SNPs, indels, structural variants)
- Star alleles (CYP2D6, CYP2C19, CYP2C9, etc.)
- HLA typing (A, B, C, DRB1, DQB1)
- SNP panels (rsIDs)

### Pharmacogenomics Genes
- **CYP450 enzymes**: CYP2D6, CYP2C19, CYP2C9, CYP3A4, CYP3A5, CYP2B6
- **Phase II enzymes**: UGT1A1, TPMT, NUDT15
- **Transporters**: SLCO1B1
- **Other**: DPYD, VKORC1

### Ancestry Groups
- African
- American (Indigenous)
- Central/South Asian
- East Asian
- European
- Latino
- Near Eastern
- Oceanian
- Mixed/Multi-ethnic

## Clinical Applications

1. **Patient Matching** - Find genetically similar patients for cohort studies
2. **Drug Dosing** - Predict optimal doses based on pharmacogenomics
3. **Adverse Event Risk** - Identify patients at risk for drug toxicity
4. **Transplant Matching** - HLA-based donor-recipient matching
5. **Privacy-Preserving Research** - Share encoded vectors without exposing raw data

## References

- CPIC Guidelines: https://cpicpgx.org/guidelines/
- PharmGKB: https://www.pharmgkb.org/
- gnomAD: https://gnomad.broadinstitute.org/
- HDC Theory: Kanerva, P. (2009). Hyperdimensional Computing

## License

MIT License - See LICENSE file for details.
