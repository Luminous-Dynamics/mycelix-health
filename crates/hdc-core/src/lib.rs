//! HDC Core - Hyperdimensional Computing Library
//!
//! Pure Rust implementation of hyperdimensional computing primitives
//! for genetic data encoding and privacy-preserving similarity search.
//!
//! # Features
//!
//! - Binary/bipolar hypervectors (2^14 = 16,384 dimensions)
//! - K-mer based DNA sequence encoding
//! - HLA typing for transplant matching
//! - SNP panel encoding
//! - Multiple similarity metrics (Hamming, Cosine, Jaccard)
//!
//! # Example
//!
//! ```rust
//! use hdc_core::{DnaEncoder, Seed};
//!
//! let seed = Seed::from_string("my-experiment-v1");
//! let encoder = DnaEncoder::new(seed, 6); // k=6
//!
//! let seq1 = "ACGTACGTACGTACGT";
//! let seq2 = "ACGTACGTACGTACGT";
//!
//! let enc1 = encoder.encode_sequence(seq1).unwrap();
//! let enc2 = encoder.encode_sequence(seq2).unwrap();
//!
//! let similarity = enc1.vector.normalized_cosine_similarity(&enc2.vector);
//! println!("Similarity: {:.3}", similarity);
//! ```

pub mod ops;
pub mod encoding;
pub mod similarity;
pub mod vcf;
pub mod confidence;
pub mod batch;

#[cfg(feature = "dp")]
pub mod differential_privacy;

#[cfg(feature = "gpu")]
pub mod gpu;

// Re-export commonly used types for convenience
pub use encoding::{
    DnaEncoder, HlaEncoder, SnpEncoder, EncodedSequence, HlaMatch,
    LocusWeightedHlaEncoder, LocusEncodedHla,
    AlleleHlaEncoder, AlleleEncodedHla,
    // Star allele / pharmacogenomics
    StarAlleleEncoder, EncodedDiplotype, EncodedPgxProfile,
    MetabolizerPhenotype, DrugRecommendation, DrugInteractionPrediction,
    // Ancestry-informed pharmacogenomics
    Ancestry, AncestryInformedEncoder, AncestryEncodedDiplotype,
    AncestryEncodedProfile, AncestryDrugPrediction, DoseAdjustment, DosingGuidance,
};
pub use vcf::{VcfReader, VcfEncoder, Variant, Genotype, EncodedVcf};
pub use vcf::{WgsVcfEncoder, WgsEncodingConfig, WgsEncodedResult, VariantIterator, GenomicRegion};
pub use confidence::{MatchConfidence, SimilarityWithConfidence};
pub use batch::{BatchEncoder, BatchConfig, BatchResult, BatchStats, SimilarityMatrix, BatchQueryBuilder};

#[cfg(feature = "dp")]
pub use differential_privacy::{DpParams, DpHypervector, PrivacyBudget, PrivacyError};

#[cfg(feature = "gpu")]
pub use gpu::{GpuSimilarityEngine, GpuError};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Standard hypervector dimension (2^14 for SIMD-friendly alignment)
pub const HYPERVECTOR_DIM: usize = 1 << 14; // 16,384

/// Number of bytes needed to store HYPERVECTOR_DIM bits
pub const HYPERVECTOR_BYTES: usize = (HYPERVECTOR_DIM + 7) / 8;

/// Default k-mer length for DNA encoding
pub const DEFAULT_KMER_LENGTH: u8 = 6;

/// Convenience Result type for HDC operations
pub type HdcResult<T> = Result<T, HdcError>;

/// A 32-byte seed for reproducible hypervector generation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seed(pub [u8; 32]);

impl Seed {
    /// Create a seed from a string (hashed to 32 bytes)
    pub fn from_string(s: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        let result = hasher.finalize();
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&result);
        Seed(seed)
    }

    /// Create a seed from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Seed(bytes)
    }

    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Default for Seed {
    fn default() -> Self {
        Seed([0u8; 32])
    }
}

