# HDC-Core API Reference

Hyperdimensional Computing library for genetic data encoding and privacy-preserving similarity search.

## Table of Contents

- [Overview](#overview)
- [Core Types](#core-types)
- [DNA Encoding](#dna-encoding)
- [SNP Encoding](#snp-encoding)
- [HLA Encoding](#hla-encoding)
- [Pharmacogenomics](#pharmacogenomics)
- [VCF Processing](#vcf-processing)
- [Batch Operations](#batch-operations)
- [Differential Privacy](#differential-privacy)
- [Similarity Metrics](#similarity-metrics)
- [Confidence Scoring](#confidence-scoring)
- [GPU Acceleration](#gpu-acceleration)
- [Error Handling](#error-handling)

---

## Overview

HDC-Core implements binary hypervectors with 10,000 dimensions for encoding genetic data. Key properties:

- **Dimension**: 10,000 bits (1,250 bytes)
- **Operations**: Bind (XOR), Bundle (majority), Permute (rotate)
- **Similarity**: Hamming, Cosine, Jaccard metrics
- **Privacy**: Built-in differential privacy support

### Quick Start

```rust
use hdc_core::{DnaEncoder, Seed};

// Create encoder with reproducible seed
let seed = Seed::from_string("my-project-v1");
let encoder = DnaEncoder::new(seed, 6);

// Encode DNA sequence
let result = encoder.encode_sequence("ATCGATCGATCG").unwrap();

// Compare sequences
let other = encoder.encode_sequence("ATCGATCGATCG").unwrap();
let similarity = result.vector.normalized_cosine_similarity(&other.vector);
println!("Similarity: {:.3}", similarity); // 1.000
```

---

## Core Types

### Constants

```rust
pub const HYPERVECTOR_DIM: usize = 10_000;      // Number of bits
pub const HYPERVECTOR_BYTES: usize = 1_250;     // Number of bytes
pub const DEFAULT_KMER_LENGTH: u8 = 6;          // Default k-mer size
```

### Seed

Reproducible 32-byte seed for deterministic hypervector generation.

```rust
pub struct Seed(pub [u8; 32]);

impl Seed {
    /// Create from any string (hashed via SHA-256)
    pub fn from_string(s: &str) -> Self;

    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self;

    /// Get underlying bytes
    pub fn as_bytes(&self) -> &[u8; 32];
}
```

**Example:**
```rust
// Same string always produces same seed
let seed1 = Seed::from_string("experiment-v1");
let seed2 = Seed::from_string("experiment-v1");
assert_eq!(seed1, seed2);
```

### Hypervector

10,000-bit binary vector supporting HDC operations.

```rust
pub struct Hypervector {
    data: Vec<u8>,
}

impl Hypervector {
    /// Create from raw bytes (must be exactly 1,250 bytes)
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, HdcError>;

    /// Create zero vector
    pub fn zero() -> Self;

    /// Create random vector from seed and identifier
    pub fn random(seed: &Seed, identifier: &str) -> Self;

    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8];

    /// Bind (XOR) with another vector
    pub fn bind(&self, other: &Hypervector) -> Hypervector;

    /// Permute (rotate) by shift positions
    pub fn permute(&self, shift: usize) -> Hypervector;

    /// Get single bit (0-indexed)
    pub fn get_bit(&self, index: usize) -> bool;

    /// Set single bit
    pub fn set_bit(&mut self, index: usize, value: bool);

    /// Count set bits
    pub fn popcount(&self) -> usize;

    // Similarity metrics
    pub fn hamming_similarity(&self, other: &Hypervector) -> f64;
    pub fn cosine_similarity(&self, other: &Hypervector) -> f64;
    pub fn normalized_cosine_similarity(&self, other: &Hypervector) -> f64;
    pub fn jaccard_similarity(&self, other: &Hypervector) -> f64;
}
```

### Bundle Operations

```rust
/// Majority bundle - bit is 1 if majority of input bits are 1
pub fn bundle(vectors: &[&Hypervector]) -> Hypervector;

/// Weighted bundle with importance weights
pub fn weighted_bundle(vectors: &[(&Hypervector, f64)]) -> Hypervector;
```

---

## DNA Encoding

Encodes DNA sequences using k-mer position encoding.

### DnaEncoder

```rust
pub struct DnaEncoder {
    seed: Seed,
    kmer_length: u8,
}

impl DnaEncoder {
    /// Create encoder with k-mer length (typically 4-8)
    pub fn new(seed: Seed, kmer_length: u8) -> Self;

    /// Encode a DNA sequence
    pub fn encode_sequence(&self, sequence: &str) -> HdcResult<EncodedSequence>;

    /// Encode with custom codebook
    pub fn encode_with_codebook(&self, sequence: &str, codebook: &KmerCodebook)
        -> HdcResult<EncodedSequence>;

    /// Encode with learned codebook (requires "learned" feature)
    pub fn encode_with_learned_codebook(&self, sequence: &str, codebook: &LearnedKmerCodebook)
        -> HdcResult<EncodedSequence>;
}
```

### EncodedSequence

```rust
pub struct EncodedSequence {
    /// The hypervector encoding
    pub vector: Hypervector,

    /// Number of k-mers extracted
    pub kmer_count: u32,

    /// Original sequence length
    pub sequence_length: usize,
}
```

**Example:**
```rust
use hdc_core::{DnaEncoder, Seed};

let seed = Seed::from_string("dna-project");
let encoder = DnaEncoder::new(seed, 6); // 6-mer encoding

// Valid nucleotides: A, C, G, T (case-insensitive)
let result = encoder.encode_sequence("ATCGATCGATCGATCG").unwrap();
println!("Encoded {} k-mers from {} bp", result.kmer_count, result.sequence_length);

// Compare two sequences
let seq1 = encoder.encode_sequence("ATCGATCG").unwrap();
let seq2 = encoder.encode_sequence("ATCGATCG").unwrap();
assert!((seq1.vector.normalized_cosine_similarity(&seq2.vector) - 1.0).abs() < 0.001);
```

---

## SNP Encoding

Encodes SNP (Single Nucleotide Polymorphism) panels.

### SnpEncoder

```rust
pub struct SnpEncoder {
    seed: Seed,
}

impl SnpEncoder {
    pub fn new(seed: Seed) -> Self;

    /// Encode a panel of SNPs
    /// rsid: SNP identifier (e.g., "rs12345")
    /// genotype: 0=homozygous ref, 1=heterozygous, 2=homozygous alt
    pub fn encode_panel(&self, snps: &[(&str, u8)]) -> Hypervector;

    /// Encode single SNP
    pub fn encode_snp(&self, rsid: &str, genotype: u8) -> Hypervector;
}
```

**Example:**
```rust
use hdc_core::{SnpEncoder, Seed};

let encoder = SnpEncoder::new(Seed::from_string("snp-study"));

let panel = vec![
    ("rs1801133", 1u8),  // MTHFR C677T heterozygous
    ("rs1801131", 0u8),  // MTHFR A1298C homozygous ref
    ("rs429358", 0u8),   // APOE ε4 marker
];

let encoded = encoder.encode_panel(&panel);
```

---

## HLA Encoding

Encodes Human Leukocyte Antigen (HLA) typing for transplant matching.

### HlaEncoder (Basic)

```rust
pub struct HlaEncoder {
    seed: Seed,
}

impl HlaEncoder {
    pub fn new(seed: Seed) -> Self;

    /// Encode HLA alleles (e.g., ["A*01:01", "B*07:02"])
    pub fn encode_hla_typing(&self, alleles: &[&str]) -> Hypervector;
}
```

### LocusWeightedHlaEncoder (Transplant Matching)

Applies clinically-validated weights per HLA locus.

```rust
pub struct LocusWeightedHlaEncoder {
    seed: Seed,
    weights: HashMap<String, f64>,
}

impl LocusWeightedHlaEncoder {
    /// Create with default clinical weights
    /// (HLA-DRB1, DQB1 weighted higher than A, B, C)
    pub fn new(seed: Seed) -> Self;

    /// Create with custom weights
    pub fn with_weights(seed: Seed, weights: HashMap<String, f64>) -> Self;

    /// Encode with locus-specific weights
    pub fn encode(&self, alleles: &[&str]) -> LocusEncodedHla;

    /// Calculate match score (0.0-1.0)
    pub fn match_score(&self, donor: &LocusEncodedHla, recipient: &LocusEncodedHla) -> f64;
}
```

**Example:**
```rust
use hdc_core::{LocusWeightedHlaEncoder, Seed};

let encoder = LocusWeightedHlaEncoder::new(Seed::from_string("transplant-match"));

let donor = encoder.encode(&["A*01:01", "A*02:01", "B*07:02", "B*08:01", "DRB1*03:01", "DRB1*04:01"]);
let recipient = encoder.encode(&["A*01:01", "A*03:01", "B*07:02", "B*44:02", "DRB1*03:01", "DRB1*07:01"]);

let score = encoder.match_score(&donor, &recipient);
println!("HLA match score: {:.2}%", score * 100.0);
```

---

## Pharmacogenomics

Encodes star allele diplotypes for drug metabolism prediction.

### StarAlleleEncoder

```rust
pub struct StarAlleleEncoder {
    seed: Seed,
}

impl StarAlleleEncoder {
    pub fn new(seed: Seed) -> Self;

    /// Encode single gene diplotype (e.g., CYP2D6 *1/*4)
    pub fn encode_diplotype(&self, gene: &str, allele1: &str, allele2: &str)
        -> HdcResult<EncodedDiplotype>;

    /// Encode full PGx profile
    pub fn encode_profile(&self, diplotypes: &[(&str, &str, &str)])
        -> HdcResult<EncodedPgxProfile>;

    /// Predict metabolizer phenotype
    pub fn predict_phenotype(&self, diplotype: &EncodedDiplotype) -> MetabolizerPhenotype;

    /// Get drug recommendation
    pub fn drug_recommendation(&self, profile: &EncodedPgxProfile, drug: &str)
        -> HdcResult<DrugRecommendation>;
}
```

### MetabolizerPhenotype

```rust
pub enum MetabolizerPhenotype {
    PoorMetabolizer,
    IntermediateMetabolizer,
    NormalMetabolizer,       // Previously "Extensive"
    UltrarapidMetabolizer,
    Unknown,
}
```

### AncestryInformedEncoder

Adjusts predictions based on population-specific allele frequencies.

```rust
pub struct AncestryInformedEncoder {
    base_encoder: StarAlleleEncoder,
    ancestry: Ancestry,
}

pub enum Ancestry {
    European,
    African,
    EastAsian,
    SouthAsian,
    Hispanic,
    Admixed(Vec<(Ancestry, f64)>),
    Unknown,
}

impl AncestryInformedEncoder {
    pub fn new(seed: Seed, ancestry: Ancestry) -> Self;

    /// Encode with ancestry context
    pub fn encode_diplotype(&self, gene: &str, allele1: &str, allele2: &str)
        -> HdcResult<AncestryEncodedDiplotype>;

    /// Get ancestry-adjusted drug guidance
    pub fn dosing_guidance(&self, profile: &AncestryEncodedProfile, drug: &str)
        -> HdcResult<DosingGuidance>;
}
```

**Example:**
```rust
use hdc_core::{StarAlleleEncoder, AncestryInformedEncoder, Ancestry, Seed};

let encoder = StarAlleleEncoder::new(Seed::from_string("pgx-clinic"));

// Encode CYP2D6 diplotype
let diplotype = encoder.encode_diplotype("CYP2D6", "*1", "*4").unwrap();
let phenotype = encoder.predict_phenotype(&diplotype);
println!("CYP2D6 phenotype: {:?}", phenotype);

// Full profile
let profile = encoder.encode_profile(&[
    ("CYP2D6", "*1", "*4"),
    ("CYP2C19", "*1", "*2"),
    ("CYP2C9", "*1", "*1"),
]).unwrap();

let rec = encoder.drug_recommendation(&profile, "codeine").unwrap();
println!("Codeine recommendation: {:?}", rec);
```

---

## VCF Processing

Parse and encode VCF (Variant Call Format) files.

### VcfReader

```rust
pub struct VcfReader {
    // Internal state
}

impl VcfReader {
    /// Open VCF file (supports .vcf and .vcf.gz with "gzip" feature)
    pub fn open<P: AsRef<Path>>(path: P) -> HdcResult<Self>;

    /// Get sample names from header
    pub fn samples(&self) -> &[String];

    /// Iterate over variants
    pub fn variants(&mut self) -> impl Iterator<Item = HdcResult<Variant>>;

    /// Filter to genomic region
    pub fn filter_region(&mut self, region: GenomicRegion) -> &mut Self;
}
```

### VcfEncoder

```rust
pub struct VcfEncoder {
    seed: Seed,
}

impl VcfEncoder {
    pub fn new(seed: Seed) -> Self;

    /// Encode all variants in VCF for a sample
    pub fn encode_vcf(&self, reader: &mut VcfReader, sample: &str) -> HdcResult<EncodedVcf>;
}
```

### WgsVcfEncoder (Whole Genome)

Streaming encoder for large WGS VCF files.

```rust
pub struct WgsVcfEncoder {
    seed: Seed,
    config: WgsEncodingConfig,
}

pub struct WgsEncodingConfig {
    /// Minimum variant quality score
    pub min_quality: Option<f64>,

    /// Only include PASS variants
    pub pass_only: bool,

    /// Chunk size for streaming
    pub chunk_size: usize,

    /// Enable parallel processing
    pub parallel: bool,
}

impl WgsVcfEncoder {
    pub fn new(seed: Seed, config: WgsEncodingConfig) -> Self;

    /// Encode entire VCF file
    pub fn encode_file<P: AsRef<Path>>(&self, path: P) -> HdcResult<WgsEncodedResult>;

    /// Stream encode with callback
    pub fn stream_encode<P, F>(&self, path: P, callback: F) -> HdcResult<()>
    where
        P: AsRef<Path>,
        F: FnMut(WgsEncodedResult);
}
```

**Example:**
```rust
use hdc_core::{VcfReader, VcfEncoder, WgsVcfEncoder, WgsEncodingConfig, Seed};

// Simple VCF encoding
let encoder = VcfEncoder::new(Seed::from_string("vcf-study"));
let mut reader = VcfReader::open("sample.vcf")?;
let encoded = encoder.encode_vcf(&mut reader, "SAMPLE001")?;

// WGS encoding with configuration
let config = WgsEncodingConfig {
    min_quality: Some(30.0),
    pass_only: true,
    chunk_size: 10000,
    parallel: true,
};
let wgs_encoder = WgsVcfEncoder::new(Seed::from_string("wgs"), config);
let result = wgs_encoder.encode_file("whole_genome.vcf.gz")?;
println!("Encoded {} variants", result.variant_count);
```

---

## Batch Operations

Process multiple encodings efficiently.

### BatchEncoder

```rust
pub struct BatchEncoder {
    config: BatchConfig,
}

pub struct BatchConfig {
    /// Number of parallel workers (requires "parallel" feature)
    pub workers: usize,

    /// Continue on individual encoding errors
    pub continue_on_error: bool,

    /// Progress callback interval
    pub progress_interval: usize,
}

impl BatchEncoder {
    pub fn new(config: BatchConfig) -> Self;

    /// Encode multiple DNA sequences
    pub fn encode_sequences(&self, encoder: &DnaEncoder, sequences: &[&str])
        -> BatchResult<EncodedSequence>;

    /// Encode multiple SNP panels
    pub fn encode_snp_panels(&self, encoder: &SnpEncoder, panels: &[Vec<(&str, u8)>])
        -> BatchResult<Hypervector>;
}
```

### BatchQueryBuilder

Fluent API for batch similarity queries.

```rust
pub struct BatchQueryBuilder<'a> {
    // Internal state
}

impl<'a> BatchQueryBuilder<'a> {
    /// Create query builder from vector collection
    pub fn new(vectors: &'a [Hypervector]) -> Self;

    /// Set similarity metric
    pub fn metric(self, metric: SimilarityMetric) -> Self;

    /// Filter by minimum similarity
    pub fn min_similarity(self, threshold: f64) -> Self;

    /// Limit to top-k results per query
    pub fn top_k(self, k: usize) -> Self;

    /// Find similar vectors for query
    pub fn find_similar(&self, query: &Hypervector) -> Vec<(usize, f64)>;

    /// Compute full similarity matrix
    pub fn similarity_matrix(&self) -> SimilarityMatrix;
}
```

**Example:**
```rust
use hdc_core::{BatchEncoder, BatchConfig, BatchQueryBuilder, DnaEncoder, Seed};

let encoder = DnaEncoder::new(Seed::from_string("batch"), 6);
let batch = BatchEncoder::new(BatchConfig {
    workers: 4,
    continue_on_error: true,
    progress_interval: 100,
});

let sequences = vec!["ATCGATCG", "GCTAGCTA", "TAGCTAGC"];
let results = batch.encode_sequences(&encoder, &sequences);
println!("Encoded {}/{} sequences", results.successful, results.total);

// Query similar
let vectors: Vec<_> = results.items.into_iter()
    .filter_map(|r| r.ok())
    .map(|e| e.vector)
    .collect();

let query_builder = BatchQueryBuilder::new(&vectors)
    .min_similarity(0.5)
    .top_k(5);

let similar = query_builder.find_similar(&vectors[0]);
```

---

## Differential Privacy

Add privacy guarantees via noise injection (requires `dp` feature).

### DpParams

```rust
pub struct DpParams {
    /// Privacy parameter (smaller = more private)
    pub epsilon: f64,

    /// Failure probability
    pub delta: f64,

    /// Sensitivity of the query
    pub sensitivity: f64,
}
```

### DpHypervector

```rust
pub struct DpHypervector {
    vector: Hypervector,
    epsilon_spent: f64,
}

impl DpHypervector {
    /// Create from hypervector with noise
    pub fn from_hypervector(hv: Hypervector, params: &DpParams, budget: &mut PrivacyBudget)
        -> Result<Self, PrivacyError>;

    /// Get similarity with privacy (adds noise to result)
    pub fn private_similarity(&self, other: &DpHypervector, params: &DpParams, budget: &mut PrivacyBudget)
        -> Result<f64, PrivacyError>;
}
```

### PrivacyBudget

```rust
pub struct PrivacyBudget {
    total_epsilon: f64,
    spent_epsilon: f64,
}

impl PrivacyBudget {
    pub fn new(total_epsilon: f64) -> Self;

    /// Check if budget allows query
    pub fn can_afford(&self, epsilon: f64) -> bool;

    /// Spend from budget
    pub fn spend(&mut self, epsilon: f64) -> Result<(), PrivacyError>;

    /// Remaining budget
    pub fn remaining(&self) -> f64;
}
```

**Example:**
```rust
use hdc_core::{DpHypervector, DpParams, PrivacyBudget, DnaEncoder, Seed};

let encoder = DnaEncoder::new(Seed::from_string("private"), 6);
let encoded = encoder.encode_sequence("ATCGATCG").unwrap();

let params = DpParams {
    epsilon: 1.0,
    delta: 1e-5,
    sensitivity: 1.0,
};

let mut budget = PrivacyBudget::new(10.0);
let private = DpHypervector::from_hypervector(encoded.vector, &params, &mut budget)?;

println!("Privacy budget remaining: {}", budget.remaining());
```

---

## Similarity Metrics

Three similarity metrics are available:

### Hamming Similarity

Fraction of matching bits (0.0 to 1.0).

```rust
let sim = v1.hamming_similarity(&v2);
// 1.0 = identical, 0.5 = random/orthogonal, 0.0 = inverted
```

### Cosine Similarity

Interprets bits as bipolar values (-1/+1).

```rust
let sim = v1.cosine_similarity(&v2);      // -1.0 to +1.0
let sim = v1.normalized_cosine_similarity(&v2); // 0.0 to 1.0
```

### Jaccard Similarity

Set-based intersection over union.

```rust
let sim = v1.jaccard_similarity(&v2);
// Good for sparse vectors or set membership
```

---

## Confidence Scoring

Statistical confidence in similarity measurements.

### SimilarityWithConfidence

```rust
pub struct SimilarityWithConfidence {
    pub similarity: f64,
    pub confidence: MatchConfidence,
    pub z_score: f64,
    pub p_value: f64,
}

pub enum MatchConfidence {
    VeryHigh,    // z > 4.0
    High,        // z > 3.0
    Moderate,    // z > 2.0
    Low,         // z > 1.0
    Uncertain,   // z <= 1.0
}

impl SimilarityWithConfidence {
    /// Calculate similarity with statistical confidence
    pub fn calculate(v1: &Hypervector, v2: &Hypervector) -> Self;

    /// Check if match is statistically significant (p < 0.05)
    pub fn is_significant(&self) -> bool;
}
```

**Example:**
```rust
use hdc_core::{SimilarityWithConfidence, DnaEncoder, Seed};

let encoder = DnaEncoder::new(Seed::from_string("confidence"), 6);
let seq1 = encoder.encode_sequence("ATCGATCG").unwrap();
let seq2 = encoder.encode_sequence("ATCGATCG").unwrap();

let result = SimilarityWithConfidence::calculate(&seq1.vector, &seq2.vector);
println!("Similarity: {:.3}", result.similarity);
println!("Confidence: {:?}", result.confidence);
println!("Significant: {}", result.is_significant());
```

---

## GPU Acceleration

Hardware-accelerated similarity computation (requires `gpu` feature).

### GpuSimilarityEngine

```rust
pub struct GpuSimilarityEngine {
    // GPU state
}

impl GpuSimilarityEngine {
    /// Create engine (async GPU initialization)
    pub async fn new() -> Result<Self, GpuError>;

    /// Compute pairwise similarities on GPU
    pub async fn pairwise_similarities(&self, vectors: &[Hypervector])
        -> Result<Vec<f64>, GpuError>;

    /// Find top-k similar vectors
    pub async fn top_k(&self, query: &Hypervector, database: &[Hypervector], k: usize)
        -> Result<Vec<(usize, f64)>, GpuError>;
}
```

---

## Error Handling

### HdcError

```rust
pub enum HdcError {
    // Core errors
    InvalidDimension { expected: usize, got: usize },
    SequenceTooShort { length: usize, kmer_length: u8 },
    InvalidNucleotide(char),
    EmptyInput,

    // VCF errors
    IoError { operation: &'static str, message: String },
    VcfFormatError { line_number: Option<usize>, message: String },
    InvalidRegion { input: String, reason: String },
    InvalidGenotype { sample: Option<String>, value: String },

    // Pharmacogenomics errors
    UnknownGene { gene: String, suggestion: Option<String> },
    InvalidStarAllele { gene: String, allele: String, reason: String },
    UnsupportedDrug { drug: String, available_drugs: Vec<String> },

    // Batch errors
    BatchError { successful: usize, failed: usize, first_error: String },
    IndexOutOfBounds { index: usize, length: usize },

    // Configuration errors
    InvalidConfig { parameter: &'static str, value: String, reason: String },

    Other(String),
}
```

### Error Helpers

```rust
impl HdcError {
    /// Check error category
    pub fn is_io_error(&self) -> bool;
    pub fn is_vcf_error(&self) -> bool;
    pub fn is_pgx_error(&self) -> bool;

    /// Create specific errors
    pub fn io_error(operation: &'static str, err: std::io::Error) -> Self;
    pub fn vcf_error(message: impl Into<String>) -> Self;
    pub fn unknown_gene(gene: impl Into<String>) -> Self;
}
```

---

## Feature Flags

Enable features in `Cargo.toml`:

```toml
[dependencies]
hdc-core = { version = "0.1", features = ["dp", "parallel", "gpu"] }
```

| Feature | Description |
|---------|-------------|
| `std` | Standard library (default) |
| `dp` | Differential privacy |
| `parallel` | Parallel processing via Rayon |
| `gpu` | GPU acceleration via wgpu |
| `gzip` | Compressed VCF support |
| `learned` | Pre-trained k-mer embeddings |
| `wasm` | WebAssembly compatibility |
| `cli` | Command-line tools |

---

## CLI Tools

Build with:
```bash
cargo build --release --features cli
```

### hdc-encode

```bash
# Encode DNA sequence
hdc-encode dna ATCGATCGATCG

# Encode VCF file
hdc-encode vcf sample.vcf.gz --pass-only

# Encode SNP panel
hdc-encode snp "rs1801133:1,rs1801131:0"

# Encode HLA typing
hdc-encode hla A*01:01 A*02:01 B*07:02

# Encode pharmacogenomics
hdc-encode pgx "CYP2D6:*1/*4,CYP2C19:*1/*2"
```

### hdc-query

```bash
# Calculate similarity
hdc-query similarity @file1.hex @file2.hex --metric cosine

# Search database
hdc-query search @query.hex --database vectors.json --top-k 10

# Batch search
hdc-query batch queries.txt --database db.json --output results.json

# Similarity matrix
hdc-query matrix vectors.json --threshold 0.8
```

---

## Performance Tips

1. **Use appropriate k-mer length**: 6 is default; 5-7 good for most cases
2. **Enable parallel feature** for batch operations: ~4x speedup
3. **Use GPU** for large similarity searches: 10-100x speedup
4. **Stream large VCF files** with `WgsVcfEncoder`
5. **Pre-compute and cache** frequently used encodings
6. **Use `BatchQueryBuilder`** with `top_k` instead of full matrix

---

## Version History

- **0.1.0**: Initial release with DNA, SNP, HLA encoding
- **0.1.1**: Added pharmacogenomics and ancestry support
- **0.1.2**: WGS VCF streaming, batch operations
- **0.1.3**: GPU acceleration, differential privacy