/// A hypervector - binary representation of high-dimensional data
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hypervector {
    /// Packed binary data (16,384 bits = 2,048 bytes)
    data: Vec<u8>,
}

impl Hypervector {
    /// Create a new hypervector from raw bytes
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, HdcError> {
        if data.len() != HYPERVECTOR_BYTES {
            return Err(HdcError::InvalidDimension {
                expected: HYPERVECTOR_BYTES,
                got: data.len(),
            });
        }
        Ok(Hypervector { data })
    }

    /// Create a zero hypervector
    pub fn zero() -> Self {
        Hypervector {
            data: vec![0u8; HYPERVECTOR_BYTES],
        }
    }

    /// Create a random hypervector from a seed and identifier
    pub fn random(seed: &Seed, identifier: &str) -> Self {
        let data = ops::generate_item_vector(seed.as_bytes(), identifier);
        Hypervector { data }
    }

    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get a mutable reference to the raw bytes
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Bind (XOR) two hypervectors
    pub fn bind(&self, other: &Hypervector) -> Hypervector {
        let data = ops::bind(&self.data, &other.data);
        Hypervector { data }
    }

    /// Permute (rotate) the hypervector
    pub fn permute(&self, shift: usize) -> Hypervector {
        let data = ops::permute(&self.data, shift);
        Hypervector { data }
    }

    /// Get a single bit value
    pub fn get_bit(&self, index: usize) -> bool {
        if index >= HYPERVECTOR_DIM {
            return false;
        }
        let byte_idx = index / 8;
        let bit_idx = index % 8;
        (self.data[byte_idx] >> bit_idx) & 1 == 1
    }

    /// Set a single bit value
    pub fn set_bit(&mut self, index: usize, value: bool) {
        if index >= HYPERVECTOR_DIM {
            return;
        }
        let byte_idx = index / 8;
        let bit_idx = index % 8;
        if value {
            self.data[byte_idx] |= 1 << bit_idx;
        } else {
            self.data[byte_idx] &= !(1 << bit_idx);
        }
    }

    /// Count the number of 1 bits
    pub fn popcount(&self) -> usize {
        self.data.iter().map(|b| b.count_ones() as usize).sum()
    }

    /// Hamming similarity (fraction of matching bits)
    pub fn hamming_similarity(&self, other: &Hypervector) -> f64 {
        ops::hamming_similarity(&self.data, &other.data)
    }

    /// Cosine similarity (bipolar interpretation: -1 to +1)
    pub fn cosine_similarity(&self, other: &Hypervector) -> f64 {
        ops::cosine_similarity(&self.data, &other.data)
    }

    /// Normalized cosine similarity (0 to 1)
    pub fn normalized_cosine_similarity(&self, other: &Hypervector) -> f64 {
        ops::normalized_cosine_similarity(&self.data, &other.data)
    }

    /// Jaccard similarity (intersection over union)
    pub fn jaccard_similarity(&self, other: &Hypervector) -> f64 {
        let intersection: usize = self.data.iter()
            .zip(other.data.iter())
            .map(|(a, b)| (a & b).count_ones() as usize)
            .sum();

        let union: usize = self.data.iter()
            .zip(other.data.iter())
            .map(|(a, b)| (a | b).count_ones() as usize)
            .sum();

        if union == 0 {
            1.0
        } else {
            intersection as f64 / union as f64
        }
    }
}

/// Bundle multiple hypervectors using majority voting
pub fn bundle(vectors: &[&Hypervector]) -> Hypervector {
    let refs: Vec<&[u8]> = vectors.iter().map(|v| v.as_bytes()).collect();
    let data = ops::bundle(&refs);
    Hypervector { data }
}

/// Weighted bundle using thresholded sums
pub fn weighted_bundle(vectors: &[(&Hypervector, f64)]) -> Hypervector {
    let weighted: Vec<(&[u8], f64)> = vectors.iter()
        .map(|(v, w)| (v.as_bytes(), *w))
        .collect();
    let data = ops::weighted_bundle(&weighted);
    Hypervector { data }
}

/// Errors that can occur in HDC operations
#[derive(Debug, Clone, PartialEq)]
pub enum HdcError {
    /// Invalid vector dimension
    InvalidDimension { expected: usize, got: usize },
    /// Sequence is too short for the k-mer length
    SequenceTooShort { length: usize, kmer_length: u8 },
    /// Invalid nucleotide in sequence
    InvalidNucleotide(char),
    /// Empty input
    EmptyInput,

    // === VCF Processing Errors ===
    /// IO error during file operations
    IoError {
        operation: &'static str,
        message: String
    },
    /// VCF file format error
    VcfFormatError {
        line_number: Option<usize>,
        message: String
    },
    /// Invalid genomic region specification
    InvalidRegion {
        input: String,
        reason: String
    },
    /// Invalid genotype in VCF
    InvalidGenotype {
        sample: Option<String>,
        value: String
    },

    // === Pharmacogenomics Errors ===
    /// Unknown gene in pharmacogenomics database
    UnknownGene {
        gene: String,
        suggestion: Option<String>,
    },
    /// Invalid star allele format
    InvalidStarAllele {
        gene: String,
        allele: String,
        reason: String
    },
    /// Unsupported drug for pharmacogenomics prediction
    UnsupportedDrug {
        drug: String,
        available_drugs: Vec<String>,
    },

    // === Batch Processing Errors ===
    /// Batch processing error with partial results
    BatchError {
        successful: usize,
        failed: usize,
        first_error: String
    },
    /// Index out of bounds in batch operation
    IndexOutOfBounds {
        index: usize,
        length: usize
    },

    // === Configuration Errors ===
    /// Invalid configuration parameter
    InvalidConfig {
        parameter: &'static str,
        value: String,
        reason: String
    },

    /// Other error (legacy, prefer specific variants)
    Other(String),
}

impl std::fmt::Display for HdcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Core errors
            HdcError::InvalidDimension { expected, got } => {
                write!(f, "Invalid dimension: expected {} bytes, got {}", expected, got)
            }
            HdcError::SequenceTooShort { length, kmer_length } => {
                write!(f, "Sequence too short: {} < k-mer length {}", length, kmer_length)
            }
            HdcError::InvalidNucleotide(c) => {
                write!(f, "Invalid nucleotide: '{}'", c)
            }
            HdcError::EmptyInput => {
                write!(f, "Empty input provided")
            }

            // VCF errors
            HdcError::IoError { operation, message } => {
                write!(f, "IO error during {}: {}", operation, message)
            }
            HdcError::VcfFormatError { line_number, message } => {
                if let Some(line) = line_number {
                    write!(f, "VCF format error at line {}: {}", line, message)
                } else {
                    write!(f, "VCF format error: {}", message)
                }
            }
            HdcError::InvalidRegion { input, reason } => {
                write!(f, "Invalid genomic region '{}': {}", input, reason)
            }
            HdcError::InvalidGenotype { sample, value } => {
                if let Some(s) = sample {
                    write!(f, "Invalid genotype '{}' for sample '{}'", value, s)
                } else {
                    write!(f, "Invalid genotype: '{}'", value)
                }
            }

            // Pharmacogenomics errors
            HdcError::UnknownGene { gene, suggestion } => {
                if let Some(s) = suggestion {
                    write!(f, "Unknown gene '{}'. Did you mean '{}'?", gene, s)
                } else {
                    write!(f, "Unknown gene '{}'", gene)
                }
            }
            HdcError::InvalidStarAllele { gene, allele, reason } => {
                write!(f, "Invalid star allele {} {} for {}", gene, allele, reason)
            }
            HdcError::UnsupportedDrug { drug, available_drugs } => {
                if available_drugs.is_empty() {
                    write!(f, "Unsupported drug: '{}'", drug)
                } else {
                    write!(f, "Unsupported drug: '{}'. Available: {}", drug,
                           available_drugs.join(", "))
                }
            }

            // Batch errors
            HdcError::BatchError { successful, failed, first_error } => {
                write!(f, "Batch processing: {} succeeded, {} failed. First error: {}",
                       successful, failed, first_error)
            }
            HdcError::IndexOutOfBounds { index, length } => {
                write!(f, "Index {} out of bounds for length {}", index, length)
            }

            // Configuration errors
            HdcError::InvalidConfig { parameter, value, reason } => {
                write!(f, "Invalid configuration for '{}': value '{}' - {}",
                       parameter, value, reason)
            }

            // Legacy catch-all
            HdcError::Other(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

impl std::error::Error for HdcError {}

impl From<std::io::Error> for HdcError {
    fn from(err: std::io::Error) -> Self {
        HdcError::IoError {
            operation: "file operation",
            message: err.to_string()
        }
    }
}

impl HdcError {
    /// Create an IO error with a specific operation context
    pub fn io_error(operation: &'static str, err: std::io::Error) -> Self {
        HdcError::IoError {
            operation,
            message: err.to_string()
        }
    }

    /// Create a VCF format error
    pub fn vcf_error(message: impl Into<String>) -> Self {
        HdcError::VcfFormatError {
            line_number: None,
            message: message.into()
        }
    }

    /// Create a VCF format error with line number
    pub fn vcf_error_at_line(line: usize, message: impl Into<String>) -> Self {
        HdcError::VcfFormatError {
            line_number: Some(line),
            message: message.into()
        }
    }

    /// Create an unknown gene error
    pub fn unknown_gene(gene: impl Into<String>) -> Self {
        HdcError::UnknownGene {
            gene: gene.into(),
            suggestion: None
        }
    }

    /// Create an unknown gene error with suggestion
    pub fn unknown_gene_with_suggestion(gene: impl Into<String>, suggestion: impl Into<String>) -> Self {
        HdcError::UnknownGene {
            gene: gene.into(),
            suggestion: Some(suggestion.into())
        }
    }

    /// Check if this is an IO-related error
    pub fn is_io_error(&self) -> bool {
        matches!(self, HdcError::IoError { .. })
    }

    /// Check if this is a VCF-related error
    pub fn is_vcf_error(&self) -> bool {
        matches!(self, HdcError::VcfFormatError { .. } | HdcError::InvalidRegion { .. } | HdcError::InvalidGenotype { .. })
    }

    /// Check if this is a pharmacogenomics-related error
    pub fn is_pgx_error(&self) -> bool {
        matches!(self, HdcError::UnknownGene { .. } | HdcError::InvalidStarAllele { .. } | HdcError::UnsupportedDrug { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_from_string() {
        let seed1 = Seed::from_string("test");
        let seed2 = Seed::from_string("test");
        let seed3 = Seed::from_string("different");

        assert_eq!(seed1, seed2);
        assert_ne!(seed1, seed3);
    }

    #[test]
    fn test_hypervector_size() {
        let seed = Seed::from_string("test");
        let hv = Hypervector::random(&seed, "item");
        assert_eq!(hv.as_bytes().len(), HYPERVECTOR_BYTES);
    }

    #[test]
    fn test_bind_is_self_inverse() {
        let seed = Seed::from_string("test");
        let a = Hypervector::random(&seed, "a");
        let b = Hypervector::random(&seed, "b");

        let bound = a.bind(&b);
        let unbound = bound.bind(&b);

        assert_eq!(a, unbound);
    }

    #[test]
    fn test_self_similarity() {
        let seed = Seed::from_string("test");
        let hv = Hypervector::random(&seed, "item");

        assert!((hv.hamming_similarity(&hv) - 1.0).abs() < 0.001);
        assert!((hv.cosine_similarity(&hv) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_bundle_majority() {
        let seed = Seed::from_string("test");
        let a = Hypervector::random(&seed, "a");
        let b = Hypervector::random(&seed, "b");
        let c = Hypervector::random(&seed, "c");

        let bundled = bundle(&[&a, &b, &c]);

        // Bundled vector should exist and have correct size
        assert_eq!(bundled.as_bytes().len(), HYPERVECTOR_BYTES);
    }
}
